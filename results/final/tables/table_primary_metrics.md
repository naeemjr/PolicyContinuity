# Primary metrics by adapter

| Primary metric | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| Containment-preservation rate | 80.4% [79.9%, 80.9%] | 92.9% [92.5%, 93.3%] | 97.3% [97.1%, 97.4%] | 100% [100%, 100%] |
| Completed containment-violation rate | 21.0% [20.5%, 21.6%] | 9.8% [9.2%, 10.4%] | 5.6% [5.3%, 6.0%] | 0% [0%, 0%] |
| Safe-refusal rate under unsafe conditions | 17.0% [16.2%, 17.8%] | 67.8% [66.6%, 68.9%] | 70.0% [68.8%, 71.3%] | 96.6% [95.9%, 97.3%] |
| False-refusal rate under safe conditions | 0% [0%, 0%] | 0% [0%, 0%] | 29.1% [28.2%, 29.9%] | 7.5% [7.2%, 7.7%] |
| Availability under safe conditions | 97.8% [97.4%, 98.2%] | 89.9% [89.5%, 90.3%] | 60.0% [59.1%, 61.0%] | 75.1% [74.6%, 75.7%] |
| Fallback policy-drift rate | n/a | 28.7% [27.1%, 30.6%] | 32.9% [30.8%, 35.2%] | n/a |

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Containment-preservation rate** = conclusive runs not labelled containment_violation / conclusive runs (all runs minus evaluator_inconclusive).
- **Completed containment-violation rate** = runs labelled containment_violation / runs labelled preserved or containment_violation (completed operations).
- **Safe-refusal rate under unsafe conditions** = runs labelled safe_refusal, plus unsupported_safe runs under an unsafe condition / runs under an unsafe condition.
- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
- **Availability under safe conditions** = runs labelled preserved under a safe condition / runs under a safe condition.
- **Fallback policy-drift rate** = fallback-path runs labelled containment_violation whose fault-free twin was not / runs that took the fallback path.
