//! PolicyContinuity: a controlled Linux benchmark that measures whether a
//! filesystem-containment policy survives state transitions, retries, and
//! compatibility fallbacks, and when an implementation refuses instead.
//!
//! Everything runs inside temporary directories the harness creates under a
//! parent the caller names. "Outside the approved root" always means a
//! synthetic sentinel inside that same temporary workspace.
//!
//! Module boundaries that the evaluation depends on:
//!
//! * [`adapters`] sees only [`env`]: no fixture ids, labels, or expectations;
//! * [`oracle`] is pure: it sees the fixture model and an operation record,
//!   and no adapter, environment, or filesystem code;
//! * [`runner`] is the trusted harness in between.

#[cfg(not(target_os = "linux"))]
compile_error!("policycontinuity is a Linux-only benchmark");

pub mod adapters;
pub mod env;
pub mod error;
pub mod experiment;
pub mod figures;
pub mod fixture;
pub mod metrics;
pub mod model;
pub mod oracle;
pub mod protocol;
pub mod record;
pub mod report;
pub mod rng;
pub mod runner;
pub mod scenario;
pub mod stats;
pub mod sys;

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::PathBuf;

    /// Parent for unit-test scratch directories: inside the project's own
    /// `target/` directory, never the system temporary directory.
    pub fn scratch_parent() -> PathBuf {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/pc-tmp/unit");
        std::fs::create_dir_all(&parent).expect("create the unit-test scratch parent");
        std::fs::canonicalize(parent).expect("resolve the unit-test scratch parent")
    }

    /// A fresh scratch directory, removed when dropped.
    pub fn scratch() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("t-")
            .tempdir_in(scratch_parent())
            .expect("create a unit-test scratch directory")
    }
}
