# Embedding in Rust

[← Back to README](../README.md)

You can embed the RustCFML engine directly in your own Rust application — compile CFML source to bytecode, run it on the VM, and read the output buffer.

```rust
use cfml_codegen::compiler::CfmlCompiler;
use cfml_compiler::parser::Parser;
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::CfmlVirtualMachine;

let source = r#"writeOutput("Hello from Rust!");"#;
let ast = Parser::new(source.to_string()).parse().unwrap();
let program = CfmlCompiler::new().compile(ast);
let mut vm = CfmlVirtualMachine::new(program);
for (name, value) in get_builtins() { vm.globals.insert(name, value); }
for (name, func) in get_builtin_functions() { vm.builtins.insert(name, func); }
vm.execute().unwrap();
println!("{}", vm.output_buffer);
```

The pipeline mirrors the architecture: **Parser → Compiler → VM**. See **[Architecture](architecture.md)** for how the crates fit together.

For extending the engine with first-class built-ins and classes written in Rust, see **[Native Modules](native-modules.md)**.
