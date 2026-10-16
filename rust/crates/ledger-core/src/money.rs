//! Money in integer minor units, mirroring `flowtrace/src/lib/ledger/money.ts`.
//!
//! These helpers are the only place a decimal string or an exchange rate turns into minor units
//! (`money.ts:3-4`). Where the TS code computes in IEEE-754 doubles, the Rust code computes in
//! `f64` the same way, so the results match bit for bit; the parity cases are in
//! `rust/fixtures/money.json`.

/// An amount as an integer count of minor units (cents), `types.ts:5-8`.
///
/// TS holds it in a `number` and checks `Number.isSafeInteger`; Rust holds an `i64` and keeps
/// the same bound ([`MAX_SAFE_MINOR`]), so every value round-trips through the TS side unchanged.
pub type Minor = i64;

/// `Number.MAX_SAFE_INTEGER`, the largest amount TS accepts (`money.ts:7`).
pub const MAX_SAFE_MINOR: Minor = 9_007_199_254_740_991;

/// The noun `assertMinor` uses when the caller names none (`money.ts:6`).
pub const AMOUNT: &str = "Amount";

/// The `RangeError`s of `money.ts`. They mark a programming or input error, not a ledger rule;
/// ledger rules fail with [`crate::errors::LedgerError`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// Not a safe integer (`money.ts:7-9`).
    #[error("{what} must be a whole number of minor units.")]
    NotMinorUnits {
        /// What the value was, for example `Amount` or `Line 1 amount`.
        what: String,
    },
    /// The rate is not a positive finite number (`money.ts:37-39`).
    #[error("An exchange rate must be a positive number.")]
    InvalidRate,
}

/// `Number.isSafeInteger(n)`: finite, whole and within [`MAX_SAFE_MINOR`].
fn is_safe_integer(n: f64) -> bool {
    #[expect(clippy::cast_precision_loss, reason = "2^53 - 1 is exact in f64")]
    let max = MAX_SAFE_MINOR as f64;
    n.is_finite() && n.fract() == 0.0 && n.abs() <= max
}

/// `assertMinor(n, what)` (`money.ts:6-10`): accepts a safe integer and returns it as [`Minor`].
///
/// Takes the `f64` the TS function takes, so unvalidated input (`10.5`, `NaN`) is checked the
/// same way. Pass [`AMOUNT`] where TS uses the default noun.
///
/// # Errors
///
/// [`MoneyError::NotMinorUnits`] when `n` is not a safe integer.
pub fn assert_minor(n: f64, what: &str) -> Result<Minor, MoneyError> {
    if is_safe_integer(n) {
        #[expect(clippy::cast_possible_truncation, reason = "n is a safe integer")]
        Ok(n as Minor)
    } else {
        Err(MoneyError::NotMinorUnits {
            what: what.to_owned(),
        })
    }
}

/// `assertMinor` for a value that is already an integer: checks the safe-integer bound only.
///
/// # Errors
///
/// [`MoneyError::NotMinorUnits`] when `|n|` exceeds [`MAX_SAFE_MINOR`].
pub fn assert_safe(n: Minor, what: &str) -> Result<Minor, MoneyError> {
    if n.unsigned_abs() <= MAX_SAFE_MINOR.unsigned_abs() {
        Ok(n)
    } else {
        Err(MoneyError::NotMinorUnits {
            what: what.to_owned(),
        })
    }
}

/// JS `\s` (`WhiteSpace` and `LineTerminator`, ECMA-262 22.2.2.9). Unlike
/// [`char::is_whitespace`] it includes U+FEFF and leaves out U+0085.
pub(crate) fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// `parseMoney(raw)` (`money.ts:12-20`): `'1,250.5'` to `125050`, `'(300)'` to `-30000`; `None`
/// for anything that is not money, including more than two decimals and unsafe amounts.
///
/// Commas and JS whitespace are removed first, a whole-string parenthesis means negative, then
/// the value must match `^(-)?(\d+)(?:\.(\d{1,2}))?$` with ASCII digits. The integer part is
/// read as an `f64` and multiplied by 100 as TS does, so the safe-integer cut-off is the same.
#[must_use]
pub fn parse_money(raw: &str) -> Option<Minor> {
    let stripped: String = raw
        .chars()
        .filter(|&c| c != ',' && !is_js_whitespace(c))
        .collect();
    let s = match stripped
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(inner) => format!("-{inner}"),
        None => stripped,
    };
    let (negative, unsigned) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.as_str()),
    };
    let (whole, frac) = match unsigned.split_once('.') {
        Some((whole, frac)) => (whole, frac),
        None => (unsigned, ""),
    };
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    let frac_ok = if unsigned.contains('.') {
        (1..=2).contains(&frac.len())
    } else {
        true
    };
    if whole.is_empty() || !all_digits(whole) || !all_digits(frac) || !frac_ok {
        return None;
    }
    // Number(m[2]) * 100 + Number(m[3].padEnd(2, '0')), all in doubles (money.ts:17).
    let whole: f64 = whole.parse().ok()?;
    let cents: f64 = format!("{frac:0<2}").parse().ok()?;
    let minor = whole * 100.0 + cents;
    let minor = assert_minor(minor, AMOUNT).ok()?;
    Some(if negative { -minor } else { minor })
}

/// `toMajor(minor)` (`money.ts:22-23`): major units as an `f64`, for display only.
///
/// Returns `f64` because TS returns the double `minor / 100` and the fixtures compare that
/// double exactly; never feed the result back into stored amounts.
#[must_use]
pub fn to_major(minor: Minor) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "TS divides the same double")]
    let minor = minor as f64;
    minor / 100.0
}

/// JS `Math.round(x)`: the nearest integer, ties toward positive infinity, so
/// `js_round(-2.5) == -2.0` where `f64::round` gives `-3.0`. Non-finite values pass through and a
/// negative result of zero stays `-0.0`, as in JS.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    // x - floor is exact for every finite double.
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

/// `fromMajor(major)` (`money.ts:25-30`): `Math.round(major * 100)` as minor units, so
/// `from_major(1.005) == 100` and `from_major(12.345) == 1235` from the double products.
///
/// # Errors
///
/// [`MoneyError::NotMinorUnits`] when the result is not a safe integer.
pub fn from_major(major: f64) -> Result<Minor, MoneyError> {
    assert_minor(js_round(major * 100.0), AMOUNT)
}

/// `convert(amount, rate)` (`money.ts:32-51`): `amount` in base currency at `rate`.
///
/// The product is an `f64`; it rounds to the nearer integer, and within `1e-9` of a half to the
/// even neighbour, so repeated conversions do not drift. A rate of exactly 1 returns `amount`.
///
/// # Errors
///
/// [`MoneyError::InvalidRate`] unless `rate` is positive and finite;
/// [`MoneyError::NotMinorUnits`] when the result is not a safe integer.
pub fn convert(amount: Minor, rate: f64) -> Result<Minor, MoneyError> {
    // `!(rate > 0) || !isFinite(rate)`: NaN, zero, negative or infinite.
    if rate.is_nan() || rate <= 0.0 || rate.is_infinite() {
        return Err(MoneyError::InvalidRate);
    }
    #[expect(clippy::float_cmp, reason = "money.ts:40 compares rate === 1 exactly")]
    if rate == 1.0 {
        return Ok(amount);
    }
    #[expect(clippy::cast_precision_loss, reason = "TS multiplies the same double")]
    let exact = amount as f64 * rate;
    let floor = exact.floor();
    let diff = exact - floor;
    let eps = 1e-9;
    // Above the half by more than eps: up; below by more than eps: down; else to the even one.
    let round_up = diff > 0.5 + eps || (diff >= 0.5 - eps && floor % 2.0 != 0.0);
    let out = if round_up { floor + 1.0 } else { floor };
    assert_minor(out, AMOUNT)
}

/// `sum(xs)` (`money.ts:53-57`). Totals of safe amounts stay far inside `i64`.
pub fn sum(xs: impl IntoIterator<Item = Minor>) -> Minor {
    xs.into_iter().sum()
}

/// `(minor / 100).toFixed(2)`, the money form in ledger messages (`engine.ts:408`,
/// `engine.ts:438`, `engine.ts:1080`): two decimals, no thousands separator, `-` for negatives.
///
/// Mirrors JS exactly: the `f64` quotient is rounded to two decimals from its exact binary
/// value, ties away from zero (ECMA-262 `Number.prototype.toFixed`: "pick the larger n" on the
/// magnitude). Above about 2^46 minor units the quotient is not the nearest cent, so this can
/// print a neighbouring cent, as TS does.
#[must_use]
pub fn format_major(minor: Minor) -> String {
    let x = to_major(minor);
    let sign = if x < 0.0 { "-" } else { "" };
    // Rust prints the exact decimal expansion of a double at any precision; 80 places cover
    // every quotient of a safe amount (the smallest, 0.01, needs 59).
    let exact = format!("{:.80}", x.abs());
    let (whole, frac) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let whole: u64 = whole.parse().unwrap_or(0);
    let digit = |i: usize| u64::from(frac.as_bytes().get(i).map_or(0, |b| b - b'0'));
    let round_up = u64::from(digit(2) >= 5);
    let cents = whole * 100 + digit(0) * 10 + digit(1) + round_up;
    format!("{sign}{}.{:02}", cents / 100, cents % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_safe_bounds_at_max_safe_integer() {
        assert_eq!(assert_safe(MAX_SAFE_MINOR, AMOUNT), Ok(MAX_SAFE_MINOR));
        assert_eq!(assert_safe(-MAX_SAFE_MINOR, AMOUNT), Ok(-MAX_SAFE_MINOR));
        assert!(assert_safe(MAX_SAFE_MINOR + 1, AMOUNT).is_err());
        assert!(assert_safe(Minor::MIN, AMOUNT).is_err());
    }

    #[test]
    fn parse_money_ignores_js_whitespace_only() {
        assert_eq!(parse_money("\u{FEFF}1\u{00A0}000"), Some(100_000));
        assert_eq!(parse_money("1\u{0085}000"), None);
        assert_eq!(parse_money("１２"), None);
        assert_eq!(parse_money("-(5)"), None);
        assert_eq!(parse_money("(-5)"), None);
    }
}
