<cfscript>
suiteBegin("Core: bare-call shadowing — own-frame data shadows UDFs, never builtins");

// Companion to test_bare_call_caller_stack_leak.cfm (PR #97). That suite pins
// the ANCESTOR-frame rule (caller data is invisible to a callee's bare-name
// call resolution). This one pins the OWN-frame rules, probed on Lucee 7:
//
//   1. A function's own param/var data DOES shadow a bare call to a
//      same-named method — Lucee throws "No matching function [FNA] found".
//   2. Builtin names are immune to data shadowing entirely (Lucee binds BIFs
//      at compile time): `function f(struct lcase = {}) { lcase("X") }`
//      calls the BIF even though the param is a struct.
//   3. A page-scope variable named like a builtin (`variables.log`,
//      `variables.len`) must READ as the variable, not resolve to the
//      builtin — guards the read/call split in the engine's resolution.

bcsProbe = createObject("component", "BareCallShadowProbe");

// (1) own-frame data shadows a same-named method: the bare call must throw.
assert("own struct param shadows bare call to same-named method (throws)",
	bcsProbe.ownParamShadows(), "THREW");
assert("own var shadows bare call to same-named method (throws)",
	bcsProbe.ownVarShadows(), "THREW");

// (2) builtins are immune to data shadowing — own frame and inherited.
assert("own struct param named lcase does not shadow the lcase() builtin",
	bcsProbe.ownBuiltinParam(), "abc");
assert("caller's struct param named ucase does not shadow callee's ucase() builtin",
	bcsProbe.viaInheritedBuiltinShadow(), "ABC");

// (3) page-scope variables named like builtins read as data, not functions.
variables.log = ["a", "b"];
assert("variables.log reads the array, not the log() builtin", arrayLen(variables.log), 2);
variables.len = "hello";
assert("variables.len reads the string, not the len() builtin", variables.len, "hello");

suiteEnd();
</cfscript>
