<cfscript>
suiteBegin("Core: component declaration-header attribute parsing");

// ============================================================
// Background
// ============================================================
// CFML component declarations carry zero or more metadata attributes in the
// header before the body brace, e.g. `component output="false" extends="Foo" {`.
// On Lucee 5/6/7, Adobe ColdFusion 2018-2025, and BoxLang these attributes are
// ORDER-INDEPENDENT and their values may be written quoted OR unquoted (a bare
// boolean keyword / identifier is a legal attribute value).
//
//   A. `extends` accepted after another attribute (order-independent header):
//        component extends="Base" output="false" {}   -> parses (control)
//        component output="false" extends="Base" {}   -> gap A
//      This is the dominant Wheels header shape — the entire boot cascade is
//      `component output="false" ... extends="wheels.Global" {`.
//
//   B. An UNQUOTED boolean attribute value:
//        component output="false" {}   -> parses (control)
//        component output=false {}     -> gap B
//      Wheels writes its database adapters this way:
//        component extends="wheels.databaseAdapters.Base" output=false {}
//
// The failing headers live in runtime-instantiated FIXTURE CFCs (not inline)
// because a parse error escapes try/catch and would abort the whole runner; via
// createObject the unparseable fixture degrades to a non-object instead.
// ============================================================

// Load a fixture and return its ping(); a sentinel if the header failed to parse.
function loadPing(required string name) {
	var o = createObject("component", arguments.name);
	return isObject(o) ? o.ping() : "NOT-A-COMPONENT";
}

// --- controls: header shapes RustCFML already accepts (regression guards) ----

assert("control: `extends` as the FIRST attribute parses", loadPing("ExtendsFirstFixture"), "pong");
assert("control: a quoted boolean attribute value parses", loadPing("QuotedBoolFixture"), "pong");

// --- gap A: `extends` after another attribute --------------------------------

assert("`extends` after another attribute parses (output=... extends=...)",
	loadPing("ExtendsAfterAttrFixture"), "pong");

// extends-after-attr must also WIRE the parent, not merely parse: the inherited
// whoAmI() comes from DeclAttrBase.
extendsAfter = createObject("component", "ExtendsAfterAttrFixture");
assert("`extends` after another attribute still links the parent",
	isObject(extendsAfter) ? extendsAfter.whoAmI() : "NOT-A-COMPONENT", "base");

// --- gap B: unquoted boolean attribute value ---------------------------------

assert("an unquoted boolean attribute value parses (output=false)",
	loadPing("UnquotedBoolFixture"), "pong");

suiteEnd();
</cfscript>
