//! Benchmark adapter: walk from the root handle one component at a time.

use std::path::Component;

use super::{failure, unavailable, Adapter, Fallback, Outcome, Plan, Request, Stop};
use crate::env::Env;

/// Starts from the root directory handle and opens each directory component
/// relative to the previous handle with `O_NOFOLLOW`, keeping the handles on
/// a stack so that `..` returns to a directory it actually came through.
/// The final component is opened relative to the last handle, again with
/// `O_NOFOLLOW`. It never follows a symlink, so it also refuses in-root
/// symlinks that would have been harmless.
pub struct DirfdRelative;

fn refuse(reason: &'static str) -> Stop {
    Stop::Done(Outcome::Refused(reason))
}

impl Adapter for DirfdRelative {
    fn name(&self) -> &'static str {
        "dirfd-relative"
    }

    fn strategy(&self) -> &'static str {
        "openat each component from the root handle with O_NOFOLLOW; never follow a symlink"
    }

    fn contract_version(&self) -> u32 {
        1
    }

    fn fallback(&self) -> Fallback {
        Fallback::LegacyWhenEnabled
    }

    fn prepare(&self, env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
        let mut components: Vec<Component<'_>> = request.path.components().collect();
        let leaf = match components.pop() {
            Some(Component::Normal(name)) => name.to_os_string(),
            _ => return Err(refuse("request does not end in a file name")),
        };
        let root = env
            .root_dir()
            .try_clone()
            .map_err(|err| Stop::Done(failure(&err)))?;
        let mut stack = vec![root];
        for component in components {
            match component {
                Component::CurDir => {}
                Component::RootDir | Component::Prefix(_) => {
                    return Err(refuse("request is an absolute path"));
                }
                Component::ParentDir => {
                    if stack.len() == 1 {
                        return Err(refuse("request climbs above the root"));
                    }
                    stack.pop();
                }
                Component::Normal(name) => {
                    let top = stack.last().expect("the stack always holds the root");
                    match env.open_dir_at(top, name) {
                        Ok(dir) => stack.push(dir),
                        Err(err) if unavailable(&err) => return Err(Stop::Unavailable),
                        // O_NOFOLLOW reports a symlink as ELOOP, or as
                        // ENOTDIR when combined with O_DIRECTORY.
                        Err(err)
                            if matches!(err.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR)) =>
                        {
                            return Err(refuse(
                                "a component is not a directory reachable without following a link",
                            ));
                        }
                        Err(err) => return Err(Stop::Done(failure(&err))),
                    }
                }
            }
        }
        let dir = stack.pop().expect("the stack always holds the root");
        Ok(Plan::At { dir, leaf })
    }
}
