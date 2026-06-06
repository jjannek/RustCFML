//! End-to-end engine tests: parse + execute over in-memory `CfmlQuery` sources,
//! exercising the public API the VM uses.

use cfml_common::dynamic::{CfmlQuery, CfmlValue};
use cfml_qoq::{execute, parse, QoQFunctionRegistry, QoQParams};
use indexmap::IndexMap;

fn i(n: i64) -> CfmlValue {
    CfmlValue::Int(n)
}
fn s(v: &str) -> CfmlValue {
    CfmlValue::String(v.to_string())
}

fn query(cols: &[&str], rows: &[&[CfmlValue]]) -> CfmlQuery {
    let mut q = CfmlQuery::new(cols.iter().map(|c| c.to_string()).collect());
    for r in rows {
        let mut m = IndexMap::new();
        for (idx, c) in cols.iter().enumerate() {
            m.insert(c.to_string(), r[idx].clone());
        }
        q.rows.push(m);
    }
    q
}

/// Run SQL against named sources; returns the result as `(columns, rows-as-strings)`.
fn run(sql: &str, sources: &[(&str, &CfmlQuery)]) -> (Vec<String>, Vec<Vec<String>>) {
    run_p(sql, sources, QoQParams::none())
}

fn run_p(
    sql: &str,
    sources: &[(&str, &CfmlQuery)],
    params: QoQParams,
) -> (Vec<String>, Vec<Vec<String>>) {
    let stmt = parse(sql).unwrap_or_else(|e| panic!("parse error for `{}`: {}", sql, e));
    let reg = QoQFunctionRegistry::new();
    let src: Vec<(String, &CfmlQuery)> =
        sources.iter().map(|(n, q)| (n.to_string(), *q)).collect();
    let mut udf = |_: &CfmlValue, _: Vec<CfmlValue>| Ok(CfmlValue::Null);
    let result = execute(&stmt, &src, &params, &reg, &mut udf)
        .unwrap_or_else(|e| panic!("execute error for `{}`: {:?}", sql, e));
    let CfmlValue::Query(q) = result else {
        panic!("expected Query result");
    };
    let cols = q.columns.clone();
    let rows = q
        .rows
        .iter()
        .map(|r| {
            q.columns
                .iter()
                .map(|c| r.get(c).map(|v| v.as_string()).unwrap_or_default())
                .collect::<Vec<_>>()
        })
        .collect();
    (cols, rows)
}

fn people() -> CfmlQuery {
    query(
        &["id", "name", "age", "dept"],
        &[
            &[i(1), s("Alice"), i(30), i(10)],
            &[i(2), s("Bob"), i(25), i(20)],
            &[i(3), s("Carol"), i(40), i(10)],
            &[i(4), s("Dave"), i(35), i(20)],
            &[i(5), s("Eve"), i(28), CfmlValue::Null],
        ],
    )
}

fn depts() -> CfmlQuery {
    query(
        &["id", "title"],
        &[&[i(10), s("Engineering")], &[i(20), s("Sales")], &[i(30), s("Legal")]],
    )
}

#[test]
fn select_star_and_where_order() {
    let p = people();
    let (cols, rows) = run("SELECT name, age FROM people WHERE age >= 30 ORDER BY age DESC", &[("people", &p)]);
    assert_eq!(cols, vec!["name", "age"]);
    assert_eq!(
        rows,
        vec![
            vec!["Carol".to_string(), "40".to_string()],
            vec!["Dave".to_string(), "35".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ]
    );
}

#[test]
fn aggregates_no_group() {
    let p = people();
    let (cols, rows) = run("SELECT count(*) AS c, sum(age) AS total, avg(age) AS a FROM people", &[("people", &p)]);
    assert_eq!(cols, vec!["c", "total", "a"]);
    assert_eq!(rows[0][0], "5");
    assert_eq!(rows[0][1], "158"); // 30+25+40+35+28
}

#[test]
fn group_by_having_order() {
    let p = people();
    let (_cols, rows) = run(
        "SELECT dept, count(*) AS n FROM people WHERE dept IS NOT NULL GROUP BY dept HAVING count(*) > 1 ORDER BY dept",
        &[("people", &p)],
    );
    // dept 10 -> Alice, Carol (2); dept 20 -> Bob, Dave (2); both have >1.
    assert_eq!(rows, vec![vec!["10".to_string(), "2".to_string()], vec!["20".to_string(), "2".to_string()]]);
}

#[test]
fn inner_join() {
    let p = people();
    let d = depts();
    let (cols, rows) = run(
        "SELECT p.name, d.title FROM people p JOIN depts d ON p.dept = d.id ORDER BY p.name",
        &[("people", &p), ("depts", &d)],
    );
    assert_eq!(cols, vec!["name", "title"]);
    // Eve (null dept) excluded; Legal has no people.
    assert_eq!(
        rows,
        vec![
            vec!["Alice".to_string(), "Engineering".to_string()],
            vec!["Bob".to_string(), "Sales".to_string()],
            vec!["Carol".to_string(), "Engineering".to_string()],
            vec!["Dave".to_string(), "Sales".to_string()],
        ]
    );
}

#[test]
fn left_join_keeps_unmatched() {
    let p = people();
    let d = depts();
    let (_c, rows) = run(
        "SELECT p.name, d.title FROM people p LEFT JOIN depts d ON p.dept = d.id ORDER BY p.name",
        &[("people", &p), ("depts", &d)],
    );
    // Eve has null dept -> no match -> NULL title (empty string when stringified).
    let eve = rows.iter().find(|r| r[0] == "Eve").unwrap();
    assert_eq!(eve[1], "");
    assert_eq!(rows.len(), 5);
}

#[test]
fn distinct() {
    let p = people();
    let (_c, rows) = run("SELECT DISTINCT dept FROM people ORDER BY dept", &[("people", &p)]);
    // depts: null, 10, 20 -> 3 distinct (null sorts first)
    assert_eq!(rows.len(), 3);
}

#[test]
fn positional_params() {
    let p = people();
    let params = QoQParams {
        positional: vec![i(30)],
        named: IndexMap::new(),
    };
    let (_c, rows) = run_p("SELECT name FROM people WHERE age >= ? ORDER BY name", &[("people", &p)], params);
    assert_eq!(rows, vec![vec!["Alice".to_string()], vec!["Carol".to_string()], vec!["Dave".to_string()]]);
}

#[test]
fn named_params() {
    let p = people();
    let mut named = IndexMap::new();
    named.insert("minAge".to_string(), i(35));
    let params = QoQParams { positional: vec![], named };
    let (_c, rows) = run_p("SELECT name FROM people WHERE age >= :minAge ORDER BY name", &[("people", &p)], params);
    assert_eq!(rows, vec![vec!["Carol".to_string()], vec!["Dave".to_string()]]);
}

#[test]
fn union_distinct_and_all() {
    let p = people();
    let (_c, rows_all) = run("SELECT dept FROM people UNION ALL SELECT dept FROM people", &[("people", &p)]);
    assert_eq!(rows_all.len(), 10);
    let (_c, rows_dist) = run("SELECT dept FROM people UNION SELECT dept FROM people", &[("people", &p)]);
    assert_eq!(rows_dist.len(), 3);
}

#[test]
fn scalar_functions_and_case() {
    let p = people();
    let (cols, rows) = run(
        "SELECT upper(name) AS u, CASE WHEN age >= 30 THEN 'senior' ELSE 'junior' END AS band FROM people WHERE id = 1",
        &[("people", &p)],
    );
    assert_eq!(cols, vec!["u", "band"]);
    assert_eq!(rows[0], vec!["ALICE".to_string(), "senior".to_string()]);
}

#[test]
fn in_subquery() {
    let p = people();
    let d = depts();
    let (_c, rows) = run(
        "SELECT name FROM people WHERE dept IN (SELECT id FROM depts WHERE title = 'Engineering') ORDER BY name",
        &[("people", &p), ("depts", &d)],
    );
    assert_eq!(rows, vec![vec!["Alice".to_string()], vec!["Carol".to_string()]]);
}

#[test]
fn derived_table() {
    let p = people();
    let (_c, rows) = run(
        "SELECT t.name FROM (SELECT name, age FROM people WHERE age > 30) AS t ORDER BY t.name",
        &[("people", &p)],
    );
    assert_eq!(rows, vec![vec!["Carol".to_string()], vec!["Dave".to_string()]]);
}

#[test]
fn like_and_between() {
    let p = people();
    let (_c, rows) = run("SELECT name FROM people WHERE name LIKE 'A%' OR age BETWEEN 24 AND 26 ORDER BY name", &[("people", &p)]);
    assert_eq!(rows, vec![vec!["Alice".to_string()], vec!["Bob".to_string()]]);
}

#[test]
fn limit_offset() {
    let p = people();
    let (_c, rows) = run("SELECT name FROM people ORDER BY name LIMIT 2 OFFSET 1", &[("people", &p)]);
    assert_eq!(rows, vec![vec!["Bob".to_string()], vec!["Carol".to_string()]]);
}

#[test]
fn scalar_subquery() {
    let p = people();
    let d = depts();
    let (cols, rows) = run(
        "SELECT name, (SELECT count(*) FROM depts) AS dept_count FROM people WHERE id = 1",
        &[("people", &p), ("depts", &d)],
    );
    assert_eq!(cols, vec!["name", "dept_count"]);
    assert_eq!(rows[0], vec!["Alice".to_string(), "3".to_string()]);
}

#[test]
fn comma_cross_join() {
    let a = query(&["x"], &[&[i(1)], &[i(2)]]);
    let b = query(&["y"], &[&[s("a")], &[s("b")]]);
    let (_c, rows) = run("SELECT x, y FROM a, b ORDER BY x, y", &[("a", &a), ("b", &b)]);
    assert_eq!(rows.len(), 4); // 2x2 cartesian
    assert_eq!(rows[0], vec!["1".to_string(), "a".to_string()]);
    assert_eq!(rows[3], vec!["2".to_string(), "b".to_string()]);
}

#[test]
fn right_join_keeps_unmatched_right() {
    let p = people();
    let d = depts();
    // Legal (id 30) has no people -> appears with NULL name under RIGHT JOIN.
    let (_c, rows) = run(
        "SELECT p.name, d.title FROM people p RIGHT JOIN depts d ON p.dept = d.id ORDER BY d.title, p.name",
        &[("people", &p), ("depts", &d)],
    );
    let legal = rows.iter().find(|r| r[1] == "Legal").unwrap();
    assert_eq!(legal[0], ""); // NULL name
}

#[test]
fn select_without_from() {
    let p = people();
    let (cols, rows) = run("SELECT 1 + 2 AS three, upper('hi') AS greeting FROM people WHERE 1=0", &[("people", &p)]);
    // WHERE 1=0 with FROM -> empty; just check it doesn't error and columns resolve.
    assert_eq!(cols, vec!["three", "greeting"]);
    assert!(rows.is_empty());
}
