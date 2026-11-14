//! Thin wrappers over the Linux system calls the benchmark needs.
//!
//! All `unsafe` code in the crate lives in this module.

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// `openat2(2)` resolve flag: do not follow kernel "magic links".
pub const RESOLVE_NO_MAGICLINKS: u64 = 0x02;
/// `openat2(2)` resolve flag: fail rather than resolve outside the directory fd.
pub const RESOLVE_BENEATH: u64 = 0x08;

/// Permissions for files the benchmark creates.
const CREATE_MODE: libc::c_uint = 0o600;

/// Device and inode number: the identity of a filesystem object, stable
/// across renames.
pub type FileId = (u64, u64);

/// Mirror of the kernel's `struct open_how`.
#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))
}

fn mode_for(flags: libc::c_int) -> libc::c_uint {
    if flags & libc::O_CREAT != 0 {
        CREATE_MODE
    } else {
        0
    }
}

fn adopt(fd: libc::c_int) -> io::Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel just returned `fd` as a fresh descriptor that
    // nothing else owns, so transferring ownership to `File` is sound.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// `open(2)`: ordinary path-based open that follows symlinks unless `flags`
/// says otherwise.
pub fn open(path: &Path, flags: libc::c_int) -> io::Result<File> {
    let path = c_path(path)?;
    // SAFETY: `path` is a valid NUL-terminated string that outlives the call.
    let fd = unsafe { libc::open(path.as_ptr(), flags | libc::O_CLOEXEC, mode_for(flags)) };
    adopt(fd)
}

/// `openat(2)` relative to an open directory.
pub fn openat(dir: &File, path: &Path, flags: libc::c_int) -> io::Result<File> {
    let path = c_path(path)?;
    // SAFETY: `dir` is an open descriptor for the duration of the call and
    // `path` is a valid NUL-terminated string that outlives it.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            path.as_ptr(),
            flags | libc::O_CLOEXEC,
            mode_for(flags),
        )
    };
    adopt(fd)
}

/// `openat2(2)` relative to an open directory with the given resolve flags.
/// Returns `ENOSYS` on kernels older than 5.6.
pub fn openat2(dir: &File, path: &Path, flags: libc::c_int, resolve: u64) -> io::Result<File> {
    let path = c_path(path)?;
    let how = OpenHow {
        flags: u64::from((flags | libc::O_CLOEXEC) as u32),
        mode: u64::from(mode_for(flags)),
        resolve,
    };
    // SAFETY: `dir` is an open descriptor, `path` is a valid NUL-terminated
    // string, and `how` is a correctly sized `struct open_how`; all three
    // outlive the call.
    let ret = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            dir.as_raw_fd(),
            path.as_ptr(),
            &how as *const OpenHow,
            std::mem::size_of::<OpenHow>(),
        )
    };
    adopt(ret as libc::c_int)
}

/// Whether this kernel (and any sandbox around the process) lets us use
/// `openat2` with `RESOLVE_BENEATH`. `dir` must be an open directory.
pub fn probe_openat2(dir: &File) -> bool {
    openat2(
        dir,
        Path::new("."),
        libc::O_RDONLY | libc::O_DIRECTORY,
        RESOLVE_BENEATH,
    )
    .is_ok()
}

/// Identity of the object behind an open descriptor (`fstat`).
pub fn file_id(file: &File) -> io::Result<FileId> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}

/// Identity of the object a path names, without following a final symlink
/// (`lstat`).
pub fn path_id(path: &Path) -> io::Result<FileId> {
    let meta = std::fs::symlink_metadata(path)?;
    Ok((meta.dev(), meta.ino()))
}

/// Kernel release string from `uname(2)`.
pub fn kernel_release() -> String {
    // SAFETY: `utsname` is plain data for which all-zero bytes are valid.
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    // SAFETY: `uts` is a valid, writable `utsname`.
    if unsafe { libc::uname(&mut uts) } != 0 {
        return "unknown".to_string();
    }
    // SAFETY: on success the kernel NUL-terminates every `utsname` field.
    unsafe { CStr::from_ptr(uts.release.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch;
    use std::fs;
    use std::io::Read;

    fn fixture() -> (tempfile::TempDir, File) {
        let tmp = scratch();
        fs::create_dir(tmp.path().join("root")).unwrap();
        fs::write(tmp.path().join("root/inside.txt"), b"inside").unwrap();
        fs::write(tmp.path().join("sentinel.txt"), b"outside").unwrap();
        let root = open(&tmp.path().join("root"), libc::O_RDONLY | libc::O_DIRECTORY).unwrap();
        (tmp, root)
    }

    #[test]
    fn open_reads_a_file() {
        let (tmp, _root) = fixture();
        let mut file = open(&tmp.path().join("root/inside.txt"), libc::O_RDONLY).unwrap();
        let mut text = String::new();
        file.read_to_string(&mut text).unwrap();
        assert_eq!(text, "inside");
    }

    #[test]
    fn openat_nofollow_refuses_a_symlink() {
        let (tmp, root) = fixture();
        std::os::unix::fs::symlink("../sentinel.txt", tmp.path().join("root/leak")).unwrap();
        let err = openat(&root, Path::new("leak"), libc::O_RDONLY | libc::O_NOFOLLOW).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::ELOOP));
    }

    #[test]
    fn openat2_beneath_blocks_escapes() {
        let (tmp, root) = fixture();
        if !probe_openat2(&root) {
            eprintln!("openat2 unavailable here; skipping");
            return;
        }
        std::os::unix::fs::symlink("../sentinel.txt", tmp.path().join("root/leak")).unwrap();

        assert!(openat2(
            &root,
            Path::new("inside.txt"),
            libc::O_RDONLY,
            RESOLVE_BENEATH
        )
        .is_ok());
        for escape in ["../sentinel.txt", "leak"] {
            let err =
                openat2(&root, Path::new(escape), libc::O_RDONLY, RESOLVE_BENEATH).unwrap_err();
            assert_eq!(err.raw_os_error(), Some(libc::EXDEV), "{escape}");
        }
    }

    #[test]
    fn handle_and_path_identity_agree_and_survive_a_rename() {
        let (tmp, _root) = fixture();
        let path = tmp.path().join("root/inside.txt");
        let file = open(&path, libc::O_RDONLY).unwrap();
        let before = file_id(&file).unwrap();
        assert_eq!(before, path_id(&path).unwrap());

        let moved = tmp.path().join("root/moved.txt");
        fs::rename(&path, &moved).unwrap();
        assert_eq!(file_id(&file).unwrap(), before);
        assert_eq!(path_id(&moved).unwrap(), before);
    }

    #[test]
    fn path_id_does_not_follow_a_final_symlink() {
        let (tmp, _root) = fixture();
        let link = tmp.path().join("root/leak");
        std::os::unix::fs::symlink("../sentinel.txt", &link).unwrap();
        assert_ne!(
            path_id(&link).unwrap(),
            path_id(&tmp.path().join("sentinel.txt")).unwrap()
        );
    }

    #[test]
    fn interior_nul_is_rejected() {
        let err = open(Path::new("a\0b"), libc::O_RDONLY).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::EINVAL));
    }

    #[test]
    fn kernel_release_is_not_empty() {
        assert!(!kernel_release().is_empty());
    }
}
