# Architecture

[← Back to README](../README.md)

RustCFML compiles CFML source through a pipeline of focused stages, inspired by [RustPython](https://github.com/RustPython/RustPython).

## Compilation pipeline

```plaintext
CFML Source (.cfm / .cfc)
    → Tag Preprocessor → CFScript
    → Lexer → Tokens
    → Parser → AST
    → Compiler → Bytecode
    → VM → Output
```

1. **Tag Preprocessor** — converts `<cfset x=1>`-style tags into CFScript (`x = 1;`). All CFML tags become function calls or script constructs, so the parser only ever sees CFScript. Body tags extract their content via closing-tag matching.
2. **Lexer** — tokenizes the CFScript source.
3. **Parser** — a recursive-descent parser producing AST nodes.
4. **Compiler (codegen)** — walks the AST and emits stack-based bytecode instructions.
5. **VM** — a stack-based bytecode execution engine. The main loop processes bytecode ops, manages scopes, and collects output into a buffer.

## Crate layout

```plaintext
crates/
├── cfml-common/     # Shared types: CfmlValue, CfmlError, Position
├── cfml-compiler/   # Lexer, Parser, AST, Tag Preprocessor
├── cfml-codegen/    # Bytecode compiler (AST → BytecodeOp)
├── cfml-vm/         # Stack-based bytecode VM
├── cfml-stdlib/     # 400+ built-in functions (feature-gated subsystems)
├── cli/             # CLI entry point + Axum web server
└── wasm/            # WebAssembly target (thin wrapper)
```

## Key design points

- **Case-insensitive** identifiers, function names, and scope keys, matching CFML.
- **`CfmlValue`** is the core value enum (`Null`, `Boolean`, `Int`, `Double`, `String`, `Array`, `Struct`, `Query`, `Function`, `Component`, `Binary`, …). Arrays and structs are **reference types** with Lucee semantics — aliases share mutations.
- **Ordered maps** (`IndexMap`) back structs and scopes so key order is preserved.
- **1-based arrays**, converted to 0-based for the underlying Rust `Vec`.
- **Output buffering** — output is collected in a buffer, with a stack of saved buffers for nested capture (`cfsavecontent`, `cfsilent`, `cfthread`).
- **Scope resolution order**: `local` → `arguments` → `thread` → `variables` → `cgi` → `url` → `form` → `cookie` → `request` → `application` → `server` → `session`. An explicit scope prefix bypasses the search chain.

For contributing changes to the engine, see **[Testing](testing.md)** and the project [CLAUDE.md](../CLAUDE.md) developer guide.
