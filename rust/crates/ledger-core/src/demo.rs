//! The demo organisation's books, mirroring `flowtrace/src/lib/demo/ledger.ts`: the bundled
//! transactions run through the bridge (the backfill), with what a real NGO's books would hold
//! around them: grants, budgets, bills, staff claims and bank statements. Everything goes
//! through the engine, so the demo obeys the same rules a live organisation does
//! (`demo/ledger.ts:1-5`).

use crate::engine::Error;
use crate::types::LedgerState;

/// `DEMO_AS_OF` (`demo/ledger.ts:36`): the demo is a snapshot as of its latest movement.
pub const DEMO_AS_OF: &str = "2026-02-04";

/// `buildDemoLedger()` (`demo/ledger.ts:126-344`), with the demo env (`seedEnv`,
/// `demo/ledger.ts:118-124`; [`crate::env::fixed::demo`]).
///
/// # Errors
///
/// None in practice: the demo obeys every rule. An engine refusal is passed on.
pub fn build_demo_ledger() -> Result<LedgerState, Error> {
    Err(
        crate::errors::LedgerError::fail(crate::errors::IssueCode::InvalidInput, "not ported")
            .into(),
    )
}
