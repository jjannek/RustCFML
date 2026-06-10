//! SQL AST for the Query-of-Queries engine.
//!
//! Modelled on BoxLang's `ortus.boxlang.compiler.ast.sql.*`: a `SelectStatement`
//! is a primary `SelectCore` plus optional `UNION` arms, with statement-level
//! `ORDER BY` / `LIMIT` that apply once after the union. Each `SelectCore` holds
//! one SELECT's projection, FROM/JOINs, WHERE, GROUP BY and HAVING.

use cfml_common::dynamic::CfmlValue;

use crate::like;

/// Top-level parsed statement. QoQ only executes `SELECT`.
#[derive(Debug, Clone)]
pub enum Statement {
    Select(SelectStatement),
}

/// A full SELECT statement: a primary body, optional UNION arms, and
/// statement-level ORDER BY / LIMIT (applied once, after the union).
#[derive(Debug, Clone)]
pub struct SelectStatement {
    pub body: SelectCore,
    pub unions: Vec<Union>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<LimitClause>,
}

/// One `UNION [ALL] SELECT …` arm.
#[derive(Debug, Clone)]
pub struct Union {
    /// `true` for `UNION ALL` (keep duplicates); `false` for `UNION` (distinct).
    pub all: bool,
    pub select: SelectCore,
}

/// A single `SELECT … FROM … WHERE … GROUP BY … HAVING`. ORDER BY / LIMIT live
/// on the enclosing [`SelectStatement`] so a UNION applies them once.
#[derive(Debug, Clone)]
pub struct SelectCore {
    pub distinct: bool,
    /// `SELECT TOP n` — a per-core row cap (used directly on UNION arms; the
    /// body's TOP is lifted to the statement LIMIT so it applies after ORDER BY).
    pub top: Option<usize>,
    pub columns: Vec<SelectColumn>,
    /// Seed table. `None` for `SELECT <expr>` with no FROM.
    pub from: Option<TableRef>,
    /// JOINs (and comma-separated CROSS joins) applied left-to-right after `from`.
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
}

/// A single item in the SELECT list.
#[derive(Debug, Clone)]
pub struct SelectColumn {
    pub expr: Expr,
    pub alias: Option<String>,
}

/// A table in FROM or a JOIN: either a named query variable or a derived
/// (sub-select) table.
#[derive(Debug, Clone)]
pub enum TableRef {
    /// `name [AS alias]` — `name` is a query variable resolved from CFML scope.
    Named { name: String, alias: Option<String> },
    /// `(SELECT …) AS alias` — a derived table (SQL requires the alias).
    Derived { select: Box<SelectStatement>, alias: String },
}

impl TableRef {
    /// The name this table is addressed by in column references (its alias if
    /// present, otherwise the source name).
    pub fn binding_name(&self) -> &str {
        match self {
            TableRef::Named { name, alias } => alias.as_deref().unwrap_or(name),
            TableRef::Derived { alias, .. } => alias,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: TableRef,
    /// `ON` predicate. `None` for CROSS / comma joins.
    pub on: Option<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// An ORDER BY item. `expr` may be an ordinary expression or an integer literal
/// referring to a 1-based position in the SELECT list.
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// `LIMIT count [OFFSET offset]` / `LIMIT offset, count`.
#[derive(Debug, Clone, Copy)]
pub struct LimitClause {
    pub offset: usize,
    pub count: usize,
}

/// A SQL expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// `*` or `table.*` (only valid in the SELECT list).
    Star { table: Option<String> },
    /// `column` or `table.column`.
    Column { table: Option<String>, name: String },
    /// Bind-resolved column reference: a `(table_index, column_index)` slot
    /// into the current `TableSet`. Produced by [`bind_core`] (execution.rs)
    /// from `Column` once per query (after `resolve_tables` + `expand_columns`)
    /// so per-row eval skips the linear table+column name scan. `name` is kept
    /// for `derive_column_names` and error messages.
    ResolvedColumn { ti: u32, ci: u32, name: String },
    /// A literal value (number, string, NULL, TRUE/FALSE).
    Literal(CfmlValue),
    /// `left <op> right`.
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    /// `<op> expr` (unary).
    Unary { op: UnaryOp, expr: Box<Expr> },
    /// `name(args)`. `distinct` is set for `COUNT(DISTINCT x)`.
    Function { name: String, args: Vec<Expr>, distinct: bool },
    /// `CASE [operand] WHEN … THEN … [ELSE …] END`.
    Case {
        operand: Option<Box<Expr>>,
        whens: Vec<WhenThen>,
        else_expr: Option<Box<Expr>>,
    },
    /// `CAST(expr AS ty)` / `CONVERT(expr, ty)`.
    Cast { expr: Box<Expr>, ty: String },
    /// `expr IS [NOT] NULL`.
    IsNull { expr: Box<Expr>, negated: bool },
    /// `expr [NOT] IN (e1, e2, …)`.
    InList { expr: Box<Expr>, negated: bool, list: Vec<Expr> },
    /// `expr [NOT] IN (SELECT …)`.
    InSubquery { expr: Box<Expr>, negated: bool, select: Box<SelectStatement> },
    /// `expr [NOT] BETWEEN low AND high`.
    Between { expr: Box<Expr>, negated: bool, low: Box<Expr>, high: Box<Expr> },
    /// `expr [NOT] LIKE pattern [ESCAPE ch]`.
    Like {
        expr: Box<Expr>,
        negated: bool,
        pattern: Box<Expr>,
        escape: Option<Box<Expr>>,
        /// Pre-compiled matcher when `pattern` (and `escape`, if present) is a
        /// constant literal. Populated by `bind_expr` (execution.rs) once per
        /// query so per-row `eval_like` skips recompiling. `None` for
        /// non-literal patterns (parameter / expression).
        compiled: Option<like::Compiled>,
    },
    /// `(SELECT …)` used as a scalar value (first row, first column).
    ScalarSubquery(Box<SelectStatement>),
    /// A bind parameter: positional `?` (0-based index) or named `:name`.
    Param(ParamRef),
}

#[derive(Debug, Clone)]
pub enum ParamRef {
    /// `?` — bound by 0-based order of appearance in the SQL.
    Positional(usize),
    /// `:name` — bound by name from a struct of params.
    Named(String),
}

#[derive(Debug, Clone)]
pub struct WhenThen {
    pub when: Expr,
    pub then: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Plus,
}
