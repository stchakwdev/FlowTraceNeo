//! Fixture loading shared by the parity tests. The fixtures are written by the TS engine through
//! `rust/tools`; see `rust/fixtures/README.md` for their shapes.
#![allow(dead_code, reason = "each test crate uses a subset of these helpers")]

use std::path::PathBuf;

use serde_json::Value;

/// Path of `rust/fixtures/<name>.json`.
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(format!("{name}.json"))
}

/// The fixture file's bytes as text.
pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap()
}

/// The fixture file parsed as JSON.
pub fn fixture(name: &str) -> Value {
    serde_json::from_str(&fixture_text(name)).unwrap()
}

/// A JS number as the exporter writes it: a JSON number, or one of the strings `"NaN"`,
/// `"Infinity"` and `"-Infinity"` for non-finite values (README, "Formatting notes").
pub fn js_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap(),
        Value::String(s) if s == "NaN" => f64::NAN,
        Value::String(s) if s == "Infinity" => f64::INFINITY,
        Value::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        other => panic!("not a JS number: {other}"),
    }
}

/// The case list under `key` of a pure-function fixture.
pub fn cases<'a>(fixture: &'a Value, key: &str) -> &'a [Value] {
    let list = fixture[key].as_array().unwrap();
    assert!(!list.is_empty(), "{key} has no cases");
    list
}

/// Every object in `value` (depth first) for which `pred` holds.
pub fn find_objects<'a>(
    value: &'a Value,
    pred: &dyn Fn(&serde_json::Map<String, Value>) -> bool,
    out: &mut Vec<&'a Value>,
) {
    match value {
        Value::Object(map) => {
            if pred(map) {
                out.push(value);
            }
            for v in map.values() {
                find_objects(v, pred, out);
            }
        }
        Value::Array(items) => {
            for v in items {
                find_objects(v, pred, out);
            }
        }
        _ => {}
    }
}

/// Every string in `value` for which `pred` holds.
pub fn find_strings<'a>(value: &'a Value, pred: &dyn Fn(&str) -> bool, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => map.values().for_each(|v| find_strings(v, pred, out)),
        Value::Array(items) => items.iter().for_each(|v| find_strings(v, pred, out)),
        Value::String(s) if pred(s) => out.push(s),
        _ => {}
    }
}
