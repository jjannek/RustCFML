<cfscript>
suiteBegin("Operators: multi-word comparison operators");

// ============================================================
// Background
// ============================================================
// CFML offers verbose, multi-word aliases for its comparison operators.
// Lucee 5/6/7, Adobe ColdFusion 2018-2025, and BoxLang all accept them:
//
//     a IS NOT b                 (alias for NEQ / !=)
//     a DOES NOT CONTAIN b       (negated CONTAINS)
//     a GREATER THAN b           (alias for GT / >)
//     a LESS THAN b              (alias for LT / <)
//     a GREATER THAN OR EQUAL TO b   /   a LESS THAN OR EQUAL TO b
//     a EQUAL b                  (alias for EQ)
//     a NOT EQUAL b              (alias for NEQ)
//
// RustCFML supports every SINGLE-word operator (IS, EQ, NEQ, GT, LT, GTE,
// LTE, CONTAINS, AND, OR, NOT, MOD) but fails to PARSE every MULTI-word one:
//
//     Parse error: Expected RParen, found Identifier("NOT")   // for IS NOT
//     Parse error: Expected RParen, found Identifier("DOES")  // for DOES NOT CONTAIN
//
// Because a parse failure degrades the WHOLE component to a non-object
// SILENTLY, a CFC anywhere in a codebase that uses one of these operators
// becomes unusable. CFWheels/Wheels' Global.cfc uses `IS NOT` (x2) and
// `DOES NOT CONTAIN` (x1); each is on the boot path, so wheels.Global will
// not parse until these operators are supported (or the framework rewrites
// them to single-word forms).
//
// This suite exercises the operators through a fixture
// (MultiWordOperatorFixture) so the parse failure is contained -- it
// degrades that one component to a non-object instead of aborting the run.
// Each operator independently triggers the parse failure (verified in
// isolation); when the component cannot parse, all three assertions report
// "(non-object ...)". All assertions PASS on Lucee/ACF/BoxLang.
// ============================================================

ok = false;
probe = "";
try {
	probe = createObject("component", "MultiWordOperatorFixture");
	ok = isObject(probe);
} catch (any e) {
	ok = false;
}

assert("IS NOT parses and evaluates (1 IS NOT 2 -> true)",
	ok ? probe.isNot() : "(non-object: component failed to parse)", "yes");
assert("DOES NOT CONTAIN parses and evaluates ('abc' DOES NOT CONTAIN 'z' -> true)",
	ok ? probe.doesNotContain() : "(non-object: component failed to parse)", "yes");
assert("GREATER THAN parses and evaluates (5 GREATER THAN 3 -> true)",
	ok ? probe.greaterThan() : "(non-object: component failed to parse)", "yes");

suiteEnd();
</cfscript>
