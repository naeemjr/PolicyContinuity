//! Benchmark adapter: kernel-enforced resolution, refusing when unavailable.

use super::{Adapter, Fallback, Plan, Request, Stop};
use crate::env::Env;

/// Hands the whole request to `openat2` with `RESOLVE_BENEATH` relative to
/// the root handle and does nothing else. If the primitive is unavailable it
/// reports that and stops: it has no weaker path to take.
pub struct StrictSafeOpen;

impl Adapter for StrictSafeOpen {
    fn name(&self) -> &'static str {
        "strict-safe-open"
    }

    fn strategy(&self) -> &'static str {
        "openat2(RESOLVE_BENEATH) from the root handle; report unsupported if unavailable"
    }

    fn contract_version(&self) -> u32 {
        1
    }

    fn fallback(&self) -> Fallback {
        Fallback::Never
    }

    fn prepare(&self, _env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
        Ok(Plan::Beneath(request.path.to_path_buf()))
    }
}
