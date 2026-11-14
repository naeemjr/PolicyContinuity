//! The environment an adapter runs in.
//!
//! `Env` is the only thing an adapter is given besides the request. It
//! exposes the approved root, the two contract parameters every adapter
//! shares (retry budget, whether a compatibility fallback is permitted), and
//! five filesystem primitives. It carries no fixture ids, labels, or
//! expectations.
//!
//! Every primitive call is a **barrier**: before the call does anything, the
//! harness is told its ordinal number and may apply a scheduled fixture
//! mutation. Schedules are therefore defined on something every adapter does
//! and none can avoid, with no cooperation from adapter code. The harness
//! cannot interleave *inside* a primitive; see `docs/VALIDITY_THREATS.md`.

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{OpKind, PathLabel, Primitive};
use crate::sys;

/// Contract parameters, identical for every adapter in a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contract {
    /// How many times a transiently failing final open may be reissued.
    pub retry_budget: u32,
    /// Whether the deployment permits the legacy routine as a fallback when
    /// a non-legacy primitive is unavailable.
    pub fallback_enabled: bool,
}

/// Faults the harness injects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Faults {
    /// Every non-legacy primitive reports `ENOSYS`.
    pub nonlegacy_unavailable: bool,
    /// The first N file-yielding calls that are otherwise available fail
    /// with `EAGAIN` without touching the filesystem.
    pub transient_failures: u32,
}

/// A point in a run, defined only by what the harness itself observes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Barrier {
    /// Immediately before the adapter's k-th primitive call (1-based).
    BeforeCall(u32),
    /// Immediately after the n-th injected transient failure (1-based),
    /// before the adapter sees the error.
    AfterTransient(u32),
}

impl Barrier {
    pub fn label(self) -> String {
        match self {
            Barrier::BeforeCall(k) => format!("before-call-{k}"),
            Barrier::AfterTransient(n) => format!("after-transient-{n}"),
        }
    }
}

/// Implemented by the harness; invoked at every barrier.
pub trait Hooks {
    fn at(&mut self, barrier: Barrier);
}

/// What the harness observed the adapter do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trace {
    /// Every primitive call, in order.
    pub calls: Vec<Primitive>,
    /// Calls that reported the primitive unavailable (injected or real).
    pub unavailable_hits: u32,
    pub transient_injected: u32,
    /// A file-yielding call was made after an injected transient failure.
    pub retried: bool,
    /// The legacy open was called after a primitive reported unavailable.
    pub fell_back: bool,
}

impl Trace {
    pub fn path_label(&self) -> PathLabel {
        match (self.fell_back, self.retried) {
            (true, true) => PathLabel::FallbackRetry,
            (true, false) => PathLabel::Fallback,
            (false, true) => PathLabel::Retry,
            (false, false) => PathLabel::Primary,
        }
    }
}

pub struct Env<'h> {
    root: PathBuf,
    root_dir: File,
    contract: Contract,
    faults: Faults,
    transient_left: Cell<u32>,
    hooks: RefCell<&'h mut dyn Hooks>,
    trace: RefCell<Trace>,
}

fn open_flags(op: OpKind) -> libc::c_int {
    match op {
        OpKind::Read => libc::O_RDONLY,
        OpKind::Write => libc::O_WRONLY,
    }
}

fn errno(code: libc::c_int) -> io::Error {
    io::Error::from_raw_os_error(code)
}

impl<'h> Env<'h> {
    /// Opens the root directory once, the way an application would at
    /// startup. `root` must be absolute and fully resolved.
    pub fn new(
        root: PathBuf,
        contract: Contract,
        faults: Faults,
        hooks: &'h mut dyn Hooks,
    ) -> io::Result<Self> {
        let root_dir = sys::open(&root, libc::O_RDONLY | libc::O_DIRECTORY)?;
        Ok(Self {
            root,
            root_dir,
            contract,
            faults,
            transient_left: Cell::new(faults.transient_failures),
            hooks: RefCell::new(hooks),
            trace: RefCell::new(Trace::default()),
        })
    }

    /// The approved root, as an absolute, fully resolved path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A directory handle on the approved root, opened before the operation.
    pub fn root_dir(&self) -> &File {
        &self.root_dir
    }

    pub fn retry_budget(&self) -> u32 {
        self.contract.retry_budget
    }

    pub fn fallback_enabled(&self) -> bool {
        self.contract.fallback_enabled
    }

    /// Barrier and fault handling shared by every primitive.
    fn enter(&self, primitive: Primitive) -> io::Result<()> {
        let ordinal = {
            let mut trace = self.trace.borrow_mut();
            trace.calls.push(primitive);
            trace.calls.len() as u32
        };
        self.hooks.borrow_mut().at(Barrier::BeforeCall(ordinal));

        if self.faults.nonlegacy_unavailable && !primitive.is_legacy() {
            self.trace.borrow_mut().unavailable_hits += 1;
            return Err(errno(libc::ENOSYS));
        }
        if !primitive.yields_file() {
            return Ok(());
        }
        let injected = {
            let mut trace = self.trace.borrow_mut();
            if trace.transient_injected > 0 {
                trace.retried = true;
            }
            if trace.unavailable_hits > 0 && primitive.is_legacy() {
                trace.fell_back = true;
            }
            let left = self.transient_left.get();
            if left == 0 {
                None
            } else {
                self.transient_left.set(left - 1);
                trace.transient_injected += 1;
                Some(trace.transient_injected)
            }
        };
        match injected {
            None => Ok(()),
            Some(n) => {
                self.hooks.borrow_mut().at(Barrier::AfterTransient(n));
                Err(errno(libc::EAGAIN))
            }
        }
    }

    /// Record a primitive the real kernel turned out not to offer.
    fn note_unavailable<T>(&self, result: io::Result<T>) -> io::Result<T> {
        if matches!(&result, Err(err) if err.raw_os_error() == Some(libc::ENOSYS)) {
            self.trace.borrow_mut().unavailable_hits += 1;
        }
        result
    }

    /// `realpath`: resolve every symlink in `path`.
    pub fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.enter(Primitive::Canonicalize)?;
        std::fs::canonicalize(path)
    }

    /// Legacy path open; follows symlinks.
    pub fn open_path(&self, path: &Path, op: OpKind) -> io::Result<File> {
        self.enter(Primitive::OpenPath)?;
        sys::open(path, open_flags(op))
    }

    /// Open one directory component relative to `dir`, refusing a symlink.
    pub fn open_dir_at(&self, dir: &File, name: &OsStr) -> io::Result<File> {
        self.enter(Primitive::OpenDirAt)?;
        sys::openat(
            dir,
            Path::new(name),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        )
    }

    /// Open the final component relative to `dir`, refusing a symlink.
    pub fn open_file_at(&self, dir: &File, name: &OsStr, op: OpKind) -> io::Result<File> {
        self.enter(Primitive::OpenFileAt)?;
        sys::openat(dir, Path::new(name), open_flags(op) | libc::O_NOFOLLOW)
    }

    /// `openat2` with `RESOLVE_BENEATH` relative to the root handle.
    pub fn open_beneath(&self, relative: &Path, op: OpKind) -> io::Result<File> {
        self.enter(Primitive::OpenBeneath)?;
        // With RESOLVE_BENEATH the kernel reports EAGAIN when any rename on
        // the system races a `..` lookup. That is real timing, which this
        // benchmark keeps out of its results: it is absorbed here so the
        // only transient errors an adapter sees are the injected ones.
        let mut spurious = 0;
        loop {
            let result = sys::openat2(
                &self.root_dir,
                relative,
                open_flags(op),
                sys::RESOLVE_BENEATH | sys::RESOLVE_NO_MAGICLINKS,
            );
            match result {
                Err(err) if err.raw_os_error() == Some(libc::EAGAIN) && spurious < 64 => {
                    spurious += 1;
                }
                other => return self.note_unavailable(other),
            }
        }
    }

    pub fn into_trace(self) -> Trace {
        self.trace.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch;
    use std::fs;

    #[derive(Default)]
    struct Log(Vec<Barrier>);

    impl Hooks for Log {
        fn at(&mut self, barrier: Barrier) {
            self.0.push(barrier);
        }
    }

    const CONTRACT: Contract = Contract {
        retry_budget: 2,
        fallback_enabled: false,
    };

    fn root() -> (tempfile::TempDir, PathBuf) {
        let tmp = scratch();
        let root = fs::canonicalize(tmp.path()).unwrap().join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.txt"), b"a").unwrap();
        (tmp, root)
    }

    #[test]
    fn every_call_is_a_numbered_barrier() {
        let (_tmp, root) = root();
        let mut log = Log::default();
        let env = Env::new(root.clone(), CONTRACT, Faults::default(), &mut log).unwrap();
        env.canonicalize(&root.join("a.txt")).unwrap();
        env.open_path(&root.join("a.txt"), OpKind::Read).unwrap();
        let trace = env.into_trace();
        assert_eq!(trace.calls, [Primitive::Canonicalize, Primitive::OpenPath]);
        assert_eq!(trace.path_label(), PathLabel::Primary);
        assert_eq!(log.0, [Barrier::BeforeCall(1), Barrier::BeforeCall(2)]);
    }

    #[test]
    fn transient_failures_hit_file_opens_only_and_mark_a_retry() {
        let (_tmp, root) = root();
        let mut log = Log::default();
        let faults = Faults {
            transient_failures: 1,
            ..Faults::default()
        };
        let env = Env::new(root.clone(), CONTRACT, faults, &mut log).unwrap();
        env.canonicalize(&root).unwrap();
        let err = env
            .open_path(&root.join("a.txt"), OpKind::Read)
            .unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::EAGAIN));
        assert!(env.open_path(&root.join("a.txt"), OpKind::Read).is_ok());
        let trace = env.into_trace();
        assert_eq!(trace.transient_injected, 1);
        assert_eq!(trace.path_label(), PathLabel::Retry);
        assert_eq!(
            log.0,
            [
                Barrier::BeforeCall(1),
                Barrier::BeforeCall(2),
                Barrier::AfterTransient(1),
                Barrier::BeforeCall(3)
            ]
        );
    }

    #[test]
    fn unavailability_removes_every_nonlegacy_primitive_and_spends_no_transients() {
        let (_tmp, root) = root();
        let mut log = Log::default();
        let faults = Faults {
            nonlegacy_unavailable: true,
            transient_failures: 1,
        };
        let env = Env::new(root.clone(), CONTRACT, faults, &mut log).unwrap();
        let dir = env.root_dir().try_clone().unwrap();
        let enosys = Some(libc::ENOSYS);
        assert_eq!(env.canonicalize(&root).unwrap_err().raw_os_error(), enosys);
        assert_eq!(
            env.open_dir_at(&dir, OsStr::new("x"))
                .unwrap_err()
                .raw_os_error(),
            enosys
        );
        assert_eq!(
            env.open_file_at(&dir, OsStr::new("a.txt"), OpKind::Read)
                .unwrap_err()
                .raw_os_error(),
            enosys
        );
        assert_eq!(
            env.open_beneath(Path::new("a.txt"), OpKind::Read)
                .unwrap_err()
                .raw_os_error(),
            enosys
        );
        // The legacy open is still there, and is what the transient hits.
        let err = env
            .open_path(&root.join("a.txt"), OpKind::Read)
            .unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::EAGAIN));
        assert!(env.open_path(&root.join("a.txt"), OpKind::Read).is_ok());
        let trace = env.into_trace();
        assert_eq!(trace.unavailable_hits, 4);
        assert_eq!(trace.path_label(), PathLabel::FallbackRetry);
    }

    #[test]
    fn a_legacy_open_without_unavailability_is_not_a_fallback() {
        let (_tmp, root) = root();
        let mut log = Log::default();
        let env = Env::new(root.clone(), CONTRACT, Faults::default(), &mut log).unwrap();
        env.open_path(&root.join("a.txt"), OpKind::Read).unwrap();
        assert!(!env.into_trace().fell_back);
    }

    #[test]
    fn opens_neither_create_nor_truncate() {
        let (_tmp, root) = root();
        let mut log = Log::default();
        let env = Env::new(root.clone(), CONTRACT, Faults::default(), &mut log).unwrap();
        let missing = env
            .open_path(&root.join("new.txt"), OpKind::Write)
            .unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        env.open_path(&root.join("a.txt"), OpKind::Write).unwrap();
        assert_eq!(fs::read(root.join("a.txt")).unwrap(), b"a");
        assert!(!root.join("new.txt").exists());
    }
}
