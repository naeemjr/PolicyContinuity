//! End-to-end checks of the command-line interface.

use std::path::PathBuf;
use std::process::{Command, Output};

fn temp_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_policycontinuity"))
        .args(args)
        .arg("--temp-parent")
        .arg(temp_parent())
        .output()
        .expect("run the binary")
}

/// For subcommands that take no --temp-parent.
fn run_plain(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_policycontinuity"))
        .args(args)
        .output()
        .expect("run the binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn lists_exactly_the_four_adapters() {
    let out = run_plain(&["list", "adapters"]);
    assert!(out.status.success());
    let text = stdout(&out);
    let names: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert_eq!(
        names,
        [
            "lexical-prefix",
            "canonicalize-then-open",
            "dirfd-relative",
            "strict-safe-open"
        ]
    );
}

#[test]
fn lists_twelve_families_seven_outcomes_and_marked_primary_metrics() {
    assert_eq!(
        stdout(&run_plain(&["list", "families"])).lines().count(),
        12
    );
    assert_eq!(stdout(&run_plain(&["list", "outcomes"])).lines().count(), 7);
    let metrics = stdout(&run_plain(&["list", "metrics"]));
    assert_eq!(metrics.matches("[primary]").count(), 6);
    assert!(metrics.contains("runs under a safe condition"));
}

#[test]
fn lists_scenarios_for_a_development_seed() {
    let out = run_plain(&["list", "scenarios", "--seed", "3"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 96);
    assert!(text.contains("s3-toctou-barrier-00"));
    assert!(!text.contains("-deep"));
}

#[test]
fn protocol_prints_the_frozen_constants() {
    let text = stdout(&run_plain(&["protocol"]));
    for needle in [
        "1..=20 (20)",
        "1001..=1040 (40)",
        "P1:",
        "P4:",
        "hypothesis:",
    ] {
        assert!(text.contains(needle), "{needle}\n{text}");
    }
}

#[test]
fn run_prints_one_line_per_run() {
    let out = run(&["run", "--seed", "2", "--family", "nested-in-root"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 1 + 8 * 4);
    assert!(text.lines().skip(1).all(|l| l.contains("preserved")));
}

#[test]
fn run_emits_json_without_host_paths() {
    let out = run(&[
        "run",
        "--seed",
        "2",
        "--family",
        "symlink-outside-target",
        "--adapter",
        "lexical-prefix",
        "--format",
        "json",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let rows: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for row in rows {
        assert_eq!(row["judgement"]["outcome"], "containment_violation");
        assert_eq!(row["record"]["adapter"], "lexical-prefix");
    }
    assert!(!text.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn held_out_seeds_are_refused_outside_the_final_run() {
    for args in [
        vec!["run", "--seed", "1001"],
        vec![
            "inspect",
            "--seed",
            "1040",
            "--scenario",
            "x",
            "--adapter",
            "lexical-prefix",
        ],
    ] {
        let out = run(&args);
        assert!(!out.status.success());
        assert!(stderr(&out).contains("held-out seed"), "{}", stderr(&out));
    }
    let out = run_plain(&["list", "scenarios", "--seed", "1020"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("held-out seed"));
}

#[test]
fn retired_adapter_names_are_explained() {
    let out = run(&["run", "--adapter", "beneath-strict"]);
    assert!(!out.status.success());
    let text = stderr(&out);
    assert!(
        text.contains("v0.1.0") && text.contains("strict-safe-open"),
        "{text}"
    );

    let out = run(&["run", "--adapter", "nope"]);
    assert!(stderr(&out).contains("unknown adapter"));
}

#[test]
fn unknown_families_are_rejected_by_the_parser() {
    let out = run(&["run", "--family", "nope"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unknown value"));
}

#[test]
fn inspect_shows_the_fixture_the_record_and_the_live_identity() {
    let out = run(&[
        "inspect",
        "--seed",
        "1",
        "--scenario",
        "s1-symlink-outside-target-00",
        "--adapter",
        "lexical-prefix",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for needle in [
        "fixture steps:",
        "\"outcome\": \"containment_violation\"",
        "live handle identity",
        "Symlink",
    ] {
        assert!(text.contains(needle), "{needle}\n{text}");
    }
    assert!(!text.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn probe_reports_the_kernel() {
    let out = run(&["probe"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("kernel release"));
    assert!(stdout(&out).contains("openat2(RESOLVE_BENEATH) usable"));
}

#[test]
fn dev_writes_tables_a_manifest_and_an_environment_file() {
    let out_dir = temp_parent().join("dev-out");
    let _ = std::fs::remove_dir_all(&out_dir);
    let out = run(&["dev", "--out", out_dir.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("No hypothesis verdict"));
    for file in [
        "manifest.json",
        "environment.json",
        "dev/runs.csv",
        "dev/summary.csv",
        "tables/table_primary_metrics.md",
        "tables/table6_outcome_matrix.md",
    ] {
        assert!(out_dir.join(file).is_file(), "{file}");
    }
    assert!(!out_dir.join("heldout").exists());
    assert!(!out_dir.join("figures").exists());
}
