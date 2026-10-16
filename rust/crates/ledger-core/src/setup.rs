//! A new organisation's books and the engine's view of a member, mirroring
//! `flowtrace/src/lib/ledger/setup.ts`.

use serde::{Deserialize, Serialize};

use crate::coa_template::{UNRESTRICTED_FUND_ID, default_settings, ngo_chart_of_accounts};
use crate::engine::{Error, Ledger, empty_ledger};
use crate::env::LedgerEnv;
use crate::types::{Actor, ActorId, Fund, FundId, FundRestriction, LedgerState};

/// `createOrgLedger(organizationId, baseCurrency, fiscalYear, env)` (`setup.ts:6-25`): a new
/// organisation's books, with the NGO chart of accounts, an unrestricted fund and the twelve
/// periods of the fiscal year; accrual basis, IFRS for SMEs presentation, one base currency.
/// TS returns the `Ledger`; Rust returns the books, for `Ledger::new(&mut books, env)`.
/// Draws no ids.
///
/// # Errors
///
/// [`Error`] when the fiscal year cannot be opened.
pub fn create_org_ledger(
    organization_id: &str,
    base_currency: &str,
    fiscal_year: i32,
    env: &mut dyn LedgerEnv,
) -> Result<LedgerState, Error> {
    let mut state = empty_ledger(default_settings(organization_id, base_currency));
    state.accounts = ngo_chart_of_accounts();
    state.funds = vec![Fund {
        id: FundId::from(UNRESTRICTED_FUND_ID),
        name: "Unrestricted".to_owned(),
        restriction: FundRestriction::Unrestricted,
    }];
    Ledger::new(&mut state, env).ensure_fiscal_year(fiscal_year)?;
    Ok(state)
}

/// The user half of `actorFromPermissions` (`setup.ts:29`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorUser {
    /// User id.
    pub id: ActorId,
    /// Display name.
    pub name: String,
}

/// A member's access-group permissions (`setup.ts:30-34`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberPermissions {
    /// May edit transactions: post and approve.
    pub can_edit_transactions: bool,
    /// May manage access groups: close periods.
    pub can_manage_groups: bool,
    /// May manage members; not used by the engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_manage_members: Option<bool>,
}

/// `actorFromPermissions(user, perms)` (`setup.ts:27-43`): editing transactions grants posting
/// and approving; managing groups grants closing periods.
#[must_use]
pub fn actor_from_permissions(user: &ActorUser, perms: &MemberPermissions) -> Actor {
    Actor {
        id: user.id.clone(),
        name: user.name.clone(),
        can_post: perms.can_edit_transactions,
        can_approve: perms.can_edit_transactions,
        can_close: perms.can_manage_groups,
    }
}

/// `SYSTEM_ACTOR` (`setup.ts:45-51`): `system`, named FlowTrace, with every permission.
#[must_use]
pub fn system_actor() -> Actor {
    Actor {
        id: ActorId::from("system"),
        name: "FlowTrace".to_owned(),
        can_post: true,
        can_approve: true,
        can_close: true,
    }
}
