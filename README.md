# PolicyContinuity

PolicyContinuity is dual-use tooling: it can be used as a practical benchmark
for evaluating containment implementations, and it is also the artifact for a
future research paper on whether a filesystem policy remains true across state
changes, retries, and fallback paths.

## Novel research question

**Can a filesystem containment policy remain continuous from check to use when
the filesystem changes between operations, and can an independent oracle
separate safe refusal from an unsafe completion without trusting the adapter
being evaluated?**

The project makes that question measurable with deterministic barriers,
independent object-level observation, and explicit refusal/availability
metrics. The current repository contains the research artifact and working
tooling; the paper is planned as a future publication.

A controlled, Linux-only benchmark for one question:

> When an application intends to constrain file operations to an approved
> directory root, which implementation strategies preserve that containment
> policy across filesystem state transitions, retries, and compatibility
> fallbacks, and when do they safely refuse rather than complete an unsafe
> operation?

## In 90 seconds

Programs that serve files "from this directory only" enforce that rule in
different ways: by checking the path as text, by resolving it and checking
the result, by walking it one directory handle at a time, or by asking the
kernel to enforce it. PolicyContinuity runs four small **benchmark
adapters**, one per strategy, through the same seeded scenarios, and asks
of every run: which object did the operation actually reach, and was that
object inside the approved root?

- **Synthetic fixtures only.** Every scenario is a throwaway directory tree
  built under `target/`. "Outside the root" always means a sentinel file
  the harness created in that same temporary workspace.
- **An independent oracle.** The label for a run comes from the harness's
  own observation of the returned file handle, checked against a model of
  the fixture. It never uses the adapter's opinion of what it did, and it
  shares no code with any adapter.
- **Deterministic schedules.** State changes are tied to barriers the
  harness counts (the adapter's k-th filesystem call). Nothing depends on
  timing.
- **A frozen protocol.** Hypothesis, metrics with denominators, a
  development/held-out seed split, and a held-out parameter region were
  committed before the held-out evaluation was run.

**Headline result, 40 held-out seeds, synthetic fixtures, one kernel.**
The kernel-enforced adapter (`strict-safe-open`) completed on an outside
sentinel in 0 of 3,260 completed operations; the text-checking adapter
(`lexical-prefix`) did so in 21.0% of its completed operations and the
resolve-then-open adapter (`canonicalize-then-open`) in 9.8%. Strictness
was not free: `strict-safe-open` refused 7.5% of requests that were safe
throughout, all of them in-root symlinks written as absolute paths or
paths that left the root and came back, and it declined every run in
which its primitive was made unavailable. An adapter that otherwise never
violated (`dirfd-relative`) did violate once it was permitted to fall back
to a legacy routine: 32.9% of its fallback-path runs. Full results,
including what these numbers do not show:
[INDEPENDENT_EVALUATION_AND_APPLICATION_RESULTS.md](INDEPENDENT_EVALUATION_AND_APPLICATION_RESULTS.md).

## Scope and dual-use safety

PolicyContinuity is **not** a vulnerability scanner, a tool for testing
other software, a claim that its adapters represent real applications, or a
demonstration against any real system.

The implementation is intended for authorized testing, reproducibility,
teaching, and research. It does not target external systems or create
real-world filesystem fixtures outside the workspace it owns.

- It runs only inside temporary directories it creates under a parent you
  name (`target/pc-tmp` by default), and removes them afterwards.
- A static check proves, from each scenario's data alone, that nothing it
  names can lie outside its workspace. It runs before anything is created.
- It needs no privileges, no network, and no container. It does not read
  `/proc`, `/sys`, `/etc`, or the system temporary directory.
- Every number describes four reference adapters, twelve authored scenario
  families, and one kernel. How far that is from real software is the
  subject of [docs/VALIDITY_THREATS.md](docs/VALIDITY_THREATS.md) and
  [docs/WHY_THIS_MAY_BE_WRONG.md](docs/WHY_THIS_MAY_BE_WRONG.md).

## Quick start

Requirements: Linux (kernel 5.6 or later for `openat2`), Rust stable with
`rustfmt` and `clippy` (developed with rustc 1.99.0), `make`, `sha256sum`.

```sh
cd policycontinuity
cargo fetch --locked        # the only step that uses the network
make check                  # format check, lint, build, unit + integration tests
```

Everything after `cargo fetch` runs offline; the Makefile sets
`CARGO_NET_OFFLINE=true`.

## One experiment command

```sh
make final
```

This runs the frozen held-out evaluation (40 seeds, 5,760 scenarios, four
adapters, 23,040 runs) and writes every table and figure to
`results/final/`. It took about 50 seconds on the development machine,
nearly all of it filesystem calls. To check that your machine reproduces
the committed results byte for byte:

```sh
make verify
```

### Expected outputs

```text
results/final/
  manifest.json                           protocol, seeds, adapters, file list
  environment.json                        kernel release, openat2 support (not checksummed)
  SHA256SUMS                              checksums of everything except environment.json
  heldout/hypothesis.json                 the pre-declared rule, its inputs, and its outcome
  heldout/summary.csv                     every metric: mean, SD, min, max, 95% CI, pooled counts
  heldout/per_seed.csv                    every metric per seed, with numerator and denominator
  heldout/outcome_counts.csv              seven outcomes per seed x adapter x family x region x condition
  heldout/by_parameter.csv                outcomes by depth, timing, link form, path taken
  heldout/paired_differences.csv          the per-seed group differences behind the verdict
  heldout/runs.csv                        one line per run: the operation-result record and its label
  tables/table_primary_metrics.md
  tables/table1_preservation_by_family.md
  tables/table2_violation_by_family.md
  tables/table3_availability_vs_refusal.md
  tables/table4_false_refusals.md
  tables/table5_fallback_drift.md
  tables/table6_outcome_matrix.md         all seven outcomes, by adapter and family
  tables/table7_per_seed_and_parameter.md
  tables/table8_when_strict_refusal_helps.md
  tables/table_worst_case.md
  tables/table_policy_comparison.md       lexical policy against object policy
  figures/fig1_preservation_by_family.svg
  figures/fig2_violation_by_family.svg
  figures/fig3_availability_vs_refusal.svg
  figures/fig4_false_refusal_safe.svg
  figures/fig5_fallback_drift.svg
```

`make final` prints the primary table and the hypothesis outcome:

```text
primary metric (mean [95% CI] over seeds)               lexical-prefix    canonicalize-then-open            dirfd-relative          strict-safe-open
Containment-preservation rate                     80.4% [79.9%, 80.9%]      92.9% [92.5%, 93.3%]      97.3% [97.1%, 97.4%]         100% [100%, 100%]
Completed containment-violation rate              21.0% [20.5%, 21.6%]        9.8% [9.2%, 10.4%]         5.6% [5.3%, 6.0%]               0% [0%, 0%]
Safe-refusal rate under unsafe conditions         17.0% [16.2%, 17.8%]      67.8% [66.6%, 68.9%]      70.0% [68.8%, 71.3%]      96.6% [95.9%, 97.3%]
False-refusal rate under safe conditions                   0% [0%, 0%]               0% [0%, 0%]      29.1% [28.2%, 29.9%]         7.5% [7.2%, 7.7%]
Availability under safe conditions                97.8% [97.4%, 98.2%]      89.9% [89.5%, 90.3%]      60.0% [59.1%, 61.0%]      75.1% [74.6%, 75.7%]
Fallback policy-drift rate                                         n/a      28.7% [27.1%, 30.6%]      32.9% [30.8%, 35.2%]                       n/a

primary hypothesis: supported
```

Numerators and denominators for every row are in
[docs/METRICS.md](docs/METRICS.md) and at the foot of each table file. The
intervals describe variation in the seeded mix of scenarios, not
measurement noise: a single run is deterministic.

## Other commands

```sh
make dev                    # the 20 development seeds; no verdict is computed
make quick                  # one development seed, one line per run
cargo run --release -- list adapters
cargo run --release -- list families
cargo run --release -- list metrics
cargo run --release -- list scenarios --seed 3
cargo run --release -- protocol                 # the frozen constants and predictions
cargo run --release -- probe                    # kernel release; is openat2 usable here?
cargo run --release -- run --seed 2 --family toctou-barrier
cargo run --release -- inspect --seed 1 --scenario s1-symlink-outside-target-00 --adapter lexical-prefix
```

`inspect` prints a scenario's fixture steps, its schedule, the full
operation-result record, and the oracle's label. Held-out seeds (1001 to
1040) are accepted only by `final`.

## Where to read next

| Question | Document |
|---|---|
| What exactly is being tested, and for whom? | [docs/RESEARCH_QUESTION.md](docs/RESEARCH_QUESTION.md) |
| What may a fixture do, and what is out of scope? | [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) |
| How do the fixtures, barriers, oracle, and families work? | [docs/METHODOLOGY.md](docs/METHODOLOGY.md) |
| What counts as preserved, a violation, a safe or false refusal? | [docs/METRICS.md](docs/METRICS.md) |
| What do the four adapters stand for? | [docs/BASELINES.md](docs/BASELINES.md) |
| What was frozen before the final run? | [docs/FROZEN_EVALUATION_PROTOCOL.md](docs/FROZEN_EVALUATION_PROTOCOL.md) |
| How does this differ from the first version? | [docs/PRELIMINARY_VS_FINAL_EVALUATION.md](docs/PRELIMINARY_VS_FINAL_EVALUATION.md) |
| What limits the conclusions? | [docs/VALIDITY_THREATS.md](docs/VALIDITY_THREATS.md) |
| How might the interpretation be wrong? | [docs/WHY_THIS_MAY_BE_WRONG.md](docs/WHY_THIS_MAY_BE_WRONG.md) |
| How do I rerun and verify it? | [docs/REPRODUCIBILITY.md](docs/REPRODUCIBILITY.md) |
| Results and decision memo | [INDEPENDENT_EVALUATION_AND_APPLICATION_RESULTS.md](INDEPENDENT_EVALUATION_AND_APPLICATION_RESULTS.md) |
| Technical report draft | [paper/technical-report.md](paper/technical-report.md) |

## Layout

```text
src/fixture.rs          declared object graph, applied to a model and to disk together
src/scenario.rs         the 12 seeded families; static confinement validation
src/env.rs              what an adapter may call: five primitives, each a barrier
src/adapters/           the four benchmark adapters and their shared contract
src/runner.rs           trusted harness: runs one adapter, observes, builds the record
src/record.rs           the operation-result record
src/oracle.rs           pure labelling function; own path-resolution model
src/metrics.rs          metric definitions, each with numerator and denominator
src/stats.rs            per-seed summaries and bootstrap intervals
src/protocol.rs         frozen seeds, ranges, decision rule, predictions
src/experiment.rs       development and held-out runs, tables
src/figures.rs          SVG figures
src/sys.rs              open, openat, openat2, fstat: all the unsafe code
tests/                  oracle independence, model soundness, benchmark properties, CLI
docs/                   protocol, methodology, metrics, threats
paper/                  technical report draft and abstracts
results/preliminary/    the first version's outputs, unmodified
results/dev/            development-seed tables
results/final/          held-out results, tables, figures
```

## Status

- 122 tests (91 unit, 31 integration) pass locally.
- A CI workflow is included at the repository root
  (`.github/workflows/policycontinuity.yml`). It has not been run on
  GitHub.
- No license has been chosen yet: see [LICENSE-TODO.md](LICENSE-TODO.md).
