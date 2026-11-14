//! Benchmark adapter: resolve the real path, check it, then open by path.

use super::{failure, unavailable, Adapter, Fallback, Outcome, Plan, Request, Stop};
use crate::env::Env;

/// Resolves every symlink with `realpath`, checks the result is beneath the
/// root, then opens the resolved path. The check is sound for the instant it
/// runs; the open is a second, separate resolution of a path string.
pub struct CanonicalizeThenOpen;

impl Adapter for CanonicalizeThenOpen {
    fn name(&self) -> &'static str {
        "canonicalize-then-open"
    }

    fn strategy(&self) -> &'static str {
        "realpath the joined path, check it is beneath the root, open the resolved path"
    }

    fn contract_version(&self) -> u32 {
        2
    }

    fn fallback(&self) -> Fallback {
        Fallback::LegacyWhenEnabled
    }

    fn prepare(&self, env: &Env<'_>, request: &Request<'_>) -> Result<Plan, Stop> {
        let real = match env.canonicalize(&env.root().join(request.path)) {
            Ok(real) => real,
            Err(err) if unavailable(&err) => return Err(Stop::Unavailable),
            Err(err) => return Err(Stop::Done(failure(&err))),
        };
        if !real.starts_with(env.root()) {
            return Err(Stop::Done(Outcome::Refused(
                "resolved path is not beneath the root",
            )));
        }
        Ok(Plan::Path(real))
    }
}
