# Wheels TestBox — remaining failures (post-v0.297.0)

Snapshot after the v0.293–v0.297 fix campaign: **Wheels TestBox = 2716 pass**
(was 2689 at v0.292). Repro recipe is in
`memory/project_wheels_testbox_fix_campaign.md` ("Repro recipe"):

```bash
# serve Wheels under RustCFML
WHEELS_CI=true RUSTCFML_JIT=0 <release-rustcfml> --serve public --port 8599
curl "http://127.0.0.1:8599/index.cfm/wheels/core/tests?db=sqlite&reload=true&format=json"
# Lucee 7.0.4 reference (webroot = RustCFML repo root):
box server start webroot=<repo> cfengine=lucee@7 port=8586 name=rcfml-lucee
```

This doc records the **remaining non-image failures** with enough detail to fix.
They fall into three buckets: a tractable engine bug (cfinvoke overlay), deeper
engine work, and not-our-bugs.

---

## 1. `cfinvoke(component=obj, method=…)` runs on a detached clone — MOST TRACTABLE

**Spec:** `model/callbacksSpec.cfc:180` "has access to changed property values in
aftersave". Error: `Variable 'hasObjectChanged' is undefined`.

### Symptom / minimal repro (RustCFML wrong, Lucee = `yes`)

```cfml
// _HocSpec.cfc
component {
    function saver()  { hasObjectChanged = "yes"; }   // unscoped write
    function getter() { return hasObjectChanged; }     // unscoped read
    function run() {
        obj = new _HocModel();          // a DIFFERENT, empty component
        obj.saver  = this.saver;        // extract this component's methods…
        obj.getter = this.getter;       // …and graft them onto obj
        cfinvoke(component=obj, method="saver");   // run saver on obj
        return obj.getter();            // expect "yes"
    }
}
// component _HocModel {}
```

- **Direct dispatch `obj.saver()` WORKS** (returns `yes`) — `call_member_function`
  overlays the live receiver's `this`/`__variables`, so the unscoped write lands
  in `obj`'s variables and persists; `getter()` reads it back.
- **`cfinvoke(component=obj, method="saver")` FAILS** — the `__cfinvoke` handler
  builds its own `method_locals` and calls `call_function` against a **detached
  clone** of the component, so the unscoped write never reaches `obj`'s live
  `__variables`; `getter()` then finds nothing.

In Wheels this is the `afterSave` callback path: `saveHasChanged` (a spec method)
is grafted onto the model and fired via `$callback → $invoke → cfinvoke`; the
write to `hasObjectChanged` is lost.

### Root cause (code)

`crates/cfml-vm/src/lib.rs`, the `"__cfinvoke"` intercept (`~10198`) and the
identical `"invoke"` BIF intercept (`~10391`). The found-method struct branch
does:

```rust
if let Some(func @ CfmlValue::Function(_)) = method_func {
    let call_args = self.build_invoke_call_args(&func, invoke_args);
    let mut method_locals = ValueMap::default();
    method_locals.insert("this".to_string(), component.clone());      // <-- clone
    if let CfmlValue::Struct(ref cs) = component {
        if let Some(vars) = cs.get("__variables") {
            method_locals.insert("__variables".to_string(), vars.clone());
        }
    }
    return self.call_function(&func, call_args, &method_locals);       // <-- detached
}
```

`call_member_function` (`~13826`) is the path that correctly overlays the live
receiver and writes mutations back.

### Why the naive fix fails (DO NOT just route through call_member_function)

Replacing the block with
`self.call_member_function(&component, &method_name, &mut call_args, None)`
**clears this spec but regresses 5 tests** in
`tests/.../Core: invoke() delivers undeclared argument-struct keys`.

Reason: `build_invoke_call_args` only produces **positional** args for the
callee's *declared* params. `cfinvoke`/`invoke` must also deliver **undeclared
named keys** from the argument struct into the callee's `arguments` scope
(`invoke(obj,"m",{declared=1, extra=2})` → `arguments.extra` must exist).
Positional-only dispatch drops them.

### Suggested fix

Route through `call_member_function` BUT preserve the named-arg surface — i.e.
pass the argument-struct's keys as `arg_names` and values as `extra_args` (the
same shape a normal `obj.m(a=1,b=2)` call uses), instead of the positional list
from `build_invoke_call_args`. Concretely:

1. From `invoke_args` (a `Struct`), first expand a top-level `argumentCollection`
   key (mirror `build_invoke_call_args` lines ~13567–13583).
2. Split the flattened entries into `(names, values)`:
   - numeric string keys `"1","2",…` → positional (push value, `arg_names` slot
     left as a positional marker), and
   - non-numeric keys → named (value into `extra_args`, key into `arg_names`).
3. Call `self.call_member_function(&component, &method_name, &mut values, Some(&names))`.

`call_member_function`'s existing named-arg reorder will then both bind declared
params AND surface undeclared keys in `arguments` (fixing the regression), while
its receiver overlay + writeback fix the `hasObjectChanged` spec.

Verify against BOTH:
- `tests/.../test_invoke_undeclared_arg_keys` (or grep the suite for "delivers
  undeclared argument-struct keys") — must stay green, AND
- the `_HocSpec` repro above — must return `yes`.

Then add a regression test (component-method grafted onto another component,
invoked via `cfinvoke component=`, asserting the unscoped write persists), and
re-run the full gate (CLI runner, `cargo test --workspace`, wasm32 + wasm-pack,
serve cold+warm) + the Wheels suite.

**Leverage:** 1 spec directly; the writeback path is shared, so watch the broader
callbacks/association cfinvoke specs for incidental wins.

---

## 2. Deeper engine work (each its own investigation)

- **`renderingSpec` "renders current action as xml without template" ×2** —
  `cannot call method [getClass] on a null value`. Wheels `toXML` iterates model
  properties and calls `.getClass()` on each; an UNSET model property is `Null`
  in RustCFML but `""` in Lucee (full-null-off). v0.297 fixed NULL→"" for raw
  SQLite **query cells**, but a missing **model property** is a different read
  path (the property simply isn't materialized as a key). Fix: when reading an
  unset declared property off a model instance, surface `""` not `Null`
  (Lucee full-null-off). Risk: model property access is hot; scope carefully.

- **`crudSpec` "selecting calculated property when implicitly selecting fields"** —
  `isDefined("posts.titleAlias")` is false. The `isDefined` query-column arm was
  fixed (v0.294); the real gap is the **calculated property's SQL expression is
  not aliased into the SELECT** when fields are selected implicitly. Trace Wheels
  `$selectClause`/`calculatedProperties` SQL building vs the generated SQL.

- **`crudSpec` "findAllByXXX works"** — dynamic finder returns 0 rows. SQL/WHERE
  building for the dynamic-finder path; could not reproduce standalone (needs the
  in-request seed data). Instrument the generated SQL in `onMissingMethod`'s
  `findAllBy` branch.

- **`crudSpec` "function hasChanged is working with binary compare"** — binary
  column change-detection; relates to how BLOBs round-trip (base64 vs
  `CfmlValue::Binary`).

- **`crudSpec` "dynamic update with named argument"** — `setProfile(profile=x)`
  via `onMissingMethod → $associationMethod` setObject path. `missingMethodArguments`
  is byte-identical to Lucee (verified), so this is a Wheels-side named-arg quirk
  in `$associationMethod`'s `[1]` positional access; low priority.

- **`nestedpropertiesSpec` PK rollback ×2** — child validation failure isn't
  propagated through nested `$saveAssociations`, so the save reports success and
  the parent PK isn't reset on rollback.

- **`miscellaneousSpec` "objectid should be sequential and norepeating"** —
  length 31 vs 30. The objectid ARITHMETIC matches Lucee exactly in isolation;
  the extra id comes from one extra child-object instantiation in the nested
  `hasMany` build path (`request`-scope counter bumped once more). Needs in-situ
  tracing of `gallery.new(photos=…)` instantiation count.

- **`formsdateplainSpec` "works with step argument" ×2** — `minuteStep=15`
  produces 8 `<option>`s instead of 4. Isolated proof: 8 options occur iff the
  loop's `$step` evaluates to **7.5** (= 15/2). Every primitive (the loop,
  `invoke(this,method,args)`, `Duplicate`, `StructAppend`, `Val`) yields 4 in
  isolation, so the `/2` is introduced somewhere in the live
  `minuteSelectTag → timeSelectTags → $dateOrTimeSelect → $minuteSelectTag →
  $yearMonthHourMinuteSecondSelectTag` arg-merge chain. Instrument
  `formsdate.cfc:298` to print `arguments.$step`, then bisect upstream.

- **`contentSpec` "is not allowing partial loading data from implicit public
  method"** — Wheels' gate is correct; the divergence is that RustCFML returns
  `Null` (empty) for a missing **explicit-scope** member access
  (`#arguments.fruit#`) where Lucee THROWS an `expression` exception. Fix in the
  `GetProperty` `None =>` arm (`lib.rs:~5742`) — throw ONLY for explicit scope
  receivers (`arguments`/`variables`/`local`, distinguishable via the
  `__arguments_scope` marker) while preserving Null-on-miss for plain structs.
  **High regression risk** — scope tightly and run the full suite.

## 3. Not RustCFML bugs (leave)

- **`formsSpec` buttonTag ×2 + `textfieldSpec` ×2** — `encode` HTML-attribute
  specs. RustCFML's `EncodeForHTMLAttribute` is OWASP/ESAPI-exact (matches Adobe
  CF and BoxLang; Lucee is the documented outlier). The Wheels test literals omit
  the `&#x5c;` for the backslash in `alert(\"xss\")`, so they're internally
  inconsistent (they DO expect `&#x28;`/`&#x20;` hex everywhere else). The engine
  encoder is correct — do not change it; the literals would need fixing upstream.

- **`pluginsSpec` "call overwridden method with identical method nesting"** —
  `Cannot read '…/_assets/views/_testpartial.cfm'`. The fixture file is genuinely
  absent from the Wheels checkout; not an engine issue.

- **`assetsSpec` imageTag ×6 / `cfimage`** — image tag, DEFERRED per project
  decision (not in scope).
