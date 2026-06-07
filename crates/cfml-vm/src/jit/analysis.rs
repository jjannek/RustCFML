//! Static analysis that decides whether a [`BytecodeFunction`] is a JIT
//! candidate and, if so, produces the [`Plan`] the translator consumes.
//!
//! A function is accepted only when *every reachable instruction* is in the
//! supported, side-effect-free, numeric subset and several safety properties
//! hold. Tier-1 covers **integer** kernels; Tier-1.5 (this revision) additionally
//! covers **floating-point** kernels — functions that produce `Double`s via
//! `Double` literals, the `/` operator, or float-typed locals. Arguments are
//! still bound as integers at the ABI boundary (the engine bails to the
//! interpreter unless every argument is a `CfmlValue::Int`), so float-ness only
//! ever arises *inside* an accepted function.
//!
//! The checks (all over the **reachable** CFG only, so dead trailing
//! `Null; Return` epilogues never disqualify a function):
//!
//! 1. **No defaulted params / not `__main__`** — args are bound positionally.
//! 2. **Op-subset** — only the numeric/arithmetic/counted-loop ops below.
//! 3. **No reserved scope names** in local ops (`variables`, `arguments`, …).
//! 4. **Slot kinds** — every local slot is *uniformly* `Int` or `Float`. A
//!    monotonic fixpoint upgrades a slot to `Float` as soon as a `Float` value is
//!    stored into it; the consistency pass then rejects any path that stores an
//!    `Int`-kind value into a `Float` slot (that would be a *path-dependent type*
//!    the monomorphic JIT cannot reproduce). Param slots are pinned `Int` (they
//!    arrive as integers across the ABI), so a function that reassigns a param to
//!    a float result is rejected.
//! 5. **Operand-stack discipline** — within each basic block the operand stack
//!    starts and ends empty; a boolean (comparison/logical) result is never
//!    stored into a local nor returned.
//! 6. **Definite assignment** — a local is never read on a path where it may be
//!    unassigned (preserves the interpreter's "undefined var ⇒ error").
//! 7. **No fall-off** — control cannot reach the end of the body without a
//!    `Return`, and every `Return` agrees on the value kind (the function's
//!    [`Plan::ret_kind`], either all `Int` or all `Float`).
//!
//! Given the above, the native body always produces a value of the statically
//! known [`Plan::ret_kind`] that the engine re-wraps (`Int` ⇒ `CfmlValue::Int`,
//! `Float` ⇒ `CfmlValue::Double` from the returned bit pattern) with no
//! observable difference.

use cfml_codegen::{BytecodeOp, BytecodeFunction};
use std::collections::{BTreeSet, HashMap};

/// A reachable basic block: the half-open instruction range `[start, end)`.
pub struct Block {
    pub start: usize,
    pub end: usize,
}

/// Operand-stack / local value kind. Shared with the translator so the two
/// agree exactly on how each op is typed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A guaranteed integer (literal, int local, or integer arithmetic result).
    Int,
    /// A guaranteed float/`Double` (Double literal, `/` result, or float local).
    Float,
    /// A boolean (comparison or logical result). May only be consumed by a
    /// branch or another logical/comparison op — never stored or returned.
    Bool,
}

impl Kind {
    /// `true` for the numeric kinds (everything but `Bool`).
    fn is_num(self) -> bool {
        matches!(self, Kind::Int | Kind::Float)
    }
}

/// Result kind of a `+`/`-`/`*`/`%` on numeric operands: `Float` if either
/// operand is `Float`, else `Int`. `None` if either operand is a boolean.
pub fn num_bin_kind(a: Kind, b: Kind) -> Option<Kind> {
    if !a.is_num() || !b.is_num() {
        return None;
    }
    Some(if a == Kind::Float || b == Kind::Float {
        Kind::Float
    } else {
        Kind::Int
    })
}

/// Everything the translator needs to emit Cranelift IR for an accepted fn.
pub struct Plan {
    /// Lowercased local name → slot index (slot = Cranelift `Variable` number).
    pub local_slot: HashMap<String, usize>,
    /// Param index → slot index (args arrive positionally and seed these slots).
    pub param_slots: Vec<usize>,
    /// Slot index → its uniform value kind (`Int` or `Float`; never `Bool`).
    pub slot_kind: Vec<Kind>,
    /// The kind every `Return` yields (`Int` or `Float`).
    pub ret_kind: Kind,
    /// Reachable blocks, sorted by `start`.
    pub blocks: Vec<Block>,
    /// Leader ip → index into `blocks`.
    pub block_at: HashMap<usize, usize>,
}

impl Plan {
    /// Slot for a (case-insensitive) local name, if referenced by the function.
    pub fn slot_of(&self, name: &str) -> Option<usize> {
        self.local_slot.get(&lower(name)).copied()
    }
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn is_reserved_scope(name: &str) -> bool {
    matches!(
        lower(name).as_str(),
        "local"
            | "variables"
            | "arguments"
            | "this"
            | "super"
            | "request"
            | "application"
            | "session"
            | "server"
            | "cgi"
            | "url"
            | "form"
            | "cookie"
            | "client"
            | "thread"
    )
}

/// Decide whether `func` can be compiled. `None` ⇒ keep on the interpreter.
pub fn analyze(func: &BytecodeFunction) -> Option<Plan> {
    if func.name == "__main__" {
        return None;
    }
    // Args are bound positionally; defaulted params need the runtime preamble.
    if func.has_default.iter().any(|d| *d) {
        return None;
    }

    let code = &func.instructions;
    let n = code.len();
    if n == 0 {
        return None;
    }

    // ── 1. Leaders & basic blocks ───────────────────────────────────────────
    let mut leader_set: BTreeSet<usize> = BTreeSet::new();
    leader_set.insert(0);
    for (ip, op) in code.iter().enumerate() {
        match op {
            BytecodeOp::Jump(t)
            | BytecodeOp::JumpIfFalse(t)
            | BytecodeOp::JumpIfTrue(t)
            | BytecodeOp::JumpIfLocalCmpConstFalse(_, _, _, t)
            | BytecodeOp::ForLoopStep(_, _, _, _, t) => {
                if *t >= n {
                    return None; // jump past end of body — bail out of JIT
                }
                leader_set.insert(*t);
                if ip + 1 < n {
                    leader_set.insert(ip + 1);
                }
            }
            BytecodeOp::Return => {
                if ip + 1 < n {
                    leader_set.insert(ip + 1);
                }
            }
            _ => {}
        }
    }
    let leaders: Vec<usize> = leader_set.iter().copied().collect();
    let mut block_at: HashMap<usize, (usize, usize)> = HashMap::new();
    for (i, &start) in leaders.iter().enumerate() {
        let end = leaders.get(i + 1).copied().unwrap_or(n);
        block_at.insert(start, (start, end));
    }

    // ── 2. Reachability from entry (structural terminator reading) ──────────
    let succ = |_start: usize, end: usize| -> Option<Vec<usize>> {
        let term = &code[end - 1];
        Some(match term {
            BytecodeOp::Jump(t) => vec![*t],
            BytecodeOp::JumpIfFalse(t)
            | BytecodeOp::JumpIfTrue(t)
            | BytecodeOp::JumpIfLocalCmpConstFalse(_, _, _, t)
            | BytecodeOp::ForLoopStep(_, _, _, _, t) => {
                if end >= n {
                    return None;
                }
                vec![*t, end]
            }
            BytecodeOp::Return => vec![],
            _ => {
                if end >= n {
                    return None; // fall-off the end without a Return
                }
                vec![end]
            }
        })
    };

    let mut reachable: BTreeSet<usize> = BTreeSet::new();
    let mut work = vec![0usize];
    while let Some(start) = work.pop() {
        if !reachable.insert(start) {
            continue;
        }
        let (s, e) = match block_at.get(&start) {
            Some(&r) => r,
            None => return None,
        };
        let succs = succ(s, e)?;
        for t in succs {
            if !block_at.contains_key(&t) {
                return None;
            }
            if !reachable.contains(&t) {
                work.push(t);
            }
        }
    }

    let reach_sorted: Vec<usize> = reachable.iter().copied().collect();
    let mut plan_blocks: Vec<Block> = Vec::with_capacity(reach_sorted.len());
    let mut plan_block_at: HashMap<usize, usize> = HashMap::new();
    for (idx, &start) in reach_sorted.iter().enumerate() {
        let (s, e) = block_at[&start];
        plan_block_at.insert(s, idx);
        plan_blocks.push(Block { start: s, end: e });
    }

    // ── 3. Intern locals + op-subset + reserved-scope + local events ────────
    let mut local_slot: HashMap<String, usize> = HashMap::new();
    let mut locals: Vec<String> = Vec::new();
    let intern = |name: &str, locals: &mut Vec<String>, map: &mut HashMap<String, usize>| -> usize {
        let key = lower(name);
        if let Some(&s) = map.get(&key) {
            s
        } else {
            let s = locals.len();
            locals.push(key.clone());
            map.insert(key, s);
            s
        }
    };
    let mut param_slots: Vec<usize> = Vec::with_capacity(func.params.len());
    for p in &func.params {
        param_slots.push(intern(p, &mut locals, &mut local_slot));
    }

    #[derive(Clone, Copy)]
    enum Ev {
        Read(usize),
        Write(usize),
    }
    let mut block_events: Vec<Vec<Ev>> = vec![Vec::new(); plan_blocks.len()];

    for (bidx, blk) in plan_blocks.iter().enumerate() {
        let events = &mut block_events[bidx];
        for ip in blk.start..blk.end {
            match &code[ip] {
                // value-producing / value-consuming ops with no local event
                BytecodeOp::Integer(_)
                | BytecodeOp::Double(_)
                | BytecodeOp::True
                | BytecodeOp::False
                | BytecodeOp::Add
                | BytecodeOp::Sub
                | BytecodeOp::Mul
                | BytecodeOp::Div
                | BytecodeOp::Mod
                | BytecodeOp::IntDiv
                | BytecodeOp::Negate
                | BytecodeOp::Eq
                | BytecodeOp::Neq
                | BytecodeOp::Lt
                | BytecodeOp::Lte
                | BytecodeOp::Gt
                | BytecodeOp::Gte
                | BytecodeOp::And
                | BytecodeOp::Or
                | BytecodeOp::Xor
                | BytecodeOp::Not
                | BytecodeOp::Jump(_)
                | BytecodeOp::JumpIfFalse(_)
                | BytecodeOp::JumpIfTrue(_)
                | BytecodeOp::Pop
                | BytecodeOp::Dup
                | BytecodeOp::Return
                | BytecodeOp::LineInfo(_, _) => {}

                BytecodeOp::LoadLocal(name) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    let s = intern(name, &mut locals, &mut local_slot);
                    events.push(Ev::Read(s));
                }
                BytecodeOp::StoreLocal(name) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    let s = intern(name, &mut locals, &mut local_slot);
                    events.push(Ev::Write(s));
                }
                BytecodeOp::Increment(name)
                | BytecodeOp::Decrement(name)
                | BytecodeOp::AddLocalConst(name, _)
                | BytecodeOp::MulLocalConst(name, _) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    let s = intern(name, &mut locals, &mut local_slot);
                    events.push(Ev::Read(s));
                    events.push(Ev::Write(s));
                }
                BytecodeOp::JumpIfLocalCmpConstFalse(name, _, _, _) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    let s = intern(name, &mut locals, &mut local_slot);
                    events.push(Ev::Read(s));
                }
                BytecodeOp::ForLoopStep(name, _, _, _, _) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    let s = intern(name, &mut locals, &mut local_slot);
                    events.push(Ev::Read(s));
                    events.push(Ev::Write(s));
                }
                BytecodeOp::DeclareLocal(name) => {
                    if is_reserved_scope(name) {
                        return None;
                    }
                    intern(name, &mut locals, &mut local_slot);
                }

                // everything else (Null, Concat, Pow, calls, heap, …)
                _ => return None,
            }
        }
    }

    // ── 4. Slot-kind fixpoint (monotonic Int → Float upgrades) ──────────────
    let nslots = locals.len();
    let mut slot_kind = vec![Kind::Int; nslots]; // params + all locals start Int
    loop {
        let mut changed = false;
        for blk in &plan_blocks {
            simulate_block(
                code,
                blk,
                &local_slot,
                &mut slot_kind,
                Mode::Infer { changed: &mut changed },
            )?;
        }
        if !changed {
            break;
        }
    }

    // Param slots must stay Int — they arrive as integers across the ABI. A
    // float reassignment of a param is a path-dependent type the JIT can't model.
    for &p in &param_slots {
        if slot_kind[p] == Kind::Float {
            return None;
        }
    }

    // ── 5. Consistency + kind validation pass (records the return kind) ─────
    let mut ret_kind: Option<Kind> = None;
    for blk in &plan_blocks {
        simulate_block(
            code,
            blk,
            &local_slot,
            &mut slot_kind,
            Mode::Check { ret_kind: &mut ret_kind },
        )?;
    }
    let ret_kind = ret_kind?; // a function with no reachable Return is rejected

    // ── 6. Definite-assignment fixpoint over reachable blocks ───────────────
    let nblk = plan_blocks.len();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nblk];
    for (bidx, blk) in plan_blocks.iter().enumerate() {
        let succs = succ(blk.start, blk.end)?;
        for t in succs {
            let ti = plan_block_at[&t];
            preds[ti].push(bidx);
        }
    }

    let full: Vec<bool> = vec![true; nslots];
    let mut assigned_in: Vec<Vec<bool>> = vec![full.clone(); nblk];
    let mut entry_in = vec![false; nslots];
    for &s in &param_slots {
        entry_in[s] = true;
    }
    assigned_in[0] = entry_in;

    let transfer = |bidx: usize, ain: &[bool]| -> Vec<bool> {
        let mut a = ain.to_vec();
        for ev in &block_events[bidx] {
            if let Ev::Write(s) = ev {
                a[*s] = true;
            }
        }
        a
    };

    let mut changed = true;
    let mut assigned_out: Vec<Vec<bool>> = vec![full.clone(); nblk];
    while changed {
        changed = false;
        for bidx in 0..nblk {
            if bidx != 0 {
                let mut newin = if preds[bidx].is_empty() {
                    vec![false; nslots]
                } else {
                    vec![true; nslots]
                };
                for &p in &preds[bidx] {
                    for s in 0..nslots {
                        newin[s] = newin[s] && assigned_out[p][s];
                    }
                }
                if newin != assigned_in[bidx] {
                    assigned_in[bidx] = newin;
                    changed = true;
                }
            }
            let newout = transfer(bidx, &assigned_in[bidx]);
            if newout != assigned_out[bidx] {
                assigned_out[bidx] = newout;
                changed = true;
            }
        }
    }

    for bidx in 0..nblk {
        let mut cur = assigned_in[bidx].clone();
        for ev in &block_events[bidx] {
            match ev {
                Ev::Read(s) => {
                    if !cur[*s] {
                        return None;
                    }
                }
                Ev::Write(s) => cur[*s] = true,
            }
        }
    }

    Some(Plan {
        local_slot,
        param_slots,
        slot_kind,
        ret_kind,
        blocks: plan_blocks,
        block_at: plan_block_at,
    })
}

/// Mode for [`simulate_block`].
enum Mode<'a> {
    /// Fixpoint inference: upgrade a slot to `Float` when a `Float` value is
    /// stored into it, flagging `changed`. Never rejects for kind reasons.
    Infer { changed: &'a mut bool },
    /// Final validation: reject an `Int`-kind value stored into a `Float` slot,
    /// reject `Bool` stores/returns, and record the (uniform) return kind.
    Check { ret_kind: &'a mut Option<Kind> },
}

/// Abstractly interpret one basic block's operand stack with the current slot
/// kinds. Returns `Some(())` if the block is well-formed (operand stack starts
/// and ends empty, no underflow, supported kinds), `None` to reject the whole
/// function. The op-subset has already been validated, so any op not handled
/// here is treated as a rejection (defensive).
fn simulate_block(
    code: &[BytecodeOp],
    blk: &Block,
    local_slot: &HashMap<String, usize>,
    slot_kind: &mut [Kind],
    mut mode: Mode,
) -> Option<()> {
    let slot_index = |name: &str| -> Option<usize> { local_slot.get(&lower(name)).copied() };
    let mut stack: Vec<Kind> = Vec::new();

    macro_rules! pop {
        () => {
            match stack.pop() {
                Some(k) => k,
                None => return None,
            }
        };
    }

    for ip in blk.start..blk.end {
        match &code[ip] {
            BytecodeOp::Integer(_) => stack.push(Kind::Int),
            BytecodeOp::Double(_) => stack.push(Kind::Float),
            BytecodeOp::True | BytecodeOp::False => stack.push(Kind::Bool),

            BytecodeOp::LoadLocal(name) => {
                stack.push(slot_kind[slot_index(name)?]);
            }
            BytecodeOp::StoreLocal(name) => {
                let s = slot_index(name)?;
                let v = pop!();
                if v == Kind::Bool {
                    return None; // a boolean must never enter a local
                }
                match &mut mode {
                    Mode::Infer { changed } => {
                        if v == Kind::Float && slot_kind[s] == Kind::Int {
                            slot_kind[s] = Kind::Float;
                            **changed = true;
                        }
                    }
                    Mode::Check { .. } => {
                        if v != slot_kind[s] {
                            return None; // mixed-kind slot — not monomorphic
                        }
                    }
                }
            }

            // `+ - *`: Int,Int → Int; any Float → Float; Bool operand rejects.
            BytecodeOp::Add | BytecodeOp::Sub | BytecodeOp::Mul => {
                let b = pop!();
                let a = pop!();
                stack.push(num_bin_kind(a, b)?);
            }
            // `%`: Int,Int → Int. Cranelift has no float-remainder instruction
            // (it's a libcall), so a float modulo rejects the whole function.
            BytecodeOp::Mod => {
                let b = pop!();
                let a = pop!();
                match num_bin_kind(a, b)? {
                    Kind::Int => stack.push(Kind::Int),
                    _ => return None,
                }
            }
            // `/` always yields a Double; operands must be numeric.
            BytecodeOp::Div => {
                let b = pop!();
                let a = pop!();
                if !a.is_num() || !b.is_num() {
                    return None;
                }
                stack.push(Kind::Float);
            }
            // `\` always yields an Int (operands truncated to i64).
            BytecodeOp::IntDiv => {
                let b = pop!();
                let a = pop!();
                if !a.is_num() || !b.is_num() {
                    return None;
                }
                stack.push(Kind::Int);
            }
            BytecodeOp::Negate => {
                let a = pop!();
                if !a.is_num() {
                    return None;
                }
                stack.push(a);
            }

            BytecodeOp::Eq
            | BytecodeOp::Neq
            | BytecodeOp::Lt
            | BytecodeOp::Lte
            | BytecodeOp::Gt
            | BytecodeOp::Gte => {
                let _ = pop!();
                let _ = pop!();
                stack.push(Kind::Bool);
            }
            BytecodeOp::And | BytecodeOp::Or | BytecodeOp::Xor => {
                let _ = pop!();
                let _ = pop!();
                stack.push(Kind::Bool);
            }
            BytecodeOp::Not => {
                let _ = pop!();
                stack.push(Kind::Bool);
            }

            // Integer-constant read-modify-write — the slot must be an Int.
            BytecodeOp::Increment(name)
            | BytecodeOp::Decrement(name)
            | BytecodeOp::AddLocalConst(name, _)
            | BytecodeOp::MulLocalConst(name, _) => {
                if slot_kind[slot_index(name)?] != Kind::Int {
                    return None;
                }
            }
            BytecodeOp::JumpIfLocalCmpConstFalse(name, _, _, _) => {
                if slot_kind[slot_index(name)?] != Kind::Int {
                    return None;
                }
            }
            BytecodeOp::ForLoopStep(name, _, _, _, _) => {
                if slot_kind[slot_index(name)?] != Kind::Int {
                    return None;
                }
            }

            BytecodeOp::Jump(_) => {}
            BytecodeOp::JumpIfFalse(_) | BytecodeOp::JumpIfTrue(_) => {
                let _ = pop!();
            }

            BytecodeOp::Pop => {
                let _ = stack.pop(); // tolerate an empty stack
            }
            BytecodeOp::Dup => {
                let k = *stack.last()?;
                stack.push(k);
            }

            BytecodeOp::Return => {
                let v = pop!();
                if v == Kind::Bool {
                    return None;
                }
                if let Mode::Check { ret_kind } = &mut mode {
                    match ret_kind {
                        Some(existing) if *existing != v => return None, // mixed ret kind
                        _ => **ret_kind = Some(v),
                    }
                }
            }

            BytecodeOp::DeclareLocal(_) | BytecodeOp::LineInfo(_, _) => {}

            _ => return None,
        }
    }

    if !stack.is_empty() {
        return None; // operand stack must be empty at the block boundary
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfml_codegen::CmpOp;

    fn mkfn(params: &[&str], instrs: Vec<BytecodeOp>) -> BytecodeFunction {
        BytecodeFunction {
            name: "f".to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            required_params: params.iter().map(|_| true).collect(),
            has_default: params.iter().map(|_| false).collect(),
            instructions: instrs,
            source_file: None,
            global_id: 1,
            declared_local_mode: None,
        }
    }

    #[test]
    fn accepts_simple_arithmetic() {
        let f = mkfn(
            &["a", "b"],
            vec![
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::LoadLocal("b".into()),
                BytecodeOp::Mul,
                BytecodeOp::Integer(1),
                BytecodeOp::Add,
                BytecodeOp::Return,
            ],
        );
        let plan = analyze(&f).expect("should be JIT-eligible");
        assert_eq!(plan.param_slots.len(), 2);
        assert!(plan.slot_of("A").is_some(), "case-insensitive local lookup");
        assert_eq!(plan.ret_kind, Kind::Int);
    }

    #[test]
    fn rejects_reserved_scope() {
        let f = mkfn(
            &[],
            vec![BytecodeOp::LoadLocal("variables".into()), BytecodeOp::Return],
        );
        assert!(analyze(&f).is_none());
    }

    #[test]
    fn rejects_unsupported_op() {
        let f = mkfn(&[], vec![BytecodeOp::String("x".into()), BytecodeOp::Return]);
        assert!(analyze(&f).is_none());
    }

    #[test]
    fn rejects_read_before_assign() {
        let f = mkfn(&[], vec![BytecodeOp::LoadLocal("x".into()), BytecodeOp::Return]);
        assert!(analyze(&f).is_none());
    }

    #[test]
    fn rejects_returning_boolean() {
        let f = mkfn(
            &["a"],
            vec![
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::Integer(0),
                BytecodeOp::Gt,
                BytecodeOp::Return,
            ],
        );
        assert!(analyze(&f).is_none());
    }

    #[test]
    fn rejects_void_via_trailing_null() {
        let f = mkfn(&[], vec![BytecodeOp::Null, BytecodeOp::Return]);
        assert!(analyze(&f).is_none());
    }

    #[test]
    fn ignores_unreachable_trailing_null() {
        let f = mkfn(
            &["a"],
            vec![
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::Return,
                BytecodeOp::Null,
                BytecodeOp::Return,
            ],
        );
        assert!(analyze(&f).is_some(), "dead Null epilogue must not disqualify");
    }

    #[test]
    fn accepts_counted_loop() {
        let f = mkfn(
            &[],
            vec![
                BytecodeOp::Integer(0),
                BytecodeOp::StoreLocal("sum".into()),
                BytecodeOp::Integer(1),
                BytecodeOp::StoreLocal("i".into()),
                BytecodeOp::JumpIfLocalCmpConstFalse("i".into(), 10, CmpOp::Lte, 9),
                BytecodeOp::LoadLocal("sum".into()),
                BytecodeOp::LoadLocal("i".into()),
                BytecodeOp::Add,
                BytecodeOp::StoreLocal("sum".into()),
                BytecodeOp::ForLoopStep("i".into(), 10, CmpOp::Lte, 1, 5),
                BytecodeOp::LoadLocal("sum".into()),
                BytecodeOp::Return,
            ],
        );
        assert!(analyze(&f).is_some());
    }

    #[test]
    fn accepts_float_divide() {
        // function f(a, b) { return a / b; }  → Double result
        let f = mkfn(
            &["a", "b"],
            vec![
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::LoadLocal("b".into()),
                BytecodeOp::Div,
                BytecodeOp::Return,
            ],
        );
        let plan = analyze(&f).expect("float divide should be eligible");
        assert_eq!(plan.ret_kind, Kind::Float);
    }

    #[test]
    fn accepts_float_local_accumulator() {
        // function f() { var s = 0.0; s = s + 1; return s; }  → s is Float
        let f = mkfn(
            &[],
            vec![
                BytecodeOp::Double(0.0),
                BytecodeOp::StoreLocal("s".into()),
                BytecodeOp::LoadLocal("s".into()),
                BytecodeOp::Integer(1),
                BytecodeOp::Add,
                BytecodeOp::StoreLocal("s".into()),
                BytecodeOp::LoadLocal("s".into()),
                BytecodeOp::Return,
            ],
        );
        let plan = analyze(&f).expect("float accumulator should be eligible");
        assert_eq!(plan.ret_kind, Kind::Float);
        assert_eq!(plan.slot_kind[plan.slot_of("s").unwrap()], Kind::Float);
    }

    #[test]
    fn rejects_mixed_kind_slot() {
        // var x = 0; x = 1.5; return x;  → Int store into an upgraded-Float slot
        let f = mkfn(
            &[],
            vec![
                BytecodeOp::Integer(0),
                BytecodeOp::StoreLocal("x".into()),
                BytecodeOp::Double(1.5),
                BytecodeOp::StoreLocal("x".into()),
                BytecodeOp::LoadLocal("x".into()),
                BytecodeOp::Return,
            ],
        );
        assert!(analyze(&f).is_none(), "path-dependent slot type must reject");
    }

    #[test]
    fn rejects_float_param_reassignment() {
        // function f(a) { a = a / 2; return a; }  → param slot would become Float
        let f = mkfn(
            &["a"],
            vec![
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::Integer(2),
                BytecodeOp::Div,
                BytecodeOp::StoreLocal("a".into()),
                BytecodeOp::LoadLocal("a".into()),
                BytecodeOp::Return,
            ],
        );
        assert!(analyze(&f).is_none(), "float-reassigned param must reject");
    }
}
