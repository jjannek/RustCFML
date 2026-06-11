# Database

[← Back to README](../README.md)

RustCFML runs parameterized queries via `queryExecute` (and the `<cfquery>` tag), with connection pooling, `cfqueryparam`, and `cftransaction`.

## Supported engines

| Engine | Driver | Feature flag | Notes |
|---|---|---|---|
| **SQLite** | `rusqlite` (bundled) | on by default | Zero-config; file-based or in-memory (`:memory:`). The default fallback datasource. |
| **MySQL / MariaDB** | `mysql` | `mysql_db` | MariaDB is wire-compatible and uses the same driver. |
| **PostgreSQL** | `postgres` (native, TLS) | `postgres_db` | See [Working with PostgreSQL](#working-with-postgresql) below. On Cloudflare Workers, reached via **Hyperdrive** (see below). |
| **Microsoft SQL Server** | `tiberius` | `mssql_db` | Also covers Azure SQL. |

Drivers are feature-gated so unused subsystems compile out. SQLite is on by default; enable the others at build time:

```bash
cargo build --release --features "mysql_db,postgres_db,mssql_db"
```

**Prebuilt release binaries include all four drivers.** On the WebAssembly / Cloudflare Workers build the native drivers are not compiled — datasource access goes through a **[Hyperdrive](https://developers.cloudflare.com/hyperdrive/)** binding instead (PostgreSQL via `postgres.js`, MySQL via `mysql2`). The `queryExecute` API and parameter handling are identical, so application code is portable between the native server and the edge build. See [PostgreSQL on Cloudflare Workers](#postgresql-on-cloudflare-workers).

## Datasources

Define datasources in `.cfconfig.json` (see **[Configuration](configuration.md)**), then reference them by name:

```cfml
qry = queryExecute(
    "select id, name from users where active = ?",
    [ true ],
    { datasource = "app" }
);
```

If no `datasource` is given, the default datasource is used, falling back to an in-memory SQLite database (`:memory:`) so quick scripts and tests work with zero configuration.

## Parameters

Both placeholder styles are supported:

```cfml
// Positional — array of values
queryExecute("select * from users where id = ? and active = ?", [ 42, true ]);

// Named — struct of values (:name)
queryExecute(
    "select * from users where id = :id and active = :active",
    { id = 42, active = true }
);

// cfqueryparam-style array of structs (with optional type hints / list expansion)
queryExecute(
    "select * from users where id in (?)",
    [ { value = "1,2,3", list = true } ]
);
```

A query column passed as a parameter (e.g. `someQuery.id`) binds as its first-row scalar value, matching scalar query-column coercion elsewhere in the language.

## Return types

```cfml
queryExecute(sql, params, { returntype = "query" });   // default — a query object
queryExecute(sql, params, { returntype = "array" });   // array of row structs
queryExecute(sql, params, { returntype = "struct", columnkey = "id" }); // keyed struct
```

## Transactions

```cfml
cftransaction {
    queryExecute("update accounts set balance = balance - ? where id = ?", [ 100, 1 ]);
    queryExecute("update accounts set balance = balance + ? where id = ?", [ 100, 2 ]);
}
```

---

## Working with PostgreSQL

PostgreSQL is well supported, but it is stricter on the wire than the other engines — it uses `$1`/`$2` positional placeholders and binds parameters in **binary** format. RustCFML handles the translation so ordinary CFML works, but a few behaviours are worth knowing.

### Placeholders are translated automatically

You write CFML-style `?` (or `:name`) placeholders; RustCFML rewrites them to PostgreSQL's `$1`, `$2`, … form. Quoted string literals and `::type` casts are left untouched.

### Untyped CFML strings are coerced to the column type

CFML values are frequently strings (form and URL values are untyped). Because PostgreSQL binds parameters in binary, RustCFML encodes each parameter at the **target column's type** — so a string `"251"` binds correctly to an `int4` column, `"3.14"` to `numeric`, `"true"` to `boolean`, and so on. Integers and floats are likewise encoded at the column's exact width (e.g. `int2`/`int4`/`int8`), avoiding "incorrect binary data format" errors.

### UUID string parameters

A CFML string can be bound directly to a `uuid` column — it is parsed and sent in PostgreSQL's UUID wire format:

```cfml
queryExecute(
    "select * from profiles where id = ?",
    [ "41048aa7-27c9-4517-a93e-82bf7c76cc66" ],   // string -> uuid column
    { datasource = "app" }
);
```

### `UNKNOWN`-typed parameters

When PostgreSQL can't pin a concrete parameter type early (for example a bare `select ?`), RustCFML encodes the value as text and lets the server coerce it — so framework-generated statements that don't force a type still work.

### Multi-statement mutations

PostgreSQL's protocol runs one parameterized statement per round-trip. RustCFML splits a multi-statement, non-`SELECT` mutation into its component statements (respecting quoted literals and comments), renumbers each statement's placeholders from `$1`, runs them in order, and returns the **total** affected row count. This makes framework "delete existing rows + insert replacements" operations work as a single `queryExecute` call:

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

If the number of supplied parameters doesn't match what the statements consume, RustCFML raises a clear error rather than binding the wrong values.

### PostgreSQL on Cloudflare Workers

The same `queryExecute` API works in the WebAssembly/Cloudflare Workers build, where PostgreSQL is reached through a [Hyperdrive](https://developers.cloudflare.com/hyperdrive/) binding (via `postgres.js`). The placeholder rewriting and multi-statement handling described above apply there too, so application code is portable between the native server and the edge build. See **[RustCFML-Cloudflare-worker](https://github.com/RustCFML/RustCFML-Cloudflare-worker)**.

## Query-of-Queries

Pass `dbtype="query"` to run an in-memory SQL `SELECT` over query variables already in scope — no datasource, no driver, no JDBC. The engine lives in `crates/cfml-qoq` and is pure Rust; it parallelises filter/projection/sort across cores (non-wasm).

```cfm
<cfscript>
employees = queryExecute("SELECT * FROM users WHERE active = 1");  // real DB
top = queryExecute(
    "SELECT name, salary FROM employees WHERE salary > :min ORDER BY salary DESC",
    { min: 50000 },
    { dbtype: "query" }                                              // QoQ
);
</cfscript>
```

Supported: `SELECT` (with `*`, `table.*`, aliases), `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY` (multi-key, ASC/DESC), `DISTINCT`, `LIMIT`/`OFFSET`; `INNER`/`LEFT`/`RIGHT`/`FULL [OUTER] JOIN ... ON`, `CROSS` and comma joins; `UNION` / `UNION ALL`; `IN (SELECT ...)`, scalar subqueries in the SELECT list, derived `FROM (SELECT ...) AS t`; `CASE`, `CAST`/`CONVERT`, `BETWEEN`, `LIKE [ESCAPE]`, `IS [NOT] NULL`; positional `?` and named `:name` params (incl. `cfqueryparam`); `returntype` `query`/`array`/`struct`. Extensible — `register_native_qoq_fn` exposes a Rust function as both a BIF and a QoQ function; `queryRegisterFunction(name, udf[, "aggregate"])` registers a CFML UDF for use in SQL.

Following BoxLang, RustCFML's QoQ is a **strict superset** of Lucee's: `LIMIT`/`OFFSET`, scalar subqueries, derived tables and `CASE` are accepted here but rejected by Lucee QoQ (it uses `TOP`). Same input → more accepted; not a wrong-result divergence — but SQL that uses those features is not portable back to Lucee. Correlated subqueries are **not** supported (subqueries run once, uncorrelated). See **[Known Issues §9](known-issues.md)** for the full superset table.

Performance (1M-row source, [bdw429s/cfml-qoq-perf-tests](https://github.com/bdw429s/cfml-qoq-perf-tests), 5-run median, same machine, lower is better):

| Engine | Total (ms) | vs RustCFML |
|---|---:|---:|
| **RustCFML** v0.112 | **1,116** | **1.00×** |
| BoxLang 1.14 | 1,368 | 1.23× slower |
| Lucee 7.0.4 | 7,884 | 7.1× slower |
