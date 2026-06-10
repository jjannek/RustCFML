# QoQ Performance — Next Steps (closing the BoxLang gap)

> Ordered plan written immediately after the v0.104.0 perf release, while the
> measurements and BoxLang source-code read are fresh. Targets BoxLang
> (`1.0.0-snapshot`), which on this machine is **~5× faster total** than RCF
> and ~7× faster than Lucee 7.

## Where we are (v0.104.0, same-machine 5-run median, 1M rows)

| # | Query | RCF | Lucee 7 | BoxLang | RCF/BL |
|---|-------|----:|--------:|--------:|:------:|
| 1 | basic SELECT | 813 | 856 | **65** | 12.5× |
| 2 | basic UNION | 782 | 563 | **72** | 10.9× |
| 3 | non-grouping agg | 305 | 297 | **146** | 2.1× |
| 4 | grouped agg | 180 | 132 | **93** | 1.9× |
| 5 | string concat | 187 | 135 | **74** | 2.5× |
| 6 | LIKE `%Harry%` | 145 | 95 | **34** | 4.3× |
| 7 | 2-tbl comma join | 525 | 1150 | **81** | 6.5× |
| 8 | 3-tbl comma join | 604 | 1371 | **231** | 2.6× |
| 9 | ANSI 3-tbl INNER JOIN | 602 | 1431 | **100** | 6.0× |
| 10 | multi-table UNION (5×) | 2304 | 3174 | **382** | 6.0× |
| | **Total** | 6447 | 9205 | **1278** | **5.0×** |

**RCF already beats Lucee 7** on Q1/Q7/Q8/Q9/Q10 and total. The remaining
target is BoxLang.

## What BoxLang does that we don't

Reading `BoxLang/src/main/java/ortus/boxlang/runtime/jdbc/qoq/`:

| BoxLang | RustCFML | Cost on bench |
|---|---|---|
| `Stream<int[]>` from seed → joins → WHERE → projection in one pipeline; never materialised | Materialises `Vec<Vec<usize>>` between phases (`build_core_intersections` → `filter_where` → `exec_simple`) | Q1/Q2 dominated by alloc + iteration overhead |
| Parallel-stream threshold **50 rows** for intersections, **50 partitions** for group-by, **100 rows** for dedup | Threshold **10,000** rows for everything | Q1/Q2 effectively single-thread; we parallelise projection but not the upstream stream |
| Result columns carry a `QueryColumnType` (INTEGER/VARCHAR/...); `QoQCompare.invoke(type, a, b)` dispatches once per column | Compares two `CfmlValue` enums per cell, branches on both discriminants | Q3/Q4 (lots of compares in `partition`/`order_by`) |
| `int[]` primitive intersection arrays (no boxing) | `Vec<usize>` heap allocation per intersection | Q4/Q5/Q9 — allocator pressure visible under `samply` |
| `java.util.regex.Pattern` (HotSpot-JIT, SIMD-backed byte scan) cached by pattern string | Hand-rolled `Compiled::matches` + ASCII fast path (byte-by-byte, branchy loop) | Q6: BoxLang 34 ms, RCF 145 ms |
| AST nodes are subclasses with virtual `evaluate(QoQExec, int[])`; HotSpot devirtualises + inlines after warm-up | Recursive `fn eval(&Expr) { match expr { ... } }` per row — no devirtualisation | Q4/Q5/Q6 — per-row dispatch tax |
| Dedup, sort, partition all use `IntStream.parallel()` | Sort is sequential, dedup is sequential, partition was sequential until v0.104 | Q10 (5× UNION → dedup) and Q4 (group-by) |
| `ChunkedArrayList<Object[]>` for the target query (no copy on resize) | `Vec<Vec<CfmlValue>>` column-major (also amortised) | Roughly tied |

## Ordered plan

Each step has an **estimated win** (median ms saved from the 6447 total) and
**estimated effort** (S/M/L). Steps are ordered by effort-adjusted leverage —
cheap big wins first, slow small wins last.

### 1. `regex` crate for LIKE — **est. -100 ms total, S** ★ start here

Replace the hand-rolled `Compiled` matcher with the `regex` crate's
`RegexBuilder::case_insensitive(true).build(escaped_pattern)`. Cache per
compiled-pattern string in the existing `compile()` site. The `regex` crate
uses Aho-Corasick / SIMD byte scanning under the hood — much like Java's
`Pattern` engine. Keep the existing fast path for `lit`/`%lit%`/etc. as it
will still beat regex compile time for trivial patterns.

- **Files:** `crates/cfml-qoq/src/like.rs`, `Cargo.toml` (add `regex`).
- **Risk:** none — `regex` is a Rust ecosystem standard, well-tested.
- **Expected:** Q6 145 → ~50 ms. Maybe Q1 if we hit a LIKE there.

### 2. Lower parallel thresholds — **est. -300 ms total, S**

`PARALLEL_ROW_THRESHOLD = 10_000` is too high. BoxLang fans out at 50–100.
Drop to:
- Intersection stream: 1,000 (covers Q4/Q5/Q6/Q9 fully)
- Partition build: 50 partitions (already added in v0.104, threshold by row count of `intersections` still 10k — change to partition-count check after)
- Dedup: 1,000 rows
- Order-by sort: 10,000 rows (`par_sort_unstable_by`)

Measure carefully — rayon overhead on a too-small input regresses. Test on Q1
(1M rows, no join, simple WHERE) where parallelism should win, and on the
~100-row group-by output where it shouldn't.

- **Files:** `crates/cfml-qoq/src/execution.rs` (constants + new par-sort).
- **Risk:** small regressions on tiny queries — gate with a row-count check.
- **Expected:** Q1 813 → ~300 ms, Q2 782 → ~300 ms.

### 3. Stream-based intersection pipeline — **est. -1000 ms total, L** ★ biggest win

Architectural. Replace `build_core_intersections() -> Vec<Vec<usize>>` +
`filter_where(Vec<Vec<usize>>) -> Vec<Vec<usize>>` + `exec_simple(...)` with
a single rayon `ParallelIterator<Item = SmallVec<[usize; 4]>>` that flows:

```
seed (1..=row_count) →
  flatMap(JOIN k, hash-probe or generic) →
  filter(WHERE) →
  (limit if no order-by) →
  map(project to Vec<CfmlValue>) →
  collect to Vec<Vec<CfmlValue>>
```

Hash-join probes stay; generic nested-loop becomes `flat_map` over the right
table. WHERE becomes `.filter()`. LIMIT without ORDER BY becomes `.take(n)`
(BoxLang's `canEarlyLimit` optimisation). The 1M-row Q1 currently
allocates 1M `Vec<usize>` for the WHERE-pass even though every row is
single-column.

Use `SmallVec<[usize; 4]>` to keep single-table and 2–3-table intersections
on the stack — kills the per-row heap allocation entirely.

- **Files:** `crates/cfml-qoq/src/execution.rs` (rewrite `run_core` glue),
  possibly `intersection.rs` (becomes stream-based too — or stays as a
  fallback materialised builder for tricky outer joins).
- **Risk:** medium — touches the query lifecycle. Cross-engine test suite
  catches semantic regressions; the 20 qoq unit tests + 3612 CFML suite
  pin behaviour.
- **Expected:** Q1 813 → ~150 ms, Q2 782 → ~200 ms, Q5 187 → ~80 ms,
  Q7/Q9 also benefit. Total -1000+ ms.

### 4. Typed result columns + typed compare — **est. -150 ms, M**

At `bind_core` time, infer the type of every result-column and ORDER BY
expression (we already track this implicitly via `CfmlValue` variants — make
it explicit). A new `enum QoQType { Int, Double, String, Bool, Null, Mixed }`
attached to each `SelectColumn` and `OrderByExpr`.

`QoQCompare::invoke(typ, a, b)` dispatches once on the column type instead of
matching both `CfmlValue` discriminants per row. Same for `eval_binary` on
the hot ops (`+`, `=`, `<`).

- **Files:** `crates/cfml-qoq/src/{compare,execution,ast}.rs`.
- **Risk:** type inference subtleties for `Mixed` columns; default to the
  generic path when uncertain.
- **Expected:** Q3 305 → ~200 ms, Q4 180 → ~130 ms.

### 5. Return `&CfmlValue` from `tables.value()` — **est. -200 ms, M**

Currently `tables.value(inter, ti, ci)` clones a `CfmlValue` on every cell
access. For ResolvedColumn evaluation that's the per-row clone tax. Returning
`&CfmlValue` requires:
- `RowCtx::Row(&[usize])` → already by ref
- `eval_resolved_column` returns `&CfmlValue` for column refs
- Binary/compare ops borrow LHS/RHS instead of taking by value

The borrow checker will fight this. The lifetime is bounded by the
intersection's lifetime which is bounded by the iterator yielding it. Doable
with explicit lifetime annotations on `Engine::eval` and a `Cow<'_, CfmlValue>`
return type for cases that genuinely need ownership (Function results,
binary op results).

- **Files:** `crates/cfml-qoq/src/execution.rs`.
- **Risk:** lifetime annotations propagate widely; might require an
  `enum EvalResult<'a> { Borrowed(&'a CfmlValue), Owned(CfmlValue) }`.
- **Expected:** ~200 ms total saved across Q4/Q5/Q9 (lots of column reads).

### 6. Parallel sort + dedup — **est. -300 ms, S**

`order_by_output` and `dedup_*` are sequential. Use
`rayon::slice::ParallelSliceMut::par_sort_unstable_by` and a `DashMap` for
dedup beyond the 1k row threshold.

- **Files:** `crates/cfml-qoq/src/execution.rs`.
- **Risk:** none significant.
- **Expected:** Q10 2304 → ~1500 ms (most of its time is dedup + sort of
  5M-row pre-union result).

### 7. `SmallVec` for intersections — **est. -100 ms, S**

Even after the streaming rewrite, intersection vecs for 2–3-table joins
benefit from stack storage. Drop-in replace with
`smallvec::SmallVec<[usize; 4]>`.

- **Files:** `crates/cfml-qoq/src/{execution,intersection,table}.rs`.
- **Risk:** none — `SmallVec` is API-compatible with `Vec`.
- **Expected:** Q4/Q9 each save ~50 ms.

### 8. Compile expressions to typed closures — **est. -500 ms, L**

The "JIT-equivalent." Each `Expr` node compiles at `bind_core` time into a
`Box<dyn Fn(&[usize], &TableSet, &EvalCtx) -> CfmlValue + Send + Sync>`.
Per-row evaluation becomes one indirect call instead of an N-deep
match-dispatch AST walk.

```rust
enum CompiledExpr {
    ColumnSlot { ti: u32, ci: u32 },
    Literal(CfmlValue),
    EqIntInt(Box<CompiledExpr>, Box<CompiledExpr>),
    EqStrStr(Box<CompiledExpr>, Box<CompiledExpr>),
    AddIntInt(Box<CompiledExpr>, Box<CompiledExpr>),
    // ... type-specialised variants
    Generic(Expr),  // fallback
}
```

Specialise the dozen most common shapes (column = literal, column +
literal, COUNT(*), AVG(col), upper(col), …). Fall back to the generic AST
walk for anything else.

- **Files:** new `crates/cfml-qoq/src/compiled.rs`; rewires `execution.rs`.
- **Risk:** large — but well-contained (the compiled-expr layer is internal).
- **Expected:** ~500 ms across Q3/Q4/Q5. This is the "BoxLang gets it for
  free from HotSpot" lever — we have to build it.

### 9. Avoid `group_key` string allocation — **est. -100 ms, M**

`partition` and dedup hash by a `String` synthesised via `group_key()`.
Switch to a `u64` `xxhash` of the concatenated cell representations, or
hash directly via `std::hash::Hasher` without materialising the string.

- **Files:** `crates/cfml-qoq/src/{compare,execution}.rs`.
- **Risk:** low (collisions need second-pass equality check on bucket).
- **Expected:** Q4 -50 ms, Q10 -50 ms.

### 10. `canEarlyLimit` optimisation (LIMIT without ORDER BY) — **est. -100 ms, S**

BoxLang's trick: if there's no ORDER BY and no aggregate, stop iterating
when the LIMIT is reached. Plumb `.take(n)` into the streaming pipeline.

Not exercised by the current bench but a real-world ergonomic win.

- **Files:** `crates/cfml-qoq/src/execution.rs`.
- **Risk:** none.
- **Expected:** N/A in this bench; 10×–100× on any `LIMIT N` query in user code.

## Out of scope / structural limits

- **HotSpot's expression devirtualisation** — we can't replicate this without
  Cranelift-compiling each query's WHERE/SELECT pipeline. The compiled-expr
  step (#8) is the closest hand-written approximation; getting past that
  requires going the JIT route the main VM already took.
- **GC vs Arc** — BoxLang inherits Java's generational GC; we inherit `Arc`
  for shared `String`s. For QoQ-like workloads with millions of short-lived
  string clones, this is a real per-row cost we'd need to address with an
  arena (e.g. `bumpalo`) scoped to one query execution.

## Suggested staging

| Wave | Steps | Total est. ms saved | Effort |
|------|-------|--------------------:|:------:|
| **3** | 1, 2 | -400 | half-day |
| **4** | 3 | -1000 | 2–3 days |
| **5** | 4, 5, 6, 7 | -750 | 2–3 days |
| **6** | 8 | -500 | week+ |
| **7** | 9, 10 | -200 | half-day |
|  | **Total** | **~-2850 ms → ~3600 ms total** | |

That lands RCF in the **2–3× BoxLang range** (vs 5× today) without changing
the language semantics or growing CfmlValue. Closing the last 2× would mean
either (a) compiled-expr's reaching a higher coverage of patterns, (b)
arena-allocated strings per query, or (c) accepting that HotSpot has a
decade head start and shipping what we've got.

## Measurement discipline

Each step lands behind:
1. `cargo test -p cfml-qoq` (20 unit tests)
2. `tests/runner.cfm` (3612 assertions, 434 suites)
3. Lucee 7 cross-engine on the QoQ suites
4. `bdw429s/cfml-qoq-perf-tests` (5-run median) — record the per-Q delta in
   the commit message

Don't ship a step that regresses any Q by >5% without an explicit reason.
