# Calling an Application-Scope Object's Method From a Custom Tag

Lucee-compatible behavior is that a CFC instance stored in the `application`
scope (typically created in `onApplicationStart`) remains fully usable
everywhere, including inside a custom tag (`cf_*` / `cfmodule`). Calling its
methods must work the same as from a regular page.

Apps rely on this constantly — services are kept in the application scope and
called from view/control tags. The shape that exposed the gap:

```cfml
<!-- Application.cfc -->
function onApplicationStart() {
    application.lib.db = createObject("component", "/app/lib/db").init();
}

<!-- a custom tag, e.g. <cf_route> -->
<cfset svc = application.lib.db.getService("moo_route") />
```

## Observed gap on current upstream (v0.52.1)

In **serve mode**, calling a method on a component read from the `application`
scope **inside a custom tag** throws:

```
Variable is not a function or function 'getService' is not defined
```

even though the component is intact (its keys include the method, and the same
call from a regular page succeeds). Characterized:

| Context | result |
| --- | --- |
| Regular page: `application.svc.ping()` | ✅ works |
| Inside `cfmodule`/`cf_` custom tag: `application.svc.ping()` | ❌ "ping is not defined" |
| Inside custom tag, nested: `application.lib.db.getService(...)` | ❌ |
| Inside custom tag, locally created `createObject(...).ping()` | ✅ works |
| CLI mode (any of the above) | ✅ works |

Two further clues toward the cause:

- It is **serve-mode only** — the component is set in one lifecycle frame
  (`onApplicationStart`) and the failing call happens in another (the
  custom-tag frame). The CLI path, which evaluates in a single flat frame,
  works.
- Passing the component into the tag as an **attribute**
  (`<cf_probe svc="#application.svc#">`) makes the in-tag call work — evaluating
  the component at the call site binds it. This points at the method-reference
  binding being lost when the component is loaded *inside* the custom-tag frame
  (cf. the v0.52.1 "bind CFC method references at the load site" /
  "identity-guard method_this_writeback for chained CFC calls" work).

## Test

`tests/lifecycle/application_scope_custom_tag/` is a fixture app whose
`onApplicationStart` puts a CFC in the application scope; its `index.cfm` prints
`page=<result>;tag=<result>;`, calling the object's method directly (control)
and from inside a `cfmodule` custom tag. `tests/lifecycle/test_application_scope_custom_tag.cfm`
drives it over HTTP (skipping under the CLI runner, like the other lifecycle
tests). The control passes today; the custom-tag case is expected to fail until
the binding is preserved in custom-tag frames.

Verified against Lucee 7.0.2.106: the fixture returns `page=pong;tag=pong;`
(both succeed).
