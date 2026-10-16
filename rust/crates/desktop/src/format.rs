//! Display forms shared by the screens: amounts in minor units and dates, the way the web app
//! writes them (`flowtrace/src/components/ledger/money.tsx`, `formatMinor`;
//! `flowtrace/src/lib/format.ts`, `formatShortDate`).

use ledger_core::Minor;
use ledger_core::types::IsoDate;

/// What an empty amount cell shows: an en dash, so empty cells read as empty (`money.tsx:12`).
pub const EMPTY: &str = "\u{2013}";

/// `formatMinor(minor, { zero })` (`money.tsx:8-21`): `1,250.50` from 125050 minor units, two
/// decimals, thousands grouped with commas (`en-KE`), negatives in parentheses, and `zero` for 0.
///
/// Integer arithmetic only: `Math.abs(minor) / 100` printed to two places is exact for every
/// safe integer amount, so no float rounding is involved.
#[must_use]
pub fn amount(minor: Minor, zero: &str) -> String {
    todo!()
}

/// A balance: as [`amount`], with zero written `0.00` (`general.tsx`, `<Money zero="0.00">`).
#[must_use]
pub fn balance(minor: Minor) -> String {
    todo!()
}

/// `n` with a comma between each group of three digits.
fn grouped(n: u64) -> String {
    todo!()
}

/// `formatShortDate` (`format.ts:22-28`, `en-GB`, day numeric, month short, UTC): `4 Feb` from
/// `2026-02-04`. A month outside 1-12 prints the date as stored.
#[must_use]
pub fn short_date(date: &IsoDate) -> String {
    todo!()
}

/// `formatDate` (`format.ts`, `en-GB`, day numeric, month short, year numeric): `4 Feb 2026`.
#[must_use]
pub fn long_date(date: &IsoDate) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    #[test]
    fn amounts_group_thousands_and_put_negatives_in_parentheses() {
        assert_eq!(amount(125_050, EMPTY), "1,250.50");
        assert_eq!(amount(-125_050, EMPTY), "(1,250.50)");
        assert_eq!(amount(5, EMPTY), "0.05");
        assert_eq!(amount(100, EMPTY), "1.00");
        assert_eq!(amount(123_456_789_012, EMPTY), "1,234,567,890.12");
        assert_eq!(amount(99_999_900, EMPTY), "999,999.00");
        assert_eq!(amount(0, EMPTY), "\u{2013}");
        assert_eq!(amount(i64::MIN, EMPTY), "(92,233,720,368,547,758.08)");
    }

    #[test]
    fn balances_write_zero_as_figures() {
        assert_eq!(balance(0), "0.00");
        assert_eq!(balance(-1), "(0.01)");
        assert_eq!(balance(1_000_000), "10,000.00");
    }

    #[test]
    fn dates_print_day_and_short_month() {
        let d = IsoDate::parse("2026-02-04").unwrap();
        assert_eq!(short_date(&d), "4 Feb");
        assert_eq!(long_date(&d), "4 Feb 2026");
        assert_eq!(short_date(&IsoDate::parse("2025-12-31").unwrap()), "31 Dec");
        assert_eq!(
            short_date(&IsoDate::parse("2025-13-01").unwrap()),
            "2025-13-01"
        );
        assert_eq!(
            long_date(&IsoDate::parse("2025-00-01").unwrap()),
            "2025-00-01"
        );
    }
}
