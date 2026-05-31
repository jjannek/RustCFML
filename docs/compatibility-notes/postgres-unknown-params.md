# PostgreSQL UNKNOWN parameter targets

## Summary

PostgreSQL sometimes asks parameter encoders to encode a value for target type
`postgres::types::Type::UNKNOWN`, especially when the SQL expression does not
force a concrete type early enough.

Lucee/JDBC-style usage accepts this pattern in ordinary `queryExecute()` calls.
RustCFML should encode CFML values for `UNKNOWN` targets as text bytes.

## Example Shape

```cfml
queryExecute(
    "insert into audit_log (label, amount) values (?, ?)",
    [ "Admin", 250 ],
    { datasource = "app" }
);
```

In some statement shapes PostgreSQL can request `UNKNOWN` for a parameter before
the final target type is available to the client encoder.

## Expected Behavior

When `PgParam` is encoded for `Type::UNKNOWN`:

- text values encode as their UTF-8 text bytes
- integers stringify, for example `250` becomes `250`
- doubles stringify without unnecessary decimal noise, for example `250.0`
  becomes `250`

## Observed RustCFML Behavior

The default `ToSql` delegation for numeric/text parameters does not provide a
clean encoding path for `Type::UNKNOWN`, which can cause otherwise valid
PostgreSQL statements to fail during parameter binding.

## Suggested Fix Shape

Handle `Type::UNKNOWN` explicitly in `impl postgres::types::ToSql for PgParam`.
For example:

```rust
PgParam::Int(i) if *ty == postgres::types::Type::UNKNOWN => {
    write_pg_text(&i.to_string(), out)
}

PgParam::Double(d) if *ty == postgres::types::Type::UNKNOWN => {
    write_pg_text(&format_pg_numeric_f64(*d)?, out)
}

PgParam::Text(s) if *ty == postgres::types::Type::UNKNOWN => {
    write_pg_text(s, out)
}
```

where `write_pg_text()` appends the UTF-8 bytes and returns
`postgres::types::IsNull::No`.

## Moopa Port Context

This surfaced in framework-generated PostgreSQL calls where Moopa passes CFML
scalar values through generic save/query helpers and expects the database layer
to coerce them as Lucee/JDBC does.
