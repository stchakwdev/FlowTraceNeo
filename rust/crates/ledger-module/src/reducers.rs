//! Reducers (`index.ts:529-691`): look the rows up, let [`crate::rules`] decide, write the result.
//!
//! A reducer that returns `Err` rolls its whole transaction back and the caller gets the message
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.reducer.html>), as a `SenderError` does in
//! TS. Arguments arrive by value from the host, hence the `needless_pass_by_value` expectations.

use std::fmt::Display;

use ledger_core::types::{AuditEvent, AuditEventId};
use spacetimedb::{Identity, ReducerContext, Table};

use crate::clock::reducer_time;
use crate::codec::{OrgRows, assemble_state, doc_to_row};
use crate::command::{self, ModuleEnv};
use crate::rules::{self, AuditNote, MemberArgs};
use crate::seed::seed_books;
use crate::write_back::{self, NewRows, TableOps, WriteBack};
// The table handle traits the table macro generates.
use crate::tables::{
    AuditEventRow, CommandResult, LedgerOrg, Member, Operator, account, audit_event, bank_account,
    bill, budget, budget_line, command_result, dimension, expense_claim, fiscal_period, fund,
    grant, journal_entry, journal_line, ledger_org, member, operator, payment, statement_line,
    vendor,
};

/// An error as the text a reducer returns.
pub(crate) fn text(e: impl Display) -> String {
    e.to_string()
}

pub(crate) fn is_operator(ctx: &ReducerContext, identity: Identity) -> bool {
    ctx.db.operator().identity().find(identity).is_some()
}

/// `appendAudit` (`index.ts:521-527`): one audit event with a reducer-seeded UUID v4 and the
/// reducer time, at the organisation's next `seq`.
pub(crate) fn append_audit(
    ctx: &ReducerContext,
    org_id: &str,
    note: AuditNote,
) -> Result<(), String> {
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

/// Opens books for a new organisation and makes the caller its owner (`index.ts:554-595`):
/// ledger-core's `createOrgLedger` with the reducer env ([`seed_books`]), written as new rows
/// numbered from 0, then the owner and the `ledger.create` audit event.
///
/// # Errors
///
/// The TS messages of [`rules::check_new_org`], and an engine refusal as `run_command` reports it.
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
    let mut env = reducer_env(ctx)?;
    let books = seed_books(&org_id, &base_currency, fiscal_year, &mut env)?;
    env.finish()?;
    let new_rows = write_back::books_to_rows(&org_id, &books, 0)?;
    let org = ctx
        .db
        .ledger_org()
        .try_insert(LedgerOrg {
            org_id: org_id.clone(),
            settings_json: new_rows.settings_json.clone(),
            counters_json: new_rows.counters_json.clone(),
            next_seq: 0,
            created_by: ctx.sender(),
            created_at: ctx.timestamp,
        })
        .map_err(text)?;
    write_all(ctx, org, new_rows)?;
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

/// `loadOrg` (`index.ts:376-410`): the organisation and every ledger row it owns.
fn load_org(ctx: &ReducerContext, org_id: &str) -> Result<(LedgerOrg, OrgRows), String> {
    let org = ctx
        .db
        .ledger_org()
        .org_id()
        .find(org_id.to_owned())
        .ok_or_else(|| format!("No ledger for organisation {org_id}."))?;
    let org_id = org_id.to_owned();
    let rows = OrgRows {
        entries: ctx.db.journal_entry().org_id().filter(&org_id).collect(),
        lines: ctx.db.journal_line().org_id().filter(&org_id).collect(),
        accounts: ctx.db.account().org_id().filter(&org_id).collect(),
        funds: ctx.db.fund().org_id().filter(&org_id).collect(),
        grants: ctx.db.grant().org_id().filter(&org_id).collect(),
        dimensions: ctx.db.dimension().org_id().filter(&org_id).collect(),
        periods: ctx.db.fiscal_period().org_id().filter(&org_id).collect(),
        budgets: ctx.db.budget().org_id().filter(&org_id).collect(),
        budget_lines: ctx.db.budget_line().org_id().filter(&org_id).collect(),
        vendors: ctx.db.vendor().org_id().filter(&org_id).collect(),
        bills: ctx.db.bill().org_id().filter(&org_id).collect(),
        payments: ctx.db.payment().org_id().filter(&org_id).collect(),
        claims: ctx.db.expense_claim().org_id().filter(&org_id).collect(),
        bank_accounts: ctx.db.bank_account().org_id().filter(&org_id).collect(),
        statement_lines: ctx.db.statement_line().org_id().filter(&org_id).collect(),
        audit: ctx.db.audit_event().org_id().filter(&org_id).collect(),
    };
    Ok((org, rows))
}

/// Applies one table's [`TableOps`] through its `key` index: `ctx.db.<table>()`, the ops.
macro_rules! write_table {
    ($table:expr, $ops:expr) => {{
        let table = $table;
        let ops: TableOps<_> = $ops;
        for key in &ops.delete {
            table.key().delete(key);
        }
        for row in ops.update {
            table.key().update(row);
        }
        for row in ops.insert {
            table.try_insert(row).map_err(text)?;
        }
    }};
}

/// The table half of `writeBack` (`index.ts:464-505`): every planned row change, then the
/// organisation's settings, counters and next `seq`.
fn write_back(ctx: &ReducerContext, org: LedgerOrg, plan: WriteBack) -> Result<(), String> {
    write_table!(ctx.db.journal_entry(), plan.entries);
    write_table!(ctx.db.journal_line(), plan.lines);
    write_table!(ctx.db.account(), plan.accounts);
    write_table!(ctx.db.fund(), plan.funds);
    write_table!(ctx.db.grant(), plan.grants);
    write_table!(ctx.db.dimension(), plan.dimensions);
    write_table!(ctx.db.fiscal_period(), plan.periods);
    write_table!(ctx.db.budget(), plan.budgets);
    write_table!(ctx.db.budget_line(), plan.budget_lines);
    write_table!(ctx.db.vendor(), plan.vendors);
    write_table!(ctx.db.bill(), plan.bills);
    write_table!(ctx.db.payment(), plan.payments);
    write_table!(ctx.db.expense_claim(), plan.claims);
    write_table!(ctx.db.bank_account(), plan.bank_accounts);
    write_table!(ctx.db.statement_line(), plan.statement_lines);
    write_table!(ctx.db.audit_event(), plan.audit);
    ctx.db.ledger_org().org_id().update(LedgerOrg {
        settings_json: plan.settings_json,
        counters_json: plan.counters_json,
        next_seq: plan.next_seq,
        ..org
    });
    Ok(())
}

/// `envFor(ctx)` (`index.ts:361-365`): the reducer time in milliseconds ([`crate::clock`]) and
/// a fresh `ctx.new_uuid_v4()` per id.
pub(crate) fn reducer_env(
    ctx: &ReducerContext,
) -> Result<ModuleEnv<impl FnMut() -> Result<String, String> + '_>, String> {
    let now = reducer_time(ctx.timestamp).map_err(text)?;
    Ok(ModuleEnv::new(now, move || {
        ctx.new_uuid_v4().map(|id| id.to_string()).map_err(text)
    }))
}

/// The single ledger write path (`runCommand`, `index.ts:655-691`): applies one `LedgerCommand`
/// (JSON) with the caller's member permissions through ledger-core, writes back what changed,
/// and stores the result in `command_result` under the caller's identity.
///
/// The order is the TS order: member, JSON, sanitise, load, apply, write back, result. Ids and
/// time come from the reducer as in TS (`envFor`, `index.ts:361-365`): UUID v4s drawn from
/// `ctx.new_uuid_v4()` and `ctx.timestamp` in milliseconds ([`crate::clock`]).
///
/// A repeated `request_id` is not special, as in TS: the command runs again and the caller's one
/// `command_result` row is overwritten (`index.ts:686-689`; the table is keyed by identity,
/// `index.ts:142-152`). Any error rolls the whole call back, so a refused command writes no row
/// and leaves the previous result in place.
///
/// # Errors
///
/// The TS texts: not a member, not JSON, `LEDGER_ERROR <issues>` for a ledger rule
/// ([`command::ledger_error_text`]), no ledger, and the write-back guards of
/// [`write_back::plan`].
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
    let actor = rules::actor_of(&org_id, caller.as_ref())?;
    let cmd = command::parse_command(&command_json)?;

    let (org, rows) = load_org(ctx, &org_id)?;
    let before = assemble_state(&org.settings_json, &org.counters_json, &rows).map_err(text)?;
    let mut after = before.clone();
    let mut env = reducer_env(ctx)?;
    let result = command::apply(&mut after, &mut env, &actor, cmd)?;
    env.finish()?;
    let plan = write_back::plan(&org_id, &rows, org.next_seq, &before, &after)?;
    write_back(ctx, org, plan)?;

    let row = CommandResult {
        identity: ctx.sender(),
        org_id,
        request_id,
        result_json: command::result_json(&result)?,
        at: ctx.timestamp,
    };
    if ctx
        .db
        .command_result()
        .identity()
        .find(ctx.sender())
        .is_some()
    {
        ctx.db.command_result().identity().update(row);
    } else {
        ctx.db.command_result().try_insert(row).map_err(text)?;
    }
    Ok(())
}

/// Replaces an organisation's books with a whole `LedgerState` (JSON): every ledger row of the
/// organisation is deleted and the books written as new rows through
/// [`crate::codec::state_to_rows`], numbered on from the organisation's `next_seq`. Members and
/// command results stay. No TS counterpart; it is how the demo books reach a database for the
/// end-to-end checks. Only an operator may call it, and the books must exist
/// (`create_org_ledger` first) and name the organisation in their settings.
///
/// # Errors
///
/// Not an operator, no ledger, not a `LedgerState`, books of another organisation, or a record
/// that does not convert ([`write_back::import`]).
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn import_books(
    ctx: &ReducerContext,
    org_id: String,
    state_json: String,
) -> Result<(), String> {
    rules::check_import_books(is_operator(ctx, ctx.sender()))?;
    let (org, _) = load_org(ctx, &org_id)?;
    let import = write_back::import(&org_id, &state_json, org.next_seq)?;
    ctx.db.journal_entry().org_id().delete(&org_id);
    ctx.db.journal_line().org_id().delete(&org_id);
    ctx.db.account().org_id().delete(&org_id);
    ctx.db.fund().org_id().delete(&org_id);
    ctx.db.grant().org_id().delete(&org_id);
    ctx.db.dimension().org_id().delete(&org_id);
    ctx.db.fiscal_period().org_id().delete(&org_id);
    ctx.db.budget().org_id().delete(&org_id);
    ctx.db.budget_line().org_id().delete(&org_id);
    ctx.db.vendor().org_id().delete(&org_id);
    ctx.db.bill().org_id().delete(&org_id);
    ctx.db.payment().org_id().delete(&org_id);
    ctx.db.expense_claim().org_id().delete(&org_id);
    ctx.db.bank_account().org_id().delete(&org_id);
    ctx.db.statement_line().org_id().delete(&org_id);
    ctx.db.audit_event().org_id().delete(&org_id);
    write_all(ctx, org, import)
}

/// Writes a whole set of books into tables that hold none of the organisation's rows, and the
/// organisation's settings, counters and next `seq`.
fn write_all(ctx: &ReducerContext, org: LedgerOrg, new_rows: NewRows) -> Result<(), String> {
    let rows = new_rows.rows;
    write_back(
        ctx,
        org,
        WriteBack {
            entries: inserted(rows.entries),
            lines: inserted(rows.lines),
            accounts: inserted(rows.accounts),
            funds: inserted(rows.funds),
            grants: inserted(rows.grants),
            dimensions: inserted(rows.dimensions),
            periods: inserted(rows.periods),
            budgets: inserted(rows.budgets),
            budget_lines: inserted(rows.budget_lines),
            vendors: inserted(rows.vendors),
            bills: inserted(rows.bills),
            payments: inserted(rows.payments),
            claims: inserted(rows.claims),
            bank_accounts: inserted(rows.bank_accounts),
            statement_lines: inserted(rows.statement_lines),
            audit: inserted(rows.audit),
            settings_json: new_rows.settings_json,
            counters_json: new_rows.counters_json,
            next_seq: new_rows.next_seq,
        },
    )
}

/// Rows to insert into an emptied table.
fn inserted<R>(rows: Vec<R>) -> TableOps<R> {
    TableOps {
        insert: rows,
        update: Vec::new(),
        delete: Vec::new(),
    }
}
