# Custom Tag `attributeCollection`

Moopa renders form controls through nested custom tags. Those tags commonly pass
resolved control metadata through `attributeCollection`, while also supplying a
small number of explicit attributes at the call site.

Lucee-compatible behavior:

- `attributeCollection` merges the supplied struct into the custom tag
  `attributes` scope.
- The behavior applies to both `cfmodule` and `cf_` prefix custom tags.
- Explicit attributes override values supplied by `attributeCollection`.
- The source struct is not mutated.

The added CFML test captures that behavior without prescribing an implementation
strategy.
