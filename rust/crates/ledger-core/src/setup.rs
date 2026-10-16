//! A new organisation's books and the engine's view of a member, mirroring
//! `flowtrace/src/lib/ledger/setup.ts`.

use serde::{Deserialize, Serialize};

use crate::engine::Error;
use crate::env::LedgerEnv;
use crate::types::{Actor, ActorId, LedgerState};

/// `createOrgLedger(organizationId, baseCurrency, fiscalYear, env)` (`setup.ts:6-25`).
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
    let _ = (organization_id, base_currency, fiscal_year, env);
    Err(
        crate::errors::LedgerError::fail(crate::errors::IssueCode::InvalidInput, "not ported")
            .into(),
    )
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

/// `actorFromPermissions(user, perms)` (`setup.ts:27-43`).
#[must_use]
pub fn actor_from_permissions(user: &ActorUser, perms: &MemberPermissions) -> Actor {
    let _ = perms;
    Actor {
        id: user.id.clone(),
        name: user.name.clone(),
        can_post: false,
        can_approve: false,
        can_close: false,
    }
}

/// `SYSTEM_ACTOR` (`setup.ts:45-51`).
#[must_use]
pub fn system_actor() -> Actor {
    Actor {
        id: ActorId::from(""),
        name: String::new(),
        can_post: false,
        can_approve: false,
        can_close: false,
    }
}
