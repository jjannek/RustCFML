# `cfqueryparam value` With Interpolated Text

Moopa search and filter queries use `cfqueryparam` values that combine literal
text with interpolated variables, commonly wildcard search parameters:

```cfml
<cfqueryparam value="%#url.q#%" cfsqltype="cf_sql_varchar">
```

Lucee-compatible behavior is that the quoted `value` attribute is evaluated as
a string with literal `%` characters preserved and `#url.q#` interpolated.

The added CFML test keeps the raw syntax that exposed the compatibility gap.
It is expected to fail during parsing on current upstream until the syntax is
supported.
