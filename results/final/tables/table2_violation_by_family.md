# Table 2. Completed containment-violation rate by adapter and scenario family

| Scenario family | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/480) |
| dotdot-normalization | 0% [0%, 0%] (0/239) | 0% [0%, 0%] (0/239) | 0% [0%, 0%] (0/161) | 0% [0%, 0%] (0/161) |
| symlink-in-root-target | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/480) | n/a (0/0) | 0% [0%, 0%] (0/240) |
| symlink-outside-target | 100% [100%, 100%] (480/480) | n/a (0/0) | n/a (0/0) | n/a (0/0) |
| symlink-swap | 16.9% [14.0%, 19.8%] (81/480) | 0% [0%, 0%] (0/399) | n/a (0/0) | 0% [0%, 0%] (0/399) |
| parent-dir-rename | 10.4% [8.1%, 12.7%] (50/480) | 9.7% [7.7%, 11.9%] (42/430) | 0% [0%, 0%] (0/389) | 0% [0%, 0%] (0/430) |
| file-replacement | 5.8% [4.2%, 7.5%] (28/480) | 5.5% [3.4%, 7.9%] (25/452) | 0% [0%, 0%] (0/277) | 0% [0%, 0%] (0/446) |
| retry-after-failure | 33.4% [29.3%, 37.4%] (111/337) | 33.4% [29.4%, 37.5%] (111/337) | 0% [0%, 0%] (0/306) | 0% [0%, 0%] (0/226) |
| compat-fallback | 28.7% [27.1%, 30.6%] (138/480) | 32.9% [30.6%, 35.2%] (158/480) | 32.9% [30.8%, 35.2%] (158/480) | n/a (0/0) |
| primitive-unavailable | 33.5% [31.5%, 35.6%] (161/480) | n/a (0/0) | n/a (0/0) | n/a (0/0) |
| toctou-barrier | 17.1% [14.4%, 19.8%] (82/480) | 18.3% [14.8%, 22.1%] (73/398) | 0% [0%, 0%] (0/400) | 0% [0%, 0%] (0/398) |
| benign-control | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/480) | 0% [0%, 0%] (0/321) | 0% [0%, 0%] (0/480) |

The fraction in brackets is the pooled numerator and denominator over all seeds.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **Completed containment-violation rate** = runs labelled containment_violation / runs labelled preserved or containment_violation (completed operations).
