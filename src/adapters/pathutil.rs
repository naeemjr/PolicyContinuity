//! Purely lexical path normalisation, private to the adapters. Nothing here
//! touches the filesystem, which is exactly why a check built on it alone
//! says nothing about symlinks.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Collapse `.`, `..`, and repeated separators without consulting the
/// filesystem. `..` at the filesystem root stays at the root; a relative
/// path keeps any `..` it cannot cancel.
pub(super) fn lexical_normalize(path: &Path) -> PathBuf {
    let mut absolute = false;
    let mut leading_parents = 0;
    let mut names: Vec<OsString> = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::CurDir => {}
            Component::RootDir => {
                absolute = true;
                leading_parents = 0;
                names.clear();
            }
            Component::ParentDir => {
                if names.pop().is_none() && !absolute {
                    leading_parents += 1;
                }
            }
            Component::Normal(name) => names.push(name.to_os_string()),
        }
    }
    let mut out = PathBuf::new();
    if absolute {
        out.push("/");
    }
    for _ in 0..leading_parents {
        out.push("..");
    }
    out.extend(names);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(text: &str) -> String {
        lexical_normalize(Path::new(text)).display().to_string()
    }

    #[test]
    fn collapses_dots_and_separators() {
        assert_eq!(norm("/w/root/a/./b//c"), "/w/root/a/b/c");
        assert_eq!(norm("/w/root/a/../b"), "/w/root/b");
        assert_eq!(norm("/w/root/../root/a"), "/w/root/a");
    }

    #[test]
    fn dotdot_stops_at_the_filesystem_root_and_is_kept_when_relative() {
        assert_eq!(norm("/../../a"), "/a");
        assert_eq!(norm("../../a"), "../../a");
        assert_eq!(norm("a/../../b"), "../b");
    }
}
