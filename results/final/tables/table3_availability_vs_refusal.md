# Table 3. Availability against safe refusal, by adapter

| Adapter | Availability under safe conditions | False-refusal rate under safe conditions | Safe-refusal rate under unsafe conditions | Violation rate under unsafe conditions | Unsupported-safe rate (all runs) | Ordinary-failure rate (all runs) |
| --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | 97.8% [97.4%, 98.2%] | 0% [0%, 0%] | 17.0% [16.2%, 17.8%] | 79.6% [78.6%, 80.7%] | 0% [0%, 0%] | 2.5% [2.2%, 2.7%] |
| canonicalize-then-open | 89.9% [89.5%, 90.3%] | 0% [0%, 0%] | 67.8% [66.6%, 68.9%] | 24.6% [23.4%, 25.9%] | 8.3% [8.3%, 8.3%] | 2.5% [2.2%, 2.7%] |
| dirfd-relative | 60.0% [59.1%, 61.0%] | 29.1% [28.2%, 29.9%] | 70.0% [68.8%, 71.3%] | 8.0% [7.5%, 8.6%] | 8.3% [8.3%, 8.3%] | 2.5% [2.2%, 2.7%] |
| strict-safe-open | 75.1% [74.6%, 75.7%] | 7.5% [7.2%, 7.7%] | 96.6% [95.9%, 97.3%] | 0% [0%, 0%] | 16.7% [16.7%, 16.7%] | 2.5% [2.2%, 2.7%] |

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Availability under safe conditions** = runs labelled preserved under a safe condition / runs under a safe condition.
- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
- **Safe-refusal rate under unsafe conditions** = runs labelled safe_refusal, plus unsupported_safe runs under an unsafe condition / runs under an unsafe condition.
- **Violation rate under unsafe conditions** = runs labelled containment_violation / runs under an unsafe condition.
- **Unsupported-safe rate (all runs)** = runs labelled unsupported_safe / all runs.
- **Ordinary-failure rate (all runs)** = runs labelled ordinary_failure / all runs.
