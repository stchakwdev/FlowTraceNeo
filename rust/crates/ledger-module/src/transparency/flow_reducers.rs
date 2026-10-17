//! The MOD2 reducers for flow nodes, transactions, their visibility and the public embed config:
//! each runs one [`super::flows`] rule through [`super::reducers::change`], so the trust score and
//! the public rows follow. Record-shaped arguments are JSON in the module's shape (money in minor
//! units).

use spacetimedb::ReducerContext;

use super::flows::{
    self, EmbedConfigPatch, FlowNodePatch, NewFlowNode, NewTransaction, NodeVisibilitySettings,
    TransactionPatch, TransactionVisibilitySettings,
};
use super::reducers::{Books, change, parse};

/// `createNode` (`api.ts:264-274`), [`flows::create_flow_node`]; `node_json` is a
/// [`NewFlowNode`].
///
/// # Errors
///
/// No books, a malformed node, or a rule of [`flows::create_flow_node`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_flow_node(
    ctx: &ReducerContext,
    org_id: String,
    node_json: String,
) -> Result<(), String> {
    let input: NewFlowNode = parse("flow node", &node_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::create_flow_node(t, c, input).map(drop)
    })
}

/// A change to a flow node (module addition), [`flows::update_flow_node`]; `patch_json` is a
/// [`FlowNodePatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`flows::update_flow_node`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_flow_node(
    ctx: &ReducerContext,
    org_id: String,
    node_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: FlowNodePatch = parse("flow node update", &patch_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::update_flow_node(t, c, &node_id, patch).map(drop)
    })
}

/// `createTransaction` (`api.ts:220-238`), [`flows::create_transaction`]; `transaction_json` is a
/// [`NewTransaction`].
///
/// # Errors
///
/// No books, a malformed transaction, or a rule of [`flows::create_transaction`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_transaction(
    ctx: &ReducerContext,
    org_id: String,
    transaction_json: String,
) -> Result<(), String> {
    let input: NewTransaction = parse("transaction", &transaction_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::create_transaction(t, c, input).map(drop)
    })
}

/// `updateTransaction` (`api.ts:240-254`), [`flows::update_transaction`]; `patch_json` is a
/// [`TransactionPatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`flows::update_transaction`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_transaction(
    ctx: &ReducerContext,
    org_id: String,
    transaction_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: TransactionPatch = parse("transaction update", &patch_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::update_transaction(t, c, &transaction_id, patch).map(drop)
    })
}

/// Sets the journal entries that record a transaction (module addition),
/// [`flows::link_transaction_entries`].
///
/// # Errors
///
/// No books, or a rule of [`flows::link_transaction_entries`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn link_transaction_entries(
    ctx: &ReducerContext,
    org_id: String,
    transaction_id: String,
    entry_ids: Vec<String>,
) -> Result<(), String> {
    let books = Books {
        ctx,
        org_id: &org_id,
    };
    change(ctx, &org_id, |t, c, _| {
        flows::link_transaction_entries(t, c, &transaction_id, entry_ids, &books).map(drop)
    })
}

/// `setNodeVisibility` (`api.ts:708-736`), [`flows::set_node_visibility`]; `settings_json` is a
/// [`NodeVisibilitySettings`].
///
/// # Errors
///
/// No books, malformed settings, or a rule of [`flows::set_node_visibility`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn set_node_visibility(
    ctx: &ReducerContext,
    org_id: String,
    node_id: String,
    group_id: String,
    settings_json: String,
) -> Result<(), String> {
    let settings: NodeVisibilitySettings = parse("node visibility", &settings_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::set_node_visibility(t, c, &node_id, &group_id, settings).map(drop)
    })
}

/// `removeNodeVisibility` (`api.ts:738-743`), [`flows::remove_node_visibility`].
///
/// # Errors
///
/// No books, or a rule of [`flows::remove_node_visibility`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn remove_node_visibility(
    ctx: &ReducerContext,
    org_id: String,
    node_id: String,
    group_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        flows::remove_node_visibility(t, c, &node_id, &group_id)
    })
}

/// [`flows::set_transaction_visibility`] (module addition); `settings_json` is a
/// [`TransactionVisibilitySettings`].
///
/// # Errors
///
/// No books, malformed settings, or a rule of [`flows::set_transaction_visibility`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn set_transaction_visibility(
    ctx: &ReducerContext,
    org_id: String,
    transaction_id: String,
    group_id: String,
    settings_json: String,
) -> Result<(), String> {
    let settings: TransactionVisibilitySettings = parse("transaction visibility", &settings_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::set_transaction_visibility(t, c, &transaction_id, &group_id, settings).map(drop)
    })
}

/// [`flows::remove_transaction_visibility`] (module addition).
///
/// # Errors
///
/// No books, or a rule of [`flows::remove_transaction_visibility`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn remove_transaction_visibility(
    ctx: &ReducerContext,
    org_id: String,
    transaction_id: String,
    group_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        flows::remove_transaction_visibility(t, c, &transaction_id, &group_id)
    })
}

/// `updatePublicEmbedConfig` (`api.ts:752-770`), [`flows::update_embed_config`]; `patch_json`
/// is an [`EmbedConfigPatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`flows::update_embed_config`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_public_embed_config(
    ctx: &ReducerContext,
    org_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: EmbedConfigPatch = parse("embed settings update", &patch_json)?;
    change(ctx, &org_id, |t, c, _| {
        flows::update_embed_config(t, c, patch).map(drop)
    })
}

/// `regenerateEmbedToken` (`supabase.ts:874-885`), [`flows::regenerate_embed_token`].
///
/// # Errors
///
/// No books, or a rule of [`flows::regenerate_embed_token`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn regenerate_embed_token(ctx: &ReducerContext, org_id: String) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        flows::regenerate_embed_token(t, c).map(drop)
    })
}
