//! Regression sweep: boolean tag attributes must EVALUATE `#expr#` (Lucee
//! parity), not literal-match the raw attribute text at parse time.
//!
//! The cfcookie `secure`/`httponly` arm and the cfcontent `reset` arm both
//! special-cased the attribute, comparing the RAW string to "true"/"yes" at
//! parse time — so `secure="#myBool#"` was seen as the literal `#myBool#`,
//! never matched, and the flag was silently dropped (always `false`). Both now
//! route interpolated values through `format_attr_value` so the expression is
//! compiled in and evaluated at runtime.
//!
//! The remaining boolean-attr tags (cflocation, cfdirectory, cfzip, cfsetting,
//! cflock) already had an `else => format_attr_value(...)` fallback, so an
//! `#expr#` value never matched the literal true/yes/false/no arms and fell
//! through to interpolation. These tests lock that in.

use cfml_compiler::tag_parser::tags_to_script;

/// A `#myBool#` value compiles to the bare expression `myBool`
/// (single_hash_expr preserves native type), never the literal `false`.
fn evaluates_expr(script: &str, key: &str) -> bool {
    script.contains(&format!("{}: myBool", key)) && !script.contains(&format!("{}: false", key))
}

#[test]
fn cfcookie_secure_httponly_expr_compiles_to_expression() {
    let s = tags_to_script(r##"<cfcookie name="c" value="v" secure="#myBool#" httponly="#myBool#">"##);
    assert!(evaluates_expr(&s, "secure"), "cfcookie secure: {}", s);
    assert!(evaluates_expr(&s, "httponly"), "cfcookie httponly: {}", s);
}

#[test]
fn cfcookie_secure_literal_still_boolean() {
    let s = tags_to_script(r##"<cfcookie name="c" value="v" secure="true" httponly="false">"##);
    assert!(s.contains("secure: true"), "literal true: {}", s);
    assert!(s.contains("httponly: false"), "literal false: {}", s);
}

#[test]
fn cfcontent_reset_expr_compiles_to_expression() {
    let s = tags_to_script(r##"<cfcontent reset="#myBool#" type="application/json">"##);
    assert!(evaluates_expr(&s, "reset"), "cfcontent reset: {}", s);
}

#[test]
fn cfcontent_reset_literal_still_boolean() {
    let s = tags_to_script(r##"<cfcontent reset="true">"##);
    assert!(s.contains("reset: true"), "literal reset: {}", s);
}

#[test]
fn cflocation_addtoken_expr_compiles_to_expression() {
    let s = tags_to_script(r##"<cflocation url="/x" addtoken="#myBool#">"##);
    assert!(evaluates_expr(&s, "addtoken"), "cflocation addtoken: {}", s);
}

#[test]
fn cfdirectory_recurse_expr_compiles_to_expression() {
    let s = tags_to_script(r##"<cfdirectory action="list" directory="." name="q" recurse="#myBool#">"##);
    assert!(evaluates_expr(&s, "recurse"), "cfdirectory recurse: {}", s);
}

#[test]
fn cfsetting_enablecfoutputonly_expr_compiles_to_expression() {
    let s = tags_to_script(r##"<cfsetting enablecfoutputonly="#myBool#">"##);
    assert!(evaluates_expr(&s, "enablecfoutputonly"), "cfsetting: {}", s);
}

#[test]
fn cflock_expr_attr_compiles_to_expression() {
    let s = tags_to_script(r##"<cflock name="l" timeout="5" throwontimeout="#myBool#">x</cflock>"##);
    assert!(evaluates_expr(&s, "throwontimeout"), "cflock throwontimeout: {}", s);
}
