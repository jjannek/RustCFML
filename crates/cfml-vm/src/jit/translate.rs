//! Bytecode → Cranelift IR translation and the owning JIT [`Backend`].
//!
//! The translator consumes an accepted [`Plan`] (see `analysis.rs`) and emits a
//! native function with the [`CompiledFn`](super::CompiledFn) ABI:
//! `fn(args: *const i64, nargs: i64, bail: *mut i64) -> i64`.
//!
//! Values are typed `Int` (native `i64`) or `Float` (native `f64`) per the
//! analysis' slot/return kinds; booleans live transiently as `i64` `0`/`1`.
//! Arguments arrive as `i64` (the engine bails unless every argument is a
//! `CfmlValue::Int`), so param slots are always `Int`. A `Float` return is
//! bit-cast to `i64` before the ABI return and re-interpreted as an `f64` by
//! the engine — see [`super::run_compiled`].
//!
//! Key choices (see `JIT_DESIGN.md`):
//! * **Locals are Cranelift `Variable`s** (`def_var`/`use_var`), typed `I64` or
//!   `F64` per `Plan::slot_kind`; `seal_all_blocks()` runs once at the end.
//! * **The operand stack is a compile-time `Vec<(Value, Kind)>`**, reset per
//!   block; the analysis guarantees it is empty at every block boundary.
//! * **Arithmetic matches the interpreter exactly**: `+ - *` on two ints use
//!   wrapping `iadd`/`isub`/`imul`; any float operand promotes (`fcvt_from_sint`)
//!   and uses `fadd`/`fsub`/`fmul`. `/` always produces an `f64` (`fdiv`) and
//!   **bails** (→ interpreter, which *throws*) on a zero divisor. `%`/`\` keep
//!   the existing integer bail on a zero/`INT_MIN`-overflow divisor.

use cfml_codegen::{BytecodeOp, BytecodeFunction, CmpOp};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlags, Signature, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use super::analysis::{Kind, Plan};
use super::CompiledFn;

const I64: types::Type = types::I64;
const F64: types::Type = types::F64;

/// `extern "C"` shims registered with the JIT module so emitted IR can call
/// them. Defining them in Rust (rather than relying on a libc/libm symbol) keeps
/// behaviour bit-exact with the interpreter (Rust `%` on `f64` is IEEE-754
/// `frem`, and `f64::powf` is the same primitive `to_number(.).powf(..)` uses).
extern "C" fn cfml_fmod(a: f64, b: f64) -> f64 {
    a % b
}
extern "C" fn cfml_pow(a: f64, b: f64) -> f64 {
    a.powf(b)
}

/// Owns the Cranelift module (and thus all executable memory it allocates) plus
/// a reusable `FunctionBuilderContext`.
pub struct Backend {
    module: JITModule,
    fbc: FunctionBuilderContext,
    /// Monotonic counter for unique per-function symbol names.
    func_counter: u32,
    /// Pre-declared `fn(f64, f64) -> f64` shims (`cfml_fmod`, `cfml_pow`).
    fmod_id: FuncId,
    pow_id: FuncId,
}

impl Backend {
    /// Initialise the JIT backend for the host ISA. `Err` ⇒ caller declines to
    /// enable the JIT and stays on the interpreter.
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| e.to_string())?;
        flag_builder.set("is_pic", "false").map_err(|e| e.to_string())?;
        flag_builder.set("opt_level", "speed").map_err(|e| e.to_string())?;
        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| e.to_string())?;
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        // Register our Rust shims so emitted IR can call them by name. Both are
        // `extern "C" fn(f64, f64) -> f64`; addresses live in this crate for the
        // life of the process, so the cast is safe.
        builder.symbol("cfml_fmod", cfml_fmod as *const u8);
        builder.symbol("cfml_pow", cfml_pow as *const u8);
        let mut module = JITModule::new(builder);

        // Declare them once in the module; emitted functions reference these
        // FuncIds via `declare_func_in_func` and then `call`.
        let make_ff_f = |module: &mut JITModule, name: &str| -> Result<FuncId, String> {
            let mut sig = Signature::new(module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(F64));
            sig.params.push(AbiParam::new(F64));
            sig.returns.push(AbiParam::new(F64));
            module
                .declare_function(name, Linkage::Import, &sig)
                .map_err(|e| e.to_string())
        };
        let fmod_id = make_ff_f(&mut module, "cfml_fmod")?;
        let pow_id = make_ff_f(&mut module, "cfml_pow")?;

        Ok(Self {
            module,
            fbc: FunctionBuilderContext::new(),
            func_counter: 0,
            fmod_id,
            pow_id,
        })
    }

    /// Compile `func` per `plan` to native code, returning a callable pointer.
    pub fn compile(&mut self, func: &BytecodeFunction, plan: &Plan) -> Result<CompiledFn, String> {
        let ptr_ty = self.module.target_config().pointer_type();
        let mut ctx = self.module.make_context();
        // Signature: (args: *const i64, nargs: i64, bail: *mut i64) -> i64
        ctx.func.signature.params.push(AbiParam::new(ptr_ty)); // args
        ctx.func.signature.params.push(AbiParam::new(I64)); // nargs (unused; checked in Rust)
        ctx.func.signature.params.push(AbiParam::new(ptr_ty)); // bail
        ctx.func.signature.returns.push(AbiParam::new(I64));

        // Import the shim FuncIds into this function's IR namespace so we can
        // emit `call`s to them. Done before the builder borrows `ctx.func`.
        let fmod_ref = self.module.declare_func_in_func(self.fmod_id, &mut ctx.func);
        let pow_ref = self.module.declare_func_in_func(self.pow_id, &mut ctx.func);

        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut self.fbc);

            let cl_blocks: Vec<_> = plan.blocks.iter().map(|_| b.create_block()).collect();
            let bail_block = b.create_block();
            let entry = b.create_block();

            // One Variable per local slot, typed I64 (Int) or F64 (Float).
            let vars: Vec<Variable> = plan
                .slot_kind
                .iter()
                .map(|k| b.declare_var(if *k == Kind::Float { F64 } else { I64 }))
                .collect();
            let bail_var = b.declare_var(ptr_ty);

            // ── Prologue ────────────────────────────────────────────────────
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args_ptr = b.block_params(entry)[0];
            let bail_ptr = b.block_params(entry)[2];
            b.def_var(bail_var, bail_ptr);
            // Zero-init every local (belt-and-suspenders; analysis proves no
            // observed read of the zero).
            let zero_i = b.ins().iconst(I64, 0);
            let zero_f = b.ins().f64const(0.0);
            for (slot, &var) in vars.iter().enumerate() {
                if plan.slot_kind[slot] == Kind::Float {
                    b.def_var(var, zero_f);
                } else {
                    b.def_var(var, zero_i);
                }
            }
            // Bind args positionally into their param slots. The args buffer
            // holds 8 raw bytes per slot — an `i64` for Int params, the bit-cast
            // of an `f64` for Float params (see `run_compiled`).
            for (i, &slot) in plan.param_slots.iter().enumerate() {
                let off = (i * 8) as i32;
                let v = if plan.slot_kind[slot] == Kind::Float {
                    b.ins().load(F64, MemFlags::new(), args_ptr, off)
                } else {
                    b.ins().load(I64, MemFlags::new(), args_ptr, off)
                };
                b.def_var(vars[slot], v);
            }
            b.ins().jump(cl_blocks[0], &[]);

            // ── Bail block: *bail = 1; return 0 ─────────────────────────────
            b.switch_to_block(bail_block);
            let bp = b.use_var(bail_var);
            let one = b.ins().iconst(I64, 1);
            b.ins().store(MemFlags::new(), one, bp, 0);
            let zero_ret = b.ins().iconst(I64, 0);
            b.ins().return_(&[zero_ret]);

            // ── Translate each reachable block ──────────────────────────────
            for (bidx, blk) in plan.blocks.iter().enumerate() {
                b.switch_to_block(cl_blocks[bidx]);
                let mut stack: Vec<(Value, Kind)> = Vec::new();
                let mut terminated = false;

                let target_block = |ip: usize| -> Result<cranelift_codegen::ir::Block, String> {
                    plan.block_at
                        .get(&ip)
                        .map(|&i| cl_blocks[i])
                        .ok_or_else(|| format!("jit: target ip {ip} not a block leader"))
                };
                let fallthrough = target_block(blk.end);

                for ip in blk.start..blk.end {
                    match &func.instructions[ip] {
                        BytecodeOp::Integer(n) => stack.push((b.ins().iconst(I64, *n), Kind::Int)),
                        BytecodeOp::Double(d) => stack.push((b.ins().f64const(*d), Kind::Float)),
                        BytecodeOp::True => stack.push((b.ins().iconst(I64, 1), Kind::Bool)),
                        BytecodeOp::False => stack.push((b.ins().iconst(I64, 0), Kind::Bool)),

                        BytecodeOp::LoadLocal(name) => {
                            let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
                            stack.push((b.use_var(vars[slot]), plan.slot_kind[slot]));
                        }
                        BytecodeOp::StoreLocal(name) => {
                            let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
                            let (v, _) = stack.pop().ok_or("jit: stack underflow")?;
                            b.def_var(vars[slot], v);
                        }

                        BytecodeOp::Add => num_bin(&mut b, &mut stack, NumOp::Add)?,
                        BytecodeOp::Sub => num_bin(&mut b, &mut stack, NumOp::Sub)?,
                        BytecodeOp::Mul => num_bin(&mut b, &mut stack, NumOp::Mul)?,

                        // `/` — always f64; bail (→ interpreter throws) on zero.
                        BytecodeOp::Div => {
                            let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
                            let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
                            let a = to_f64(&mut b, lhs, lk);
                            let d = to_f64(&mut b, rhs, rk);
                            let fz = b.ins().f64const(0.0);
                            let is_zero = b.ins().fcmp(FloatCC::Equal, d, fz);
                            let cont = b.create_block();
                            b.ins().brif(is_zero, bail_block, &[], cont, &[]);
                            b.switch_to_block(cont);
                            stack.push((b.ins().fdiv(a, d), Kind::Float));
                        }

                        // `%` — Int,Int → wrapping `srem` (guarded against 0 /
                        // INT_MIN÷-1). Any float operand promotes both to f64
                        // and calls the `cfml_fmod` shim (matches the
                        // interpreter's `f64 %` on mixed/float operands).
                        BytecodeOp::Mod => {
                            let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
                            let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
                            if lk == Kind::Float || rk == Kind::Float {
                                let a = to_f64(&mut b, lhs, lk);
                                let d = to_f64(&mut b, rhs, rk);
                                let call = b.ins().call(fmod_ref, &[a, d]);
                                let r = b.inst_results(call)[0];
                                stack.push((r, Kind::Float));
                            } else {
                                let cont = guard_int_div(&mut b, bail_block, lhs, rhs);
                                b.switch_to_block(cont);
                                stack.push((b.ins().srem(lhs, rhs), Kind::Int));
                            }
                        }

                        // `^` — always Double. Promote both operands to f64 and
                        // call the `cfml_pow` shim (matches `f64::powf` used by
                        // the interpreter).
                        BytecodeOp::Pow => {
                            let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
                            let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
                            if !matches!(lk, Kind::Int | Kind::Float)
                                || !matches!(rk, Kind::Int | Kind::Float)
                            {
                                return Err("jit: pow operand not numeric".into());
                            }
                            let a = to_f64(&mut b, lhs, lk);
                            let d = to_f64(&mut b, rhs, rk);
                            let call = b.ins().call(pow_ref, &[a, d]);
                            let r = b.inst_results(call)[0];
                            stack.push((r, Kind::Float));
                        }

                        // `\` — always Int; float operands truncate toward zero.
                        BytecodeOp::IntDiv => {
                            let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
                            let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
                            let a = to_i64(&mut b, lhs, lk);
                            let d = to_i64(&mut b, rhs, rk);
                            let cont = guard_int_div(&mut b, bail_block, a, d);
                            b.switch_to_block(cont);
                            stack.push((b.ins().sdiv(a, d), Kind::Int));
                        }

                        BytecodeOp::Negate => {
                            let (a, k) = stack.pop().ok_or("jit: stack underflow")?;
                            let r = if k == Kind::Float {
                                b.ins().fneg(a)
                            } else {
                                b.ins().ineg(a)
                            };
                            stack.push((r, k));
                        }

                        BytecodeOp::Eq => cmp(&mut b, &mut stack, IntCC::Equal, FloatCC::Equal)?,
                        BytecodeOp::Neq => {
                            cmp(&mut b, &mut stack, IntCC::NotEqual, FloatCC::NotEqual)?
                        }
                        BytecodeOp::Lt => cmp(
                            &mut b,
                            &mut stack,
                            IntCC::SignedLessThan,
                            FloatCC::LessThan,
                        )?,
                        BytecodeOp::Lte => cmp(
                            &mut b,
                            &mut stack,
                            IntCC::SignedLessThanOrEqual,
                            FloatCC::LessThanOrEqual,
                        )?,
                        BytecodeOp::Gt => cmp(
                            &mut b,
                            &mut stack,
                            IntCC::SignedGreaterThan,
                            FloatCC::GreaterThan,
                        )?,
                        BytecodeOp::Gte => cmp(
                            &mut b,
                            &mut stack,
                            IntCC::SignedGreaterThanOrEqual,
                            FloatCC::GreaterThanOrEqual,
                        )?,

                        BytecodeOp::And => logic2(&mut b, &mut stack, LogicOp::And)?,
                        BytecodeOp::Or => logic2(&mut b, &mut stack, LogicOp::Or)?,
                        BytecodeOp::Xor => logic2(&mut b, &mut stack, LogicOp::Xor)?,
                        BytecodeOp::Not => {
                            let (a, k) = stack.pop().ok_or("jit: stack underflow")?;
                            let t = is_zero_test(&mut b, a, k);
                            stack.push((bool_to_i64(&mut b, t), Kind::Bool));
                        }

                        BytecodeOp::Increment(name) => rmw_imm(&mut b, &vars, plan, name, 1)?,
                        BytecodeOp::Decrement(name) => rmw_imm(&mut b, &vars, plan, name, -1)?,
                        BytecodeOp::AddLocalConst(name, k) => rmw_imm(&mut b, &vars, plan, name, *k)?,
                        BytecodeOp::MulLocalConst(name, k) => {
                            let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
                            let v = b.use_var(vars[slot]);
                            let nv = b.ins().imul_imm(v, *k);
                            b.def_var(vars[slot], nv);
                        }

                        BytecodeOp::JumpIfLocalCmpConstFalse(name, c, cmpop, target) => {
                            let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
                            let v = b.use_var(vars[slot]);
                            let cc = b.ins().icmp_imm(int_cc(*cmpop), v, *c);
                            b.ins().brif(cc, fallthrough.clone()?, &[], target_block(*target)?, &[]);
                            terminated = true;
                        }
                        BytecodeOp::ForLoopStep(name, limit, cmpop, step, target) => {
                            let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
                            let v = b.use_var(vars[slot]);
                            let nv = b.ins().iadd_imm(v, *step);
                            b.def_var(vars[slot], nv);
                            let cc = b.ins().icmp_imm(int_cc(*cmpop), nv, *limit);
                            b.ins().brif(cc, target_block(*target)?, &[], fallthrough.clone()?, &[]);
                            terminated = true;
                        }

                        BytecodeOp::Jump(target) => {
                            b.ins().jump(target_block(*target)?, &[]);
                            terminated = true;
                        }
                        BytecodeOp::JumpIfFalse(target) => {
                            let (cond, k) = stack.pop().ok_or("jit: stack underflow")?;
                            let is_false = is_zero_test(&mut b, cond, k);
                            b.ins().brif(is_false, target_block(*target)?, &[], fallthrough.clone()?, &[]);
                            terminated = true;
                        }
                        BytecodeOp::JumpIfTrue(target) => {
                            let (cond, k) = stack.pop().ok_or("jit: stack underflow")?;
                            let is_true = is_truthy(&mut b, cond, k);
                            b.ins().brif(is_true, target_block(*target)?, &[], fallthrough.clone()?, &[]);
                            terminated = true;
                        }

                        BytecodeOp::Pop => {
                            let _ = stack.pop();
                        }
                        BytecodeOp::Dup => {
                            let v = *stack.last().ok_or("jit: stack underflow")?;
                            stack.push(v);
                        }

                        BytecodeOp::Return => {
                            let (v, k) = stack.pop().ok_or("jit: stack underflow")?;
                            // A Float result is bit-cast to i64 for the ABI and
                            // re-interpreted as f64 by the engine.
                            let out = if k == Kind::Float {
                                b.ins().bitcast(I64, MemFlags::new(), v)
                            } else {
                                v
                            };
                            b.ins().return_(&[out]);
                            terminated = true;
                        }

                        BytecodeOp::DeclareLocal(_) => {}
                        BytecodeOp::LineInfo(_, _) => {}

                        other => return Err(format!("jit: unsupported op reached codegen: {other:?}")),
                    }
                }

                if !terminated {
                    b.ins().jump(fallthrough.clone()?, &[]);
                }
            }

            b.seal_all_blocks();
            b.finalize();
        }

        let name = format!("cfml_jit_{}", self.func_counter);
        self.func_counter += 1;
        let id = self
            .module
            .declare_function(&name, Linkage::Export, &ctx.func.signature)
            .map_err(|e| e.to_string())?;
        self.module
            .define_function(id, &mut ctx)
            .map_err(|e| e.to_string())?;
        self.module.clear_context(&mut ctx);
        self.module
            .finalize_definitions()
            .map_err(|e| e.to_string())?;
        let code = self.module.get_finalized_function(id);
        // SAFETY: `code` points at freshly-emitted native code for our exact
        // signature; it lives as long as the `JITModule` (owned by the engine).
        Ok(unsafe { std::mem::transmute::<*const u8, CompiledFn>(code) })
    }
}

/// CFML `CmpOp` → Cranelift signed integer condition (fused loop ops only).
fn int_cc(op: CmpOp) -> IntCC {
    match op {
        CmpOp::Lt => IntCC::SignedLessThan,
        CmpOp::Lte => IntCC::SignedLessThanOrEqual,
        CmpOp::Gt => IntCC::SignedGreaterThan,
        CmpOp::Gte => IntCC::SignedGreaterThanOrEqual,
        CmpOp::Eq => IntCC::Equal,
        CmpOp::Neq => IntCC::NotEqual,
    }
}

/// Promote a value to `f64` (int/bool → `fcvt_from_sint`; float passes through).
fn to_f64(b: &mut FunctionBuilder, v: Value, k: Kind) -> Value {
    if k == Kind::Float {
        v
    } else {
        b.ins().fcvt_from_sint(F64, v)
    }
}

/// Truncate a value to `i64` (float → saturating `fcvt_to_sint_sat`, matching
/// Rust's `as i64`; int/bool pass through).
fn to_i64(b: &mut FunctionBuilder, v: Value, k: Kind) -> Value {
    if k == Kind::Float {
        b.ins().fcvt_to_sint_sat(I64, v)
    } else {
        v
    }
}

/// Emit the integer divide/modulo guard: branch to `bail` when the divisor is
/// `0` or the `INT_MIN / -1` overflow case, returning the continuation block.
fn guard_int_div(
    b: &mut FunctionBuilder,
    bail: cranelift_codegen::ir::Block,
    dividend: Value,
    divisor: Value,
) -> cranelift_codegen::ir::Block {
    let is_zero = b.ins().icmp_imm(IntCC::Equal, divisor, 0);
    let div_neg1 = b.ins().icmp_imm(IntCC::Equal, divisor, -1);
    let dvd_min = b.ins().icmp_imm(IntCC::Equal, dividend, i64::MIN);
    let ov = b.ins().band(div_neg1, dvd_min);
    let bad = b.ins().bor(is_zero, ov);
    let cont = b.create_block();
    b.ins().brif(bad, bail, &[], cont, &[]);
    cont
}

/// Materialise a Cranelift boolean (`I8`) into an `i64` `0`/`1`.
fn bool_to_i64(b: &mut FunctionBuilder, cond: Value) -> Value {
    let one = b.ins().iconst(I64, 1);
    let zero = b.ins().iconst(I64, 0);
    b.ins().select(cond, one, zero)
}

/// Boolean: `v == 0` (`fcmp`/`icmp` per kind). Used by `Not` and `JumpIfFalse`.
fn is_zero_test(b: &mut FunctionBuilder, v: Value, k: Kind) -> Value {
    if k == Kind::Float {
        let z = b.ins().f64const(0.0);
        b.ins().fcmp(FloatCC::Equal, v, z)
    } else {
        b.ins().icmp_imm(IntCC::Equal, v, 0)
    }
}

/// Boolean: `v != 0` (`fcmp`/`icmp` per kind). Truthiness for logical ops.
fn is_truthy(b: &mut FunctionBuilder, v: Value, k: Kind) -> Value {
    if k == Kind::Float {
        let z = b.ins().f64const(0.0);
        b.ins().fcmp(FloatCC::NotEqual, v, z)
    } else {
        b.ins().icmp_imm(IntCC::NotEqual, v, 0)
    }
}

enum NumOp {
    Add,
    Sub,
    Mul,
}

/// `+ - *`: pop b, pop a. Two ints use wrapping integer ops (bit-exact with the
/// interpreter's `Int(i op j)`); any float operand promotes both and uses the
/// float op (matching the interpreter's int→double promotion on mixed operands).
fn num_bin(b: &mut FunctionBuilder, stack: &mut Vec<(Value, Kind)>, op: NumOp) -> Result<(), String> {
    let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
    let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
    if lk == Kind::Float || rk == Kind::Float {
        let a = to_f64(b, lhs, lk);
        let d = to_f64(b, rhs, rk);
        let r = match op {
            NumOp::Add => b.ins().fadd(a, d),
            NumOp::Sub => b.ins().fsub(a, d),
            NumOp::Mul => b.ins().fmul(a, d),
        };
        stack.push((r, Kind::Float));
    } else {
        let r = match op {
            NumOp::Add => b.ins().iadd(lhs, rhs),
            NumOp::Sub => b.ins().isub(lhs, rhs),
            NumOp::Mul => b.ins().imul(lhs, rhs),
        };
        stack.push((r, Kind::Int));
    }
    Ok(())
}

/// Comparison: pop b, pop a, push `(a CMP b)` as i64 `0`/`1`. Two ints compare
/// with `icmp`; a float operand promotes both and uses `fcmp` (matching the
/// interpreter, which compares mixed int/double in f64).
fn cmp(
    b: &mut FunctionBuilder,
    stack: &mut Vec<(Value, Kind)>,
    icc: IntCC,
    fcc: FloatCC,
) -> Result<(), String> {
    let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
    let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
    let c = if lk == Kind::Float || rk == Kind::Float {
        let a = to_f64(b, lhs, lk);
        let d = to_f64(b, rhs, rk);
        b.ins().fcmp(fcc, a, d)
    } else {
        b.ins().icmp(icc, lhs, rhs)
    };
    let r = bool_to_i64(b, c);
    stack.push((r, Kind::Bool));
    Ok(())
}

enum LogicOp {
    And,
    Or,
    Xor,
}

/// Logical op on truthiness: pop b, pop a, push `(a≠0 OP b≠0)` as i64 `0`/`1`.
fn logic2(b: &mut FunctionBuilder, stack: &mut Vec<(Value, Kind)>, op: LogicOp) -> Result<(), String> {
    let (rhs, rk) = stack.pop().ok_or("jit: stack underflow")?;
    let (lhs, lk) = stack.pop().ok_or("jit: stack underflow")?;
    let at = is_truthy(b, lhs, lk);
    let bt = is_truthy(b, rhs, rk);
    let r = match op {
        LogicOp::And => b.ins().band(at, bt),
        LogicOp::Or => b.ins().bor(at, bt),
        LogicOp::Xor => b.ins().bxor(at, bt),
    };
    let r = bool_to_i64(b, r);
    stack.push((r, Kind::Bool));
    Ok(())
}

/// Read-modify-write an integer local by a constant addend: `local += k`.
fn rmw_imm(
    b: &mut FunctionBuilder,
    vars: &[Variable],
    plan: &Plan,
    name: &str,
    k: i64,
) -> Result<(), String> {
    let slot = plan.slot_of(name).ok_or("jit: unknown local")?;
    let v = b.use_var(vars[slot]);
    let nv = b.ins().iadd_imm(v, k);
    b.def_var(vars[slot], nv);
    Ok(())
}
