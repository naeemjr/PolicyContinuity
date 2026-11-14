//! Metric definitions. Every rate has an explicit numerator and
//! denominator, stated once here and printed wherever the rate is shown.

use serde::Serialize;

use crate::model::{Condition, RunOutcome};
use crate::runner::RunRow;

const OUTCOMES: usize = 7;
const CONDITIONS: usize = 3;

/// Run counts for one group of runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// `cells[condition][outcome]` under the primary (object) policy.
    pub cells: [[u64; OUTCOMES]; CONDITIONS],
    /// Outcomes under the lexical policy.
    pub lexical: [u64; OUTCOMES],
    /// Runs whose two policy outcomes differ.
    pub policy_disagreements: u64,
    /// Runs in which at least one scheduled mutation fired.
    pub mutation_fired: u64,
    pub retry_runs: u64,
    pub retry_violations: u64,
    pub fallback_runs: u64,
    pub fallback_violations: u64,
    /// Fallback-path violations whose fault-free twin was not a violation.
    pub fallback_drift: u64,
}

impl Counts {
    pub fn add(&mut self, row: &RunRow) {
        let outcome = row.judgement.outcome;
        self.cells[row.judgement.condition.index()][outcome.index()] += 1;
        self.lexical[row.judgement.lexical_outcome.index()] += 1;
        self.policy_disagreements += u64::from(outcome != row.judgement.lexical_outcome);
        self.mutation_fired += u64::from(!row.record.mutations_fired.is_empty());
        let violated = u64::from(outcome == RunOutcome::ContainmentViolation);
        if row.record.path_label.is_retry() {
            self.retry_runs += 1;
            self.retry_violations += violated;
        }
        if row.record.path_label.is_fallback() {
            self.fallback_runs += 1;
            self.fallback_violations += violated;
            self.fallback_drift += u64::from(row.fallback_drift());
        }
    }

    pub fn merge(&mut self, other: &Counts) {
        for (mine, theirs) in self
            .cells
            .iter_mut()
            .flatten()
            .zip(other.cells.iter().flatten())
        {
            *mine += theirs;
        }
        for (mine, theirs) in self.lexical.iter_mut().zip(&other.lexical) {
            *mine += theirs;
        }
        self.policy_disagreements += other.policy_disagreements;
        self.mutation_fired += other.mutation_fired;
        self.retry_runs += other.retry_runs;
        self.retry_violations += other.retry_violations;
        self.fallback_runs += other.fallback_runs;
        self.fallback_violations += other.fallback_violations;
        self.fallback_drift += other.fallback_drift;
    }

    pub fn cell(&self, condition: Condition, outcome: RunOutcome) -> u64 {
        self.cells[condition.index()][outcome.index()]
    }

    /// Runs with this outcome, over every condition.
    pub fn outcome(&self, outcome: RunOutcome) -> u64 {
        self.cells.iter().map(|row| row[outcome.index()]).sum()
    }

    /// Runs under this condition, over every outcome.
    pub fn condition(&self, condition: Condition) -> u64 {
        self.cells[condition.index()].iter().sum()
    }

    pub fn total(&self) -> u64 {
        self.cells.iter().flatten().sum()
    }

    /// Runs the evaluator could label.
    pub fn conclusive(&self) -> u64 {
        self.total() - self.outcome(RunOutcome::EvaluatorInconclusive)
    }

    /// Runs labelled `preserved` or `containment_violation`.
    pub fn completed(&self) -> u64 {
        self.outcome(RunOutcome::Preserved) + self.outcome(RunOutcome::ContainmentViolation)
    }

    /// The most severe outcome present, if any run was counted.
    pub fn worst(&self) -> Option<RunOutcome> {
        RunOutcome::ALL
            .iter()
            .copied()
            .filter(|o| self.outcome(*o) > 0)
            .max_by_key(|o| o.severity())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Better {
    Higher,
    Lower,
}

/// One rate: how to compute it and how to describe it.
pub struct Metric {
    pub key: &'static str,
    pub label: &'static str,
    pub numerator: &'static str,
    pub denominator: &'static str,
    pub better: Better,
    pub primary: bool,
    pub interpretation: &'static str,
    pub parts: fn(&Counts) -> (u64, u64),
}

impl Metric {
    /// The rate, or `None` when the denominator is zero.
    pub fn rate(&self, counts: &Counts) -> Option<f64> {
        let (numerator, denominator) = (self.parts)(counts);
        (denominator > 0).then(|| numerator as f64 / denominator as f64)
    }
}

use Condition::{Safe, Unsafe};
use RunOutcome::{
    ContainmentViolation, EvaluatorInconclusive, FalseRefusal, OrdinaryFailure, Preserved,
    SafeRefusal, UnsupportedSafe,
};

/// Every reported rate. The first six are the frozen primary metrics.
pub const METRICS: &[Metric] = &[
    Metric {
        key: "containment_preservation_rate",
        label: "Containment-preservation rate",
        numerator: "conclusive runs not labelled containment_violation",
        denominator: "conclusive runs (all runs minus evaluator_inconclusive)",
        better: Better::Higher,
        primary: true,
        interpretation: "Share of runs in which nothing completed on, or modified, an outside \
            object. Refusing counts as preserving.",
        parts: |c| {
            (
                c.conclusive() - c.outcome(ContainmentViolation),
                c.conclusive(),
            )
        },
    },
    Metric {
        key: "completed_violation_rate",
        label: "Completed containment-violation rate",
        numerator: "runs labelled containment_violation",
        denominator: "runs labelled preserved or containment_violation (completed operations)",
        better: Better::Lower,
        primary: true,
        interpretation: "Of the operations the adapter let through, the share that acted on an \
            outside sentinel.",
        parts: |c| (c.outcome(ContainmentViolation), c.completed()),
    },
    Metric {
        key: "safe_refusal_rate_unsafe",
        label: "Safe-refusal rate under unsafe conditions",
        numerator:
            "runs labelled safe_refusal, plus unsupported_safe runs under an unsafe condition",
        denominator: "runs under an unsafe condition",
        better: Better::Higher,
        primary: true,
        interpretation: "When the request denoted a sentinel at some point in the run, how often \
            the adapter declined.",
        parts: |c| {
            (
                c.outcome(SafeRefusal) + c.cell(Unsafe, UnsupportedSafe),
                c.condition(Unsafe),
            )
        },
    },
    Metric {
        key: "false_refusal_rate_safe",
        label: "False-refusal rate under safe conditions",
        numerator: "runs labelled false_refusal",
        denominator: "runs under a safe condition",
        better: Better::Lower,
        primary: true,
        interpretation: "When the request denoted an authorized object throughout, how often the \
            adapter refused on policy grounds.",
        parts: |c| (c.outcome(FalseRefusal), c.condition(Safe)),
    },
    Metric {
        key: "availability_safe",
        label: "Availability under safe conditions",
        numerator: "runs labelled preserved under a safe condition",
        denominator: "runs under a safe condition",
        better: Better::Higher,
        primary: true,
        interpretation: "When the request denoted an authorized object throughout, how often the \
            operation completed. The shortfall is false refusals, unsupported, and ordinary \
            failures.",
        parts: |c| (c.cell(Safe, Preserved), c.condition(Safe)),
    },
    Metric {
        key: "fallback_drift_rate",
        label: "Fallback policy-drift rate",
        numerator:
            "fallback-path runs labelled containment_violation whose fault-free twin was not",
        denominator: "runs that took the fallback path",
        better: Better::Lower,
        primary: true,
        interpretation: "How often taking the compatibility fallback turned a run that would not \
            have violated into one that did.",
        parts: |c| (c.fallback_drift, c.fallback_runs),
    },
    Metric {
        key: "violation_rate_unsafe",
        label: "Violation rate under unsafe conditions",
        numerator: "runs labelled containment_violation",
        denominator: "runs under an unsafe condition",
        better: Better::Lower,
        primary: false,
        interpretation: "Exposure-conditioned violation rate.",
        parts: |c| (c.outcome(ContainmentViolation), c.condition(Unsafe)),
    },
    Metric {
        key: "completion_rate",
        label: "Completion rate (all runs)",
        numerator: "runs labelled preserved or containment_violation",
        denominator: "all runs",
        better: Better::Higher,
        primary: false,
        interpretation: "How often the adapter completed at all, safely or not.",
        parts: |c| (c.completed(), c.total()),
    },
    Metric {
        key: "unsupported_rate",
        label: "Unsupported-safe rate (all runs)",
        numerator: "runs labelled unsupported_safe",
        denominator: "all runs",
        better: Better::Lower,
        primary: false,
        interpretation: "How often the adapter declined because a primitive was unavailable.",
        parts: |c| (c.outcome(UnsupportedSafe), c.total()),
    },
    Metric {
        key: "ordinary_failure_rate",
        label: "Ordinary-failure rate (all runs)",
        numerator: "runs labelled ordinary_failure",
        denominator: "all runs",
        better: Better::Lower,
        primary: false,
        interpretation:
            "Failures that were not policy decisions, such as an exhausted retry budget.",
        parts: |c| (c.outcome(OrdinaryFailure), c.total()),
    },
    Metric {
        key: "inconclusive_rate",
        label: "Evaluator-inconclusive rate (all runs)",
        numerator: "runs labelled evaluator_inconclusive",
        denominator: "all runs",
        better: Better::Lower,
        primary: false,
        interpretation: "Runs the evaluator declined to label.",
        parts: |c| (c.outcome(EvaluatorInconclusive), c.total()),
    },
    Metric {
        key: "retry_violation_rate",
        label: "Violation rate on the retry path",
        numerator: "retry-path runs labelled containment_violation",
        denominator: "runs in which a file open was reissued after an injected transient failure",
        better: Better::Lower,
        primary: false,
        interpretation: "Whether reissuing the final open re-uses a stale decision.",
        parts: |c| (c.retry_violations, c.retry_runs),
    },
    Metric {
        key: "fallback_violation_rate",
        label: "Violation rate on the fallback path",
        numerator: "fallback-path runs labelled containment_violation",
        denominator: "runs that took the fallback path",
        better: Better::Lower,
        primary: false,
        interpretation: "Unpaired counterpart of the drift rate.",
        parts: |c| (c.fallback_violations, c.fallback_runs),
    },
    Metric {
        key: "lexical_completed_violation_rate",
        label: "Completed violation rate, lexical policy",
        numerator: "runs labelled containment_violation under the lexical policy",
        denominator: "runs labelled preserved or containment_violation under the lexical policy",
        better: Better::Lower,
        primary: false,
        interpretation: "The same question asked of the request text alone.",
        parts: |c| {
            let violations = c.lexical[ContainmentViolation.index()];
            (violations, violations + c.lexical[Preserved.index()])
        },
    },
    Metric {
        key: "policy_disagreement_rate",
        label: "Lexical/object policy disagreement rate",
        numerator: "runs whose outcome differs between the two policies",
        denominator: "all runs",
        better: Better::Lower,
        primary: false,
        interpretation: "How often judging the text and judging the object give different labels.",
        parts: |c| (c.policy_disagreements, c.total()),
    },
    Metric {
        key: "mutation_fired_rate",
        label: "Share of runs in which a mutation fired",
        numerator: "runs in which at least one scheduled mutation fired",
        denominator: "all runs",
        better: Better::Higher,
        primary: false,
        interpretation:
            "Exposure: an adapter that makes fewer primitive calls reaches fewer barriers.",
        parts: |c| (c.mutation_fired, c.total()),
    },
];

/// Look a metric up by key.
pub fn metric(key: &str) -> &'static Metric {
    METRICS
        .iter()
        .find(|m| m.key == key)
        .unwrap_or_else(|| panic!("no metric named `{key}`"))
}

/// The frozen primary metrics, in report order.
pub fn primary() -> impl Iterator<Item = &'static Metric> {
    METRICS.iter().filter(|m| m.primary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(cells: &[(Condition, RunOutcome, u64)]) -> Counts {
        let mut c = Counts::default();
        for (condition, outcome, n) in cells {
            c.cells[condition.index()][outcome.index()] += n;
        }
        c
    }

    fn sample() -> Counts {
        counts(&[
            (Safe, Preserved, 50),
            (Safe, FalseRefusal, 6),
            (Safe, OrdinaryFailure, 2),
            (Safe, UnsupportedSafe, 2),
            (Unsafe, Preserved, 4),
            (Unsafe, ContainmentViolation, 10),
            (Unsafe, SafeRefusal, 20),
            (Unsafe, UnsupportedSafe, 5),
            (Unsafe, OrdinaryFailure, 1),
            (Condition::Undetermined, EvaluatorInconclusive, 1),
        ])
    }

    fn parts(key: &str, c: &Counts) -> (u64, u64) {
        (metric(key).parts)(c)
    }

    #[test]
    fn there_are_six_primary_metrics_with_unique_keys() {
        assert_eq!(primary().count(), 6);
        let keys: std::collections::BTreeSet<&str> = METRICS.iter().map(|m| m.key).collect();
        assert_eq!(keys.len(), METRICS.len());
        for m in METRICS {
            assert!(
                !m.numerator.is_empty() && !m.denominator.is_empty(),
                "{}",
                m.key
            );
        }
    }

    #[test]
    fn totals_and_margins_add_up() {
        let c = sample();
        assert_eq!(c.total(), 101);
        assert_eq!(c.conclusive(), 100);
        assert_eq!(c.completed(), 64);
        assert_eq!(c.condition(Safe), 60);
        assert_eq!(c.condition(Unsafe), 40);
        assert_eq!(c.outcome(UnsupportedSafe), 7);
        let by_outcome: u64 = RunOutcome::ALL.iter().map(|o| c.outcome(*o)).sum();
        assert_eq!(by_outcome, c.total());
    }

    #[test]
    fn primary_metrics_use_the_declared_denominators() {
        let c = sample();
        assert_eq!(parts("containment_preservation_rate", &c), (90, 100));
        assert_eq!(parts("completed_violation_rate", &c), (10, 64));
        assert_eq!(parts("safe_refusal_rate_unsafe", &c), (25, 40));
        assert_eq!(parts("false_refusal_rate_safe", &c), (6, 60));
        assert_eq!(parts("availability_safe", &c), (50, 60));
        assert_eq!(parts("violation_rate_unsafe", &c), (10, 40));
    }

    #[test]
    fn a_zero_denominator_is_not_a_rate() {
        let c = sample();
        assert_eq!(metric("fallback_drift_rate").rate(&c), None);
        assert_eq!(metric("availability_safe").rate(&Counts::default()), None);
        assert_eq!(metric("false_refusal_rate_safe").rate(&c), Some(0.1));
    }

    #[test]
    fn merge_adds_every_field() {
        let mut a = sample();
        a.fallback_runs = 3;
        a.fallback_drift = 1;
        a.lexical[0] = 7;
        let mut b = a;
        b.merge(&a);
        assert_eq!(b.total(), 2 * a.total());
        assert_eq!(
            (b.fallback_runs, b.fallback_drift, b.lexical[0]),
            (6, 2, 14)
        );
    }

    #[test]
    fn worst_picks_the_most_severe_outcome_present() {
        assert_eq!(Counts::default().worst(), None);
        assert_eq!(sample().worst(), Some(ContainmentViolation));
        let mild = counts(&[(Safe, Preserved, 3), (Unsafe, SafeRefusal, 1)]);
        assert_eq!(mild.worst(), Some(SafeRefusal));
        let costly = counts(&[
            (Safe, Preserved, 3),
            (Safe, FalseRefusal, 1),
            (Safe, UnsupportedSafe, 1),
        ]);
        assert_eq!(costly.worst(), Some(FalseRefusal));
    }
}
