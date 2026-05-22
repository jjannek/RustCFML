# Markdown → HTML native module

A more realistic native-module example than the Counter demo. Wraps the
[`pulldown_cmark`](https://crates.io/crates/pulldown-cmark) Rust crate so
CFML can render CommonMark to HTML via a single BIF:

```cfml
html = rustMarkdown(source);
stats = rustMarkdownStats(source);  // { chars, words, lines, code_blocks }
```

`rustMarkdown` returns a string; `rustMarkdownStats` returns a CFML struct
— shows that a native BIF can hand back structured values, not just
primitives.

## Build & run

```bash
cargo run --release -- --build examples/native_markdown \
    -o /tmp/markdown_demo \
    --mode cli \
    --entry main.cfm

/tmp/markdown_demo
```

Expected output starts:

```
=== Source ===

# RustCFML + Rust
...

=== Rendered HTML ===
<h1>RustCFML + Rust</h1>
<p>This is <strong>bold</strong>, this is <em>italic</em>, and this is <code>code</code>.</p>
<ul>
<li>one</li>
...
```

## Why this is interesting

Pure CFML could never do this in any sane amount of code — markdown parsing
involves character-level state machines, escape rules, nested-block
tracking, and so on. The native module hides all of that behind one
function call, while the rest of the application (templating, IO, glue) can
stay in idiomatic CFML.

The binary is ~15 MB self-contained — no system markdown library, no JRE,
nothing to install on the target machine.

## File layout

```
native_markdown/
├── main.cfm                       — CFML caller
└── native/
    └── md/
        ├── Cargo.toml             — declares pulldown-cmark dep
        └── src/lib.rs             — register(vm) + the two BIFs
```
