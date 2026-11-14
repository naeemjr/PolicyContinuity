# Limitations

Read this before drawing any conclusion from a run.

## Scope of the claims

- **No real application is measured.** The adapters are fixtures written for
  this benchmark. They resemble ordinary approaches but model no specific
  program, library, or version. A result for an adapter does not transfer to
  real software that uses a similar technique: real implementations combine
  techniques, add checks, and differ in details that decide the outcome.
- **This is not evidence that applications are vulnerable**, in general or
  in any proportion. The share of failing cases reflects which scenarios and
  fixtures were written.
- **Passing is not a safety proof.** An adapter with no containment failures
  survived the covered scenarios only.

## Scenario coverage

- The catalogue is small: 41 hand-written scenarios and a 48-combination
  race grid. Categories have unequal sizes, so pooled rates are weighted by
  the mix.
- Only two operations exist: read, and create/truncate-write. There is no
  rename, unlink, mkdir, chmod, stat, directory listing, or execute.
- Not covered: hard links, bind mounts and mount points, `/proc` magic links
  as targets, device files and FIFOs, case-insensitive or normalising
  filesystems, non-UTF-8 names, path-length limits, NUL bytes, deep symlink
  chains (`ELOOP` limits), `O_TMPFILE`, extended attributes.
- The root directory and its ancestors are trusted and never change.
- Objects never cross the policy boundary; only names and symlinks change.
  Scenarios in which an in-root directory is moved out from under a walk
  (the `..` escape against naive `openat` walks) are therefore not covered.
- One mutation per scenario, at one phase. No sequences such as
  swap-then-swap-back (ABA), and no mutation timed inside a system call.
- Expectations are judgement calls. For example, leaving the root and
  re-entering it is labelled allow-or-refuse, and a benign absolute in-root
  symlink is labelled must-allow, which counts kernel-enforced refusal of it
  as a false refusal. Different labels would move the false-refusal figures.

## The race model

- **Races are simulated, not raced.** Mutations are applied synchronously at
  checkpoints the adapter announces. There are no threads and no timing.
  This shows whether a window *exists* and is exploitable in the worst case.
  It says nothing about how wide the window is or how likely a real race is
  to hit it.
- **Checkpoints are cooperative.** A window the adapter does not announce is
  not tested. The reference adapters announce their windows faithfully, but
  the benchmark cannot verify that for a new adapter.
- **Kernel-internal windows are not modelled.** `openat2` is treated as
  atomic. Its real behaviour under concurrent renames is not exercised.
- **Reach differs by adapter.** A mutation scheduled for a phase an adapter
  lacks is never applied to it, and the case counts as a clean run. Adapters
  with fewer code paths therefore face fewer hostile cases. Pooled rates are
  not comparable across adapters; `interleave_reach_rate` and the per-case
  `interleave_fired` flag show how much each adapter was actually exposed.

## The fault model

- Only two faults: `EAGAIN` on the first *N* opens, and `ENOSYS` from the
  preferred safe open. No `ENOMEM`, `EMFILE`, `EINTR` mid-I/O, short reads
  or writes, signals, or permission changes.
- The injected `EAGAIN` applies to every open made through the environment,
  including intermediate directory opens in a component-wise walk.
- Genuine `EAGAIN` results from `RESOLVE_BENEATH` are retried inside the
  harness so that real timing cannot enter the results. A real application
  would have to handle them.
- "Unavailable" is simulated by returning `ENOSYS`. Other real-world causes
  of a fallback (seccomp filters returning `EPERM`, container runtimes,
  filesystems that reject the flags) are not distinguished.

## The oracle

- Containment is judged on objects, by inode identity and by the contents of
  the outside region. Metadata changes (mode, ownership, timestamps) are not
  observed.
- Opening an outside file for reading and then discarding the handle is not
  counted as a failure, because no data is returned and nothing changes.
  Side channels (existence, timing, error text) are out of scope.
- The oracle and the `open-then-verify` adapter both rely on
  `/proc/self/fd`. An environment without `/proc` cannot run the benchmark.
- Hard links are excluded partly because they would make inode identity
  ambiguous between regions.

## Environment dependence

- **Linux only.** The crate does not compile elsewhere.
- **Kernel dependent.** `openat2` requires kernel 5.6. Symlink, `O_NOFOLLOW`,
  and `RESOLVE_BENEATH` semantics are the kernel's. Results were produced on
  a single kernel (6.18, WSL2, x86_64) and have not been reproduced on other
  kernels, distributions, or architectures.
- **Filesystem dependent in principle.** The sample results were identical
  on tmpfs and on an ext-family filesystem. Network, overlay, FUSE, and
  case-insensitive filesystems are untested.
- **Only `RESOLVE_BENEATH` is studied** as the preferred mechanism. Other
  containment mechanisms (`RESOLVE_IN_ROOT`, `chroot`, mount namespaces,
  Landlock, seccomp, capability-based runtimes) are not represented.

## Validity of the benchmark itself

- **Circularity.** The same author wrote the scenarios, the adapters, the
  oracle, and the expected results in the test suite. The tests show the
  implementation matches the author's reasoning; they are not independent
  confirmation that the reasoning is right.
- **Adapter design drives the results.** Small changes change outcomes: had
  `open-then-verify` opened without `O_TRUNC` and truncated after verifying,
  its write-path failures on existing files would disappear. The fixtures
  encode one plausible choice each, not the best version of each approach.
- **The confinement validator is conservative and hand-argued**, not
  formally verified. It is backed by runtime integrity checks and a decoy
  test, which reduce but do not eliminate the risk of a mistake.
- **No minimum supported Rust version has been established.** The code was
  built and tested only with rustc 1.99.0.
- **CI has not been executed.** The workflow file was written but never run
  on GitHub Actions.

## What would strengthen it

- Adapters that wrap real libraries or run real programs as subprocesses.
- A real-concurrency mode to complement the deterministic one, reporting hit
  rates over many trials.
- More operations and object types, especially rename, unlink, hard links,
  and mount points.
- Runs across kernel versions, architectures, and filesystems.
- Independent review of scenario expectations.
