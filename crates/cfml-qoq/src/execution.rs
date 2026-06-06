//! QoQ execution engine.
//!
//! A single `eval` walks each expression with a `RowCtx` discriminator —
//! `Row` for scalar (per-row) evaluation, `Group` for aggregate (per-partition)
//! evaluation — so operator logic is written once (BoxLang's dual-path
//! `evaluate` / `evaluateAggregate`, unified). The pipeline per SELECT core is:
//! resolve tables → build join intersections → WHERE → GROUP BY/HAVING (or
//! simple projection) → DISTINCT → (statement level) UNION → ORDER BY →
//! LIMIT/OFFSET.

use std::cmp::Ordering;
use std::collections::HashSet;

use cfml_common::dynamic::{CfmlQuery, CfmlValue};
use cfml_common::vm::{CfmlError, CfmlResult};
use indexmap::IndexMap;

use crate::ast::*;
use crate::compare::{compare_total, group_key, sql_equal};
use crate::function::{QoQFnKind, QoQFunctionRegistry};
use crate::functions;
use crate::intersection::build_intersections;
use crate::like::like_match;
use crate::table::{QoQTable, TableSet};

/// Bind parameters supplied to a parameterised QoQ query.
#[derive(Debug, Default)]
pub struct QoQParams {
    /// Positional `?` parameters, in order.
    pub positional: Vec<CfmlValue>,
    /// Named `:name` parameters (matched case-insensitively).
    pub named: IndexMap<String, CfmlValue>,
}

impl QoQParams {
    pub fn none() -> Self {
        Self::default()
    }

    fn lookup_named(&self, name: &str) -> Option<&CfmlValue> {
        self.named
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }
}

/// Execute a parsed statement against the supplied source queries.
///
/// `udf` invokes a CFML UDF/closure (its first arg is the `CfmlValue::Function`
/// or `Closure`); the VM supplies a closure wrapping its own call machinery.
pub fn execute(
    stmt: &Statement,
    sources: &[(String, &CfmlQuery)],
    params: &QoQParams,
    registry: &QoQFunctionRegistry,
    udf: &mut dyn FnMut(&CfmlValue, Vec<CfmlValue>) -> CfmlResult,
) -> CfmlResult {
    let Statement::Select(select) = stmt;
    let mut engine = Engine {
        sources,
        params,
        registry,
        udf,
    };
    let query = engine.run_statement(select)?;
    Ok(CfmlValue::Query(Box::new(query)))
}

/// Collect every named (query-variable) table referenced anywhere in the
/// statement — including derived-table bodies and subqueries — so the VM can
/// resolve them from CFML scope before executing. Case-insensitive, first
/// spelling wins.
pub fn base_table_names(stmt: &Statement) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let Statement::Select(s) = stmt;
    walk_select(s, &mut out, &mut seen);
    out
}

fn walk_select(s: &SelectStatement, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    walk_core(&s.body, out, seen);
    for u in &s.unions {
        walk_core(&u.select, out, seen);
    }
    for ob in &s.order_by {
        walk_expr(&ob.expr, out, seen);
    }
}

fn walk_core(c: &SelectCore, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    if let Some(t) = &c.from {
        walk_tableref(t, out, seen);
    }
    for j in &c.joins {
        walk_tableref(&j.table, out, seen);
        if let Some(on) = &j.on {
            walk_expr(on, out, seen);
        }
    }
    if let Some(w) = &c.where_clause {
        walk_expr(w, out, seen);
    }
    for g in &c.group_by {
        walk_expr(g, out, seen);
    }
    if let Some(h) = &c.having {
        walk_expr(h, out, seen);
    }
    for col in &c.columns {
        walk_expr(&col.expr, out, seen);
    }
}

fn walk_tableref(t: &TableRef, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    match t {
        TableRef::Named { name, .. } => {
            if seen.insert(name.to_lowercase()) {
                out.push(name.clone());
            }
        }
        TableRef::Derived { select, .. } => walk_select(select, out, seen),
    }
}

fn walk_expr(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    match e {
        Expr::Binary { left, right, .. } => {
            walk_expr(left, out, seen);
            walk_expr(right, out, seen);
        }
        Expr::Unary { expr, .. } => walk_expr(expr, out, seen),
        Expr::Function { args, .. } => {
            for a in args {
                walk_expr(a, out, seen);
            }
        }
        Expr::Case {
            operand,
            whens,
            else_expr,
        } => {
            if let Some(o) = operand {
                walk_expr(o, out, seen);
            }
            for w in whens {
                walk_expr(&w.when, out, seen);
                walk_expr(&w.then, out, seen);
            }
            if let Some(e) = else_expr {
                walk_expr(e, out, seen);
            }
        }
        Expr::Cast { expr, .. } => walk_expr(expr, out, seen),
        Expr::IsNull { expr, .. } => walk_expr(expr, out, seen),
        Expr::InList { expr, list, .. } => {
            walk_expr(expr, out, seen);
            for e in list {
                walk_expr(e, out, seen);
            }
        }
        Expr::InSubquery { expr, select, .. } => {
            walk_expr(expr, out, seen);
            walk_select(select, out, seen);
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            walk_expr(expr, out, seen);
            walk_expr(low, out, seen);
            walk_expr(high, out, seen);
        }
        Expr::Like {
            expr,
            pattern,
            escape,
            ..
        } => {
            walk_expr(expr, out, seen);
            walk_expr(pattern, out, seen);
            if let Some(e) = escape {
                walk_expr(e, out, seen);
            }
        }
        Expr::ScalarSubquery(select) => walk_select(select, out, seen),
        Expr::Star { .. } | Expr::Column { .. } | Expr::Literal(_) | Expr::Param(_) => {}
    }
}

/// Evaluation context: a single row (scalar) or a partition (aggregate).
#[derive(Clone, Copy)]
enum RowCtx<'b> {
    Row(&'b [usize]),
    Group(&'b [Vec<usize>]),
}

/// Result of executing one SELECT core.
struct CoreResult {
    columns: Vec<String>,
    rows: Vec<Vec<CfmlValue>>,
    /// Parallel to `rows`: the ORDER BY key values for each row (empty when no
    /// statement-level ORDER BY was supplied to the core).
    sort_keys: Vec<Vec<CfmlValue>>,
}

struct Engine<'a> {
    sources: &'a [(String, &'a CfmlQuery)],
    params: &'a QoQParams,
    registry: &'a QoQFunctionRegistry,
    udf: &'a mut dyn FnMut(&CfmlValue, Vec<CfmlValue>) -> CfmlResult,
}

impl Engine<'_> {
    // ── Statement / core ───────────────────────────────────────────────

    fn run_statement(&mut self, stmt: &SelectStatement) -> Result<CfmlQuery, CfmlError> {
        // ORDER BY keys are computed in-context only for the single-core case;
        // for UNION, ordering is by output column post-merge.
        let body_order: &[OrderByExpr] = if stmt.unions.is_empty() {
            &stmt.order_by
        } else {
            &[]
        };
        let body = self.run_core(&stmt.body, body_order)?;
        let columns = body.columns;
        let mut rows = body.rows;

        if stmt.unions.is_empty() {
            let sort_keys = body.sort_keys;
            sort_rows(&mut rows, &sort_keys, &stmt.order_by);
            apply_limit(&mut rows, &stmt.limit);
            return Ok(build_query(columns, rows));
        }

        // UNION: append each arm, then dedup if any arm is a distinct UNION.
        let any_distinct = stmt.unions.iter().any(|u| !u.all);
        for u in &stmt.unions {
            let arm = self.run_core(&u.select, &[])?;
            if arm.columns.len() != columns.len() {
                return Err(CfmlError::runtime(format!(
                    "Query of Queries: UNION column count mismatch ({} vs {})",
                    columns.len(),
                    arm.columns.len()
                )));
            }
            rows.extend(arm.rows);
        }
        if any_distinct {
            dedup_rows(&mut rows);
        }
        self.order_by_output(&mut rows, &columns, &stmt.order_by)?;
        apply_limit(&mut rows, &stmt.limit);
        Ok(build_query(columns, rows))
    }

    fn run_core(
        &mut self,
        core: &SelectCore,
        order_by: &[OrderByExpr],
    ) -> Result<CoreResult, CfmlError> {
        let tables = self.resolve_tables(core)?;

        if tables.is_empty() {
            return self.run_no_from(core, order_by);
        }

        let select_cols = expand_columns(&core.columns, &tables)?;
        let columns = derive_column_names(&select_cols);

        let intersections = self.build_core_intersections(core, &tables)?;
        let filtered = self.filter_where(intersections, &core.where_clause, &tables)?;

        let has_agg = select_cols.iter().any(|c| expr_has_aggregate(&c.expr, self.registry))
            || core
                .having
                .as_ref()
                .map(|h| expr_has_aggregate(h, self.registry))
                .unwrap_or(false);

        let (mut rows, mut sort_keys) = if has_agg || !core.group_by.is_empty() {
            self.exec_aggregate(core, &select_cols, &filtered, &tables, order_by)?
        } else {
            self.exec_simple(&select_cols, &filtered, &tables, order_by)?
        };

        if core.distinct {
            dedup_rows_and_keys(&mut rows, &mut sort_keys);
        }
        if let Some(n) = core.top {
            if rows.len() > n {
                rows.truncate(n);
                sort_keys.truncate(n);
            }
        }

        Ok(CoreResult {
            columns,
            rows,
            sort_keys,
        })
    }

    fn run_no_from(
        &mut self,
        core: &SelectCore,
        order_by: &[OrderByExpr],
    ) -> Result<CoreResult, CfmlError> {
        let tables = TableSet::new();
        let inter: Vec<usize> = Vec::new();
        let mut row = Vec::with_capacity(core.columns.len());
        for sc in &core.columns {
            if matches!(sc.expr, Expr::Star { .. }) {
                return Err(CfmlError::runtime(
                    "Query of Queries: '*' requires a FROM clause".to_string(),
                ));
            }
            row.push(self.eval(&sc.expr, &tables, RowCtx::Row(&inter))?);
        }
        let columns = derive_column_names(&core.columns);
        let mut key = Vec::with_capacity(order_by.len());
        for ob in order_by {
            key.push(self.order_key(ob, &tables, RowCtx::Row(&inter), &row)?);
        }
        Ok(CoreResult {
            columns,
            rows: vec![row],
            sort_keys: vec![key],
        })
    }

    fn resolve_tables(&mut self, core: &SelectCore) -> Result<TableSet, CfmlError> {
        let mut ts = TableSet::new();
        if let Some(from) = &core.from {
            ts.add(self.resolve_table_ref(from)?);
            for j in &core.joins {
                ts.add(self.resolve_table_ref(&j.table)?);
            }
        }
        Ok(ts)
    }

    fn resolve_table_ref(&mut self, tref: &TableRef) -> Result<QoQTable, CfmlError> {
        match tref {
            TableRef::Named { name, alias } => {
                let query = self
                    .sources
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, q)| *q)
                    .ok_or_else(|| {
                        CfmlError::runtime(format!(
                            "Query of Queries: table '{}' not found (no such query variable)",
                            name
                        ))
                    })?;
                let binding = alias.clone().unwrap_or_else(|| name.clone());
                Ok(QoQTable::from_query(&binding, query))
            }
            TableRef::Derived { select, alias } => {
                let q = self.run_statement(select)?;
                Ok(QoQTable::from_query(alias, &q))
            }
        }
    }

    fn build_core_intersections(
        &mut self,
        core: &SelectCore,
        tables: &TableSet,
    ) -> Result<Vec<Vec<usize>>, CfmlError> {
        let row_counts = tables.row_counts();
        let join_types: Vec<JoinType> = core.joins.iter().map(|j| j.join_type).collect();
        let joins = &core.joins;
        let mut on_err: Option<CfmlError> = None;

        let inters = build_intersections(&row_counts, &join_types, |k, cand| {
            if on_err.is_some() {
                return false;
            }
            match &joins[k].on {
                None => true, // CROSS / comma join
                Some(expr) => match self.eval(expr, tables, RowCtx::Row(cand)) {
                    Ok(v) => is_truthy(&v),
                    Err(e) => {
                        on_err = Some(e);
                        false
                    }
                },
            }
        });

        match on_err {
            Some(e) => Err(e),
            None => Ok(inters),
        }
    }

    fn filter_where(
        &mut self,
        intersections: Vec<Vec<usize>>,
        where_clause: &Option<Expr>,
        tables: &TableSet,
    ) -> Result<Vec<Vec<usize>>, CfmlError> {
        let Some(expr) = where_clause else {
            return Ok(intersections);
        };
        let mut out = Vec::new();
        for inter in intersections {
            let v = self.eval(expr, tables, RowCtx::Row(&inter))?;
            if is_truthy(&v) {
                out.push(inter);
            }
        }
        Ok(out)
    }

    fn exec_simple(
        &mut self,
        select_cols: &[SelectColumn],
        intersections: &[Vec<usize>],
        tables: &TableSet,
        order_by: &[OrderByExpr],
    ) -> Result<(Vec<Vec<CfmlValue>>, Vec<Vec<CfmlValue>>), CfmlError> {
        let mut rows = Vec::with_capacity(intersections.len());
        let mut keys = Vec::with_capacity(intersections.len());
        for inter in intersections {
            let mut row = Vec::with_capacity(select_cols.len());
            for sc in select_cols {
                row.push(self.eval(&sc.expr, tables, RowCtx::Row(inter))?);
            }
            let mut key = Vec::with_capacity(order_by.len());
            for ob in order_by {
                key.push(self.order_key(ob, tables, RowCtx::Row(inter), &row)?);
            }
            rows.push(row);
            keys.push(key);
        }
        Ok((rows, keys))
    }

    fn exec_aggregate(
        &mut self,
        core: &SelectCore,
        select_cols: &[SelectColumn],
        intersections: &[Vec<usize>],
        tables: &TableSet,
        order_by: &[OrderByExpr],
    ) -> Result<(Vec<Vec<CfmlValue>>, Vec<Vec<CfmlValue>>), CfmlError> {
        let partitions: Vec<Vec<Vec<usize>>> = if core.group_by.is_empty() {
            // Pure aggregate (no GROUP BY): one partition over all rows — even
            // if empty, which still yields one row (COUNT = 0, etc.).
            vec![intersections.to_vec()]
        } else {
            self.partition(intersections, &core.group_by, tables)?
        };

        let mut rows = Vec::new();
        let mut keys = Vec::new();
        for part in &partitions {
            if let Some(h) = &core.having {
                let v = self.eval(h, tables, RowCtx::Group(part))?;
                if !is_truthy(&v) {
                    continue;
                }
            }
            let mut row = Vec::with_capacity(select_cols.len());
            for sc in select_cols {
                row.push(self.eval(&sc.expr, tables, RowCtx::Group(part))?);
            }
            let mut key = Vec::with_capacity(order_by.len());
            for ob in order_by {
                key.push(self.order_key(ob, tables, RowCtx::Group(part), &row)?);
            }
            rows.push(row);
            keys.push(key);
        }
        Ok((rows, keys))
    }

    fn partition(
        &mut self,
        intersections: &[Vec<usize>],
        group_by: &[Expr],
        tables: &TableSet,
    ) -> Result<Vec<Vec<Vec<usize>>>, CfmlError> {
        let mut groups: IndexMap<String, Vec<Vec<usize>>> = IndexMap::new();
        for inter in intersections {
            let mut key_vals = Vec::with_capacity(group_by.len());
            for g in group_by {
                key_vals.push(self.eval(g, tables, RowCtx::Row(inter))?);
            }
            let k = group_key(&key_vals);
            groups.entry(k).or_default().push(inter.clone());
        }
        Ok(groups.into_values().collect())
    }

    /// The sort-key value for one ORDER BY item. A bare integer literal is a
    /// 1-based reference into the projected row; otherwise evaluate the expr.
    fn order_key(
        &mut self,
        ob: &OrderByExpr,
        tables: &TableSet,
        ctx: RowCtx,
        projected_row: &[CfmlValue],
    ) -> CfmlResult {
        if let Expr::Literal(CfmlValue::Int(n)) = &ob.expr {
            let idx = (*n - 1).max(0) as usize;
            return Ok(projected_row.get(idx).cloned().unwrap_or(CfmlValue::Null));
        }
        self.eval(&ob.expr, tables, ctx)
    }

    /// Order a UNION's merged rows by output column (position / name / expr over
    /// the output columns).
    fn order_by_output(
        &mut self,
        rows: &mut Vec<Vec<CfmlValue>>,
        columns: &[String],
        order_by: &[OrderByExpr],
    ) -> Result<(), CfmlError> {
        if order_by.is_empty() || rows.len() < 2 {
            return Ok(());
        }
        let mut keys: Vec<Vec<CfmlValue>> = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            // Synthetic single-row table over the output columns.
            let tbl = QoQTable {
                name: String::new(),
                columns: columns.to_vec(),
                data: row.iter().map(|v| vec![v.clone()]).collect(),
                row_count: 1,
            };
            let ts = TableSet { tables: vec![tbl] };
            let inter = vec![1usize];
            let mut key = Vec::with_capacity(order_by.len());
            for ob in order_by {
                key.push(self.order_key(ob, &ts, RowCtx::Row(&inter), row)?);
            }
            keys.push(key);
        }
        sort_rows(rows, &keys, order_by);
        Ok(())
    }

    // ── Expression evaluation (dual-path via RowCtx) ────────────────────

    fn eval(&mut self, expr: &Expr, tables: &TableSet, ctx: RowCtx) -> CfmlResult {
        match expr {
            Expr::Literal(v) => Ok(v.clone()),

            Expr::Star { .. } => Err(CfmlError::runtime(
                "Query of Queries: '*' is only valid in a SELECT list".to_string(),
            )),

            Expr::Column { table, name } => self.eval_column(table.as_deref(), name, tables, ctx),

            Expr::Param(p) => self.eval_param(p),

            Expr::Function { name, args, distinct } => {
                if is_aggregate(name, self.registry) {
                    return self.eval_aggregate_call(name, args, *distinct, tables, ctx);
                }
                let mut evaled = Vec::with_capacity(args.len());
                for a in args {
                    evaled.push(self.eval(a, tables, ctx)?);
                }
                self.call_scalar_fn(name, evaled)
            }

            Expr::Cast { expr, ty } => {
                let v = self.eval(expr, tables, ctx)?;
                functions::cast_value(&v, ty)
            }

            Expr::Unary { op, expr } => {
                let v = self.eval(expr, tables, ctx)?;
                apply_unary(*op, &v)
            }

            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right, tables, ctx),

            Expr::Case {
                operand,
                whens,
                else_expr,
            } => self.eval_case(operand.as_deref(), whens, else_expr.as_deref(), tables, ctx),

            Expr::IsNull { expr, negated } => {
                let v = self.eval(expr, tables, ctx)?;
                let is_null = matches!(v, CfmlValue::Null);
                Ok(CfmlValue::Bool(is_null ^ negated))
            }

            Expr::InList {
                expr,
                negated,
                list,
            } => self.eval_in_list(expr, *negated, list, tables, ctx),

            Expr::InSubquery {
                expr,
                negated,
                select,
            } => self.eval_in_subquery(expr, *negated, select, tables, ctx),

            Expr::Between {
                expr,
                negated,
                low,
                high,
            } => self.eval_between(expr, *negated, low, high, tables, ctx),

            Expr::Like {
                expr,
                negated,
                pattern,
                escape,
            } => self.eval_like(expr, *negated, pattern, escape.as_deref(), tables, ctx),

            Expr::ScalarSubquery(select) => self.eval_scalar_subquery(select),
        }
    }

    fn eval_column(
        &self,
        table: Option<&str>,
        name: &str,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        match ctx {
            RowCtx::Row(inter) => self.column_value(table, name, tables, inter),
            RowCtx::Group(part) => match part.first() {
                Some(inter) => self.column_value(table, name, tables, inter),
                None => Ok(CfmlValue::Null),
            },
        }
    }

    fn column_value(
        &self,
        table: Option<&str>,
        name: &str,
        tables: &TableSet,
        inter: &[usize],
    ) -> CfmlResult {
        match tables.resolve_column(table, name) {
            Some((ti, ci)) => Ok(tables.value(inter, ti, ci)),
            None => Err(CfmlError::runtime(format!(
                "Query of Queries: column '{}{}' not found",
                table.map(|t| format!("{}.", t)).unwrap_or_default(),
                name
            ))),
        }
    }

    fn eval_param(&self, p: &ParamRef) -> CfmlResult {
        match p {
            ParamRef::Positional(i) => self.params.positional.get(*i).cloned().ok_or_else(|| {
                CfmlError::runtime(format!(
                    "Query of Queries: positional parameter #{} has no value",
                    i + 1
                ))
            }),
            ParamRef::Named(n) => self.params.lookup_named(n).cloned().ok_or_else(|| {
                CfmlError::runtime(format!("Query of Queries: named parameter ':{}' has no value", n))
            }),
        }
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        // Short-circuit logical operators (also keeps 3-valued logic correct).
        match op {
            BinaryOp::And => {
                let l = self.eval(left, tables, ctx)?;
                if tri(&l) == Some(false) {
                    return Ok(CfmlValue::Bool(false));
                }
                let r = self.eval(right, tables, ctx)?;
                return Ok(and3(tri(&l), tri(&r)));
            }
            BinaryOp::Or => {
                let l = self.eval(left, tables, ctx)?;
                if tri(&l) == Some(true) {
                    return Ok(CfmlValue::Bool(true));
                }
                let r = self.eval(right, tables, ctx)?;
                return Ok(or3(tri(&l), tri(&r)));
            }
            _ => {}
        }
        let l = self.eval(left, tables, ctx)?;
        let r = self.eval(right, tables, ctx)?;
        apply_binary(&l, op, &r)
    }

    fn eval_case(
        &mut self,
        operand: Option<&Expr>,
        whens: &[WhenThen],
        else_expr: Option<&Expr>,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        let operand_val = match operand {
            Some(o) => Some(self.eval(o, tables, ctx)?),
            None => None,
        };
        for wt in whens {
            let matched = match &operand_val {
                // Simple CASE: operand = when-value.
                Some(ov) => {
                    let wv = self.eval(&wt.when, tables, ctx)?;
                    sql_equal(ov, &wv) == Some(true)
                }
                // Searched CASE: when is a boolean predicate.
                None => is_truthy(&self.eval(&wt.when, tables, ctx)?),
            };
            if matched {
                return self.eval(&wt.then, tables, ctx);
            }
        }
        match else_expr {
            Some(e) => self.eval(e, tables, ctx),
            None => Ok(CfmlValue::Null),
        }
    }

    fn eval_in_list(
        &mut self,
        expr: &Expr,
        negated: bool,
        list: &[Expr],
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        let target = self.eval(expr, tables, ctx)?;
        if matches!(target, CfmlValue::Null) {
            return Ok(CfmlValue::Null);
        }
        let mut found_null = false;
        for item in list {
            let v = self.eval(item, tables, ctx)?;
            match sql_equal(&target, &v) {
                Some(true) => return Ok(CfmlValue::Bool(!negated)),
                Some(false) => {}
                None => found_null = true,
            }
        }
        Ok(membership_result(false, found_null, negated))
    }

    fn eval_in_subquery(
        &mut self,
        expr: &Expr,
        negated: bool,
        select: &SelectStatement,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        let target = self.eval(expr, tables, ctx)?;
        if matches!(target, CfmlValue::Null) {
            return Ok(CfmlValue::Null);
        }
        let values = self.subquery_first_column(select)?;
        let mut found_null = false;
        for v in &values {
            match sql_equal(&target, v) {
                Some(true) => return Ok(CfmlValue::Bool(!negated)),
                Some(false) => {}
                None => found_null = true,
            }
        }
        Ok(membership_result(false, found_null, negated))
    }

    fn eval_between(
        &mut self,
        expr: &Expr,
        negated: bool,
        low: &Expr,
        high: &Expr,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        let v = self.eval(expr, tables, ctx)?;
        let lo = self.eval(low, tables, ctx)?;
        let hi = self.eval(high, tables, ctx)?;
        let (Some(c_lo), Some(c_hi)) = (
            crate::compare::compare_sql(&v, &lo),
            crate::compare::compare_sql(&v, &hi),
        ) else {
            return Ok(CfmlValue::Null);
        };
        let in_range = c_lo != Ordering::Less && c_hi != Ordering::Greater;
        Ok(CfmlValue::Bool(in_range ^ negated))
    }

    fn eval_like(
        &mut self,
        expr: &Expr,
        negated: bool,
        pattern: &Expr,
        escape: Option<&Expr>,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        let v = self.eval(expr, tables, ctx)?;
        let p = self.eval(pattern, tables, ctx)?;
        if matches!(v, CfmlValue::Null) || matches!(p, CfmlValue::Null) {
            return Ok(CfmlValue::Null);
        }
        let esc = match escape {
            Some(e) => self.eval(e, tables, ctx)?.as_string().chars().next(),
            None => None,
        };
        let matched = like_match(&v.as_string(), &p.as_string(), esc);
        Ok(CfmlValue::Bool(matched ^ negated))
    }

    fn eval_scalar_subquery(&mut self, select: &SelectStatement) -> CfmlResult {
        let q = self.run_statement(select)?;
        let Some(first_col) = q.columns.first() else {
            return Ok(CfmlValue::Null);
        };
        Ok(q
            .rows
            .first()
            .and_then(|r| r.get(first_col).cloned())
            .unwrap_or(CfmlValue::Null))
    }

    fn subquery_first_column(
        &mut self,
        select: &SelectStatement,
    ) -> Result<Vec<CfmlValue>, CfmlError> {
        let q = self.run_statement(select)?;
        let Some(first_col) = q.columns.first() else {
            return Ok(Vec::new());
        };
        Ok(q
            .rows
            .iter()
            .map(|r| r.get(first_col).cloned().unwrap_or(CfmlValue::Null))
            .collect())
    }

    // ── Function dispatch ───────────────────────────────────────────────

    fn call_scalar_fn(&mut self, name: &str, args: Vec<CfmlValue>) -> CfmlResult {
        // 1. built-in scalar
        if let Some(r) = functions::call_scalar(name, &args) {
            return r;
        }
        // 2. native scalar registered from Rust
        if let Some((QoQFnKind::Scalar, f)) = self.registry.get_native(name) {
            return f(args);
        }
        // 3. custom CFML UDF (scalar)
        if let Some((cfval, QoQFnKind::Scalar)) = self.registry.get_custom(name).cloned() {
            return (self.udf)(&cfval, args);
        }
        Err(CfmlError::runtime(format!(
            "Query of Queries: unknown function '{}'",
            name
        )))
    }

    fn eval_aggregate_call(
        &mut self,
        name: &str,
        args: &[Expr],
        distinct: bool,
        tables: &TableSet,
        ctx: RowCtx,
    ) -> CfmlResult {
        // Aggregates need a partition. In a Row context (no grouping) treat the
        // single row as a one-element partition.
        let single;
        let partition: &[Vec<usize>] = match ctx {
            RowCtx::Group(p) => p,
            RowCtx::Row(r) => {
                single = vec![r.to_vec()];
                &single
            }
        };

        let lname = name.to_lowercase();

        // COUNT is special: COUNT(*) counts rows; COUNT(x) counts non-null.
        if lname == "count" {
            let is_star = args.is_empty() || matches!(args.first(), Some(Expr::Star { .. }));
            if is_star {
                return Ok(CfmlValue::Int(partition.len() as i64));
            }
            let mut vals = self.collect_arg(&args[0], tables, partition)?;
            vals.retain(|v| !matches!(v, CfmlValue::Null));
            if distinct {
                dedup_values(&mut vals);
            }
            return Ok(CfmlValue::Int(vals.len() as i64));
        }

        if is_builtin_aggregate(&lname) {
            let arg0 = args.first().ok_or_else(|| {
                CfmlError::runtime(format!("Query of Queries: {} requires an argument", lname))
            })?;
            let mut col = self.collect_arg(arg0, tables, partition)?;
            if distinct {
                let mut nonnull: Vec<CfmlValue> =
                    col.into_iter().filter(|v| !matches!(v, CfmlValue::Null)).collect();
                dedup_values(&mut nonnull);
                col = nonnull;
            }
            if lname == "group_concat" || lname == "string_agg" {
                let sep = match args.get(1) {
                    Some(e) => self.eval(e, tables, RowCtx::Row(&partition[0.min(partition.len().saturating_sub(1))]))?.as_string(),
                    None => ",".to_string(),
                };
                let parts: Vec<String> = col
                    .iter()
                    .filter(|v| !matches!(v, CfmlValue::Null))
                    .map(|v| v.as_string())
                    .collect();
                if parts.is_empty() {
                    return Ok(CfmlValue::Null);
                }
                return Ok(CfmlValue::String(parts.join(&sep)));
            }
            return Ok(aggregate_numeric(&lname, &col));
        }

        // Native / custom aggregate: pass each arg as a column array.
        let mut arrays = Vec::with_capacity(args.len());
        for a in args {
            let col = self.collect_arg(a, tables, partition)?;
            arrays.push(CfmlValue::array(col));
        }
        if let Some((QoQFnKind::Aggregate, f)) = self.registry.get_native(&lname) {
            return f(arrays);
        }
        if let Some((cfval, QoQFnKind::Aggregate)) = self.registry.get_custom(&lname).cloned() {
            return (self.udf)(&cfval, arrays);
        }
        Err(CfmlError::runtime(format!(
            "Query of Queries: unknown aggregate function '{}'",
            name
        )))
    }

    fn collect_arg(
        &mut self,
        arg: &Expr,
        tables: &TableSet,
        partition: &[Vec<usize>],
    ) -> Result<Vec<CfmlValue>, CfmlError> {
        let mut out = Vec::with_capacity(partition.len());
        for inter in partition {
            out.push(self.eval(arg, tables, RowCtx::Row(inter))?);
        }
        Ok(out)
    }
}

// ── Aggregate helpers ───────────────────────────────────────────────────

fn is_builtin_aggregate(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "count" | "sum" | "avg" | "min" | "max" | "group_concat" | "string_agg"
    )
}

fn is_aggregate(name: &str, registry: &QoQFunctionRegistry) -> bool {
    is_builtin_aggregate(name) || registry.is_aggregate(name)
}

fn expr_has_aggregate(expr: &Expr, registry: &QoQFunctionRegistry) -> bool {
    match expr {
        Expr::Function { name, args, .. } => {
            is_aggregate(name, registry) || args.iter().any(|a| expr_has_aggregate(a, registry))
        }
        Expr::Binary { left, right, .. } => {
            expr_has_aggregate(left, registry) || expr_has_aggregate(right, registry)
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::IsNull { expr, .. } => {
            expr_has_aggregate(expr, registry)
        }
        Expr::Case {
            operand,
            whens,
            else_expr,
        } => {
            operand.as_ref().map(|e| expr_has_aggregate(e, registry)).unwrap_or(false)
                || whens.iter().any(|w| {
                    expr_has_aggregate(&w.when, registry) || expr_has_aggregate(&w.then, registry)
                })
                || else_expr.as_ref().map(|e| expr_has_aggregate(e, registry)).unwrap_or(false)
        }
        Expr::InList { expr, list, .. } => {
            expr_has_aggregate(expr, registry) || list.iter().any(|e| expr_has_aggregate(e, registry))
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            expr_has_aggregate(expr, registry)
                || expr_has_aggregate(low, registry)
                || expr_has_aggregate(high, registry)
        }
        Expr::Like { expr, pattern, .. } => {
            expr_has_aggregate(expr, registry) || expr_has_aggregate(pattern, registry)
        }
        // Subqueries are their own scope; their aggregates don't make the outer
        // query aggregate.
        _ => false,
    }
}

/// SUM / AVG / MIN / MAX over a column of values (NULLs ignored).
fn aggregate_numeric(name: &str, col: &[CfmlValue]) -> CfmlValue {
    match name {
        "sum" => {
            let mut sum = 0.0f64;
            let mut all_int = true;
            let mut any = false;
            for v in col {
                if matches!(v, CfmlValue::Null) {
                    continue;
                }
                if let Some(n) = functions::to_f64(v) {
                    any = true;
                    if !matches!(v, CfmlValue::Int(_)) {
                        all_int = false;
                    }
                    sum += n;
                }
            }
            if !any {
                CfmlValue::Null
            } else if all_int && sum.fract() == 0.0 && sum.abs() < 9.0e15 {
                CfmlValue::Int(sum as i64)
            } else {
                CfmlValue::Double(sum)
            }
        }
        "avg" => {
            let mut sum = 0.0f64;
            let mut count = 0i64;
            for v in col {
                if let Some(n) = functions::to_f64(v) {
                    if !matches!(v, CfmlValue::Null) {
                        sum += n;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                CfmlValue::Null
            } else {
                CfmlValue::Double(sum / count as f64)
            }
        }
        "min" | "max" => {
            let want_min = name == "min";
            let mut best: Option<&CfmlValue> = None;
            for v in col {
                if matches!(v, CfmlValue::Null) {
                    continue;
                }
                best = Some(match best {
                    None => v,
                    Some(cur) => {
                        let ord = compare_total(v, cur);
                        if (want_min && ord == Ordering::Less)
                            || (!want_min && ord == Ordering::Greater)
                        {
                            v
                        } else {
                            cur
                        }
                    }
                });
            }
            best.cloned().unwrap_or(CfmlValue::Null)
        }
        _ => CfmlValue::Null,
    }
}

// ── Boolean / operator helpers ──────────────────────────────────────────

/// Three-valued truth of a value: `None` = unknown (NULL).
fn tri(v: &CfmlValue) -> Option<bool> {
    match v {
        CfmlValue::Null => None,
        CfmlValue::Bool(b) => Some(*b),
        other => Some(other.is_true()),
    }
}

/// `true` only when definitely true (NULL / unknown → false). For WHERE, ON,
/// HAVING and CASE conditions.
fn is_truthy(v: &CfmlValue) -> bool {
    tri(v) == Some(true)
}

fn and3(l: Option<bool>, r: Option<bool>) -> CfmlValue {
    match (l, r) {
        (Some(false), _) | (_, Some(false)) => CfmlValue::Bool(false),
        (Some(true), Some(true)) => CfmlValue::Bool(true),
        _ => CfmlValue::Null,
    }
}

fn or3(l: Option<bool>, r: Option<bool>) -> CfmlValue {
    match (l, r) {
        (Some(true), _) | (_, Some(true)) => CfmlValue::Bool(true),
        (Some(false), Some(false)) => CfmlValue::Bool(false),
        _ => CfmlValue::Null,
    }
}

fn membership_result(found_true: bool, found_null: bool, negated: bool) -> CfmlValue {
    let base = if found_true {
        Some(true)
    } else if found_null {
        None
    } else {
        Some(false)
    };
    match base {
        Some(b) => CfmlValue::Bool(b ^ negated),
        None => CfmlValue::Null,
    }
}

fn apply_unary(op: UnaryOp, v: &CfmlValue) -> CfmlResult {
    match op {
        UnaryOp::Not => Ok(match tri(v) {
            Some(b) => CfmlValue::Bool(!b),
            None => CfmlValue::Null,
        }),
        UnaryOp::Neg => {
            if matches!(v, CfmlValue::Null) {
                return Ok(CfmlValue::Null);
            }
            match v {
                CfmlValue::Int(i) => Ok(CfmlValue::Int(-*i)),
                _ => match functions::to_f64(v) {
                    Some(n) => Ok(CfmlValue::Double(-n)),
                    None => Err(CfmlError::runtime(
                        "Query of Queries: cannot negate a non-numeric value".to_string(),
                    )),
                },
            }
        }
        UnaryOp::Plus => Ok(v.clone()),
    }
}

fn apply_binary(l: &CfmlValue, op: BinaryOp, r: &CfmlValue) -> CfmlResult {
    use crate::compare::compare_sql;
    match op {
        BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte => {
            match compare_sql(l, r) {
                None => Ok(CfmlValue::Null),
                Some(ord) => {
                    let res = match op {
                        BinaryOp::Eq => ord == Ordering::Equal,
                        BinaryOp::Neq => ord != Ordering::Equal,
                        BinaryOp::Lt => ord == Ordering::Less,
                        BinaryOp::Lte => ord != Ordering::Greater,
                        BinaryOp::Gt => ord == Ordering::Greater,
                        BinaryOp::Gte => ord != Ordering::Less,
                        _ => unreachable!(),
                    };
                    Ok(CfmlValue::Bool(res))
                }
            }
        }
        BinaryOp::And => Ok(and3(tri(l), tri(r))),
        BinaryOp::Or => Ok(or3(tri(l), tri(r))),
        BinaryOp::Concat => {
            if matches!(l, CfmlValue::Null) || matches!(r, CfmlValue::Null) {
                Ok(CfmlValue::Null)
            } else {
                Ok(CfmlValue::String(l.as_string() + &r.as_string()))
            }
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            arith(l, op, r)
        }
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
            if matches!(l, CfmlValue::Null) || matches!(r, CfmlValue::Null) {
                return Ok(CfmlValue::Null);
            }
            let (a, b) = (
                functions::to_i64(l).unwrap_or(0),
                functions::to_i64(r).unwrap_or(0),
            );
            Ok(CfmlValue::Int(match op {
                BinaryOp::BitAnd => a & b,
                BinaryOp::BitOr => a | b,
                BinaryOp::BitXor => a ^ b,
                _ => unreachable!(),
            }))
        }
    }
}

fn arith(l: &CfmlValue, op: BinaryOp, r: &CfmlValue) -> CfmlResult {
    if matches!(l, CfmlValue::Null) || matches!(r, CfmlValue::Null) {
        return Ok(CfmlValue::Null);
    }
    // Integer-preserving for + - * % when both sides are integers.
    if let (CfmlValue::Int(a), CfmlValue::Int(b)) = (l, r) {
        match op {
            BinaryOp::Add => {
                if let Some(v) = a.checked_add(*b) {
                    return Ok(CfmlValue::Int(v));
                }
            }
            BinaryOp::Sub => {
                if let Some(v) = a.checked_sub(*b) {
                    return Ok(CfmlValue::Int(v));
                }
            }
            BinaryOp::Mul => {
                if let Some(v) = a.checked_mul(*b) {
                    return Ok(CfmlValue::Int(v));
                }
            }
            BinaryOp::Mod => {
                if *b == 0 {
                    return Err(CfmlError::runtime("Query of Queries: modulo by zero".to_string()));
                }
                return Ok(CfmlValue::Int(a % b));
            }
            _ => {}
        }
    }
    let a = functions::to_f64(l).ok_or_else(|| non_numeric(l))?;
    let b = functions::to_f64(r).ok_or_else(|| non_numeric(r))?;
    let res = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => {
            if b == 0.0 {
                return Err(CfmlError::runtime("Query of Queries: division by zero".to_string()));
            }
            a / b
        }
        BinaryOp::Mod => {
            if b == 0.0 {
                return Err(CfmlError::runtime("Query of Queries: modulo by zero".to_string()));
            }
            a % b
        }
        _ => unreachable!(),
    };
    // Render an integral double result as Int (preserve native types).
    if res.fract() == 0.0 && res.abs() < 9.0e15 && op != BinaryOp::Div {
        Ok(CfmlValue::Int(res as i64))
    } else {
        Ok(CfmlValue::Double(res))
    }
}

fn non_numeric(v: &CfmlValue) -> CfmlError {
    CfmlError::runtime(format!(
        "Query of Queries: '{}' is not numeric in an arithmetic expression",
        v.as_string()
    ))
}

// ── Projection / column-name helpers ────────────────────────────────────

/// Expand `*` and `table.*` into explicit column references.
fn expand_columns(
    columns: &[SelectColumn],
    tables: &TableSet,
) -> Result<Vec<SelectColumn>, CfmlError> {
    let mut out = Vec::new();
    for sc in columns {
        match &sc.expr {
            Expr::Star { table: None } => {
                for t in &tables.tables {
                    for col in &t.columns {
                        out.push(SelectColumn {
                            expr: Expr::Column {
                                table: Some(t.name.clone()),
                                name: col.clone(),
                            },
                            alias: Some(col.clone()),
                        });
                    }
                }
            }
            Expr::Star { table: Some(tn) } => {
                let t = tables
                    .tables
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(tn))
                    .ok_or_else(|| {
                        CfmlError::runtime(format!("Query of Queries: unknown table '{}' in {}.*", tn, tn))
                    })?;
                for col in &t.columns {
                    out.push(SelectColumn {
                        expr: Expr::Column {
                            table: Some(t.name.clone()),
                            name: col.clone(),
                        },
                        alias: Some(col.clone()),
                    });
                }
            }
            _ => out.push(sc.clone()),
        }
    }
    Ok(out)
}

/// Output column names: explicit alias, else the column name, else a generated
/// `column_N`. Names are made unique (case-insensitively) by suffixing.
fn derive_column_names(columns: &[SelectColumn]) -> Vec<String> {
    let mut names = Vec::with_capacity(columns.len());
    let mut seen: HashSet<String> = HashSet::new();
    for (i, sc) in columns.iter().enumerate() {
        let base = match (&sc.alias, &sc.expr) {
            (Some(a), _) => a.clone(),
            (None, Expr::Column { name, .. }) => name.clone(),
            (None, Expr::Function { name, .. }) => name.clone(),
            _ => format!("column_{}", i),
        };
        let mut name = base.clone();
        let mut n = 1;
        while !seen.insert(name.to_lowercase()) {
            n += 1;
            name = format!("{}_{}", base, n);
        }
        names.push(name);
    }
    names
}

fn build_query(columns: Vec<String>, rows: Vec<Vec<CfmlValue>>) -> CfmlQuery {
    let mut q = CfmlQuery::new(columns.clone());
    for row in rows {
        let mut map = IndexMap::with_capacity(columns.len());
        for (i, c) in columns.iter().enumerate() {
            map.insert(c.clone(), row.get(i).cloned().unwrap_or(CfmlValue::Null));
        }
        q.rows.push(map);
    }
    q
}

// ── DISTINCT / ORDER BY / LIMIT ─────────────────────────────────────────

fn dedup_values(values: &mut Vec<CfmlValue>) {
    let mut seen = HashSet::new();
    values.retain(|v| seen.insert(group_key(std::slice::from_ref(v))));
}

fn dedup_rows(rows: &mut Vec<Vec<CfmlValue>>) {
    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert(group_key(row)));
}

fn dedup_rows_and_keys(rows: &mut Vec<Vec<CfmlValue>>, keys: &mut Vec<Vec<CfmlValue>>) {
    let mut seen = HashSet::new();
    let mut i = 0;
    let mut keep = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        keep.push(seen.insert(group_key(row)));
    }
    rows.retain(|_| {
        let k = keep[i];
        i += 1;
        k
    });
    let mut j = 0;
    keys.retain(|_| {
        let k = keep[j];
        j += 1;
        k
    });
}

/// Above this many rows, ORDER BY uses a parallel (stable) sort on non-wasm
/// targets. The comparator is pure (no VM callback), so it parallelises safely;
/// WHERE/projection stay sequential because they may invoke a CFML UDF through
/// the VM's non-thread-safe callback.
#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_SORT_THRESHOLD: usize = 2048;

fn sort_rows(rows: &mut Vec<Vec<CfmlValue>>, keys: &[Vec<CfmlValue>], order_by: &[OrderByExpr]) {
    if order_by.is_empty() || rows.len() < 2 {
        return;
    }
    let null = CfmlValue::Null;
    let dirs: Vec<SortDirection> = order_by.iter().map(|o| o.direction).collect();
    let cmp = |&a: &usize, &b: &usize| -> Ordering {
        for (c, dir) in dirs.iter().enumerate() {
            let ka = keys[a].get(c).unwrap_or(&null);
            let kb = keys[b].get(c).unwrap_or(&null);
            let mut ord = compare_total(ka, kb);
            if *dir == SortDirection::Desc {
                ord = ord.reverse();
            }
            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    };

    let mut idx: Vec<usize> = (0..rows.len()).collect();
    #[cfg(not(target_arch = "wasm32"))]
    {
        if idx.len() >= PARALLEL_SORT_THRESHOLD {
            use rayon::slice::ParallelSliceMut;
            idx.par_sort_by(cmp); // stable parallel sort
        } else {
            idx.sort_by(cmp);
        }
    }
    #[cfg(target_arch = "wasm32")]
    idx.sort_by(cmp);

    *rows = idx.iter().map(|&i| rows[i].clone()).collect();
}

fn apply_limit(rows: &mut Vec<Vec<CfmlValue>>, limit: &Option<LimitClause>) {
    if let Some(l) = limit {
        let len = rows.len();
        let start = l.offset.min(len);
        let end = l.offset.saturating_add(l.count).min(len);
        *rows = rows[start..end].to_vec();
    }
}
