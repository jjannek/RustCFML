# Tag Attribute And Tag-Form String Interpolation

Moopa control templates build Alpine bindings and element ids using tag-form
`cfset` strings such as:

```cfml
<cfset model = "#attributes.model_record#.#field_name#">
```

Lucee-compatible behavior is that quoted tag attribute strings preserve literal
text while evaluating `#...#` interpolation. Text before, between, and after
interpolation segments should remain part of the final string.

The added CFML test captures the Moopa-shaped `cfset` behavior without
prescribing an implementation strategy.
