# TestBox compatibility — running blockers

Tracking what stops TestBox 7.1.0 from running on RustCFML, in the order hit.

---

## Blocker #1 — comma-less function params  ✅ FIXED

`parse_param_list` now tolerates newline-separated params without a comma
(matches Lucee/ACF/BoxLang). Fix is in `crates/cfml-compiler/src/parser.rs`
(~line 3578). Confirmed: `function f( boolean a=false  boolean b=true ){}` parses.

---

## Setup note (environment, not an engine bug)

A fresh TestBox git checkout has **no installed modules**. `TestBox.cfc` init
loads `system/modules` (globber, cbstreams, cbMockData). Install them first:

```bash
cd /path/to/TestBox && box install
```

Then run the harness:

```bash
mkdir -p /tmp/tbroot && ln -s /abs/path/to/TestBox /tmp/tbroot/testbox
cat > /tmp/tbroot/Application.cfc <<'CFC'
component {
    this.name = "tbtest";
    this.mappings[ "/testbox" ] = expandPath( "./testbox" );
    this.mappings[ "/tests" ]   = expandPath( "./testbox/tests" );
}
CFC
cat > /tmp/tbroot/run.cfm <<'CFM'
<cfscript>
try {
    tb = new testbox.system.TestBox( bundles="testbox.tests.specs.BDDTest" );
    writeOutput( tb.run() );
} catch (any e) { writeOutput("ERROR: " & e.message); }
</cfscript>
CFM
./target/release/rustcfml --serve /tmp/tbroot --port 8799 &
curl -s http://localhost:8799/run.cfm
```

With blocker #1 fixed and modules installed, TestBox now gets through parsing and
module registration, then dies during **module activation** on blocker #2.

---

## Blocker #2 — method-reference extraction poisons later bare self-calls  ✅ FIXED (v0.201.0)

**Root cause was broader than the original framing.** Reference *extraction* was a
red herring — the real trigger is any **bare call** to a method that returns a
freshly-built component whose own method name collides with the caller's method
name (two sequential `getEnv()` calls reproduce it; no `.getEnv` extraction needed).

A component pseudo-constructor (`__cfc_body__`) runs via `execute_function_with_args`
with an *injected parent scope* (the inherited component's vars, so unscoped
lookups in the body resolve). When the body returned, the closure-parent-scope
write-back diff (`cfml-vm/src/lib.rs`, normal-exit ~6383 and early-return ~4464)
treated the body's **method declarations** as locals to propagate to the parent —
populating `self.closure_parent_writeback` with `{getEnv, Anonymous, …}`. A
component with **no `init()`** never cleared it (the `new` handler only resets it
on the init() path), so the stale write-back lingered and was consumed by the
*enclosing* method's bare-`Call` handler, which merges `closure_parent_writeback`
into the caller's locals. The caller frame's locals then had `getEnv` bound to the
returned object's method (required `key`), so the next bare `getEnv()` misresolved.

**Fix:** a `__cfc_body__`/`__main__` frame is a component/template frame whose
locals are captured separately via `captured_locals` and must never produce a
closure-parent write-back. Both write-back sites are now gated on
`parent_scope.filter(|_| !is_template_frame)`. Regression test:
`tests/oop/test_method_return_name_collision.cfm` (+ fixtures `CollisionHost.cfc`,
`CollisionInner.cfc`). Confirmed: TestBox now runs through `activateModule()` and
hits blocker #3 below.

---

## Blocker #3 — `directoryList()` ignores the filter for directories  ✅ FIXED (v0.202.0)

**NOT a component-resolution bug** (original framing was a red herring). Root cause
is in `directoryList()`: with a non-glob `filter`, **subdirectories bypass the
filter** and are returned as results. TestBox then mistakes a leaked directory for
a module.

**Fix:** the name filter now applies to **both** files and directories; recursion
into subdirs always happens regardless of whether the dir's own name matches.
Fixed at both sites:
- `crates/cfml-stdlib/src/builtins.rs` `fn_directory_list` (~5401) — non-sandbox path.
- `crates/cfml-vm/src/lib.rs` `sandbox_directory_list` — the sandbox/VFS path now
  reads arg 3 (filter) and applies `matches_directory_filter` (it previously dropped
  the filter entirely, a latent bug for sandboxed binaries).

Regression test in `tests/stdlib/test_directorylist.cfm`. Confirmed via the minimal
repro below: `directoryList(dir, true, "name", "Target.cfc")` → `[Target.cfc,
Target.cfc]` (subdirs excluded, recursion intact).

### Symptom (running TestBox, blockers #1 & #2 cleared, modules installed)

```
ERROR: Could not find the component
  [testbox.system.modules.cbstreams.modules.cbproxies.models.ModuleConfig].
```

### How it happens

`TestBox.cfc` `loadTestBoxModules()` discovers modules with:

```cfc
directoryList( modulesPath, true, "path", "ModuleConfig.cfc" )
    .map( ( item ) => item.replaceNoCase( "ModuleConfig.cfc", "" ) )   // -> module dirs
```

It expects only the **4 `ModuleConfig.cfc` files**. RustCFML returns **15 entries** —
every file *and directory* under `system/modules`. The directory
`…/cbstreams/modules/cbproxies/models` slips through; `.map()` leaves it unchanged;
TestBox registers it as a module and tries to activate
`…cbproxies.models.ModuleConfig`, which doesn't exist → the error.

### Minimal repro

```cfc
// files: ./Target.cfc, ./other.txt, ./sub/Target.cfc, ./sub/deep/nope.cfc
r = directoryList( expandPath("./"), true, "name", "Target.cfc" );
// EXPECTED (Lucee/ACF): [Target.cfc, Target.cfc]   (filter applies to dirs too)
// ACTUAL  (RustCFML):   [sub, deep, Target.cfc, Target.cfc]  ← dirs leak through
```

Files are filtered correctly (`other.txt`, `nope.cfc` excluded); **directories are
not** filtered.

### Where to fix

`crates/cfml-stdlib/src/builtins.rs` → `fn_directory_list`, the include test
(~line 5401):

```rust
// current — dirs are included unless the filter is a "*." glob
if (is_file && matches_filter(&file_name, filter))
   || (is_dir && (filter.is_empty() || !filter.starts_with("*."))) {
```

A directory is kept whenever the filter doesn't start with `"*."`. TestBox's filter
is the literal `"ModuleConfig.cfc"` → every dir is kept. Fix: apply the name filter
to **both** files and directories, and always recurse into dirs regardless of match:

```rust
if matches_filter(&file_name, filter) {
    // push (file or dir) ...
}
if recurse && is_dir {
    results.extend(list_dir(&full_path, true, filter, list_info)?);
}
```

(Lucee applies the filter to entry names of both files and directories, but still
descends into every subdirectory.)

### Second, latent site — sandbox/VFS path drops the filter entirely

`crates/cfml-vm/src/lib.rs` (~line 14691) intercepts `directorylist` for the
sandbox/VFS and calls `sandbox_directory_list(&path, recurse, &list_info)` — it
never reads arg 3 (filter) at all. Non-sandbox serve mode uses the builtin above
(so fixing builtins.rs unblocks TestBox here), but **sandboxed binaries** will
return unfiltered results until the VFS path forwards the filter too.

---

## Blocker #2 (original notes, kept for reference) — ❌ OPEN → see FIXED above

### Symptom (running TestBox)

```
Runtime Error: The parameter [key] to function [getEnv] is required but was not passed in.
  1: activateModule  (system/util/Env.cfc)
  3: loadTestBoxModules (testbox/system/TestBox.cfc:134)
  4: init (testbox/system/TestBox.cfc:99)
```

`TestBox.cfc` `activateModule` (lines 196–199) does:

```cfc
moduleRecord.moduleConfig
    .injectPropertyMixin( "getJavaSystem",   getEnv().getJavaSystem )
    .injectPropertyMixin( "getSystemSetting",getEnv().getSystemSetting )
    .injectPropertyMixin( "getSystemProperty",getEnv().getSystemProperty )
    .injectPropertyMixin( "getEnv",          getEnv().getEnv );
```

Here `getEnv()` is TestBox's **own** no-arg method (`TestBox.cfc:257`, returns a
`util.Env` instance). `getEnv().getEnv` is meant to *extract a reference* to
`Env.getEnv( required key )` (Env.cfc:67) — NOT call it. The first one works; a
later one ends up **invoking** `Env.getEnv()` with no key → the error.

### Minimal repro (2 lines, no TestBox needed)

`Env.cfc`:
```cfc
component { function getEnv( required key ) { return "got:" & key; } }
```
`Host.cfc`:
```cfc
component {
    function getEnv() { if (isNull(variables.e)) variables.e = new Env(); return variables.e; }
    function go() {
        var r1 = getEnv().getEnv;   // OK   — extracts Env.getEnv as a reference
        var r2 = getEnv().getEnv;   // FAIL — the 2nd bare getEnv() resolves to Env.getEnv (required key)
    }
}
```
```cfscript
new Host().go();
// Runtime Error: The parameter [key] to function [getEnv] is required but was not passed in.
```

### What is and isn't the trigger (each tested in isolation)

- ✅ Method shadowing a builtin (`getEnv`) — fine on its own.
- ✅ Bare method reference `obj.method` (no parens) — returns a function.
- ✅ Bare member-ref on a call result `getEnv().getEnv` — fine, **once**.
- ✅ Passing `getEnv().getEnv` as a function argument — fine, once.
- ❌ **Doing `getEnv().getEnv` twice in the same frame** — the *second* bare
  `getEnv()` call misresolves. Chaining is NOT required; two sequential
  statements reproduce it.

### Hypothesis / where to look

Extracting a method reference via bare member access (`expr.methodName` with no
call) appears to register/cache `methodName` in the current frame's name
resolution, so a subsequent **bare unqualified call** to that name resolves to
the extracted reference (`Env.getEnv`, required key) instead of the enclosing
component's own method (`Host.getEnv`, no args).

Likely in the VM's member-access / "get method as value" path interacting with
bare-call resolution — probably `cfml-vm/src/lib.rs` around how a bare member
access yields a function value and how unqualified call names are resolved
(check whether reference extraction writes a binding into locals/variables, or
mutates a per-frame method cache). Compare with the bare-name resolution fixes
from PRs #97 / #94 / #79.

### Bonus finding — unreliable exception line numbers

For these dynamic/mixin paths the `cfcatch.tagContext` line numbers are wrong
(e.g. `run.cfm:150`, `TestBox.cfc:0` for files far shorter). Diagnosing took a
detour because of it — worth a look once blocker #2 is cleared. The server-log
Rust stack trace was more accurate than the CFML `tagContext`.

---

## Status

TestBox: **parser ✅ · modules installed ✅ · #2 method-ref resolution ✅ (v0.201.0)
· #3 `directoryList` filter applies to dirs ✅ (v0.202.0).** Next: re-run the
harness above (needs a fresh TestBox checkout + `box install`) to find blocker #4.
