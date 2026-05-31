# cfloop list item attribute

## Summary

Lucee accepts `cfloop` over a list with an `item` attribute even when `index`
is omitted. The `item` variable receives the current list element.

## Minimal CFML

```cfml
<cfset fields = "name,email,mobile">
<cfset result = "">

<cfloop list="#fields#" item="field_name">
    <cfset result = listAppend(result, uCase(field_name))>
</cfloop>

<cfoutput>#result#</cfoutput>
```

## Expected Lucee-Compatible Output

```text
NAME,EMAIL,MOBILE
```

## Observed RustCFML Behavior

The minimal repro appeared to hang current upstream when expressed as a runner
test, so this was not submitted as an executable test because it could hang CI.

## Compatibility Requirement

For list loops, `item` should be accepted as the current element binding. When
`index` is not present, the parser/runtime should still advance the list loop
and expose the current item value through the `item` variable.

## Moopa Port Context

Moopa uses this pattern when iterating field lists, for example assigning each
field name to a semantically named loop variable instead of using `index`.
