<cfscript>
suiteBegin("Tags: cfscript transaction with attributes + body");

// ============================================================
// Background  (parse gap surfaced in PR #32 by bpamiri)
// ============================================================
// The cfscript `transaction` statement accepts space-separated tag attributes
// before its body, mirroring the angle-bracket <cftransaction action="..."> tag:
//   transaction action="begin" { ... }
// RustCFML supported the bare `transaction { ... }` form but rejected the
// attribute form ("Expected RBrace, found Semicolon"). Lucee/Adobe CF/BoxLang
// accept both. A transaction with no query inside is a no-op on both engines,
// so the body simply runs.
// ============================================================

function runAttrTxn() {
	var marker = "";
	transaction action="begin" {
		marker = "ran";
	}
	return marker;
}
assert("transaction action=begin executes its body", runAttrTxn(), "ran");

function runBareTxn() {
	var n = 0;
	transaction {
		n = 42;
	}
	return n;
}
assert("bare transaction executes its body (regression guard)", runBareTxn(), 42);

suiteEnd();
</cfscript>
