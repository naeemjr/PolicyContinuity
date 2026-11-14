//! Small shared vocabulary types used across the benchmark.

/// Declares a fieldless enum whose variants each carry a stable label. The
/// label is what appears in JSON, CSV, scenario ids, and the CLI.
macro_rules! labelled_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $variant:ident => $label:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $label),+
                }
            }

            /// Position in [`Self::ALL`].
            pub fn index(self) -> usize {
                self as usize
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|v| v.as_str() == s)
                    .ok_or_else(|| {
                        let known: Vec<&str> = Self::ALL.iter().map(|v| v.as_str()).collect();
                        format!("unknown value `{s}` (expected one of: {})", known.join(", "))
                    })
            }
        }
    };
}

labelled_enum! {
    /// Scenario families of the frozen matrix, in report order.
    pub enum Family {
        NestedInRoot => "nested-in-root",
        DotDot => "dotdot-normalization",
        SymlinkInRoot => "symlink-in-root-target",
        SymlinkOutside => "symlink-outside-target",
        SymlinkSwap => "symlink-swap",
        ParentDirRename => "parent-dir-rename",
        FileReplacement => "file-replacement",
        Retry => "retry-after-failure",
        CompatFallback => "compat-fallback",
        PrimitiveUnavailable => "primitive-unavailable",
        ToctouBarrier => "toctou-barrier",
        BenignControl => "benign-control",
    }
}

labelled_enum! {
    /// The operation requested on the path. Neither creates nor truncates.
    pub enum OpKind {
        /// Open read-only; the harness then reads through the handle.
        Read => "read",
        /// Open write-only; the harness then writes a marker through the handle.
        Write => "write",
    }
}

labelled_enum! {
    /// Which side of the policy boundary a fixture object was created on.
    /// Fixed for the life of the object: no fixture step moves an object
    /// across the boundary.
    pub enum Region {
        /// Created at or beneath the approved root.
        Authorized => "authorized",
        /// Created in the temporary workspace but outside the approved root.
        /// Regular files here are the synthetic sentinels.
        Outside => "outside",
    }
}

labelled_enum! {
    /// What the adapter reported about its own run. The evaluator treats this
    /// as a claim to be classified, never as evidence of containment.
    pub enum Reported {
        Completed => "completed",
        /// A policy decision not to proceed.
        Refused => "refused",
        /// An error the adapter did not treat as a policy decision.
        Failed => "failed",
        /// A primitive the adapter requires was unavailable and its contract
        /// does not permit a substitute.
        Unsupported => "unsupported",
    }
}

labelled_enum! {
    /// The evaluator's label for one run. Exactly one per run.
    pub enum RunOutcome {
        Preserved => "preserved",
        ContainmentViolation => "containment_violation",
        SafeRefusal => "safe_refusal",
        FalseRefusal => "false_refusal",
        OrdinaryFailure => "ordinary_failure",
        UnsupportedSafe => "unsupported_safe",
        EvaluatorInconclusive => "evaluator_inconclusive",
    }
}

impl RunOutcome {
    /// Severity rank for the worst-case table: higher is worse.
    pub fn severity(self) -> u8 {
        match self {
            RunOutcome::Preserved => 0,
            RunOutcome::SafeRefusal => 1,
            RunOutcome::OrdinaryFailure => 2,
            RunOutcome::UnsupportedSafe => 3,
            RunOutcome::FalseRefusal => 4,
            RunOutcome::EvaluatorInconclusive => 5,
            RunOutcome::ContainmentViolation => 6,
        }
    }
}

labelled_enum! {
    /// What the fixture state made of the request during the run, decided by
    /// the evaluator from its own model of the fixture.
    pub enum Condition {
        /// In every fixture state realized during the run, the request
        /// denoted an authorized file.
        Safe => "safe",
        /// In at least one realized state, the request denoted an outside
        /// sentinel.
        Unsafe => "unsafe",
        /// The model could not say (the request denoted no file in some state).
        Undetermined => "undetermined",
    }
}

labelled_enum! {
    /// Which branch of its logic an adapter ended up on, derived by the
    /// harness from the primitive calls it observed.
    pub enum PathLabel {
        Primary => "primary",
        /// A file open was attempted again after an injected transient failure.
        Retry => "retry",
        /// The legacy path open was used after a non-legacy primitive
        /// reported unavailable.
        Fallback => "fallback",
        FallbackRetry => "fallback+retry",
    }
}

labelled_enum! {
    /// The filesystem primitives the environment offers to adapters.
    pub enum Primitive {
        /// `realpath` of a path.
        Canonicalize => "canonicalize",
        /// Legacy path open: `open(path)`, following symlinks.
        OpenPath => "open-path",
        /// One directory step: `openat(dir, name, O_DIRECTORY|O_NOFOLLOW)`.
        OpenDirAt => "open-dir-at",
        /// Final step: `openat(dir, name, O_NOFOLLOW)`.
        OpenFileAt => "open-file-at",
        /// `openat2(root, path, RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS)`.
        OpenBeneath => "open-beneath",
    }
}

impl PathLabel {
    pub fn is_fallback(self) -> bool {
        matches!(self, PathLabel::Fallback | PathLabel::FallbackRetry)
    }

    pub fn is_retry(self) -> bool {
        matches!(self, PathLabel::Retry | PathLabel::FallbackRetry)
    }
}

impl Primitive {
    /// Every primitive except the plain path open is "non-legacy": the
    /// unavailability fault removes all of them at once.
    pub fn is_legacy(self) -> bool {
        self == Primitive::OpenPath
    }

    /// Whether a successful call yields the handle the operation acts on.
    pub fn yields_file(self) -> bool {
        matches!(
            self,
            Primitive::OpenPath | Primitive::OpenFileAt | Primitive::OpenBeneath
        )
    }
}

labelled_enum! {
    /// Which seed list a run belongs to.
    pub enum Split {
        Development => "development",
        HeldOut => "held-out",
    }
}

labelled_enum! {
    /// Parameter region a scenario was drawn from.
    pub enum DepthRegion {
        /// Nesting depth 1 to 4: used in development and in the final run.
        InDistribution => "in-distribution",
        /// Nesting depth 5 to 6: generated only for held-out seeds.
        HeldOutDepth => "held-out-depth",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip() {
        for family in Family::ALL {
            assert_eq!(family.as_str().parse::<Family>().unwrap(), *family);
        }
        assert!("nope".parse::<Family>().is_err());
    }

    #[test]
    fn there_are_twelve_families_and_seven_outcomes() {
        assert_eq!(Family::ALL.len(), 12);
        assert_eq!(RunOutcome::ALL.len(), 7);
    }

    #[test]
    fn index_matches_position() {
        for (i, outcome) in RunOutcome::ALL.iter().enumerate() {
            assert_eq!(outcome.index(), i);
        }
    }

    #[test]
    fn a_violation_is_the_worst_outcome() {
        let worst = RunOutcome::ALL.iter().max_by_key(|o| o.severity()).unwrap();
        assert_eq!(*worst, RunOutcome::ContainmentViolation);
    }

    #[test]
    fn only_the_path_open_is_legacy() {
        let legacy: Vec<_> = Primitive::ALL.iter().filter(|p| p.is_legacy()).collect();
        assert_eq!(legacy, [&Primitive::OpenPath]);
    }
}
