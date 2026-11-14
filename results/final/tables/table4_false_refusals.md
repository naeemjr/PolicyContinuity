# Table 4. False refusals under safe conditions

| Family / variant | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root / plain | 0/480 (0%) | 0/480 (0%) | 0/480 (0%) | 0/480 (0%) |
| dotdot-normalization / inner-dotdot | 0/80 (0%) | 0/80 (0%) | 0/80 (0%) | 0/80 (0%) |
| dotdot-normalization / dot-segments | 0/81 (0%) | 0/81 (0%) | 0/81 (0%) | 0/81 (0%) |
| dotdot-normalization / reenter-root | 0/78 (0%) | 0/78 (0%) | 78/78 (100%) | 78/78 (100%) |
| symlink-in-root-target / relative-link | 0/240 (0%) | 0/240 (0%) | 240/240 (100%) | 0/240 (0%) |
| symlink-in-root-target / absolute-link | 0/240 (0%) | 0/240 (0%) | 240/240 (100%) | 240/240 (100%) |
| symlink-swap / retarget-to-outside | 0/399 (0%) | 0/320 (0%) | 282/282 (100%) | 0/399 (0%) |
| parent-dir-rename / fresh-dir | 0/240 (0%) | 0/240 (0%) | 0/240 (0%) | 0/240 (0%) |
| parent-dir-rename / outside-link | 0/190 (0%) | 0/148 (0%) | 0/78 (0%) | 0/190 (0%) |
| file-replacement / fresh-file | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) |
| file-replacement / in-root-link | 0/159 (0%) | 0/159 (0%) | 101/159 (63.5%) | 6/159 (3.8%) |
| file-replacement / outside-link | 0/133 (0%) | 0/108 (0%) | 0/59 (0%) | 0/133 (0%) |
| retry-after-failure / plain | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) |
| retry-after-failure / fresh-after-failure | 0/161 (0%) | 0/161 (0%) | 0/161 (0%) | 0/161 (0%) |
| compat-fallback / plain | 0/120 (0%) | 0/120 (0%) | 0/120 (0%) | 0/120 (0%) |
| compat-fallback / in-root-link | 0/120 (0%) | 0/120 (0%) | 0/120 (0%) | 0/120 (0%) |
| compat-fallback / swap-outside | 0/102 (0%) | 0/82 (0%) | 0/82 (0%) | 0/102 (0%) |
| primitive-unavailable / plain | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) |
| primitive-unavailable / in-root-link | 0/159 (0%) | 0/159 (0%) | 0/159 (0%) | 0/159 (0%) |
| toctou-barrier / swap-and-restore | 0/398 (0%) | 0/325 (0%) | 0/166 (0%) | 0/398 (0%) |
| benign-control / unrelated-change | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) |
| benign-control / in-root-retarget | 0/159 (0%) | 0/159 (0%) | 159/159 (100%) | 0/159 (0%) |
| benign-control / transient-within-budget | 0/161 (0%) | 0/161 (0%) | 0/161 (0%) | 0/161 (0%) |
| **all safe-condition runs** | 0% [0%, 0%] | 0% [0%, 0%] | 29.1% [28.2%, 29.9%] | 7.5% [7.2%, 7.7%] |

Each cell is pooled over seeds: runs labelled false_refusal / runs of that family and variant under a safe condition. Variants with no safe-condition run for any adapter are omitted. The last row is the per-seed mean with its interval.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
