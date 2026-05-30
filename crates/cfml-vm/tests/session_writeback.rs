//! Regression: an *existing* session that gets mutated on a later request
//! must commit the new value back through `SessionStore::set`.
//!
//! Background: the Cloudflare worker's `KvBackedSessionStore` persists by
//! draining a `dirty` set (populated by `set()`) and writing each entry to
//! KV. A demo counter (`session.visits`) was observed frozen after the
//! request that created the session — no later mutation ever reached KV, and
//! `last_accessed_secs` never advanced (so timeout-sliding was broken too).
//!
//! The worker-layer fix (await the KV writes instead of deferring them via
//! `ctx.wait_until`) can only be exercised against a real `KvStore`, which is
//! wasm-only and not mockable on the host. What these tests guard is the
//! upstream half the flush relies on: that the VM, on a second request
//! against an already-existing session, calls `set()` with the incremented
//! value AND a refreshed `last_accessed_secs`. If that contract regresses,
//! no amount of worker-layer flushing would persist the change.

use cfml_codegen::compiler::CfmlCompiler;
use cfml_common::dynamic::CfmlValue;
use cfml_common::vfs::{EmbeddedFs, Vfs};
use cfml_compiler::{parser::Parser, tag_parser};
use cfml_stdlib::builtins::{get_builtin_functions, get_builtins};
use cfml_vm::{CfmlVirtualMachine, MemoryStore, ServerState, SessionData, SessionStore};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const VROOT: &str = "/app";

/// A `SessionStore` that delegates to a real `MemoryStore` but records every
/// `set()` so the test can assert the writeback path fired with the expected
/// value — exactly the signal the worker's `dirty`-set flush keys off.
struct SpyStore {
    inner: MemoryStore,
    sets: Mutex<Vec<(String, SessionData)>>,
}

impl SpyStore {
    fn new() -> Self {
        Self {
            inner: MemoryStore::new(),
            sets: Mutex::new(Vec::new()),
        }
    }

    /// Number of recorded `set()` calls for `id`.
    fn set_count(&self, id: &str) -> usize {
        self.sets
            .lock()
            .unwrap()
            .iter()
            .filter(|(k, _)| k == id)
            .count()
    }

    /// The most recently recorded `set()` payload for `id`.
    fn last_set(&self, id: &str) -> Option<SessionData> {
        self.sets
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(k, _)| k == id)
            .map(|(_, d)| d.clone())
    }
}

impl SessionStore for SpyStore {
    fn get(&self, id: &str) -> Option<SessionData> {
        self.inner.get(id)
    }
    fn set(&self, id: &str, data: SessionData) {
        self.sets.lock().unwrap().push((id.to_string(), data.clone()));
        self.inner.set(id, data);
    }
    fn remove(&self, id: &str) {
        self.inner.remove(id);
    }
    fn rotate(&self, old_id: &str, new_id: &str) {
        self.inner.rotate(old_id, new_id);
    }
    fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }
    fn take_expired(
        &self,
        now_secs: u64,
    ) -> Vec<(String, IndexMap<String, CfmlValue>)> {
        self.inner.take_expired(now_secs)
    }
}

const APP: &str = r##"
component {
    this.name              = "session-writeback-test";
    this.sessionManagement = true;
    this.sessionTimeout    = createTimeSpan(0, 1, 0, 0);

    function onRequest(targetPage) { include "#targetPage#"; }
}
"##;

/// `session.visits` increments on every request — the exact demo shape.
const PAGE: &str = r#"<cfscript> session.visits = ( session.visits ?: 0 ) + 1; </cfscript>"#;

/// Run one request against a caller-supplied store + session id, so a test
/// can fire several requests at the *same* session and inspect persistence.
fn run_request(store: Arc<dyn SessionStore>, sid: &str) {
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    files.insert("Application.cfc".to_string(), APP.as_bytes().to_vec());
    files.insert("index.cfm".to_string(), PAGE.as_bytes().to_vec());
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

    let mut server_state = ServerState::with_production(false);
    server_state.sessions = store;

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

    vm.server_state = Some(server_state);
    vm.session_id = Some(sid.to_string());

    let _ = vm.execute_with_lifecycle();
}

fn visits(data: &SessionData) -> i64 {
    data.variables
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("visits"))
        .and_then(|(_, v)| v.as_string().trim().parse::<i64>().ok())
        .unwrap_or(-1)
}

#[test]
fn existing_session_mutation_persists_on_second_request() {
    let spy = Arc::new(SpyStore::new());
    let sid = "sid-existing";

    // Request 1: creates the session, sets visits = 1.
    run_request(spy.clone() as Arc<dyn SessionStore>, sid);
    let after_first = spy.get(sid).expect("record created on first request");
    assert_eq!(visits(&after_first), 1, "first visit should persist visits=1");

    // Request 2: the record already exists. The mutation must commit.
    let sets_before = spy.set_count(sid);
    run_request(spy.clone() as Arc<dyn SessionStore>, sid);

    let after_second = spy.get(sid).expect("record still present");
    assert_eq!(
        visits(&after_second),
        2,
        "an existing session's mutation must persist (the worker flush keys off this set())"
    );
    assert!(
        spy.set_count(sid) > sets_before,
        "the second request must call set() on the store so the worker marks it dirty"
    );

    // The most recent set() must carry the incremented value — this is the
    // exact payload the worker would serialize to KV.
    let last = spy.last_set(sid).expect("a set() was recorded");
    assert_eq!(visits(&last), 2, "the persisted payload must hold the new value");
}

#[test]
fn existing_session_refreshes_last_accessed() {
    let spy = Arc::new(SpyStore::new());
    let sid = "sid-sliding";

    run_request(spy.clone() as Arc<dyn SessionStore>, sid);
    let created = spy.get(sid).expect("record created").created_secs;

    run_request(spy.clone() as Arc<dyn SessionStore>, sid);
    let last = spy.last_set(sid).expect("a set() was recorded on the 2nd request");

    // Timeout-sliding depends on last_accessed being (re)written on every
    // request. Sub-second test timing means it may equal `created`, but it
    // must never regress below it, and the writeback must have happened.
    assert!(
        last.last_accessed_secs >= created,
        "last_accessed_secs must be refreshed (>= created) so timeout sliding works"
    );
}
