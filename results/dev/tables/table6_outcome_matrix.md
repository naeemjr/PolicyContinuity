# Table 6. Outcome counts by adapter and scenario family

| Adapter | Scenario family | Runs | preserved | containment_violation | safe_refusal | false_refusal | ordinary_failure | unsupported_safe | evaluator_inconclusive |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | nested-in-root | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | dotdot-normalization | 160 | 79 | 0 | 81 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-in-root-target | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-outside-target | 160 | 0 | 160 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-swap | 160 | 133 | 27 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | parent-dir-rename | 160 | 146 | 14 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | file-replacement | 160 | 149 | 11 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | retry-after-failure | 160 | 75 | 32 | 0 | 0 | 53 | 0 | 0 |
| lexical-prefix | compat-fallback | 160 | 112 | 48 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | primitive-unavailable | 160 | 106 | 54 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | toctou-barrier | 160 | 130 | 30 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | benign-control | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| **lexical-prefix** | **all** | 1920 | 1410 | 376 | 81 | 0 | 53 | 0 | 0 |
| canonicalize-then-open | nested-in-root | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | dotdot-normalization | 160 | 79 | 0 | 81 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-in-root-target | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-outside-target | 160 | 0 | 0 | 160 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-swap | 160 | 133 | 0 | 27 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | parent-dir-rename | 160 | 128 | 18 | 14 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | file-replacement | 160 | 140 | 9 | 11 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | retry-after-failure | 160 | 75 | 32 | 0 | 0 | 53 | 0 | 0 |
| canonicalize-then-open | compat-fallback | 160 | 106 | 54 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | primitive-unavailable | 160 | 0 | 0 | 0 | 0 | 0 | 160 | 0 |
| canonicalize-then-open | toctou-barrier | 160 | 99 | 31 | 30 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | benign-control | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| **canonicalize-then-open** | **all** | 1920 | 1240 | 144 | 323 | 0 | 53 | 160 | 0 |
| dirfd-relative | nested-in-root | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| dirfd-relative | dotdot-normalization | 160 | 53 | 0 | 81 | 26 | 0 | 0 | 0 |
| dirfd-relative | symlink-in-root-target | 160 | 0 | 0 | 0 | 160 | 0 | 0 | 0 |
| dirfd-relative | symlink-outside-target | 160 | 0 | 0 | 160 | 0 | 0 | 0 | 0 |
| dirfd-relative | symlink-swap | 160 | 0 | 0 | 66 | 94 | 0 | 0 | 0 |
| dirfd-relative | parent-dir-rename | 160 | 132 | 0 | 28 | 0 | 0 | 0 | 0 |
| dirfd-relative | file-replacement | 160 | 97 | 0 | 36 | 27 | 0 | 0 | 0 |
| dirfd-relative | retry-after-failure | 160 | 95 | 0 | 12 | 0 | 53 | 0 | 0 |
| dirfd-relative | compat-fallback | 160 | 106 | 54 | 0 | 0 | 0 | 0 | 0 |
| dirfd-relative | primitive-unavailable | 160 | 0 | 0 | 0 | 0 | 0 | 160 | 0 |
| dirfd-relative | toctou-barrier | 160 | 118 | 0 | 42 | 0 | 0 | 0 | 0 |
| dirfd-relative | benign-control | 160 | 107 | 0 | 0 | 53 | 0 | 0 | 0 |
| **dirfd-relative** | **all** | 1920 | 868 | 54 | 425 | 360 | 53 | 160 | 0 |
| strict-safe-open | nested-in-root | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| strict-safe-open | dotdot-normalization | 160 | 53 | 0 | 81 | 26 | 0 | 0 | 0 |
| strict-safe-open | symlink-in-root-target | 160 | 80 | 0 | 0 | 80 | 0 | 0 | 0 |
| strict-safe-open | symlink-outside-target | 160 | 0 | 0 | 160 | 0 | 0 | 0 | 0 |
| strict-safe-open | symlink-swap | 160 | 133 | 0 | 27 | 0 | 0 | 0 | 0 |
| strict-safe-open | parent-dir-rename | 160 | 146 | 0 | 14 | 0 | 0 | 0 | 0 |
| strict-safe-open | file-replacement | 160 | 148 | 0 | 11 | 1 | 0 | 0 | 0 |
| strict-safe-open | retry-after-failure | 160 | 75 | 0 | 32 | 0 | 53 | 0 | 0 |
| strict-safe-open | compat-fallback | 160 | 0 | 0 | 0 | 0 | 0 | 160 | 0 |
| strict-safe-open | primitive-unavailable | 160 | 0 | 0 | 0 | 0 | 0 | 160 | 0 |
| strict-safe-open | toctou-barrier | 160 | 130 | 0 | 30 | 0 | 0 | 0 | 0 |
| strict-safe-open | benign-control | 160 | 160 | 0 | 0 | 0 | 0 | 0 | 0 |
| **strict-safe-open** | **all** | 1920 | 1085 | 0 | 355 | 107 | 53 | 320 | 0 |

Pooled run counts. Every run has exactly one of the seven outcomes, so each row sums to its run count.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
