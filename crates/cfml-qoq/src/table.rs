//! Column-major source tables for QoQ execution.
//!
//! Row indices in intersections are **1-based**, with **0 reserved as the NULL
//! sentinel** for outer joins (a missing row on one side of a LEFT/RIGHT/FULL
//! join). `get(0, _)` therefore returns `Null`.

use cfml_common::dynamic::{CfmlQueryData, CfmlValue};

/// A source table converted to column-major layout for O(1) cell access.
#[derive(Debug, Clone)]
pub struct QoQTable {
    /// Name this table is addressed by in column refs (alias or source name).
    pub name: String,
    /// Column names, in order.
    pub columns: Vec<String>,
    /// Column-major data: `data[col_idx][row_idx0]`.
    pub data: Vec<Vec<CfmlValue>>,
    pub row_count: usize,
}

impl QoQTable {
    /// Build from query data (row-major → column-major, a one-time O(R·C) cost).
    /// For a source `CfmlQuery` handle, read it under a guard first:
    /// `query.with_read(|d| QoQTable::from_query_data(name, d))`.
    ///
    /// Optimised for the dominant case in QoQ workloads: source queries built
    /// positionally (via `addRow` / `queryNew(columns, types, rows)`) where each
    /// row's `IndexMap` keys are in insertion order matching `columns`. We use
    /// `get_index(ci)` (Vec-style positional access) instead of `get(name)`
    /// (hash lookup) — and fall back to `get(name)` if the row's `ci`-th key
    /// doesn't match the expected column name (e.g. after an out-of-order
    /// `queryAddColumn`). Columns are built in parallel via rayon on host
    /// targets (not wasm).
    pub fn from_query_data(name: &str, query: &CfmlQueryData) -> Self {
        let col_count = query.columns.len();
        let row_count = query.rows.len();
        let rows = &query.rows;
        let columns = &query.columns;

        // Per-column builder: for each column ci, walk all rows once and read
        // the cell. Positional `get_index` is hot; named `get` is the fallback.
        let build_column = |ci: usize| -> Vec<CfmlValue> {
            let col_name = &columns[ci];
            let mut col = Vec::with_capacity(row_count);
            for row in rows {
                let v = match row.get_index(ci) {
                    Some((k, v)) if k.eq_ignore_ascii_case(col_name) => v.clone(),
                    _ => row.get(col_name).cloned().unwrap_or(CfmlValue::Null),
                };
                col.push(v);
            }
            col
        };

        #[cfg(not(target_arch = "wasm32"))]
        let data: Vec<Vec<CfmlValue>> = {
            use rayon::prelude::*;
            // Only fan out when there's enough work to amortise rayon overhead.
            // Threshold matches the engine's PARALLEL_ROW_THRESHOLD.
            const PARALLEL_CELL_THRESHOLD: usize = 10_000;
            if col_count > 1 && row_count >= PARALLEL_CELL_THRESHOLD {
                (0..col_count)
                    .into_par_iter()
                    .map(build_column)
                    .collect()
            } else {
                (0..col_count).map(build_column).collect()
            }
        };
        #[cfg(target_arch = "wasm32")]
        let data: Vec<Vec<CfmlValue>> = (0..col_count).map(build_column).collect();

        QoQTable {
            name: name.to_string(),
            columns: columns.clone(),
            data,
            row_count,
        }
    }

    /// 0-based column index for a name (case-insensitive).
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name))
    }

    /// Value at (1-based `row`, 0-based `col`). Row `0` is the NULL sentinel.
    pub fn get(&self, row: usize, col: usize) -> CfmlValue {
        if row == 0 {
            return CfmlValue::Null;
        }
        self.data
            .get(col)
            .and_then(|c| c.get(row - 1))
            .cloned()
            .unwrap_or(CfmlValue::Null)
    }
}

/// The ordered set of tables in scope during one SELECT-core execution.
#[derive(Debug, Clone, Default)]
pub struct TableSet {
    pub tables: Vec<QoQTable>,
}

impl TableSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, table: QoQTable) {
        self.tables.push(table);
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Row count per table, in order (used to seed the intersection builder).
    pub fn row_counts(&self) -> Vec<usize> {
        self.tables.iter().map(|t| t.row_count).collect()
    }

    /// Resolve a column reference to `(table_index, column_index)`.
    /// With a table hint, only that table is consulted; without, the first
    /// table containing the column wins.
    pub fn resolve_column(&self, table_hint: Option<&str>, column: &str) -> Option<(usize, usize)> {
        match table_hint {
            Some(hint) => {
                for (ti, t) in self.tables.iter().enumerate() {
                    if t.name.eq_ignore_ascii_case(hint) {
                        return t.column_index(column).map(|ci| (ti, ci));
                    }
                }
                None
            }
            None => {
                for (ti, t) in self.tables.iter().enumerate() {
                    if let Some(ci) = t.column_index(column) {
                        return Some((ti, ci));
                    }
                }
                None
            }
        }
    }

    /// Fetch a cell given an intersection (1-based row index per table).
    pub fn value(&self, intersection: &[usize], ti: usize, ci: usize) -> CfmlValue {
        let row = intersection.get(ti).copied().unwrap_or(0);
        self.tables[ti].get(row, ci)
    }
}
