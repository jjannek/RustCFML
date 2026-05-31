# PostgreSQL multi-statement mutations

## Summary

Moopa sometimes sends multi-statement mutation SQL through `queryExecute()`.
Lucee/JDBC handles this style for framework-generated delete/insert replacement
operations.

RustCFML's PostgreSQL path currently rewrites all parameters globally and calls
`client.execute()` once. The `postgres` crate expects one parameterized
statement per `execute()` call, so multi-statement mutations can fail.

## Example Shape

```cfml
queryExecute(
    "
        delete from profile_role where profile_id = ?;
        insert into profile_role (profile_id, role_id) values (?, ?);
    ",
    [ profileId, profileId, roleId ],
    { datasource = "app" }
);
```

## Expected Behavior

For non-select PostgreSQL SQL containing multiple statements, RustCFML should
execute each statement in order and return a mutation result using the total
affected row count.

## Observed RustCFML Behavior

The PostgreSQL adapter rewrites the full SQL to positional placeholders once,
for example:

```sql
delete from profile_role where profile_id = $1;
insert into profile_role (profile_id, role_id) values ($2, $3);
```

It then calls `client.execute()` once with all parameters. This does not work
for multi-statement parameterized execution in the `postgres` crate.

## Suggested Fix Shape

For mutation SQL only:

1. Split the SQL into statements on semicolons while respecting single-quoted
   string literals.
2. For each statement, count the positional `?` placeholders outside quoted
   strings.
3. Slice the ordered params for that statement.
4. Rewrite that statement's placeholders starting again at `$1`.
5. Execute each statement separately with its own parameter slice.
6. Sum affected row counts.
7. Fail clearly if the consumed parameter count does not match the available
   ordered params.

Example rewrite:

```sql
delete from profile_role where profile_id = ?
```

becomes:

```sql
delete from profile_role where profile_id = $1
```

and:

```sql
insert into profile_role (profile_id, role_id) values (?, ?)
```

becomes:

```sql
insert into profile_role (profile_id, role_id) values ($1, $2)
```

## Moopa Port Context

This surfaced when saving relationship fields, where Moopa clears existing
bridge-table rows and inserts replacement rows as one framework-generated
mutation operation.
