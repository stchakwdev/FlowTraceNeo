//! The bridge from the original transaction model to the ledger, mirroring
//! `flowtrace/src/lib/ledger/bridge.ts` (`Docs/flowtrace-ledger-plan.md` section 5). A
//! transaction is a business document; posting it creates a journal entry. Its from and to
//! nodes stay on the lines as `flow_node_id`, so every edge of the flow map traces to its
//! journal lines (`bridge.ts:1-4`).

use serde::{Deserialize, Serialize};

use crate::money::MoneyError;
use crate::types::{AccountId, DimensionId, EntryInput, FlowNodeId, FundId, GrantId};

/// `BridgeNode['type']` (`bridge.ts:13`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeNodeType {
    /// A donor.
    Donor,
    /// A department.
    Department,
    /// A vendor.
    Vendor,
    /// A beneficiary.
    Beneficiary,
    /// A cash account.
    Account,
    /// A project.
    Project,
}

/// `BridgeNode` (`bridge.ts:10-14`): a flow node as the bridge sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeNode {
    /// Node id.
    pub id: FlowNodeId,
    /// Display name.
    pub name: String,
    /// What the node is (`type` in TS).
    #[serde(rename = "type")]
    pub node_type: BridgeNodeType,
}

/// `BridgeTransaction['verification_status']` (`bridge.ts:26`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Not yet verified.
    Pending,
    /// Verified.
    Verified,
    /// Flagged for review.
    Flagged,
    /// Rejected; never posted.
    Rejected,
}

/// `BridgeTransaction` (`bridge.ts:16-28`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeTransaction {
    /// Transaction id; the entry's source id.
    pub id: String,
    /// Where the money came from.
    pub from_node_id: Option<FlowNodeId>,
    /// Where it went.
    pub to_node_id: Option<FlowNodeId>,
    /// Amount in major units, as transactions store it.
    #[serde(with = "crate::js_number")]
    pub amount: f64,
    /// Currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(with = "crate::js_number::option")]
    pub exchange_rate: Option<f64>,
    /// Description.
    pub description: String,
    /// Category, for example `Funding` or `Salaries`.
    pub category: Option<String>,
    /// Reference, appended to the memo.
    pub reference: Option<String>,
    /// Verification status.
    pub verification_status: VerificationStatus,
    /// Date, `YYYY-MM-DD`.
    pub transaction_date: String,
}

/// The grant and fund a transaction belongs to (`bridge.ts:38`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantRef {
    /// The grant.
    pub grant_id: GrantId,
    /// Its fund.
    pub fund_id: FundId,
}

/// `BridgeContext` (`bridge.ts:30-42`): what the bridge needs to know about an organisation.
pub trait BridgeContext {
    /// The base currency.
    fn base_currency(&self) -> &str;
    /// The node with this id.
    fn node(&self, id: &FlowNodeId) -> Option<BridgeNode>;
    /// The GL cash account of an `account` node.
    fn cash_account_for_node(&self, id: &FlowNodeId) -> Option<AccountId>;
    /// The cash account that pays for spend made by departments and projects.
    fn default_cash_account(&self) -> AccountId;
    /// The grant (and so the fund) a transaction belongs to, if any.
    fn grant_for(&self, t: &BridgeTransaction) -> Option<GrantRef>;
    /// The dimension value standing for a flow node.
    fn dimension_for_node(&self, id: &FlowNodeId) -> Option<DimensionId>;
    /// An optional per-transaction expense account override.
    fn expense_account_for(&self, t: &BridgeTransaction) -> Option<AccountId> {
        let _ = t;
        None
    }
}

/// `BridgeResult` (`bridge.ts:44-48`), tagged by `kind` as in TS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BridgeResult {
    /// An entry to post.
    Entry {
        /// The entry.
        input: EntryInput,
    },
    /// A budget allocation to a department or project: a budget, not a posting.
    #[serde(rename_all = "camelCase")]
    Allocation {
        /// The department or project node.
        dimension_node_id: FlowNodeId,
        /// The amount in major units, as on the transaction.
        #[serde(with = "crate::js_number")]
        amount: f64,
    },
    /// Not posted, and why.
    Skip {
        /// Why.
        reason: String,
    },
}

/// `expenseAccountForCategory(category, to)` (`bridge.ts:62-67`).
#[must_use]
pub fn expense_account_for_category(
    category: Option<&str>,
    to: Option<&BridgeNode>,
) -> Option<AccountId> {
    let _ = (category, to);
    None
}

/// `bridgeTransaction(t, ctx)` (`bridge.ts:69-131`).
///
/// # Errors
///
/// [`MoneyError`] when the amount is not a safe number of minor units, as `fromMajor` throws.
pub fn bridge_transaction(
    t: &BridgeTransaction,
    ctx: &dyn BridgeContext,
) -> Result<BridgeResult, MoneyError> {
    let _ = (t, ctx);
    Ok(BridgeResult::Skip {
        reason: String::new(),
    })
}
