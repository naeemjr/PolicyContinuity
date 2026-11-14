# Table 5. Fallback policy drift by adapter

| Adapter | Declared fallback rule | Runs with the primitive unavailable | Took the fallback path | Reported unsupported | Fallback-path violations | Of which drift | Fallback policy-drift rate |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | not-applicable | 0 | 0 | 0 | 0 | 0 | n/a |
| canonicalize-then-open | legacy-when-enabled | 960 | 480 | 480 | 158 | 138 | 28.7% [27.1%, 30.6%] |
| dirfd-relative | legacy-when-enabled | 960 | 480 | 480 | 158 | 158 | 32.9% [30.8%, 35.2%] |
| strict-safe-open | never | 960 | 0 | 960 | 0 | 0 | n/a |

A run "drifts" when it took the fallback path and was labelled containment_violation while the same scenario and adapter, with the unavailability fault removed, was not. `n/a` means the adapter never took a fallback path, so the rate has no denominator.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Fallback policy-drift rate** = fallback-path runs labelled containment_violation whose fault-free twin was not / runs that took the fallback path.
- **Violation rate on the fallback path** = fallback-path runs labelled containment_violation / runs that took the fallback path.
