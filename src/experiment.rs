//! Running a seed list and turning the runs into tables and figures.
//!
//! The unit of analysis is the seed: every rate is computed per seed, and
//! intervals are a bootstrap over seeds. A single run is deterministic, so
//! an interval describes how much a rate moves with the seeded mix of
//! scenario parameters, not measurement noise.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::json;

use crate::adapters;
use crate::error::{Context, Error};
use crate::figures::{self, Bar, Caption, Cell, Point};
use crate::metrics::{self, Counts, Metric};
use crate::model::{Condition, DepthRegion, Family, RunOutcome, Split};
use crate::protocol::{self, Verdict};
use crate::report::{
    csv_row, fraction, interval, interval_opt, md_table, num, num_opt, pct, pct_opt, points,
};
use crate::runner::{RunRow, Session};
use crate::scenario;
use crate::stats::{stream_for, summarize, Summary};
use crate::sys;

/// Every run of one seed list.
pub struct Evaluation {
    pub split: Split,
    pub seeds: Vec<u64>,
    pub rows: Vec<RunRow>,
    pub scenarios: usize,
    pub openat2_supported: bool,
    pub kernel_release: String,
}

/// One output file, relative to the output directory.
pub struct Artifact {
    pub path: String,
    pub content: String,
}

/// Run every scenario of every seed against every adapter.
pub fn evaluate(split: Split, seeds: &[u64], temp_parent: &Path) -> Result<Evaluation, Error> {
    let adapters = adapters::all();
    let mut session = Session::open(temp_parent)?;
    let mut rows = Vec::new();
    let mut scenarios = 0;
    for seed in seeds {
        for scenario in scenario::generate(*seed, split) {
            scenarios += 1;
            for adapter in &adapters {
                rows.push(session.run(&scenario, adapter.as_ref())?);
            }
        }
    }
    let openat2_supported = session.openat2_supported();
    session.close()?;
    Ok(Evaluation {
        split,
        seeds: seeds.to_vec(),
        rows,
        scenarios,
        openat2_supported,
        kernel_release: sys::kernel_release(),
    })
}

type Keep<'a> = &'a dyn Fn(&RunRow) -> bool;

/// An owned run filter, for building lists of named scopes.
type Scope = Box<dyn Fn(&RunRow) -> bool>;

fn everything(_: &RunRow) -> bool {
    true
}

fn adapter_names() -> Vec<&'static str> {
    adapters::all().iter().map(|a| a.name()).collect()
}

impl Evaluation {
    /// Counts for one adapter (or a group of adapters) over the kept runs.
    fn pooled(&self, adapters: &[&str], keep: Keep<'_>) -> Counts {
        let mut counts = Counts::default();
        for row in &self.rows {
            if adapters.contains(&row.record.adapter.as_str()) && keep(row) {
                counts.add(row);
            }
        }
        counts
    }

    /// The same, split by seed, in seed order.
    fn per_seed(&self, adapters: &[&str], keep: Keep<'_>) -> Vec<Counts> {
        let mut by_seed: BTreeMap<u64, Counts> =
            self.seeds.iter().map(|s| (*s, Counts::default())).collect();
        for row in &self.rows {
            if adapters.contains(&row.record.adapter.as_str()) && keep(row) {
                by_seed.entry(row.record.seed).or_default().add(row);
            }
        }
        self.seeds.iter().map(|s| by_seed[s]).collect()
    }

    /// Mean, spread, and interval of a rate over the seeds where it is
    /// defined. `scope` only names the bootstrap stream.
    fn summary(
        &self,
        metric: &Metric,
        adapter: &str,
        scope: &str,
        keep: Keep<'_>,
    ) -> Option<Summary> {
        let rates: Vec<f64> = self
            .per_seed(&[adapter], keep)
            .iter()
            .filter_map(|c| metric.rate(c))
            .collect();
        summarize(
            &rates,
            stream_for(&format!("{}|{adapter}|{scope}", metric.key)),
        )
    }

    /// Per-seed difference of a rate between the two adapter groups:
    /// path-string minus descriptor-anchored. Each group's rate pools its
    /// two adapters' counts within the seed.
    fn group_difference(&self, metric: &Metric) -> (Vec<Option<f64>>, Option<Summary>) {
        let path = self.per_seed(&protocol::PATH_STRING_ADAPTERS, &everything);
        let anchored = self.per_seed(&protocol::DESCRIPTOR_ANCHORED_ADAPTERS, &everything);
        let diffs: Vec<Option<f64>> = path
            .iter()
            .zip(&anchored)
            .map(|(p, a)| Some(metric.rate(p)? - metric.rate(a)?))
            .collect();
        let defined: Vec<f64> = diffs.iter().flatten().copied().collect();
        let summary = summarize(&defined, stream_for(&format!("group-diff|{}", metric.key)));
        (diffs, summary)
    }
}

/// One exact prediction and whether it held.
struct Prediction {
    id: &'static str,
    statement: &'static str,
    held: bool,
    evidence: String,
}

fn predictions(eval: &Evaluation) -> Vec<Prediction> {
    let violations = |adapter: &str, keep: Keep<'_>| {
        let c = eval.pooled(&[adapter], keep);
        (c.outcome(RunOutcome::ContainmentViolation), c.total())
    };
    let exact = |index: usize, (bad, total): (u64, u64)| Prediction {
        id: protocol::PREDICTIONS[index].0,
        statement: protocol::PREDICTIONS[index].1,
        held: bad == 0,
        evidence: format!("{bad} containment violations in {total} runs"),
    };
    let mut out = vec![
        exact(0, violations("strict-safe-open", &everything)),
        exact(
            1,
            violations("dirfd-relative", &|r| !r.record.path_label.is_fallback()),
        ),
        exact(
            2,
            violations("canonicalize-then-open", &|r| {
                r.record.path_label == crate::model::PathLabel::Primary
                    && r.record.mutations_fired.is_empty()
            }),
        ),
    ];

    let mut mismatches = Vec::new();
    for adapter in adapter_names() {
        for family in Family::ALL {
            let violates = |region: DepthRegion| {
                eval.pooled(&[adapter], &|r| r.family == *family && r.region == region)
                    .outcome(RunOutcome::ContainmentViolation)
                    > 0
            };
            let (near, deep) = (
                violates(DepthRegion::InDistribution),
                violates(DepthRegion::HeldOutDepth),
            );
            if near != deep {
                mismatches.push(format!(
                    "{adapter}/{family}: in-distribution {near}, held-out depth {deep}"
                ));
            }
        }
    }
    out.push(Prediction {
        id: protocol::PREDICTIONS[3].0,
        statement: protocol::PREDICTIONS[3].1,
        held: mismatches.is_empty(),
        evidence: if mismatches.is_empty() {
            "48 of 48 adapter x family cells agree".to_string()
        } else {
            format!("cells that differ: {}", mismatches.join("; "))
        },
    });
    out
}

/// The outcome of applying the frozen decision rule.
struct Hypothesis {
    violation_diff: Option<Summary>,
    availability_diff: Option<Summary>,
    verdict: Option<Verdict>,
    predictions: Vec<Prediction>,
}

fn hypothesis(eval: &Evaluation) -> Hypothesis {
    let violation_diff = eval
        .group_difference(metrics::metric("completed_violation_rate"))
        .1;
    let availability_diff = eval
        .group_difference(metrics::metric("availability_safe"))
        .1;
    let verdict = match (&violation_diff, &availability_diff) {
        (Some(v), Some(a)) => Some(protocol::primary_verdict(v, a)),
        _ => None,
    };
    Hypothesis {
        violation_diff,
        availability_diff,
        verdict,
        predictions: predictions(eval),
    }
}

fn denominators(keys: &[&str]) -> String {
    let mut out = String::from("\nDenominators:\n\n");
    for key in keys {
        let m = metrics::metric(key);
        out.push_str(&format!(
            "- **{}** = {} / {}.\n",
            m.label, m.numerator, m.denominator
        ));
    }
    out
}

fn scope_note(eval: &Evaluation) -> String {
    format!(
        "\n{} seeds ({} split), {} scenarios, {} evaluated runs. Each cell is the mean of the \
         per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); \
         seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a \
         temporary workspace only.\n",
        eval.seeds.len(),
        eval.split,
        eval.scenarios,
        eval.rows.len()
    )
}

fn table_primary(eval: &Evaluation) -> String {
    let adapters = adapter_names();
    let mut headers = vec!["Primary metric".to_string()];
    headers.extend(adapters.iter().map(|a| a.to_string()));
    let rows: Vec<Vec<String>> = metrics::primary()
        .map(|m| {
            let mut row = vec![m.label.to_string()];
            row.extend(
                adapters
                    .iter()
                    .map(|a| interval_opt(eval.summary(m, a, "all", &everything).as_ref())),
            );
            row
        })
        .collect();
    let keys: Vec<&str> = metrics::primary().map(|m| m.key).collect();
    format!(
        "# Primary metrics by adapter\n\n{}{}{}",
        md_table(&headers, &rows),
        scope_note(eval),
        denominators(&keys)
    )
}

fn table_by_family(eval: &Evaluation, number: u32, key: &str) -> String {
    let metric = metrics::metric(key);
    let adapters = adapter_names();
    let mut headers = vec!["Scenario family".to_string()];
    headers.extend(adapters.iter().map(|a| a.to_string()));
    let mut rows = Vec::new();
    for family in Family::ALL {
        let mut row = vec![family.to_string()];
        for adapter in &adapters {
            let keep = |r: &RunRow| r.family == *family;
            let pooled = (metric.parts)(&eval.pooled(&[adapter], &keep));
            let summary = eval.summary(metric, adapter, family.as_str(), &keep);
            row.push(format!(
                "{} ({}/{})",
                interval_opt(summary.as_ref()),
                pooled.0,
                pooled.1
            ));
        }
        rows.push(row);
    }
    format!(
        "# Table {number}. {} by adapter and scenario family\n\n{}\nThe fraction in brackets is \
         the pooled numerator and denominator over all seeds.\n{}{}",
        metric.label,
        md_table(&headers, &rows),
        scope_note(eval),
        denominators(&[key])
    )
}

fn table_tradeoff(eval: &Evaluation) -> String {
    let keys = [
        "availability_safe",
        "false_refusal_rate_safe",
        "safe_refusal_rate_unsafe",
        "violation_rate_unsafe",
        "unsupported_rate",
        "ordinary_failure_rate",
    ];
    let mut headers = vec!["Adapter".to_string()];
    headers.extend(keys.iter().map(|k| metrics::metric(k).label.to_string()));
    let rows: Vec<Vec<String>> = adapter_names()
        .iter()
        .map(|adapter| {
            let mut row = vec![adapter.to_string()];
            row.extend(keys.iter().map(|k| {
                interval_opt(
                    eval.summary(metrics::metric(k), adapter, "all", &everything)
                        .as_ref(),
                )
            }));
            row
        })
        .collect();
    format!(
        "# Table 3. Availability against safe refusal, by adapter\n\n{}{}{}",
        md_table(&headers, &rows),
        scope_note(eval),
        denominators(&keys)
    )
}

fn table_false_refusals(eval: &Evaluation) -> String {
    let adapters = adapter_names();
    let mut headers = vec!["Family / variant".to_string()];
    headers.extend(adapters.iter().map(|a| a.to_string()));
    let mut rows = Vec::new();
    for family in Family::ALL {
        for variant in scenario::variants(*family) {
            let cells: Vec<(u64, u64)> = adapters
                .iter()
                .map(|adapter| {
                    let c = eval.pooled(&[adapter], &|r| {
                        r.family == *family && r.variant == *variant
                    });
                    (
                        c.outcome(RunOutcome::FalseRefusal),
                        c.condition(Condition::Safe),
                    )
                })
                .collect();
            if cells.iter().all(|(_, safe)| *safe == 0) {
                continue;
            }
            let mut row = vec![format!("{family} / {variant}")];
            row.extend(cells.iter().map(|(k, n)| fraction(*k, *n)));
            rows.push(row);
        }
    }
    let mut total = vec!["**all safe-condition runs**".to_string()];
    total.extend(adapters.iter().map(|adapter| {
        interval_opt(
            eval.summary(
                metrics::metric("false_refusal_rate_safe"),
                adapter,
                "all",
                &everything,
            )
            .as_ref(),
        )
    }));
    rows.push(total);
    format!(
        "# Table 4. False refusals under safe conditions\n\n{}\nEach cell is pooled over seeds: \
         runs labelled false_refusal / runs of that family and variant under a safe condition. \
         Variants with no safe-condition run for any adapter are omitted. The last row is the \
         per-seed mean with its interval.\n{}{}",
        md_table(&headers, &rows),
        scope_note(eval),
        denominators(&["false_refusal_rate_safe"])
    )
}

fn table_fallback(eval: &Evaluation) -> String {
    let headers = [
        "Adapter",
        "Declared fallback rule",
        "Runs with the primitive unavailable",
        "Took the fallback path",
        "Reported unsupported",
        "Fallback-path violations",
        "Of which drift",
        "Fallback policy-drift rate",
    ];
    let rows: Vec<Vec<String>> = adapters::all()
        .iter()
        .map(|adapter| {
            let name = adapter.name();
            let faulted = eval.pooled(&[name], &|r| r.record.unavailable_observed);
            let all = eval.pooled(&[name], &everything);
            vec![
                name.to_string(),
                adapter.fallback().as_str().to_string(),
                faulted.total().to_string(),
                all.fallback_runs.to_string(),
                faulted.outcome(RunOutcome::UnsupportedSafe).to_string(),
                all.fallback_violations.to_string(),
                all.fallback_drift.to_string(),
                interval_opt(
                    eval.summary(
                        metrics::metric("fallback_drift_rate"),
                        name,
                        "all",
                        &everything,
                    )
                    .as_ref(),
                ),
            ]
        })
        .collect();
    format!(
        "# Table 5. Fallback policy drift by adapter\n\n{}\nA run \"drifts\" when it took the \
         fallback path and was labelled containment_violation while the same scenario and \
         adapter, with the unavailability fault removed, was not. `n/a` means the adapter never \
         took a fallback path, so the rate has no denominator.\n{}{}",
        md_table(&headers, &rows),
        scope_note(eval),
        denominators(&["fallback_drift_rate", "fallback_violation_rate"])
    )
}

fn table_outcome_matrix(eval: &Evaluation) -> String {
    let mut headers = vec![
        "Adapter".to_string(),
        "Scenario family".to_string(),
        "Runs".to_string(),
    ];
    headers.extend(RunOutcome::ALL.iter().map(|o| o.to_string()));
    let mut rows = Vec::new();
    for adapter in adapter_names() {
        for family in Family::ALL {
            let c = eval.pooled(&[adapter], &|r| r.family == *family);
            let mut row = vec![
                adapter.to_string(),
                family.to_string(),
                c.total().to_string(),
            ];
            row.extend(RunOutcome::ALL.iter().map(|o| c.outcome(*o).to_string()));
            rows.push(row);
        }
        let c = eval.pooled(&[adapter], &everything);
        let mut row = vec![
            format!("**{adapter}**"),
            "**all**".to_string(),
            c.total().to_string(),
        ];
        row.extend(RunOutcome::ALL.iter().map(|o| c.outcome(*o).to_string()));
        rows.push(row);
    }
    format!(
        "# Table 6. Outcome counts by adapter and scenario family\n\n{}\nPooled run counts. Every \
         run has exactly one of the seven outcomes, so each row sums to its run count.\n{}",
        md_table(&headers, &rows),
        scope_note(eval)
    )
}

fn table_worst_case(eval: &Evaluation) -> String {
    let adapters = adapter_names();
    let mut headers = vec!["Scenario family".to_string()];
    headers.extend(adapters.iter().map(|a| a.to_string()));
    let rows: Vec<Vec<String>> = Family::ALL
        .iter()
        .map(|family| {
            let mut row = vec![family.to_string()];
            row.extend(adapters.iter().map(|adapter| {
                eval.pooled(&[adapter], &|r| r.family == *family)
                    .worst()
                    .map_or("none".to_string(), |o| o.to_string())
            }));
            row
        })
        .collect();
    format!(
        "# Worst-case policy outcome by adapter and scenario family\n\n{}\nThe most severe \
         outcome observed in any run of that cell. Severity, worst first: containment_violation, \
         evaluator_inconclusive, false_refusal, unsupported_safe, ordinary_failure, safe_refusal, \
         preserved.\n{}",
        md_table(&headers, &rows),
        scope_note(eval)
    )
}

fn table_per_seed(eval: &Evaluation) -> String {
    let adapters = adapter_names();
    let violation = metrics::metric("completed_violation_rate");
    let availability = metrics::metric("availability_safe");
    let mut headers = vec!["Seed".to_string()];
    for adapter in &adapters {
        headers.push(format!("{adapter}: violation"));
        headers.push(format!("{adapter}: availability"));
    }
    let per_adapter: Vec<Vec<Counts>> = adapters
        .iter()
        .map(|a| eval.per_seed(&[a], &everything))
        .collect();
    let mut rows = Vec::new();
    for (i, seed) in eval.seeds.iter().enumerate() {
        let mut row = vec![seed.to_string()];
        for counts in &per_adapter {
            row.push(pct_opt(violation.rate(&counts[i])));
            row.push(pct_opt(availability.rate(&counts[i])));
        }
        rows.push(row);
    }

    let mut by_depth_headers = vec![
        "Region".to_string(),
        "Depth".to_string(),
        "Scenarios".to_string(),
    ];
    for adapter in &adapters {
        by_depth_headers.push(format!("{adapter}: violation"));
        by_depth_headers.push(format!("{adapter}: availability"));
    }
    let mut depths: Vec<(DepthRegion, usize)> = eval
        .rows
        .iter()
        .map(|r| (r.region, r.params.depth))
        .collect();
    depths.sort();
    depths.dedup();
    let by_depth: Vec<Vec<String>> = depths
        .iter()
        .map(|(region, depth)| {
            let keep = |r: &RunRow| r.region == *region && r.params.depth == *depth;
            let scenarios = eval.pooled(&[adapters[0]], &keep).total();
            let mut row = vec![region.to_string(), depth.to_string(), scenarios.to_string()];
            for adapter in &adapters {
                let c = eval.pooled(&[adapter], &keep);
                let (vn, vd) = (violation.parts)(&c);
                let (an, ad) = (availability.parts)(&c);
                row.push(fraction(vn, vd));
                row.push(fraction(an, ad));
            }
            row
        })
        .collect();

    format!(
        "# Table 7. Per-seed and per-parameter summary\n\n## Per seed\n\n{}\n## By nesting depth \
         (pooled over seeds)\n\n{}\nDepths 5 and 6 are the held-out parameter region: no \
         development run generated them.\n{}{}",
        md_table(&headers, &rows),
        md_table(&by_depth_headers, &by_depth),
        scope_note(eval),
        denominators(&["completed_violation_rate", "availability_safe"])
    )
}

/// Scenario-level comparison of `strict-safe-open` against each path-string
/// adapter: the same scenario, the same inputs, two outcomes.
fn table_strict_refusal(eval: &Evaluation) -> String {
    let mut by_scenario: BTreeMap<&str, BTreeMap<&str, &RunRow>> = BTreeMap::new();
    for row in &eval.rows {
        by_scenario
            .entry(row.record.scenario_id.as_str())
            .or_default()
            .insert(row.record.adapter.as_str(), row);
    }
    let not_completed = |o: RunOutcome| {
        matches!(
            o,
            RunOutcome::SafeRefusal
                | RunOutcome::FalseRefusal
                | RunOutcome::UnsupportedSafe
                | RunOutcome::OrdinaryFailure
        )
    };
    let mut headers = vec!["Scenario family".to_string(), "Scenarios".to_string()];
    for other in protocol::PATH_STRING_ADAPTERS {
        headers.push(format!("helps vs {other}"));
        headers.push(format!("costs vs {other}"));
    }
    headers.push("of the costs: false_refusal / unsupported_safe".to_string());

    let mut rows = Vec::new();
    let mut totals = [0u64; 7];
    for family in Family::ALL {
        let mut cells = vec![0u64; 7];
        for runs in by_scenario.values() {
            let Some(strict) = runs.get("strict-safe-open") else {
                continue;
            };
            if strict.family != *family {
                continue;
            }
            cells[0] += 1;
            let strict_outcome = strict.judgement.outcome;
            let mut cost_kind = None;
            for (i, other) in protocol::PATH_STRING_ADAPTERS.iter().enumerate() {
                let Some(other) = runs.get(other) else {
                    continue;
                };
                let other_outcome = other.judgement.outcome;
                if other_outcome == RunOutcome::ContainmentViolation
                    && strict_outcome != RunOutcome::ContainmentViolation
                {
                    cells[1 + 2 * i] += 1;
                }
                if other_outcome == RunOutcome::Preserved
                    && not_completed(strict_outcome)
                    && strict.judgement.condition == Condition::Safe
                {
                    cells[2 + 2 * i] += 1;
                    cost_kind = Some(strict_outcome);
                }
            }
            match cost_kind {
                Some(RunOutcome::FalseRefusal) => cells[5] += 1,
                Some(RunOutcome::UnsupportedSafe) => cells[6] += 1,
                _ => {}
            }
        }
        for (total, cell) in totals.iter_mut().zip(&cells) {
            *total += cell;
        }
        let mut row = vec![family.to_string()];
        row.extend(cells[..5].iter().map(u64::to_string));
        row.push(format!("{} / {}", cells[5], cells[6]));
        rows.push(row);
    }
    let mut row = vec!["**all families**".to_string()];
    row.extend(totals[..5].iter().map(u64::to_string));
    row.push(format!("{} / {}", totals[5], totals[6]));
    rows.push(row);

    format!(
        "# Table 8. When strict refusal helps, and when it costs availability\n\n{}\nCounts of \
         scenarios, comparing `strict-safe-open` with one path-string adapter on the identical \
         scenario. **Helps**: the other adapter's run was a containment_violation and \
         strict-safe-open's was not. **Costs**: the other adapter's run was preserved, and \
         strict-safe-open, under a safe condition, did not complete (false_refusal, \
         unsupported_safe, or ordinary_failure). The last column splits the scenarios with a \
         cost against either adapter by strict-safe-open's own outcome.\n{}",
        md_table(&headers, &rows),
        scope_note(eval)
    )
}

fn table_policies(eval: &Evaluation) -> String {
    let mut headers = vec!["Adapter".to_string(), "Policy".to_string()];
    headers.extend(RunOutcome::ALL.iter().map(|o| o.to_string()));
    let mut rows = Vec::new();
    for adapter in adapter_names() {
        let c = eval.pooled(&[adapter], &everything);
        let mut object = vec![
            adapter.to_string(),
            "authorized-root object (primary)".to_string(),
        ];
        object.extend(RunOutcome::ALL.iter().map(|o| c.outcome(*o).to_string()));
        let mut lexical = vec![adapter.to_string(), "lexical".to_string()];
        lexical.extend(
            RunOutcome::ALL
                .iter()
                .map(|o| c.lexical[o.index()].to_string()),
        );
        rows.push(object);
        rows.push(lexical);
        rows.push(vec![
            adapter.to_string(),
            "runs labelled differently".to_string(),
            fraction(c.policy_disagreements, c.total()),
        ]);
    }
    format!(
        "# The two containment policies compared\n\n{}\nPooled run counts. The lexical policy \
         judges the request text alone: it never yields evaluator_inconclusive, and it calls a \
         completed operation preserved whenever the normalised text stays beneath the root, \
         whatever object was actually reached.\n{}",
        md_table(&headers, &rows),
        scope_note(eval)
    )
}

fn runs_csv(eval: &Evaluation) -> String {
    let opt = |v: Option<usize>| v.map(|v| v.to_string()).unwrap_or_default();
    let mut out = csv_row(&[
        "seed",
        "scenario_id",
        "family",
        "variant",
        "region",
        "operation",
        "depth",
        "position",
        "timing",
        "transient_failures",
        "absolute_link",
        "fallback_enabled",
        "retry_budget",
        "schedule",
        "requested_path",
        "root_object",
        "adapter",
        "adapter_contract_version",
        "reported",
        "reason",
        "path_label",
        "primitive_calls",
        "mutations_fired",
        "transient_injected",
        "unavailable_observed",
        "identity_snapshot",
        "identity_matches",
        "handle_object",
        "effect_object",
        "sentinels_modified",
        "registry_stable",
        "condition",
        "outcome",
        "lexically_contained",
        "lexical_outcome",
        "twin_outcome",
    ]);
    let join = |items: Vec<String>| items.join(">");
    for row in &eval.rows {
        let r = &row.record;
        let j = &row.judgement;
        out.push_str(&csv_row(&[
            r.seed.to_string(),
            r.scenario_id.clone(),
            row.family.to_string(),
            row.variant.to_string(),
            row.region.to_string(),
            row.operation.to_string(),
            row.params.depth.to_string(),
            opt(row.params.position),
            row.params.timing.map(|t| t.to_string()).unwrap_or_default(),
            row.params.transient.to_string(),
            row.params
                .absolute_link
                .map(|a| a.to_string())
                .unwrap_or_default(),
            r.fallback_enabled.to_string(),
            r.retry_budget.to_string(),
            r.schedule.clone(),
            r.requested_path.clone(),
            format!("obj-{}", r.root_object),
            r.adapter.clone(),
            r.adapter_contract_version.to_string(),
            r.reported.to_string(),
            r.reason.clone(),
            r.path_label.to_string(),
            join(r.primitive_calls.iter().map(|p| p.to_string()).collect()),
            join(r.mutations_fired.iter().map(|i| i.to_string()).collect()),
            r.transient_injected.to_string(),
            r.unavailable_observed.to_string(),
            r.handle_identity.is_some().to_string(),
            r.identity_matches.to_string(),
            r.handle_object
                .map(|o| format!("obj-{o}"))
                .unwrap_or_default(),
            r.effect_object
                .map(|o| format!("obj-{o}"))
                .unwrap_or_default(),
            join(
                r.sentinels_modified
                    .iter()
                    .map(|o| format!("obj-{o}"))
                    .collect(),
            ),
            r.registry_stable.to_string(),
            j.condition.to_string(),
            j.outcome.to_string(),
            j.lexically_contained.to_string(),
            j.lexical_outcome.to_string(),
            row.twin_outcome.map(|o| o.to_string()).unwrap_or_default(),
        ]));
    }
    out
}

fn per_seed_csv(eval: &Evaluation) -> String {
    let mut out = csv_row(&[
        "seed",
        "adapter",
        "metric",
        "primary",
        "numerator",
        "denominator",
        "rate",
    ]);
    for adapter in adapter_names() {
        let counts = eval.per_seed(&[adapter], &everything);
        for (seed, c) in eval.seeds.iter().zip(&counts) {
            for m in metrics::METRICS {
                let (n, d) = (m.parts)(c);
                out.push_str(&csv_row(&[
                    seed.to_string(),
                    adapter.to_string(),
                    m.key.to_string(),
                    m.primary.to_string(),
                    n.to_string(),
                    d.to_string(),
                    num_opt(m.rate(c)),
                ]));
            }
        }
    }
    out
}

fn outcome_counts_csv(eval: &Evaluation) -> String {
    let mut header: Vec<String> = ["seed", "adapter", "family", "region", "condition", "runs"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    header.extend(RunOutcome::ALL.iter().map(|o| o.to_string()));
    let mut cells: BTreeMap<(u64, usize, Family, DepthRegion, Condition), [u64; 7]> =
        BTreeMap::new();
    let names = adapter_names();
    for row in &eval.rows {
        let adapter = names
            .iter()
            .position(|n| *n == row.record.adapter)
            .unwrap_or(usize::MAX);
        let key = (
            row.record.seed,
            adapter,
            row.family,
            row.region,
            row.judgement.condition,
        );
        cells.entry(key).or_default()[row.judgement.outcome.index()] += 1;
    }
    let mut out = csv_row(&header);
    for ((seed, adapter, family, region, condition), counts) in cells {
        let mut fields = vec![
            seed.to_string(),
            names.get(adapter).unwrap_or(&"?").to_string(),
            family.to_string(),
            region.to_string(),
            condition.to_string(),
            counts.iter().sum::<u64>().to_string(),
        ];
        fields.extend(counts.iter().map(u64::to_string));
        out.push_str(&csv_row(&fields));
    }
    out
}

fn summary_csv(eval: &Evaluation) -> String {
    let mut out = csv_row(&[
        "scope",
        "adapter",
        "metric",
        "primary",
        "seeds_defined",
        "mean",
        "sd",
        "min",
        "max",
        "ci95_low",
        "ci95_high",
        "pooled_numerator",
        "pooled_denominator",
    ]);
    let mut scopes: Vec<(String, Scope)> = vec![("all".to_string(), Box::new(everything))];
    for region in DepthRegion::ALL {
        if eval.rows.iter().any(|r| r.region == *region) {
            scopes.push((
                format!("region:{region}"),
                Box::new(move |r: &RunRow| r.region == *region),
            ));
        }
    }
    for family in Family::ALL {
        scopes.push((
            format!("family:{family}"),
            Box::new(move |r: &RunRow| r.family == *family),
        ));
    }
    for (scope, keep) in &scopes {
        for adapter in adapter_names() {
            let pooled = eval.pooled(&[adapter], keep.as_ref());
            for m in metrics::METRICS {
                let (n, d) = (m.parts)(&pooled);
                let s = eval.summary(m, adapter, scope, keep.as_ref());
                let stat = |f: fn(&Summary) -> f64| num_opt(s.as_ref().map(f));
                out.push_str(&csv_row(&[
                    scope.clone(),
                    adapter.to_string(),
                    m.key.to_string(),
                    m.primary.to_string(),
                    s.map_or(0, |s| s.n).to_string(),
                    stat(|s| s.mean),
                    stat(|s| s.sd),
                    stat(|s| s.min),
                    stat(|s| s.max),
                    stat(|s| s.ci_low),
                    stat(|s| s.ci_high),
                    n.to_string(),
                    d.to_string(),
                ]));
            }
        }
    }
    out
}

fn by_parameter_csv(eval: &Evaluation) -> String {
    let mut header: Vec<String> = ["adapter", "parameter", "value", "runs"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    header.extend(RunOutcome::ALL.iter().map(|o| o.to_string()));
    let labels = |row: &RunRow| -> Vec<(&'static str, String)> {
        let p = &row.params;
        let mut out = vec![
            ("region", row.region.to_string()),
            ("depth", p.depth.to_string()),
            ("operation", row.operation.to_string()),
            ("fallback_enabled", row.record.fallback_enabled.to_string()),
            ("transient_failures", p.transient.to_string()),
            ("path_label", row.record.path_label.to_string()),
            (
                "mutation_fired",
                (!row.record.mutations_fired.is_empty()).to_string(),
            ),
        ];
        if let Some(position) = p.position {
            let kind = if position > p.depth {
                "leaf"
            } else {
                "directory"
            };
            out.push(("position_kind", kind.to_string()));
        }
        if let Some(timing) = p.timing {
            out.push(("timing_call", timing.to_string()));
        }
        if let Some(absolute) = p.absolute_link {
            let form = if absolute { "absolute" } else { "relative" };
            out.push(("link_form", form.to_string()));
        }
        out
    };
    let mut cells: BTreeMap<(String, &'static str, String), [u64; 7]> = BTreeMap::new();
    for row in &eval.rows {
        for (parameter, value) in labels(row) {
            cells
                .entry((row.record.adapter.clone(), parameter, value))
                .or_default()[row.judgement.outcome.index()] += 1;
        }
    }
    let mut out = csv_row(&header);
    for ((adapter, parameter, value), counts) in cells {
        let mut fields = vec![
            adapter,
            parameter.to_string(),
            value,
            counts.iter().sum::<u64>().to_string(),
        ];
        fields.extend(counts.iter().map(u64::to_string));
        out.push_str(&csv_row(&fields));
    }
    out
}

fn paired_csv(eval: &Evaluation) -> String {
    let mut out = csv_row(&["seed", "metric", "path_string_minus_descriptor_anchored"]);
    for key in ["completed_violation_rate", "availability_safe"] {
        let (diffs, _) = eval.group_difference(metrics::metric(key));
        for (seed, diff) in eval.seeds.iter().zip(diffs) {
            out.push_str(&csv_row(&[
                seed.to_string(),
                key.to_string(),
                num_opt(diff),
            ]));
        }
    }
    out
}

fn summary_json(summary: Option<&Summary>) -> serde_json::Value {
    match summary {
        None => serde_json::Value::Null,
        Some(s) => json!({
            "seeds": s.n,
            "mean": num(s.mean),
            "sd": num(s.sd),
            "min": num(s.min),
            "max": num(s.max),
            "ci95_low": num(s.ci_low),
            "ci95_high": num(s.ci_high),
        }),
    }
}

fn hypothesis_json(eval: &Evaluation, h: &Hypothesis) -> String {
    let strict = eval.pooled(&["strict-safe-open"], &everything);
    // With zero events in n runs, the one-sided 95% upper bound on the rate
    // would be 1 - 0.05^(1/n) if the runs were independent draws.
    let completed = strict.completed();
    let bound = (completed > 0 && strict.outcome(RunOutcome::ContainmentViolation) == 0)
        .then(|| 1.0 - 0.05_f64.powf(1.0 / completed as f64));
    let value = json!({
        "protocol_version": protocol::PROTOCOL_VERSION,
        "split": eval.split.as_str(),
        "hypothesis": protocol::HYPOTHESIS,
        "null_hypothesis": protocol::NULL_HYPOTHESIS,
        "decision_rule": "rejected if the violation-difference interval lies at or below zero; \
            supported if both intervals lie above zero; mixed if exactly one does; weakened otherwise",
        "difference_definition": "per seed, path-string group rate minus descriptor-anchored group \
            rate, each group pooling its two adapters' counts within the seed",
        "completed_violation_rate_difference": summary_json(h.violation_diff.as_ref()),
        "availability_safe_difference": summary_json(h.availability_diff.as_ref()),
        "verdict": h.verdict.map(Verdict::as_str),
        "predictions": h.predictions.iter().map(|p| json!({
            "id": p.id,
            "statement": p.statement,
            "held": p.held,
            "evidence": p.evidence,
        })).collect::<Vec<_>>(),
        "strict_safe_open_zero_violation_bound": {
            "completed_runs": completed,
            "one_sided_95_upper_bound": bound.map(num),
            "caveat": "treats runs as independent draws from the declared scenario distribution; \
                they are deterministic functions of seeded parameters, so this is indicative only",
        },
    });
    let mut text = serde_json::to_string_pretty(&value).expect("serialize the hypothesis record");
    text.push('\n');
    text
}

fn heat_figure(eval: &Evaluation, key: &str, title: &str, lead: &str) -> String {
    let metric = metrics::metric(key);
    let adapters = adapter_names();
    let rows: Vec<(String, Vec<Cell>)> = Family::ALL
        .iter()
        .map(|family| {
            let cells = adapters
                .iter()
                .map(|adapter| {
                    let keep = |r: &RunRow| r.family == *family;
                    let (n, d) = (metric.parts)(&eval.pooled(&[adapter], &keep));
                    let summary = eval.summary(metric, adapter, family.as_str(), &keep);
                    Cell {
                        value: summary.map(|s| s.mean),
                        text: pct_opt(summary.map(|s| s.mean)),
                        detail: format!(
                            "{adapter}, {family}: {} (per-seed mean, 95% interval); pooled {n}/{d}",
                            interval_opt(summary.as_ref())
                        ),
                    }
                })
                .collect();
            (family.to_string(), cells)
        })
        .collect();
    let caption = Caption {
        title: title.to_string(),
        subtitle: format!(
            "{lead} Mean of the per-seed rate over {} held-out seeds; synthetic fixtures only.",
            eval.seeds.len()
        ),
        note: format!(
            "{} = {} / {}. n/a: the denominator was zero in every seed. Intervals are in the \
             matching table.",
            metric.label, metric.numerator, metric.denominator
        ),
    };
    let columns: Vec<String> = adapters.iter().map(|a| a.to_string()).collect();
    figures::heat_table(&caption, &columns, &rows)
}

fn bar_figure(eval: &Evaluation, key: &str, title: &str, lead: &str) -> String {
    let metric = metrics::metric(key);
    let rows: Vec<Bar> = adapter_names()
        .iter()
        .map(|adapter| {
            let (n, d) = (metric.parts)(&eval.pooled(&[adapter], &everything));
            let summary = eval.summary(metric, adapter, "all", &everything);
            Bar {
                label: adapter.to_string(),
                value: summary.map(|s| s.mean),
                interval: summary.map(|s| (s.ci_low, s.ci_high)),
                text: match summary {
                    Some(s) => format!("{} ({n}/{d})", pct(s.mean)),
                    None => "n/a (no such runs)".to_string(),
                },
                detail: format!(
                    "{adapter}: {}; pooled {n}/{d}",
                    interval_opt(summary.as_ref())
                ),
            }
        })
        .collect();
    let caption = Caption {
        title: title.to_string(),
        subtitle: format!(
            "{lead} Bars: mean of the per-seed rate over {} held-out seeds; whiskers: 95% \
             bootstrap interval; in brackets: pooled counts.",
            eval.seeds.len()
        ),
        note: format!(
            "{} = {} / {}. Synthetic fixtures only.",
            metric.label, metric.numerator, metric.denominator
        ),
    };
    figures::bars(&caption, metric.label, &rows)
}

fn tradeoff_figure(eval: &Evaluation) -> String {
    let x = metrics::metric("availability_safe");
    let y = metrics::metric("safe_refusal_rate_unsafe");
    let points: Vec<Point> = adapter_names()
        .iter()
        .filter_map(|adapter| {
            let sx = eval.summary(x, adapter, "all", &everything)?;
            let sy = eval.summary(y, adapter, "all", &everything)?;
            Some(Point {
                label: adapter.to_string(),
                x: sx.mean,
                y: sy.mean,
                x_interval: (sx.ci_low, sx.ci_high),
                y_interval: (sy.ci_low, sy.ci_high),
                detail: format!("available {}, declined {}", pct(sx.mean), pct(sy.mean)),
            })
        })
        .collect();
    let caption = Caption {
        title: "Availability against safe refusal, by adapter".to_string(),
        subtitle: format!(
            "Right is better (completes when the request is safe); up is better (declines when \
             it is not). Mean of the per-seed rate over {} held-out seeds, with 95% bootstrap \
             intervals; synthetic fixtures only.",
            eval.seeds.len()
        ),
        note: format!(
            "x: {} = {} / {}. y: {} = {} / {}.",
            x.label, x.numerator, x.denominator, y.label, y.numerator, y.denominator
        ),
    };
    figures::scatter(&caption, x.label, y.label, &points)
}

fn manifest(eval: &Evaluation, files: &[String]) -> String {
    let twins = eval
        .rows
        .iter()
        .filter(|r| r.twin_outcome.is_some())
        .count();
    let value = json!({
        "tool": "policycontinuity",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": protocol::PROTOCOL_VERSION,
        "split": eval.split.as_str(),
        "seeds": eval.seeds,
        "scenarios": eval.scenarios,
        "evaluated_runs": eval.rows.len(),
        "auxiliary_twin_runs": twins,
        "instances_per_family": protocol::INSTANCES_PER_FAMILY,
        "held_out_depth_instances_per_family": protocol::HELD_OUT_DEPTH_INSTANCES_PER_FAMILY,
        "depth_in_distribution": [protocol::DEPTH_IN_DISTRIBUTION.0, protocol::DEPTH_IN_DISTRIBUTION.1],
        "depth_held_out": [protocol::DEPTH_HELD_OUT.0, protocol::DEPTH_HELD_OUT.1],
        "retry_budget": protocol::RETRY_BUDGET,
        "adapters": adapters::all().iter().map(|a| json!({
            "name": a.name(),
            "contract_version": a.contract_version(),
            "fallback_rule": a.fallback().as_str(),
            "strategy": a.strategy(),
        })).collect::<Vec<_>>(),
        "families": Family::ALL.iter().map(|f| f.as_str()).collect::<Vec<_>>(),
        "outcomes": RunOutcome::ALL.iter().map(|o| o.as_str()).collect::<Vec<_>>(),
        "bootstrap_resamples": crate::stats::BOOTSTRAP_RESAMPLES,
        "files": files,
    });
    let mut text = serde_json::to_string_pretty(&value).expect("serialize the manifest");
    text.push('\n');
    text
}

/// The machine this was run on. Kept out of the checksummed set, because
/// it is the one output expected to differ between machines.
pub fn environment_json(eval: &Evaluation) -> String {
    let value = json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "kernel_release": eval.kernel_release,
        "openat2_resolve_beneath_supported": eval.openat2_supported,
    });
    let mut text = serde_json::to_string_pretty(&value).expect("serialize the environment");
    text.push('\n');
    text
}

/// Every reproducible output file for an evaluation.
pub fn artifacts(eval: &Evaluation) -> Vec<Artifact> {
    let dir = match eval.split {
        Split::Development => "dev",
        Split::HeldOut => "heldout",
    };
    let mut out: Vec<Artifact> = Vec::new();
    let mut add = |path: String, content: String| out.push(Artifact { path, content });

    add(format!("{dir}/runs.csv"), runs_csv(eval));
    add(format!("{dir}/per_seed.csv"), per_seed_csv(eval));
    add(
        format!("{dir}/outcome_counts.csv"),
        outcome_counts_csv(eval),
    );
    add(format!("{dir}/summary.csv"), summary_csv(eval));
    add(format!("{dir}/by_parameter.csv"), by_parameter_csv(eval));
    add(
        "tables/table_primary_metrics.md".to_string(),
        table_primary(eval),
    );
    add(
        "tables/table1_preservation_by_family.md".to_string(),
        table_by_family(eval, 1, "containment_preservation_rate"),
    );
    add(
        "tables/table2_violation_by_family.md".to_string(),
        table_by_family(eval, 2, "completed_violation_rate"),
    );
    add(
        "tables/table3_availability_vs_refusal.md".to_string(),
        table_tradeoff(eval),
    );
    add(
        "tables/table4_false_refusals.md".to_string(),
        table_false_refusals(eval),
    );
    add(
        "tables/table5_fallback_drift.md".to_string(),
        table_fallback(eval),
    );
    add(
        "tables/table6_outcome_matrix.md".to_string(),
        table_outcome_matrix(eval),
    );
    add(
        "tables/table_worst_case.md".to_string(),
        table_worst_case(eval),
    );
    add(
        "tables/table_policy_comparison.md".to_string(),
        table_policies(eval),
    );

    if eval.split == Split::HeldOut {
        let h = hypothesis(eval);
        add(format!("{dir}/paired_differences.csv"), paired_csv(eval));
        add(format!("{dir}/hypothesis.json"), hypothesis_json(eval, &h));
        add(
            "tables/table7_per_seed_and_parameter.md".to_string(),
            table_per_seed(eval),
        );
        add(
            "tables/table8_when_strict_refusal_helps.md".to_string(),
            table_strict_refusal(eval),
        );
        add(
            "figures/fig1_preservation_by_family.svg".to_string(),
            heat_figure(
                eval,
                "containment_preservation_rate",
                "Containment-preservation rate by adapter and scenario family",
                "Darker is better: the share of runs in which nothing completed on an outside object.",
            ),
        );
        add(
            "figures/fig2_violation_by_family.svg".to_string(),
            heat_figure(
                eval,
                "completed_violation_rate",
                "Completed containment-violation rate by adapter and scenario family",
                "Lighter is better: of the operations an adapter let through, the share that reached a sentinel.",
            ),
        );
        add(
            "figures/fig3_availability_vs_refusal.svg".to_string(),
            tradeoff_figure(eval),
        );
        add(
            "figures/fig4_false_refusal_safe.svg".to_string(),
            bar_figure(
                eval,
                "false_refusal_rate_safe",
                "False-refusal rate under safe conditions, by adapter",
                "Shorter is better: refusals of requests that denoted an authorized object throughout.",
            ),
        );
        add(
            "figures/fig5_fallback_drift.svg".to_string(),
            bar_figure(
                eval,
                "fallback_drift_rate",
                "Fallback policy-drift rate, by adapter",
                "Shorter is better: fallback-path runs that violated when the fault-free twin did not.",
            ),
        );
    }

    let files: Vec<String> = out.iter().map(|a| a.path.clone()).collect();
    out.push(Artifact {
        path: "manifest.json".to_string(),
        content: manifest(eval, &files),
    });
    out
}

/// Write artifacts beneath `dir`, creating directories as needed.
pub fn write(dir: &Path, artifacts: &[Artifact]) -> Result<(), Error> {
    for artifact in artifacts {
        let path = dir.join(&artifact.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("creating an output directory")?;
        }
        fs::write(&path, &artifact.content).context(&format!("writing {}", artifact.path))?;
    }
    Ok(())
}

/// The text printed to the terminal after a run.
pub fn console_summary(eval: &Evaluation) -> String {
    let adapters = adapter_names();
    let mut out = format!(
        "{} split: {} seeds, {} scenarios, {} evaluated runs\n\n",
        eval.split,
        eval.seeds.len(),
        eval.scenarios,
        eval.rows.len()
    );
    out.push_str(&format!(
        "{:<44}",
        "primary metric (mean [95% CI] over seeds)"
    ));
    for adapter in &adapters {
        out.push_str(&format!("{adapter:>26}"));
    }
    out.push('\n');
    for m in metrics::primary() {
        out.push_str(&format!("{:<44}", m.label));
        for adapter in &adapters {
            let cell = interval_opt(eval.summary(m, adapter, "all", &everything).as_ref());
            out.push_str(&format!("{cell:>26}"));
        }
        out.push('\n');
    }

    out.push_str(&format!("\n{:<44}", "outcome counts (pooled)"));
    for adapter in &adapters {
        out.push_str(&format!("{adapter:>26}"));
    }
    out.push('\n');
    for outcome in RunOutcome::ALL {
        out.push_str(&format!("{:<44}", outcome.as_str()));
        for adapter in &adapters {
            let n = eval.pooled(&[adapter], &everything).outcome(*outcome);
            out.push_str(&format!("{n:>26}"));
        }
        out.push('\n');
    }

    if eval.split == Split::HeldOut {
        let h = hypothesis(eval);
        out.push_str("\npath-string minus descriptor-anchored, per seed:\n");
        if let Some(v) = &h.violation_diff {
            out.push_str(&format!(
                "  completed containment-violation rate: {}\n",
                points(v)
            ));
        }
        if let Some(a) = &h.availability_diff {
            out.push_str(&format!(
                "  availability under safe conditions:   {}\n",
                points(a)
            ));
        }
        out.push_str(&format!(
            "\nprimary hypothesis: {}\n",
            h.verdict.map_or("undefined", Verdict::as_str)
        ));
        for p in &h.predictions {
            out.push_str(&format!(
                "  {} {}: {} ({})\n",
                p.id,
                if p.held { "held" } else { "NOT held" },
                p.statement,
                p.evidence
            ));
        }
    } else {
        out.push_str(
            "\nDevelopment seeds only. No hypothesis verdict is computed from this split.\n",
        );
    }
    out
}

/// Print the summary value used by `interval` for tests and callers that
/// want one cell.
pub fn primary_cell(eval: &Evaluation, key: &str, adapter: &str) -> Option<String> {
    eval.summary(metrics::metric(key), adapter, "all", &everything)
        .as_ref()
        .map(interval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch_parent;

    fn small() -> Evaluation {
        evaluate(Split::Development, &[1, 2, 3], &scratch_parent()).unwrap()
    }

    #[test]
    fn every_scenario_is_run_once_per_adapter() {
        let eval = small();
        assert_eq!(
            eval.scenarios,
            3 * 12 * protocol::INSTANCES_PER_FAMILY as usize
        );
        assert_eq!(eval.rows.len(), eval.scenarios * 4);
        let total: u64 = adapter_names()
            .iter()
            .map(|a| eval.pooled(&[a], &everything).total())
            .sum();
        assert_eq!(total as usize, eval.rows.len());
    }

    #[test]
    fn per_seed_counts_add_up_to_the_pooled_counts() {
        let eval = small();
        for adapter in adapter_names() {
            let mut sum = Counts::default();
            for c in eval.per_seed(&[adapter], &everything) {
                sum.merge(&c);
            }
            assert_eq!(sum, eval.pooled(&[adapter], &everything), "{adapter}");
        }
    }

    #[test]
    fn two_evaluations_of_the_same_seeds_produce_identical_artifacts() {
        let (a, b) = (small(), small());
        let (a, b) = (artifacts(&a), artifacts(&b));
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.path, y.path);
            assert_eq!(x.content, y.content, "{}", x.path);
        }
    }

    #[test]
    fn development_artifacts_carry_no_verdict_and_no_host_paths() {
        let eval = small();
        let files = artifacts(&eval);
        assert!(files.iter().all(|a| !a.path.contains("hypothesis")));
        assert!(files.iter().all(|a| !a.path.starts_with("figures/")));
        assert!(files.iter().any(|a| a.path == "dev/runs.csv"));
        for file in &files {
            assert!(!file.content.contains("/home/"), "{}", file.path);
            assert!(!file.content.contains("pc-tmp"), "{}", file.path);
        }
        assert!(console_summary(&eval).contains("No hypothesis verdict"));
    }

    #[test]
    fn the_runs_file_has_one_line_per_run_and_a_fixed_width() {
        let eval = small();
        let csv = runs_csv(&eval);
        let mut lines = csv.lines();
        let width = lines.next().unwrap().split(',').count();
        assert_eq!(width, 36);
        assert_eq!(lines.count(), eval.rows.len());
    }

    #[test]
    fn group_differences_are_defined_per_seed() {
        let eval = small();
        let (diffs, summary) = eval.group_difference(metrics::metric("completed_violation_rate"));
        assert_eq!(diffs.len(), 3);
        assert!(diffs.iter().all(Option::is_some));
        assert_eq!(summary.unwrap().n, 3);
    }
}
