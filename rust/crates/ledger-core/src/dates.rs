//! Calendar arithmetic on `YYYY-MM-DD` dates, as the TS modules do it through UTC `Date` values
//! (`reconcile.ts:107-108`, `csv.ts:82-87`, `flowtrace/src/lib/demo/ledger.ts:347-351`).

use crate::types::IsoDate;

/// Whether `year` is a Gregorian leap year.
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1 to 12) of `year`.
pub(crate) fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's `days_from_civil`).
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date `days` days after 1970-01-01 (the inverse of [`days_from_civil`]).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Year, month and day of an [`IsoDate`] (its shape is checked, so the parts are digits).
fn parts(date: &IsoDate) -> (i64, i64, i64) {
    let s = date.as_str();
    let n = |r: std::ops::Range<usize>| s.get(r).and_then(|p| p.parse().ok()).unwrap_or(0);
    (n(0..4), n(5..7), n(8..10))
}

/// The day number of a date as `new Date(`${d}T00:00:00Z`)` reads it: `Date` rolls an
/// out-of-range day or month over (`2026-02-30` is `2026-03-02`), and so does this.
pub(crate) fn day_number(date: &IsoDate) -> i64 {
    let (y, m, d) = parts(date);
    let (y, m) = (y + (m - 1).div_euclid(12), (m - 1).rem_euclid(12) + 1);
    days_from_civil(y, m, 1) + d - 1
}

/// `YYYY-MM-DD` of a day number; years outside 0 to 9999 are clamped into the shape.
pub(crate) fn from_day_number(days: i64) -> IsoDate {
    let (y, m, d) = civil_from_days(days);
    IsoDate::from_parts(y.clamp(0, 9999), m, d)
}

/// `addDays(d, n)` (`demo/ledger.ts:347-351`).
pub(crate) fn add_days(date: &IsoDate, n: i64) -> IsoDate {
    from_day_number(day_number(date) + n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> IsoDate {
        IsoDate::parse(s).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn day_numbers_round_trip() {
        assert_eq!(day_number(&d("1970-01-01")), 0);
        assert_eq!(
            day_number(&d("2026-01-31")) + 1,
            day_number(&d("2026-02-01"))
        );
        assert_eq!(add_days(&d("2028-02-28"), 1), d("2028-02-29"));
        assert_eq!(add_days(&d("2026-12-31"), 1), d("2027-01-01"));
        assert_eq!(day_number(&d("2026-02-30")), day_number(&d("2026-03-02")));
    }
}
