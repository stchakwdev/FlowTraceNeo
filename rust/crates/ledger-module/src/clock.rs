//! Reducer time as the ledger writes it.
//!
//! The TS module stamps audit events with `ctx.timestamp.toISOString()` (`index.ts:363`,
//! `index.ts:524`). In the TS SDK that string has microseconds
//! (`spacetimedb` 2.11 `src/lib/timestamp.ts:125-148`, `.123456Z`), but [`Timestamp`] holds the
//! engine's `toISOString` shape with milliseconds (`ledger_core::env`), so this module writes
//! milliseconds, truncated as the TS SDK truncates micros to millis (`micros / 1000n`). See
//! `docs/adr/0003-module-tables.md`, "Timestamps".

use ledger_core::env::TimestampError;
use ledger_core::types::Timestamp;

/// `YYYY-MM-DDTHH:MM:SS.mmmZ` for a count of microseconds since the Unix epoch.
///
/// # Errors
///
/// For an instant outside years 0000 to 9999, which has no `toISOString` four-digit year.
pub fn iso_millis(micros_since_epoch: i64) -> Result<Timestamp, TimestampError> {
    const MILLIS_PER_DAY: i64 = 86_400_000;
    // `micros / 1000n` in the TS SDK truncates toward zero, as Rust's `/` does.
    let millis = micros_since_epoch / 1000;
    let (year, month, day) = civil_from_days(millis.div_euclid(MILLIS_PER_DAY));
    let in_day = millis.rem_euclid(MILLIS_PER_DAY);
    let text = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        in_day / 3_600_000,
        in_day / 60_000 % 60,
        in_day / 1000 % 60,
        in_day % 1000
    );
    Timestamp::parse(&text)
}

/// Year, month and day of a day count since 1970-01-01 in the proleptic Gregorian calendar
/// (H. Hinnant, "chrono-Compatible Low-Level Date Algorithms", `civil_from_days`:
/// <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// [`iso_millis`] of a reducer's timestamp.
///
/// # Errors
///
/// As [`iso_millis`].
pub fn reducer_time(at: spacetimedb::Timestamp) -> Result<Timestamp, TimestampError> {
    iso_millis(at.to_micros_since_unix_epoch())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    fn iso(micros: i64) -> String {
        iso_millis(micros).unwrap().to_string()
    }

    #[test]
    fn the_epoch_is_1970_01_01() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn the_demo_clock_reads_as_the_demo_env_writes_it() {
        // rust/fixtures/README.md: demo env now 2026-02-04T09:00:00.000Z.
        assert_eq!(iso(1_770_195_600_000_000), "2026-02-04T09:00:00.000Z");
    }

    #[test]
    fn a_leap_day_ends_at_its_last_millisecond() {
        assert_eq!(iso(1_709_251_199_999_000), "2024-02-29T23:59:59.999Z");
    }

    #[test]
    fn the_day_after_a_400_year_leap_day_is_march_first() {
        assert_eq!(iso(951_868_800_000_000), "2000-03-01T00:00:00.000Z");
    }

    #[test]
    fn microseconds_are_truncated_to_milliseconds() {
        assert_eq!(iso(1_000_999), "1970-01-01T00:00:01.000Z");
    }

    #[test]
    fn a_microsecond_before_the_epoch_truncates_toward_zero_as_bigint_division_does() {
        assert_eq!(iso(-1), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn a_millisecond_before_the_epoch_is_the_last_of_1969() {
        assert_eq!(iso(-1_000), "1969-12-31T23:59:59.999Z");
    }

    #[test]
    fn year_10000_has_no_four_digit_year() {
        assert!(iso_millis(253_402_300_800_000_000).is_err());
    }
}
