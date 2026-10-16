//! Synthetic books for the General ledger's performance check: a fresh organisation from
//! `ledger_core::setup::create_org_ledger` and a run of balanced two-line entries posted through
//! the engine with the fixed test env, so ids and times are the same on every run.

use ledger_core::LedgerState;
use ledger_core::engine::Error;

/// The organisation name the large books show under.
pub const LARGE_ORG_NAME: &str = "Large test books";

/// The organisation id of the large books.
pub const LARGE_ORG_ID: &str = "org-large-test";

/// Books with `lines` posted journal lines (rounded down to an even number: every entry has
/// two), spread over the twelve months of 2026 across eight account pairs.
///
/// # Errors
///
/// The engine's [`Error`] when the books or an entry cannot be built.
pub fn build(lines: usize) -> Result<LedgerState, Error> {
    let _ = lines;
    todo!()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use ledger_core::reports::{self, Basis};
    use ledger_core::types::EntryStatus;

    use super::*;
    use crate::general_ledger::{GlFilter, GlTable};

    #[test]
    fn the_books_hold_the_lines_asked_for_and_balance() {
        let books = build(2_000).unwrap();
        assert_eq!(books.lines.len(), 2_000);
        assert_eq!(books.entries.len(), 1_000);
        assert!(
            books
                .entries
                .iter()
                .all(|e| e.status == EntryStatus::Posted)
        );
        assert_eq!(books.settings.organization_id.as_str(), LARGE_ORG_ID);
        let tb = reports::trial_balance(&books, None, Basis::Accrual);
        assert_eq!(tb.difference, 0);
        let months: std::collections::BTreeSet<_> = books
            .entries
            .iter()
            .map(|e| &e.date.as_str()[..7])
            .collect();
        assert_eq!(months.len(), 12);
    }

    #[test]
    fn fifty_thousand_lines_derive_one_row_each() {
        let books = build(50_000).unwrap();
        let table = GlTable::derive(&books, &GlFilter::default());
        assert_eq!(table.rows.len(), 50_000);
        assert_eq!(table.totals.debits, table.totals.credits);
    }
}
