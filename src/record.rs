//! The operation-result record: the only description of a run the oracle
//! is given.
//!
//! The harness fills it in from what it observed directly: the handle the
//! adapter returned (identified with `fstat`), the contents read or written
//! through that handle, the state of the sentinels afterwards, and the
//! primitive calls it counted. The adapter contributes only its reported
//! outcome and a fixed reason string. No field holds a filesystem path
//! outside the temporary workspace; the request is shown relative to the
//! root, or with a `<workspace>` placeholder.

use serde::Serialize;

use crate::fixture::{Loc, ObjId, WORKSPACE_TOKEN};
use crate::model::{PathLabel, Primitive, Reported};
use crate::sys::FileId;

/// The untrusted path handed to the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestPath {
    /// Relative text, interpreted against the approved root.
    Relative(String),
    /// An absolute path to a workspace location, built at runtime.
    AbsoluteInWorkspace(Loc),
}

impl RequestPath {
    /// Stable, host-independent rendering.
    pub fn display(&self) -> String {
        match self {
            RequestPath::Relative(text) => text.clone(),
            RequestPath::AbsoluteInWorkspace(loc) => {
                format!("{WORKSPACE_TOKEN}/{}", loc.display())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperationRecord {
    pub scenario_id: String,
    pub seed: u64,
    /// Tracked id of the approved root object.
    pub root_object: ObjId,
    pub requested_path: String,
    pub adapter: String,
    pub adapter_contract_version: u32,
    pub retry_budget: u32,
    pub fallback_enabled: bool,
    /// The declared mutation schedule and injected faults.
    pub schedule: String,
    /// Indices of the schedule entries that actually fired.
    pub mutations_fired: Vec<usize>,

    /// The adapter's own report.
    pub reported: Reported,
    pub reason: String,

    /// Branch taken, from the harness's call trace.
    pub path_label: PathLabel,
    pub primitive_calls: Vec<Primitive>,
    pub transient_injected: u32,
    /// Whether a primitive actually reported unavailable during the run.
    pub unavailable_observed: bool,

    /// (device, inode) of the returned handle. Used for matching and shown
    /// by `inspect`, but never written to result files: inode numbers are
    /// not reproducible.
    #[serde(skip)]
    pub handle_identity: Option<FileId>,
    /// How many tracked objects have that identity. Exactly one is required.
    pub identity_matches: usize,
    /// The tracked object the handle is, when the match is unique.
    pub handle_object: Option<ObjId>,
    /// The tracked object the I/O through the handle actually reached: the
    /// object named by the canary a read returned, or the single file that
    /// holds the write marker afterwards.
    pub effect_object: Option<ObjId>,
    /// Outside files that no longer hold their canary.
    pub sentinels_modified: Vec<ObjId>,
    /// Whether every tracked object still had its recorded identity and
    /// location after the run.
    pub registry_stable: bool,
    /// Set when the harness could not apply a scheduled mutation.
    pub harness_error: Option<String>,
}
