<cfscript>
suiteBegin("Core: isDefined('variables.x') resolves to the component scope");

// Regression: inside a component method, `isDefined("variables.x")` must check
// the component `variables` scope (__variables) — where pseudo-constructor and
// inherited-method writes live — not the function-local frame. Previously it
// checked the local frame and wrongly returned false, so Wheels' SQLite
// migrator (sqlTypes map set in the subclass body, read by an inherited
// typeToSQL) emitted columns with NO type, breaking 11 migrationSpec specs.

_c = new core.IsDefinedVarsChild();

// Set by the child pseudo-ctor, read by an INHERITED parent method:
assert("inherited method sees child-ctor var via isDefined", _c.probe(), "Y");
assert("inherited method resolves the mapped value", _c.lookup("date"), "TEXT");
assert("inherited method resolves a second mapped value", _c.lookup("integer"), "INTEGER");

// Same scope seen by a child-defined method:
assert("child method sees child-ctor var via isDefined", _c.childProbe(), "Y");

// Page-scope isDefined('variables.x') must still work.
variables.pageOnly = "present";
assert("page-scope isDefined variables.x still works", isDefined("variables.pageOnly") ? "Y" : "N", "Y");
assert("page-scope isDefined for a missing var is false", isDefined("variables.nope_xyz") ? "Y" : "N", "N");

suiteEnd();
</cfscript>
