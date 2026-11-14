# Table 4. False refusals under safe conditions

| Family / variant | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root / plain | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) | 0/160 (0%) |
| dotdot-normalization / inner-dotdot | 0/26 (0%) | 0/26 (0%) | 0/26 (0%) | 0/26 (0%) |
| dotdot-normalization / dot-segments | 0/27 (0%) | 0/27 (0%) | 0/27 (0%) | 0/27 (0%) |
| dotdot-normalization / reenter-root | 0/26 (0%) | 0/26 (0%) | 26/26 (100%) | 26/26 (100%) |
| symlink-in-root-target / relative-link | 0/80 (0%) | 0/80 (0%) | 80/80 (100%) | 0/80 (0%) |
| symlink-in-root-target / absolute-link | 0/80 (0%) | 0/80 (0%) | 80/80 (100%) | 80/80 (100%) |
| symlink-swap / retarget-to-outside | 0/133 (0%) | 0/100 (0%) | 94/94 (100%) | 0/133 (0%) |
| parent-dir-rename / fresh-dir | 0/80 (0%) | 0/80 (0%) | 0/80 (0%) | 0/80 (0%) |
| parent-dir-rename / outside-link | 0/66 (0%) | 0/48 (0%) | 0/28 (0%) | 0/66 (0%) |
| file-replacement / fresh-file | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) |
| file-replacement / in-root-link | 0/53 (0%) | 0/53 (0%) | 27/53 (50.9%) | 1/53 (1.9%) |
| file-replacement / outside-link | 0/43 (0%) | 0/34 (0%) | 0/18 (0%) | 0/43 (0%) |
| retry-after-failure / plain | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) |
| retry-after-failure / fresh-after-failure | 0/54 (0%) | 0/54 (0%) | 0/54 (0%) | 0/54 (0%) |
| compat-fallback / plain | 0/40 (0%) | 0/40 (0%) | 0/40 (0%) | 0/40 (0%) |
| compat-fallback / in-root-link | 0/40 (0%) | 0/40 (0%) | 0/40 (0%) | 0/40 (0%) |
| compat-fallback / swap-outside | 0/32 (0%) | 0/26 (0%) | 0/26 (0%) | 0/32 (0%) |
| primitive-unavailable / plain | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) |
| primitive-unavailable / in-root-link | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) |
| toctou-barrier / swap-and-restore | 0/130 (0%) | 0/99 (0%) | 0/58 (0%) | 0/130 (0%) |
| benign-control / unrelated-change | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) | 0/53 (0%) |
| benign-control / in-root-retarget | 0/53 (0%) | 0/53 (0%) | 53/53 (100%) | 0/53 (0%) |
| benign-control / transient-within-budget | 0/54 (0%) | 0/54 (0%) | 0/54 (0%) | 0/54 (0%) |
| **all safe-condition runs** | 0% [0%, 0%] | 0% [0%, 0%] | 28.5% [27.4%, 29.5%] | 7.4% [7.1%, 7.8%] |

Each cell is pooled over seeds: runs labelled false_refusal / runs of that family and variant under a safe condition. Variants with no safe-condition run for any adapter are omitted. The last row is the per-seed mean with its interval.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.

Denominators:

- **False-refusal rate under safe conditions** = runs labelled false_refusal / runs under a safe condition.
