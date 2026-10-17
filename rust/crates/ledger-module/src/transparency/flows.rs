//! Flow nodes, transactions, their visibility settings and the public embed config (MOD2): the
//! records (`FlowNode`, `Transaction`, `NodeVisibility`, `TransactionVisibility`,
//! `PublicEmbedConfig` of `flowtrace/src/lib/supabase.ts`), the reducers' inputs, the rules of each
//! write, and the reads whose rules clients share (flow data, organisation stats, campaign report).
//!
//! As in [`super::plan`]: behaviour is the demo API's (`flowtrace/src/lib/demo/api.ts`), who may
//! write is the web's row-level security (`flowtrace/supabase/migrations/002`, `006`), money is
//! integer minor units, and every function checks everything before it changes anything.
//! Transactions gain `journal_entry_ids`: the journal entries that record them in the books
//! (`flowtrace/src/lib/ledger/bridge.ts:1-4`, an entry with `source_type` `transaction`).

use ledger_core::money::{MAX_SAFE_MINOR, js_round};
use ledger_core::types::{IsoDate, Minor, Nullish};
use serde::{Deserialize, Serialize};

use super::OrgTransparency;
use super::plan::{Ctx, group_in_org, not_found, set, set_nullish};
use super::types::{
    Campaign, CampaignStat, CampaignStatus, Metadata, VerificationStatus as ProofStatus,
};

// ---------------------------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------------------------

/// `FlowNode.type` (`supabase.ts:55-61`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Gives money.
    Donor,
    /// A department.
    Department,
    /// Is paid for goods or services.
    Vendor,
    /// Receives help.
    Beneficiary,
    /// A bank or cash account.
    Account,
    /// A project.
    Project,
}

/// A node of the flow map (`FlowNode`, `supabase.ts:52-66`); `balance` in minor units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowNode {
    /// Node id; journal lines name it as `flow_node_id`.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Name.
    pub name: String,
    /// Kind of node.
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Balance shown on the map, minor units; set by hand, not kept by transactions (as in the
    /// demo).
    pub balance: Minor,
    /// Anything else.
    pub metadata: Metadata,
    /// When it was made.
    pub created_at: String,
}

/// `Transaction.verification_status` (`supabase.ts:81`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    /// Waiting for proof.
    Pending,
    /// Proven.
    Verified,
    /// Questioned.
    Flagged,
    /// Refused.
    Rejected,
}

/// A movement of money between two nodes (`Transaction`, `supabase.ts:68-87`); amounts in minor
/// units, plus the journal entries that record it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction id; its journal entries carry it as `source_id`.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Where the money came from.
    pub from_node_id: Option<String>,
    /// Where it went.
    pub to_node_id: Option<String>,
    /// Amount, minor units of `currency`.
    pub amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// Rate to the base currency.
    pub exchange_rate: Option<f64>,
    /// Amount in US dollar cents.
    pub amount_usd: Option<Minor>,
    /// What it was.
    pub description: String,
    /// Category.
    pub category: Option<String>,
    /// Reference.
    pub reference: Option<String>,
    /// Notes.
    pub notes: Option<String>,
    /// Proof state.
    pub verification_status: TransactionStatus,
    /// Proofs attached; follows the proofs, never set directly.
    pub verification_count: u64,
    /// When the money moved, `YYYY-MM-DD`.
    pub transaction_date: String,
    /// Who recorded it: the member's user id.
    pub created_by: Option<String>,
    /// When it was recorded.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
    /// The journal entries that record it, in link order; module addition.
    #[serde(default)]
    pub journal_entry_ids: Vec<String>,
}

/// What one access group sees of one node (`NodeVisibility`, `supabase.ts:225-235`); `None`
/// means the group's own setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeVisibility {
    /// Row id.
    pub id: String,
    /// The node.
    pub node_id: String,
    /// The group.
    pub group_id: String,
    /// Sees the node at all.
    pub can_view: bool,
    /// Sees its amounts.
    pub can_view_amounts: Option<bool>,
    /// Sees its proofs.
    pub can_view_proofs: Option<bool>,
    /// Sees who handled its payments.
    pub can_view_executor: Option<bool>,
    /// Name shown to the group instead.
    pub display_name: Option<String>,
    /// When it was made.
    pub created_at: String,
}

/// What one access group sees of one transaction (`TransactionVisibility`, `supabase.ts:237-246`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionVisibility {
    /// Row id.
    pub id: String,
    /// The transaction.
    pub transaction_id: String,
    /// The group.
    pub group_id: String,
    /// Sees the transaction at all.
    pub can_view: bool,
    /// Sees its amount.
    pub can_view_amount: Option<bool>,
    /// Sees its proofs.
    pub can_view_proofs: Option<bool>,
    /// Sees who handled it.
    pub can_view_executor: Option<bool>,
    /// When it was made.
    pub created_at: String,
}

/// `PublicEmbedConfig.display_mode` (`supabase.ts:251`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    /// `simplified`.
    Simplified,
    /// `detailed`.
    Detailed,
    /// `custom`.
    Custom,
}

/// `PublicEmbedConfig.amount_format` (`supabase.ts:253`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountFormat {
    /// `exact`.
    Exact,
    /// `ranges`.
    Ranges,
    /// `percentages`.
    Percentages,
    /// `hidden`.
    Hidden,
}

/// `PublicEmbedConfig.theme.background` (`supabase.ts:262`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    /// `light`.
    Light,
    /// `dark`.
    Dark,
}

/// `PublicEmbedConfig.theme.border_radius` (`supabase.ts:263`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderRadius {
    /// `none`.
    None,
    /// `sm`.
    Sm,
    /// `md`.
    Md,
    /// `lg`.
    Lg,
    /// `full`.
    Full,
}

/// `PublicEmbedConfig.theme` (`supabase.ts:260-265`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedTheme {
    /// `#rrggbb`.
    pub primary_color: String,
    /// Light or dark.
    pub background: Background,
    /// Corner rounding.
    pub border_radius: BorderRadius,
    /// Shows the logo.
    pub show_logo: bool,
}

/// What the organisation's public embeds show (`PublicEmbedConfig`, `supabase.ts:248-271`), one
/// per organisation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the TS display flags, field for field"
)]
pub struct PublicEmbedConfig {
    /// Row id.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Layout.
    pub display_mode: DisplayMode,
    /// Shows amounts.
    pub show_amounts: bool,
    /// How amounts read.
    pub amount_format: AmountFormat,
    /// Shows node names.
    pub show_node_names: bool,
    /// Shows the trust score.
    pub show_trust_score: bool,
    /// Shows the transaction count.
    pub show_transaction_count: bool,
    /// Shows when it last changed.
    pub show_last_updated: bool,
    /// Node types shown.
    pub visible_node_types: Vec<String>,
    /// Nodes left out.
    pub hidden_node_ids: Vec<String>,
    /// Styling.
    pub theme: EmbedTheme,
    /// Embedding allowed.
    pub allow_embed: bool,
    /// Sites allowed to embed; empty for any.
    pub allowed_domains: Vec<String>,
    /// The token in embed URLs.
    pub embed_token: String,
    /// When it was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

// ---------------------------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------------------------

/// `createNode`'s input (`api.ts:264-274`) less the organisation; the balance starts at zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewFlowNode {
    /// Name.
    pub name: String,
    /// Kind.
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Anything else.
    #[serde(default)]
    pub metadata: Metadata,
}

/// A change to a flow node; module addition (the demo has no `updateNode`). A field left out is
/// kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowNodePatch {
    /// New name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New kind.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    /// New balance, minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<Minor>,
    /// New metadata, whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// `createTransaction`'s input (`api.ts:220-238`) less the organisation and `created_by`, which is
/// the caller; amounts in minor units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTransaction {
    /// From.
    #[serde(default)]
    pub from_node_id: Option<String>,
    /// To.
    #[serde(default)]
    pub to_node_id: Option<String>,
    /// Amount, minor units.
    pub amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// Rate to the base currency.
    #[serde(default)]
    pub exchange_rate: Option<f64>,
    /// Amount in US dollar cents.
    #[serde(default)]
    pub amount_usd: Option<Minor>,
    /// What it was.
    pub description: String,
    /// Category.
    #[serde(default)]
    pub category: Option<String>,
    /// Reference.
    #[serde(default)]
    pub reference: Option<String>,
    /// Notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Proof state.
    pub verification_status: TransactionStatus,
    /// `YYYY-MM-DD`.
    pub transaction_date: String,
}

/// `updateTransaction`'s update (`api.ts:240-254`); a field left out is kept. The proof count and
/// who recorded it are not in it (the TS type allows both).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionPatch {
    /// From, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub from_node_id: Nullish<String>,
    /// To, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub to_node_id: Nullish<String>,
    /// Amount, minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Minor>,
    /// Currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Rate, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub exchange_rate: Nullish<f64>,
    /// US dollar cents, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub amount_usd: Nullish<Minor>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Category, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub category: Nullish<String>,
    /// Reference, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub reference: Nullish<String>,
    /// Notes, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub notes: Nullish<String>,
    /// Proof state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<TransactionStatus>,
    /// Date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_date: Option<String>,
}

/// `setNodeVisibility`'s settings (`api.ts:708-736`); a field left out is kept (or the default
/// for a new row).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeVisibilitySettings {
    /// Sees the node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_view: Option<bool>,
    /// Sees amounts, or `null` for the group's setting.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_amounts: Nullish<bool>,
    /// Sees proofs, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_proofs: Nullish<bool>,
    /// Sees the executor, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_executor: Nullish<bool>,
    /// Name shown instead, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub display_name: Nullish<String>,
}

/// The settings of a transaction visibility row; module addition modelled on
/// [`NodeVisibilitySettings`] (the web has the table, `005_access_groups.sql:105-119`, and no
/// call).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionVisibilitySettings {
    /// Sees the transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_view: Option<bool>,
    /// Sees the amount, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_amount: Nullish<bool>,
    /// Sees proofs, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_proofs: Nullish<bool>,
    /// Sees the executor, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub can_view_executor: Nullish<bool>,
}

/// `updatePublicEmbedConfig`'s update (`api.ts:752-770`); a field left out is kept, the theme is
/// replaced whole (`Object.assign`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedConfigPatch {
    /// Layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<DisplayMode>,
    /// Shows amounts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_amounts: Option<bool>,
    /// Amount format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_format: Option<AmountFormat>,
    /// Shows node names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_node_names: Option<bool>,
    /// Shows the trust score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_trust_score: Option<bool>,
    /// Shows the transaction count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_transaction_count: Option<bool>,
    /// Shows when it last changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_last_updated: Option<bool>,
    /// Node types shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_node_types: Option<Vec<String>>,
    /// Nodes left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_node_ids: Option<Vec<String>>,
    /// Styling, whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<EmbedTheme>,
    /// Embedding allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_embed: Option<bool>,
    /// Sites allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------------------------

/// What `link_transaction_entries` checks against the books.
pub trait EntryLinks {
    /// The organisation's books have a journal entry with this id.
    fn has_entry(&self, id: &str) -> bool;
}

const MANAGE_VISIBILITY: &str = "change what access groups see";

fn node_in_org(t: &OrgTransparency, id: &str) -> Result<(), String> {
    if t.flow_nodes.iter().any(|n| n.id == id) {
        Ok(())
    } else {
        Err(format!("Flow node {id} not found."))
    }
}

fn nodes_in_org(t: &OrgTransparency, ids: [Option<&String>; 2]) -> Result<(), String> {
    ids.into_iter()
        .flatten()
        .try_for_each(|id| node_in_org(t, id))
}

fn check_amount(amount: Minor) -> Result<(), String> {
    if amount > 0 && amount <= MAX_SAFE_MINOR {
        Ok(())
    } else {
        Err("A transaction amount must be above zero.".to_owned())
    }
}

fn check_date(date: &str) -> Result<(), String> {
    IsoDate::parse(date)
        .map(drop)
        .map_err(|_| format!("Transaction date {date} is not a date (YYYY-MM-DD)."))
}

/// `createNode` (`api.ts:264-274`): any member (policy "Users can create org flow nodes",
/// `002_rls_policies.sql:72-74`); the balance starts at zero. The module refuses a blank name.
///
/// # Errors
///
/// Not a member, or a blank name.
pub fn create_flow_node(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewFlowNode,
) -> Result<FlowNode, String> {
    ctx.member()?;
    if crate::rules::js_trim(&input.name).is_empty() {
        return Err("A flow node needs a name.".to_owned());
    }
    let id = ctx.new_id();
    let row = FlowNode {
        id,
        organization_id: ctx.org_id.to_owned(),
        name: input.name,
        node_type: input.node_type,
        balance: 0,
        metadata: input.metadata,
        created_at: ctx.now(),
    };
    t.flow_nodes.push(row.clone());
    Ok(row)
}

/// A change to a flow node (module addition; policy "Users can update org flow nodes",
/// `002_rls_policies.sql:76-79`): any member, field by field.
///
/// # Errors
///
/// Not a member, no such node, a blank name or a balance beyond safe money.
pub fn update_flow_node(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    patch: FlowNodePatch,
) -> Result<FlowNode, String> {
    ctx.member()?;
    let at = t
        .flow_nodes
        .iter()
        .position(|n| n.id == id)
        .ok_or_else(|| not_found("Flow node"))?;
    if patch
        .name
        .as_deref()
        .is_some_and(|n| crate::rules::js_trim(n).is_empty())
    {
        return Err("A flow node needs a name.".to_owned());
    }
    if patch.balance.is_some_and(|b| b.abs() > MAX_SAFE_MINOR) {
        return Err("A flow node balance must be safe money.".to_owned());
    }
    let n = &mut t.flow_nodes[at];
    set(&mut n.name, patch.name);
    set(&mut n.node_type, patch.node_type);
    set(&mut n.balance, patch.balance);
    set(&mut n.metadata, patch.metadata);
    Ok(n.clone())
}

/// `createTransaction` (`api.ts:220-238`): any member (policy "Users can create org
/// transactions", `002_rls_policies.sql:90-92`); no proofs yet, `created_by` the caller. The
/// module refuses an amount not above zero, a date that is not `YYYY-MM-DD` and a node of
/// another organisation. The demo also posts it to its books through the bridge with its own
/// node-to-account mapping (`postDemoTransaction`, `flowtrace/src/lib/demo/ledger.ts:432-456`);
/// the module has no such mapping, so posting stays a `run_command` and
/// [`link_transaction_entries`] records the link.
///
/// # Errors
///
/// Not a member, an amount not above zero, a malformed date or an unknown node.
pub fn create_transaction(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewTransaction,
) -> Result<Transaction, String> {
    let user_id = ctx.member()?.user_id.clone();
    check_amount(input.amount)?;
    check_date(&input.transaction_date)?;
    nodes_in_org(t, [input.from_node_id.as_ref(), input.to_node_id.as_ref()])?;
    let id = ctx.new_id();
    let now = ctx.now();
    let NewTransaction {
        from_node_id,
        to_node_id,
        amount,
        currency,
        exchange_rate,
        amount_usd,
        description,
        category,
        reference,
        notes,
        verification_status,
        transaction_date,
    } = input;
    let row = Transaction {
        id,
        organization_id: ctx.org_id.to_owned(),
        from_node_id,
        to_node_id,
        amount,
        currency,
        exchange_rate,
        amount_usd,
        description,
        category,
        reference,
        notes,
        verification_status,
        verification_count: 0,
        transaction_date,
        created_by: Some(user_id),
        created_at: now.clone(),
        updated_at: now,
        journal_entry_ids: Vec::new(),
    };
    t.transactions.push(row.clone());
    Ok(row)
}

/// `updateTransaction` (`api.ts:240-254`): any member (policy "Users can update org
/// transactions", `002_rls_policies.sql:94-97`); the update applied field by field and
/// `updated_at` stamped. Same checks as [`create_transaction`] on what changes.
///
/// # Errors
///
/// Not a member, no such transaction, or a changed amount, date or node the checks refuse.
pub fn update_transaction(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    patch: TransactionPatch,
) -> Result<Transaction, String> {
    ctx.member()?;
    let at = t
        .transactions
        .iter()
        .position(|x| x.id == id)
        .ok_or_else(|| not_found("Transaction"))?;
    if let Some(amount) = patch.amount {
        check_amount(amount)?;
    }
    if let Some(date) = &patch.transaction_date {
        check_date(date)?;
    }
    let node = |n: &Nullish<String>| match n {
        Nullish::Value(v) => Some(v.clone()),
        _ => None,
    };
    nodes_in_org(
        t,
        [
            node(&patch.from_node_id).as_ref(),
            node(&patch.to_node_id).as_ref(),
        ],
    )?;
    let now = ctx.now();
    let x = &mut t.transactions[at];
    set_nullish(&mut x.from_node_id, patch.from_node_id);
    set_nullish(&mut x.to_node_id, patch.to_node_id);
    set(&mut x.amount, patch.amount);
    set(&mut x.currency, patch.currency);
    set_nullish(&mut x.exchange_rate, patch.exchange_rate);
    set_nullish(&mut x.amount_usd, patch.amount_usd);
    set(&mut x.description, patch.description);
    set_nullish(&mut x.category, patch.category);
    set_nullish(&mut x.reference, patch.reference);
    set_nullish(&mut x.notes, patch.notes);
    set(&mut x.verification_status, patch.verification_status);
    set(&mut x.transaction_date, patch.transaction_date);
    x.updated_at = now;
    Ok(x.clone())
}

/// Sets which journal entries record a transaction (module addition; the TS bridge ties them by
/// `source_type` `transaction` and `source_id`, `bridge.ts:83-86`): any member, as
/// [`update_transaction`]. Each entry must be in the books and not already recorded against
/// another transaction; repeats are dropped, order kept; an empty list unlinks.
///
/// # Errors
///
/// Not a member, no such transaction, an unknown entry or one linked elsewhere.
pub fn link_transaction_entries(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    entry_ids: Vec<String>,
    books: &dyn EntryLinks,
) -> Result<Transaction, String> {
    ctx.member()?;
    let at = t
        .transactions
        .iter()
        .position(|x| x.id == id)
        .ok_or_else(|| not_found("Transaction"))?;
    let mut ids: Vec<String> = Vec::with_capacity(entry_ids.len());
    for entry in entry_ids {
        if ids.contains(&entry) {
            continue;
        }
        if !books.has_entry(&entry) {
            return Err(format!("Journal entry {entry} not found."));
        }
        if let Some(other) = t
            .transactions
            .iter()
            .find(|x| x.id != id && x.journal_entry_ids.contains(&entry))
        {
            return Err(format!(
                "Journal entry {entry} already records transaction {}.",
                other.id
            ));
        }
        ids.push(entry);
    }
    let now = ctx.now();
    let x = &mut t.transactions[at];
    x.journal_entry_ids = ids;
    x.updated_at = now;
    Ok(x.clone())
}

/// The proof triggers of migration 001 as the demo mirrors them (`insertVerification`,
/// `api.ts:300-306`): one more proof on the transaction, and a pending one becomes verified at
/// two. A transaction not in the organisation is left alone, as in the demo.
pub fn count_proof(t: &mut OrgTransparency, transaction_id: &str) {
    if let Some(x) = t.transactions.iter_mut().find(|x| x.id == transaction_id) {
        x.verification_count += 1;
        if x.verification_count >= 2 && x.verification_status == TransactionStatus::Pending {
            x.verification_status = TransactionStatus::Verified;
        }
    }
}

/// `setNodeVisibility` (`api.ts:708-736`): only an admin or manager (policy "Admins can manage
/// node visibility", `006_access_groups_rls.sql:143-157`). An existing row for the node and group
/// takes the settings field by field; otherwise a new row sees the node with the group's own
/// settings, then the given ones. Node and group must be the organisation's.
///
/// # Errors
///
/// Not an admin or manager, or an unknown node or group.
pub fn set_node_visibility(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    node_id: &str,
    group_id: &str,
    settings: NodeVisibilitySettings,
) -> Result<NodeVisibility, String> {
    ctx.manager(MANAGE_VISIBILITY)?;
    node_in_org(t, node_id)?;
    group_in_org(t, group_id)?;
    let found = t
        .node_visibility
        .iter()
        .position(|v| v.node_id == node_id && v.group_id == group_id);
    let at = if let Some(at) = found {
        at
    } else {
        let id = ctx.new_id();
        t.node_visibility.push(NodeVisibility {
            id,
            node_id: node_id.to_owned(),
            group_id: group_id.to_owned(),
            can_view: true,
            can_view_amounts: None,
            can_view_proofs: None,
            can_view_executor: None,
            display_name: None,
            created_at: ctx.now(),
        });
        t.node_visibility.len() - 1
    };
    let v = &mut t.node_visibility[at];
    set(&mut v.can_view, settings.can_view);
    set_nullish(&mut v.can_view_amounts, settings.can_view_amounts);
    set_nullish(&mut v.can_view_proofs, settings.can_view_proofs);
    set_nullish(&mut v.can_view_executor, settings.can_view_executor);
    set_nullish(&mut v.display_name, settings.display_name);
    Ok(v.clone())
}

/// `removeNodeVisibility` (`api.ts:738-743`): an unknown pair changes nothing. Only an admin or
/// manager.
///
/// # Errors
///
/// Not an admin or manager.
pub fn remove_node_visibility(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    node_id: &str,
    group_id: &str,
) -> Result<(), String> {
    ctx.manager(MANAGE_VISIBILITY)?;
    t.node_visibility
        .retain(|v| !(v.node_id == node_id && v.group_id == group_id));
    Ok(())
}

/// [`set_node_visibility`] for one transaction (module addition; policy "Admins can manage
/// transaction visibility", `006_access_groups_rls.sql:179-193`).
///
/// # Errors
///
/// Not an admin or manager, or an unknown transaction or group.
pub fn set_transaction_visibility(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    transaction_id: &str,
    group_id: &str,
    settings: TransactionVisibilitySettings,
) -> Result<TransactionVisibility, String> {
    ctx.manager(MANAGE_VISIBILITY)?;
    if !t.transactions.iter().any(|x| x.id == transaction_id) {
        return Err(format!("Transaction {transaction_id} not found."));
    }
    group_in_org(t, group_id)?;
    let found = t
        .transaction_visibility
        .iter()
        .position(|v| v.transaction_id == transaction_id && v.group_id == group_id);
    let at = if let Some(at) = found {
        at
    } else {
        let id = ctx.new_id();
        t.transaction_visibility.push(TransactionVisibility {
            id,
            transaction_id: transaction_id.to_owned(),
            group_id: group_id.to_owned(),
            can_view: true,
            can_view_amount: None,
            can_view_proofs: None,
            can_view_executor: None,
            created_at: ctx.now(),
        });
        t.transaction_visibility.len() - 1
    };
    let v = &mut t.transaction_visibility[at];
    set(&mut v.can_view, settings.can_view);
    set_nullish(&mut v.can_view_amount, settings.can_view_amount);
    set_nullish(&mut v.can_view_proofs, settings.can_view_proofs);
    set_nullish(&mut v.can_view_executor, settings.can_view_executor);
    Ok(v.clone())
}

/// [`remove_node_visibility`] for one transaction (module addition).
///
/// # Errors
///
/// Not an admin or manager.
pub fn remove_transaction_visibility(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    transaction_id: &str,
    group_id: &str,
) -> Result<(), String> {
    ctx.manager(MANAGE_VISIBILITY)?;
    t.transaction_visibility
        .retain(|v| !(v.transaction_id == transaction_id && v.group_id == group_id));
    Ok(())
}

/// The token in embed URLs: 32 lowercase hex digits (the column default
/// `encode(gen_random_bytes(16), 'hex')`, `005_access_groups.sql:154`), from a fresh id.
fn embed_token(ctx: &mut Ctx<'_>) -> String {
    ctx.new_id()
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect()
}

/// `updatePublicEmbedConfig` (`api.ts:752-770`): only an admin or manager (policy "Admins can
/// manage embed config", `006_access_groups_rls.sql:216-226`); the update applied field by field
/// and `updated_at` stamped. The demo refuses an organisation with no config; the module makes
/// one on the first update from the column defaults (`005_access_groups.sql:126-160`).
///
/// # Errors
///
/// Not an admin or manager.
pub fn update_embed_config(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    patch: EmbedConfigPatch,
) -> Result<PublicEmbedConfig, String> {
    ctx.manager("change the public embed settings")?;
    let mut c = if let Some(c) = t.embed.take() {
        c
    } else {
        let id = ctx.new_id();
        let now = ctx.now();
        PublicEmbedConfig {
            id,
            organization_id: ctx.org_id.to_owned(),
            display_mode: DisplayMode::Simplified,
            show_amounts: true,
            amount_format: AmountFormat::Ranges,
            show_node_names: true,
            show_trust_score: true,
            show_transaction_count: true,
            show_last_updated: true,
            visible_node_types: ["donor", "department", "project", "beneficiary"]
                .map(str::to_owned)
                .to_vec(),
            hidden_node_ids: Vec::new(),
            theme: EmbedTheme {
                primary_color: "#6366f1".to_owned(),
                background: Background::Light,
                border_radius: BorderRadius::Lg,
                show_logo: true,
            },
            allow_embed: true,
            allowed_domains: Vec::new(),
            embed_token: embed_token(ctx),
            created_at: now.clone(),
            updated_at: now,
        }
    };
    set(&mut c.display_mode, patch.display_mode);
    set(&mut c.show_amounts, patch.show_amounts);
    set(&mut c.amount_format, patch.amount_format);
    set(&mut c.show_node_names, patch.show_node_names);
    set(&mut c.show_trust_score, patch.show_trust_score);
    set(&mut c.show_transaction_count, patch.show_transaction_count);
    set(&mut c.show_last_updated, patch.show_last_updated);
    set(&mut c.visible_node_types, patch.visible_node_types);
    set(&mut c.hidden_node_ids, patch.hidden_node_ids);
    set(&mut c.theme, patch.theme);
    set(&mut c.allow_embed, patch.allow_embed);
    set(&mut c.allowed_domains, patch.allowed_domains);
    c.updated_at = ctx.now();
    t.embed = Some(c.clone());
    Ok(c)
}

/// `regenerateEmbedToken` (`supabase.ts:874-885`; the demo has none): a new token, so old embed
/// URLs stop working. Only an admin or manager.
///
/// # Errors
///
/// Not an admin or manager, or no embed config yet.
pub fn regenerate_embed_token(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
) -> Result<PublicEmbedConfig, String> {
    ctx.manager("change the public embed settings")?;
    if t.embed.is_none() {
        return Err(not_found("Embed settings"));
    }
    let token = embed_token(ctx);
    let now = ctx.now();
    let c = t
        .embed
        .as_mut()
        .ok_or_else(|| not_found("Embed settings"))?;
    c.embed_token = token;
    c.updated_at = now;
    Ok(c.clone())
}

// ---------------------------------------------------------------------------------------------
// Reads shared by the clients
// ---------------------------------------------------------------------------------------------

/// `Pick<FlowNode, 'id' | 'name' | 'type'>` (`nodeRef`, `api.ts:89-92`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRef {
    /// Node id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Kind.
    #[serde(rename = "type")]
    pub node_type: NodeType,
}

/// The node of an id, as a reference.
#[must_use]
pub fn node_ref(t: &OrgTransparency, id: Option<&str>) -> Option<NodeRef> {
    let id = id?;
    t.flow_nodes.iter().find(|n| n.id == id).map(|n| NodeRef {
        id: n.id.clone(),
        name: n.name.clone(),
        node_type: n.node_type,
    })
}

/// `TransactionWithNodes` (`supabase.ts:89-92`, `withNodes`, `api.ts:196-200`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionWithNodes {
    /// The transaction.
    #[serde(flatten)]
    pub transaction: Transaction,
    /// Its source node.
    pub from_node: Option<NodeRef>,
    /// Its destination node.
    pub to_node: Option<NodeRef>,
}

/// A transaction with its node references.
#[must_use]
pub fn with_nodes(t: &OrgTransparency, x: &Transaction) -> TransactionWithNodes {
    TransactionWithNodes {
        transaction: x.clone(),
        from_node: node_ref(t, x.from_node_id.as_deref()),
        to_node: node_ref(t, x.to_node_id.as_deref()),
    }
}

/// `FlowDataNode` (`supabase.ts:935-948`), amounts in minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDataNode {
    /// Node id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Kind.
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Money in.
    pub inflow: Minor,
    /// Money out.
    pub outflow: Minor,
    /// The node's balance.
    pub balance: Minor,
    /// Proofs on its transactions.
    pub verification_count: u64,
    /// Of which verified.
    pub verified_count: u64,
    /// Of which pending.
    pub pending_count: u64,
    /// Of which rejected.
    pub rejected_count: u64,
    /// Their trust points.
    pub trust_points: i64,
}

/// `FlowDataEdge` (`supabase.ts:950-956`), value in minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDataEdge {
    /// From.
    pub source: String,
    /// To.
    pub target: String,
    /// Sum of the transactions.
    pub value: Minor,
    /// How many.
    pub count: u64,
    /// The first transaction's currency.
    pub currency: String,
}

/// `FlowDataResult.summary` (`supabase.ts:961-967`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowSummary {
    /// Nodes.
    pub total_nodes: u64,
    /// Edges.
    pub total_edges: u64,
    /// Sum of the edges.
    pub total_flow: Minor,
    /// Transactions counted.
    pub transaction_count: u64,
    /// Currency of the figures.
    pub currency: String,
}

/// `FlowDataResult` (`supabase.ts:958-968`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowData {
    /// Nodes in first-seen order.
    pub nodes: Vec<FlowDataNode>,
    /// Edges in first-seen order.
    pub edges: Vec<FlowDataEdge>,
    /// Totals.
    pub summary: FlowSummary,
}

fn len_u64<T>(items: &[T]) -> u64 {
    u64::try_from(items.len()).unwrap_or(u64::MAX)
}

/// `getFlowDataAPI` (`api.ts:475-566`): every transaction that is not rejected, within the dates
/// when given (compared as `YYYY-MM-DD`), summed onto its nodes and onto one edge per ordered node
/// pair, with each node's proof figures. `currency` is the organisation's (the demo writes the
/// demo organisation's).
#[must_use]
pub fn flow_data(
    t: &OrgTransparency,
    start: Option<&str>,
    end: Option<&str>,
    currency: &str,
) -> FlowData {
    let day = |d: &str| d.chars().take(10).collect::<String>();
    let start = start.map(day);
    let end = end.map(day);
    let txns: Vec<&Transaction> = t
        .transactions
        .iter()
        .filter(|x| {
            x.verification_status != TransactionStatus::Rejected
                && start
                    .as_deref()
                    .is_none_or(|s| x.transaction_date.as_str() >= s)
                && end
                    .as_deref()
                    .is_none_or(|e| x.transaction_date.as_str() <= e)
        })
        .collect();
    let mut nodes: Vec<FlowDataNode> = Vec::new();
    let mut edges: Vec<FlowDataEdge> = Vec::new();
    let touch = |nodes: &mut Vec<FlowDataNode>, id: Option<&String>| -> Option<usize> {
        let id = id?;
        if let Some(at) = nodes.iter().position(|n| &n.id == id) {
            return Some(at);
        }
        let n = t.flow_nodes.iter().find(|n| &n.id == id)?;
        nodes.push(FlowDataNode {
            id: n.id.clone(),
            name: n.name.clone(),
            node_type: n.node_type,
            inflow: 0,
            outflow: 0,
            balance: n.balance,
            verification_count: 0,
            verified_count: 0,
            pending_count: 0,
            rejected_count: 0,
            trust_points: 0,
        });
        Some(nodes.len() - 1)
    };
    for x in &txns {
        let from = touch(&mut nodes, x.from_node_id.as_ref());
        let to = touch(&mut nodes, x.to_node_id.as_ref());
        if let Some(i) = from {
            nodes[i].outflow += x.amount;
        }
        if let Some(i) = to {
            nodes[i].inflow += x.amount;
        }
        let proofs: Vec<_> = t
            .verifications
            .iter()
            .filter(|v| v.transaction_id.as_deref() == Some(x.id.as_str()))
            .collect();
        let with =
            |s: ProofStatus| len_u64(&proofs.iter().filter(|p| p.status == s).collect::<Vec<_>>());
        for i in [from, to].into_iter().flatten() {
            let n = &mut nodes[i];
            n.verification_count += len_u64(&proofs);
            n.verified_count += with(ProofStatus::Verified);
            n.pending_count += with(ProofStatus::Pending);
            n.rejected_count += with(ProofStatus::Rejected);
            n.trust_points += proofs.iter().map(|p| p.trust_points).sum::<i64>();
        }
        if let (Some(source), Some(target)) = (&x.from_node_id, &x.to_node_id) {
            if let Some(e) = edges
                .iter_mut()
                .find(|e| &e.source == source && &e.target == target)
            {
                e.value += x.amount;
                e.count += 1;
            } else {
                edges.push(FlowDataEdge {
                    source: source.clone(),
                    target: target.clone(),
                    value: x.amount,
                    count: 1,
                    currency: x.currency.clone(),
                });
            }
        }
    }
    let summary = FlowSummary {
        total_nodes: len_u64(&nodes),
        total_edges: len_u64(&edges),
        total_flow: edges.iter().map(|e| e.value).sum(),
        transaction_count: len_u64(&txns),
        currency: currency.to_owned(),
    };
    FlowData {
        nodes,
        edges,
        summary,
    }
}

/// `OrgStats.proofStats` (`supabase.ts:288-296`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStats {
    /// Proofs on the organisation's transactions.
    pub total_proofs: u64,
    /// Verified.
    pub verified_proofs: u64,
    /// Pending.
    pub pending_proofs: u64,
    /// Rejected.
    pub rejected_proofs: u64,
    /// Their trust points.
    pub total_trust_points: i64,
    /// Transactions with a proof.
    pub transactions_with_proofs: u64,
    /// Percent of transactions with a proof, rounded.
    pub proof_coverage_rate: i64,
}

/// `OrgStats` (`supabase.ts:281-297`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgStats {
    /// Transactions.
    pub total: u64,
    /// Verified.
    pub verified: u64,
    /// Pending.
    pub pending: u64,
    /// Flagged.
    pub flagged: u64,
    /// Percent verified, rounded.
    pub verification_rate: i64,
    /// Proof figures.
    pub proof_stats: ProofStats,
}

/// `Math.round((part / whole) * 100)`, or 0 for no whole (`api.ts:137,145`).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "counts are far below 2^52 and a percentage fits i64"
)]
pub fn rate(part: u64, whole: u64) -> i64 {
    if whole == 0 {
        0
    } else {
        js_round((part as f64 / whole as f64) * 100.0) as i64
    }
}

/// `getOrganizationStats` (`api.ts:120-149`): the transactions by proof state and the proofs on
/// them.
#[must_use]
pub fn org_stats(t: &OrgTransparency) -> OrgStats {
    let count = |s: TransactionStatus| {
        len_u64(
            &t.transactions
                .iter()
                .filter(|x| x.verification_status == s)
                .collect::<Vec<_>>(),
        )
    };
    let proofs: Vec<_> = t
        .verifications
        .iter()
        .filter(|v| {
            v.transaction_id
                .as_ref()
                .is_some_and(|id| t.transactions.iter().any(|x| &x.id == id))
        })
        .collect();
    let with =
        |s: ProofStatus| len_u64(&proofs.iter().filter(|p| p.status == s).collect::<Vec<_>>());
    let mut covered: Vec<&str> = proofs
        .iter()
        .filter_map(|p| p.transaction_id.as_deref())
        .collect();
    covered.sort_unstable();
    covered.dedup();
    let total = len_u64(&t.transactions);
    let verified = count(TransactionStatus::Verified);
    let with_proofs = len_u64(&covered);
    OrgStats {
        total,
        verified,
        pending: count(TransactionStatus::Pending),
        flagged: count(TransactionStatus::Flagged),
        verification_rate: rate(verified, total),
        proof_stats: ProofStats {
            total_proofs: len_u64(&proofs),
            verified_proofs: with(ProofStatus::Verified),
            pending_proofs: with(ProofStatus::Pending),
            rejected_proofs: with(ProofStatus::Rejected),
            total_trust_points: proofs.iter().map(|p| p.trust_points).sum(),
            transactions_with_proofs: with_proofs,
            proof_coverage_rate: rate(with_proofs, total),
        },
    }
}

/// `CampaignWithDetails` (`supabase.ts:1429-1432`, `withTarget`, `api.ts:1036-1039`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignWithTarget {
    /// The campaign.
    #[serde(flatten)]
    pub campaign: Campaign,
    /// Its target node.
    pub target_node: Option<NodeRef>,
}

/// `CampaignReport.organization` (`api.ts:1368-1373`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportOrganization {
    /// Organisation id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Trust score.
    pub trust_score: i64,
    /// Checked by FlowTrace.
    pub verified: bool,
}

/// `CampaignReportEarmark` (`api.ts:1354-1364`), amounts in minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Earmark {
    /// Node id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Kind (project or department).
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Money received (transactions in that are not rejected).
    pub received: Minor,
    /// The part of it on verified transactions.
    pub evidenced: Minor,
    /// The campaign's own target node.
    pub is_target: bool,
}

/// `CampaignReport` (`api.ts:1366-1379`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignReport {
    /// The campaign.
    pub campaign: CampaignWithTarget,
    /// Who runs it.
    pub organization: ReportOrganization,
    /// Verified money into the target node, capped at the raised amount.
    pub evidenced_amount: Option<Minor>,
    /// Programme nodes, target first, then most received.
    pub earmarks: Vec<Earmark>,
    /// The six latest programme transactions.
    pub spending: Vec<TransactionWithNodes>,
}

/// `getCampaignReport` (`api.ts:1381-1443`) of one of the organisation's campaigns: `organization`
/// is the caller's view of the organisation (profile name, the trust row's score).
#[must_use]
pub fn campaign_report(
    t: &OrgTransparency,
    campaign_id: &str,
    organization: ReportOrganization,
) -> Option<CampaignReport> {
    let c = t.campaigns.iter().find(|c| c.id == campaign_id)?;
    let txns: Vec<&Transaction> = t
        .transactions
        .iter()
        .filter(|x| x.verification_status != TransactionStatus::Rejected)
        .collect();
    let mut earmarks: Vec<Earmark> = t
        .flow_nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Project | NodeType::Department))
        .map(|n| {
            let inflow = txns
                .iter()
                .filter(|x| x.to_node_id.as_deref() == Some(n.id.as_str()));
            Earmark {
                id: n.id.clone(),
                name: n.name.clone(),
                node_type: n.node_type,
                received: inflow.clone().map(|x| x.amount).sum(),
                evidenced: inflow
                    .filter(|x| x.verification_status == TransactionStatus::Verified)
                    .map(|x| x.amount)
                    .sum(),
                is_target: c.target_node_id.as_deref() == Some(n.id.as_str()),
            }
        })
        .collect();
    earmarks.sort_by(|a, b| {
        b.is_target
            .cmp(&a.is_target)
            .then(b.received.cmp(&a.received))
    });
    let programme = |id: Option<&String>| id.is_some_and(|id| earmarks.iter().any(|e| &e.id == id));
    let mut spending: Vec<&Transaction> = txns
        .iter()
        .copied()
        .filter(|x| programme(x.to_node_id.as_ref()) || programme(x.from_node_id.as_ref()))
        .collect();
    spending.sort_by(|a, b| b.transaction_date.cmp(&a.transaction_date));
    let evidenced_amount = earmarks
        .iter()
        .find(|e| e.is_target)
        .map(|e| e.evidenced.min(c.current_amount));
    Some(CampaignReport {
        campaign: CampaignWithTarget {
            campaign: c.clone(),
            target_node: node_ref(t, c.target_node_id.as_deref()),
        },
        organization,
        evidenced_amount,
        spending: spending
            .into_iter()
            .take(6)
            .map(|x| with_nodes(t, x))
            .collect(),
        earmarks,
    })
}

/// Whether a campaign has a public donate page (`getCampaignBySlug`, `api.ts:1059-1066`: public,
/// active and with a link).
#[must_use]
pub fn is_open_to_public(c: &Campaign) -> bool {
    c.is_public && c.status == CampaignStatus::Active && c.public_slug.is_some()
}

/// The stat row of a campaign.
#[must_use]
pub fn stat_of<'a>(t: &'a OrgTransparency, campaign_id: &str) -> Option<&'a CampaignStat> {
    t.campaign_stats
        .iter()
        .find(|s| s.campaign_id == campaign_id)
}
