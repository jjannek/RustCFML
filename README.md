## RustCFML

![RustCFML Mascot](crab.svg)

A CFML (ColdFusion&reg; Markup Language) interpreter written in Rust — a single, fast, run-anywhere binary with a tiny memory footprint.

![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)

> ColdFusion is a registered trademark of Adobe Inc. RustCFML is not affiliated with or endorsed by Adobe.

**[Try RustCFML in your browser](https://rustcfml.github.io/RustCFML/demo/)** — interactive demo running on WebAssembly.

## Project Aims

RustCFML aims to be a **compatible, fast, run-anywhere** CFML engine with a minimal memory footprint and maximum performance. It is deliberately opinionated:

- **A lean, stable core.** We don't add things to the core that are prone to constant churn in the wider ecosystem. Reliability comes first — think of RustCFML as an LTS-style engine. It is already blazingly fast.
- **Libraries over built-ins.** We won't add core functions that are better served by libraries. Instead, where possible, we make the engine compatible enough to *run* those libraries.
- **No administrator, ever.** RustCFML does not have — and never will have — a ColdFusion Administrator. Configuration is file-based via [`.cfconfig.json`](docs/configuration.md), with environment-variable substitution for secrets.
- **Inspired by real apps and modern deployment.** We follow modern deployment practices and won't bake in CI/CD features that belong in your pipeline, not your language runtime.
- **Honest about Java.** There is no JVM under the hood. We provide limited Java-class shim support (faking common Java classes), but you should expect — and will find — differences.

## Project Background

RustCFML began as a proof of AI model capabilities by Alex Skinner, CEO of [Pixl8 Group](https://www.pixl8.co.uk/). It has been written almost entirely by AI — predominantly Claude Opus — with research and test synthesis assisted by local models.

## Getting Started

The fastest way to start is with a prebuilt binary — no toolchain required.

1. **Download a binary** for your platform from the **[latest release](https://github.com/RustCFML/RustCFML/releases/latest)** (Linux x86_64/aarch64, macOS aarch64), then put it on your `PATH`:

   ```bash
   chmod +x rustcfml-macos-aarch64
   sudo mv rustcfml-macos-aarch64 /usr/local/bin/rustcfml
   ```

2. **Run a web application** — the most common way to get going. Point RustCFML at a directory of `.cfm` files:

   ```bash
   rustcfml --serve ./mywebroot --port 8500
   ```

   It serves `.cfm` pages and static assets, runs the `Application.cfc` lifecycle, and supports sessions, URL rewriting, and file uploads. See **[Web Server](docs/web-server.md)**.

3. **Go to production** — add `--production` to enable warm in-memory caching (skips per-request filesystem checks for a 3–4× throughput gain):

   ```bash
   rustcfml --serve ./mywebroot --production
   ```

   See **[Deployment](docs/deployment.md)** for production mode, the sandbox/virtual filesystem, standalone binaries, and Cloudflare Workers.

You can also run a single template (`rustcfml myapp.cfm`), drop into a REPL (`rustcfml -r`), or run inline code (`rustcfml -c '...'`). See **[Getting Started](docs/getting-started.md)** for those and shebang scripts.

## Performance

RustCFML compiles to a native binary with no runtime VM overhead, so it starts instantly and serves requests with a fraction of the memory of JVM-based CFML engines.

Serving a "Hello World" `.cfm` page in `--production` mode against a warmed Lucee 7 — same machine (Apple M-series), same page, Apache Bench, 8s runs. Requests/sec, higher is better:

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

RustCFML serves roughly 2–3.5× the throughput at about a tenth of the memory, with no JVM warmup. Both engines benefit from HTTP keep-alive; RustCFML scales further with it, sustaining ~26,000 req/s. See **[Performance](docs/performance.md)** for full methodology and production-mode caching.

## Documentation

| Topic | Description |
|---|---|
| **[Getting Started](docs/getting-started.md)** | Prebuilt binaries, running files, REPL, shebang scripts, building from source |
| **[Web Server](docs/web-server.md)** | Serve mode, Application.cfc lifecycle, URL rewriting, distributed sessions |
| **[Configuration](docs/configuration.md)** | `.cfconfig.json` — datasources, mappings, mail, security, caches, env vars |
| **[Deployment](docs/deployment.md)** | Web app, Docker, CLI tools, Cloudflare Workers; production mode & sandbox |
| **[Database](docs/database.md)** | `queryExecute`, datasources, `cfqueryparam`, engine specifics |
| **[Object Storage](docs/s3.md)** | S3 / R2 / MinIO — `S3*` functions and transparent `s3://` paths |
| **[Native Modules](docs/native-modules.md)** | Extend a binary with first-class Rust built-ins and classes |
| **[Embedding](docs/embedding.md)** | Use the RustCFML engine from your own Rust code |
| **[WebAssembly](docs/wasm.md)** | Compile to WASM; Cloudflare Workers notes |
| **[Architecture](docs/architecture.md)** | Compilation pipeline and crate layout |
| **[Performance](docs/performance.md)** | Benchmarks and production-mode caching |
| **[Testing](docs/testing.md)** | Running the test suites and writing tests |
| **[Status](docs/status.md)** | Implementation status and remaining work |

## Deployment

RustCFML is designed to deploy as a single artifact in several shapes — see **[Deployment](docs/deployment.md)** for full detail:

- **Web application** — run `--serve` behind your reverse proxy, with `--production` for warm in-memory caching:

  ```bash
  rustcfml --serve ./mywebroot --production
  ```

- **Optimised Docker container** — *coming soon*: a minimal image for containerised deployment.

- **CLI tool** — compile a CFML app into a standalone command-line binary. See **[Deployment → CLI tools](docs/deployment.md#cli-tools)**.

- **Cloudflare Workers** — run RustCFML at the edge via WebAssembly. See **[RustCFML-Cloudflare-worker](https://github.com/RustCFML/RustCFML-Cloudflare-worker)**.

**Production mode** (warm caching) and **sandbox / virtual filesystem** (host isolation, embedded files) apply to both web and CLI deployments — they're documented once in **[Deployment](docs/deployment.md)**.

## Features

- **Complete CFML language** — CFScript and tag syntax (a preprocessor converts 50+ tags to CFScript), components with inheritance and interfaces, closures, member functions, and higher-order functions across arrays, structs, queries, and lists.
- **400+ built-in functions** — strings, arrays, structs, dates, math, lists, queries, JSON, XML, regex, encoding, hashing, and modern password hashing (bcrypt/scrypt/argon2).
- **Batteries-included web server** — `Application.cfc` lifecycle, sessions (in-process, Memcached, or clustered), cookies, authentication, URL rewriting, and file uploads.
- **Data & integration** — `queryExecute` over SQLite, MySQL, PostgreSQL, and MSSQL with pooling and `cftransaction`; `cfhttp`; `cfmail`; and S3-compatible object storage (AWS S3, Cloudflare R2, MinIO).
- **Run anywhere** — native binaries, self-contained single-file apps, and a WebAssembly target that runs on Cloudflare Workers.
- **Extensible** — drop in first-class built-ins and classes written in Rust ([native modules](docs/native-modules.md)).

See **[Compatibility & Status](docs/status.md)** for implementation status.

### Not Supported

- **Query-of-Queries (QoQ)** — SQL SELECT on in-memory query objects
- Image functions, Spreadsheet functions, ORM, SOAP/WSDL, Flash/Flex, PDF, LDAP, Registry
- `cfschedule`, `cfwddx`, real concurrent `cfthread` execution

## Architecture

```plaintext
CFML Source (.cfm / .cfc)
    → Tag Preprocessor → CFScript → Lexer → Parser → AST → Compiler → Bytecode → VM → Output
```

RustCFML is a Cargo workspace of focused crates (`cfml-common`, `cfml-compiler`, `cfml-codegen`, `cfml-vm`, `cfml-stdlib`, `cli`, `wasm`). See **[Architecture](docs/architecture.md)** for the full breakdown.

## Building from Source

If you'd rather build it yourself (or you're contributing), you need Rust stable (>= 1.75.0) — install via [rustup.rs](https://rustup.rs/):

```bash
git clone https://github.com/RustCFML/RustCFML.git
cd RustCFML
cargo build --release        # binary at target/release/rustcfml
cargo install --path crates/cli   # optional: install on your PATH
```

See **[Getting Started → Building from source](docs/getting-started.md#building-from-source)** for feature flags, the WebAssembly target, and the self-contained-binary build path.

## Contributing

Contributions are welcome.

- **New here?** If you haven't contributed before, please **[open an Issue](https://github.com/RustCFML/RustCFML/issues)** with detail (a minimal reproducible CFML snippet, expected vs actual behaviour) before opening a PR.
- **Pull requests are the preferred way to contribute a fix.** A great place to start is a **CFML-based test** that demonstrates the behaviour (see **[Testing](docs/testing.md)**).
- **Lucee is the reference for compatibility.** Your test **must pass on Lucee** — if it doesn't, we won't accept it. RustCFML targets [cfdocs.org](https://cfdocs.org) with Lucee as the primary implementation target. (By rare exception, where Lucee allows something genuinely unreasonable, we may choose not to match it.)

See **[Testing](docs/testing.md)** for how to run the suite against both RustCFML and Lucee.

### Contributors

[![Contributors](https://contrib.rocks/image?repo=RustCFML/RustCFML)](https://github.com/RustCFML/RustCFML/graphs/contributors)

_Avatars are generated automatically from the [GitHub contributor graph](https://github.com/RustCFML/RustCFML/graphs/contributors) by [contrib.rocks](https://contrib.rocks)._

## Project Inspiration

- [Lucee](https://github.com/lucee/Lucee) — open-source CFML engine (Java)
- [BoxLang](https://github.com/ortus-boxlang/BoxLang) — modern CFML+ runtime (Java)
- [RustPython](https://github.com/RustPython/RustPython) — Python interpreter in Rust (architectural reference)

## License

MIT
