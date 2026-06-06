//! Query-of-Queries engine for RustCFML — in-memory SQL `SELECT` execution
//! against `CfmlQuery` objects (`queryExecute(sql, params, {dbtype:"query"})`).
//!
//! Public entry points: [`parse`] a SQL string, [`base_table_names`] to discover
//! the query variables it references (so the VM can resolve them from scope),
//! and [`execute`] the parsed statement against those source queries.

pub mod ast;
pub mod compare;
pub mod execution;
pub mod function;
pub mod functions;
pub mod intersection;
pub mod lexer;
pub mod like;
pub mod parser;
pub mod table;

pub use ast::{SelectStatement, Statement};
pub use execution::{base_table_names, execute, QoQParams};
pub use function::{QoQFn, QoQFnKind, QoQFunctionRegistry};
pub use parser::parse;
