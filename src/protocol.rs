//! The frozen evaluation protocol, as constants and decision rules.
//!
//! `docs/FROZEN_EVALUATION_PROTOCOL.md` is the prose form of this file and
//! was committed, together with it, before any held-out seed was run. A
//! change to anything here after that commit is a protocol deviation and
//! must be recorded in section 15 of that document.

use serde::Serialize;

use crate::stats::Summary;

/// Version of the frozen protocol.
pub const PROTOCOL_VERSION: &str = "1.0";

/// Seeds used while building and debugging the harness. Never used for the
/// final tables.
pub fn development_seeds() -> Vec<u64> {
    (1..=20).collect()
}

/// Seeds reserved for the single final evaluation. Never used in a test or
/// during development.
pub fn held_out_seeds() -> Vec<u64> {
    (1001..=1040).collect()
}

/// Scenarios generated per family, per seed, in the in-distribution region.
pub const INSTANCES_PER_FAMILY: u32 = 8;

/// Additional scenarios per family, per held-out seed, from the held-out
/// depth region.
pub const HELD_OUT_DEPTH_INSTANCES_PER_FAMILY: u32 = 4;

/// Nesting depth (directories above the leaf), inclusive.
pub const DEPTH_IN_DISTRIBUTION: (u64, u64) = (1, 4);

/// Nesting depth of the held-out parameter region, inclusive. Never
/// generated for a development seed.
pub const DEPTH_HELD_OUT: (u64, u64) = (5, 6);

/// How many times every adapter may reissue a transiently failing open.
pub const RETRY_BUDGET: u32 = 2;

/// Largest number of injected transient failures. One more than the budget,
/// so that some runs exhaust it.
pub const MAX_TRANSIENT_FAILURES: u32 = 3;

/// `BeforeCall` timings are drawn from `1..=depth + TIMING_BEYOND_DEPTH`,
/// which covers every call of the longest adapter plus its retries.
pub const TIMING_BEYOND_DEPTH: u64 = 3;

/// Adapters whose final open names its target by a path string.
pub const PATH_STRING_ADAPTERS: [&str; 2] = ["lexical-prefix", "canonicalize-then-open"];

/// Adapters whose final open is anchored to a directory handle.
pub const DESCRIPTOR_ANCHORED_ADAPTERS: [&str; 2] = ["dirfd-relative", "strict-safe-open"];

/// The primary hypothesis.
pub const HYPOTHESIS: &str = "On held-out seeds, the descriptor-anchored adapters \
(dirfd-relative, strict-safe-open) have a lower completed containment-violation rate than the \
path-string adapters (lexical-prefix, canonicalize-then-open), and pay for it with lower \
availability under safe conditions.";

/// The null hypothesis.
pub const NULL_HYPOTHESIS: &str = "The two adapter groups do not differ in completed \
containment-violation rate: the 95% interval of the per-seed difference includes zero.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Supported,
    Mixed,
    Weakened,
    Rejected,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Supported => "supported",
            Verdict::Mixed => "mixed",
            Verdict::Weakened => "weakened",
            Verdict::Rejected => "rejected",
        }
    }
}

/// The pre-declared decision rule.
///
/// Both arguments summarise a per-seed difference, path-string group mean
/// minus descriptor-anchored group mean:
///
/// * `violation_diff`: completed containment-violation rate;
/// * `availability_diff`: availability under safe conditions.
///
/// The hypothesis predicts both are positive.
///
/// * **rejected** if the violation interval lies at or below zero;
/// * **supported** if both intervals lie above zero;
/// * **mixed** if exactly one does;
/// * **weakened** otherwise.
pub fn primary_verdict(violation_diff: &Summary, availability_diff: &Summary) -> Verdict {
    let preserves = violation_diff.ci_low > 0.0;
    let costs = availability_diff.ci_low > 0.0;
    if violation_diff.ci_high <= 0.0 {
        Verdict::Rejected
    } else if preserves && costs {
        Verdict::Supported
    } else if preserves || costs {
        Verdict::Mixed
    } else {
        Verdict::Weakened
    }
}

/// Exact secondary predictions, each reported as held or not held.
pub const PREDICTIONS: [(&str, &str); 4] = [
    (
        "P1",
        "strict-safe-open records zero containment violations in the held-out run",
    ),
    (
        "P2",
        "dirfd-relative records zero containment violations on runs that did not take the fallback path",
    ),
    (
        "P3",
        "canonicalize-then-open records zero containment violations on primary-path runs in which no mutation fired",
    ),
    (
        "P4",
        "for every adapter and family, containment violations occur in the held-out depth region if and only if they occur in the in-distribution region",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(ci_low: f64, ci_high: f64) -> Summary {
        Summary {
            n: 40,
            mean: (ci_low + ci_high) / 2.0,
            sd: 0.0,
            min: ci_low,
            max: ci_high,
            ci_low,
            ci_high,
        }
    }

    #[test]
    fn the_seed_lists_have_the_declared_sizes_and_do_not_overlap() {
        let dev = development_seeds();
        let held = held_out_seeds();
        assert_eq!((dev.len(), held.len()), (20, 40));
        assert!(dev.iter().all(|seed| !held.contains(seed)));
    }

    #[test]
    fn the_held_out_depth_region_is_disjoint_from_development() {
        assert!(DEPTH_HELD_OUT.0 > DEPTH_IN_DISTRIBUTION.1);
    }

    #[test]
    fn the_two_groups_partition_the_adapters() {
        let mut all: Vec<&str> = PATH_STRING_ADAPTERS
            .iter()
            .chain(&DESCRIPTOR_ANCHORED_ADAPTERS)
            .copied()
            .collect();
        all.sort_unstable();
        let mut registry: Vec<&str> = crate::adapters::all().iter().map(|a| a.name()).collect();
        registry.sort_unstable();
        assert_eq!(all, registry);
    }

    #[test]
    fn the_decision_rule_covers_every_case() {
        let above = summary(0.1, 0.2);
        let straddle = summary(-0.1, 0.1);
        let below = summary(-0.2, -0.1);
        assert_eq!(primary_verdict(&above, &above), Verdict::Supported);
        assert_eq!(primary_verdict(&above, &straddle), Verdict::Mixed);
        assert_eq!(primary_verdict(&above, &below), Verdict::Mixed);
        assert_eq!(primary_verdict(&straddle, &above), Verdict::Mixed);
        assert_eq!(primary_verdict(&straddle, &straddle), Verdict::Weakened);
        assert_eq!(primary_verdict(&below, &above), Verdict::Rejected);
        assert_eq!(
            primary_verdict(&summary(0.0, 0.0), &above),
            Verdict::Rejected
        );
    }
}
