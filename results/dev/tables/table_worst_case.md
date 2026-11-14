# Worst-case policy outcome by adapter and scenario family

| Scenario family | lexical-prefix | canonicalize-then-open | dirfd-relative | strict-safe-open |
| --- | --- | --- | --- | --- |
| nested-in-root | preserved | preserved | preserved | preserved |
| dotdot-normalization | safe_refusal | safe_refusal | false_refusal | false_refusal |
| symlink-in-root-target | preserved | preserved | false_refusal | false_refusal |
| symlink-outside-target | containment_violation | safe_refusal | safe_refusal | safe_refusal |
| symlink-swap | containment_violation | safe_refusal | false_refusal | safe_refusal |
| parent-dir-rename | containment_violation | containment_violation | safe_refusal | safe_refusal |
| file-replacement | containment_violation | containment_violation | false_refusal | false_refusal |
| retry-after-failure | containment_violation | containment_violation | ordinary_failure | ordinary_failure |
| compat-fallback | containment_violation | containment_violation | containment_violation | unsupported_safe |
| primitive-unavailable | containment_violation | unsupported_safe | unsupported_safe | unsupported_safe |
| toctou-barrier | containment_violation | containment_violation | safe_refusal | safe_refusal |
| benign-control | preserved | preserved | false_refusal | preserved |

The most severe outcome observed in any run of that cell. Severity, worst first: containment_violation, evaluator_inconclusive, false_refusal, unsupported_safe, ordinary_failure, safe_refusal, preserved.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
