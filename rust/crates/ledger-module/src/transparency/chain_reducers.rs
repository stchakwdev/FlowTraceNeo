//! The MOD2 chain reducers: each loads the chains its organisations are on, runs one
//! [`super::chains`] rule with the caller's member row on the side it acts for, and writes what
//! changed. Chains feed none of the trust score's inputs and none of the public rows, so neither
//! moves here.

use spacetimedb::{ReducerContext, Table};

use super::chains::{self, ChainStatus, Chains};
use super::reducers::load;
use super::rows::{ChainRows, chains_from_rows, plan_chain_write};
use super::tables::{chain_activity_log, organization_chain};
use crate::reducers::{reducer_env, text};
use crate::rules::member_key;
use crate::tables::{Member, ledger_org, member};

/// The chains any of `orgs` is on, and their log entries.
pub(super) fn load_chains(ctx: &ReducerContext, orgs: &[&str]) -> ChainRows {
    let mut rows = ChainRows::default();
    for org in orgs {
        let org = (*org).to_owned();
        for c in ctx
            .db
            .organization_chain()
            .source_org_id()
            .filter(&org)
            .chain(ctx.db.organization_chain().target_org_id().filter(&org))
        {
            if !rows.chains.iter().any(|r| r.id == c.id) {
                rows.chains.push(c);
            }
        }
    }
    for c in &rows.chains {
        rows.log
            .extend(ctx.db.chain_activity_log().chain_id().filter(&c.id));
    }
    rows
}

/// Writes the change from `before` to `after`, new rows numbered after each table's highest.
pub(super) fn write_chains(
    ctx: &ReducerContext,
    before: &ChainRows,
    after: &Chains,
) -> Result<(), String> {
    let next_chain = ctx
        .db
        .organization_chain()
        .iter()
        .map(|r| r.seq + 1)
        .max()
        .unwrap_or(0);
    let next_log = ctx
        .db
        .chain_activity_log()
        .iter()
        .map(|r| r.seq + 1)
        .max()
        .unwrap_or(0);
    let w = plan_chain_write(before, after, next_chain, next_log).map_err(text)?;
    for id in &w.chains.delete {
        ctx.db.organization_chain().id().delete(id);
    }
    for row in w.chains.update {
        ctx.db.organization_chain().id().update(row);
    }
    for row in w.chains.insert {
        ctx.db.organization_chain().try_insert(row).map_err(text)?;
    }
    for id in &w.log.delete {
        ctx.db.chain_activity_log().id().delete(id);
    }
    for row in w.log.update {
        ctx.db.chain_activity_log().id().update(row);
    }
    for row in w.log.insert {
        ctx.db.chain_activity_log().try_insert(row).map_err(text)?;
    }
    Ok(())
}

fn member_of(ctx: &ReducerContext, org_id: &str) -> Option<Member> {
    ctx.db.member().key().find(member_key(org_id, ctx.sender()))
}

/// The chain as stored, for which side acts; the demo's refusal when there is none.
fn find(
    ctx: &ReducerContext,
    chain_id: &str,
    required: ChainStatus,
) -> Result<(String, String), String> {
    ctx.db
        .organization_chain()
        .id()
        .find(chain_id.to_owned())
        .map(|r| (r.source_org_id, r.target_org_id))
        .ok_or_else(|| chains::no_longer(required))
}

/// Loads the chains of `orgs`, runs `f` on them with the reducer's ids and clock, writes back.
fn run<T>(
    ctx: &ReducerContext,
    orgs: &[&str],
    f: impl FnOnce(&mut Chains, &mut dyn ledger_core::env::LedgerEnv) -> Result<T, String>,
) -> Result<T, String> {
    let before = load_chains(ctx, orgs);
    let mut c = chains_from_rows(&before).map_err(text)?;
    let mut env = reducer_env(ctx)?;
    let out = f(&mut c, &mut env)?;
    env.finish()?;
    write_chains(ctx, &before, &c)?;
    Ok(out)
}

/// `requestChain` (`api.ts:865-892`), [`chains::request_chain`]: the caller asks for
/// `source_org_id`.
///
/// # Errors
///
/// A rule of [`chains::request_chain`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn request_chain(
    ctx: &ReducerContext,
    source_org_id: String,
    target_org_id: String,
    message: Option<String>,
) -> Result<(), String> {
    let caller = member_of(ctx, &source_org_id);
    let target_exists = ctx.db.ledger_org().org_id().find(&target_org_id).is_some();
    run(ctx, &[source_org_id.as_str()], |c, env| {
        chains::request_chain(
            c,
            env,
            caller.as_ref(),
            &source_org_id,
            &target_org_id,
            target_exists,
            message,
        )
        .map(drop)
    })
}

/// `approveChain` (`api.ts:921-940`), [`chains::approve_chain`]: the caller answers for the
/// chain's target.
///
/// # Errors
///
/// A rule of [`chains::approve_chain`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn approve_chain(
    ctx: &ReducerContext,
    chain_id: String,
    granted_group_id: String,
    link_node_id: Option<String>,
) -> Result<(), String> {
    let (_, target) = find(ctx, &chain_id, ChainStatus::Pending)?;
    let caller = member_of(ctx, &target);
    let records = load(ctx, &target)?;
    run(ctx, &[target.as_str()], |c, env| {
        chains::approve_chain(
            c,
            env,
            caller.as_ref(),
            &records,
            &chain_id,
            &granted_group_id,
            link_node_id,
        )
        .map(drop)
    })
}

/// `rejectChain` (`api.ts:942-959`), [`chains::reject_chain`].
///
/// # Errors
///
/// A rule of [`chains::reject_chain`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn reject_chain(
    ctx: &ReducerContext,
    chain_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    let (_, target) = find(ctx, &chain_id, ChainStatus::Pending)?;
    let caller = member_of(ctx, &target);
    run(ctx, &[target.as_str()], |c, env| {
        chains::reject_chain(c, env, caller.as_ref(), &chain_id, reason).map(drop)
    })
}

/// `revokeChain` (`api.ts:961-969`), [`chains::revoke_chain`].
///
/// # Errors
///
/// A rule of [`chains::revoke_chain`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn revoke_chain(ctx: &ReducerContext, chain_id: String) -> Result<(), String> {
    let (_, target) = find(ctx, &chain_id, ChainStatus::Active)?;
    let caller = member_of(ctx, &target);
    run(ctx, &[target.as_str()], |c, env| {
        chains::revoke_chain(c, env, caller.as_ref(), &chain_id).map(drop)
    })
}

/// `updateChainPermissions` (`api.ts:978-990`), [`chains::update_chain_permissions`].
///
/// # Errors
///
/// A rule of [`chains::update_chain_permissions`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_chain_permissions(
    ctx: &ReducerContext,
    chain_id: String,
    granted_group_id: String,
    link_node_id: Option<String>,
) -> Result<(), String> {
    let (_, target) = find(ctx, &chain_id, ChainStatus::Active)?;
    let caller = member_of(ctx, &target);
    let records = load(ctx, &target)?;
    run(ctx, &[target.as_str()], |c, env| {
        chains::update_chain_permissions(
            c,
            env,
            caller.as_ref(),
            &records,
            &chain_id,
            &granted_group_id,
            link_node_id,
        )
        .map(drop)
    })
}

/// `cancelChainRequest` (`api.ts:971-976`), [`chains::cancel_chain_request`]: an unknown chain
/// changes nothing.
///
/// # Errors
///
/// A rule of [`chains::cancel_chain_request`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn cancel_chain_request(ctx: &ReducerContext, chain_id: String) -> Result<(), String> {
    let Some(row) = ctx.db.organization_chain().id().find(&chain_id) else {
        return Ok(());
    };
    let caller = member_of(ctx, &row.source_org_id);
    run(ctx, &[row.source_org_id.as_str()], |c, _| {
        chains::cancel_chain_request(c, caller.as_ref(), &chain_id)
    })
}
