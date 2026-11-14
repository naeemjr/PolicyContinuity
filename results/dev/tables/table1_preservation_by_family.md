# Table 1. Containment-preservation rate by adapter and scenario family

| Scenario family | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| dotdot-normalization | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| symlink-in-root-target | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| symlink-outside-target | 0% [0%, 0%] (0/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| symlink-swap | 83.1% [78.1%, 88.1%] (133/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| parent-dir-rename | 91.2% [86.9%, 95.0%] (146/160) | 88.8% [84.4%, 92.5%] (142/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| file-replacement | 93.1% [90.6%, 95.6%] (149/160) | 94.4% [90.0%, 98.1%] (151/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| retry-after-failure | 80.0% [76.2%, 83.8%] (128/160) | 80.0% [76.2%, 83.8%] (128/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| compat-fallback | 70.0% [66.9%, 73.1%] (112/160) | 66.2% [62.5%, 70.0%] (106/160) | 66.2% [62.5%, 70.0%] (106/160) | 100% [100%, 100%] (160/160) |
| primitive-unavailable | 66.2% [63.7%, 68.8%] (106/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| toctou-barrier | 81.2% [75.0%, 87.5%] (130/160) | 80.6% [75.0%, 86.2%] (129/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |
| benign-control | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) | 100% [100%, 100%] (160/160) |

The fraction in brackets is the pooled numerator and denominator over all seeds.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Containment-preservation rate** = conclusive runs not labelled containment_violation / conclusive runs (all runs minus evaluator_inconclusive).
