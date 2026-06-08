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

/// Same setup as `run` but also returns the count of OSR-compiled loop bodies
/// — used to confirm OSR specifically fired (not just whole-fn JIT).
fn run_with_osr(src: &str) -> (String, usize, usize) {
    std::env::set_var("RUSTCFML_JIT_THRESHOLD", "1");
    std::env::remove_var("RUSTCFML_JIT");
    let mut vm = CfmlVirtualMachine::new(compile(src));
    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }
    vm.execute().expect("execute");
    (
        vm.get_output().trim().to_string(),
        vm.jit_compiled_count(),
        vm.osr_compiled_count(),
    )
}

#[test]
fn jit_bit_twiddling_builtins_match_interpreter() {
    // Exercises bitAnd / bitOr / bitXor / bitNot / bitShln / bitShrn inside a
    // hot loop. Tests bit-level parity vs the interpreter — important
    // because the interpreter does the 32-bit truncation dance on bitNot
    // (Java parity) that Rust's `!` on i64 does NOT do natively.
    let src = r#"
        function kernel(n) {
            var t = 0;
            for (var i = 1; i <= n; i++) {
                t = t + bitAnd(i, 7) + bitOr(i, 16) + bitXor(i, 42)
                      + bitShln(1, i % 5) + bitShrn(i * 100, 2)
                      + bitNot(i);
            }
            return t;
        }
        for (k = 1; k <= 120; k++) { x = kernel(100); }
        writeOutput(x);
    "#;
    let oracle = run_interpreter(src);
    let (out, jit, _osr) = run_with_osr(src);
    assert_eq!(out, oracle, "bit builtins JIT output must match interpreter");
    assert!(jit >= 1, "kernel() should have been whole-fn-JIT-compiled");
}

#[test]
fn jit_extended_pure_math_builtins_match_interpreter() {
    // Exercises the v0.79.0 widened builtin allowlist: floor / ceiling /
    // round / sgn / fix (Numeric→Int) and sqr / exp / log / log10 / sin /
    // cos / tan / asin / acos / atan (Numeric→Float). All inside a hot
    // loop so OSR / whole-fn JIT compiles the kernel and the shims fire
    // from native code.
    let src = r#"
        function kernel(n) {
            var t = 0.0;
            for (var i = 1; i <= n; i++) {
                t = t + sqr(i) + log(i + 1) + sin(i / 10.0) + cos(i / 10.0)
                      + floor(i / 3.0) + ceiling(i / 7.0) + round(i / 4.0)
                      + sgn(i - 50) + fix(i / 11.0)
                      + exp(i / 100.0) + log10(i + 1) + tan(i / 100.0)
                      + pow(1.001, i % 10);
            }
            return t;
        }
        // Call once to warm up the JIT, then assert a stable known-good
        // value cross-checked against the interpreter oracle.
        for (k = 1; k <= 120; k++) { x = kernel(60); }
        writeOutput(x);
    "#;
    let oracle = run_interpreter(src);
    let (out, jit, _osr) = run_with_osr(src);
    assert_eq!(out, oracle, "extended builtins JIT output must match interpreter");
    assert!(jit >= 1, "kernel() should have been whole-fn-JIT-compiled");
}

#[test]
fn osr_do_while_loop_matches_interpreter() {
    // do { body } while (cond) — terminates in `JumpIfTrue(loop_start)`.
    let src = r#"
        sum = 0;
        i = 1;
        do {
            sum = sum + i;
            i = i + 1;
        } while (i <= 1000);
        writeOutput(sum);
    "#;
    let oracle = run_interpreter(src);
    let (out, _jit, osr) = run_with_osr(src);
    assert_eq!(out, oracle);
    assert_eq!(out, "500500");
    assert!(osr >= 1, "expected do-while to OSR-compile, got osr={osr}");
}

#[test]
fn osr_while_loop_in_main_matches_interpreter() {
    // While-loop in __main__ — terminates in `Jump(loop_start)` rather than
    // a fused ForLoopStep. Pre-OSR-Phase-2 this never JIT'd; post-Phase-2
    // the Jump back-edge triggers OSR analysis + compilation just like
    // ForLoopStep does for counted loops.
    let src = r#"
        sum = 0;
        i = 1;
        while (i <= 1000) {
            sum = sum + i;
            i = i + 1;
        }
        writeOutput(sum);
    "#;
    // 1+2+...+1000 = 500500
    let oracle = run_interpreter(src);
    let (out, _jit, osr) = run_with_osr(src);
    assert_eq!(out, oracle, "while-loop OSR output must equal the interpreter");
    assert_eq!(out, "500500");
    assert!(osr >= 1, "expected the while-loop to OSR-compile, got osr={osr}");
}

#[test]
fn osr_simple_main_loop_matches_interpreter() {
    // Simplest possible hot __main__ loop. Threshold=1 means OSR engages on
    // the 2nd back-edge — the rest of the iterations run natively.
    let src = "sum = 0; for (i = 1; i <= 10; i++) { sum = sum + i; } writeOutput(sum);";
    let oracle = run_interpreter(src);
    let (out, _jit, osr) = run_with_osr(src);
    assert_eq!(out, oracle);
    assert!(osr >= 1, "expected at least one loop to OSR-compile, got {osr}");
}

#[test]
fn osr_nested_inner_loop_exits_to_outer_body_not_writeback() {
    // Regression: in an early draft of compile_loop the INNER ForLoopStep's
    // matched-false branch jumped to the OUTER writeback block, exiting the
    // entire OSR'd region after one inner iteration. Correct behaviour is to
    // fall through to the next basic block (the outer-body code after the
    // inner loop), so only the OUTERMOST ForLoopStep exits via writeback.
    let src = r#"
        acc = 0;
        for (k = 1; k <= 4; k++) {
            t = 0;
            for (i = 1; i <= 3; i++) { t = t + i; }
            acc = acc + t;
        }
        writeOutput(acc);
    "#;
    // inner = 1+2+3 = 6; acc = 4 * 6 = 24
    let oracle = run_interpreter(src);
    let (out, _jit, _osr) = run_with_osr(src);
    assert_eq!(out, oracle);
}

#[test]
fn hot_main_loop_osrs_and_matches_interpreter() {
    // The hot loop lives at __main__ scope, which the whole-fn JIT rejects
    // outright. Without OSR none of this runs natively; with OSR the body of
    // the outer loop compiles and the interpreter only runs each iteration's
    // first step before handing over.
    let src = r#"
        acc = 0;
        for (k = 1; k <= 200; k++) {
            t = 0;
            for (i = 1; i <= 50; i++) { t = t + abs(i - 25); }
            acc = acc + t;
        }
        writeOutput(acc);
    "#;
    let oracle = run_interpreter(src);
    let (out, _jit, osr) = run_with_osr(src);
    assert_eq!(out, oracle, "OSR'd loop output must equal the interpreter");
    assert!(osr >= 1, "expected at least one loop to be OSR-compiled, got {osr}");
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

// ── UDF→UDF direct call tests (Phase 1) ──────────────────────────────────────
//
// These cover the new path where a JIT'd function calls another user-defined
// function. The dispatcher (`cfml_call_jit_udf`) consults the engine's cache
// at runtime and either invokes the compiled callee or bails to the
// interpreter. All three cases below verify result-correctness against the
// interpreter oracle; the compile-counter check confirms at least the leaf
// callee actually JIT'd (the caller's compile depends on the callee being
// in cache, which only happens after warm-up).

#[test]
fn udf_to_udf_leaf_call_jits_and_matches_interpreter() {
    // Two-level call chain: `outer(n)` calls `helper(n)` in its hot loop. On
    // a warm-up run the leaf `helper` JITs first; subsequent calls then let
    // `outer` JIT too via direct dispatch.
    let src = r#"
        function helper(x) { return x * x + 1; }
        function outer(n) {
            var s = 0;
            for (var i = 1; i <= n; i++) { s = s + helper(i); }
            return s;
        }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & outer(20) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "UDF→UDF JIT output must match the interpreter");
    assert!(
        compiled >= 1,
        "expected at least the leaf helper to be JIT-compiled, got {compiled}"
    );
}

#[test]
fn udf_self_recursion_jits_and_matches_interpreter() {
    // The canonical motivating case: fib() self-recurses. The Phase-1
    // resolver synthesises a self-call binding so the analyser admits the
    // body; cache insertion before first run means the libcall finds it on
    // every recursion.
    //
    // n is kept modest (fib(7) = 13, max recursion depth 7) so the test
    // fits inside a 2MB debug-mode thread stack — unoptimised Cranelift
    // emits very large per-frame allocations, and fib(10) overflows. The
    // recursion *path* is what's being verified, not depth.
    let src = r#"
        function fib(n) {
            if (n < 2) { return n; }
            return fib(n - 1) + fib(n - 2);
        }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & fib(7) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "self-recursive JIT output must match the interpreter");
    assert!(compiled >= 1, "expected fib() to be JIT-compiled, got {compiled}");
}

#[test]
fn udf_call_with_double_arg_jits_via_signature_match() {
    // Caller passes a Double arg through to the callee. Both specializations
    // must compile with the same Float-arg signature for the libcall lookup
    // to hit.
    let src = r#"
        function scale(x) { return x * 2.5 + 1; }
        function chain(x) { return scale(x) + scale(x); }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & chain(3.0) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "Double-arg UDF→UDF output must match the interpreter");
    assert!(compiled >= 1, "expected at least scale() to be JIT-compiled");
}
