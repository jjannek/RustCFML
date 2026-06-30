//! Request-boundary cycle collector.
//!
//! RustCFML's reference-typed containers (`CfmlStruct`, `CfmlArray`, `CfmlQuery`)
//! and closure capture scopes (`Arc<RwLock<ValueMap>>`) are `Arc`-refcounted, so
//! a reference *cycle* (`a.other = b; b.other = a`, or a closure stored into the
//! scope it captures) is never reclaimed by refcounting alone — its internal
//! refs keep `strong_count > 0` even after every external root is gone. In a
//! long-lived `--serve` process that builds cyclic per-request graphs (Preside,
//! ColdBox, WireBox), this leaks a little on every request and RSS climbs without
//! bound.
//!
//! This module reclaims those cycles with a **request-scoped trial-deletion**
//! pass (Bacon–Rajan, bounded to one request's allocations). It does NOT replace
//! refcounting: the ~99% acyclic garbage is still freed eagerly, on-thread, with
//! zero pause. The collector only ever processes the small set of containers a
//! request allocated that are *still alive* at request end — never the whole
//! heap, never the resident persistent scopes — so there is no global
//! stop-the-world pause.
//!
//! ## How it stays correct without tracing the persistent scopes
//! The `Arc::strong_count` itself is the oracle. After the request's transient
//! roots (page `variables`, request scope, thread scope) are cleared, a survivor
//! that is still referenced from a *persistent* root (application/session/server
//! scope — which Arc-share the objects that escaped into them) has a strong count
//! greater than the number of references it gets from inside the survivor set; a
//! pure cycle does not. So we compute, per survivor `n`:
//!
//! ```text
//! external(n) = strong_count(n) − 1 (our own probe handle) − internal_in(n)
//! ```
//!
//! `external(n) > 0` ⟺ `n` has an owner outside the request's cyclic garbage ⟹
//! `n` is a live root. We mark the transitive closure of the roots live, and
//! everything else in the survivor set is an unreachable cycle: we clear its
//! backing (dropping its outgoing refs) so the whole subgraph's counts fall to
//! zero and it frees.
//!
//! ## Safety w.r.t. threads
//! Reading `strong_count` is only stable if no other thread is concurrently
//! cloning/dropping the same `Arc`. A truly-internal cycle (the only thing we
//! ever collect) is unreachable from any other request's thread by construction;
//! anything shared across threads escaped to a shared scope and thus reads as a
//! live root. The one case to guard is *this* request's own `cfthread`s, which
//! share `application`/`request` scope by Arc — the VM caller MUST skip
//! collection while `live_threads` is non-empty. See `CYCLE_GC_PLAN.md`.

use crate::dynamic::{CfmlQueryData, CfmlValue, StructInner, ValueMap};
use parking_lot::RwLock as PlRwLock;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock, Weak};

/// Process-wide arm switch. `false` (default) makes every allocation hook a
/// single predictable-false relaxed load — CLI, tests, and wasm pay essentially
/// nothing. Set true once at `--serve` startup (unless `RUSTCFML_NO_CYCLE_GC`).
static GC_ARMED: AtomicBool = AtomicBool::new(false);

/// Total cycle nodes reclaimed across the process, for observability.
static COLLECTED_TOTAL: AtomicUsize = AtomicUsize::new(0);

/// Arm the collector (serve mode). Idempotent.
pub fn arm() {
    GC_ARMED.store(true, Ordering::Relaxed);
}

/// Disarm globally (e.g. `RUSTCFML_NO_CYCLE_GC=1`).
pub fn disarm() {
    GC_ARMED.store(false, Ordering::Relaxed);
}

#[inline]
pub fn is_armed() -> bool {
    GC_ARMED.load(Ordering::Relaxed)
}

/// Cumulative count of cycle nodes reclaimed (for the debug footer / logs).
pub fn collected_total() -> usize {
    COLLECTED_TOTAL.load(Ordering::Relaxed)
}

/// One logged allocation, held weakly so the log never extends an object's
/// lifetime (a dead object's `Weak` simply fails to upgrade at collection time).
enum TrackedAlloc {
    Struct(Weak<PlRwLock<StructInner>>),
    Array(Weak<PlRwLock<Vec<CfmlValue>>>),
    Query(Weak<PlRwLock<CfmlQueryData>>),
    Scope(Weak<RwLock<ValueMap>>),
}

thread_local! {
    /// Per-request allocation log. `Some` only while a TOP-LEVEL request body is
    /// executing on this worker thread; `None` everywhere else (CLI, between
    /// requests, and inside `cfthread` child threads — so child-thread allocs are
    /// never logged and never accumulate). Taking the log out (`collect`) also
    /// leaves it `None`, so the collector's own allocations are never logged.
    static ALLOC_LOG: RefCell<Option<Vec<TrackedAlloc>>> = const { RefCell::new(None) };
}

/// Cap on the per-request allocation log. A request that allocates more than this
/// (a cold framework boot wiring thousands of singletons, or a bulk data job)
/// overflows the log, which is then dropped and collection skipped for that
/// request — bounding the collector's own memory to ~`LOG_CAP * sizeof(Weak)`
/// (a few MB) instead of ballooning to gigabytes. Dropping a partial log only
/// means that one request's cycles are not reclaimed (safe: such requests are
/// one-off, and refcounting still frees all acyclic garbage). Normal requests
/// allocate far below this and collect as usual.
const LOG_CAP: usize = 1_000_000;

/// Begin logging allocations for a request. Call at the very top of a top-level
/// request execution (serve mode only).
pub fn enable() {
    ALLOC_LOG.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

/// Stop logging and drop the log without collecting.
pub fn disable_and_clear() {
    ALLOC_LOG.with(|c| *c.borrow_mut() = None);
}

#[inline]
fn log_push(t: TrackedAlloc) {
    ALLOC_LOG.with(|c| {
        let mut b = c.borrow_mut();
        if let Some(v) = b.as_mut() {
            if v.len() >= LOG_CAP {
                // Overflow: drop the partial log and stop logging for the rest of
                // this request. `collect` will then see `None` and skip — bounding
                // memory on cold boots / bulk jobs. Safe: only this request's
                // cycles go unreclaimed.
                *b = None;
            } else {
                v.push(t);
            }
        }
    });
}

// --- Allocation hooks (called from the container constructors) ---------------
// Each is gated by `is_armed()` so the disarmed path is a single relaxed load.

#[inline]
pub fn log_struct(arc: &Arc<PlRwLock<StructInner>>) {
    if is_armed() {
        log_push(TrackedAlloc::Struct(Arc::downgrade(arc)));
    }
}

#[inline]
pub fn log_array(arc: &Arc<PlRwLock<Vec<CfmlValue>>>) {
    if is_armed() {
        log_push(TrackedAlloc::Array(Arc::downgrade(arc)));
    }
}

#[inline]
pub fn log_query(arc: &Arc<PlRwLock<CfmlQueryData>>) {
    if is_armed() {
        log_push(TrackedAlloc::Query(Arc::downgrade(arc)));
    }
}

/// Allocate a closure-capture scope, tracking it as a cycle node. Use this in
/// place of `Arc::new(RwLock::new(map))` for every `captured_scope`/`closure_env`
/// so closure↔scope cycles are reclaimable.
#[inline]
pub fn tracked_scope(map: ValueMap) -> Arc<RwLock<ValueMap>> {
    let arc = Arc::new(RwLock::new(map));
    if is_armed() {
        log_push(TrackedAlloc::Scope(Arc::downgrade(&arc)));
    }
    arc
}

// --- The collection pass -----------------------------------------------------

/// A strong handle to one survivor, holding exactly ONE reference (subtracted as
/// the "probe handle" when computing external ownership).
enum NodeHandle {
    Struct(Arc<PlRwLock<StructInner>>),
    Array(Arc<PlRwLock<Vec<CfmlValue>>>),
    Query(Arc<PlRwLock<CfmlQueryData>>),
    Scope(Arc<RwLock<ValueMap>>),
}

impl NodeHandle {
    #[inline]
    fn strong_count(&self) -> usize {
        match self {
            NodeHandle::Struct(a) => Arc::strong_count(a),
            NodeHandle::Array(a) => Arc::strong_count(a),
            NodeHandle::Query(a) => Arc::strong_count(a),
            NodeHandle::Scope(a) => Arc::strong_count(a),
        }
    }

    /// Enumerate the immediate child *nodes* (members of `in_set`) without
    /// cloning any `Arc` (so refcounts are undisturbed) — terminal at node
    /// types, descending through non-node carriers (Function/Component/Closure/
    /// QueryColumn). Holds a read guard for the duration; the callback only
    /// records ids and never locks another node, so this cannot deadlock.
    fn for_each_child_node(&self, in_set: &HashSet<usize>, emit: &mut impl FnMut(usize)) {
        match self {
            NodeHandle::Struct(a) => {
                let g = a.read();
                for v in g.map.values() {
                    classify(v, in_set, emit);
                }
            }
            NodeHandle::Array(a) => {
                let g = a.read();
                for v in g.iter() {
                    classify(v, in_set, emit);
                }
            }
            NodeHandle::Query(a) => {
                let g = a.read();
                for col in &g.data {
                    for v in col.iter() {
                        classify(v, in_set, emit);
                    }
                }
            }
            NodeHandle::Scope(a) => {
                if let Ok(g) = a.read() {
                    for v in g.values() {
                        classify(v, in_set, emit);
                    }
                }
            }
        }
    }

    /// Break this node's cycle by clearing its contents (drops its outgoing refs).
    fn clear(&self) {
        match self {
            NodeHandle::Struct(a) => a.write().map.clear(),
            NodeHandle::Array(a) => a.write().clear(),
            NodeHandle::Query(a) => {
                let mut g = a.write();
                g.data.clear();
                g.columns.clear();
            }
            NodeHandle::Scope(a) => {
                if let Ok(mut g) = a.write() {
                    g.clear();
                }
            }
        }
    }
}

/// Record any child *nodes* reachable from `v`. Node types (Struct/Array/Query,
/// and the Scope behind a Function's `captured_scope`) are terminal — emitted but
/// not descended (each is processed as its own survivor). Non-node carriers
/// (Component/Closure boxes, QueryColumn) are descended into, since they are not
/// separately collectible. `NativeObject` is opaque and treated as an external
/// owner (anything it holds stays protected — conservative, never over-collects).
fn classify(v: &CfmlValue, in_set: &HashSet<usize>, emit: &mut impl FnMut(usize)) {
    match v {
        CfmlValue::Struct(s) => {
            let p = s.backing_ptr();
            if in_set.contains(&p) {
                emit(p);
            }
        }
        CfmlValue::Array(a) => {
            let p = a.backing_ptr();
            if in_set.contains(&p) {
                emit(p);
            }
        }
        CfmlValue::Query(q) => {
            let p = q.backing_ptr();
            if in_set.contains(&p) {
                emit(p);
            }
        }
        CfmlValue::Function(f) => {
            if let Some(sc) = &f.captured_scope {
                let p = Arc::as_ptr(sc) as *const () as usize;
                if in_set.contains(&p) {
                    emit(p);
                }
            }
        }
        CfmlValue::Component(c) => {
            for pv in c.properties.values() {
                classify(pv, in_set, emit);
            }
            for m in c.methods.values() {
                if let Some(sc) = &m.captured_scope {
                    let p = Arc::as_ptr(sc) as *const () as usize;
                    if in_set.contains(&p) {
                        emit(p);
                    }
                }
            }
        }
        CfmlValue::Closure(c) => {
            for cv in c.captured_vars.values() {
                classify(cv, in_set, emit);
            }
        }
        CfmlValue::QueryColumn(col) => {
            for cv in col.iter() {
                classify(cv, in_set, emit);
            }
        }
        _ => {}
    }
}

/// Run the request-scoped cycle collection. Drains this thread's allocation log,
/// reclaims unreachable cycles among the request's surviving allocations, and
/// returns the number of nodes reclaimed.
///
/// PRECONDITIONS (the VM caller must establish these):
///  1. No live `cfthread`s (`live_threads.is_empty()`).
///  2. Persistent scopes already written back to `ServerState`.
///  3. Transient roots (page `variables`, request scope, thread scope) cleared.
pub fn collect() -> usize {
    let Some(log) = ALLOC_LOG.with(|c| c.borrow_mut().take()) else {
        return 0;
    };
    if log.is_empty() {
        return 0;
    }

    // 1. Upgrade survivors; one strong probe handle per distinct backing.
    let mut nodes: HashMap<usize, NodeHandle> = HashMap::with_capacity(log.len());
    for t in log {
        match t {
            TrackedAlloc::Struct(w) => {
                if let Some(a) = w.upgrade() {
                    nodes
                        .entry(Arc::as_ptr(&a) as *const () as usize)
                        .or_insert(NodeHandle::Struct(a));
                }
            }
            TrackedAlloc::Array(w) => {
                if let Some(a) = w.upgrade() {
                    nodes
                        .entry(Arc::as_ptr(&a) as *const () as usize)
                        .or_insert(NodeHandle::Array(a));
                }
            }
            TrackedAlloc::Query(w) => {
                if let Some(a) = w.upgrade() {
                    nodes
                        .entry(Arc::as_ptr(&a) as *const () as usize)
                        .or_insert(NodeHandle::Query(a));
                }
            }
            TrackedAlloc::Scope(w) => {
                if let Some(a) = w.upgrade() {
                    nodes
                        .entry(Arc::as_ptr(&a) as *const () as usize)
                        .or_insert(NodeHandle::Scope(a));
                }
            }
        }
    }
    if nodes.is_empty() {
        return 0;
    }

    let in_set: HashSet<usize> = nodes.keys().copied().collect();

    // 2. internal_in[n] = number of references to n from other survivors.
    let mut internal_in: HashMap<usize, usize> = HashMap::with_capacity(nodes.len());
    for h in nodes.values() {
        h.for_each_child_node(&in_set, &mut |child| {
            *internal_in.entry(child).or_insert(0) += 1;
        });
    }

    // 3. Roots = survivors with an owner OUTSIDE the survivor set.
    //    external(n) = strong_count − 1 (probe handle) − internal_in(n).
    let mut live: HashSet<usize> = HashSet::with_capacity(nodes.len());
    let mut worklist: Vec<usize> = Vec::new();
    for (&p, h) in &nodes {
        let internal = *internal_in.get(&p).unwrap_or(&0);
        let external = h.strong_count().saturating_sub(1).saturating_sub(internal);
        if external > 0 && live.insert(p) {
            worklist.push(p);
        }
    }

    // 4. Mark the transitive closure of the roots live (a node reachable from a
    //    live root is live even if its own external count is 0).
    while let Some(p) = worklist.pop() {
        if let Some(h) = nodes.get(&p) {
            h.for_each_child_node(&in_set, &mut |child| {
                if live.insert(child) {
                    worklist.push(child);
                }
            });
        }
    }

    // 5. Everything not live is an unreachable cycle: clear it to break the
    //    cycle, then dropping the probe handles frees the whole subgraph.
    let survivors = nodes.len();
    let mut collected = 0usize;
    for (&p, h) in &nodes {
        if !live.contains(&p) {
            h.clear();
            collected += 1;
        }
    }
    drop(nodes);

    if collected > 0 {
        COLLECTED_TOTAL.fetch_add(collected, Ordering::Relaxed);
    }
    if std::env::var("RUSTCFML_GC_DEBUG").is_ok() {
        eprintln!(
            "[cycle_gc] survivors={} live={} collected={}",
            survivors,
            live.len(),
            collected
        );
    }
    collected
}
