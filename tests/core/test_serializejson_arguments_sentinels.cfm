<cfscript>
suiteBegin("Core: SerializeJSON must not leak internal arguments-scope sentinel keys");

// Background: RustCFML tags an arguments-derived struct with internal sentinel
// keys (__arguments_scope, __arguments_params) — and structKeyList / structCount
// / structKeyExists / for-in all correctly FILTER them. But SerializeJSON does
// NOT filter them, so a struct built by structAppend(s, arguments) serializes
// the sentinels into the JSON. Lucee has no such keys.
//
//   function f() { var s = {}; structAppend(s, arguments); return serializeJSON(s); }
//   f(from="a", template="welcome")
//     RustCFML 0.161.0 -> {"from":"a","template":"welcome","__arguments_scope":true,"__arguments_params":[]}
//     Lucee 5.4.8.2    -> {"template":"welcome","from":"a"}
//
// Why it matters for Wheels: controller sendEmail() / many helpers build option
// structs by copying the arguments scope, then serialize/spool them — the
// leaked sentinels corrupt the output. The fix is the same filtering
// structKeyList/Count/Exists/for-in already apply, extended to SerializeJSON.

function sjasBuild() {
    var s = {};
    structAppend(s, arguments);
    return serializeJSON(s);
}
sjasJson = sjasBuild(from = "a", template = "welcome");

assertFalse("serializeJSON of an arguments-derived struct omits __arguments_scope",
    findNoCase("__arguments_scope", sjasJson) gt 0);
assertFalse("serializeJSON of an arguments-derived struct omits __arguments_params",
    findNoCase("__arguments_params", sjasJson) gt 0);
assertTrue("the real keys are present", findNoCase("welcome", sjasJson) gt 0 && findNoCase("from", sjasJson) gt 0);

// CONTROL (green on both): the struct funcs already filter the sentinels.
function sjasKeys() { var s = {}; structAppend(s, arguments); return structKeyList(s); }
sjasKl = sjasKeys(from = "a", template = "welcome");
assertFalse("CONTROL: structKeyList already filters __arguments_scope", listFindNoCase(sjasKl, "__arguments_scope") gt 0);

// --- The arguments scope is a HYBRID array/struct (Lucee/ACF parity) ---------
// MockBox-style call capture does arrayAppend(log, arguments), then TestBox's
// equalize() compares the stored scope to an expected array via isArray() +
// arrayLen() + indexing. Lucee's arguments scope reports isArray=true (for
// positional, named AND empty arg lists) so that comparison takes the array
// branch and matches; ours used to report false and leaked the sentinel keys
// through the struct branch. (Preside TestBox suite, the doNext/webflow specs.)
function asCapturePos() { var log = []; arrayAppend(log, arguments); return log; }
asPos = asCapturePos("next", "ID123");
assertTrue("isArray(arguments) is true for a positional arguments scope", isArray(asPos[1]));
assert("arrayLen(arguments) counts positional args", arrayLen(asPos[1]), 2);
assertTrue("arrayIsDefined(arguments, 2) is true", arrayIsDefined(asPos[1], 2));
assertFalse("arrayIsDefined(arguments, 3) is false", arrayIsDefined(asPos[1], 3));
assert("arguments scope still indexes positionally as an array", asPos[1][1], "next");

function asCaptureNamed(required any state) { var log = []; arrayAppend(log, arguments); return log; }
asNamed = asCaptureNamed({ test: true });
assertTrue("isArray(arguments) is true for a named arguments scope", isArray(asNamed[1]));
assert("arrayLen(arguments) counts named args", arrayLen(asNamed[1]), 1);
assertTrue("named arguments scope is still a struct too", isStruct(asNamed[1]));

function asCaptureEmpty() { var log = []; arrayAppend(log, arguments); return log; }
asEmpty = asCaptureEmpty();
assertTrue("isArray(arguments) is true for an empty arguments scope", isArray(asEmpty[1]));
assert("arrayLen of an empty arguments scope is 0", arrayLen(asEmpty[1]), 0);

// A plain numeric-keyed struct is NOT an array (no __arguments_scope marker).
plainNumeric = { "1": "a", "2": "b" };
assertFalse("a plain numeric-keyed struct is not an array", isArray(plainNumeric));

suiteEnd();
</cfscript>
