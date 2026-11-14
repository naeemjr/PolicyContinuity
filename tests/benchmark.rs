//! Properties of the benchmark as a whole, on development seeds only.
//! Held-out seeds (1001 to 1040) must never appear in a test.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use policycontinuity::experiment::{artifacts, evaluate, Evaluation};
use policycontinuity::model::{
    Condition, DepthRegion, Family, OpKind, PathLabel, Reported, RunOutcome, Split,
};
use policycontinuity::protocol;
use policycontinuity::runner::RunRow;

fn temp_parent(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn development(seeds: &[u64], name: &str) -> Evaluation {
    evaluate(Split::Development, seeds, &temp_parent(name)).unwrap()
}

fn of<'a>(eval: &'a Evaluation, adapter: &'a str) -> impl Iterator<Item = &'a RunRow> {
    eval.rows
        .iter()
        .filter(move |r| r.record.adapter == adapter)
}

#[test]
fn tests_use_development_seeds_only() {
    let held_out = protocol::held_out_seeds();
    for seed in protocol::development_seeds() {
        assert!(!held_out.contains(&seed));
    }
    // The seeds this file uses.
    for seed in 1..=8 {
        assert!(protocol::development_seeds().contains(&seed));
    }
}

#[test]
fn every_run_is_labelled_and_none_is_inconclusive() {
    let eval = development(&[1, 2, 3, 4], "labelled");
    assert_eq!(eval.rows.len(), 4 * 12 * 8 * 4);
    for row in &eval.rows {
        assert_ne!(
            row.judgement.outcome,
            RunOutcome::EvaluatorInconclusive,
            "{} / {}: {}",
            row.record.scenario_id,
            row.record.adapter,
            row.judgement.basis
        );
        assert_ne!(row.judgement.condition, Condition::Undetermined);
        assert!(row.record.registry_stable);
        assert_eq!(row.record.harness_error, None);
        assert_eq!(row.region, DepthRegion::InDistribution);
    }
}

#[test]
fn outcomes_are_consistent_with_what_the_harness_observed() {
    let eval = development(&[1, 2, 3, 4], "consistent");
    for row in &eval.rows {
        let (r, j) = (&row.record, &row.judgement);
        let context = format!("{} / {}", r.scenario_id, r.adapter);
        match j.outcome {
            RunOutcome::Preserved | RunOutcome::ContainmentViolation => {
                assert_eq!(r.reported, Reported::Completed, "{context}");
                assert_eq!(r.identity_matches, 1, "{context}");
                assert_eq!(r.handle_object, r.effect_object, "{context}");
            }
            RunOutcome::SafeRefusal => {
                assert_eq!(r.reported, Reported::Refused, "{context}");
                assert_eq!(j.condition, Condition::Unsafe, "{context}");
            }
            RunOutcome::FalseRefusal => {
                assert_eq!(r.reported, Reported::Refused, "{context}");
                assert_eq!(j.condition, Condition::Safe, "{context}");
            }
            RunOutcome::UnsupportedSafe => {
                assert_eq!(r.reported, Reported::Unsupported, "{context}");
                assert!(r.unavailable_observed, "{context}");
            }
            RunOutcome::OrdinaryFailure => assert_eq!(r.reported, Reported::Failed, "{context}"),
            RunOutcome::EvaluatorInconclusive => panic!("{context}: inconclusive"),
        }
        // A violation is only possible when the request denoted a sentinel
        // at some point, and a sentinel is modified exactly when a write
        // violated.
        if j.outcome == RunOutcome::ContainmentViolation {
            assert_eq!(j.condition, Condition::Unsafe, "{context}");
        }
        let wrote_outside =
            j.outcome == RunOutcome::ContainmentViolation && row.operation == OpKind::Write;
        assert_eq!(!r.sentinels_modified.is_empty(), wrote_outside, "{context}");
        if r.reported != Reported::Completed {
            assert_eq!(r.handle_object, None, "{context}");
        }
    }
}

#[test]
fn path_labels_follow_from_the_faults_that_were_injected() {
    let eval = development(&[1, 2, 3], "labels");
    for row in &eval.rows {
        let r = &row.record;
        let context = format!("{} / {}", r.scenario_id, r.adapter);
        if r.path_label.is_fallback() {
            assert!(r.unavailable_observed && r.fallback_enabled, "{context}");
            assert_ne!(r.adapter, "strict-safe-open", "{context}");
            assert_ne!(r.adapter, "lexical-prefix", "{context}");
        }
        if r.path_label.is_retry() {
            assert!(r.transient_injected > 0, "{context}");
        }
        if r.transient_injected == 0 && !r.unavailable_observed {
            assert_eq!(r.path_label, PathLabel::Primary, "{context}");
        }
        assert!(
            r.transient_injected <= protocol::MAX_TRANSIENT_FAILURES,
            "{context}"
        );
        // A twin exists exactly for the compat-fallback family.
        assert_eq!(
            row.twin_outcome.is_some(),
            row.family == Family::CompatFallback,
            "{context}"
        );
        // Unavailability is only ever met by adapters that use a
        // non-legacy primitive.
        if r.adapter == "lexical-prefix" {
            assert!(!r.unavailable_observed, "{context}");
        }
    }
}

#[test]
fn every_adapter_sees_the_same_scenarios_with_the_same_contract() {
    let eval = development(&[5], "identical");
    let mut by_scenario: BTreeMap<&str, Vec<&RunRow>> = BTreeMap::new();
    for row in &eval.rows {
        by_scenario
            .entry(row.record.scenario_id.as_str())
            .or_default()
            .push(row);
    }
    assert_eq!(by_scenario.len(), 96);
    for (id, rows) in by_scenario {
        assert_eq!(rows.len(), 4, "{id}");
        let first = &rows[0].record;
        for row in &rows {
            let r = &row.record;
            assert_eq!(r.requested_path, first.requested_path, "{id}");
            assert_eq!(r.schedule, first.schedule, "{id}");
            assert_eq!(r.retry_budget, first.retry_budget, "{id}");
            assert_eq!(r.fallback_enabled, first.fallback_enabled, "{id}");
            assert_eq!(row.operation, rows[0].operation, "{id}");
        }
    }
}

#[test]
fn controls_with_no_link_and_no_fault_are_preserved_by_every_adapter() {
    let eval = development(&[1, 2, 3, 4, 5, 6], "controls");
    let mut checked = 0;
    for row in &eval.rows {
        let plain = row.family == Family::NestedInRoot
            || (row.family == Family::BenignControl && row.variant != "in-root-retarget")
            || (row.family == Family::DotDot
                && matches!(row.variant, "inner-dotdot" | "dot-segments"));
        if plain {
            assert_eq!(
                row.judgement.outcome,
                RunOutcome::Preserved,
                "{} / {}",
                row.record.scenario_id,
                row.record.adapter
            );
            checked += 1;
        }
    }
    assert!(checked > 300, "{checked}");
}

/// Facts about the kernel primitives that the write-up relies on. These are
/// checked on development seeds; the frozen predictions P1 to P3 restate
/// them for the held-out run.
#[test]
fn development_runs_match_the_documented_kernel_behaviour() {
    let eval = development(&[1, 2, 3, 4, 5, 6, 7, 8], "kernel");
    if !eval.openat2_supported {
        eprintln!("openat2 is unavailable here; skipping");
        return;
    }
    let violations = |adapter: &str, keep: &dyn Fn(&RunRow) -> bool| {
        of(&eval, adapter)
            .filter(|r| keep(r))
            .filter(|r| r.judgement.outcome == RunOutcome::ContainmentViolation)
            .count()
    };
    assert_eq!(violations("strict-safe-open", &|_| true), 0);
    assert_eq!(
        violations("dirfd-relative", &|r| !r.record.path_label.is_fallback()),
        0
    );
    assert_eq!(
        violations("canonicalize-then-open", &|r| {
            r.record.path_label == PathLabel::Primary && r.record.mutations_fired.is_empty()
        }),
        0
    );
    // The benchmark is not vacuous: the path-string adapters do violate.
    assert!(violations("lexical-prefix", &|_| true) > 50);
    assert!(violations("canonicalize-then-open", &|_| true) > 20);
    // And strictness has a cost: in-root links written as absolute paths
    // are refused by RESOLVE_BENEATH.
    let refused_absolute = of(&eval, "strict-safe-open")
        .filter(|r| r.family == Family::SymlinkInRoot && r.variant == "absolute-link")
        .all(|r| r.judgement.outcome == RunOutcome::FalseRefusal);
    assert!(refused_absolute);
    let allowed_relative = of(&eval, "strict-safe-open")
        .filter(|r| r.family == Family::SymlinkInRoot && r.variant == "relative-link")
        .all(|r| r.judgement.outcome == RunOutcome::Preserved);
    assert!(allowed_relative);
}

#[test]
fn the_lexical_policy_is_what_lexical_prefix_enforces() {
    let eval = development(&[1, 2, 3], "lexical");
    for row in of(&eval, "lexical-prefix") {
        assert_ne!(
            row.judgement.lexical_outcome,
            RunOutcome::ContainmentViolation,
            "{}",
            row.record.scenario_id
        );
        assert_ne!(row.judgement.lexical_outcome, RunOutcome::FalseRefusal);
    }
    // Under the object policy the same adapter does violate.
    assert!(of(&eval, "lexical-prefix")
        .any(|r| r.judgement.outcome == RunOutcome::ContainmentViolation));
}

#[test]
fn a_run_leaves_its_surroundings_untouched_and_its_temp_parent_empty() {
    let area = temp_parent("confinement");
    let _ = fs::remove_dir_all(&area);
    let parent = area.join("parent");
    fs::create_dir_all(&parent).unwrap();
    // Decoys beside the temp parent, with names fixtures also use.
    let decoys = [
        ("root", "decoy-root-file"),
        ("outside", "decoy-outside-file"),
        ("sentinel.dat", "decoy-sentinel"),
    ];
    for (name, content) in decoys {
        fs::write(area.join(name), content).unwrap();
    }
    let listing = |dir: &PathBuf| {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let before = listing(&area);

    let eval = evaluate(Split::Development, &[1, 2], &parent).unwrap();
    assert!(eval
        .rows
        .iter()
        .any(|r| r.judgement.outcome == RunOutcome::ContainmentViolation));

    assert_eq!(listing(&area), before);
    for (name, content) in decoys {
        assert_eq!(fs::read_to_string(area.join(name)).unwrap(), content);
    }
    assert!(
        listing(&parent).is_empty(),
        "the temp parent was not cleaned up"
    );
}

#[test]
fn artifacts_are_complete_and_free_of_host_details() {
    let eval = development(&[1, 2], "artifacts");
    let files = artifacts(&eval);
    let manifest = files.iter().find(|a| a.path == "manifest.json").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&manifest.content).unwrap();
    assert_eq!(parsed["split"], "development");
    assert_eq!(parsed["evaluated_runs"], eval.rows.len());
    let listed: Vec<&str> = parsed["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(listed.len(), files.len() - 1);
    for file in &files {
        assert!(!file.content.is_empty(), "{}", file.path);
        assert!(
            !file.content.contains(env!("CARGO_MANIFEST_DIR")),
            "{}",
            file.path
        );
        assert!(!file.content.contains("inode"), "{}", file.path);
    }
    let primary = files
        .iter()
        .find(|a| a.path == "tables/table_primary_metrics.md")
        .unwrap();
    for needle in ["Denominators", "lexical-prefix", "strict-safe-open", " / "] {
        assert!(primary.content.contains(needle), "{needle}");
    }
}
