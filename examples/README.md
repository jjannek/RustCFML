# Examples

[← Back to README](../README.md)

A tour of RustCFML, from language basics to full applications and native Rust extensions. Run a single file with `rustcfml <file>` (or `cargo run -- examples/<file>`), and serve an app directory with `rustcfml --serve examples/<dir>`.

## Language basics

Numbered, self-contained scripts — run each with `rustcfml examples/0N_*.cfm`:

| File | Shows |
|---|---|
| `01_hello.cfm` | Output and the basics |
| `02_variables.cfm` | Variables and scopes |
| `03_conditionals.cfm` | `if`/`else`, `switch` |
| `04_arrays.cfm` | Arrays and member functions |
| `05_ternary.cfm` | Ternary and Elvis operators |
| `06_expressions.cfm` | Expressions and operators |
| `07_booleans.cfm` | Boolean coercion |
| `08_builtins.cfm` | A sampling of built-in functions |

## Scripts

| File | Shows |
|---|---|
| `shebang_test.cfm` / `shebang_test.sh` | Running a `.cfm` as an executable shell script (shebang support) |
| `mem_stress.cfm` | Building large data structures (memory behaviour) |

## Web applications

Serve these with `rustcfml --serve examples/<dir>`:

| Directory | Shows |
|---|---|
| `miniapp/` | A small multi-page web app — `Application.cfc`, includes (header/footer), and templates |
| `taffytest/` | A [Taffy](https://github.com/atuttle/Taffy) REST API with `urlrewrite.xml` routing |
| `interactivejs/` | A browser front-end (`index.html`) driving the WASM build |

## Performance

| Directory | Shows |
|---|---|
| `perf/` | Micro-benchmarks (`bench_loop`, `bench_struct`, `bench_closure`, `bench_concat`, `bench_template`). See its [README](perf/README.md). |

## Native (Rust) modules

Built with `rustcfml --build` — see **[Native Modules](../docs/native-modules.md)**:

| Directory | Shows |
|---|---|
| `native_module_demo/` | First-class Rust built-ins and a Rust-backed CFC (`BoostedTally.cfc`). See its [README](native_module_demo/README.md). |
| `native_markdown/` | A native module wrapping a Rust markdown crate. See its [README](native_markdown/README.md). |
