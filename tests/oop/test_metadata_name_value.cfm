<cfscript>
suiteBegin("OOP: getMetadata().name reflects component identity");

// ============================================================
// Background
// ============================================================
// CFML reflects a component's identity through `getMetadata(cfc).name`
// (and equivalently `getComponentMetadata(cfc).name`). The contract is
// that `name` is a non-empty string that uniquely identifies the
// component class — typically the dotted path used to load it, or the
// fully-qualified name derived from its filesystem location.
//
// Frameworks rely on this:
//   - CFWheels/Wheels uses `getMetadata(this).name` to derive table
//     names from model class names (a `User` model → `users` table).
//   - DI containers use it to dedupe singleton registrations.
//   - Logging / error reporting includes class names for diagnostics.
//
// Lucee and Adobe ColdFusion both return the dotted path the component
// was loaded under (e.g. "oop.Greeter"). BoxLang returns the simple
// classname.
//
// The minimum contract this test asserts:
//   1. `name` is non-empty
//   2. `name` is NOT the literal string "Anonymous" — that's a sentinel
//      meaning the engine failed to track the class identity
//   3. The last segment of the name matches the .cfc filename
//
// The existing tests/oop/test_metadata.cfm only checks that the `name`
// KEY exists on the metadata struct — it never asserts the VALUE. A
// component whose name resolves to "Anonymous" passes the existing
// check, masking a regression.
// ============================================================

g = createObject("component", "oop.Greeter").init();
md = getMetadata(g);

// 1. structure sanity
assertNotNull("getMetadata returns non-null", md);
assertTrue("metadata is a struct", isStruct(md));
assertTrue("metadata has .name key", structKeyExists(md, "name"));

// 2. name is non-empty
assertTrue("metadata.name is non-empty",
    len(toString(md.name)) > 0);

// 3. name is not the sentinel "Anonymous"
assertFalse("metadata.name is not literally 'Anonymous'",
    compareNoCase(toString(md.name), "Anonymous") == 0);

// 4. last segment of dotted name matches the .cfc filename
// Works on Lucee/Adobe (returns "oop.Greeter") and BoxLang (returns
// "Greeter") — we look at the last list element either way.
assert("metadata.name ends with 'Greeter'",
    listLast(md.name, "."), "Greeter");

// ------------------------------------------------------------
// Inherited component — same contract
// ------------------------------------------------------------
d = createObject("component", "oop.Dog").init();
dmd = getMetadata(d);

assertTrue("inherited metadata has .name key",
    structKeyExists(dmd, "name"));

assertFalse("inherited metadata.name is not 'Anonymous'",
    compareNoCase(toString(dmd.name), "Anonymous") == 0);

assert("inherited metadata.name ends with 'Dog'",
    listLast(dmd.name, "."), "Dog");

// ------------------------------------------------------------
// fullname — the fully-qualified dotted component path. Lucee and
// Adobe expose `fullname` alongside `name`; frameworks rely on it
// (Wheels' Mapper.cfc keys its router mix-ins off getMetaData().fullname).
// getComponentMetadata() exposes the same key.
// ------------------------------------------------------------
assertTrue("metadata has .fullname key", structKeyExists(md, "fullname"));
assertTrue("metadata.fullname is non-empty", len(toString(md.fullname)) > 0);
assert("metadata.fullname ends with 'Greeter'",
    listLast(md.fullname, "."), "Greeter");

cmd = getComponentMetadata(g);
assertTrue("getComponentMetadata has .fullname key", structKeyExists(cmd, "fullname"));
assert("getComponentMetadata.fullname ends with 'Greeter'",
    listLast(cmd.fullname, "."), "Greeter");

suiteEnd();
</cfscript>
