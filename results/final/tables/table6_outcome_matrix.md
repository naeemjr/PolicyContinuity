# Table 6. Outcome counts by adapter and scenario family

| Adapter | Scenario family | Runs | preserved | containment_violation | safe_refusal | false_refusal | ordinary_failure | unsupported_safe | evaluator_inconclusive |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | nested-in-root | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | dotdot-normalization | 480 | 239 | 0 | 241 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-in-root-target | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-outside-target | 480 | 0 | 480 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | symlink-swap | 480 | 399 | 81 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | parent-dir-rename | 480 | 430 | 50 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | file-replacement | 480 | 452 | 28 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | retry-after-failure | 480 | 226 | 111 | 0 | 0 | 143 | 0 | 0 |
| lexical-prefix | compat-fallback | 480 | 342 | 138 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | primitive-unavailable | 480 | 319 | 161 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | toctou-barrier | 480 | 398 | 82 | 0 | 0 | 0 | 0 | 0 |
| lexical-prefix | benign-control | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| **lexical-prefix** | **all** | 5760 | 4245 | 1131 | 241 | 0 | 143 | 0 | 0 |
| canonicalize-then-open | nested-in-root | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | dotdot-normalization | 480 | 239 | 0 | 241 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-in-root-target | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-outside-target | 480 | 0 | 0 | 480 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | symlink-swap | 480 | 399 | 0 | 81 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | parent-dir-rename | 480 | 388 | 42 | 50 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | file-replacement | 480 | 427 | 25 | 28 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | retry-after-failure | 480 | 226 | 111 | 0 | 0 | 143 | 0 | 0 |
| canonicalize-then-open | compat-fallback | 480 | 322 | 158 | 0 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | primitive-unavailable | 480 | 0 | 0 | 0 | 0 | 0 | 480 | 0 |
| canonicalize-then-open | toctou-barrier | 480 | 325 | 73 | 82 | 0 | 0 | 0 | 0 |
| canonicalize-then-open | benign-control | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| **canonicalize-then-open** | **all** | 5760 | 3766 | 409 | 962 | 0 | 143 | 480 | 0 |
| dirfd-relative | nested-in-root | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| dirfd-relative | dotdot-normalization | 480 | 161 | 0 | 241 | 78 | 0 | 0 | 0 |
| dirfd-relative | symlink-in-root-target | 480 | 0 | 0 | 0 | 480 | 0 | 0 | 0 |
| dirfd-relative | symlink-outside-target | 480 | 0 | 0 | 480 | 0 | 0 | 0 | 0 |
| dirfd-relative | symlink-swap | 480 | 0 | 0 | 198 | 282 | 0 | 0 | 0 |
| dirfd-relative | parent-dir-rename | 480 | 389 | 0 | 91 | 0 | 0 | 0 | 0 |
| dirfd-relative | file-replacement | 480 | 277 | 0 | 102 | 101 | 0 | 0 | 0 |
| dirfd-relative | retry-after-failure | 480 | 306 | 0 | 31 | 0 | 143 | 0 | 0 |
| dirfd-relative | compat-fallback | 480 | 322 | 158 | 0 | 0 | 0 | 0 | 0 |
| dirfd-relative | primitive-unavailable | 480 | 0 | 0 | 0 | 0 | 0 | 480 | 0 |
| dirfd-relative | toctou-barrier | 480 | 400 | 0 | 80 | 0 | 0 | 0 | 0 |
| dirfd-relative | benign-control | 480 | 321 | 0 | 0 | 159 | 0 | 0 | 0 |
| **dirfd-relative** | **all** | 5760 | 2656 | 158 | 1223 | 1100 | 143 | 480 | 0 |
| strict-safe-open | nested-in-root | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| strict-safe-open | dotdot-normalization | 480 | 161 | 0 | 241 | 78 | 0 | 0 | 0 |
| strict-safe-open | symlink-in-root-target | 480 | 240 | 0 | 0 | 240 | 0 | 0 | 0 |
| strict-safe-open | symlink-outside-target | 480 | 0 | 0 | 480 | 0 | 0 | 0 | 0 |
| strict-safe-open | symlink-swap | 480 | 399 | 0 | 81 | 0 | 0 | 0 | 0 |
| strict-safe-open | parent-dir-rename | 480 | 430 | 0 | 50 | 0 | 0 | 0 | 0 |
| strict-safe-open | file-replacement | 480 | 446 | 0 | 28 | 6 | 0 | 0 | 0 |
| strict-safe-open | retry-after-failure | 480 | 226 | 0 | 111 | 0 | 143 | 0 | 0 |
| strict-safe-open | compat-fallback | 480 | 0 | 0 | 0 | 0 | 0 | 480 | 0 |
| strict-safe-open | primitive-unavailable | 480 | 0 | 0 | 0 | 0 | 0 | 480 | 0 |
| strict-safe-open | toctou-barrier | 480 | 398 | 0 | 82 | 0 | 0 | 0 | 0 |
| strict-safe-open | benign-control | 480 | 480 | 0 | 0 | 0 | 0 | 0 | 0 |
| **strict-safe-open** | **all** | 5760 | 3260 | 0 | 1073 | 324 | 143 | 960 | 0 |

Pooled run counts. Every run has exactly one of the seven outcomes, so each row sums to its run count.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
