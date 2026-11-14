# The two containment policies compared

| Adapter | Policy | preserved | containment_violation | safe_refusal | false_refusal | ordinary_failure | unsupported_safe | evaluator_inconclusive |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | authorized-root object (primary) | 4245 | 1131 | 241 | 0 | 143 | 0 | 0 |
| lexical-prefix | lexical | 5376 | 0 | 241 | 0 | 143 | 0 | 0 |
| lexical-prefix | runs labelled differently | 1131/5760 (19.6%) |
| canonicalize-then-open | authorized-root object (primary) | 3766 | 409 | 962 | 0 | 143 | 480 | 0 |
| canonicalize-then-open | lexical | 4175 | 0 | 241 | 721 | 143 | 480 | 0 |
| canonicalize-then-open | runs labelled differently | 1130/5760 (19.6%) |
| dirfd-relative | authorized-root object (primary) | 2656 | 158 | 1223 | 1100 | 143 | 480 | 0 |
| dirfd-relative | lexical | 2814 | 0 | 241 | 2082 | 143 | 480 | 0 |
| dirfd-relative | runs labelled differently | 1140/5760 (19.8%) |
| strict-safe-open | authorized-root object (primary) | 3260 | 0 | 1073 | 324 | 143 | 960 | 0 |
| strict-safe-open | lexical | 3260 | 0 | 241 | 1156 | 143 | 960 | 0 |
| strict-safe-open | runs labelled differently | 832/5760 (14.4%) |

Pooled run counts. The lexical policy judges the request text alone: it never yields evaluator_inconclusive, and it calls a completed operation preserved whenever the normalised text stays beneath the root, whatever object was actually reached.

40 seeds (held-out split), 5760 scenarios, 23040 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
