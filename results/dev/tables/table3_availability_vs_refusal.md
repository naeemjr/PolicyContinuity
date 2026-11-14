# Table 3. Availability against safe refusal, by adapter

| Adapter | Availability under safe conditions | False-refusal rate under safe conditions | Safe-refusal rate under unsafe conditions | Violation rate under unsafe conditions | Unsupported-safe rate (all runs) | Ordinary-failure rate (all runs) |
| --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | 97.8% [97.4%, 98.1%] | 0% [0%, 0%] | 17.0% [15.9%, 18.3%] | 78.5% [76.6%, 80.4%] | 0% [0%, 0%] | 2.8% [2.3%, 3.2%] |
| canonicalize-then-open | 89.7% [89.2%, 90.2%] | 0% [0%, 0%] | 65.7% [63.3%, 68.0%] | 25.0% [23.0%, 27.2%] | 8.3% [8.3%, 8.3%] | 2.8% [2.3%, 3.2%] |
| dirfd-relative | 60.6% [59.4%, 61.8%] | 28.5% [27.4%, 29.5%] | 72.6% [70.1%, 75.2%] | 8.2% [7.3%, 9.1%] | 8.3% [8.3%, 8.3%] | 2.8% [2.3%, 3.2%] |
| strict-safe-open | 75.2% [74.5%, 76.0%] | 7.4% [7.1%, 7.8%] | 95.5% [94.2%, 96.8%] | 0% [0%, 0%] | 16.7% [16.7%, 16.7%] | 2.8% [2.3%, 3.2%] |

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Availability under safe conditions** = runs labelled preserved under a safe condition / runs under a safe condition.
- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
- **Safe-refusal rate under unsafe conditions** = runs labelled safe_refusal, plus unsupported_safe runs under an unsafe condition / runs under an unsafe condition.
- **Violation rate under unsafe conditions** = runs labelled containment_violation / runs under an unsafe condition.
- **Unsupported-safe rate (all runs)** = runs labelled unsupported_safe / all runs.
- **Ordinary-failure rate (all runs)** = runs labelled ordinary_failure / all runs.
