//! The books `create_org_ledger` opens: ledger-core's `createOrgLedger`
//! (`flowtrace/src/lib/ledger/setup.ts:11-25`, `ledger_core::setup::create_org_ledger`) with the
//! reducer's env, as the TS module calls `seedOrgLedger(orgId, baseCurrency, fiscalYear,
//! envFor(ctx))` (`spacetimedb/src/index.ts:564-569`): default settings, the NGO chart of
//! accounts, the unrestricted fund and the twelve periods of the fiscal year.

use ledger_core::LedgerEnv;
use ledger_core::types::LedgerState;

use crate::command::engine_error_text;

/// A new organisation's books.
///
/// # Errors
///
/// The engine's error as `run_command` reports it ([`engine_error_text`]), as TS rethrows it
/// (`index.ts:567-569`); a fiscal year past `i32` cannot reach the engine, as the reducer checks
/// 2000 to 2100 first.
pub fn seed_books(
    org_id: &str,
    base_currency: &str,
    fiscal_year: u32,
    env: &mut dyn LedgerEnv,
) -> Result<LedgerState, String> {
    let year = i32::try_from(fiscal_year)
        .map_err(|_| "Fiscal year must be between 2000 and 2100.".to_owned())?;
    ledger_core::setup::create_org_ledger(org_id, base_currency, year, env)
        .map_err(|e| engine_error_text(&e))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_core::env::fixed;

    use crate::codec::assemble_state;
    use crate::write_back::books_to_rows;

    fn setup_fixture() -> serde_json::Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/setup.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn the_seeded_books_are_the_ts_create_org_ledger_books() {
        let case = &setup_fixture()["createOrgLedger"];
        assert_eq!(case["args"], serde_json::json!(["org", "KES", 2026]));
        let books = seed_books("org", "KES", 2026, &mut fixed::test()).unwrap();
        assert_eq!(serde_json::to_value(&books).unwrap(), case["state"]);
    }

    #[test]
    fn the_seeded_books_survive_their_rows() {
        let books = seed_books("org", "KES", 2026, &mut fixed::test()).unwrap();
        let rows = books_to_rows("org", &books, 0).unwrap();
        let back = assemble_state(&rows.settings_json, &rows.counters_json, &rows.rows).unwrap();
        // 54 accounts, 1 fund, 12 periods and nothing else (`setup.json#/createOrgLedger`).
        assert_eq!((back, rows.next_seq), (books, 54 + 1 + 12));
    }

    #[test]
    fn seeding_draws_no_ids() {
        let mut env = fixed::test();
        seed_books("org", "UGX", 2027, &mut env).unwrap();
        assert_eq!(env.drawn(), 0);
    }
}
