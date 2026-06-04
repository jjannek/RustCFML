# Tag Attribute String Interpolation (cfthrow / cfargument / cffile and peers)

Lucee-compatible behavior is that a quoted tag-attribute value evaluates
`#...#` interpolation while preserving the literal text before, between, and
after each interpolated segment — and this holds for **every** tag, not just a
favored subset.

Moopa hit this with messages, defaults, and file paths that combine literal
text with interpolated variables:

```cfml
<cfthrow message="APP_NAME '#application.app_name#' does not match an app directory at /apps/#application.app_name#." />
<cfargument name="path" default="/apps/#request.app_name#/routes" />
<cffile action="read" file="#local.filePath#" variable="local.jsonContent" />
```

The local workaround rewrote every such string into explicit `&` concatenation
(or, for `cffile`, wrapped the whole path in one `#expr#`). The added CFML test
captures the Moopa-shaped behavior without prescribing an implementation
strategy.

## Observed gap on current upstream (v0.52.1)

Interpolation is applied inconsistently across tags:

| Tag attribute | bare `#x#` interpolation | notes |
| --- | --- | --- |
| `<cfset x = "...">` / `cfif` / `cfreturn` | ✅ works | control |
| `<cfparam default="...">` | ✅ works | control |
| `<cfqueryparam value="...">` | ✅ works | fixed previously |
| `<cfinclude template="...">` | ✅ works | control |
| `<cfdirectory directory="...">` | ✅ works | control |
| `<cfthrow message=/type=/detail=>` | ❌ emitted as literal text | single-quote-wrapped `'#x#'` does interpolate |
| `<cfargument default="...">` | ❌ literal, or mis-parsed as an expression | |
| `<cffile file="...">` | ❌ single-var path becomes a literal; literal path fails to parse | only a full `#expr#` works |
| `<cfcookie value="...">` | ❌ mis-parsed as an expression | not asserted in the test (cookie scope is request-time only) |
| `<cfmail subject="...">` | ❌ mis-parsed as an expression | not asserted (requires a mail server) |

A telling inconsistency: `<cfparam default="/apps/#x#/cfg">` interpolates
correctly, but `<cfargument default="/apps/#x#/cfg">` (the same value shape) does
not. Single-quote-wrapping the interpolated segment (`'#x#'`) is the reliable
escape hatch on current upstream.

## Where the divergence lives

The tag preprocessor (`crates/cfml-compiler/src/tag_parser.rs`) dispatches a
separate hand-written arm per tag, and those arms emit attribute values through
**different** helpers:

- **Correct (Lucee parity):** `cfparam` / `cfqueryparam` / custom-tag attrs use
  `format_attr_value()`, which splits a quoted value into literal segments and
  `#expr#` segments and emits `"lit" & (expr) & "lit"`.
- **Affected:** `cfthrow`, `cfargument`, and `cffile` (in `parse_cffile_tag`)
  use `strip_hashes()` plus ad-hoc re-quoting. `strip_hashes` removes the `#`
  delimiters from a bare `#expr#`, so the segment is re-emitted as flat literal
  text. (`cffile` additionally guesses literal-vs-expression from whether the
  stripped value contains `.`/`(`, so a single-variable path like `#filePath#`
  is quoted as the literal `"filePath"`, and a literal `/a/b.cfm` is parsed as a
  division expression.) `'#x#'` survives only because `strip_hashes` preserves
  hashes inside an inner string literal.

So routing these attributes through `format_attr_value()` — as `cfparam`
already does — is the natural direction.

The test asserts the Lucee-compatible result. The control cases pass today; the
`cfthrow` / `cfargument` / `cffile` cases are expected to fail on current
upstream until tag-attribute interpolation is applied uniformly. Verified
against Lucee 7.0.2.106 (all assertions pass).
