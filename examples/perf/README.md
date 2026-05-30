# RustCFML performance benchmarks (PR-0)

The measurement foundation for the performance plan. Every optimization in that
plan is currently an *estimate*; this harness turns the estimates into measured
deltas so we can tell whether a refactor actually paid off (and didn't regress
correctness or memory).

## Running

```bash
examples/perf/run.sh              # build release, run all benches, append results.csv
examples/perf/run.sh --no-build   # reuse the existing release binary
REPEATS=5 examples/perf/run.sh    # more samples (default 3)
```

Each row in `results.csv` is tagged with the git short SHA (with `+dirty` if the
tree has uncommitted changes), so you compare a before/after by diffing rows:

```
sha,stamp,bench,median_ms,min_ms,max_ms,peak_rss_kb,repeats
```

- **wall-clock** comes from the bench's own `getTickCount()` around the hot
  region only — it excludes process startup and (for most benches) data setup,
  so it isolates the thing under test.
- **peak RSS** comes from `/usr/bin/time -l` (macOS) wrapping the whole process.
  It includes startup, so treat it as a coarse memory-footprint signal, useful
  mainly for the T1.1 "shrink CfmlValue" delta.

## The benches

| Bench | Hot region | Plan items it tracks |
|---|---|---|
| `bench_loop` | 10M-iter integer loop, local load/store/add | T3.1 (slot locals), T1.2 (drop `to_lowercase` per access) |
| `bench_struct` | 100k `new Point()` + property reads + method call | T1.3 (CI-hash lookup), T3.2 (inline caches), call dispatch |
| `bench_closure` | `arrayMap` over 10k with a **function-local** capturing callback | T2.4 (per-variable closure capture vs whole-scope CoW) |
| `bench_concat` | 2M bounded string concats | T1.1 (`String(Arc<str>)`), T4.1 (in-place writer) |
| `bench_template` | 200k-iter `cfloop`/`cfif`/`cfoutput` into a captured buffer | tag pipeline + output buffering |

## Size probes

Type sizes are asserted (as non-regression ceilings) by Rust unit tests:

```bash
cargo test -p cfml-common  size_probe -- --nocapture   # CfmlValue & friends
cargo test -p cfml-codegen size_probe -- --nocapture   # BytecodeOp
```

Baseline at PR-0: `CfmlValue` = **112 B** (the plan's "88 B" was stale — the
enum has since grown via the `QueryColumn` variant and a fatter `CfmlFunction`),
`BytecodeOp` = **64 B**. Tighten the ceilings as planned shrinks land.

## Findings surfaced while building this harness

Two real bugs fell out of writing the benches — both worth fixing independently
of the perf work:

1. **Array index-assign past the end doesn't auto-grow.** `a = []; a[1] = "x"`
   leaves `a` empty (`arrayLen` 0) instead of extending it. Lucee/CFML
   auto-extend (filling gaps). Assigning to an *existing* index works;
   `arrayAppend` works. Correctness bug.
2. **`arrayAppend` is O(n²).** Appending in a loop clones the whole backing
   `Vec` every call (Arc `make_mut` sees ≥2 refs because the array also lives in
   the scope map): 10k→340ms, 20k→1.8s, 40k→7.8s. A pure-performance bug, and a
   data point for the CoW-aliasing work in the plan.

The benches avoid both (build with `arrayAppend` at modest sizes, build data
outside the timed region) so they measure their intended hot path, not these.
