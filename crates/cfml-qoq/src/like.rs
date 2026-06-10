//! SQL `LIKE` pattern matching: `%` = any sequence, `_` = any single char,
//! with optional `ESCAPE` char. Case-insensitive (CFML convention).
//!
//! Dispatch:
//! - Trivial ASCII shapes (`lit`, `lit%`, `%lit`) take a non-allocating
//!   anchored ASCII-CI byte compare.
//! - Everything else (`%lit%` contains, embedded `%`/`_`, non-ASCII, escape
//!   chars) routes through the `regex` crate, which under the hood uses
//!   Teddy / Aho-Corasick / Boyer-Moore for literal-bearing patterns and a
//!   SIMD-backed DFA otherwise. Compiled once per pattern in [`compile`].

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
    inner: Inner,
}

#[derive(Debug, Clone)]
enum Inner {
    /// `literal` — anchored ASCII-CI equality.
    Equals(String),
    /// `literal%` — anchored ASCII-CI prefix.
    StartsWith(String),
    /// `%literal` — anchored ASCII-CI suffix.
    EndsWith(String),
    /// `%literal%` — ASCII-CI substring. Hand-rolled byte loop is measurably
    /// faster than `regex::is_match` on short needles + 1M-row scans (the
    /// regex crate's per-call dispatch cost dominates when the literal
    /// prefilter is the whole match).
    Contains(String),
    /// Anything else: a regex compiled with `(?i)`. Anchors are emitted
    /// only at edges that don't begin/end with `%`, so the crate's literal
    /// extraction can pick the strongest prefilter.
    Regex(regex::Regex),
}

/// Compile a LIKE pattern, honouring the escape char (the char after
/// `escape` is always a literal, even `%`/`_`).
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
    Compiled { inner: build_inner(&elems) }
}

/// Pick the cheapest matcher for the compiled element sequence. Trivial
/// anchored ASCII shapes (`lit`, `lit%`, `%lit`) get inlined byte compares;
/// everything else builds a `regex::Regex` once per pattern.
fn build_inner(elems: &[Elem]) -> Inner {
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
    // `lit`, `lit%`, `%lit` over a pure-ASCII literal run → anchored ASCII-CI
    // byte compare. `%lit%` (Contains) goes to regex — its Teddy/memmem
    // implementation beats a hand-rolled O(n·m) ASCII loop on long haystacks
    // (Q6 in the bench was the headline cost).
    if !starts_pct && !ends_pct {
        if let Some(lit) = lit_run_ascii(elems) {
            return Inner::Equals(lit);
        }
    } else if !starts_pct && ends_pct && n >= 1 {
        if let Some(lit) = lit_run_ascii(&elems[..n - 1]) {
            return Inner::StartsWith(lit);
        }
    } else if starts_pct && !ends_pct && n >= 1 {
        if let Some(lit) = lit_run_ascii(&elems[1..]) {
            return Inner::EndsWith(lit);
        }
    } else if starts_pct && ends_pct && n >= 2 {
        if let Some(lit) = lit_run_ascii(&elems[1..n - 1]) {
            return Inner::Contains(lit);
        }
    }
    Inner::Regex(build_regex(elems))
}

/// Translate the compiled LIKE elements into a `(?i)`-flagged regex usable
/// with `is_match` (which already scans for a match anywhere in the haystack).
///
/// Anchoring: leading/trailing `%` are *omitted*, not turned into `.*`.
/// `%Harry%` ⇒ `(?i)harry` — the `regex` crate then sees a pure literal and
/// dispatches to Teddy/memmem (SIMD substring) instead of a general DFA scan.
/// `Harry%` ⇒ `(?i)^harry`. `%Harry` ⇒ `(?i)harry$`. `H_arry` ⇒
/// `(?i)^h.arry$`. This matches the LIKE semantics exactly: a LIKE pattern
/// must consume the entire string, so anchors are added at any edge that
/// isn't a `%`.
fn build_regex(elems: &[Elem]) -> regex::Regex {
    let n = elems.len();
    let starts_pct = matches!(elems.first(), Some(Elem::Any));
    let ends_pct = matches!(elems.last(), Some(Elem::Any));
    let body = match (starts_pct, ends_pct) {
        (false, false) => &elems[..],
        (true, false) if n >= 1 => &elems[1..],
        (false, true) if n >= 1 => &elems[..n - 1],
        (true, true) if n >= 2 => &elems[1..n - 1],
        // patterns that are just `%` (or `%%…`) match everything
        _ => return regex::Regex::new("").unwrap(),
    };
    let mut pat = String::with_capacity(body.len() * 2 + 8);
    pat.push_str("(?is)");
    if !starts_pct {
        pat.push('^');
    }
    for e in body {
        match e {
            Elem::Any => pat.push_str(".*"),
            Elem::One => pat.push('.'),
            Elem::Lit(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                pat.push_str(&regex::escape(s));
            }
        }
    }
    if !ends_pct {
        pat.push('$');
    }
    // `unwrap` is safe: every construction above is a valid regex by
    // construction (we only emit `^`, `$`, `.`, `.*`, and escaped literals).
    regex::Regex::new(&pat).unwrap()
}

/// Case-insensitive ASCII substring search. Both haystack and needle bytes are
/// compared via `eq_ignore_ascii_case`, no allocation. Measurably faster than
/// `regex::Regex::is_match` for short needles on the 1M-row bench (Q6).
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
        match &self.inner {
            // Anchored ASCII-CI fast paths: non-allocating, branch-light. If
            // the haystack contains non-ASCII bytes, fall back to a regex —
            // `regex` does the right thing on Unicode case folding under
            // `(?i)`, which the byte path cannot.
            Inner::Equals(n) if text.is_ascii() => {
                ascii_ci_equals(text.as_bytes(), n.as_bytes())
            }
            Inner::StartsWith(n) if text.is_ascii() => {
                ascii_ci_starts_with(text.as_bytes(), n.as_bytes())
            }
            Inner::EndsWith(n) if text.is_ascii() => {
                ascii_ci_ends_with(text.as_bytes(), n.as_bytes())
            }
            Inner::Contains(n) if text.is_ascii() => {
                ascii_ci_contains(text.as_bytes(), n.as_bytes())
            }
            Inner::Equals(n) => {
                // Non-ASCII haystack: build a one-off anchored regex on the
                // already-lowercased literal. Rare path — exact-match LIKEs
                // are dominantly ASCII identifiers in practice.
                regex_for_literal(n, /*anchor_start*/ true, /*anchor_end*/ true).is_match(text)
            }
            Inner::StartsWith(n) => {
                regex_for_literal(n, true, false).is_match(text)
            }
            Inner::EndsWith(n) => {
                regex_for_literal(n, false, true).is_match(text)
            }
            Inner::Contains(n) => {
                regex_for_literal(n, false, false).is_match(text)
            }
            Inner::Regex(r) => r.is_match(text),
        }
    }
}

/// Build a `(?i)`-flagged regex for a literal (already-lowercased) needle
/// with optional anchors. Used only on the cold non-ASCII fallback paths.
fn regex_for_literal(n: &str, anchor_start: bool, anchor_end: bool) -> regex::Regex {
    let mut pat = String::with_capacity(n.len() + 8);
    pat.push_str("(?is)");
    if anchor_start {
        pat.push('^');
    }
    pat.push_str(&regex::escape(n));
    if anchor_end {
        pat.push('$');
    }
    regex::Regex::new(&pat).unwrap()
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
