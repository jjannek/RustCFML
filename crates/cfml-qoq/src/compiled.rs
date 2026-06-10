//! Compiled-expression form for QoQ — a hand-rolled mini-JIT.
//!
//! At `bind_core` time the engine walks each per-row expression (currently the
//! WHERE clause; SELECT and ORDER BY are candidates for follow-up releases)
//! into [`CompiledExpr`], an enum that pre-resolves shape-specific dispatch.
//! The per-row evaluator (`Engine::eval_compiled`, in `execution.rs`) then
//! matches once on the variant instead of walking the full [`Expr`] tree per
//! node per row.
//!
//! Shapes we don't recognise become [`CompiledExpr::Generic`] and fall back to
//! the existing AST evaluator — so partial specialisation is safe to ship
//! incrementally. The first round targets the WHERE-clause hot path on the
//! `bdw429s/cfml-qoq-perf-tests` benchmark (Q1's `age > 20 AND department IN
//! (...) AND isActive = 1` AND-chain of column-vs-literal predicates).

use std::sync::Arc;

use cfml_common::dynamic::CfmlValue;

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::like;

/// Comparison opcode separated from [`BinaryOp`] so the eval-time match only
/// has the cases that actually do a 3-valued compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

/// Pre-specialised expression form. Built once at bind time; consulted per row.
#[derive(Debug, Clone)]
pub enum CompiledExpr {
    // ── Leaves ────────────────────────────────────────────────────────────
    Null,
    LitBool(bool),
    LitInt(i64),
    LitDouble(f64),
    LitString(Arc<str>),
    /// Pre-resolved column slot.
    Column { ti: u32, ci: u32 },

    // ── Logical / 3-valued ───────────────────────────────────────────────
    /// N-ary AND with short-circuit on first FALSE.
    And(Vec<CompiledExpr>),
    /// N-ary OR with short-circuit on first TRUE.
    Or(Vec<CompiledExpr>),
    Not(Box<CompiledExpr>),
    IsNull { expr: Box<CompiledExpr>, negated: bool },

    // ── Comparison specialisations ───────────────────────────────────────
    /// `column <op> literal` — the most common WHERE shape. Skips the rhs
    /// AST eval (no recursive call, no Result wrapping).
    ColCmpLit { ti: u32, ci: u32, op: CmpOp, rhs: CfmlValue },
    /// Two arbitrary subexpressions joined by a comparison.
    Cmp { lhs: Box<CompiledExpr>, op: CmpOp, rhs: Box<CompiledExpr> },

    // ── Membership ───────────────────────────────────────────────────────
    /// `column [NOT] IN (lit, lit, ...)` — every list element a literal.
    ColInLits { ti: u32, ci: u32, negated: bool, lits: Vec<CfmlValue> },

    // ── LIKE (constant pattern) ──────────────────────────────────────────
    LikeConst {
        lhs: Box<CompiledExpr>,
        negated: bool,
        compiled: Arc<like::Compiled>,
    },

    // ── Fallback ─────────────────────────────────────────────────────────
    /// Subtree we haven't specialised — routed to `Engine::eval()`.
    Generic(Expr),
}

/// Compile a (post-`bind_expr`) Expr into [`CompiledExpr`]. The bound form is
/// expected — `Column` references should have been rewritten to
/// `ResolvedColumn` by `bind_expr`; otherwise they fall into `Generic`.
pub fn compile(expr: &Expr) -> CompiledExpr {
    match expr {
        Expr::Literal(v) => match v {
            CfmlValue::Null => CompiledExpr::Null,
            CfmlValue::Bool(b) => CompiledExpr::LitBool(*b),
            CfmlValue::Int(i) => CompiledExpr::LitInt(*i),
            CfmlValue::Double(d) => CompiledExpr::LitDouble(*d),
            CfmlValue::String(s) => CompiledExpr::LitString(Arc::from(s.as_str())),
            _ => CompiledExpr::Generic(expr.clone()),
        },
        Expr::ResolvedColumn { ti, ci, .. } => CompiledExpr::Column { ti: *ti, ci: *ci },
        Expr::IsNull { expr: inner, negated } => CompiledExpr::IsNull {
            expr: Box::new(compile(inner)),
            negated: *negated,
        },
        Expr::Unary {
            op: UnaryOp::Not,
            expr: inner,
        } => CompiledExpr::Not(Box::new(compile(inner))),
        Expr::Binary { left, op, right } => compile_binary(left, *op, right, expr),
        Expr::InList {
            expr: lhs,
            negated,
            list,
        } => {
            if let Expr::ResolvedColumn { ti, ci, .. } = &**lhs {
                let all_lit: Option<Vec<CfmlValue>> = list
                    .iter()
                    .map(|e| match e {
                        Expr::Literal(v) => Some(v.clone()),
                        _ => None,
                    })
                    .collect();
                if let Some(lits) = all_lit {
                    return CompiledExpr::ColInLits {
                        ti: *ti,
                        ci: *ci,
                        negated: *negated,
                        lits,
                    };
                }
            }
            CompiledExpr::Generic(expr.clone())
        }
        Expr::Like {
            expr: lhs,
            negated,
            compiled: Some(c),
            ..
        } => CompiledExpr::LikeConst {
            lhs: Box::new(compile(lhs)),
            negated: *negated,
            compiled: Arc::new(c.clone()),
        },
        _ => CompiledExpr::Generic(expr.clone()),
    }
}

fn compile_binary(left: &Expr, op: BinaryOp, right: &Expr, whole: &Expr) -> CompiledExpr {
    match op {
        BinaryOp::And => {
            let mut parts = Vec::new();
            push_and(left, &mut parts);
            push_and(right, &mut parts);
            CompiledExpr::And(parts)
        }
        BinaryOp::Or => {
            let mut parts = Vec::new();
            push_or(left, &mut parts);
            push_or(right, &mut parts);
            CompiledExpr::Or(parts)
        }
        BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => {
            let cmp = bin_to_cmp(op);
            // Column-vs-literal in either order.
            if let (Expr::ResolvedColumn { ti, ci, .. }, Expr::Literal(v)) = (left, right) {
                return CompiledExpr::ColCmpLit {
                    ti: *ti,
                    ci: *ci,
                    op: cmp,
                    rhs: v.clone(),
                };
            }
            if let (Expr::Literal(v), Expr::ResolvedColumn { ti, ci, .. }) = (left, right) {
                // Swap so the column is the LHS; reverse the operator.
                return CompiledExpr::ColCmpLit {
                    ti: *ti,
                    ci: *ci,
                    op: reverse_cmp(cmp),
                    rhs: v.clone(),
                };
            }
            CompiledExpr::Cmp {
                lhs: Box::new(compile(left)),
                op: cmp,
                rhs: Box::new(compile(right)),
            }
        }
        _ => CompiledExpr::Generic(whole.clone()),
    }
}

fn push_and(e: &Expr, out: &mut Vec<CompiledExpr>) {
    if let Expr::Binary {
        left,
        op: BinaryOp::And,
        right,
    } = e
    {
        push_and(left, out);
        push_and(right, out);
    } else {
        out.push(compile(e));
    }
}

fn push_or(e: &Expr, out: &mut Vec<CompiledExpr>) {
    if let Expr::Binary {
        left,
        op: BinaryOp::Or,
        right,
    } = e
    {
        push_or(left, out);
        push_or(right, out);
    } else {
        out.push(compile(e));
    }
}

fn bin_to_cmp(op: BinaryOp) -> CmpOp {
    match op {
        BinaryOp::Eq => CmpOp::Eq,
        BinaryOp::Neq => CmpOp::Neq,
        BinaryOp::Lt => CmpOp::Lt,
        BinaryOp::Lte => CmpOp::Lte,
        BinaryOp::Gt => CmpOp::Gt,
        BinaryOp::Gte => CmpOp::Gte,
        _ => unreachable!("bin_to_cmp called with non-comparison op"),
    }
}

fn reverse_cmp(op: CmpOp) -> CmpOp {
    match op {
        CmpOp::Eq => CmpOp::Eq,
        CmpOp::Neq => CmpOp::Neq,
        CmpOp::Lt => CmpOp::Gt,
        CmpOp::Gt => CmpOp::Lt,
        CmpOp::Lte => CmpOp::Gte,
        CmpOp::Gte => CmpOp::Lte,
    }
}
