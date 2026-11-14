# Table 1. Containment-preservation rate by adapter and scenario family

| Scenario family | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| dotdot-normalization | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| symlink-in-root-target | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| symlink-outside-target | 0% [0%, 0%] (0/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| symlink-swap | 83.1% [80.2%, 86.0%] (399/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| parent-dir-rename | 89.6% [87.3%, 91.9%] (430/480) | 91.2% [89.4%, 93.1%] (438/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| file-replacement | 94.2% [92.5%, 95.8%] (452/480) | 94.8% [92.5%, 96.9%] (455/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| retry-after-failure | 76.9% [74.2%, 79.6%] (369/480) | 76.9% [74.2%, 79.6%] (369/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| compat-fallback | 71.3% [69.4%, 72.9%] (342/480) | 67.1% [64.8%, 69.2%] (322/480) | 67.1% [64.8%, 69.4%] (322/480) | 100% [100%, 100%] (480/480) |
| primitive-unavailable | 66.5% [64.4%, 68.5%] (319/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| toctou-barrier | 82.9% [80.2%, 85.8%] (398/480) | 84.8% [81.7%, 87.7%] (407/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |
| benign-control | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) | 100% [100%, 100%] (480/480) |

The fraction in brackets is the pooled numerator and denominator over all seeds.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Containment-preservation rate** = conclusive runs not labelled containment_violation / conclusive runs (all runs minus evaluator_inconclusive).
