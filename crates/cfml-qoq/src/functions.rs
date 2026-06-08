//! Built-in scalar SQL functions for QoQ (BoxLang-parity set), implemented in a
//! char-safe, type-preserving way. Aggregates (COUNT/SUM/AVG/MIN/MAX/…) live in
//! the execution engine because they iterate a partition.

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::{CfmlError, CfmlResult};

use crate::compare::sql_equal;

/// Dispatch a built-in scalar function. Returns `None` if `name` is not a
/// built-in (so the caller can fall through to native/custom registries).
pub fn call_scalar(name: &str, args: &[CfmlValue]) -> Option<CfmlResult> {
    let r = match name.to_lowercase().as_str() {
        "upper" | "ucase" => Ok(CfmlValue::string(arg_str(args, 0).to_uppercase())),
        "lower" | "lcase" => Ok(CfmlValue::string(arg_str(args, 0).to_lowercase())),
        "trim" => Ok(CfmlValue::string(arg_str(args, 0).trim().to_string())),
        "ltrim" => Ok(CfmlValue::string(arg_str(args, 0).trim_start().to_string())),
        "rtrim" => Ok(CfmlValue::string(arg_str(args, 0).trim_end().to_string())),
        "len" | "length" => Ok(CfmlValue::Int(arg_str(args, 0).chars().count() as i64)),
        "left" => Ok(str_left(&arg_str(args, 0), arg_i64(args, 1).unwrap_or(0))),
        "right" => Ok(str_right(&arg_str(args, 0), arg_i64(args, 1).unwrap_or(0))),
        "mid" | "substring" | "substr" => Ok(str_mid(
            &arg_str(args, 0),
            arg_i64(args, 1).unwrap_or(1),
            args.get(2).and_then(to_i64),
        )),
        "replace" => Ok(CfmlValue::string(
            arg_str(args, 0).replace(&arg_str(args, 1), &arg_str(args, 2)),
        )),
        "replacenocase" => Ok(CfmlValue::string(replace_nocase(
            &arg_str(args, 0),
            &arg_str(args, 1),
            &arg_str(args, 2),
        ))),
        "concat" => Ok(CfmlValue::string(
            args.iter()
                .filter(|v| !matches!(v, CfmlValue::Null))
                .map(|v| v.as_string())
                .collect::<String>(),
        )),
        "abs" => Ok(match args.first() {
            Some(CfmlValue::Int(i)) => CfmlValue::Int(i.abs()),
            Some(v) => num_result(to_f64(v).unwrap_or(0.0).abs()),
            None => CfmlValue::Null,
        }),
        "ceiling" | "ceil" => Ok(num_result(arg_f64(args, 0).ceil())),
        "floor" => Ok(num_result(arg_f64(args, 0).floor())),
        "sign" | "sgn" => Ok(CfmlValue::Int(sign(arg_f64(args, 0)))),
        "round" => Ok(round(arg_f64(args, 0), args.get(1).and_then(to_i64).unwrap_or(0))),
        "mod" => match (args.first(), args.get(1)) {
            (Some(a), Some(b)) => modulo(a, b),
            _ => Ok(CfmlValue::Null),
        },
        "sqrt" => Ok(CfmlValue::Double(arg_f64(args, 0).sqrt())),
        "exp" => Ok(CfmlValue::Double(arg_f64(args, 0).exp())),
        "sin" => Ok(CfmlValue::Double(arg_f64(args, 0).sin())),
        "cos" => Ok(CfmlValue::Double(arg_f64(args, 0).cos())),
        "tan" => Ok(CfmlValue::Double(arg_f64(args, 0).tan())),
        "power" | "pow" => Ok(CfmlValue::Double(arg_f64(args, 0).powf(arg_f64(args, 1)))),
        "pi" => Ok(CfmlValue::Double(std::f64::consts::PI)),
        "coalesce" => Ok(args
            .iter()
            .find(|v| !matches!(v, CfmlValue::Null))
            .cloned()
            .unwrap_or(CfmlValue::Null)),
        "isnull" => Ok(match args.first() {
            Some(CfmlValue::Null) | None => args.get(1).cloned().unwrap_or(CfmlValue::Null),
            Some(v) => v.clone(),
        }),
        "nullif" => Ok(match (args.first(), args.get(1)) {
            (Some(a), Some(b)) if sql_equal(a, b) == Some(true) => CfmlValue::Null,
            (Some(a), _) => a.clone(),
            _ => CfmlValue::Null,
        }),
        "iif" => Ok(if args.first().map(|v| v.is_true()).unwrap_or(false) {
            args.get(1).cloned().unwrap_or(CfmlValue::Null)
        } else {
            args.get(2).cloned().unwrap_or(CfmlValue::Null)
        }),
        _ => return None,
    };
    Some(r)
}

/// Coerce `value` to the type named by a CAST/CONVERT target (lower-cased).
pub fn cast_value(value: &CfmlValue, ty: &str) -> CfmlResult {
    if matches!(value, CfmlValue::Null) {
        return Ok(CfmlValue::Null);
    }
    match ty {
        "int" | "integer" | "bigint" | "smallint" | "tinyint" => match to_i64(value) {
            Some(i) => Ok(CfmlValue::Int(i)),
            None => Err(CfmlError::runtime(format!(
                "QoQ: cannot CAST '{}' to {}",
                value.as_string(),
                ty
            ))),
        },
        "double" | "float" | "real" | "decimal" | "numeric" | "number" | "money" => {
            match to_f64(value) {
                Some(f) => Ok(CfmlValue::Double(f)),
                None => Err(CfmlError::runtime(format!(
                    "QoQ: cannot CAST '{}' to {}",
                    value.as_string(),
                    ty
                ))),
            }
        }
        "bit" | "boolean" | "bool" => Ok(CfmlValue::Bool(value.is_true())),
        "varchar" | "char" | "nvarchar" | "nchar" | "string" | "text" | "clob" => {
            Ok(CfmlValue::string(value.as_string()))
        }
        // Dates/timestamps and unknown types pass through unchanged (RustCFML
        // has no distinct date variant).
        _ => Ok(value.clone()),
    }
}

// ── helpers ────────────────────────────────────────────────────────────

fn arg_str(args: &[CfmlValue], i: usize) -> String {
    args.get(i).map(|v| v.as_string()).unwrap_or_default()
}

fn arg_f64(args: &[CfmlValue], i: usize) -> f64 {
    args.get(i).and_then(to_f64).unwrap_or(0.0)
}

fn arg_i64(args: &[CfmlValue], i: usize) -> Option<i64> {
    args.get(i).and_then(to_i64)
}

pub fn to_f64(v: &CfmlValue) -> Option<f64> {
    match v {
        CfmlValue::Int(i) => Some(*i as f64),
        CfmlValue::Double(d) => Some(*d),
        CfmlValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        CfmlValue::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

pub fn to_i64(v: &CfmlValue) -> Option<i64> {
    match v {
        CfmlValue::Int(i) => Some(*i),
        CfmlValue::Double(d) => Some(*d as i64),
        CfmlValue::Bool(b) => Some(if *b { 1 } else { 0 }),
        CfmlValue::String(s) => {
            let t = s.trim();
            t.parse::<i64>().ok().or_else(|| t.parse::<f64>().ok().map(|f| f as i64))
        }
        _ => None,
    }
}

/// An f64 result rendered as `Int` when it is a whole number that fits, else
/// `Double` — keeps integer-valued results clean for "preserve native types".
fn num_result(n: f64) -> CfmlValue {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 9.0e15 {
        CfmlValue::Int(n as i64)
    } else {
        CfmlValue::Double(n)
    }
}

fn sign(n: f64) -> i64 {
    if n > 0.0 {
        1
    } else if n < 0.0 {
        -1
    } else {
        0
    }
}

fn round(n: f64, places: i64) -> CfmlValue {
    let factor = 10f64.powi(places as i32);
    let r = (n * factor).round() / factor;
    if places <= 0 {
        num_result(r)
    } else {
        CfmlValue::Double(r)
    }
}

fn modulo(a: &CfmlValue, b: &CfmlValue) -> CfmlResult {
    if let (CfmlValue::Int(x), CfmlValue::Int(y)) = (a, b) {
        if *y == 0 {
            return Err(CfmlError::runtime("QoQ: modulo by zero".to_string()));
        }
        return Ok(CfmlValue::Int(x % y));
    }
    let (x, y) = (to_f64(a).unwrap_or(0.0), to_f64(b).unwrap_or(0.0));
    if y == 0.0 {
        return Err(CfmlError::runtime("QoQ: modulo by zero".to_string()));
    }
    Ok(num_result(x % y))
}

fn str_left(s: &str, n: i64) -> CfmlValue {
    let n = n.max(0) as usize;
    CfmlValue::string(s.chars().take(n).collect::<String>())
}

fn str_right(s: &str, n: i64) -> CfmlValue {
    let chars: Vec<char> = s.chars().collect();
    let n = (n.max(0) as usize).min(chars.len());
    CfmlValue::string(chars[chars.len() - n..].iter().collect::<String>())
}

fn str_mid(s: &str, start: i64, len: Option<i64>) -> CfmlValue {
    let chars: Vec<char> = s.chars().collect();
    // CFML/SQL positions are 1-based.
    let start0 = if start < 1 { 0 } else { (start - 1) as usize };
    if start0 >= chars.len() {
        return CfmlValue::string(String::new());
    }
    let take = match len {
        Some(l) if l >= 0 => (l as usize).min(chars.len() - start0),
        _ => chars.len() - start0,
    };
    CfmlValue::string(chars[start0..start0 + take].iter().collect::<String>())
}

fn replace_nocase(s: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return s.to_string();
    }
    let sc: Vec<char> = s.chars().collect();
    let fc: Vec<char> = from.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < sc.len() {
        if i + fc.len() <= sc.len() && (0..fc.len()).all(|k| char_ci_eq(sc[i + k], fc[k])) {
            out.push_str(to);
            i += fc.len();
        } else {
            out.push(sc[i]);
            i += 1;
        }
    }
    out
}

fn char_ci_eq(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> CfmlValue {
        CfmlValue::string(v.to_string())
    }

    fn call(name: &str, args: &[CfmlValue]) -> CfmlValue {
        call_scalar(name, args).unwrap().unwrap()
    }

    /// CfmlValue has no PartialEq; compare via Debug so the exact variant
    /// (Int vs Double) is checked too.
    fn check(actual: CfmlValue, expected: CfmlValue) {
        assert_eq!(format!("{:?}", actual), format!("{:?}", expected));
    }

    #[test]
    fn string_fns_are_char_safe() {
        // multi-byte: "café" is 4 chars / 5 bytes
        check(call("len", &[s("café")]), CfmlValue::Int(4));
        check(call("left", &[s("café"), CfmlValue::Int(3)]), s("caf"));
        check(call("right", &[s("café"), CfmlValue::Int(2)]), s("fé"));
        check(call("mid", &[s("héllo"), CfmlValue::Int(2), CfmlValue::Int(3)]), s("éll"));
        // No panic when count exceeds length.
        check(call("left", &[s("hi"), CfmlValue::Int(10)]), s("hi"));
    }

    #[test]
    fn numeric_fns_preserve_type() {
        check(call("abs", &[CfmlValue::Int(-5)]), CfmlValue::Int(5));
        check(call("abs", &[CfmlValue::Double(-2.5)]), CfmlValue::Double(2.5));
        check(call("ceiling", &[CfmlValue::Double(2.1)]), CfmlValue::Int(3));
        check(call("floor", &[CfmlValue::Double(2.9)]), CfmlValue::Int(2));
        check(call("round", &[CfmlValue::Double(2.5)]), CfmlValue::Int(3));
        check(call("round", &[CfmlValue::Double(2.567), CfmlValue::Int(2)]), CfmlValue::Double(2.57));
        check(call("sign", &[CfmlValue::Int(-3)]), CfmlValue::Int(-1));
    }

    #[test]
    fn null_and_conditional_fns() {
        check(call("coalesce", &[CfmlValue::Null, CfmlValue::Null, s("x")]), s("x"));
        check(call("isnull", &[CfmlValue::Null, s("d")]), s("d"));
        check(call("isnull", &[s("v"), s("d")]), s("v"));
        check(call("nullif", &[s("a"), s("a")]), CfmlValue::Null);
        check(call("nullif", &[s("a"), s("b")]), s("a"));
        check(call("iif", &[CfmlValue::Bool(true), s("t"), s("f")]), s("t"));
    }

    #[test]
    fn replace_variants() {
        check(call("replace", &[s("a-b-c"), s("-"), s("+")]), s("a+b+c"));
        check(call("replacenocase", &[s("AbAbA"), s("a"), s("x")]), s("xbxbx"));
    }

    #[test]
    fn cast_coercions() {
        check(cast_value(&s("42"), "integer").unwrap(), CfmlValue::Int(42));
        check(cast_value(&CfmlValue::Int(7), "varchar").unwrap(), s("7"));
        check(cast_value(&CfmlValue::Null, "integer").unwrap(), CfmlValue::Null);
    }

    #[test]
    fn unknown_returns_none() {
        assert!(call_scalar("totallyMadeUp", &[]).is_none());
    }
}
