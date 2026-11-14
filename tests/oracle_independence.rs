//! The evaluator must not depend on the adapters, and the adapters must not
//! be able to see the evaluator's hidden state.

use std::fs;
use std::path::{Path, PathBuf};

use policycontinuity::adapters::{self, Adapter, Fallback, Outcome, Plan, Request, Stop};
use policycontinuity::env::Env;
use policycontinuity::fixture::Step;
use policycontinuity::model::{
    Condition, DepthRegion, Family, Region, Reported, RunOutcome, Split,
};
use policycontinuity::runner::{RunRow, Session};
use policycontinuity::scenario::{generate, Scenario};

fn temp_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("oracle-independence")
}

fn source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Source text with comment lines removed, so that prose may mention a
/// module without tripping the scan.
fn code_only(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_oracle_source_refers_to_no_adapter_environment_or_filesystem_code() {
    let code = code_only(&source("src/oracle.rs"));
    for forbidden in [
        "crate::adapters",
        "crate::env",
        "crate::sys",
        "crate::runner",
        "crate::scenario",
        "std::fs",
        "std::path",
        "libc::",
        "unsafe {",
        "unsafe fn",
    ] {
        assert!(
            !code.contains(forbidden),
            "oracle.rs mentions `{forbidden}`"
        );
    }
}

#[test]
fn the_adapter_sources_refer_to_no_evaluator_or_fixture_code() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapters");
    let mut scanned = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let code = code_only(&fs::read_to_string(&path).unwrap());
        for forbidden in [
            "crate::fixture",
            "crate::oracle",
            "crate::scenario",
            "crate::record",
            "crate::runner",
            "crate::metrics",
            "crate::protocol",
            "crate::experiment",
            // All filesystem access goes through `Env`.
            "fs::read",
            "fs::canonicalize",
            "fs::metadata",
            "fs::symlink_metadata",
            "read_dir",
            "read_link",
            "File::open",
            "OpenOptions",
            "crate::sys",
        ] {
            assert!(
                !code.contains(forbidden),
                "{} mentions `{forbidden}`",
                path.display()
            );
        }
        scanned += 1;
    }
    assert_eq!(scanned, 6, "four adapters, the shared module, and pathutil");
}

#[test]
fn nothing_in_the_sources_reads_proc_or_the_system_temp_directory() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![dir];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let code = code_only(&fs::read_to_string(&path).unwrap());
            for forbidden in [
                "/proc",
                "/sys/",
                "temp_dir()",
                "tempdir()",
                "\"/tmp",
                "/dev/",
            ] {
                assert!(
                    !code.contains(forbidden),
                    "{} mentions `{forbidden}`",
                    path.display()
                );
            }
        }
    }
}

/// What an adapter did, as far as anything outside the evaluator can tell.
fn observable(row: &RunRow) -> (Reported, String, String, Vec<String>, u32) {
    (
        row.record.reported,
        row.record.reason.clone(),
        row.record.path_label.to_string(),
        row.record
            .primitive_calls
            .iter()
            .map(|p| p.to_string())
            .collect(),
        row.record.transient_injected,
    )
}

fn run_all(scenarios: &[Scenario]) -> Vec<RunRow> {
    let mut session = Session::open(&temp_parent()).unwrap();
    let mut rows = Vec::new();
    for scenario in scenarios {
        for adapter in adapters::all() {
            rows.push(session.run(scenario, adapter.as_ref()).unwrap());
        }
    }
    session.close().unwrap();
    rows
}

/// Changing every label the evaluator keeps about a scenario (its id, seed,
/// family, variant, and region) must not change what any adapter does: no
/// label reaches an adapter.
#[test]
fn relabelling_hidden_scenario_metadata_does_not_change_adapter_behaviour() {
    let original = generate(4, Split::Development);
    let relabelled: Vec<Scenario> = original
        .iter()
        .enumerate()
        .map(|(i, s)| Scenario {
            id: format!("relabelled-{i}"),
            seed: 999_999,
            family: Family::ALL[(s.family.index() + 5) % Family::ALL.len()],
            variant: "relabelled",
            region: DepthRegion::HeldOutDepth,
            ..s.clone()
        })
        .collect();
    let (a, b) = (run_all(&original), run_all(&relabelled));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(observable(x), observable(y), "{}", x.record.scenario_id);
        // The evaluator's own label is a function of the fixture, not of
        // the names it was given.
        assert_eq!(x.judgement, y.judgement, "{}", x.record.scenario_id);
    }
}

/// Building the outside tree first gives every object a different tracked
/// id. Adapters must behave identically, and the evaluator must reach the
/// same outcome through the renumbered objects.
#[test]
fn renumbering_hidden_object_ids_does_not_change_adapter_behaviour() {
    let original = generate(6, Split::Development);
    let renumbered: Vec<Scenario> = original
        .iter()
        .map(|s| {
            let loc_region = |step: &Step| match step {
                Step::Dir(loc) | Step::File(loc) => loc.region(),
                Step::Symlink { at, .. } => at.region(),
                Step::Rename { from, .. } => from.region(),
            };
            let (outside, inside): (Vec<Step>, Vec<Step>) = s
                .setup
                .iter()
                .cloned()
                .partition(|step| loc_region(step) == Region::Outside);
            Scenario {
                setup: outside.into_iter().chain(inside).collect(),
                ..s.clone()
            }
        })
        .collect();
    let (a, b) = (run_all(&original), run_all(&renumbered));
    let mut ids_differ = false;
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(observable(x), observable(y), "{}", x.record.scenario_id);
        assert_eq!(x.judgement, y.judgement, "{}", x.record.scenario_id);
        ids_differ |= x.record.handle_object != y.record.handle_object;
    }
    assert!(ids_differ, "the renumbering did not change any object id");
}

/// An adapter that checks nothing: it opens whatever the request names.
struct NoCheck;

impl Adapter for NoCheck {
    fn name(&self) -> &'static str {
        "test-no-check"
    }
    fn strategy(&self) -> &'static str {
        "join and open"
    }
    fn contract_version(&self) -> u32 {
        0
    }
    fn fallback(&self) -> Fallback {
        Fallback::NotApplicable
    }
    fn prepare(&self, env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
        Ok(Plan::Path(env.root().join(request.path)))
    }
}

/// An adapter whose reports are untrue: it refuses everything, calling it
/// "unsupported".
struct ClaimsUnsupported;

impl Adapter for ClaimsUnsupported {
    fn name(&self) -> &'static str {
        "test-claims-unsupported"
    }
    fn strategy(&self) -> &'static str {
        "always report unsupported"
    }
    fn contract_version(&self) -> u32 {
        0
    }
    fn fallback(&self) -> Fallback {
        Fallback::Never
    }
    fn prepare(&self, _env: &Env<'_>, _request: &Request<'_>) -> Result<Plan, Stop> {
        Err(Stop::Done(Outcome::Unsupported("not really")))
    }
}

fn first(family: Family, variant: &str) -> Scenario {
    generate(2, Split::Development)
        .into_iter()
        .find(|s| s.family == family && s.variant == variant)
        .unwrap()
}

#[test]
fn the_oracle_recognises_an_in_root_result_and_a_sentinel_result() {
    let mut session = Session::open(&temp_parent()).unwrap();

    let inside = session
        .run(&first(Family::NestedInRoot, "plain"), &NoCheck)
        .unwrap();
    assert_eq!(inside.record.reported, Reported::Completed);
    assert_eq!(inside.judgement.outcome, RunOutcome::Preserved);
    assert_eq!(inside.judgement.condition, Condition::Safe);

    // The same adapter, reporting the same "completed", on a request that
    // names a sentinel: only the observed object differs.
    let escaped = session
        .run(&first(Family::DotDot, "escape-to-sentinel"), &NoCheck)
        .unwrap();
    assert_eq!(escaped.record.reported, Reported::Completed);
    assert_eq!(escaped.judgement.outcome, RunOutcome::ContainmentViolation);
    assert_eq!(escaped.judgement.condition, Condition::Unsafe);
    assert_ne!(inside.record.handle_object, None);
    assert_eq!(escaped.record.identity_matches, 1);

    let absolute = session
        .run(&first(Family::DotDot, "absolute-sentinel"), &NoCheck)
        .unwrap();
    assert_eq!(absolute.judgement.outcome, RunOutcome::ContainmentViolation);
    assert!(!absolute.record.requested_path.starts_with('/'));
    session.close().unwrap();
}

#[test]
fn an_unbacked_unsupported_claim_is_classified_as_a_refusal() {
    let mut session = Session::open(&temp_parent()).unwrap();
    let safe = session
        .run(&first(Family::NestedInRoot, "plain"), &ClaimsUnsupported)
        .unwrap();
    assert_eq!(safe.record.reported, Reported::Unsupported);
    assert!(!safe.record.unavailable_observed);
    assert_eq!(safe.judgement.outcome, RunOutcome::FalseRefusal);

    let unsafe_run = session
        .run(
            &first(Family::SymlinkOutside, "relative-link"),
            &ClaimsUnsupported,
        )
        .unwrap();
    assert_eq!(unsafe_run.judgement.outcome, RunOutcome::SafeRefusal);
    session.close().unwrap();
}
