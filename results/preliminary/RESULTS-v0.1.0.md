# Results

Status of the prototype as built and verified on the development machine on
2026-10-04. Everything in "Commands run" was executed; the outputs quoted are
the real outputs.

These results describe **benchmark fixtures** on **one kernel**. They are
not measurements of any real application. See
[docs/LIMITATIONS.md](docs/LIMITATIONS.md).

## Environment

| Item      | Value                                               |
|-----------|-----------------------------------------------------|
| Kernel    | Linux 6.18.33.1-microsoft-standard-WSL2, x86_64     |
| rustc     | 1.99.0 (b940084d7 2026-09-28)                       |
| cargo     | 1.99.0 (5f94df478 2026-08-27)                       |
| clippy    | 0.1.99                                              |
| rustfmt   | 1.10.0-stable                                       |
| `openat2(RESOLVE_BENEATH)` | available                          |
| Temp dir  | `/tmp` (tmpfs); repeated on an ext-family filesystem |

## Commands run

| Command          | What it runs                                              | Result |
|------------------|-----------------------------------------------------------|--------|
| `make fmt-check` | `cargo fmt --check`                                       | pass (exit 0) |
| `make lint`      | `cargo clippy --locked --all-targets -- -D warnings`      | pass, no warnings |
| `make build`     | `cargo build --locked`                                    | pass |
| `make test`      | `cargo test --locked`                                     | pass, 146 tests |
| `make results`   | default run, seed 1, writes `results/seed-1/`             | pass, 712/712 cases executed |
| `cargo run --release -- run --seed 1 --format json` | release build   | output byte-identical to the debug build |
| `cargo run -- run --seed 1 --format json --temp-parent <ext dir>` | other filesystem | output byte-identical to the tmpfs run |
| `cargo run -- run --adapter beneath-strict --fail-on-containment-failure` | gate | exit 0 |
| `cargo run -- run --adapter naive-join --fail-on-containment-failure`     | gate | exit 2 |
| `cargo run -- probe`, `list-adapters`, `list-scenarios`                   |      | pass |

`make check` runs the first four in sequence and passed.

### Test results

```text
unit tests (src/lib.rs)      77 passed; 0 failed
tests/cli.rs                 12 passed; 0 failed
tests/scenarios.rs           57 passed; 0 failed
total                       146 passed; 0 failed; 0 ignored
```

- `tests/scenarios.rs` has one test per fixed scenario (41), each asserting
  the classification of all eight adapters, and a test that fails if a
  scenario is added without one.
- In its final form the full suite was run 20 consecutive times with no
  failures (and 25 times before the last change to the race schedule).
- The expected classifications in `tests/scenarios.rs` were worked out by
  hand before the first run, and the first run matched all 41 rows. They are
  a regression check written by the same author as the adapters, not
  independent ground truth.

## Sample run (seed 1, default settings)

`results/seed-1/summary.txt`:

```text
policycontinuity 0.1.0  seed=1 race-trials=48  kernel=6.18.33.1-microsoft-standard-WSL2 openat2=yes
coverage: 8/8 categories, 89 scenarios, 712/712 cases executed, mutation reached in 353/480 interleaved cases (73.5%)
adapters are benchmark fixtures, not production implementations

adapter                    cases allowed safe-ref false-ref  escapes preserved false-ref%  correct
naive-join                    89      46       13         1       29     67.4%       4.3%    66.3%
string-prefix                 89      49        6         0       34     61.8%       0.0%    61.8%
lexical-prefix                89      49        7         0       33     62.9%       0.0%    62.9%
canonicalize-then-open        89      49       21         0       19     78.7%       0.0%    78.7%
open-then-verify              89      49       24         0       16     82.0%       0.0%    82.0%
beneath-strict                89      26       59         4        0    100.0%      17.4%    95.5%
beneath-fallback-lexical      89      40       30         1       18     79.8%       4.3%    78.7%
beneath-fallback-walk         89      34       53         2        0    100.0%       8.7%    97.8%

policy preservation by code path taken (cases / containment failures / preserved)
adapter                                  direct          after retry       after fallback
naive-join                      89 / 29 / 67.4%          0 / 0 / n/a          0 / 0 / n/a
string-prefix                   76 / 25 / 67.1%       13 / 9 / 30.8%          0 / 0 / n/a
lexical-prefix                  76 / 24 / 68.4%       13 / 9 / 30.8%          0 / 0 / n/a
canonicalize-then-open          77 / 11 / 85.7%       12 / 8 / 33.3%          0 / 0 / n/a
open-then-verify                76 / 12 / 84.2%       13 / 4 / 69.2%          0 / 0 / n/a
beneath-strict                  76 / 0 / 100.0%      13 / 0 / 100.0%          0 / 0 / n/a
beneath-fallback-lexical        52 / 0 / 100.0%       13 / 9 / 30.8%      37 / 18 / 51.4%
beneath-fallback-walk           52 / 0 / 100.0%      13 / 0 / 100.0%      24 / 0 / 100.0%
```

89 scenarios = 41 fixed + 48 race trials (the full phase x mutation x
operation grid). Seeds 2, 3, and 99 gave the same two tables; at the default
trial count the seed changes trial order and fixture nonces only.

### Containment failures by category

From `results/seed-1/metrics.csv` (`adapter-category` rows). Each cell is
containment failures, with false refusals after a slash where there are any.

| Category (scenarios) | naive-join | string-prefix | lexical-prefix | canonicalize-then-open | open-then-verify | beneath-strict | beneath-fallback-lexical | beneath-fallback-walk |
|----------------------|-----------:|--------------:|---------------:|-----------------------:|-----------------:|---------------:|-------------------------:|----------------------:|
| normal (3)           | 0          | 0             | 0              | 0                      | 0                | 0              | 0                        | 0                     |
| normalization (8)    | 4          | 1             | 0              | 0                      | 1                | 0              | 0                        | 0                     |
| symlink (9)          | 6          | 6             | 6              | 1                      | 2                | 0 / 1          | 0 / 1                    | 0 / 1                 |
| dir-rename (5)       | 2          | 2             | 2              | 2                      | 1                | 0              | 0                        | 0                     |
| file-replace (4)     | 2          | 2             | 2              | 2                      | 1                | 0              | 0                        | 0                     |
| retry (5)            | 0 / 1      | 3             | 3              | 2                      | 1                | 0              | 3                        | 0                     |
| fallback (7)         | 3          | 2             | 2              | 0                      | 1                | 0 / 3          | 3                        | 0 / 1                 |
| race-sim (48)        | 12         | 18            | 18             | 12                     | 9                | 0              | 12                       | 0                     |
| **total (89)**       | **29 / 1** | **34**        | **33**         | **19**                 | **16**           | **0 / 4**      | **18 / 1**               | **0 / 2**             |

### Exposure

Adapters are not equally exposed, because a mutation scheduled for a phase
an adapter does not have is never applied to it. The first row is the
`interleave_fired` / `interleave_cases` figure from each adapter's coverage
block. The second was computed from `run.json` with a short script and is
not a field the tool emits: of the cases where an *escape* mutation was
applied before the handle was open, how many became containment failures.

|                                  | naive-join | string-prefix | lexical-prefix | canonicalize-then-open | open-then-verify | beneath-strict | beneath-fallback-lexical | beneath-fallback-walk |
|----------------------------------|-----------:|--------------:|---------------:|-----------------------:|-----------------:|---------------:|-------------------------:|----------------------:|
| mutations applied / scheduled    | 32 / 60    | 42 / 60       | 42 / 60        | 42 / 60                | 42 / 60          | 42 / 60        | 51 / 60                  | 60 / 60               |
| failures / escape mutations applied | 16 / 16 | 22 / 22       | 22 / 22        | 16 / 22                | 11 / 22          | 0 / 22         | 12 / 28                  | 0 / 34                |

## What the fixtures show

Each statement is about these reference adapters, on this kernel, in the
covered scenarios.

- **Baseline works.** Every adapter completes all three benign `normal`
  scenarios, and the unchecked `naive-join` fails 13 of the 14 must-refuse
  scenarios, so those scenarios are in fact hostile. The fourteenth injects
  a transient error, which stops `naive-join` before it opens anything.
- **Path-bound checks do not survive a state change in the check/use gap.**
  `canonicalize-then-open` refuses every static symlink escape, yet fails
  whenever an escape link is swapped in after its check (16 of the 22 cases
  where one was applied; the other 6 are swaps made before it started, which
  its check sees).
- **Verifying the handle after the open holds for reads and not for
  writes.** All 16 `open-then-verify` failures are writes. In each one the
  adapter refused, but its truncating or creating open had already changed
  an outside file. The oracle reports these with `outside-region-modified`
  as the only evidence.
- **Retries can reuse a stale decision.** `canonicalize-then-open` validates
  once and retries the open; when the target is swapped between the error
  and the retry, it fails (8 of its 12 retried cases).
- **A weaker fallback fails only on the fallback path.**
  `beneath-fallback-lexical` has no failures in 52 direct cases and 18 in 37
  cases that reached its fallback. Its preferred path and its compatibility
  path enforce different policies.
- **An equally strict fallback keeps the policy and costs availability.**
  `beneath-fallback-walk` has no failures on any path, with mutations
  applied in all 60 interleaved cases. It has two false refusals: a benign
  in-root symlink under fallback that the preferred method allows, and the
  benign absolute in-root symlink that `RESOLVE_BENEATH` itself rejects.
- **Failing closed keeps the policy and costs more availability.**
  `beneath-strict` has no failures and the highest false-refusal rate
  (4 of 23 must-allow cases): three when the preferred method is
  unavailable, one for a benign absolute in-root symlink that
  `RESOLVE_BENEATH` rejects.
- **Pooled rates mislead.** `naive-join` has fewer failures than
  `string-prefix` (29 against 34) although it checks nothing, because it
  never retries and so is not exposed to the before-retry mutation.

## Current capabilities

- CLI `policycontinuity` with `run`, `list-scenarios`, `list-adapters`,
  `probe`.
- One containment policy: operations stay beneath one approved root.
- 41 fixed scenarios across normal access, path normalization, symlink
  resolution, directory rename, file replacement, retry/error paths, and
  fallback; plus a seeded race simulation over a 48-combination grid.
- Deterministic interleaving of state changes at six named phases; injected
  transient errors; simulated absence of `openat2`.
- Eight reference adapters, labelled as benchmark fixtures in code, CLI
  output, and JSON.
- A policy oracle with three independent signals (handle identity,
  outside-region snapshot, returned-data canary).
- JSON and CSV output (per-case and per-aggregate), with no timestamps or
  host paths; byte-identical for a repeated seed.
- Metrics: policy-preservation rate, containment failures, safe refusals,
  false refusals, correct-outcome rate, scenario coverage and mutation
  reach, and behaviour split by direct / retry / fallback path.
- `--seed`, `--race-trials`, `--adapter`, `--category`, `--temp-parent`,
  and a `--fail-on-containment-failure` gate.
- Static confinement validation of every scenario, runtime integrity
  checks, and a decoy test confirming nothing outside the temporary
  workspace is touched.
- `make test` / `make check`; GitHub Actions workflow for format, lint,
  build, test, and a smoke run.

## Remaining limitations

The full list is in [docs/LIMITATIONS.md](docs/LIMITATIONS.md). The ones
that most affect how far these results can be taken:

- **No real application is measured.** Adapters are fixtures; their results
  do not transfer to real software.
- **Races are simulated at cooperative checkpoints.** There is no real
  concurrency, and kernel-internal windows are not modelled.
- **One kernel, one architecture.** Not reproduced on other kernels,
  distributions, or on aarch64. The path where `openat2` is genuinely
  missing has only been exercised through simulation.
- **Two operations** (read, create/truncate-write) and a narrow set of
  object types: no hard links, mount points, rename, or unlink.
- **Small, unevenly sized scenario set**; expectation labels are judgement
  calls; pooled rates are not rankings.
- **Single author** for scenarios, adapters, oracle, and expected results.
- **The GitHub Actions workflow has not been executed.** It was written and
  reviewed but there was no way to run it locally. It lives in
  `policycontinuity/.github/workflows/`, which GitHub only reads when this
  directory is the root of its repository.
- **No minimum supported Rust version** has been established; only
  rustc 1.99.0 was used.

## Notes on how this was produced

- The machine had no Rust toolchain. Stable Rust was installed with rustup
  into `~/.rustup` and `~/.cargo`, and the crate dependencies were
  downloaded from crates.io. Those are the only network uses; the benchmark
  itself has no network code.
- Nothing was committed to git, and no nested repository was initialised.
