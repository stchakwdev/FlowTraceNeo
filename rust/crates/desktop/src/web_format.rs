//! The web's display formatting (`flowtrace/src/lib/format.ts`) over minor units, for the
//! screens that mirror pages built on it (Dashboard and Campaigns, DT1). The ledger screens use
//! `formatMinor` instead (`crate::accounts::format_minor`), which writes cents, parentheses and
//! an en dash for zero; these write whole units, a hyphen-minus and a plain `0`, as
//! `toLocaleString('en-KE')` does.

use ledger_core::Minor;
use ledger_core::types::IsoDate;

use crate::accounts::{format_date as format_iso_date, format_short_date as format_iso_short};
use crate::statements::to_fixed;

/// `formatAmount(amount)` (`format.ts:42-47`) of `minor / 100`: whole units with commas every
/// three digits (`en-KE`), rounded half away from zero as `Intl.NumberFormat` rounds, `-` before
/// a negative. A negative amount that rounds to zero keeps its sign (`-0`), as `Intl` writes it.
#[must_use]
pub fn format_amount(minor: Minor) -> String {
    let whole = (minor.unsigned_abs() + 50) / 100;
    let digits = whole.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if minor < 0 {
        out.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `formatMoney(amount, currency)` (`format.ts:50-52`): `KES 2,500,000`.
#[must_use]
pub fn format_money(minor: Minor, currency: &str) -> String {
    format!("{currency} {}", format_amount(minor))
}

/// `formatCompact(amount)` (`format.ts:55-61`) of `minor / 100`: `15.7M` (one decimal by
/// `toFixed`, a trailing `.0` dropped), `850K` (`Math.round` of thousands, ties toward
/// +Infinity), else [`format_amount`].
#[must_use]
pub fn format_compact(minor: Minor) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a JS number, as the web holds it"
    )]
    let major = minor as f64 / 100.0;
    if major.abs() >= 1_000_000.0 {
        let m = to_fixed(major / 1_000_000.0, 1);
        return format!("{}M", m.strip_suffix(".0").unwrap_or(&m));
    }
    if major.abs() >= 10_000.0 {
        return format!("{}K", js_round(major / 1_000.0));
    }
    format_amount(minor)
}

/// `Math.round`: halves go toward +Infinity.
fn js_round(x: f64) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "thousands of a safe amount fit an i64"
    )]
    let n = (x + 0.5).floor() as i64;
    n
}

/// The calendar date of an ISO date or UTC timestamp (`2026-01-18`, `2026-01-18T00:00:00Z`);
/// `format.ts` formats in UTC, so the first ten characters are the day shown.
#[must_use]
pub fn day_of(value: &str) -> Option<IsoDate> {
    IsoDate::parse(value.get(..10)?).ok()
}

/// `formatDate(value)` (`format.ts:12-19`) of an ISO date or UTC timestamp: `18 Jan 2026`;
/// the value as given when it is not one.
#[must_use]
pub fn format_date(value: &str) -> String {
    day_of(value).map_or_else(|| value.to_owned(), |d| format_iso_date(&d))
}

/// `formatShortDate(value)` (`format.ts:22-28`): `18 Jan`; the value as given when it is not a
/// date.
#[must_use]
pub fn format_short_date(value: &str) -> String {
    day_of(value).map_or_else(|| value.to_owned(), |d| format_iso_short(&d))
}

/// `formatDateTime(value)` (`format.ts:31-39`) of a UTC timestamp: `18 Jan 2026, 14:05`. The
/// web writes the viewer's local time; the desktop writes UTC, as `claims::format_date_time`
/// does, so a record reads the same on every machine. The value as given when it is not one.
#[must_use]
pub fn format_date_time(value: &str) -> String {
    match (day_of(value), value.get(11..16)) {
        (Some(day), Some(time)) if time.as_bytes().get(2) == Some(&b':') => {
            format!("{}, {time}", format_iso_date(&day))
        }
        (Some(day), _) => format!("{}, 00:00", format_iso_date(&day)),
        _ => value.to_owned(),
    }
}

/// `formatDateTimeEAT(value)` (`format.ts:68-79`) of a UTC timestamp: the time in East Africa
/// Time (UTC+3, no daylight saving) with the zone named, `18 Jan 2026, 07:00 EAT`, the day
/// moving on past midnight. The value as given when it is not a timestamp.
#[must_use]
pub fn format_date_time_eat(value: &str) -> String {
    let Some(day) = day_of(value) else {
        return value.to_owned();
    };
    let hour: u32 = value.get(11..13).and_then(|h| h.parse().ok()).unwrap_or(0);
    let minute = value.get(14..16).unwrap_or("00");
    let (day, hour) = if hour + 3 >= 24 {
        (crate::bills::add_days(&day, 1), hour + 3 - 24)
    } else {
        (day, hour + 3)
    };
    format!("{}, {hour:02}:{minute} EAT", format_iso_date(&day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_times_are_utc_and_eat_moves_three_hours_on() {
        assert_eq!(
            format_date_time("2026-02-04T09:00:00.000Z"),
            "4 Feb 2026, 09:00"
        );
        assert_eq!(format_date_time("2026-02-04"), "4 Feb 2026, 00:00");
        assert_eq!(format_date_time("soon"), "soon");
        assert_eq!(
            format_date_time_eat("2026-02-04T09:05:00.000Z"),
            "4 Feb 2026, 12:05 EAT"
        );
        assert_eq!(
            format_date_time_eat("2025-12-31T22:30:00.000Z"),
            "1 Jan 2026, 01:30 EAT"
        );
    }

    #[test]
    fn amounts_are_whole_units_grouped_by_three() {
        assert_eq!(format_amount(0), "0");
        assert_eq!(format_amount(92_000), "920");
        assert_eq!(format_amount(250_000_000), "2,500,000");
        assert_eq!(format_amount(123_450), "1,235", "half away from zero");
        assert_eq!(format_amount(-123_450), "-1,235");
        assert_eq!(format_amount(-40), "-0");
        assert_eq!(format_money(320_000_000, "KES"), "KES 3,200,000");
    }

    #[test]
    fn compact_figures_follow_format_compact() {
        assert_eq!(format_compact(1_570_000_000), "15.7M");
        assert_eq!(
            format_compact(200_000_000),
            "2M",
            "a trailing .0 is dropped"
        );
        assert_eq!(format_compact(85_000_000), "850K");
        assert_eq!(format_compact(1_250_000), "13K", "12.5 rounds up");
        assert_eq!(format_compact(-1_250_000), "-12K", "Math.round of -12.5");
        assert_eq!(format_compact(950_000), "9,500");
        assert_eq!(format_compact(-250_000_000), "-2.5M");
    }

    #[test]
    fn dates_are_the_utc_day() {
        assert_eq!(format_date("2026-12-31T23:59:59Z"), "31 Dec 2026");
        assert_eq!(format_date("2026-01-05"), "5 Jan 2026");
        assert_eq!(format_short_date("2026-09-03T08:00:00.000Z"), "3 Sept");
        assert_eq!(format_date("soon"), "soon");
    }
}
