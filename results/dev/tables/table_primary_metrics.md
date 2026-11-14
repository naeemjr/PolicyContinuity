# Primary metrics by adapter

| Primary metric | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| Containment-preservation rate | 80.4% [79.4%, 81.4%] | 92.5% [91.8%, 93.2%] | 97.2% [96.9%, 97.5%] | 100% [100%, 100%] |
| Completed containment-violation rate | 21.0% [20.0%, 22.0%] | 10.4% [9.5%, 11.3%] | 5.9% [5.2%, 6.6%] | 0% [0%, 0%] |
| Safe-refusal rate under unsafe conditions | 17.0% [15.9%, 18.3%] | 65.7% [63.3%, 68.0%] | 72.6% [70.1%, 75.2%] | 95.5% [94.2%, 96.8%] |
| False-refusal rate under safe conditions | 0% [0%, 0%] | 0% [0%, 0%] | 28.5% [27.4%, 29.5%] | 7.4% [7.1%, 7.8%] |
| Availability under safe conditions | 97.8% [97.4%, 98.1%] | 89.7% [89.2%, 90.2%] | 60.6% [59.4%, 61.8%] | 75.2% [74.5%, 76.0%] |
| Fallback policy-drift rate | n/a | 30.0% [26.9%, 33.1%] | 33.8% [30.0%, 37.5%] | n/a |

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Containment-preservation rate** = conclusive runs not labelled containment_violation / conclusive runs (all runs minus evaluator_inconclusive).
- **Completed containment-violation rate** = runs labelled containment_violation / runs labelled preserved or containment_violation (completed operations).
- **Safe-refusal rate under unsafe conditions** = runs labelled safe_refusal, plus unsupported_safe runs under an unsafe condition / runs under an unsafe condition.
- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
- **Availability under safe conditions** = runs labelled preserved under a safe condition / runs under a safe condition.
- **Fallback policy-drift rate** = fallback-path runs labelled containment_violation whose fault-free twin was not / runs that took the fallback path.
