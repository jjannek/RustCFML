//! `${env.VAR:default}` placeholder expansion.
//!
//! Matches the BoxLang / Ortus CFConfig convention. Syntax:
//!
//! ```text
//! ${env.VAR_NAME}             — env var, empty string if unset
//! ${env.VAR_NAME:fallback}    — env var with literal fallback
//! ```
//!
//! Only `env.` is recognised; other prefixes are left untouched so future
//! variable namespaces can be added without breaking older configs.
//! Substitution is one-pass: expanded values are NOT re-scanned, so an env var
//! whose value itself contains `${...}` does not recurse.

use std::env;

pub fn expand_env_vars(input: &str) -> String {
    if !input.contains("${") {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'$' && bytes[i + 1] == b'{' {
            if let Some(end) = find_close(bytes, i + 2) {
                let inner = &input[i + 2..end];
                if let Some(rest) = inner.strip_prefix("env.") {
                    let (name, fallback) = match rest.find(':') {
                        Some(idx) => (&rest[..idx], Some(&rest[idx + 1..])),
                        None => (rest, None),
                    };
                    let resolved = env::var(name).ok();
                    let value = resolved
                        .as_deref()
                        .or(fallback)
                        .unwrap_or("");
                    out.push_str(value);
                    i = end + 1;
                    continue;
                }
                // Unknown namespace — leave verbatim.
                out.push_str(&input[i..=end]);
                i = end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn find_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_var<F: FnOnce()>(name: &str, value: &str, f: F) {
        let prev = env::var(name).ok();
        env::set_var(name, value);
        f();
        match prev {
            Some(v) => env::set_var(name, v),
            None => env::remove_var(name),
        }
    }

    #[test]
    fn no_placeholders_is_unchanged() {
        assert_eq!(expand_env_vars("hello world"), "hello world");
        assert_eq!(expand_env_vars(""), "");
    }

    #[test]
    fn env_var_resolves() {
        with_var("RUSTCFML_TEST_X", "abc", || {
            assert_eq!(expand_env_vars("${env.RUSTCFML_TEST_X}"), "abc");
            assert_eq!(
                expand_env_vars("prefix-${env.RUSTCFML_TEST_X}-suffix"),
                "prefix-abc-suffix"
            );
        });
    }

    #[test]
    fn fallback_used_when_unset() {
        env::remove_var("RUSTCFML_TEST_MISSING");
        assert_eq!(
            expand_env_vars("${env.RUSTCFML_TEST_MISSING:localhost}"),
            "localhost"
        );
    }

    #[test]
    fn empty_when_no_fallback_and_unset() {
        env::remove_var("RUSTCFML_TEST_EMPTY");
        assert_eq!(expand_env_vars("${env.RUSTCFML_TEST_EMPTY}"), "");
    }

    #[test]
    fn fallback_may_contain_colons() {
        env::remove_var("RUSTCFML_TEST_URL");
        assert_eq!(
            expand_env_vars("${env.RUSTCFML_TEST_URL:http://localhost:8080/path}"),
            "http://localhost:8080/path"
        );
    }

    #[test]
    fn unknown_namespace_is_preserved() {
        assert_eq!(expand_env_vars("${other.X}"), "${other.X}");
    }

    #[test]
    fn unclosed_placeholder_is_preserved() {
        assert_eq!(expand_env_vars("${env.X"), "${env.X");
    }

    #[test]
    fn multiple_placeholders() {
        with_var("RUSTCFML_TEST_A", "1", || {
            with_var("RUSTCFML_TEST_B", "2", || {
                assert_eq!(
                    expand_env_vars("${env.RUSTCFML_TEST_A}-${env.RUSTCFML_TEST_B}"),
                    "1-2"
                );
            });
        });
    }

    #[test]
    fn env_value_with_dollar_brace_is_not_recursed() {
        with_var("RUSTCFML_TEST_REC", "${env.OTHER}", || {
            assert_eq!(
                expand_env_vars("${env.RUSTCFML_TEST_REC}"),
                "${env.OTHER}"
            );
        });
    }
}
