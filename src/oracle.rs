//! The semantic containment oracle.
//!
//! A pure function from the evaluator's hidden fixture state and one
//! [`OperationRecord`] to one [`RunOutcome`]. It touches no filesystem,
//! calls no adapter code, and shares no logic with one: it has its own
//! model of path resolution over the declared object graph, used only to
//! decide what the request *denoted* in each fixture state.
//!
//! Two policies are judged.
//!
//! * **Authorized-root object containment** (primary). A completed
//!   operation is contained when the object it acted on was created at or
//!   beneath the approved root. Which object that was comes from the
//!   harness's own observation of the handle, never from the adapter.
//! * **Lexical containment** (reported alongside). The request, joined to
//!   the root and normalised as text, stays beneath the root. This is a
//!   property of the request string alone.
//!
//! This module must not refer to the adapter, environment, or system-call
//! modules, or to `std::fs`; a test enforces that.

use std::collections::VecDeque;

use serde::Serialize;

use crate::fixture::{Graph, Kind, LinkText, ObjId, ROOT_NAME};
use crate::model::{Condition, Region, Reported, RunOutcome};
use crate::record::{OperationRecord, RequestPath};

/// Symlink expansions allowed in one resolution, as in Linux.
const MAX_LINK_EXPANSIONS: u32 = 40;

/// What a path denotes in one fixture state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denotation {
    Object(ObjId),
    /// Resolution fails (missing entry, not a directory, link loop).
    Nothing,
    /// Resolution would climb above the workspace base. Scenario validation
    /// rules this out; it is reported rather than assumed.
    LeavesWorkspace,
}

fn components(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split('/').map(str::to_string)
}

/// Resolve `request` the way the kernel resolves `root/request`: following
/// every symlink, with `..` moving to the directory's actual parent.
pub fn denote(graph: &Graph, request: &RequestPath) -> Denotation {
    let (mut at, mut queue): (ObjId, VecDeque<String>) = match request {
        RequestPath::Relative(text) => (Graph::ROOT, components(text).collect()),
        RequestPath::AbsoluteInWorkspace(loc) => (Graph::BASE, loc.0.iter().cloned().collect()),
    };
    let mut expansions = 0;
    while let Some(component) = queue.pop_front() {
        if !matches!(graph.node(at).kind, Kind::Dir(_)) {
            return Denotation::Nothing;
        }
        match component.as_str() {
            "" | "." => {}
            ".." => match graph.node(at).parent {
                Some(parent) => at = parent,
                None => return Denotation::LeavesWorkspace,
            },
            name => {
                let Some(child) = graph.child(at, name) else {
                    return Denotation::Nothing;
                };
                match &graph.node(child).kind {
                    Kind::Symlink(target) => {
                        expansions += 1;
                        if expansions > MAX_LINK_EXPANSIONS {
                            return Denotation::Nothing;
                        }
                        let expansion: Vec<String> = match target {
                            LinkText::Relative(text) => components(text).collect(),
                            LinkText::AbsoluteInWorkspace(loc) => {
                                at = Graph::BASE;
                                loc.0.clone()
                            }
                        };
                        for part in expansion.into_iter().rev() {
                            queue.push_front(part);
                        }
                    }
                    _ => at = child,
                }
            }
        }
    }
    Denotation::Object(at)
}

/// Whether the request, joined to the root and normalised purely as text,
/// stays beneath the root. The workspace prefix is opaque: climbing above
/// it is never contained.
pub fn lexically_contained(request: &RequestPath) -> bool {
    let (mut stack, parts): (Vec<String>, Vec<String>) = match request {
        RequestPath::Relative(text) => (vec![ROOT_NAME.to_string()], components(text).collect()),
        RequestPath::AbsoluteInWorkspace(loc) => (Vec::new(), loc.0.clone()),
    };
    for part in parts {
        match part.as_str() {
            "" | "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return false;
                }
            }
            _ => stack.push(part),
        }
    }
    stack.first().map(String::as_str) == Some(ROOT_NAME)
}

/// The evaluator's hidden state for one run.
#[derive(Debug, Clone, Copy)]
pub struct Truth<'a> {
    /// The fixture state before the run, then after each mutation that
    /// actually fired, in order.
    pub states: &'a [Graph],
    pub request: &'a RequestPath,
}

/// Classify the fixture states realized during a run.
pub fn condition(truth: &Truth<'_>) -> Condition {
    let mut any_outside = false;
    for graph in truth.states {
        let Denotation::Object(id) = denote(graph, truth.request) else {
            return Condition::Undetermined;
        };
        let node = graph.node(id);
        if node.kind != Kind::File {
            return Condition::Undetermined;
        }
        any_outside |= node.region == Region::Outside;
    }
    if truth.states.is_empty() {
        Condition::Undetermined
    } else if any_outside {
        Condition::Unsafe
    } else {
        Condition::Safe
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Judgement {
    /// Outcome under authorized-root object containment (primary).
    pub outcome: RunOutcome,
    pub condition: Condition,
    /// Outcome under lexical containment (secondary).
    pub lexical_outcome: RunOutcome,
    pub lexically_contained: bool,
    /// Why the run was labelled as it was.
    pub basis: &'static str,
}

/// Whether the adapter's `Unsupported` report is backed by the harness
/// having seen a primitive report unavailable. If not, it is a refusal.
fn effective_report(record: &OperationRecord) -> Reported {
    match record.reported {
        Reported::Unsupported if !record.unavailable_observed => Reported::Refused,
        other => other,
    }
}

fn object_outcome(
    truth: &Truth<'_>,
    record: &OperationRecord,
) -> (RunOutcome, Condition, &'static str) {
    let condition = condition(truth);
    let inconclusive = |basis| (RunOutcome::EvaluatorInconclusive, condition, basis);

    if record.harness_error.is_some() {
        return inconclusive("the harness could not apply a scheduled mutation");
    }
    if !record.registry_stable {
        return inconclusive("tracked object identities were not stable");
    }
    if condition == Condition::Undetermined {
        return inconclusive("the request denoted no file in some fixture state");
    }
    let Some(last) = truth.states.last() else {
        return inconclusive("no fixture state was recorded");
    };

    if effective_report(record) == Reported::Completed {
        if record.identity_matches != 1 {
            return inconclusive("the handle matched no single tracked object");
        }
        let Some(object) = record.handle_object else {
            return inconclusive("the handle matched no single tracked object");
        };
        if object >= last.len() || last.node(object).kind != Kind::File {
            return inconclusive("the handle is not a tracked regular file");
        }
        if record.effect_object != Some(object) {
            return inconclusive("handle identity and I/O evidence disagree");
        }
        return match last.node(object).region {
            Region::Outside => (
                RunOutcome::ContainmentViolation,
                condition,
                "completed on an outside sentinel object",
            ),
            Region::Authorized if !record.sentinels_modified.is_empty() => (
                RunOutcome::ContainmentViolation,
                condition,
                "a sentinel was modified during the run",
            ),
            Region::Authorized => (
                RunOutcome::Preserved,
                condition,
                "completed on an authorized object",
            ),
        };
    }

    // No completion was reported. An effect outside the root is still a
    // violation, whatever the adapter said afterwards.
    if !record.sentinels_modified.is_empty() {
        return (
            RunOutcome::ContainmentViolation,
            condition,
            "a sentinel was modified although no completion was reported",
        );
    }
    match effective_report(record) {
        Reported::Refused if condition == Condition::Unsafe => (
            RunOutcome::SafeRefusal,
            condition,
            "refused while the request denoted a sentinel",
        ),
        Reported::Refused => (
            RunOutcome::FalseRefusal,
            condition,
            "refused although the request denoted an authorized object throughout",
        ),
        Reported::Unsupported => (
            RunOutcome::UnsupportedSafe,
            condition,
            "declined because a required primitive was unavailable",
        ),
        _ => (
            RunOutcome::OrdinaryFailure,
            condition,
            "failed without a policy decision",
        ),
    }
}

fn lexical_outcome(record: &OperationRecord, contained: bool) -> RunOutcome {
    match (effective_report(record), contained) {
        (Reported::Completed, true) => RunOutcome::Preserved,
        (Reported::Completed, false) => RunOutcome::ContainmentViolation,
        (Reported::Refused, true) => RunOutcome::FalseRefusal,
        (Reported::Refused, false) => RunOutcome::SafeRefusal,
        (Reported::Unsupported, _) => RunOutcome::UnsupportedSafe,
        (Reported::Failed, _) => RunOutcome::OrdinaryFailure,
    }
}

/// Label one run.
pub fn judge(truth: &Truth<'_>, record: &OperationRecord) -> Judgement {
    let (outcome, condition, basis) = object_outcome(truth, record);
    let lexically_contained = lexically_contained(truth.request);
    Judgement {
        outcome,
        condition,
        lexical_outcome: lexical_outcome(record, lexically_contained),
        lexically_contained,
        basis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{Loc, Step};
    use crate::model::PathLabel;

    fn rel(text: &str) -> RequestPath {
        RequestPath::Relative(text.to_string())
    }

    fn link(at: &str, target: &str) -> Step {
        Step::Symlink {
            at: Loc::parse(at),
            target: LinkText::Relative(target.to_string()),
        }
    }

    /// root/a/f (authorized, id 3), outside/a/f (sentinel, id 6).
    fn graph() -> Graph {
        let mut g = Graph::new();
        for step in [
            Step::Dir(Loc::parse("root/a")),
            Step::File(Loc::parse("root/a/f")),
            Step::Dir(Loc::parse("outside")),
            Step::Dir(Loc::parse("outside/a")),
            Step::File(Loc::parse("outside/a/f")),
        ] {
            g.apply(&step).unwrap();
        }
        g
    }

    const INSIDE: ObjId = 3;
    const SENTINEL: ObjId = 6;

    fn record(reported: Reported, object: Option<ObjId>) -> OperationRecord {
        OperationRecord {
            scenario_id: "t".into(),
            seed: 0,
            root_object: Graph::ROOT,
            requested_path: "a/f".into(),
            adapter: "t".into(),
            adapter_contract_version: 1,
            retry_budget: 2,
            fallback_enabled: false,
            schedule: "none".into(),
            mutations_fired: vec![],
            reported,
            reason: String::new(),
            path_label: PathLabel::Primary,
            primitive_calls: vec![],
            transient_injected: 0,
            unavailable_observed: false,
            handle_identity: object.map(|o| (1, o as u64)),
            identity_matches: usize::from(object.is_some()),
            handle_object: object,
            effect_object: object,
            sentinels_modified: vec![],
            registry_stable: true,
            harness_error: None,
        }
    }

    fn outcome(states: &[Graph], request: &RequestPath, record: &OperationRecord) -> RunOutcome {
        judge(&Truth { states, request }, record).outcome
    }

    #[test]
    fn denote_follows_links_and_physical_dotdot() {
        let mut g = graph();
        assert_eq!(denote(&g, &rel("a/f")), Denotation::Object(INSIDE));
        assert_eq!(denote(&g, &rel("a/./../a//f")), Denotation::Object(INSIDE));
        assert_eq!(
            denote(&g, &rel("../outside/a/f")),
            Denotation::Object(SENTINEL)
        );
        assert_eq!(denote(&g, &rel("../root/a/f")), Denotation::Object(INSIDE));
        assert_eq!(denote(&g, &rel("a/missing")), Denotation::Nothing);
        assert_eq!(denote(&g, &rel("a/f/x")), Denotation::Nothing);
        assert_eq!(denote(&g, &rel("../../x")), Denotation::LeavesWorkspace);

        g.apply(&link("root/dir", "../outside/a")).unwrap();
        g.apply(&link("root/file", "a/f")).unwrap();
        assert_eq!(denote(&g, &rel("dir/f")), Denotation::Object(SENTINEL));
        assert_eq!(denote(&g, &rel("file")), Denotation::Object(INSIDE));
        // `..` after a link moves from the link's target, not its location.
        assert_eq!(denote(&g, &rel("dir/../a/f")), Denotation::Object(SENTINEL));
    }

    #[test]
    fn denote_handles_absolute_links_and_loops() {
        let mut g = graph();
        g.apply(&Step::Symlink {
            at: Loc::parse("root/abs"),
            target: LinkText::AbsoluteInWorkspace(Loc::parse("outside/a")),
        })
        .unwrap();
        g.apply(&link("root/loop", "loop")).unwrap();
        assert_eq!(denote(&g, &rel("abs/f")), Denotation::Object(SENTINEL));
        assert_eq!(denote(&g, &rel("loop")), Denotation::Nothing);
        let absolute = RequestPath::AbsoluteInWorkspace(Loc::parse("outside/a/f"));
        assert_eq!(denote(&g, &absolute), Denotation::Object(SENTINEL));
    }

    #[test]
    fn lexical_containment_is_about_the_text_alone() {
        assert!(lexically_contained(&rel("a/f")));
        assert!(lexically_contained(&rel("a/../a/./f")));
        assert!(lexically_contained(&rel("../root/a/f")));
        assert!(!lexically_contained(&rel("../outside/a/f")));
        assert!(!lexically_contained(&rel("a/../../outside/a/f")));
        assert!(!lexically_contained(&rel("../../root/a/f")));
        assert!(!lexically_contained(&rel("../rootx/a")));
        let absolute = |text| RequestPath::AbsoluteInWorkspace(Loc::parse(text));
        assert!(!lexically_contained(&absolute("outside/a/f")));
        assert!(lexically_contained(&absolute("root/a/f")));
    }

    #[test]
    fn a_safe_in_root_result_is_preserved() {
        let g = [graph()];
        let r = record(Reported::Completed, Some(INSIDE));
        assert_eq!(outcome(&g, &rel("a/f"), &r), RunOutcome::Preserved);
    }

    #[test]
    fn a_sentinel_result_is_a_violation_whatever_the_request_looked_like() {
        let mut g = graph();
        g.apply(&link("root/leak", "../outside/a/f")).unwrap();
        let states = [g];
        let r = record(Reported::Completed, Some(SENTINEL));
        let j = judge(
            &Truth {
                states: &states,
                request: &rel("leak"),
            },
            &r,
        );
        assert_eq!(j.outcome, RunOutcome::ContainmentViolation);
        assert_eq!(j.condition, Condition::Unsafe);
        // The same run is fine under the lexical policy: the two disagree.
        assert!(j.lexically_contained);
        assert_eq!(j.lexical_outcome, RunOutcome::Preserved);
    }

    #[test]
    fn refusals_are_classified_by_the_fixture_state_not_the_adapter() {
        let safe = [graph()];
        let refused = record(Reported::Refused, None);
        assert_eq!(
            outcome(&safe, &rel("a/f"), &refused),
            RunOutcome::FalseRefusal
        );
        assert_eq!(
            outcome(&safe, &rel("../outside/a/f"), &refused),
            RunOutcome::SafeRefusal
        );
        // A request that leaves the root textually but lands on an
        // authorized object: refusing it is a false refusal under the
        // object policy and under the lexical one.
        let j = judge(
            &Truth {
                states: &safe,
                request: &rel("../root/a/f"),
            },
            &refused,
        );
        assert_eq!(j.outcome, RunOutcome::FalseRefusal);
        assert_eq!(j.lexical_outcome, RunOutcome::FalseRefusal);
    }

    #[test]
    fn one_unsafe_state_during_the_run_makes_the_condition_unsafe() {
        let before = graph();
        let mut after = before.clone();
        after
            .apply(&Step::Rename {
                from: Loc::parse("root/a"),
                to: Loc::parse("root/a.old"),
            })
            .unwrap();
        after.apply(&link("root/a", "../outside/a")).unwrap();
        let states = [before, after];
        let truth = Truth {
            states: &states,
            request: &rel("a/f"),
        };
        assert_eq!(condition(&truth), Condition::Unsafe);
        // Completing on the object that was authorized all along is fine.
        let held = record(Reported::Completed, Some(INSIDE));
        assert_eq!(judge(&truth, &held).outcome, RunOutcome::Preserved);
        let refused = record(Reported::Refused, None);
        assert_eq!(judge(&truth, &refused).outcome, RunOutcome::SafeRefusal);
    }

    #[test]
    fn failures_and_unsupported_are_their_own_outcomes() {
        let g = [graph()];
        let failed = record(Reported::Failed, None);
        assert_eq!(
            outcome(&g, &rel("a/f"), &failed),
            RunOutcome::OrdinaryFailure
        );

        let mut unsupported = record(Reported::Unsupported, None);
        unsupported.unavailable_observed = true;
        assert_eq!(
            outcome(&g, &rel("a/f"), &unsupported),
            RunOutcome::UnsupportedSafe
        );

        // Claiming "unsupported" when nothing was unavailable is a refusal.
        unsupported.unavailable_observed = false;
        assert_eq!(
            outcome(&g, &rel("a/f"), &unsupported),
            RunOutcome::FalseRefusal
        );
    }

    #[test]
    fn unstable_or_contradictory_evidence_is_inconclusive() {
        let g = [graph()];
        let request = rel("a/f");
        let inconclusive = RunOutcome::EvaluatorInconclusive;

        let mut r = record(Reported::Completed, Some(INSIDE));
        r.registry_stable = false;
        assert_eq!(outcome(&g, &request, &r), inconclusive);

        let mut r = record(Reported::Completed, Some(INSIDE));
        r.effect_object = Some(SENTINEL);
        assert_eq!(outcome(&g, &request, &r), inconclusive);

        let mut r = record(Reported::Completed, None);
        r.identity_matches = 2;
        assert_eq!(outcome(&g, &request, &r), inconclusive);

        // A handle on a directory is not an operated-on file.
        let r = record(Reported::Completed, Some(Graph::ROOT));
        assert_eq!(outcome(&g, &request, &r), inconclusive);

        let mut r = record(Reported::Refused, None);
        r.harness_error = Some("boom".into());
        assert_eq!(outcome(&g, &request, &r), inconclusive);

        let r = record(Reported::Refused, None);
        assert_eq!(outcome(&g, &rel("a/missing"), &r), inconclusive);
        assert_eq!(outcome(&[], &request, &r), inconclusive);
    }

    #[test]
    fn a_modified_sentinel_is_a_violation_even_without_a_completion() {
        let g = [graph()];
        let mut r = record(Reported::Refused, None);
        r.sentinels_modified = vec![SENTINEL];
        assert_eq!(
            outcome(&g, &rel("a/f"), &r),
            RunOutcome::ContainmentViolation
        );

        let mut r = record(Reported::Completed, Some(INSIDE));
        r.sentinels_modified = vec![SENTINEL];
        assert_eq!(
            outcome(&g, &rel("a/f"), &r),
            RunOutcome::ContainmentViolation
        );
    }
}
