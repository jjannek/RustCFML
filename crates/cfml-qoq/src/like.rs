//! SQL `LIKE` pattern matching: `%` = any sequence, `_` = any single char,
//! with optional `ESCAPE` char. Case-insensitive (CFML convention).
//!
//! Uses the linear-time iterative wildcard algorithm (single backtrack point
//! per `%`), so it cannot blow up on pathological patterns like `%%%%a`.

#[derive(PartialEq, Debug, Clone)]
enum Elem {
    /// `%`
    Any,
    /// `_`
    One,
    /// a literal character (already lower-cased)
    Lit(char),
}

/// A LIKE pattern compiled to its element sequence. Compiling once and calling
/// [`Compiled::matches`] per row avoids recompiling a constant pattern on every
/// row (the QoQ engine pre-compiles literal patterns once per query). `Send +
/// Sync`, so the shared compiled-pattern cache is safe to read across the rayon
/// parallel filter.
#[derive(Debug, Clone)]
pub struct Compiled {
    elems: Vec<Elem>,
    /// Specialised fast-path for the dominant pattern shapes (`%lit%`, `lit%`,
    /// `%lit`, `lit`). When set, [`Compiled::matches`] does an ASCII-CI byte
    /// search and skips the per-row `Vec<char>` allocation + char lowercasing
    /// that the general path needs.
    fast: Option<FastPath>,
}

/// Pre-detected pattern shape for cheap matching. `needle` is the literal run
/// between (or outside) `%` wildcards, already ASCII-lowercased at compile time.
#[derive(Debug, Clone)]
enum FastPath {
    /// `literal` — no wildcards. Case-insensitive equality.
    Equals(String),
    /// `literal%` — case-insensitive prefix match.
    StartsWith(String),
    /// `%literal` — case-insensitive suffix match.
    EndsWith(String),
    /// `%literal%` — case-insensitive substring match.
    Contains(String),
}

/// Compile a LIKE pattern into elements, honouring the escape char (the char
/// after `escape` is always a literal, even `%`/`_`).
pub fn compile(pattern: &str, escape: Option<char>) -> Compiled {
    let esc = escape.map(|c| c.to_ascii_lowercase());
    let mut elems = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        let lc = lower(c);
        if Some(lc) == esc {
            if let Some(next) = chars.next() {
                elems.push(Elem::Lit(lower(next)));
            } else {
                // dangling escape → treat literally
                elems.push(Elem::Lit(lc));
            }
        } else {
            match c {
                '%' => elems.push(Elem::Any),
                '_' => elems.push(Elem::One),
                _ => elems.push(Elem::Lit(lc)),
            }
        }
    }
    let fast = detect_fast_path(&elems);
    Compiled { elems, fast }
}

/// Recognise the four dominant pattern shapes — `lit`, `lit%`, `%lit`, `%lit%` —
/// over a pure-ASCII literal run. Anything else (embedded `%`, `_`, non-ASCII
/// literals) falls back to the general matcher.
fn detect_fast_path(elems: &[Elem]) -> Option<FastPath> {
    fn lit_run_ascii(es: &[Elem]) -> Option<String> {
        let mut out = String::with_capacity(es.len());
        for e in es {
            match e {
                Elem::Lit(c) if c.is_ascii() => out.push(*c),
                _ => return None,
            }
        }
        Some(out)
    }
    let n = elems.len();
    let starts_pct = matches!(elems.first(), Some(Elem::Any));
    let ends_pct = matches!(elems.last(), Some(Elem::Any));
    let body = match (starts_pct, ends_pct) {
        (false, false) => &elems[..],
        (true, false) if n >= 1 => &elems[1..],
        (false, true) if n >= 1 => &elems[..n - 1],
        (true, true) if n >= 2 => &elems[1..n - 1],
        _ => return None,
    };
    let lit = lit_run_ascii(body)?;
    Some(match (starts_pct, ends_pct) {
        (false, false) => FastPath::Equals(lit),
        (false, true) => FastPath::StartsWith(lit),
        (true, false) => FastPath::EndsWith(lit),
        (true, true) => FastPath::Contains(lit),
    })
}

/// Case-insensitive ASCII substring search. Both haystack and needle bytes are
/// compared via `eq_ignore_ascii_case`, no allocation.
#[inline]
fn ascii_ci_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    let first = needle[0];
    let end = haystack.len() - needle.len();
    let mut i = 0;
    while i <= end {
        if haystack[i].eq_ignore_ascii_case(&first) {
            let mut ok = true;
            for j in 1..needle.len() {
                if !haystack[i + j].eq_ignore_ascii_case(&needle[j]) {
                    ok = false;
                    break;
                }
            }
            if ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[inline]
fn ascii_ci_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack[..needle.len()]
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

#[inline]
fn ascii_ci_ends_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack[haystack.len() - needle.len()..]
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

#[inline]
fn ascii_ci_equals(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() == needle.len()
        && haystack
            .iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

#[inline]
fn lower(c: char) -> char {
    // Single-char lowercase; falls back to the original for multi-char mappings.
    c.to_lowercase().next().unwrap_or(c)
}

impl Compiled {
    /// Does `text` match this compiled LIKE pattern? Case-insensitive.
    pub fn matches(&self, text: &str) -> bool {
        // ASCII-CI fast path for the dominant `%lit%` / `lit%` / `%lit` / `lit`
        // shapes. Skips the per-row `Vec<char>` allocation and lower-cases on
        // the fly via byte compare. Falls back to the general matcher if the
        // haystack contains non-ASCII bytes.
        if let Some(fp) = &self.fast {
            if text.is_ascii() {
                let h = text.as_bytes();
                return match fp {
                    FastPath::Equals(n) => ascii_ci_equals(h, n.as_bytes()),
                    FastPath::StartsWith(n) => ascii_ci_starts_with(h, n.as_bytes()),
                    FastPath::EndsWith(n) => ascii_ci_ends_with(h, n.as_bytes()),
                    FastPath::Contains(n) => ascii_ci_contains(h, n.as_bytes()),
                };
            }
        }
        let pat = &self.elems;
        let txt: Vec<char> = text.chars().map(lower).collect();

        let mut i = 0usize; // index into txt
        let mut j = 0usize; // index into pat
        let mut star_j: Option<usize> = None;
        let mut star_i = 0usize;

        while i < txt.len() {
            match pat.get(j) {
                Some(Elem::Lit(c)) if *c == txt[i] => {
                    i += 1;
                    j += 1;
                }
                Some(Elem::One) => {
                    i += 1;
                    j += 1;
                }
                Some(Elem::Any) => {
                    star_j = Some(j);
                    star_i = i;
                    j += 1;
                }
                _ => {
                    // mismatch: backtrack to the last `%`, consuming one more char
                    if let Some(sj) = star_j {
                        j = sj + 1;
                        star_i += 1;
                        i = star_i;
                    } else {
                        return false;
                    }
                }
            }
        }

        // consume trailing `%`
        while matches!(pat.get(j), Some(Elem::Any)) {
            j += 1;
        }
        j == pat.len()
    }
}

/// Compile-and-match in one call. Convenience for non-hot paths and tests; the
/// engine's per-row path uses a pre-compiled [`Compiled`] instead.
pub fn like_match(text: &str, pattern: &str, escape: Option<char>) -> bool {
    compile(pattern, escape).matches(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert!(like_match("hello", "hello", None));
        assert!(like_match("Hello", "hello", None)); // case-insensitive
        assert!(like_match("hello", "h%o", None));
        assert!(like_match("hello", "h_llo", None));
        assert!(like_match("hello", "%", None));
        assert!(like_match("", "%", None));
        assert!(!like_match("hello", "h_lo", None));
        assert!(!like_match("hello", "world", None));
    }

    #[test]
    fn anchors_and_sequences() {
        assert!(like_match("abcabc", "a%c", None));
        assert!(like_match("abc", "abc%", None));
        assert!(like_match("abc", "%abc", None));
        assert!(!like_match("abcd", "abc", None));
        assert!(like_match("a", "_", None));
        assert!(!like_match("ab", "_", None));
    }

    #[test]
    fn no_exponential_blowup() {
        // Would explode under naive recursion.
        let text = "a".repeat(50);
        assert!(!like_match(&text, &format!("{}b", "%".repeat(20)), None));
        assert!(like_match(&text, &"%".repeat(20), None));
    }

    #[test]
    fn escape_char() {
        assert!(like_match("50%", "50\\%", Some('\\')));
        assert!(!like_match("500", "50\\%", Some('\\')));
        assert!(like_match("a_b", "a\\_b", Some('\\')));
        assert!(like_match("a_b", "a_b", None)); // _ wildcard still matches
    }
}
