//! The JavaScript coercions the HTML relies on, for JSON values.
use serde_json::Value;

/// JavaScript truthiness.
pub(super) fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `+value || 0`.
pub(super) fn num(value: &Value) -> f64 {
    let n = match value {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) | Value::Object(_) => f64::NAN,
    };
    if n.is_nan() { 0.0 } else { n }
}

/// `+value || 0` for an optional field.
pub(super) fn field(record: &Value, key: &str) -> f64 {
    record.get(key).map_or(0.0, num)
}

/// `String(value)`.
pub(super) fn string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_text(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    string(v)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// A text field as the HTML reads it after `migrate()`: `""` when absent or null.
pub(super) fn text(record: &Value, key: &str) -> String {
    match record.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(v) => string(v),
    }
}

fn number_text(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e21 {
        format!("{f:.0}")
    } else {
        format!("{f}")
    }
}
