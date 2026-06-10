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
fn jit_increment_decrement_and_bitmask_builtins_match_interpreter() {
    // Exercises the v0.85.0 additions: incrementValue / decrementValue
    // (both Int and Float overloads) and the 3/4-arg bitMaskRead/Set/Clear
    // shims, all from a hot kernel that the JIT compiles whole-function.
    let src = r#"
        function intKernel(n) {
            var t = 0;
            for (var i = 1; i <= n; i++) {
                t = t + incrementValue(i) - decrementValue(i)
                      + bitMaskRead(i, 1, 3)
                      + bitMaskSet(i, 5, 2, 3)
                      + bitMaskClear(i, 0, 2);
            }
            return t;
        }
        function floatKernel(n) {
            var f = 0.0;
            for (var i = 1; i <= n; i++) {
                f = f + incrementValue(i / 10.0) - decrementValue(i / 10.0);
            }
            return f;
        }
        for (k = 1; k <= 120; k++) {
            x = intKernel(80);
            y = floatKernel(80);
        }
        writeOutput(x & ":" & y);
    "#;
    let oracle = run_interpreter(src);
    let (out, jit, _osr) = run_with_osr(src);
    assert_eq!(out, oracle, "new builtins JIT output must match interpreter");
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
//  bare-name calls to the canonical builtin passThroughper in `vm.globals` before any
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

// ── v0.89.0 — Boxed scalar in/out at the ABI ─────────────────────────────────
//
// Pure pass-through functions admit a Boxed param + Boxed return: the body
// loads/stores the tagged value through slots without ever operating on it.
// The interpreter oracle must accept the same call shape (a CfmlValue::String
// passed straight through) and yield byte-identical output.

#[test]
fn boxed_pass_through_string_arg_jits_and_matches_interpreter() {
    // `identity(s)` is loaded with a String, hot-warmed past threshold so the
    // JIT engages with a Boxed-param specialization, then must echo the
    // string back through the tagged-pointer ABI.
    let src = r#"
        function identity(s) { return s; }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & identity("hello-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "Boxed-pass-through output must match the interpreter");
    assert!(
        compiled >= 1,
        "expected identity() to be JIT-compiled with a Boxed signature, got {compiled}"
    );
}

#[test]
fn boxed_pass_through_via_intermediate_local_jits_and_matches() {
    // `relay(s)` has the value flow Param → Local → Return. The slot-kind
    // fixpoint must upgrade the non-param local to Boxed via store-flow
    // from the Boxed param.
    let src = r#"
        function relay(s) { var t = s; return t; }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & relay("relay-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle);
    assert!(compiled >= 1, "expected relay() to be JIT-compiled, got {compiled}");
}

#[test]
fn boxed_pass_through_mixed_int_and_boxed_args() {
    // `pick(_, s)` has a Boxed param alongside an Int param. The Int param
    // is read by a discardable comparison (admitted via pop_value), and the
    // Boxed `s` is passed through unchanged. Confirms the 2-bit-per-arg sig
    // encoding distinguishes (Int, Boxed) from (Int, Int) / (Int, Float).
    let src = r#"
        function pick(n, s) { var t = s; return t; }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & pick(k, "mixed-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle);
    assert!(compiled >= 1, "expected pick() to be JIT-compiled, got {compiled}");
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

// ── v0.90.0 — Boxed mid-body operations ───────────────────────────────────

#[test]
fn string_literal_pass_through_jits() {
    // `function f() { return "x"; }` admits with Kind::Boxed return now.
    let src = r#"
        function f() { return "x"; }
        for (k = 1; k <= 60; k++) { v = f(); }
        writeOutput(v);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle);
    assert!(compiled >= 1, "expected f() to JIT, got {compiled}");
}

#[test]
fn boxed_concat_in_jitted_udf_matches_interpreter() {
    // The signature for `build` is (Boxed, Int) — admissible since v0.89.0.
    // The body uses String literal + Concat (mixed Boxed + Int + Boxed)
    // and a Boxed loop accumulator, which are the v0.90.0 additions.
    let src = r#"
        function build(prefix, n) {
            var s = prefix;
            for (var i = 1; i <= n; i++) { s = s & "-" & i; }
            return s;
        }
        out = "";
        for (k = 1; k <= 60; k++) { out = out & build("row" & k, 5) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "Boxed-concat UDF must produce identical output");
    assert!(compiled >= 1, "expected build() to JIT, got {compiled}");
}

// ── v0.90.1 — UDF→UDF dispatch carrying Boxed args / Boxed returns ──────────
//
// Before v0.90.1 the UDF resolver refused any callee whose specialization
// took Boxed args or returned Boxed — the binding was unrepresentable in
// the dispatcher (ret_float: bool) and there was no IR-level Boxed pipeline
// to feed the result into. v0.90.0 lit the IR pipeline (String/Concat +
// arena). v0.90.1 lifts the resolver gate and grows expected_ret_float into
// a tri-state expected_ret_kind (0=Int / 1=Float / 2=Boxed), letting a
// JIT'd caller invoke a JIT'd Boxed-returning UDF.

#[test]
fn jit_caller_invokes_boxed_returning_udf_and_matches_interpreter() {
    // `buildLine(prefix, n) → Boxed` is called from a JIT-eligible non-main
    // passThroughper `joinMany(label, count) → Boxed`. The passThroughper's Call site
    // receives a Boxed prefix arg (`"row" & i`) and consumes buildLine's
    // Boxed return via another Concat — both newly admitted in v0.90.1.
    let src = r#"
        function buildLine(prefix, n) {
            var s = prefix;
            for (var i = 1; i <= n; i++) { s = s & "-" & i; }
            return s;
        }
        function joinMany(label, count) {
            var out = label;
            for (var i = 1; i <= count; i++) {
                out = out & buildLine("row" & i, 3) & ";";
            }
            return out;
        }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & joinMany("L", 4) & "|"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "Boxed UDF→UDF dispatch output must match the interpreter");
    assert!(
        compiled >= 2,
        "expected both buildLine and joinMany to JIT (Boxed-arg + Boxed-ret dispatch), got {compiled}"
    );
}

#[test]
fn jit_caller_threads_boxed_arg_through_to_jitted_callee() {
    // A Boxed value crosses TWO UDF call boundaries:
    //   passThrough(s) → echo(s) → s
    // Both functions specialise on Boxed args + Boxed returns. The runtime
    // tagged-pointer threads from caller's slot → dispatcher's i64 arg →
    // callee's slot → return → caller's stack as Boxed. No IR-level box
    // operations beyond the dispatch itself.
    let src = r#"
        function echo(s) { return s; }
        function passThrough(s) {
            var t = echo(s);
            return t;
        }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & passThrough("v-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(
        out, oracle,
        "Boxed arg threaded across two JIT'd UDFs must match the interpreter"
    );
    assert!(
        compiled >= 2,
        "expected both echo and passThrough to JIT, got {compiled}"
    );
}

#[test]
fn boxed_concat_with_float_operand_matches_interpreter() {
    // Concat of String + Float — the box_float shim should fire on the
    // Float side and the concat result must stringify identically to
    // the interpreter (`d` formatted by `CfmlValue::as_string`).
    let src = r#"
        function fmt(label, d) { return label & "=" & d; }
        out = "";
        for (k = 1; k <= 60; k++) { out = out & fmt("x", 2.5) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "Boxed-concat with Float operand must match");
    assert!(compiled >= 1, "expected fmt() to JIT, got {compiled}");
}

// ── v0.91.0 — OSR Boxed slots + UDF dispatch ────────────────────────────────
//
// The whole-function JIT can already compile a UDF with Boxed slots + UDF
// calls (since v0.90.1). v0.91.0 adds the same capabilities to OSR so a hot
// outer loop in __main__ (or in any function whose whole body is otherwise
// non-admissible) can be compiled to native code.

#[test]
fn osr_boxed_concat_loop_in_main_matches_interpreter() {
    // `__main__` body contains a non-allowlist top-level call (writeOutput)
    // which the whole-fn analyser rejects. The for-loop region by itself,
    // however, contains only String/Concat/StoreLocal ops on a Boxed slot —
    // exactly the v0.91.0 OSR admission surface.
    let src = r#"
        out = "";
        for (k = 1; k <= 200; k++) { out = out & "row" & k & ";"; }
        writeOutput(len(out));
    "#;
    let oracle = run_interpreter(src);
    let (out, _fn_compiled, osr_compiled) = run_with_osr(src);
    assert_eq!(out, oracle, "OSR Boxed-concat loop must match interpreter");
    assert!(osr_compiled >= 1, "expected OSR to fire on the Boxed concat loop, got {osr_compiled}");
}

#[test]
fn osr_calls_jitted_udf_from_outer_loop_in_main() {
    // The headline v0.91.0 unlock: `__main__`'s outer loop invokes a
    // JIT-compiled UDF (`buildLine`) that takes Boxed + Int args and returns
    // Boxed. The outer loop region is now OSR-eligible because OSR admits
    // UDF callsites (Phase-2 dispatcher) and Boxed slots.
    //
    // The body intentionally does meaningful work *between* the UDF calls
    // (two Concats) so it clears the v0.91.1 OSR-UDF admission heuristic —
    // a thin "loop-of-UDF-call-wrappers" body would (correctly) reject as
    // a libcall-overhead net pessimization.
    let src = r#"
        function buildLine(prefix, n) {
            var s = prefix;
            for (var i = 1; i <= n; i++) { s = s & "-" & i; }
            return s;
        }
        total = "";
        for (k = 1; k <= 200; k++) {
            total = total & buildLine("row" & k, 3) & ";";
        }
        writeOutput(total);
    "#;
    let oracle = run_interpreter(src);
    let (out, fn_compiled, osr_compiled) = run_with_osr(src);
    assert_eq!(out, oracle, "OSR-with-UDF-dispatch output must match interpreter");
    assert!(fn_compiled >= 1, "expected buildLine to JIT, got fn_compiled={fn_compiled}");
    assert!(
        osr_compiled >= 1,
        "expected outer loop in __main__ to OSR-compile (UDF dispatch + Boxed slot), got osr_compiled={osr_compiled}"
    );
}

#[test]
fn osr_rejects_thin_udf_wrapper_loop() {
    // v0.91.1 regression guard. A `for-loop { x = udf(arg) }` body — i.e.
    // a UDF-call wrapper with no real work between calls — must NOT be
    // OSR-compiled, because the UDF dispatcher libcall adds ~100ns/call
    // overhead vs the interpreter's already-cached Call→try_call path.
    // Surfaced empirically as a 6.2pp slowdown on udf_call_graph.cfm
    // between v0.90.1 and v0.91.0; v0.91.1's analyser heuristic removes it.
    let src = r#"
        function id(n) { return n; }
        total = 0;
        for (k = 1; k <= 200; k++) { total = id(k); }
        writeOutput(total);
    "#;
    let oracle = run_interpreter(src);
    let (out, _fn_compiled, osr_compiled) = run_with_osr(src);
    assert_eq!(out, oracle, "interpreter parity for thin UDF-wrapper loop");
    assert_eq!(
        osr_compiled, 0,
        "thin UDF-wrapper outer loop must be rejected by the admission heuristic (got osr_compiled={osr_compiled})"
    );
}

// ── v0.92.0 — Boxed-argument string shims (len / uCase / lCase / trim…) ──

#[test]
fn ucase_lcase_concat_in_jitted_udf_matches_interpreter() {
    // `tag(s)` takes a Boxed param, calls uCase + lCase (both Boxed → Boxed
    // shims), and concatenates. Whole-function JIT must engage and produce
    // byte-identical output to the interpreter oracle.
    let src = r#"
        function tag(s) { return uCase(s) & ":" & lCase(s); }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & tag("AbCd-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "uCase/lCase Boxed shim output must match the interpreter");
    assert!(compiled >= 1, "expected tag() to be JIT-compiled, got {compiled}");
}

#[test]
fn len_of_boxed_string_arg_returns_int_in_jit() {
    // `sz(s)` calls len(s) which returns Int through a Boxed-arg shim. The
    // value flows back into arithmetic — proving the Int return kind plumbs
    // back into the standard numeric lattice from a Boxed callsite.
    let src = r#"
        function sz(s) { return len(s) * 2 + 1; }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & sz("len-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "len() Boxed shim output must match the interpreter");
    assert!(compiled >= 1, "expected sz() to be JIT-compiled, got {compiled}");
}

#[test]
fn trim_family_in_jitted_udf_matches_interpreter() {
    // trim / ltrim / rtrim each accept Boxed → return Boxed. Compose them
    // through a Concat to confirm a chain of Boxed-producing shims survives
    // round-trip through the arena.
    let src = r#"
        function clean(s) { return trim(s) & "|" & ltrim(s) & "|" & rtrim(s); }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & clean("  AbC-#k#  ") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "trim-family Boxed shim output must match the interpreter");
    assert!(compiled >= 1, "expected clean() to be JIT-compiled, got {compiled}");
}

#[test]
fn reverse_in_jitted_udf_matches_interpreter() {
    // reverse(string) — Boxed → Boxed. Confirms chars().rev() round-trips
    // through the arena identically to the interpreter.
    let src = r#"
        function rev(s) { return reverse(s); }
        out = "";
        for (k = 1; k <= 80; k++) { out = out & rev("abc-#k#") & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "reverse Boxed shim output must match the interpreter");
    assert!(compiled >= 1, "expected rev() to be JIT-compiled, got {compiled}");
}

#[test]
fn asc_returns_int_from_boxed_arg_in_jit() {
    // asc(s) is the second Boxed→Int shim after len(). Result feeds back
    // into arithmetic to prove the Int return kind plumbs correctly.
    let src = r##"
        function code(s) { return asc(s) + 1; }
        out = "";
        for (k = 1; k <= 80; k++) {
            ch = chr(64 + (k % 26));
            out = out & code(ch & "x") & ";";
        }
        writeOutput(out);
    "##;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "asc Boxed shim output must match the interpreter");
    assert!(compiled >= 1, "expected code() to be JIT-compiled, got {compiled}");
}

#[test]
fn array_len_shim_matches_interpreter() {
    // v0.99.3 bail plumbing — arrayLen(Boxed) → Int. Pure-array path is
    // the happy case (no bail); struct/numeric fall-throughs covered by
    // the interpreter oracle.
    let src = r#"
        function sz(a) { return arrayLen(a) * 2 + 1; }
        out = "";
        for (k = 1; k <= 80; k++) {
            arr = [1, 2, 3, k, k+1, k+2];
            out = out & sz(arr) & ";";
        }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "arrayLen Boxed shim must match interpreter");
    assert!(compiled >= 1, "expected sz() to be JIT-compiled, got {compiled}");
}

#[test]
fn array_len_bails_on_query_column_and_matches_interpreter() {
    // v0.99.3 — arrayLen on a QueryColumn THROWS in Lucee/RustCFML. The
    // JIT'd shim sets *bail = 1; the engine then re-interprets the call,
    // and the interpreter throws the same `Can't cast` runtime error,
    // which the cftry/cfcatch grabs. The error message must match exactly.
    let src = r#"
        function probe(a) { return arrayLen(a); }
        q = queryNew("name", "varchar");
        queryAddRow(q, {name: "alice"});
        out = "";
        for (k = 1; k <= 80; k++) {
            try {
                v = probe(q.name);  // QueryColumn — must throw
                out = out & "ok:" & v & ";";
            } catch (any e) {
                out = out & "err:" & e.message & ";";
            }
        }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, _compiled) = run(src);
    assert_eq!(
        out, oracle,
        "arrayLen(QueryColumn) bail must re-interpret to same error as interpreter"
    );
    // We deliberately don't assert compiled>=1 — under JIT the shim sets
    // *bail every call, so the function may be evicted from cache. The
    // critical guarantee is output parity with the interpreter.
}

#[test]
fn struct_key_list_shim_matches_interpreter() {
    // v0.99.3 — structKeyList(Boxed) → Boxed. Infallible: non-struct
    // inputs return empty string. Default delimiter "," (1-arg form).
    let src = r##"
        function keys(s) { return "[" & structKeyList(s) & "]"; }
        out = "";
        for (k = 1; k <= 60; k++) {
            st = {a: k, b: k+1, c: k+2};
            out = out & keys(st) & ";";
        }
        writeOutput(out);
    "##;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "structKeyList Boxed shim must match interpreter");
    assert!(compiled >= 1, "expected keys() to be JIT-compiled, got {compiled}");
}

#[test]
fn url_and_js_string_format_shims_match_interpreter() {
    // v0.99.2 single-arg Boxed→Boxed shims: urlEncodedFormat / urlDecode /
    // jsStringFormat. Chain through Concat to exercise all three.
    let src = r##"
        function fmt(s) {
            return urlEncodedFormat(s) & "|" & jsStringFormat(s) & "|"
                & urlDecode(urlEncodedFormat(s));
        }
        out = "";
        for (k = 1; k <= 60; k++) {
            out = out & fmt("a b+c/d=#k# 'q' ""r"" \t") & ";";
        }
        writeOutput(out);
    "##;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "url / jsStringFormat shims must match interpreter");
    assert!(compiled >= 1, "expected fmt() to be JIT-compiled, got {compiled}");
}

#[test]
fn left_right_mid_shims_match_interpreter() {
    // v0.99.2 (Boxed, Int[, Int]) → Boxed shims. Confirms the multi-arg
    // Boxed+Int ABI path: arg 0 crosses as tagged ptr (to_i64 pass-through),
    // args 1+ cross as int (to_i64 no-op).
    let src = r#"
        function slice(s, i) {
            return left(s, i) & "|" & right(s, i) & "|" & mid(s, i, 3);
        }
        out = "";
        for (k = 1; k <= 60; k++) {
            out = out & slice("abcdefghij-#k#", (k % 5) + 1) & ";";
        }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "left/right/mid Boxed shims must match interpreter");
    assert!(compiled >= 1, "expected slice() to be JIT-compiled, got {compiled}");
}

#[test]
fn find_and_replace_shims_match_interpreter() {
    // v0.99.2 (Boxed, Boxed) → Int (find/findNoCase) and
    // (Boxed, Boxed, Boxed) → Boxed (replace/replaceNoCase). The find result
    // flows back into arithmetic, proving the Int return kind plumbs out of a
    // multi-Boxed-arg callsite.
    let src = r#"
        function check(s, needle, with) {
            var pos = find(needle, s) + findNoCase(needle, s);
            return pos & ":" & replace(s, needle, with) & "|"
                & replaceNoCase(s, needle, with);
        }
        out = "";
        for (k = 1; k <= 60; k++) {
            out = out & check("AbCdAbCd-#k#", "ab", "X") & ";";
        }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "find/replace Boxed shims must match interpreter");
    assert!(compiled >= 1, "expected check() to be JIT-compiled, got {compiled}");
}

#[test]
fn repeat_string_shim_matches_interpreter() {
    // (Boxed, Int) → Boxed; output grows with k so we exercise the arena's
    // string-box arena allocation under volume too.
    let src = r#"
        function band(s, n) { return repeatString(s, n) & "."; }
        out = "";
        for (k = 1; k <= 30; k++) { out = out & band("ab-#k#", k % 4) & ";"; }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "repeatString Boxed shim must match interpreter");
    assert!(compiled >= 1, "expected band() to be JIT-compiled, got {compiled}");
}

#[test]
fn html_format_shims_in_jitted_udf_match_interpreter() {
    // htmlEditFormat / htmlCodeFormat / encodeForHtml / stripCr — chained
    // through a Concat. Confirms the entity-escape semantics match interp
    // byte-for-byte.
    let src = r#"
        function fmt(s) {
            return htmlEditFormat(s) & "|" & htmlCodeFormat(s) & "|"
                & encodeForHtml(s) & "|" & stripCr(s);
        }
        out = "";
        for (k = 1; k <= 60; k++) {
            out = out & fmt("a<b>&""c'/d#chr(13)#e-#k#") & ";";
        }
        writeOutput(out);
    "#;
    let oracle = run_interpreter(src);
    let (out, compiled) = run(src);
    assert_eq!(out, oracle, "html-format Boxed shims must match the interpreter");
    assert!(compiled >= 1, "expected fmt() to be JIT-compiled, got {compiled}");
}
