# Table 8. When strict refusal helps, and when it costs availability

| Scenario family | Scenarios | helps vs lexical-prefix | costs vs lexical-prefix | helps vs canonicalize-then-open | costs vs canonicalize-then-open | of the costs: false_refusal / unsupported_safe |
| --- | --- | --- | --- | --- | --- | --- |
| nested-in-root | 480 | 0 | 0 | 0 | 0 | 0 / 0 |
| dotdot-normalization | 480 | 0 | 78 | 0 | 78 | 78 / 0 |
| symlink-in-root-target | 480 | 0 | 240 | 0 | 240 | 240 / 0 |
| symlink-outside-target | 480 | 480 | 0 | 0 | 0 | 0 / 0 |
| symlink-swap | 480 | 81 | 0 | 0 | 0 | 0 / 0 |
| parent-dir-rename | 480 | 50 | 0 | 42 | 0 | 0 / 0 |
| file-replacement | 480 | 28 | 6 | 25 | 6 | 6 / 0 |
| retry-after-failure | 480 | 111 | 0 | 111 | 0 | 0 / 0 |
| compat-fallback | 480 | 138 | 342 | 158 | 322 | 0 / 342 |
| primitive-unavailable | 480 | 161 | 319 | 0 | 0 | 0 / 319 |
| toctou-barrier | 480 | 82 | 0 | 73 | 0 | 0 / 0 |
| benign-control | 480 | 0 | 0 | 0 | 0 | 0 / 0 |
| **all families** | 5760 | 1131 | 985 | 409 | 646 | 324 / 661 |

Counts of scenarios, comparing `strict-safe-open` with one path-string adapter on the identical scenario. **Helps**: the other adapter's run was a containment_violation and strict-safe-open's was not. **Costs**: the other adapter's run was preserved, and strict-safe-open, under a safe condition, did not complete (false_refusal, unsupported_safe, or ordinary_failure). The last column splits the scenarios with a cost against either adapter by strict-safe-open's own outcome.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
