//! Benchmark adapter: lexical normalisation plus a prefix check.

use super::{legacy_plan, Adapter, Fallback, Plan, Request, Stop};
use crate::env::Env;

/// Joins the request to the root, normalises the result as text, checks it
/// is beneath the root component by component, and opens it by path. The
/// check never consults the filesystem, so it knows nothing about symlinks.
///
/// This strategy is also the "legacy routine" other adapters fall back to.
pub struct LexicalPrefix;

impl Adapter for LexicalPrefix {
    fn name(&self) -> &'static str {
        "lexical-prefix"
    }

    fn strategy(&self) -> &'static str {
        "normalise the joined path as text, check it is beneath the root, open it by path"
    }

    fn contract_version(&self) -> u32 {
        2
    }

    fn fallback(&self) -> Fallback {
        // It uses only the legacy primitive, so it never meets the fault.
        Fallback::NotApplicable
    }

    fn prepare(&self, env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
        legacy_plan(env, request)
    }
}
