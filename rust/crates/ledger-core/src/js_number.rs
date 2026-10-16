//! Serde helpers that write an `f64` the way the TS side writes a `number`.
//!
//! `JSON.stringify` writes a whole number without a fraction (`1`, not `1.0`), and the fixture
//! exporter writes non-finite numbers as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`
//! (`rust/fixtures/README.md`, "Formatting notes"). Use with
//! `#[serde(with = "crate::js_number")]` on `f64` fields such as `fx_rate`, and
//! `#[serde(with = "crate::js_number::option")]` on `Option<f64>`.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};

/// Largest integer below which every whole `f64` converts to `i64` exactly (2^53).
const EXACT_INT: f64 = 9_007_199_254_740_992.0;

/// Writes `x` as a JS number: a whole value as an integer (so `-0` as `0`), a fraction in
/// shortest round-trip form, a non-finite value as its string.
///
/// # Errors
///
/// Whatever the serializer returns.
pub fn serialize<S: Serializer>(x: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    let x = *x;
    if x.is_nan() {
        serializer.serialize_str("NaN")
    } else if x.is_infinite() {
        serializer.serialize_str(if x > 0.0 { "Infinity" } else { "-Infinity" })
    } else if x.fract() == 0.0 && x.abs() < EXACT_INT {
        #[expect(clippy::cast_possible_truncation, reason = "whole and below 2^53")]
        serializer.serialize_i64(x as i64)
    } else {
        serializer.serialize_f64(x)
    }
}

/// `String(x)` for a JS number (ECMA-262 `Number::toString`): `NaN`, `Infinity`, an integer
/// without a fraction, decimal notation from 1e-6 up to 1e21, and exponent notation (`1e-7`,
/// `1.5e+21`) outside it, always with the shortest digits that round-trip.
#[must_use]
pub fn to_js_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if x == 0.0 {
        return "0".to_owned();
    }
    let abs = x.abs();
    if (1e-6..1e21).contains(&abs) {
        // Rust's `Display` writes the shortest round-trip digits in decimal notation.
        return format!("{x}");
    }
    // `{:e}` writes `1.5e-7` and `1e21`; JS signs a positive exponent (`1e+21`).
    let e = format!("{x:e}");
    match e.split_once('e') {
        Some((mantissa, exp)) if !exp.starts_with('-') => format!("{mantissa}e+{exp}"),
        _ => e,
    }
}

/// Reads a JSON number or one of the non-finite strings.
///
/// # Errors
///
/// When the value is neither.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    deserializer.deserialize_any(JsNumberVisitor)
}

struct JsNumberVisitor;

impl Visitor<'_> for JsNumberVisitor {
    type Value = f64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number, \"NaN\", \"Infinity\" or \"-Infinity\"")
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
        Ok(v)
    }

    #[expect(clippy::cast_precision_loss, reason = "JSON numbers are doubles in TS")]
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
        Ok(v as f64)
    }

    #[expect(clippy::cast_precision_loss, reason = "JSON numbers are doubles in TS")]
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
        Ok(v as f64)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
        match v {
            "NaN" => Ok(f64::NAN),
            "Infinity" => Ok(f64::INFINITY),
            "-Infinity" => Ok(f64::NEG_INFINITY),
            other => Err(E::invalid_value(de::Unexpected::Str(other), &self)),
        }
    }
}

/// The same for `Option<f64>`; pair with `default` and `skip_serializing_if = "Option::is_none"`
/// for an optional TS field.
pub mod option {
    use serde::{Deserialize, Deserializer, Serializer};

    /// Writes `Some(x)` as [`super::serialize`] does and `None` as `null`.
    ///
    /// # Errors
    ///
    /// Whatever the serializer returns.
    pub fn serialize<S: Serializer>(x: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error> {
        match x {
            Some(x) => super::serialize(x, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Reads `null` as `None`, anything else as [`super::deserialize`] does.
    ///
    /// # Errors
    ///
    /// When the value is neither `null` nor a JS number.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<f64>, D::Error> {
        #[derive(Deserialize)]
        struct Wrap(#[serde(with = "super")] f64);
        Ok(Option::<Wrap>::deserialize(deserializer)?.map(|Wrap(x)| x))
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Rate(#[serde(with = "super")] f64);

    fn write(x: f64) -> String {
        serde_json::to_string(&Rate(x)).unwrap()
    }

    #[test]
    fn whole_numbers_print_without_a_fraction() {
        assert_eq!(write(1.0), "1");
        assert_eq!(write(-0.0), "0");
        assert_eq!(write(129.37), "129.37");
        assert_eq!(write(-0.01), "-0.01");
    }

    #[test]
    fn non_finite_numbers_print_as_strings_and_read_back() {
        assert_eq!(write(f64::INFINITY), "\"Infinity\"");
        assert_eq!(write(f64::NEG_INFINITY), "\"-Infinity\"");
        assert_eq!(write(f64::NAN), "\"NaN\"");
        let Rate(x) = serde_json::from_str("\"Infinity\"").unwrap();
        assert_eq!(x, f64::INFINITY);
        let Rate(x) = serde_json::from_str("\"NaN\"").unwrap();
        assert!(x.is_nan());
        assert!(serde_json::from_str::<Rate>("\"inf\"").is_err());
    }
}
