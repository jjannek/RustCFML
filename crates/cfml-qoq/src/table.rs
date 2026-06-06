//! Column-major source tables for QoQ execution.
//!
//! Row indices in intersections are **1-based**, with **0 reserved as the NULL
//! sentinel** for outer joins (a missing row on one side of a LEFT/RIGHT/FULL
//! join). `get(0, _)` therefore returns `Null`.

use cfml_common::dynamic::{CfmlQuery, CfmlValue};

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
    /// Build from a `CfmlQuery` (row-major → column-major, a one-time O(R·C) cost).
    pub fn from_query(name: &str, query: &CfmlQuery) -> Self {
        let col_count = query.columns.len();
        let row_count = query.rows.len();
        let mut data: Vec<Vec<CfmlValue>> = (0..col_count)
            .map(|_| Vec::with_capacity(row_count))
            .collect();
        for row in &query.rows {
            for (ci, col) in query.columns.iter().enumerate() {
                data[ci].push(row.get(col).cloned().unwrap_or(CfmlValue::Null));
            }
        }
        QoQTable {
            name: name.to_string(),
            columns: query.columns.clone(),
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
