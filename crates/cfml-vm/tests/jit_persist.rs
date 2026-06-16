//! Serve-mode cross-request JIT persistence.
//!
//! The JIT engine (compiled native code + hotness counters + Cranelift module)
//! is moved into the executing VM for a request and returned to a per-thread
//! `thread_local!` afterwards, so it survives across requests landing on the
//! same worker thread. These tests drive that round-trip directly through the
//! public `jit_take_persistent` / `jit_return_persistent` / `JitLease` API.
//!
//! Determinism: every test re-seeds the thread-local with an explicit
//! threshold-1 engine before exercising it, so test-thread reuse (cargo runs
//! tests on a shared pool) cannot leak a prior test's engine in.
#![cfg(feature = "jit")]

use cfml_codegen::{compiler::CfmlCompiler, BytecodeProgram};
use cfml_compiler::parser::Parser;
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::{CfmlVirtualMachine, JitLease};

fn compile(src: &str) -> BytecodeProgram {
    let ast = Parser::new(src.to_string()).parse().expect("parse");
    CfmlCompiler::new().compile(ast)
}

fn register(vm: &mut CfmlVirtualMachine) {
    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }
}

/// Overwrite this thread's persistent-JIT thread-local with a fresh,
/// threshold-1 engine. Done by handing a threshold-1 engine to a throwaway VM
/// and returning it — `jit_return_persistent` overwrites whatever was there.
fn seed_threshold_1_engine() {
    let mut seed = CfmlVirtualMachine::new(compile("x = 1;"));
    seed.jit_set_threshold(1);
    seed.jit_return_persistent();
}

/// A hot integer kernel: `sumTo` called in a counted loop so it compiles, with
/// a closed-form answer the interpreter agrees on. sumTo(100)=5050, ×60=303000.
const KERNEL: &str = r#"
    function sumTo(n) {
        var t = 0;
        for (var i = 1; i <= n; i++) { t = t + i; }
        return t;
    }
    total = 0;
    for (var c = 1; c <= 60; c++) { total = total + sumTo(100); }
    writeOutput(total);
"#;

#[test]
fn engine_persists_compiled_code_across_vms_on_one_thread() {
    seed_threshold_1_engine();

    // Both VMs run clones of ONE compiled program, so they share global_ids —
    // the cache key. Compiling the source twice would mint fresh ids and miss.
    let program = compile(KERNEL);

    // Request 1: adopt the persistent engine, run the kernel (compiles sumTo),
    // return the engine to the thread-local.
    let compiled_after_first;
    {
        let mut vm1 = CfmlVirtualMachine::new(program.clone());
        register(&mut vm1);
        vm1.jit_take_persistent();
        vm1.execute().expect("vm1 execute");
        assert_eq!(vm1.get_output().trim(), "303000");
        compiled_after_first = vm1.jit_compiled_count();
        assert!(
            compiled_after_first > 0,
            "expected sumTo to JIT-compile on request 1, got {compiled_after_first}"
        );
        vm1.jit_return_persistent();
    }

    // Request 2: a brand-new VM adopts the SAME persistent engine. Its compiled
    // count must already be > 0 BEFORE running anything — proving the native
    // code survived the round-trip (and that moving the engine between VMs did
    // not invalidate the compiled function pointers).
    {
        let mut vm2 = CfmlVirtualMachine::new(program.clone());
        register(&mut vm2);
        vm2.jit_take_persistent();
        assert_eq!(
            vm2.jit_compiled_count(),
            compiled_after_first,
            "request 2 should inherit request 1's warmed cache, not start cold"
        );
        // And the cached body must execute to the correct result.
        vm2.execute().expect("vm2 execute");
        assert_eq!(vm2.get_output().trim(), "303000");
        vm2.jit_return_persistent();
    }
}

#[test]
fn jit_lease_returns_engine_on_panic_unwind() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // Seed a WARM engine (compiled_count > 0) so we can tell "engine returned"
    // (count stays > 0) from "engine lost and rebuilt cold" (count == 0).
    seed_threshold_1_engine();
    let program = compile(KERNEL);
    {
        let mut warm = CfmlVirtualMachine::new(program.clone());
        register(&mut warm);
        warm.jit_take_persistent();
        warm.execute().expect("warm execute");
        assert!(warm.jit_compiled_count() > 0);
        warm.jit_return_persistent();
    }

    // Panic while holding a JitLease. Its Drop must still return the (warm)
    // engine to the thread-local during unwind.
    let mut vm = CfmlVirtualMachine::new(program.clone());
    register(&mut vm);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _lease = JitLease::new(&mut vm);
        panic!("boom mid-request");
    }));
    assert!(result.is_err(), "the closure must have panicked");

    // A subsequent request inherits the warm engine — proving the lease's Drop
    // returned it despite the unwind (a lost engine would rebuild cold = 0).
    let mut next = CfmlVirtualMachine::new(program);
    register(&mut next);
    next.jit_take_persistent();
    assert!(
        next.jit_compiled_count() > 0,
        "JitLease::drop must return the warm engine to the thread-local on panic"
    );
    next.jit_return_persistent();
}
