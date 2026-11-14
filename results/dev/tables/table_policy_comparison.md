# The two containment policies compared

| Adapter | Policy | preserved | containment_violation | safe_refusal | false_refusal | ordinary_failure | unsupported_safe | evaluator_inconclusive |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lexical-prefix | authorized-root object (primary) | 1410 | 376 | 81 | 0 | 53 | 0 | 0 |
| lexical-prefix | lexical | 1786 | 0 | 81 | 0 | 53 | 0 | 0 |
| lexical-prefix | runs labelled differently | 376/1920 (19.6%) |
| canonicalize-then-open | authorized-root object (primary) | 1240 | 144 | 323 | 0 | 53 | 160 | 0 |
| canonicalize-then-open | lexical | 1384 | 0 | 81 | 242 | 53 | 160 | 0 |
| canonicalize-then-open | runs labelled differently | 386/1920 (20.1%) |
| dirfd-relative | authorized-root object (primary) | 868 | 54 | 425 | 360 | 53 | 160 | 0 |
| dirfd-relative | lexical | 922 | 0 | 81 | 704 | 53 | 160 | 0 |
| dirfd-relative | runs labelled differently | 398/1920 (20.7%) |
| strict-safe-open | authorized-root object (primary) | 1085 | 0 | 355 | 107 | 53 | 320 | 0 |
| strict-safe-open | lexical | 1085 | 0 | 81 | 381 | 53 | 320 | 0 |
| strict-safe-open | runs labelled differently | 274/1920 (14.3%) |

Pooled run counts. The lexical policy judges the request text alone: it never yields evaluator_inconclusive, and it calls a completed operation preserved whenever the normalised text stays beneath the root, whatever object was actually reached.

20 seeds (development split), 1920 scenarios, 7680 evaluated runs. Each cell is the mean of the per-seed rate with a 95% percentile-bootstrap interval over seeds (10,000 resamples); seeds where a denominator is zero are left out of that cell. Synthetic fixtures in a temporary workspace only.
