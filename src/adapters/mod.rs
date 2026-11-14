//! Benchmark adapters.
//!
//! **These are benchmark adapters, not production implementations.** Each is
//! a compact stand-in for one way of enforcing "stay inside this directory",
//! written to make one design choice visible. None models any specific
//! application, and a result for an adapter says nothing about software that
//! merely uses a similar-sounding technique.
//!
//! All four share one contract:
//!
//! * the same inputs: the approved root (path and directory handle), the
//!   requested path, the operation, the retry budget, and whether the
//!   deployment permits a compatibility fallback;
//! * **preparation** is the adapter's own: it gathers whatever evidence its
//!   strategy uses and returns a [`Plan`] for the final open, or stops;
//! * **completion** is shared ([`complete`]): the plan's final open is
//!   issued, and reissued on a transient failure up to the retry budget.
//!   Preparation is not repeated on a retry, for any adapter;
//! * if a non-legacy primitive is unavailable, an adapter either reports
//!   `Unsupported` or, when its declared [`Fallback`] rule and the contract
//!   both allow, switches to the shared legacy routine ([`legacy_plan`]).
//!
//! Adapters see `Env` and `Request` and nothing else. This directory must
//! not refer to the fixture, scenario, record, or oracle modules; a test
//! enforces that.

mod canonicalize_then_open;
mod dirfd_relative;
mod lexical_prefix;
mod pathutil;
mod strict_safe_open;

use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::env::Env;
use crate::model::OpKind;

/// One request: perform `op` on the untrusted `path`, which is meant to be
/// interpreted relative to the approved root.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub path: &'a Path,
    pub op: OpKind,
}

/// What an adapter reports. Reasons are fixed strings: they never contain a
/// path.
#[derive(Debug)]
pub enum Outcome {
    /// The open succeeded; this is the handle the operation would act on.
    Completed(File),
    /// The adapter decided the request violates the policy.
    Refused(&'static str),
    /// The adapter gave up on an error it did not treat as a policy decision.
    Failed(&'static str),
    /// A required primitive is unavailable and no fallback is permitted.
    Unsupported(&'static str),
}

/// The final open an adapter has decided on.
#[derive(Debug)]
pub enum Plan {
    /// Open this absolute path with the legacy primitive.
    Path(PathBuf),
    /// Open `leaf` relative to a directory handle, without following a link.
    At { dir: File, leaf: OsString },
    /// Resolve this path beneath the root handle in the kernel.
    Beneath(PathBuf),
}

/// Why preparation or completion did not produce a handle.
#[derive(Debug)]
pub enum Stop {
    Done(Outcome),
    /// A non-legacy primitive reported that it is unavailable.
    Unavailable,
}

/// What an adapter does when a primitive it needs is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    /// Report `Unsupported`, whatever the contract permits.
    Never,
    /// Use the legacy routine if the contract permits a fallback; otherwise
    /// report `Unsupported`.
    LegacyWhenEnabled,
    /// The adapter uses only the legacy primitive.
    NotApplicable,
}

impl Fallback {
    pub fn as_str(self) -> &'static str {
        match self {
            Fallback::Never => "never",
            Fallback::LegacyWhenEnabled => "legacy-when-enabled",
            Fallback::NotApplicable => "not-applicable",
        }
    }
}

pub trait Adapter {
    /// Stable identifier used in the CLI and in reports.
    fn name(&self) -> &'static str;
    /// One-line description of the strategy it stands in for.
    fn strategy(&self) -> &'static str;
    /// Bumped whenever the adapter's observable behaviour changes.
    fn contract_version(&self) -> u32;
    fn fallback(&self) -> Fallback;
    /// Gather evidence and decide on the final open.
    fn prepare(&self, env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop>;
}

/// The four benchmark adapters, in report order.
pub fn all() -> Vec<Box<dyn Adapter>> {
    vec![
        Box::new(lexical_prefix::LexicalPrefix),
        Box::new(canonicalize_then_open::CanonicalizeThenOpen),
        Box::new(dirfd_relative::DirfdRelative),
        Box::new(strict_safe_open::StrictSafeOpen),
    ]
}

/// Adapter names of the preliminary version (v0.1.0) that no longer exist,
/// with what replaced each.
pub const RETIRED: &[(&str, &str)] = &[
    ("naive-join", "retired; not one of the four frozen adapters"),
    (
        "string-prefix",
        "retired; not one of the four frozen adapters",
    ),
    (
        "open-then-verify",
        "retired; it depended on a kernel pseudo-filesystem",
    ),
    ("beneath-strict", "renamed to strict-safe-open"),
    (
        "beneath-fallback-walk",
        "split: the walk is dirfd-relative, the preferred path is strict-safe-open",
    ),
    (
        "beneath-fallback-lexical",
        "retired; fallback is now a contract parameter, not an adapter",
    ),
];

/// Look an adapter up by name.
pub fn by_name(name: &str) -> Result<Box<dyn Adapter>, String> {
    if let Some((_, note)) = RETIRED.iter().find(|(old, _)| *old == name) {
        return Err(format!("adapter `{name}` existed only in v0.1.0 ({note})"));
    }
    all()
        .into_iter()
        .find(|a| a.name() == name)
        .ok_or_else(|| format!("unknown adapter `{name}`"))
}

fn unavailable(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::ENOSYS)
}

fn transient(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(libc::EAGAIN | libc::EINTR))
}

fn failure(err: &io::Error) -> Outcome {
    Outcome::Failed(match err.kind() {
        io::ErrorKind::NotFound => "no such file or directory",
        io::ErrorKind::PermissionDenied => "permission denied",
        _ => "unexpected error from the filesystem",
    })
}

/// The legacy routine: the lexical check followed by a plan to open by path.
/// It is the `lexical-prefix` adapter's whole strategy and the only
/// compatibility fallback in the benchmark.
fn legacy_plan(env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
    let candidate = pathutil::lexical_normalize(&env.root().join(request.path));
    if candidate.starts_with(env.root()) {
        Ok(Plan::Path(candidate))
    } else {
        Err(Stop::Done(Outcome::Refused(
            "normalised path is not beneath the root",
        )))
    }
}

/// Issue the plan's final open, reissuing it on a transient failure up to
/// the retry budget. Shared by every adapter.
fn complete(env: &Env<'_>, plan: &Plan, op: OpKind) -> Result<File, Stop> {
    let mut retries_left = env.retry_budget();
    loop {
        let result = match plan {
            Plan::Path(path) => env.open_path(path, op),
            Plan::At { dir, leaf } => env.open_file_at(dir, leaf, op),
            Plan::Beneath(relative) => env.open_beneath(relative, op),
        };
        let err = match result {
            Ok(file) => return Ok(file),
            Err(err) => err,
        };
        if transient(&err) {
            if retries_left == 0 {
                return Err(Stop::Done(Outcome::Failed(
                    "transient failure persisted past the retry budget",
                )));
            }
            retries_left -= 1;
            continue;
        }
        if unavailable(&err) {
            return Err(Stop::Unavailable);
        }
        let refused = match (plan, err.raw_os_error()) {
            (Plan::Beneath(_), Some(libc::EXDEV | libc::ELOOP)) => {
                Some("kernel refused to resolve the path beneath the root")
            }
            (Plan::At { .. }, Some(libc::ELOOP)) => Some("final component is a symbolic link"),
            _ => None,
        };
        return Err(Stop::Done(match refused {
            Some(reason) => Outcome::Refused(reason),
            None => failure(&err),
        }));
    }
}

/// Run one adapter on one request.
pub fn perform(adapter: &dyn Adapter, env: &Env<'_>, request: &Request<'_>) -> Outcome {
    let primary = adapter
        .prepare(env, request)
        .and_then(|plan| complete(env, &plan, request.op));
    match primary {
        Ok(file) => return Outcome::Completed(file),
        Err(Stop::Done(outcome)) => return outcome,
        Err(Stop::Unavailable) => {}
    }
    if adapter.fallback() != Fallback::LegacyWhenEnabled || !env.fallback_enabled() {
        return Outcome::Unsupported("a required primitive is unavailable");
    }
    let fallback = legacy_plan(env, request).and_then(|plan| complete(env, &plan, request.op));
    match fallback {
        Ok(file) => Outcome::Completed(file),
        Err(Stop::Done(outcome)) => outcome,
        // The legacy primitive is never reported unavailable.
        Err(Stop::Unavailable) => Outcome::Unsupported("the legacy primitive is unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn there_are_exactly_the_four_frozen_adapters() {
        let names: Vec<&str> = all().iter().map(|a| a.name()).collect();
        assert_eq!(
            names,
            [
                "lexical-prefix",
                "canonicalize-then-open",
                "dirfd-relative",
                "strict-safe-open"
            ]
        );
        let unique: HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 4);
        assert!(all().iter().all(|a| !a.strategy().is_empty()));
    }

    #[test]
    fn only_strict_safe_open_never_falls_back() {
        let never: Vec<&str> = all()
            .iter()
            .filter(|a| a.fallback() == Fallback::Never)
            .map(|a| a.name())
            .collect();
        assert_eq!(never, ["strict-safe-open"]);
    }

    #[test]
    fn retired_names_are_rejected_with_an_explanation() {
        let err = by_name("beneath-strict").err().unwrap();
        assert!(err.contains("strict-safe-open"), "{err}");
        assert!(by_name("open-then-verify").is_err());
        assert!(by_name("nope").err().unwrap().contains("unknown adapter"));
        assert_eq!(by_name("dirfd-relative").unwrap().name(), "dirfd-relative");
    }
}
