//! The seeded scenario matrix and its static confinement validation.
//!
//! A scenario is plain data: a fixture, a requested path, a schedule of
//! mutations tied to barriers, injected faults, and the contract parameters
//! every adapter shares. It carries no expected outcome: what a run "should"
//! have done is decided afterwards by the oracle from the fixture state.
//!
//! Every fixture has the same shape, with seeded names and depth:
//!
//! ```text
//! <base>/
//!   root/                       approved root
//!     <d1>/<d2>/.../<leaf>      primary chain: what the request names
//!     <alt>/<d1>/.../<leaf>     in-root mirror: an authorized link target
//!   <out>/<d1>/.../<leaf>       outside mirror: the sentinels
//! ```
//!
//! "Position" p names one component of the chain: 1 to `depth` are the
//! directories, `depth + 1` is the leaf file.

use crate::env::{Barrier, Contract, Faults};
use crate::fixture::{LinkText, Loc, Step, ROOT_NAME};
use crate::model::{DepthRegion, Family, OpKind, Split};
use crate::protocol;
use crate::record::RequestPath;
use crate::rng::{mix, SplitMix64};

/// Fixture steps applied when the harness reaches a barrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mutation {
    pub at: Barrier,
    pub label: &'static str,
    pub steps: Vec<Step>,
}

/// The parameters a scenario was drawn with, for stratified reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Params {
    pub depth: usize,
    /// Chain position the link or replacement applies to.
    pub position: Option<usize>,
    /// Ordinal of the primitive call a `BeforeCall` mutation is tied to.
    pub timing: Option<u32>,
    /// Injected transient failures.
    pub transient: u32,
    /// Whether a link in the scenario is written as absolute text.
    pub absolute_link: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub seed: u64,
    pub family: Family,
    pub variant: &'static str,
    pub region: DepthRegion,
    pub op: OpKind,
    pub request: RequestPath,
    pub setup: Vec<Step>,
    pub schedule: Vec<Mutation>,
    pub faults: Faults,
    pub contract: Contract,
    pub params: Params,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tree {
    Primary,
    Alt,
    Outside,
}

struct Layout {
    dirs: Vec<String>,
    leaf: String,
    alt: String,
    out: String,
}

impl Layout {
    fn draw(rng: &mut SplitMix64, depth: usize) -> Self {
        let mut tag = |prefix: &str| format!("{prefix}{:04x}", rng.below(0x1_0000));
        Self {
            dirs: (0..depth).map(|i| tag(&format!("d{}-", i + 1))).collect(),
            leaf: format!("{}.dat", tag("f-")),
            alt: tag("alt-"),
            out: tag("out-"),
        }
    }

    fn depth(&self) -> usize {
        self.dirs.len()
    }

    fn name(&self, position: usize) -> &str {
        if position <= self.depth() {
            &self.dirs[position - 1]
        } else {
            &self.leaf
        }
    }

    /// The chain component at `position` in `tree`; position 0 is the top.
    fn at(&self, tree: Tree, position: usize) -> Loc {
        let mut parts = match tree {
            Tree::Primary => vec![ROOT_NAME.to_string()],
            Tree::Alt => vec![ROOT_NAME.to_string(), self.alt.clone()],
            Tree::Outside => vec![self.out.clone()],
        };
        parts.extend((1..=position).map(|p| self.name(p).to_string()));
        Loc(parts)
    }

    fn leaf_position(&self) -> usize {
        self.depth() + 1
    }

    fn base_steps(&self) -> Vec<Step> {
        let mut steps = vec![
            Step::Dir(self.at(Tree::Alt, 0)),
            Step::Dir(self.at(Tree::Outside, 0)),
        ];
        for tree in [Tree::Primary, Tree::Alt, Tree::Outside] {
            steps.extend((1..=self.depth()).map(|p| Step::Dir(self.at(tree, p))));
            steps.push(Step::File(self.at(tree, self.leaf_position())));
        }
        steps
    }

    fn chain(&self) -> Vec<String> {
        (1..=self.leaf_position())
            .map(|p| self.name(p).to_string())
            .collect()
    }

    /// Link text, for a link sitting at primary `position`, that points at
    /// the same position of another tree.
    fn link_text(&self, position: usize, tree: Tree, absolute: bool) -> LinkText {
        if absolute {
            return LinkText::AbsoluteInWorkspace(self.at(tree, position));
        }
        // The link's directory is `position - 1` levels below the root.
        let (climb, target) = match tree {
            Tree::Outside => (position, self.at(tree, position)),
            _ => (position - 1, Loc(self.at(tree, position).0[1..].to_vec())),
        };
        let mut parts = vec!["..".to_string(); climb];
        parts.extend(target.0);
        LinkText::Relative(parts.join("/"))
    }

    fn aside(&self, position: usize, kind: &str, tag: &str) -> Loc {
        self.at(Tree::Primary, position)
            .with_suffix(&format!(".{kind}-{tag}"))
    }

    /// Move the component aside and put a link in its place.
    fn swap_to_link(&self, position: usize, target: LinkText, tag: &str) -> Vec<Step> {
        let at = self.at(Tree::Primary, position);
        vec![
            Step::Rename {
                from: at.clone(),
                to: self.aside(position, "old", tag),
            },
            Step::Symlink { at, target },
        ]
    }

    /// Move an existing link aside and put a different link in its place.
    fn retarget(&self, position: usize, target: LinkText, tag: &str) -> Vec<Step> {
        let at = self.at(Tree::Primary, position);
        vec![
            Step::Rename {
                from: at.clone(),
                to: self.aside(position, "lnk", tag),
            },
            Step::Symlink { at, target },
        ]
    }

    /// Move the component aside and build a new authorized one in its place.
    fn swap_to_fresh(&self, position: usize, tag: &str) -> Vec<Step> {
        let mut steps = vec![Step::Rename {
            from: self.at(Tree::Primary, position),
            to: self.aside(position, "old", tag),
        }];
        steps.extend((position..=self.depth()).map(|p| Step::Dir(self.at(Tree::Primary, p))));
        steps.push(Step::File(self.at(Tree::Primary, self.leaf_position())));
        steps
    }

    /// Undo `swap_to_link(position, _, old_tag)`.
    fn restore(&self, position: usize, old_tag: &str, tag: &str) -> Vec<Step> {
        let at = self.at(Tree::Primary, position);
        vec![
            Step::Rename {
                from: at.clone(),
                to: self.aside(position, "lnk", tag),
            },
            Step::Rename {
                from: self.aside(position, "old", old_tag),
                to: at,
            },
        ]
    }
}

/// Variant names per family, in rotation order.
pub fn variants(family: Family) -> &'static [&'static str] {
    match family {
        Family::NestedInRoot => &["plain"],
        Family::DotDot => &[
            "inner-dotdot",
            "dot-segments",
            "escape-to-sentinel",
            "deep-escape",
            "reenter-root",
            "absolute-sentinel",
        ],
        Family::SymlinkInRoot | Family::SymlinkOutside => &["relative-link", "absolute-link"],
        Family::SymlinkSwap => &["retarget-to-outside"],
        Family::ParentDirRename => &["fresh-dir", "outside-link"],
        Family::FileReplacement => &["fresh-file", "in-root-link", "outside-link"],
        Family::Retry => &["plain", "swap-after-failure", "fresh-after-failure"],
        Family::CompatFallback => &["plain", "in-root-link", "outside-link", "swap-outside"],
        Family::PrimitiveUnavailable => &["plain", "in-root-link", "outside-link"],
        Family::ToctouBarrier => &["swap-and-restore"],
        Family::BenignControl => &[
            "unrelated-change",
            "in-root-retarget",
            "transient-within-budget",
        ],
    }
}

fn build(seed: u64, family: Family, index: u32, region: DepthRegion) -> Scenario {
    let stream = mix(
        seed,
        ((family.index() as u64) << 16) | ((region.index() as u64) << 12) | u64::from(index),
    );
    let mut rng = SplitMix64::new(stream);
    let (low, high) = match region {
        DepthRegion::InDistribution => protocol::DEPTH_IN_DISTRIBUTION,
        DepthRegion::HeldOutDepth => protocol::DEPTH_HELD_OUT,
    };
    let depth = rng.range(low, high) as usize;
    let layout = Layout::draw(&mut rng, depth);
    let leaf = layout.leaf_position();
    let op = if rng.below(2) == 0 {
        OpKind::Read
    } else {
        OpKind::Write
    };
    let absolute = rng.below(2) == 1;
    let any_position = rng.range(1, leaf as u64) as usize;
    let dir_position = rng.range(1, depth as u64) as usize;
    let timing = rng.range(1, depth as u64 + protocol::TIMING_BEYOND_DEPTH) as u32;
    let transient = rng.range(1, u64::from(protocol::MAX_TRANSIENT_FAILURES)) as u32;
    let transient_at = rng.range(1, u64::from(transient)) as u32;
    let mut fallback_enabled = rng.below(2) == 1;

    let names = variants(family);
    let variant = names[(index as usize + (seed % names.len() as u64) as usize) % names.len()];

    let mut request = RequestPath::Relative(layout.chain().join("/"));
    let mut setup = layout.base_steps();
    let mut schedule = Vec::new();
    let mut faults = Faults::default();
    let mut params = Params {
        depth,
        ..Params::default()
    };

    let inside = |position| layout.link_text(position, Tree::Alt, absolute);
    let outside = |position| layout.link_text(position, Tree::Outside, absolute);
    let before = |timing, label, steps| Mutation {
        at: Barrier::BeforeCall(timing),
        label,
        steps,
    };

    match (family, variant) {
        (Family::NestedInRoot, _) => {}

        (Family::DotDot, variant) => {
            let chain = layout.chain();
            let split = dir_position;
            let up = || "..".to_string();
            let outside_chain = layout.at(Tree::Outside, leaf).0;
            let parts: Vec<String> = match variant {
                "inner-dotdot" => {
                    let mut parts = chain[..split].to_vec();
                    parts.push(up());
                    parts.extend(chain[split - 1..].iter().cloned());
                    parts
                }
                "dot-segments" => {
                    let mut parts = vec![".".to_string()];
                    parts.extend(chain[..split].iter().cloned());
                    parts.push(".".to_string());
                    parts.extend(chain[split..].iter().cloned());
                    parts
                }
                "escape-to-sentinel" => {
                    let mut parts = vec![up()];
                    parts.extend(outside_chain);
                    parts
                }
                "deep-escape" => {
                    let mut parts = chain[..split].to_vec();
                    parts.extend((0..=split).map(|_| up()));
                    parts.extend(outside_chain);
                    parts
                }
                "reenter-root" => {
                    let mut parts = vec![up(), ROOT_NAME.to_string()];
                    parts.extend(chain);
                    parts
                }
                _ => Vec::new(),
            };
            request = if variant == "absolute-sentinel" {
                RequestPath::AbsoluteInWorkspace(layout.at(Tree::Outside, leaf))
            } else {
                RequestPath::Relative(parts.join("/"))
            };
            params.position = Some(split);
        }

        (Family::SymlinkInRoot | Family::SymlinkOutside, variant) => {
            let absolute = variant == "absolute-link";
            let tree = if family == Family::SymlinkInRoot {
                Tree::Alt
            } else {
                Tree::Outside
            };
            let target = layout.link_text(any_position, tree, absolute);
            setup.extend(layout.swap_to_link(any_position, target, "s"));
            params.position = Some(any_position);
            params.absolute_link = Some(absolute);
        }

        (Family::SymlinkSwap, _) => {
            let start = layout.link_text(any_position, Tree::Alt, false);
            setup.extend(layout.swap_to_link(any_position, start, "s"));
            schedule.push(before(
                timing,
                "retarget-link-to-outside",
                layout.retarget(any_position, outside(any_position), "m"),
            ));
            params.position = Some(any_position);
            params.timing = Some(timing);
            params.absolute_link = Some(absolute);
        }

        (Family::ParentDirRename, variant) => {
            let (label, steps) = if variant == "fresh-dir" {
                (
                    "rename-dir-replace-with-fresh",
                    layout.swap_to_fresh(dir_position, "m"),
                )
            } else {
                params.absolute_link = Some(absolute);
                (
                    "rename-dir-replace-with-outside-link",
                    layout.swap_to_link(dir_position, outside(dir_position), "m"),
                )
            };
            schedule.push(before(timing, label, steps));
            params.position = Some(dir_position);
            params.timing = Some(timing);
        }

        (Family::FileReplacement, variant) => {
            let (label, steps) = match variant {
                "fresh-file" => ("replace-file-with-fresh", layout.swap_to_fresh(leaf, "m")),
                "in-root-link" => (
                    "replace-file-with-in-root-link",
                    layout.swap_to_link(leaf, inside(leaf), "m"),
                ),
                _ => (
                    "replace-file-with-outside-link",
                    layout.swap_to_link(leaf, outside(leaf), "m"),
                ),
            };
            if variant != "fresh-file" {
                params.absolute_link = Some(absolute);
            }
            schedule.push(before(timing, label, steps));
            params.position = Some(leaf);
            params.timing = Some(timing);
        }

        (Family::Retry, variant) => {
            faults.transient_failures = transient;
            params.transient = transient;
            let mutation = match variant {
                "swap-after-failure" => Some((
                    "swap-to-outside-link",
                    layout.swap_to_link(any_position, outside(any_position), "m"),
                )),
                "fresh-after-failure" => Some((
                    "replace-with-fresh",
                    layout.swap_to_fresh(any_position, "m"),
                )),
                _ => None,
            };
            if let Some((label, steps)) = mutation {
                schedule.push(Mutation {
                    at: Barrier::AfterTransient(transient_at),
                    label,
                    steps,
                });
                params.position = Some(any_position);
                if variant == "swap-after-failure" {
                    params.absolute_link = Some(absolute);
                }
            }
        }

        (Family::CompatFallback | Family::PrimitiveUnavailable, variant) => {
            faults.nonlegacy_unavailable = true;
            fallback_enabled = family == Family::CompatFallback;
            match variant {
                "in-root-link" => {
                    setup.extend(layout.swap_to_link(any_position, inside(any_position), "s"));
                }
                "outside-link" => {
                    setup.extend(layout.swap_to_link(any_position, outside(any_position), "s"));
                }
                "swap-outside" => {
                    schedule.push(before(
                        timing,
                        "swap-to-outside-link",
                        layout.swap_to_link(any_position, outside(any_position), "m"),
                    ));
                    params.timing = Some(timing);
                }
                _ => {}
            }
            if variant != "plain" {
                params.position = Some(any_position);
                params.absolute_link = Some(absolute);
            }
        }

        (Family::ToctouBarrier, _) => {
            // The restore needs a later call to exist for at least the
            // longest adapter, so the swap is drawn one call earlier.
            let swap_at = timing.min(depth as u32 + 2);
            schedule.push(before(
                swap_at,
                "swap-to-outside-link",
                layout.swap_to_link(any_position, outside(any_position), "m"),
            ));
            schedule.push(before(
                swap_at + 1,
                "restore-original",
                layout.restore(any_position, "m", "r"),
            ));
            params.position = Some(any_position);
            params.timing = Some(swap_at);
            params.absolute_link = Some(absolute);
        }

        (Family::BenignControl, variant) => match variant {
            "unrelated-change" => {
                let noise = Loc(vec![
                    ROOT_NAME.to_string(),
                    format!("noise-{:04x}", rng.below(0x1_0000)),
                ]);
                schedule.push(before(
                    timing,
                    "unrelated-in-root-change",
                    vec![
                        Step::File(noise.clone()),
                        Step::Rename {
                            from: noise.clone(),
                            to: noise.with_suffix(".moved"),
                        },
                    ],
                ));
                params.timing = Some(timing);
            }
            "in-root-retarget" => {
                let start = layout.link_text(any_position, Tree::Alt, false);
                setup.extend(layout.swap_to_link(any_position, start, "s"));
                // The original, set aside as a sibling, is still authorized.
                let sibling = format!("{}.old-s", layout.name(any_position));
                schedule.push(before(
                    timing,
                    "retarget-link-within-root",
                    layout.retarget(any_position, LinkText::Relative(sibling), "m"),
                ));
                params.position = Some(any_position);
                params.timing = Some(timing);
                params.absolute_link = Some(false);
            }
            _ => {
                let within = transient.min(protocol::RETRY_BUDGET);
                faults.transient_failures = within;
                params.transient = within;
            }
        },
    }

    let suffix = match region {
        DepthRegion::InDistribution => "",
        DepthRegion::HeldOutDepth => "-deep",
    };
    Scenario {
        id: format!("s{seed}-{family}-{index:02}{suffix}"),
        seed,
        family,
        variant,
        region,
        op,
        request,
        setup,
        schedule,
        faults,
        contract: Contract {
            retry_budget: protocol::RETRY_BUDGET,
            fallback_enabled,
        },
        params,
    }
}

/// Every scenario for one seed. Held-out seeds additionally get scenarios
/// from the held-out depth region, which development never generates.
pub fn generate(seed: u64, split: Split) -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    for family in Family::ALL {
        for index in 0..protocol::INSTANCES_PER_FAMILY {
            scenarios.push(build(seed, *family, index, DepthRegion::InDistribution));
        }
        if split == Split::HeldOut {
            for index in 0..protocol::HELD_OUT_DEPTH_INSTANCES_PER_FAMILY {
                scenarios.push(build(seed, *family, index, DepthRegion::HeldOutDepth));
            }
        }
    }
    scenarios
}

fn plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\0')
}

fn check_loc(loc: &Loc) -> Result<(), String> {
    if loc.0.is_empty() || !loc.0.iter().all(|name| plain_name(name)) {
        return Err(format!(
            "`{}` is not a plain workspace location",
            loc.display()
        ));
    }
    Ok(())
}

fn check_step(step: &Step, in_root_only: bool) -> Result<bool, String> {
    let mut locs: Vec<&Loc> = Vec::new();
    let mut is_link = false;
    match step {
        Step::Dir(loc) | Step::File(loc) => locs.push(loc),
        Step::Rename { from, to } => {
            if from.region() != to.region() {
                return Err("a rename crosses the policy boundary".to_string());
            }
            locs.extend([from, to]);
        }
        Step::Symlink { at, target } => {
            is_link = true;
            locs.push(at);
            match target {
                LinkText::AbsoluteInWorkspace(loc) => check_loc(loc)?,
                LinkText::Relative(text) => {
                    // `..` only as a leading run, and never more of them
                    // than the link's directory is deep: the target cannot
                    // name anything above the workspace base.
                    let parts: Vec<&str> = text.split('/').collect();
                    let climbs = parts.iter().take_while(|p| **p == "..").count();
                    if climbs == parts.len() || !parts[climbs..].iter().all(|p| plain_name(p)) {
                        return Err(format!("link text `{text}` has an unexpected shape"));
                    }
                    if climbs > at.0.len().saturating_sub(1) {
                        return Err(format!("link text `{text}` climbs above the workspace"));
                    }
                }
            }
        }
    }
    for loc in locs {
        check_loc(loc)?;
        if in_root_only && loc.0[0] != ROOT_NAME {
            return Err(format!(
                "a scheduled mutation touches `{}`, outside the root",
                loc.display()
            ));
        }
    }
    Ok(is_link)
}

impl Scenario {
    /// One-line description of the schedule and faults.
    pub fn schedule_label(&self) -> String {
        let mut parts: Vec<String> = self
            .schedule
            .iter()
            .map(|m| format!("{}:{}", m.at.label(), m.label))
            .collect();
        if self.faults.transient_failures > 0 {
            parts.push(format!(
                "transient-failures={}",
                self.faults.transient_failures
            ));
        }
        if self.faults.nonlegacy_unavailable {
            parts.push("nonlegacy-unavailable".to_string());
        }
        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(";")
        }
    }

    /// Static confinement validation: a proof, from the scenario's data
    /// alone, that nothing it names can lie outside the temporary workspace
    /// and that no scheduled mutation touches anything outside the root.
    pub fn validate(&self) -> Result<(), String> {
        let mut has_link = false;
        for step in &self.setup {
            has_link |= check_step(step, false)?;
        }
        for mutation in &self.schedule {
            for step in &mutation.steps {
                has_link |= check_step(step, true)?;
            }
        }
        match &self.request {
            RequestPath::AbsoluteInWorkspace(loc) => check_loc(loc)?,
            RequestPath::Relative(text) => {
                // Without `..` a request cannot climb at all. With `..` it
                // is allowed only in a fixture with no symlinks, where the
                // textual depth is the real depth; the root sits one level
                // below the workspace base.
                let mut depth: i64 = 1;
                let mut climbs = false;
                for part in text.split('/') {
                    match part {
                        "." => {}
                        ".." => {
                            climbs = true;
                            depth -= 1;
                        }
                        name if plain_name(name) => depth += 1,
                        other => return Err(format!("request component `{other}` is not allowed")),
                    }
                    if depth < 0 {
                        return Err("the request climbs above the workspace".to_string());
                    }
                }
                if climbs && has_link {
                    return Err("a request with `..` in a fixture with symlinks".to_string());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Graph;
    use crate::fixture::Kind;
    use crate::oracle::{denote, Denotation};

    const DEV: std::ops::RangeInclusive<u64> = 1..=20;

    #[test]
    fn generation_is_deterministic_and_seed_dependent() {
        assert_eq!(
            generate(3, Split::Development),
            generate(3, Split::Development)
        );
        assert_ne!(
            generate(3, Split::Development),
            generate(4, Split::Development)
        );
    }

    #[test]
    fn every_family_and_variant_appears_for_every_seed() {
        for seed in DEV {
            let scenarios = generate(seed, Split::Development);
            for family in Family::ALL {
                let seen: std::collections::BTreeSet<&str> = scenarios
                    .iter()
                    .filter(|s| s.family == *family)
                    .map(|s| s.variant)
                    .collect();
                assert_eq!(seen.len(), variants(*family).len(), "seed {seed} {family}");
            }
        }
    }

    #[test]
    fn the_deep_region_exists_only_in_the_held_out_split() {
        // Seed 7 is a development seed; the split flag alone decides.
        let dev = generate(7, Split::Development);
        assert!(dev.iter().all(|s| s.region == DepthRegion::InDistribution));
        assert!(dev.iter().all(|s| s.params.depth <= 4));
        let held = generate(7, Split::HeldOut);
        let deep: Vec<_> = held
            .iter()
            .filter(|s| s.region == DepthRegion::HeldOutDepth)
            .collect();
        assert_eq!(
            deep.len(),
            12 * protocol::HELD_OUT_DEPTH_INSTANCES_PER_FAMILY as usize
        );
        assert!(deep.iter().all(|s| (5..=6).contains(&s.params.depth)));
        // The in-distribution part is identical in both splits.
        assert_eq!(
            dev[..],
            held.iter()
                .filter(|s| s.region == DepthRegion::InDistribution)
                .cloned()
                .collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn ids_are_unique() {
        let scenarios = generate(5, Split::HeldOut);
        let ids: std::collections::BTreeSet<&str> =
            scenarios.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids.len(), scenarios.len());
    }

    #[test]
    fn every_generated_scenario_passes_confinement_validation() {
        for seed in DEV {
            for scenario in generate(seed, Split::HeldOut) {
                scenario
                    .validate()
                    .unwrap_or_else(|e| panic!("{}: {e}", scenario.id));
            }
        }
    }

    /// In every fixture state a schedule can reach, the model must be able
    /// to apply the steps and the request must denote a regular file.
    #[test]
    fn every_schedule_prefix_is_applicable_and_denotes_a_file() {
        for seed in DEV {
            for scenario in generate(seed, Split::HeldOut) {
                let mut graph = Graph::new();
                for step in &scenario.setup {
                    graph
                        .apply(step)
                        .unwrap_or_else(|e| panic!("{}: {e}", scenario.id));
                }
                let check = |graph: &Graph| match denote(graph, &scenario.request) {
                    Denotation::Object(id) => {
                        assert_eq!(graph.node(id).kind, Kind::File, "{}", scenario.id)
                    }
                    other => panic!("{}: request denotes {other:?}", scenario.id),
                };
                check(&graph);
                for mutation in &scenario.schedule {
                    for step in &mutation.steps {
                        graph
                            .apply(step)
                            .unwrap_or_else(|e| panic!("{}: {e}", scenario.id));
                    }
                    check(&graph);
                }
            }
        }
    }

    #[test]
    fn validation_rejects_anything_that_could_leave_the_workspace() {
        let base = generate(1, Split::Development)
            .into_iter()
            .find(|s| s.family == Family::NestedInRoot)
            .unwrap();
        let with_request = |text: &str| Scenario {
            request: RequestPath::Relative(text.to_string()),
            ..base.clone()
        };
        assert!(with_request("a/b").validate().is_ok());
        assert!(with_request("../x").validate().is_ok());
        assert!(with_request("../../x").validate().is_err());
        assert!(with_request("a/../../../x").validate().is_err());
        assert!(with_request("/absolute/path").validate().is_err());

        let with_setup = |step: Step| {
            let mut s = base.clone();
            s.setup.push(step);
            s
        };
        let link = |at: &str, text: &str| Step::Symlink {
            at: Loc::parse(at),
            target: LinkText::Relative(text.to_string()),
        };
        assert!(with_setup(link("root/l", "../x")).validate().is_ok());
        assert!(with_setup(link("root/l", "../../x")).validate().is_err());
        assert!(with_setup(link("root/l", "a/../../../x"))
            .validate()
            .is_err());
        assert!(with_setup(link("root/l", "/absolute")).validate().is_err());
        assert!(with_setup(Step::Dir(Loc::parse("root/../x")))
            .validate()
            .is_err());

        // `..` in the request is not allowed once any link exists.
        let mut mixed = with_setup(link("root/l", "x"));
        mixed.request = RequestPath::Relative("l/../x".to_string());
        assert!(mixed.validate().is_err());

        // Scheduled mutations must stay inside the root.
        let mut outside_mutation = base.clone();
        outside_mutation.schedule.push(Mutation {
            at: Barrier::BeforeCall(1),
            label: "bad",
            steps: vec![Step::File(Loc::parse("elsewhere/x"))],
        });
        assert!(outside_mutation.validate().is_err());
    }

    #[test]
    fn fallback_is_forced_on_and_off_in_the_two_unavailability_families() {
        for scenario in generate(2, Split::Development) {
            match scenario.family {
                Family::CompatFallback => {
                    assert!(
                        scenario.faults.nonlegacy_unavailable && scenario.contract.fallback_enabled
                    )
                }
                Family::PrimitiveUnavailable => {
                    assert!(
                        scenario.faults.nonlegacy_unavailable
                            && !scenario.contract.fallback_enabled
                    )
                }
                _ => assert!(!scenario.faults.nonlegacy_unavailable),
            }
        }
    }
}
