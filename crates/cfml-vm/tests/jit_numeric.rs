//! End-to-end JIT verification through the public VM API.
//!
//! Only built with `--features jit`. These tests run real, compiler-emitted
//! bytecode through `execute_function_with_args` (so they exercise the actual
//! dispatch hook, hotness counter, cache, and trampoline — not `Backend`
//! directly), with the hotness threshold forced to 1 so the JIT engages on the
//! second call. The interpreter is the trusted oracle: matching the closed-form
//! results with the JIT active proves the whole pipeline is correct.
#![cfg(feature = "jit")]

use cfml_codegen::{compiler::CfmlCompiler, BytecodeProgram};
use cfml_compiler::parser::Parser;
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::CfmlVirtualMachine;

fn compile(src: &str) -> BytecodeProgram {
    let ast = Parser::new(src.to_string()).parse().expect("parse");
    CfmlCompiler::new().compile(ast)
}

fn run(src: &str) -> (String, usize) {
    // Compile the native body on the 2nd invocation of a hot function.
    std::env::set_var("RUSTCFML_JIT_THRESHOLD", "1");
    std::env::remove_var("RUSTCFML_JIT"); // ensure not force-disabled
    let mut vm = CfmlVirtualMachine::new(compile(src));
    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }
    vm.execute().expect("execute");
    (vm.get_output().trim().to_string(), vm.jit_compiled_count())
}

/// Run `src` with the JIT force-disabled (`RUSTCFML_JIT=0`) — the interpreter
/// oracle. Returns the trimmed output.
fn run_interpreter(src: &str) -> String {
    std::env::set_var("RUSTCFML_JIT", "0");
    let mut vm = CfmlVirtualMachine::new(compile(src));
    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }
    vm.execute().expect("execute");
    std::env::remove_var("RUSTCFML_JIT");
    vm.get_output().trim().to_string()
}

#[test]
fn counted_loop_function_jits_and_is_correct() {
    // sumTo(100) = 5050; called 60× so the JIT engages, and the running total
    // must still be exact (303000) — proving JIT output == interpreter output.
    let src = r#"
        function sumTo(n) {
            var t = 0;
            for (var i = 1; i <= n; i++) { t = t + i; }
            return t;
        }
        total = 0;
        for (k = 1; k <= 60; k++) { total = total + sumTo(100); }
        writeOutput(total);
    "#;
    let (out, compiled) = run(src);
    assert_eq!(out, "303000", "JIT result must equal the interpreter result");
    assert!(compiled >= 1, "expected the hot function to be JIT-compiled");
}

#[test]
fn factorial_function_jits_and_is_correct() {
    let src = r#"
        function fact(n) {
            var r = 1;
            for (var i = 2; i <= n; i++) { r = r * i; }
            return r;
        }
        out = "";
        for (k = 1; k <= 30; k++) { out = out & fact(10) & ";"; }
        writeOutput(out);
    "#;
    let (out, compiled) = run(src);
    // 10! = 3628800, repeated 30×
    let expected = "3628800;".repeat(30);
    assert_eq!(out, expected);
    assert!(compiled >= 1, "expected fact() to be JIT-compiled");
}

#[test]
fn jit_result_matches_interpreter_across_inputs() {
    // A polynomial kernel over many inputs; compare against the closed form.
    let src = r#"
        function poly(a, b) { return a * b + a - b; }
        out = "";
        for (k = 0; k <= 40; k++) { out = out & poly(k, 3) & ","; }
        writeOutput(out);
    "#;
    let (out, compiled) = run(src);
    let expected: String = (0..=40).map(|k| format!("{},", k * 3 + k - 3)).collect();
    assert_eq!(out, expected);
    assert!(compiled >= 1, "expected poly() to be JIT-compiled");
}

#[test]
fn double_arg_kernel_jits_and_matches_interpreter() {
    // Pass a `Double` argument across the ABI boundary. With Option-B
    // follow-ups landed, the param slot specialises to Float and the JIT runs;
    // before, the engine bailed and the interpreter handled every call.
    let src = r#"
        function area(r) { return r * r * 3.14159; }
        out = "";
        for (k = 1; k <= 60; k++) { out = out & area(2.5) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "JIT Double-arg output must equal the interpreter");
    assert!(compiled >= 1, "expected area() to be JIT-compiled");
}

#[test]
fn pow_kernel_jits_and_matches_interpreter() {
    let src = r#"
        function p(a, b) { return a ^ b + a % 3; }
        out = "";
        for (k = 1; k <= 60; k++) { out = out & p(7, 3) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "JIT pow output must equal the interpreter");
    assert!(compiled >= 1, "expected p() to be JIT-compiled");
}

#[test]
fn float_kernel_jits_and_matches_interpreter() {
    // A Double-returning kernel (the `/` operator + a Float accumulator with an
    // Int loop counter). The interpreter (JIT off) is the oracle; the JIT-on run
    // must produce byte-identical output *and* have compiled the hot function.
    let src = r#"
        function stat(n) {
            var s = 0.0;
            for (var i = 1; i <= n; i++) { s = s + i / n; }
            return s / 2 + 0.5;
        }
        out = "";
        for (k = 1; k <= 40; k++) { out = out & stat(8) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "JIT float output must equal the interpreter");
    assert!(compiled >= 1, "expected stat() to be JIT-compiled");
}

#[test]
fn builtin_calls_jit_and_match_interpreter() {
    // Exercises Option A: JIT → native builtin calls. abs() picks the Int
    // overload (returning Int); min() / max() always return Double; nesting
    // (abs(i - 5)) inside the loop body is the realistic shape.
    let src = r#"
        function score(n) {
            var t = 0;
            for (var i = 1; i <= n; i++) { t = t + abs(i - 5); }
            return min(max(t, 1), 100);
        }
        out = "";
        for (k = 1; k <= 60; k++) { out = out & score(8) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "JIT builtin-call output must equal the interpreter");
    assert!(compiled >= 1, "expected score() to be JIT-compiled");
}

// (engine-level shadow guard is covered by the `shadow_check_short_circuits_jit`
//  unit test in `crates/cfml-vm/src/jit/mod.rs`. A full e2e shadowing test
//  isn't included here because RustCFML's `LoadGlobal` lookup order resolves
//  bare-name calls to the canonical builtin wrapper in `vm.globals` before any
//  user `function abs(){…}` or `abs = ...` reassignment — so the language
//  doesn't currently expose a path to actually shadow an Option-A builtin name
//  from CFML source. The guard remains as defence-in-depth: if/when CFML adds
//  user-overridable globals for builtin names, no JIT correctness regression
//  will follow.)
