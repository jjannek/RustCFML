component {

	// Each method uses ONE multi-word CFML comparison operator. Lucee 5/6/7,
	// Adobe CF 2018-2025, and BoxLang parse and evaluate all of them. RustCFML
	// fails to PARSE any multi-word operator ("Expected RParen, found <next
	// token>"), which degrades the WHOLE component to a non-object at
	// instantiation. (Single-word operators -- IS, EQ, NEQ, GT, LT, GTE, LTE,
	// CONTAINS, AND, OR, NOT, MOD -- all parse fine on RustCFML; only the
	// multi-word/verbose forms fail.) Kept in a fixture so the parse failure is
	// contained and does not abort the run.

	function isNot() {
		return (1 IS NOT 2) ? "yes" : "no";
	}

	function doesNotContain() {
		return ("abc" DOES NOT CONTAIN "z") ? "yes" : "no";
	}

	function greaterThan() {
		return (5 GREATER THAN 3) ? "yes" : "no";
	}

}
