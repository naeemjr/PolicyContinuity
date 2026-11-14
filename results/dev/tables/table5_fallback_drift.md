# Table 5. Fallback policy drift by adapter

| Adapter | Declared fallback rule | Runs with the primitive unavailable | Took the fallback path | Reported unsupported | Fallback-path violations | Of which drift | Fallback policy-drift rate |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | not-applicable | 0 | 0 | 0 | 0 | 0 | n/a |
| canonicalize-then-open | legacy-when-enabled | 320 | 160 | 160 | 54 | 48 | 30.0% [26.9%, 33.1%] |
| dirfd-relative | legacy-when-enabled | 320 | 160 | 160 | 54 | 54 | 33.8% [30.0%, 37.5%] |
| strict-safe-open | never | 320 | 0 | 320 | 0 | 0 | n/a |

A run "drifts" when it took the fallback path and was labelled containment_violation while the same scenario and adapter, with the unavailability fault removed, was not. `n/a` means the adapter never took a fallback path, so the rate has no denominator.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Fallback policy-drift rate** = fallback-path runs labelled containment_violation whose fault-free twin was not / runs that took the fallback path.
- **Violation rate on the fallback path** = fallback-path runs labelled containment_violation / runs that took the fallback path.
