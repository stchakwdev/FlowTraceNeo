//! Reducers (`index.ts:529-691`): look the rows up, let [`crate::rules`] decide, write the result.
//!
//! A reducer that returns `Err` rolls its whole transaction back and the caller gets the message
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.reducer.html>), as a `SenderError` does in
//! TS. Arguments arrive by value from the host, hence the `needless_pass_by_value` expectations.

use std::collections::BTreeMap;
use std::fmt::Display;

use ledger_core::types::{AuditEvent, AuditEventId};
use spacetimedb::{Identity, ReducerContext, Table};

use crate::clock::reducer_time;
use crate::codec::{counters_json, doc_to_row, settings_json};
use crate::rules::{self, AuditNote, MemberArgs};
use crate::seed::default_settings;
// The table handle traits the table macro generates.
use crate::tables::{
    AuditEventRow, LedgerOrg, Member, Operator, audit_event, ledger_org, member, operator,
};

/// An error as the text a reducer returns.
fn text(e: impl Display) -> String {
    e.to_string()
}

fn is_operator(ctx: &ReducerContext, identity: Identity) -> bool {
    ctx.db.operator().identity().find(identity).is_some()
}

/// `appendAudit` (`index.ts:521-527`): one audit event with a reducer-seeded UUID v4 and the
/// reducer time, at the organisation's next `seq`.
fn append_audit(ctx: &ReducerContext, org_id: &str, note: AuditNote) -> Result<(), String> {
    let mut org = ctx
        .db
        .ledger_org()
        .org_id()
        .find(org_id.to_owned())
        .ok_or_else(|| format!("No ledger for organisation {org_id}."))?;
    let event = AuditEvent {
        id: AuditEventId::from(ctx.new_uuid_v4().map_err(text)?.to_string()),
        at: reducer_time(ctx.timestamp).map_err(text)?,
        by: note.by,
        action: note.action.to_owned(),
        subject: note.subject,
        detail: note.detail,
    };
    let row: AuditEventRow = doc_to_row(org_id, &event, org.next_seq).map_err(text)?;
    ctx.db.audit_event().try_insert(row).map_err(text)?;
    org.next_seq = org
        .next_seq
        .checked_add(1)
        .ok_or_else(|| format!("Organisation {org_id} has run out of row numbers."))?;
    ctx.db.ledger_org().org_id().update(org);
    Ok(())
}

/// The publisher becomes the first operator (`index.ts:541-543`).
///
/// # Errors
///
/// Never in practice: the operator table is empty when the module is first published.
#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    ctx.db
        .operator()
        .try_insert(Operator {
            identity: ctx.sender(),
            added_at: ctx.timestamp,
        })
        .map_err(text)?;
    Ok(())
}

/// Lets an operator allow another identity to open books (`index.ts:545-552`). Adding an
/// existing operator changes nothing.
///
/// # Errors
///
/// When the caller is not an operator.
#[spacetimedb::reducer]
pub fn add_operator(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    rules::check_add_operator(is_operator(ctx, ctx.sender()))?;
    if !is_operator(ctx, identity) {
        ctx.db
            .operator()
            .try_insert(Operator {
                identity,
                added_at: ctx.timestamp,
            })
            .map_err(text)?;
    }
    Ok(())
}

/// Withdraws an operator; no TS counterpart, rules in [`rules::check_remove_operator`].
///
/// # Errors
///
/// When the caller is not an operator, the identity is not one, or it is the last one.
#[spacetimedb::reducer]
pub fn remove_operator(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    rules::check_remove_operator(
        is_operator(ctx, ctx.sender()),
        is_operator(ctx, identity),
        ctx.db.operator().count(),
    )?;
    ctx.db.operator().identity().delete(identity);
    Ok(())
}

/// Opens books for a new organisation and makes the caller its owner (`index.ts:554-595`).
///
/// The scaffold stores the default settings and no ledger rows yet; [`crate::seed`] says why
/// and what task S4 adds.
///
/// # Errors
///
/// The TS messages of [`rules::check_new_org`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_org_ledger(
    ctx: &ReducerContext,
    org_id: String,
    base_currency: String,
    fiscal_year: u32,
    display_name: String,
) -> Result<(), String> {
    let exists = ctx.db.ledger_org().org_id().find(&org_id).is_some();
    rules::check_new_org(
        is_operator(ctx, ctx.sender()),
        &org_id,
        &base_currency,
        fiscal_year,
        exists,
    )?;
    let settings = default_settings(&org_id, &base_currency).map_err(text)?;
    ctx.db
        .ledger_org()
        .try_insert(LedgerOrg {
            org_id: org_id.clone(),
            settings_json: settings_json(&settings).map_err(text)?,
            counters_json: counters_json(&BTreeMap::new()).map_err(text)?,
            next_seq: 0,
            created_by: ctx.sender(),
            created_at: ctx.timestamp,
        })
        .map_err(text)?;
    let jwt_subject = ctx.sender_auth().jwt().map(|jwt| jwt.subject().to_owned());
    let made = rules::new_org(
        &org_id,
        ctx.sender(),
        jwt_subject.as_deref(),
        &base_currency,
        fiscal_year,
        &display_name,
    );
    ctx.db.member().try_insert(made.owner).map_err(text)?;
    append_audit(ctx, &org_id, made.audit)
}

/// Adds a member or changes their permissions; needs `can_close` (`index.ts:597-638`).
///
/// # Errors
///
/// The TS messages of [`rules::plan_add_member`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    reason = "reducer arguments arrive by value from the host, one per TS argument (index.ts:599-607)"
)]
pub fn add_member(
    ctx: &ReducerContext,
    org_id: String,
    identity: Identity,
    user_id: String,
    name: String,
    can_post: bool,
    can_approve: bool,
    can_close: bool,
) -> Result<(), String> {
    let org_members: Vec<Member> = ctx.db.member().org_id().filter(&org_id).collect();
    let args = MemberArgs {
        identity,
        user_id: &user_id,
        name: &name,
        can_post,
        can_approve,
        can_close,
    };
    let change = rules::plan_add_member(&org_id, ctx.sender(), &org_members, &args)?;
    if change.replaces {
        ctx.db.member().key().update(change.row);
    } else {
        ctx.db.member().try_insert(change.row).map_err(text)?;
    }
    append_audit(ctx, &org_id, change.audit)
}

/// Removes a member; needs `can_close`, and nobody removes themselves (`index.ts:640-653`).
///
/// # Errors
///
/// The TS messages of [`rules::plan_remove_member`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn remove_member(
    ctx: &ReducerContext,
    org_id: String,
    identity: Identity,
) -> Result<(), String> {
    let org_members: Vec<Member> = ctx.db.member().org_id().filter(&org_id).collect();
    let removal = rules::plan_remove_member(&org_id, ctx.sender(), &org_members, identity)?;
    ctx.db.member().key().delete(&removal.key);
    append_audit(ctx, &org_id, removal.audit)
}

/// The single ledger write path (`index.ts:655-691`), with the TS arguments and the TS checks
/// that need no engine: the caller must be a member and the command must be JSON. Running the
/// command through ledger-core, writing back the diff and storing the result in
/// `command_result` is task S4; until then the reducer answers with
/// [`rules::not_implemented`] and writes nothing.
///
/// # Errors
///
/// Always: not a member, not JSON, or not implemented yet.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn run_command(
    ctx: &ReducerContext,
    org_id: String,
    request_id: String,
    command_json: String,
) -> Result<(), String> {
    let caller = ctx
        .db
        .member()
        .key()
        .find(rules::member_key(&org_id, ctx.sender()));
    rules::actor_of(&org_id, caller.as_ref())?;
    let shape = rules::command_shape(&command_json)?;
    Err(rules::not_implemented(&org_id, &request_id, &shape))
}
