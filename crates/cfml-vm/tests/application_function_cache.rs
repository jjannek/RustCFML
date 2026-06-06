//! Regression coverage for the stable per-application function table.
//!
//! Application scope can hold long-lived CFC instances. The VM re-homes the
//! bytecode functions reachable from that scope into a per-application,
//! append-only table (`ApplicationState::app_function_table`) keyed by a
//! stable `(source_file, name, ordinal)` identity, so those instances remain
//! callable on later requests via a stable id that never needs remapping.
//! Because the table is keyed by that stable identity, re-instantiating the
//! same CFC (the `overwrite` fixture) or re-creating a per-request CFC (the
//! `sparse` fixture) reuses the existing ids instead of appending duplicates —
//! so the table stays bounded by the number of *distinct* functions in the
//! codebase, never growing per request.

use cfml_codegen::{compiler::CfmlCompiler, BytecodeProgram};
use cfml_common::dynamic::CfmlValue;
use cfml_common::vfs::{EmbeddedFs, Vfs};
use cfml_compiler::{parser::Parser, tag_parser};
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::{CfmlVirtualMachine, ServerState};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

const VROOT: &str = "/app";
const APP_NAME: &str = "application-function-cache-test";

fn compile_page(vfs: &Arc<dyn Vfs>, path: &str) -> BytecodeProgram {
    let source = vfs.read_to_string(path).unwrap();
    let processed = if tag_parser::has_cfml_tags(&source) {
        tag_parser::tags_to_script(&source)
    } else {
        source
    };
    let ast = Parser::new(processed).parse().unwrap();
    CfmlCompiler::new().compile(ast)
}

fn run_request(server_state: &ServerState, vfs: Arc<dyn Vfs>) {
    run_request_with_expected_output(server_state, vfs, "ok");
}

fn run_request_with_expected_output(
    server_state: &ServerState,
    vfs: Arc<dyn Vfs>,
    expected_output: &str,
) {
    let page_path = format!("{}/index.cfm", VROOT);
    let program = compile_page(&vfs, &page_path);

    let mut vm = CfmlVirtualMachine::new(program);
    vm.vfs = vfs;
    vm.source_file = Some(page_path.clone());
    vm.base_template_path = Some(page_path);
    for (name, value) in get_builtins() {
        vm.globals.insert(name, value);
    }
    for (name, func) in get_builtin_functions() {
        vm.builtins.insert(name, func);
    }
    vm.globals
        .entry("url".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));
    vm.globals
        .entry("cgi".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));
    vm.globals
        .entry("form".to_string())
        .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));

    vm.server_state = Some(server_state.clone());

    vm.execute_with_lifecycle().unwrap();
    assert_eq!(expected_output, vm.output_buffer.trim());
}

fn app_function_table_size(server_state: &ServerState) -> usize {
    server_state
        .applications
        .get(APP_NAME)
        .expect("application state should exist")
        .app_function_table
        .len()
}

fn overwrite_fixture() -> HashMap<String, Vec<u8>> {
    let mut files = HashMap::new();
    files.insert(
        "Application.cfc".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/overwrite/Application.cfc")
            .as_bytes()
            .to_vec(),
    );
    files.insert(
        "Factory.cfc".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/overwrite/Factory.cfc")
            .as_bytes()
            .to_vec(),
    );
    files.insert(
        "index.cfm".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/overwrite/index.cfm")
            .as_bytes()
            .to_vec(),
    );
    files
}

fn sparse_fixture() -> HashMap<String, Vec<u8>> {
    let mut files = HashMap::new();
    files.insert(
        "Application.cfc".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/sparse/Application.cfc")
            .as_bytes()
            .to_vec(),
    );
    files.insert(
        "Factory.cfc".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/sparse/Factory.cfc")
            .as_bytes()
            .to_vec(),
    );
    files.insert(
        "RequestFactory.cfc".to_string(),
        include_str!(
            "../../../tests/lifecycle/application_function_cache/sparse/RequestFactory.cfc"
        )
        .as_bytes()
        .to_vec(),
    );
    files.insert(
        "index.cfm".to_string(),
        include_str!("../../../tests/lifecycle/application_function_cache/sparse/index.cfm")
            .as_bytes()
            .to_vec(),
    );
    files
}

#[test]
fn overwritten_application_scope_cfc_does_not_grow_function_cache() {
    let vfs: Arc<dyn Vfs> = Arc::new(EmbeddedFs::new(overwrite_fixture(), VROOT.to_string()));
    let server_state = ServerState::with_production(false);

    run_request(&server_state, vfs.clone());
    let first_count = app_function_table_size(&server_state);
    assert!(
        first_count > 0,
        "test fixture must re-home at least one app-scope CFC function"
    );

    run_request(&server_state, vfs.clone());
    assert_eq!(
        first_count,
        app_function_table_size(&server_state),
        "stable function table must not grow when the same CFC is re-instantiated"
    );

    run_request(&server_state, vfs);
    assert_eq!(
        first_count,
        app_function_table_size(&server_state),
        "stable function table should remain bounded across repeated overwrites"
    );
}

#[test]
fn persistent_and_overwritten_application_cfc_functions_are_cached_sparsely() {
    let vfs: Arc<dyn Vfs> = Arc::new(EmbeddedFs::new(sparse_fixture(), VROOT.to_string()));
    let server_state = ServerState::with_production(false);

    run_request_with_expected_output(&server_state, vfs.clone(), "okok");
    let first_count = app_function_table_size(&server_state);
    assert!(
        first_count > 1,
        "test fixture must re-home persistent and per-request app-scope CFC functions"
    );

    run_request_with_expected_output(&server_state, vfs.clone(), "okok");
    assert_eq!(
        first_count,
        app_function_table_size(&server_state),
        "stable table must reuse ids for persistent and re-created CFCs, not duplicate them"
    );

    run_request_with_expected_output(&server_state, vfs, "okok");
    assert_eq!(
        first_count,
        app_function_table_size(&server_state),
        "stable table should remain bounded across repeated mixed app-scope CFCs"
    );
}
