<cfscript>
suiteBegin("Core: invoke() delivers undeclared argument-struct keys");

// ============================================================
// Background  (companion to test_invoke_canonical_forms.cfm; sibling of the
//              argumentCollection-spread suite)
// ============================================================
// The argument struct handed to the positional BIF
// invoke(objectOrName, method, argStruct) is a NAMED-ARGUMENT COLLECTION:
// Lucee 5/6/7 and Adobe ColdFusion deliver EVERY key to the callee's
// arguments scope — declared parameter or not, and even when the target
// method declares no parameters at all. It is the same contract the engines
// already honor for undeclared named args on a direct call
// (test_undeclared_named_args.cfm covers that direct-call flavor).
//
// RustCFML binds only DECLARED parameter names when marshaling the invoke()
// argument struct; undeclared keys are silently dropped, and a paramless
// target receives an EMPTY arguments scope:
//
//   function declared(string x = "(default)") {...}   // $locked undeclared
//   invoke(o, "declared", { x: "hello", "$locked": true })
//     Lucee 5.4.8.2    -> x=hello, $locked present
//     RustCFML 0.105.0 -> x=hello, $locked DROPPED
//
//   function paramless() {...}
//   invoke(o, "paramless", { x: 1, "$locked": true })
//     Lucee 5.4.8.2    -> both keys present
//     RustCFML 0.105.0 -> arguments scope EMPTY
//
// The CONTROLS below already agree on both engines and point at the fix
// shape: a direct named-arg call obj.m(argumentCollection = st) and an
// in-context dynamic dispatch this[name](argumentCollection = st) both
// deliver ALL keys — only the invoke() marshaling path filters by the
// declared parameter list. (The `argumentCollection` key nested INSIDE the
// invoke() argument struct is a SEPARATE contract covered by its own suite;
// this one is about plain keys.)
//
// Wheels rides the undeclared-key contract on every request: $simpleLock()
// re-enters its callback through $invoke() with a "$locked" guard key, and
// the guarded method ($readFlash, ...) checks
// StructKeyExists(arguments, "$locked") to know it already holds the lock
// (vendor/wheels/Global.cfc). When the key is dropped the guard never trips,
// $readFlash -> $simpleLock -> $readFlash recurses to depth 256, and every
// request 500s.
// ============================================================

fixture = createObject("component", "InvokeUndeclaredKeysFixture");

// --- CONTROL: declared params bind through invoke() on both engines ---
// Guards the wiring: if this fails, invoke() itself is broken, not the
// undeclared-key marshaling under test.
assert("CONTROL: declared param binds through invoke()",
	invoke(fixture, "declared", { x: "hello" }),
	"x=hello|hasLocked=false");

// --- the gap: an undeclared key must arrive alongside a declared param ---
assert("undeclared key is delivered alongside a declared param",
	invoke(fixture, "declared", { x: "hello", "$locked": true }),
	"x=hello|hasLocked=true");

// --- the gap: a paramless target must receive ALL keys ---
assert("paramless target receives every argument-struct key",
	invoke(fixture, "paramless", { x: 1, "$locked": true }),
	"hasX=true|hasLocked=true");

// --- the gap, in the exact Wheels guard shape ---
assert("re-entry guard keyed on an undeclared $-prefixed key trips",
	invoke(fixture, "guarded", { "$locked": true }),
	"LOCKED-OK");

// --- CONTROL: direct named-arg call delivers ALL keys on both engines ---
args = { x: 9, "$locked": true };
assert("CONTROL: obj.m(argumentCollection = st) delivers undeclared keys",
	fixture.paramless(argumentCollection = args),
	"hasX=true|hasLocked=true");

// --- CONTROL: in-context dynamic dispatch delivers ALL keys on both engines ---
assert("CONTROL: this[name](argumentCollection = st) delivers undeclared keys",
	fixture.callViaThisBracket("paramless", args),
	"hasX=true|hasLocked=true");
</cfscript>

<!--- --- the gap, tag form: <cfinvoke> extra attributes are the same
	named-argument collection, so undeclared ones must arrive too
	(surfaced independently by PR 106, credit Blute). Plain attribute
	names only: Lucee parse-rejects `$` inside a tag attribute name. --- --->
<cfinvoke component="#fixture#" method="declaredPlain" returnvariable="tagDeclared"
	x="hello" extra="1" />
<cfinvoke component="#fixture#" method="paramlessPlain" returnvariable="tagParamless"
	x="1" extra="1" />

<cfscript>
assert("cfinvoke tag delivers an undeclared attribute alongside a declared param",
	tagDeclared, "x=hello|hasExtra=true");
assert("cfinvoke tag delivers every attribute to a paramless target",
	tagParamless, "hasX=true|hasExtra=true");

suiteEnd();
</cfscript>
