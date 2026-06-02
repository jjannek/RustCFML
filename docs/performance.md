# Performance

[← Back to README](../README.md)

RustCFML compiles to a native binary with no runtime VM overhead — it starts instantly and serves requests with a fraction of the memory of JVM-based CFML engines.

## Benchmark

Serving a "Hello World" `.cfm` page in `--production` mode against a warmed Lucee 7, on the same machine (Apple M-series), same page, using Apache Bench with 8-second runs. Requests/sec, higher is better:

| Concurrency | RustCFML | Lucee 7.0 | RustCFML (keep-alive) | Lucee (keep-alive) |
|---|---|---|---|---|
| `-c 1`   | **1,908** | 1,205 | **3,118**  | 1,625 |
| `-c 10`  | **5,466** | 2,648 | **21,716** | 8,125 |
| `-c 50`  | **6,983** | 3,503 | **25,833** | 8,085 |
| `-c 100` | **7,528** | 3,107 | **25,855** | 7,419 |

| | RustCFML | Lucee 7.0 |
|---|---|---|
| **Memory (RSS, under load)** | **~60 MB** | ~560 MB |
| **Startup** | **instant** | ~15s |

### Notes on methodology

- **Both engines were warmed** before measuring (a JVM needs JIT warmup to reach steady state; an early, cold run undercounts Lucee heavily).
- **Keep-alive helps both engines** — roughly 3× for Lucee and 4× for RustCFML — by amortising TCP connection setup. The non-keep-alive columns are closer to a "cold client" worst case.
- Numbers are from a developer laptop and are indicative, not a lab benchmark. They will vary by hardware, OS, and page complexity.
- BoxLang is not included in this run; it is broadly comparable to Lucee on this workload.

To reproduce: build `--release`, serve a one-line `<cfoutput>Hello World</cfoutput>` page with `--production`, warm with `ab -n 20000 -c 50`, then measure with `ab -t 8 -c <N>` (add `-k` for keep-alive).

## Production mode caching

By default the server re-validates files on each request (statting `Application.cfc` resolution and every cached bytecode entry) so edits are picked up live. Passing `--production` (or `RUSTCFML_PRODUCTION=1`) enables three persistent in-memory caches:

- **Application.cfc path resolution** — the directory walk is done once, then memoized (including negative results).
- **URL → file resolution** — routing `is_file` stats are memoized.
- **Bytecode cache trust** — the per-hit `mtime` check on every compiled file is skipped.

Once warm, requests pay zero filesystem IO. The typical speedup on an app with `Application.cfc` + cfincludes is 3–4× requests/sec. Files changed on disk are not picked up until restart. Self-contained binaries running in sandbox mode enable production caching automatically, since the embedded VFS is immutable. See **[Deployment](deployment.md)**.
