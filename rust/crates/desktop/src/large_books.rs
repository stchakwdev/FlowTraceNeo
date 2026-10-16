//! Synthetic books for the General ledger's performance check: a fresh organisation from
//! `ledger_core::setup::create_org_ledger` and a run of balanced two-line entries posted through
//! the engine with the fixed test env, so ids and times are the same on every run.

use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
use ledger_core::engine::{EntryOptions, Error};
use ledger_core::env::fixed;
use ledger_core::setup::{create_org_ledger, system_actor};
use ledger_core::types::{EntryInput, FundId, LineInput, Nullish};
use ledger_core::{Ledger, LedgerState, Minor};

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
    let mut env = fixed::test();
    let mut state = create_org_ledger(LARGE_ORG_ID, "KES", YEAR, &mut env)?;
    let actor = system_actor();
    let mut ledger = Ledger::new(&mut state, &mut env);
    for i in 0..lines / 2 {
        let (debit, credit, memo) = PAIRS[i % PAIRS.len()];
        // Twelve months, days 1 to 28, so every date is in an open 2026 period.
        let month = i % 12 + 1;
        let day = i / 12 % 28 + 1;
        let amount = 100 * (1 + i64::try_from(i % 997).unwrap_or(0)) + 37;
        let input = EntryInput {
            date: format!("{YEAR}-{month:02}-{day:02}"),
            memo: format!("{memo} {}", i + 1),
            lines: vec![side(debit, amount, true), side(credit, amount, false)],
            source_type: None,
            source_id: Nullish::Undefined,
            override_reason: Nullish::Undefined,
        };
        ledger.post(&actor, &input, EntryOptions::default())?;
    }
    Ok(state)
}

/// The fiscal year of the large books.
const YEAR: i32 = 2026;

/// Debit account, credit account and memo of each kind of entry, by template code.
const PAIRS: [(&str, &str, &str); 8] = [
    ("7100", "1110", "Office supplies"),
    ("1110", "4200", "Donation"),
    ("6100", "1110", "Salaries"),
    ("5100", "1120", "Water point repairs"),
    ("1120", "1110", "Transfer to operations"),
    ("7200", "1110", "Rent"),
    ("1110", "4300", "Other income"),
    ("7300", "1120", "Utilities"),
];

/// One line of `amount` minor units on the debit or credit side, unrestricted, base currency.
fn side(code: &str, amount: Minor, debit: bool) -> LineInput {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a JS number input, as TS holds it; amounts stay far below 2^53"
    )]
    let amount = Some(amount as f64);
    LineInput {
        account_id: account_id(code),
        debit: if debit { amount } else { None },
        credit: if debit { None } else { amount },
        currency: None,
        fx_rate: None,
        fund_id: FundId::from(UNRESTRICTED_FUND_ID),
        grant_id: Nullish::Undefined,
        dimension_ids: None,
        flow_node_id: Nullish::Undefined,
        memo: Nullish::Undefined,
    }
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
