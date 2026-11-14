# Table 2. Completed containment-violation rate by adapter and scenario family

| Scenario family | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/160) |
| dotdot-normalization | 0% [0%, 0%] (0/79) | 0% [0%, 0%] (0/79) | 0% [0%, 0%] (0/53) | 0% [0%, 0%] (0/53) |
| symlink-in-root-target | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/160) | n/a (0/0) | 0% [0%, 0%] (0/80) |
| symlink-outside-target | 100% [100%, 100%] (160/160) | n/a (0/0) | n/a (0/0) | n/a (0/0) |
| symlink-swap | 16.9% [11.9%, 21.9%] (27/160) | 0% [0%, 0%] (0/133) | n/a (0/0) | 0% [0%, 0%] (0/133) |
| parent-dir-rename | 8.8% [5.0%, 13.1%] (14/160) | 12.3% [7.9%, 17.0%] (18/146) | 0% [0%, 0%] (0/132) | 0% [0%, 0%] (0/146) |
| file-replacement | 6.9% [4.4%, 9.4%] (11/160) | 6.0% [2.0%, 10.5%] (9/149) | 0% [0%, 0%] (0/97) | 0% [0%, 0%] (0/148) |
| retry-after-failure | 29.5% [25.4%, 33.7%] (32/107) | 29.5% [25.5%, 33.7%] (32/107) | 0% [0%, 0%] (0/95) | 0% [0%, 0%] (0/75) |
| compat-fallback | 30.0% [26.9%, 33.1%] (48/160) | 33.8% [30.0%, 37.5%] (54/160) | 33.8% [30.0%, 37.5%] (54/160) | n/a (0/0) |
| primitive-unavailable | 33.8% [31.2%, 36.2%] (54/160) | n/a (0/0) | n/a (0/0) | n/a (0/0) |
| toctou-barrier | 18.8% [12.5%, 24.4%] (30/160) | 23.6% [16.5%, 30.7%] (31/130) | 0% [0%, 0%] (0/118) | 0% [0%, 0%] (0/130) |
| benign-control | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/160) | 0% [0%, 0%] (0/107) | 0% [0%, 0%] (0/160) |

The fraction in brackets is the pooled numerator and denominator over all seeds.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Completed containment-violation rate** = runs labelled containment_violation / runs labelled preserved or containment_violation (completed operations).
