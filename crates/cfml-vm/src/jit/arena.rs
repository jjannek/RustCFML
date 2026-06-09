//! Per-call value arena for v0.90.0 mid-body Boxed allocations.
//!
//! Background. v0.89.0 admits `Kind::Boxed` only at the outer ABI: input args
//! and the return value cross as tagged `usize` pointers, and the engine
//! reclaims every leaked box once the body has returned. There is no
//! mid-body box producer — every Boxed value flowing through the compiled
//! body is one of the input pointers. v0.90.0 lifts that restriction: shims
//! such as `cfml_jit_box_int` / `cfml_jit_concat_boxed` allocate fresh
//! boxes and hand the caller a tagged pointer. Those pointers must outlive
//! the compiled body but must not leak past it.
//!
//! Design. Rather than widen the [`super::CompiledFn`] signature with an
//! arena pointer (which would invalidate every existing test, every OSR
//! signature, and the `cfml_call_jit_udf` shim), we stash the *active*
//! arena in a thread-local while a compiled body is on the stack. Shims
//! push freshly-allocated tags into the active arena; the engine drains
//! the arena on the way out and reclaims everything except, on a successful
//! `Boxed` return, the tag that matches the return value.
//!
//! Nesting. UDF→UDF dispatch keeps using the **same** arena: a JIT'd
//! caller invokes the callee through `dispatch_jit_udf` while the caller's
//! arena is active, so any boxes the callee allocates are tracked in the
//! caller's frame. The frame is set up by the *outermost* `run_compiled`
//! and torn down on the way out. Re-entry from the interpreter (one
//! `run_compiled` from inside another via the VM main loop) does push a
//! new frame — see [`ArenaGuard`]. A null `ACTIVE_ARENA` outside JIT
//! execution is the normal interpreter state.

use std::cell::Cell;

use super::boxed;
use cfml_common::dynamic::CfmlValue;

thread_local! {
    /// Pointer to the [`Arena`] owned by the innermost active `run_compiled`,
    /// or null when no JIT body is on the stack. Shims read it via
    /// [`track`]; setup/teardown goes through [`ArenaGuard`].
    static ACTIVE_ARENA: Cell<*mut Arena> = const { Cell::new(std::ptr::null_mut()) };
}

/// Tags allocated by shims for the duration of one outermost compiled call.
///
/// `tags` is append-only during the call (no in-place reuse — a slot
/// overwrite simply abandons the old tag, which will be reclaimed on
/// drain). Capacity scales linearly with the number of mid-body Boxed
/// productions; for the v0.90.0 perf-target kernels (string concat loops)
/// this is bounded by loop trip count × per-iter productions and is
/// expected to stay in the low thousands. A hard cap is the simplest
/// runaway backstop.
#[derive(Default, Debug)]
pub struct Arena {
    tags: Vec<usize>,
}

impl Arena {
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    /// Push a tag produced this call. Mid-body allocations push into here;
    /// the engine drains the entire arena on the way out. A pathological
    /// unbounded shim loop would grow this `Vec` without bound, but Rust's
    /// `Vec::push` panics on OOM, which is the same failure mode as the
    /// interpreter under the same conditions — no separate cap is needed.
    pub fn track(&mut self, tag: usize) {
        self.tags.push(tag);
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Drain the arena, dropping every tag except the one matching
    /// `keep`. Used on a successful `Boxed` return: the engine consumes
    /// the kept tag by re-wrapping it into a `CfmlValue`. The kept tag
    /// is *removed* from the arena but **not** reclaimed by this
    /// function — the caller does that via [`boxed::reclaim_tagged`].
    pub fn drain_except(&mut self, keep: Option<usize>) {
        let kept = match keep {
            Some(k) => Some(k),
            None => None,
        };
        for tag in self.tags.drain(..) {
            if Some(tag) == kept {
                // Ownership transfers to the engine; do not reclaim here.
                continue;
            }
            // SAFETY: every tag in the arena came from `boxed::box_value`
            // (only producer) and has not yet been reclaimed.
            drop(unsafe { boxed::reclaim_tagged(tag) });
        }
    }
}

/// RAII guard that installs an arena as the active one for the duration
/// of a `run_compiled` call. Saves and restores any previously-installed
/// arena so nested re-entries from the interpreter work.
pub struct ArenaGuard {
    prev: *mut Arena,
}

impl ArenaGuard {
    /// Install `arena` as the active arena. Must be paired with the
    /// guard dropping at the end of the body call.
    pub fn install(arena: &mut Arena) -> ArenaGuard {
        ACTIVE_ARENA.with(|a| {
            let prev = a.get();
            a.set(arena as *mut Arena);
            ArenaGuard { prev }
        })
    }
}

impl Drop for ArenaGuard {
    fn drop(&mut self) {
        ACTIVE_ARENA.with(|a| a.set(self.prev));
    }
}

/// Track a freshly-allocated tag in the active arena, returning the tag
/// unchanged. Panics in debug builds if no arena is installed; in release
/// builds it reclaims and drops the tag (an obvious leak, but at least
/// not undefined behaviour).
#[inline]
pub fn track(tag: usize) -> usize {
    ACTIVE_ARENA.with(|a| {
        let p = a.get();
        if p.is_null() {
            // Defensive: should never happen — every shim is reached from
            // inside a compiled body which `run_compiled` wrapped in an
            // ArenaGuard. Reclaim to avoid leaking.
            debug_assert!(false, "track() with no active arena");
            drop(unsafe { boxed::reclaim_tagged(tag) });
            return 0;
        }
        // SAFETY: the active pointer is a `&mut Arena` borrow that
        // lives for the duration of the `ArenaGuard` enclosing this
        // call. Shims execute on the same thread, so no aliasing.
        unsafe { (*p).track(tag) };
        tag
    })
}

/// Box `v` into the active arena, returning the tagged pointer. Shims call
/// this rather than `boxed::box_value` directly so the engine drains the
/// box on the way out.
#[inline]
pub fn box_into_active(v: CfmlValue) -> usize {
    let tag = boxed::box_value(v);
    track(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_and_drain_drops_everything() {
        let mut arena = Arena::new();
        let g = ArenaGuard::install(&mut arena);
        let a = box_into_active(CfmlValue::Int(1));
        let b = box_into_active(CfmlValue::Int(2));
        assert_ne!(a, 0);
        assert_ne!(b, 0);
        // Drop guard; we expect track() to have stored both tags.
        drop(g);
        assert_eq!(arena.tags.len(), 2);
        arena.drain_except(None);
        assert!(arena.is_empty());
    }

    #[test]
    fn drain_except_skips_returned_tag() {
        let mut arena = Arena::new();
        let g = ArenaGuard::install(&mut arena);
        let _a = box_into_active(CfmlValue::Int(1));
        let ret = box_into_active(CfmlValue::Int(42));
        drop(g);
        // Pretend the engine is consuming `ret` as the return value.
        arena.drain_except(Some(ret));
        // `_a` reclaimed and dropped; `ret` removed from the arena but
        // still owned by the caller — reclaim it ourselves so the test
        // doesn't leak.
        drop(unsafe { boxed::reclaim_tagged(ret) });
        assert!(arena.is_empty());
    }

    #[test]
    fn nested_guards_restore_previous_active() {
        let mut outer = Arena::new();
        let mut inner = Arena::new();
        let go = ArenaGuard::install(&mut outer);
        let _outer_tag = box_into_active(CfmlValue::Int(1));
        {
            let gi = ArenaGuard::install(&mut inner);
            let _inner_tag = box_into_active(CfmlValue::Int(2));
            drop(gi);
        }
        let _outer_tag2 = box_into_active(CfmlValue::Int(3));
        drop(go);
        assert_eq!(outer.tags.len(), 2);
        assert_eq!(inner.tags.len(), 1);
        outer.drain_except(None);
        inner.drain_except(None);
    }
}
