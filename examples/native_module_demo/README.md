# Native module demo

A CFML app that calls into a Rust module. Demonstrates the `rustcfml --build`
**cocktail** path: when a project contains `native/<crate>/Cargo.toml`, the
build step mixes the Rust module into a fresh self-contained binary
alongside the embedded CFML app.

## Layout

```
native_module_demo/
├── main.cfm                       — CFML entry point
└── native/
    └── greeter/                   — one native module (one Rust crate)
        ├── Cargo.toml
        └── src/lib.rs             — exposes pub fn register(vm: &mut Vm)
```

## Build & run

From the repo root:

```bash
cargo run --release -- --build examples/native_module_demo \
    -o /tmp/greeter_demo \
    --mode cli \
    --entry main.cfm

/tmp/greeter_demo
```

Expected output:

```
Greeting from Rust: Hello, Alex (from Rust)
2 + 3 (computed in Rust) = 5
Tally after 3 bumps: 3
BoostedTally.bumpBy(5) = 5
BoostedTally.value() (parent method) = 5
```

The last two lines come from `BoostedTally.cfc`, which declares
`extends="rust:Tally"` — a CFC subclassing a Rust-backed class. The
CFC defines `bumpBy(n)` on top of the parent's `bump()`, and unhandled
calls (`value()`) fall through to the Rust parent automatically.

The first build is slow (cargo compiles rustcfml-cli + all its dependencies
into a brand-new binary, ~2 minutes cold). Subsequent rebuilds use cargo's
incremental cache under `.rustcfml-cocktail/target/` and finish in a few
seconds.

## The contract

Every native module is a regular Rust crate whose `src/lib.rs` exposes one
function:

```rust
pub fn register(vm: &mut rustcfml_cli::Vm) {
    vm.register_native_fn("myFn", my_fn);
    vm.register_native_class("MyClass", my_class_new);
    // QoQ functions: callable as BIFs AND inside Query-of-Queries SQL.
    vm.register_native_qoq_fn("myHash", my_hash, rustcfml_cli::QoQFnKind::Scalar);
    vm.register_native_qoq_fn("myAgg",  my_agg,  rustcfml_cli::QoQFnKind::Aggregate);
}
```

`--build` discovers each module under `native/`, reads its
`[package].name`, generates a Cargo workspace that path-deps on every
module + `rustcfml-cli`, and synthesises a `main.rs` that chains every
module's `register(vm)` call inside `rustcfml_cli::run_with_registrar(...)`.

Module `Cargo.toml` must declare `rustcfml-cli` as a dependency — this is
where you get `Vm`, `Value`, `CfmlNative`, `CfmlError`, `CfmlResult`. While
RustCFML is pre-1.0 and `rustcfml-cli` isn't on crates.io yet, this has to
be a path dep pointing at your RustCFML checkout. The example above uses a
relative path because it lives inside the repo.

## Requirements

- `cargo` / `rustc` on PATH (the same toolchain you use for any Rust work).
  `--build` errors cleanly with a rustup.rs install link if missing.
- A RustCFML source checkout. The cocktail build needs `rustcfml-cli` as a
  path dep; it auto-detects the checkout via `CARGO_MANIFEST_DIR` baked
  into the running `rustcfml` binary, or you can override with
  `RUSTCFML_SOURCE=/path/to/RustCFML`.

Plain CFML apps that don't have a `native/` directory continue through the
original bundling path and require no toolchain at all.
