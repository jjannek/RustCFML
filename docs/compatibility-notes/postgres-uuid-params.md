# PostgreSQL UUID text parameters

## Summary

Moopa commonly passes UUID values into `queryExecute()` as CFML strings. Lucee's
JDBC path binds those values successfully for PostgreSQL `uuid` columns.

RustCFML should accept a string UUID parameter when PostgreSQL asks the
parameter encoder for `postgres::types::Type::UUID`.

## Example Shape

```cfml
queryExecute(
    "select * from profiles where id = ?",
    [ "41048aa7-27c9-4517-a93e-82bf7c76cc66" ],
    { datasource = "app" }
);
```

where `profiles.id` is a PostgreSQL `uuid` column.

## Expected Behavior

The string value should be parsed as a UUID and encoded using PostgreSQL's UUID
wire format. The query should execute without a parameter type error.

## Observed RustCFML Behavior

`PgParam::Text` is currently encoded as text for all target types. When the
target type is `Type::UUID`, PostgreSQL expects UUID wire encoding rather than a
plain text encoding.

## Suggested Fix Shape

In `impl postgres::types::ToSql for PgParam`, handle text parameters specially
when the target type is UUID:

```rust
PgParam::Text(s) if *ty == postgres::types::Type::UUID => {
    let uuid = uuid::Uuid::parse_str(s)?;
    uuid.to_sql(ty, out)
}
```

## Related QueryColumn Case

Moopa can also pass values derived from a query column. If a `CfmlValue` is a
`QueryColumn`, PostgreSQL parameter conversion should use the first row value,
matching scalar query-column coercion elsewhere.

## Moopa Port Context

This surfaced in sysadmin/security tables where UUID primary keys and foreign
keys are passed through framework save/load helpers as ordinary CFML string
values.
