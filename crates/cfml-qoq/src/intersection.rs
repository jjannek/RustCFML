//! Join-aware intersection builder.
//!
//! An *intersection* is a `Vec<usize>` of 1-based row indices, one per table in
//! FROM order (`0` = NULL sentinel for an unmatched outer-join side). The
//! builder seeds with the first table's rows and folds each JOIN in turn,
//! filtering by its `ON` predicate (evaluated by a caller-supplied closure that
//! holds the table data + expression evaluator). This is the Rust analogue of
//! BoxLang's `QoQIntersectionGenerator`.

use crate::ast::JoinType;

/// Build the set of row intersections for `table_rows` (row count per table,
/// FROM order) joined per `join_types` (one entry per table after the seed).
///
/// `eval_on(join_index, candidate)` returns whether `candidate` (a 1-based
/// intersection of width `join_index + 2`) satisfies that join's `ON` predicate;
/// callers return `true` for CROSS / comma joins (no `ON`).
pub fn build_intersections<F>(
    table_rows: &[usize],
    join_types: &[JoinType],
    max_size: usize,
    mut eval_on: F,
) -> Result<Vec<Vec<usize>>, usize>
where
    F: FnMut(usize, &[usize]) -> bool,
{
    if table_rows.is_empty() {
        return Ok(Vec::new());
    }

    // Seed with the first table's rows.
    let mut inters: Vec<Vec<usize>> = (1..=table_rows[0]).map(|r| vec![r]).collect();

    for (k, &jt) in join_types.iter().enumerate() {
        let right_rows = table_rows[k + 1];
        // Guard against materialising an unbounded cross-product (e.g. an
        // N-table comma join over a huge table). The pre-fold upper bound is
        // exact for CROSS and a worst case for filtered joins, so ON-filtered
        // joins (which keep `inters` small) are not penalised.
        let upper_bound = inters.len().saturating_mul(right_rows);
        if upper_bound > max_size {
            return Err(upper_bound);
        }
        let width = k + 1; // number of tables already in each intersection
        let mut next: Vec<Vec<usize>> = Vec::new();

        match jt {
            JoinType::Inner | JoinType::Cross => {
                for inter in &inters {
                    for r in 1..=right_rows {
                        let mut cand = inter.clone();
                        cand.push(r);
                        if eval_on(k, &cand) {
                            next.push(cand);
                        }
                    }
                }
            }
            JoinType::Left => {
                for inter in &inters {
                    let mut matched = false;
                    for r in 1..=right_rows {
                        let mut cand = inter.clone();
                        cand.push(r);
                        if eval_on(k, &cand) {
                            next.push(cand);
                            matched = true;
                        }
                    }
                    if !matched {
                        let mut cand = inter.clone();
                        cand.push(0); // NULL right row
                        next.push(cand);
                    }
                }
            }
            JoinType::Right => {
                for r in 1..=right_rows {
                    let mut matched = false;
                    for inter in &inters {
                        let mut cand = inter.clone();
                        cand.push(r);
                        if eval_on(k, &cand) {
                            next.push(cand);
                            matched = true;
                        }
                    }
                    if !matched {
                        let mut cand = vec![0usize; width]; // NULL left rows
                        cand.push(r);
                        next.push(cand);
                    }
                }
            }
            JoinType::Full => {
                let mut right_matched = vec![false; right_rows + 1];
                for inter in &inters {
                    let mut matched = false;
                    for r in 1..=right_rows {
                        let mut cand = inter.clone();
                        cand.push(r);
                        if eval_on(k, &cand) {
                            next.push(cand);
                            matched = true;
                            right_matched[r] = true;
                        }
                    }
                    if !matched {
                        let mut cand = inter.clone();
                        cand.push(0);
                        next.push(cand);
                    }
                }
                for r in 1..=right_rows {
                    if !right_matched[r] {
                        let mut cand = vec![0usize; width];
                        cand.push(r);
                        next.push(cand);
                    }
                }
            }
        }
        inters = next;
    }

    Ok(inters)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test wrapper with an unbounded size guard.
    fn build<F: FnMut(usize, &[usize]) -> bool>(
        rows: &[usize],
        jts: &[JoinType],
        f: F,
    ) -> Vec<Vec<usize>> {
        build_intersections(rows, jts, usize::MAX, f).unwrap()
    }

    // ON that matches when the two most-recent indices are equal (a "diagonal").
    fn diag(_k: usize, cand: &[usize]) -> bool {
        let n = cand.len();
        n >= 2 && cand[n - 1] == cand[n - 2]
    }

    fn always(_k: usize, _c: &[usize]) -> bool {
        true
    }

    #[test]
    fn single_table() {
        assert_eq!(build(&[3], &[], always), vec![vec![1], vec![2], vec![3]]);
        assert!(build(&[0], &[], always).is_empty());
    }

    #[test]
    fn cross_join() {
        let got = build(&[2, 2], &[JoinType::Cross], always);
        assert_eq!(got, vec![vec![1, 1], vec![1, 2], vec![2, 1], vec![2, 2]]);
    }

    #[test]
    fn inner_join_filters() {
        let got = build(&[3, 3], &[JoinType::Inner], diag);
        assert_eq!(got, vec![vec![1, 1], vec![2, 2], vec![3, 3]]);
    }

    #[test]
    fn left_join_null_fills_unmatched() {
        // right table has only 2 rows, so left row 3 has no diagonal match.
        let got = build(&[3, 2], &[JoinType::Left], diag);
        assert_eq!(got, vec![vec![1, 1], vec![2, 2], vec![3, 0]]);
    }

    #[test]
    fn right_join_null_fills_unmatched() {
        // left table has only 2 rows, so right row 3 has no diagonal match.
        let got = build(&[2, 3], &[JoinType::Right], diag);
        assert_eq!(got, vec![vec![1, 1], vec![2, 2], vec![0, 3]]);
    }

    #[test]
    fn full_join_null_fills_both_sides() {
        let got = build(&[3, 3], &[JoinType::Full], |_, c| {
            // match left row i with right row i, but only for i in {1,2}
            let n = c.len();
            n >= 2 && c[n - 1] == c[n - 2] && c[n - 1] <= 2
        });
        // 1-1, 2-2 match; left 3 -> NULL right; right 3 -> NULL left.
        assert!(got.contains(&vec![1, 1]));
        assert!(got.contains(&vec![2, 2]));
        assert!(got.contains(&vec![3, 0]));
        assert!(got.contains(&vec![0, 3]));
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn three_table_cross() {
        let got = build(&[2, 2, 2], &[JoinType::Cross, JoinType::Cross], always);
        assert_eq!(got.len(), 8);
        assert_eq!(got[0], vec![1, 1, 1]);
        assert_eq!(got[7], vec![2, 2, 2]);
    }
}
