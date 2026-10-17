//! The MOD2 reducers for services, flags and the trust score (pre-decisions P15 to P17).
//!
//! Services are registered by an operator (`register_service`, `remove_service`); the model
//! service (JV1) then calls `record_flags` and `record_rubric`, and `recompute_trust` for a
//! history snapshot. Members who can approve spend resolve flags (`resolve_flag`), each
//! resolution an audit event in the books' audit log. Any member may call `recompute_trust` too.

use ledger_core::types::ActorId;
use spacetimedb::{Identity, ReducerContext, Table};

use super::reducers::{Books, change, parse};
use super::services::{
    self, SCOPE_FLAGS, SCOPE_RUBRIC, SCOPE_TRUST, check_remove_service, plan_register_service,
};
use super::tables::{ServiceRow, service};
use super::trust::{self, FlagBatch, Resolution, RubricInput};
use crate::clock::reducer_time;
use crate::reducers::{append_audit, is_operator, text};
use crate::rules::AuditNote;

/// The caller's service row as (label, scopes), if the caller is a service.
fn caller_service(ctx: &ReducerContext) -> Option<(String, Vec<String>)> {
    ctx.db
        .service()
        .identity()
        .find(ctx.sender())
        .map(|s| (s.label, s.scopes))
}

/// The caller as a service holding `scope`, signed for the records it writes.
fn service_with(ctx: &ReducerContext, scope: &str, what: &str) -> Result<String, String> {
    let found = caller_service(ctx);
    let label = services::require_scope(
        found.as_ref().map(|(l, s)| (l.as_str(), s.as_slice())),
        scope,
        what,
    )?;
    Ok(services::signature(label))
}

/// Registers (or re-registers with a new label and scopes) a service identity (P17): operators
/// only. The scopes are `flags`, `rubric` and `trust` ([`services::SCOPES`]).
///
/// # Errors
///
/// A rule of [`plan_register_service`], or the clock.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn register_service(
    ctx: &ReducerContext,
    identity: Identity,
    label: String,
    scopes: Vec<String>,
) -> Result<(), String> {
    let (label, scopes) = plan_register_service(is_operator(ctx, ctx.sender()), &label, &scopes)?;
    let row = ServiceRow {
        identity,
        label,
        scopes,
        registered_by: ctx.sender(),
        registered_at: reducer_time(ctx.timestamp).map_err(text)?.to_string(),
    };
    if ctx.db.service().identity().find(identity).is_some() {
        ctx.db.service().identity().update(row);
    } else {
        ctx.db.service().try_insert(row).map_err(text)?;
    }
    Ok(())
}

/// Removes a service (P17): operators only. What it wrote stays.
///
/// # Errors
///
/// A rule of [`check_remove_service`].
#[spacetimedb::reducer]
pub fn remove_service(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    check_remove_service(
        is_operator(ctx, ctx.sender()),
        ctx.db.service().identity().find(identity).is_some(),
    )?;
    ctx.db.service().identity().delete(identity);
    Ok(())
}

/// One model run's findings, [`trust::record_flags`]: a service with the `flags` scope only;
/// `batch_json` is a [`FlagBatch`].
///
/// # Errors
///
/// Not such a service, no books, a malformed batch, or a rule of [`trust::record_flags`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn record_flags(
    ctx: &ReducerContext,
    org_id: String,
    batch_json: String,
) -> Result<(), String> {
    let by = service_with(ctx, SCOPE_FLAGS, "record flags")?;
    let batch: FlagBatch = parse("flag batch", &batch_json)?;
    let books = Books {
        ctx,
        org_id: &org_id,
    };
    change(ctx, &org_id, |t, c, _| {
        trust::record_flags(t, c.org_id, &mut *c.env, &books, &by, batch).map(drop)
    })
}

/// One model run's rubric scores, [`trust::record_rubric`]: a service with the `rubric` scope
/// only; `rubric_json` is a [`RubricInput`].
///
/// # Errors
///
/// Not such a service, no books, a malformed rubric, or a rule of [`trust::record_rubric`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn record_rubric(
    ctx: &ReducerContext,
    org_id: String,
    rubric_json: String,
) -> Result<(), String> {
    let by = service_with(ctx, SCOPE_RUBRIC, "record rubric scores")?;
    let input: RubricInput = parse("rubric", &rubric_json)?;
    let now = reducer_time(ctx.timestamp).map_err(text)?.to_string();
    change(ctx, &org_id, |t, c, _| {
        trust::record_rubric(t, c.org_id, &now, &by, input).map(drop)
    })
}

/// Resolves an open flag, [`trust::resolve_flag`]: an approver dismisses it (with a reason) or
/// confirms it; `resolution` is `dismissed` or `confirmed`. The books' audit log gets
/// `flag.dismiss` or `flag.confirm` with the flag id and the reason.
///
/// # Errors
///
/// No books, an unknown resolution, or a rule of [`trust::resolve_flag`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn resolve_flag(
    ctx: &ReducerContext,
    org_id: String,
    flag_id: String,
    resolution: String,
    reason: Option<String>,
) -> Result<(), String> {
    let resolution: Resolution = parse(
        "flag resolution",
        &serde_json::Value::String(resolution).to_string(),
    )?;
    let flag = change(ctx, &org_id, |t, c, _| {
        trust::resolve_flag(t, c, &flag_id, resolution, reason.as_deref())
    })?;
    append_audit(
        ctx,
        &org_id,
        AuditNote {
            by: ActorId::from(flag.resolved_by.clone().unwrap_or_default()),
            action: match resolution {
                Resolution::Dismissed => "flag.dismiss",
                Resolution::Confirmed => "flag.confirm",
            },
            subject: flag.id,
            detail: flag.reason.unwrap_or_default(),
        },
    )
}

/// `calculateTrustScoreAPI` (`api.ts:406-472`), [`trust::recompute`]: any member, or a service
/// with the `trust` scope. Writes the row and a history snapshot.
///
/// # Errors
///
/// Neither a member nor such a service, or no books.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn recompute_trust(ctx: &ReducerContext, org_id: String) -> Result<(), String> {
    let service = service_with(ctx, SCOPE_TRUST, "recompute the trust score");
    change(ctx, &org_id, |t, c, _| {
        let by = match (c.member, service) {
            (Some(m), _) => m.user_id.clone(),
            (None, Ok(by)) => by,
            (None, Err(_)) => {
                return Err(format!(
                    "Only a member of organisation {org_id} or a service with the trust scope can recompute its trust score."
                ));
            }
        };
        trust::recompute(t, c.org_id, &mut *c.env, &by);
        Ok(())
    })
}
