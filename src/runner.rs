//! The trusted harness: builds a fixture, runs one adapter on one scenario,
//! observes what happened, and hands the oracle a record.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::adapters::{self, Adapter, Outcome, Request};
use crate::env::{Barrier, Env, Hooks};
use crate::error::{Context, Error};
use crate::fixture::{canary_object, Fixture, Graph};
use crate::model::{DepthRegion, Family, OpKind, Reported, RunOutcome};
use crate::oracle::{self, Judgement, Truth};
use crate::record::{OperationRecord, RequestPath};
use crate::rng;
use crate::scenario::{Mutation, Params, Scenario};
use crate::stats::stream_for;
use crate::sys;

/// One evaluated run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunRow {
    pub family: Family,
    pub variant: &'static str,
    pub region: DepthRegion,
    pub operation: OpKind,
    #[serde(skip)]
    pub params: Params,
    pub record: OperationRecord,
    pub judgement: Judgement,
    /// For runs with the unavailability fault and fallback permitted: the
    /// outcome of the same scenario and adapter with the fault removed.
    pub twin_outcome: Option<RunOutcome>,
}

impl RunRow {
    /// Whether this run took the fallback path and ended in a violation
    /// that its fault-free twin did not.
    pub fn fallback_drift(&self) -> bool {
        self.record.path_label.is_fallback()
            && self.judgement.outcome == RunOutcome::ContainmentViolation
            && self.twin_outcome != Some(RunOutcome::ContainmentViolation)
    }
}

/// Applies scheduled mutations at barriers and keeps the fixture states the
/// oracle will be shown.
struct Harness<'a> {
    fixture: &'a mut Fixture,
    pending: Vec<Option<&'a Mutation>>,
    states: Vec<Graph>,
    fired: Vec<usize>,
    error: Option<String>,
}

impl Hooks for Harness<'_> {
    fn at(&mut self, barrier: Barrier) {
        for index in 0..self.pending.len() {
            if !self.pending[index].is_some_and(|m| m.at == barrier) {
                continue;
            }
            let Some(mutation) = self.pending[index].take() else {
                continue;
            };
            for step in &mutation.steps {
                if let Err(err) = self.fixture.apply(step) {
                    self.error.get_or_insert(err);
                    return;
                }
            }
            self.states.push(self.fixture.graph().clone());
            self.fired.push(index);
        }
    }
}

/// One temporary directory in which any number of runs execute, each in
/// its own workspace, one at a time.
pub struct Session {
    temp: tempfile::TempDir,
    top: PathBuf,
    arena: PathBuf,
    next: u64,
    openat2_supported: bool,
}

impl Session {
    /// Create the temporary directory under `parent`, which is created if
    /// missing. Nothing is ever placed in the system temporary directory.
    pub fn open(parent: &Path) -> Result<Self, Error> {
        fs::create_dir_all(parent).context("creating the temporary parent")?;
        let temp = tempfile::Builder::new()
            .prefix("policycontinuity-")
            .tempdir_in(parent)
            .context("creating the temporary directory")?;
        // Workspaces live two levels below the temporary directory.
        // Validation already proves no fixture can leave its workspace; the
        // extra level is a margin.
        let top = fs::canonicalize(temp.path()).context("resolving the temporary directory")?;
        let arena = top.join("arena");
        fs::create_dir(&arena).context("creating the arena")?;
        let arena_dir =
            sys::open(&arena, libc::O_RDONLY | libc::O_DIRECTORY).context("opening the arena")?;
        let openat2_supported = sys::probe_openat2(&arena_dir);
        Ok(Self {
            temp,
            top,
            arena,
            next: 0,
            openat2_supported,
        })
    }

    /// Whether the running kernel offers `openat2(RESOLVE_BENEATH)`.
    pub fn openat2_supported(&self) -> bool {
        self.openat2_supported
    }

    /// Run one adapter on one scenario and label the result.
    pub fn run(&mut self, scenario: &Scenario, adapter: &dyn Adapter) -> Result<RunRow, Error> {
        scenario
            .validate()
            .map_err(|reason| Error::InvalidScenario {
                id: scenario.id.clone(),
                reason,
            })?;
        let mut row = self.execute(scenario, adapter)?;
        if scenario.faults.nonlegacy_unavailable && scenario.contract.fallback_enabled {
            let mut twin = scenario.clone();
            twin.faults.nonlegacy_unavailable = false;
            twin.id.push_str("~twin");
            row.twin_outcome = Some(self.execute(&twin, adapter)?.judgement.outcome);
        }
        Ok(row)
    }

    fn execute(&mut self, scenario: &Scenario, adapter: &dyn Adapter) -> Result<RunRow, Error> {
        let base = self.arena.join(format!("case-{:07}", self.next));
        self.next += 1;
        let fixture_error = |reason: String| Error::Fixture {
            id: scenario.id.clone(),
            reason,
        };
        let nonce = rng::mix(
            scenario.seed,
            stream_for(&format!("{}|{}", scenario.id, adapter.name())),
        );
        let mut fixture = Fixture::create(base, nonce).context("creating a case workspace")?;
        for step in &scenario.setup {
            fixture.apply(step).map_err(fixture_error)?;
        }

        let request_path = match &scenario.request {
            RequestPath::Relative(text) => PathBuf::from(text),
            RequestPath::AbsoluteInWorkspace(loc) => fixture.path(loc),
        };
        let root = fixture.root_path();
        let initial = fixture.graph().clone();
        let mut harness = Harness {
            fixture: &mut fixture,
            pending: scenario.schedule.iter().map(Some).collect(),
            states: vec![initial],
            fired: Vec::new(),
            error: None,
        };
        let (outcome, trace) = {
            let env = Env::new(root, scenario.contract, scenario.faults, &mut harness)
                .context("opening the approved root")?;
            let request = Request {
                path: &request_path,
                op: scenario.op,
            };
            let outcome = adapters::perform(adapter, &env, &request);
            (outcome, env.into_trace())
        };
        let Harness {
            states,
            fired,
            error,
            ..
        } = harness;

        let (reported, reason, handle) = match outcome {
            Outcome::Completed(file) => (Reported::Completed, "", Some(file)),
            Outcome::Refused(reason) => (Reported::Refused, reason, None),
            Outcome::Failed(reason) => (Reported::Failed, reason, None),
            Outcome::Unsupported(reason) => (Reported::Unsupported, reason, None),
        };

        // What the harness can see for itself about a returned handle:
        // which object it is, and which object I/O through it reaches.
        let mut handle_identity = None;
        let mut identity_matches = 0;
        let mut handle_object = None;
        let mut effect_object = None;
        if let Some(mut file) = handle {
            if let Ok(identity) = sys::file_id(&file) {
                let matches = fixture.objects_with_identity(identity);
                handle_identity = Some(identity);
                identity_matches = matches.len();
                if let [only] = matches[..] {
                    handle_object = Some(only);
                }
            }
            effect_object = match scenario.op {
                OpKind::Read => {
                    let mut content = Vec::new();
                    file.read_to_end(&mut content)
                        .ok()
                        .and_then(|_| canary_object(&content))
                        .filter(|id| *id < fixture.graph().len() && content == fixture.canary(*id))
                }
                OpKind::Write => {
                    let written = file.write_all(&fixture.write_marker()).is_ok();
                    drop(file);
                    match fixture.files_holding_marker()[..] {
                        [only] if written => Some(only),
                        _ => None,
                    }
                }
            };
        }

        let record = OperationRecord {
            scenario_id: scenario.id.clone(),
            seed: scenario.seed,
            root_object: Graph::ROOT,
            requested_path: scenario.request.display(),
            adapter: adapter.name().to_string(),
            adapter_contract_version: adapter.contract_version(),
            retry_budget: scenario.contract.retry_budget,
            fallback_enabled: scenario.contract.fallback_enabled,
            schedule: scenario.schedule_label(),
            mutations_fired: fired,
            reported,
            reason: reason.to_string(),
            path_label: trace.path_label(),
            primitive_calls: trace.calls,
            transient_injected: trace.transient_injected,
            unavailable_observed: trace.unavailable_hits > 0,
            handle_identity,
            identity_matches,
            handle_object,
            effect_object,
            sentinels_modified: fixture.sentinels_modified(),
            registry_stable: fixture.registry_stable(),
            harness_error: error,
        };
        let truth = Truth {
            states: &states,
            request: &scenario.request,
        };
        let judgement = oracle::judge(&truth, &record);

        fixture.destroy().context("removing a case workspace")?;
        self.check_clean()?;

        Ok(RunRow {
            family: scenario.family,
            variant: scenario.variant,
            region: scenario.region,
            operation: scenario.op,
            params: scenario.params,
            record,
            judgement,
            twin_outcome: None,
        })
    }

    /// After every case the temporary directory must hold exactly the empty
    /// arena. Anything else means something was written outside a workspace.
    fn check_clean(&self) -> Result<(), Error> {
        let names = |dir: &Path| -> Result<Vec<String>, Error> {
            let mut names = Vec::new();
            for entry in fs::read_dir(dir).context("checking the temporary directory")? {
                let entry = entry.context("checking the temporary directory")?;
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
            Ok(names)
        };
        let leftover = names(&self.arena)?;
        if !leftover.is_empty() {
            return Err(Error::Integrity(format!(
                "arena not empty after a case: {leftover:?}"
            )));
        }
        let top_level = names(&self.top)?;
        if top_level != ["arena"] {
            return Err(Error::Integrity(format!(
                "unexpected entries beside the arena: {top_level:?}"
            )));
        }
        Ok(())
    }

    /// Remove the temporary directory.
    pub fn close(self) -> Result<(), Error> {
        self.temp
            .close()
            .context("removing the temporary directory")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Condition, PathLabel, Split};
    use crate::scenario::generate;
    use crate::testutil::scratch_parent;

    fn find(seed: u64, family: Family, variant: &str) -> Scenario {
        generate(seed, Split::Development)
            .into_iter()
            .find(|s| s.family == family && s.variant == variant)
            .unwrap_or_else(|| panic!("no {family}/{variant} for seed {seed}"))
    }

    fn outcomes(scenario: &Scenario) -> Vec<(String, RunOutcome)> {
        let mut session = Session::open(&scratch_parent()).unwrap();
        let rows: Vec<_> = adapters::all()
            .iter()
            .map(|a| {
                let row = session.run(scenario, a.as_ref()).unwrap();
                (row.record.adapter.clone(), row.judgement.outcome)
            })
            .collect();
        session.close().unwrap();
        rows
    }

    #[test]
    fn an_ordinary_request_is_preserved_by_every_adapter() {
        let scenario = find(1, Family::NestedInRoot, "plain");
        for (adapter, outcome) in outcomes(&scenario) {
            assert_eq!(outcome, RunOutcome::Preserved, "{adapter}");
        }
    }

    #[test]
    fn a_static_outside_link_separates_the_adapters() {
        let scenario = find(1, Family::SymlinkOutside, "relative-link");
        let got = outcomes(&scenario);
        let expect = [
            ("lexical-prefix", RunOutcome::ContainmentViolation),
            ("canonicalize-then-open", RunOutcome::SafeRefusal),
            ("dirfd-relative", RunOutcome::SafeRefusal),
            ("strict-safe-open", RunOutcome::SafeRefusal),
        ];
        for ((adapter, outcome), (name, expected)) in got.iter().zip(expect) {
            assert_eq!(adapter, name);
            assert_eq!(*outcome, expected, "{adapter}");
        }
    }

    #[test]
    fn the_record_carries_identity_evidence_and_no_host_path() {
        let scenario = find(1, Family::SymlinkOutside, "relative-link");
        let mut session = Session::open(&scratch_parent()).unwrap();
        let adapter = adapters::by_name("lexical-prefix").unwrap();
        let row = session.run(&scenario, adapter.as_ref()).unwrap();
        let top = session.top.display().to_string();
        session.close().unwrap();

        assert!(row.record.handle_identity.is_some());
        assert_eq!(row.record.identity_matches, 1);
        assert_eq!(row.record.handle_object, row.record.effect_object);
        assert!(row.record.registry_stable);
        assert_eq!(row.judgement.condition, Condition::Unsafe);
        assert_eq!(row.record.path_label, PathLabel::Primary);

        let json = serde_json::to_string(&row).unwrap();
        assert!(!json.contains(&top), "{json}");
        assert!(!json.contains("/home"), "{json}");
        assert!(!json.contains("handle_identity"), "{json}");
    }

    #[test]
    fn a_write_violation_is_seen_on_the_sentinel_itself() {
        let scenario = generate(1, Split::Development)
            .into_iter()
            .chain(generate(2, Split::Development))
            .chain(generate(3, Split::Development))
            .find(|s| s.family == Family::SymlinkOutside && s.op == OpKind::Write)
            .expect("a write scenario with an outside link among the first seeds");
        let mut session = Session::open(&scratch_parent()).unwrap();
        let adapter = adapters::by_name("lexical-prefix").unwrap();
        let row = session.run(&scenario, adapter.as_ref()).unwrap();
        session.close().unwrap();
        assert_eq!(row.judgement.outcome, RunOutcome::ContainmentViolation);
        assert_eq!(row.record.sentinels_modified.len(), 1);
        assert_eq!(row.record.effect_object, row.record.handle_object);
        assert_eq!(
            row.record.sentinels_modified[0],
            row.record.handle_object.unwrap()
        );
    }

    #[test]
    fn fallback_runs_get_a_twin_and_strict_safe_open_reports_unsupported() {
        let scenario = find(1, Family::CompatFallback, "outside-link");
        let mut session = Session::open(&scratch_parent()).unwrap();
        let mut by_adapter = std::collections::BTreeMap::new();
        for adapter in adapters::all() {
            let row = session.run(&scenario, adapter.as_ref()).unwrap();
            by_adapter.insert(row.record.adapter.clone(), row);
        }
        session.close().unwrap();

        let strict = &by_adapter["strict-safe-open"];
        assert_eq!(strict.judgement.outcome, RunOutcome::UnsupportedSafe);
        assert_eq!(strict.record.path_label, PathLabel::Primary);
        assert!(!strict.fallback_drift());

        for name in ["canonicalize-then-open", "dirfd-relative"] {
            let row = &by_adapter[name];
            assert_eq!(row.record.path_label, PathLabel::Fallback, "{name}");
            assert_eq!(
                row.judgement.outcome,
                RunOutcome::ContainmentViolation,
                "{name}"
            );
            assert_eq!(row.twin_outcome, Some(RunOutcome::SafeRefusal), "{name}");
            assert!(row.fallback_drift(), "{name}");
        }

        // lexical-prefix never meets the fault: no fallback, so no drift,
        // although it violates here exactly as its twin does.
        let lexical = &by_adapter["lexical-prefix"];
        assert_eq!(lexical.record.path_label, PathLabel::Primary);
        assert_eq!(lexical.twin_outcome, Some(RunOutcome::ContainmentViolation));
        assert!(!lexical.fallback_drift());
    }

    #[test]
    fn an_invalid_scenario_is_refused_before_anything_is_created() {
        let mut scenario = find(1, Family::NestedInRoot, "plain");
        scenario.request = RequestPath::Relative("../../escape".to_string());
        let mut session = Session::open(&scratch_parent()).unwrap();
        let adapter = adapters::by_name("lexical-prefix").unwrap();
        let err = session.run(&scenario, adapter.as_ref()).unwrap_err();
        assert!(matches!(err, Error::InvalidScenario { .. }), "{err}");
        assert_eq!(fs::read_dir(&session.arena).unwrap().count(), 0);
        session.close().unwrap();
    }

    #[test]
    fn a_session_removes_its_directory_on_close() {
        let parent = scratch_parent().join("close-check");
        let session = Session::open(&parent).unwrap();
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
        session.close().unwrap();
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
    }
}
