//! Data-only session enforcement (issue #88).
//!
//! The session scope persists data values only. Writing a closure, function,
//! component, or native object must fail loudly — on every store, memory
//! included — instead of silently serialising to null on an external store.

use cfml_codegen::compiler::CfmlCompiler;
use cfml_common::dynamic::CfmlValue;
use cfml_common::vfs::{EmbeddedFs, Vfs};
use cfml_compiler::{parser::Parser, tag_parser};
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::{CfmlVirtualMachine, MemoryStore, ServerState, SessionStore};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

const VROOT: &str = "/app";

/// Run a request and return the lifecycle result so the test can assert on
/// whether the session write was rejected.
fn run(page_cfm: &str) -> Result<CfmlValue, cfml_common::vm::CfmlError> {
    let app_cfc = r##"
component {
    this.name              = "data-only-test";
    this.sessionManagement = true;
    function onRequest(targetPage) { include "#targetPage#"; }
}
"##;

    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    files.insert("Application.cfc".to_string(), app_cfc.as_bytes().to_vec());
    files.insert("index.cfm".to_string(), page_cfm.as_bytes().to_vec());
    let vfs: Arc<dyn Vfs> = Arc::new(EmbeddedFs::new(files, VROOT.to_string()));

    let page_path = format!("{}/index.cfm", VROOT);
    let source = vfs.read_to_string(&page_path).unwrap();
    let processed = if tag_parser::has_cfml_tags(&source) {
        tag_parser::tags_to_script(&source)
    } else {
        source
    };
    let ast = Parser::new(processed).parse().unwrap();
    let program = CfmlCompiler::new().compile(ast);

    let store = Arc::new(MemoryStore::new());
    let mut server_state = ServerState::with_production(false);
    server_state.sessions = store.clone() as Arc<dyn SessionStore>;

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
    for scope in ["url", "cgi", "form"] {
        vm.globals
            .entry(scope.to_string())
            .or_insert_with(|| CfmlValue::strukt(IndexMap::new()));
    }
    vm.server_state = Some(server_state);
    vm.session_id = Some("sid-data-only".to_string());

    vm.execute_with_lifecycle()
}

#[test]
fn plain_data_is_allowed() {
    let r = run(r#"<cfscript> session.cart = ["a", "b"]; session.count = 2; </cfscript>"#);
    assert!(r.is_ok(), "plain data values must persist without error: {:?}", r.err());
}

#[test]
fn closure_in_session_is_rejected() {
    let r = run(r#"<cfscript> session.handler = function() { return 1; }; </cfscript>"#);
    let err = r.expect_err("a closure in session must be rejected");
    assert!(
        err.message.to_lowercase().contains("session.handler")
            && err.message.to_lowercase().contains("data values"),
        "error should name the offending key path: {}",
        err.message
    );
}

#[test]
fn nested_closure_is_rejected_with_path() {
    let r = run(r#"<cfscript> session.cfg = { cb: function(){ return 1; } }; </cfscript>"#);
    let err = r.expect_err("a nested closure in session must be rejected");
    assert!(
        err.message.contains("session.cfg.cb"),
        "error should name the nested key path: {}",
        err.message
    );
}

#[test]
fn reference_smuggled_closure_is_caught_at_persist() {
    // The shallow assignment check can't see this — the persist-time deep walk
    // is the airtight gate.
    let r = run(
        r#"<cfscript>
            local.holder = {};
            session.box = local.holder;   // plain struct at write time
            local.holder.fn = function(){ return 1; };  // mutated through the alias
        </cfscript>"#,
    );
    let err = r.expect_err("a reference-smuggled closure must be caught at persist");
    assert!(
        err.message.to_lowercase().contains("data values"),
        "persist gate should reject smuggled non-data value: {}",
        err.message
    );
}
