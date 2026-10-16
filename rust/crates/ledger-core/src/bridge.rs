//! The bridge from the original transaction model to the ledger, mirroring
//! `flowtrace/src/lib/ledger/bridge.ts` (`Docs/flowtrace-ledger-plan.md` section 5). A
//! transaction is a business document; posting it creates a journal entry. Its from and to
//! nodes stay on the lines as `flow_node_id`, so every edge of the flow map traces to its
//! journal lines (`bridge.ts:1-4`).

use serde::{Deserialize, Serialize};

use crate::coa_template::{UNRESTRICTED_FUND_ID, account_id};
use crate::money::{MoneyError, from_major};
use crate::types::{
    AccountId, DimensionId, EntryInput, FlowNodeId, FundId, GrantId, LineInput, Nullish, SourceType,
};

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

/// `CATEGORY_ACCOUNT` (`bridge.ts:50-60`): account codes by lower-cased category.
const CATEGORY_ACCOUNT: [(&str, &str); 9] = [
    ("salaries", "6100"),
    ("operations", "7100"),
    ("supplies", "7100"),
    ("transport", "7200"),
    ("utilities", "7300"),
    ("training", "5600"),
    ("equipment", "5500"),
    ("programs", "5500"),
    ("funding", "4100"),
];

/// `expenseAccountForCategory(category, to)` (`bridge.ts:62-67`): the expense account of a
/// category, case-insensitive; programme money to a beneficiary is a stipend or transfer
/// (`5400`). Funding is income, not an expense, so it has none.
#[must_use]
pub fn expense_account_for_category(
    category: Option<&str>,
    to: Option<&BridgeNode>,
) -> Option<AccountId> {
    let key = category.unwrap_or("").to_lowercase();
    if key == "programs" && to.is_some_and(|n| n.node_type == BridgeNodeType::Beneficiary) {
        return Some(account_id("5400"));
    }
    CATEGORY_ACCOUNT
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, code)| *code)
        .filter(|code| *code != "4100")
        .map(account_id)
}

/// `bridgeTransaction(t, ctx)` (`bridge.ts:69-131`): the posting a transaction stands for.
///
/// Rejected transactions, a zero or negative amount, and a foreign currency with no rate are
/// skipped. A budget allocation to a department or project is a budget line, not money moving.
/// Money from a donor is income into a cash account (grant income when it is funding or
/// belongs to a grant, else donations), in the grant's fund. Money between two cash accounts,
/// payroll aside, is a transfer. Anything else is spend from a cash account to the override,
/// the category's account, or suspense when there is no recipient or no account. Memos carry
/// the reference after ` · `; lines carry the flow nodes and their dimensions.
///
/// # Errors
///
/// [`MoneyError`] when the amount is not a safe number of minor units, as `fromMajor` throws.
#[expect(
    clippy::too_many_lines,
    reason = "one case per kind of movement, as bridgeTransaction (bridge.ts:69-131)"
)]
pub fn bridge_transaction(
    t: &BridgeTransaction,
    ctx: &dyn BridgeContext,
) -> Result<BridgeResult, MoneyError> {
    let skip = |reason: String| Ok(BridgeResult::Skip { reason });
    if t.verification_status == VerificationStatus::Rejected {
        return skip("Rejected transactions are not posted.".to_owned());
    }
    let from = t.from_node_id.as_ref().and_then(|id| ctx.node(id));
    let to = t.to_node_id.as_ref().and_then(|id| ctx.node(id));
    let amount = from_major(t.amount)?;
    if amount <= 0 {
        return skip("No amount.".to_owned());
    }
    let currency = &t.currency;
    let fx_rate = if *currency == ctx.base_currency() {
        Some(1.0)
    } else {
        t.exchange_rate
    };
    // `if (!fx_rate)`: no rate, or a rate of 0 or NaN.
    let Some(fx_rate) = fx_rate.filter(|r| *r != 0.0 && !r.is_nan()) else {
        return skip(format!("No exchange rate for {currency}."));
    };
    let category = t.category.as_deref().unwrap_or("").to_lowercase();
    let memo = match &t.reference {
        Some(r) if !r.is_empty() => format!("{} \u{b7} {r}", t.description),
        _ => t.description.clone(),
    };
    let dims = |nodes: &[Option<&BridgeNode>]| -> Vec<DimensionId> {
        nodes
            .iter()
            .flatten()
            .filter_map(|n| ctx.dimension_for_node(&n.id))
            .filter(|d| !d.as_str().is_empty())
            .collect()
    };
    let cash_of = |n: Option<&BridgeNode>| {
        n.filter(|n| n.node_type == BridgeNodeType::Account)
            .and_then(|n| ctx.cash_account_for_node(&n.id))
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a safe integer, as the TS number"
    )]
    let amount = amount as f64;
    let line = |account_id: AccountId, debit: bool, fund_id: &FundId| LineInput {
        account_id,
        debit: debit.then_some(amount),
        credit: (!debit).then_some(amount),
        currency: Some(currency.clone()),
        fx_rate: Some(fx_rate),
        fund_id: fund_id.clone(),
        grant_id: Nullish::Undefined,
        dimension_ids: None,
        flow_node_id: Nullish::Undefined,
        memo: Nullish::Undefined,
    };
    let node_of = |n: Option<&BridgeNode>| Nullish::from(n.map(|n| n.id.clone()));
    let entry = |lines: Vec<LineInput>| {
        Ok(BridgeResult::Entry {
            input: EntryInput {
                date: t.transaction_date.clone(),
                memo: memo.clone(),
                lines,
                source_type: Some(SourceType::Transaction),
                source_id: Nullish::Value(t.id.clone()),
                override_reason: Nullish::Undefined,
            },
        })
    };
    let (from, to) = (from.as_ref(), to.as_ref());

    if category == "budget"
        && let Some(to) = to.filter(|n| {
            matches!(
                n.node_type,
                BridgeNodeType::Department | BridgeNodeType::Project
            )
        })
    {
        return Ok(BridgeResult::Allocation {
            dimension_node_id: to.id.clone(),
            amount: t.amount,
        });
    }

    let g = ctx.grant_for(t);
    let fund_id = g
        .as_ref()
        .map_or_else(|| FundId::from(UNRESTRICTED_FUND_ID), |g| g.fund_id.clone());
    let grant_id = Nullish::from(g.as_ref().map(|g| g.grant_id.clone()));

    if let Some(from) = from.filter(|n| n.node_type == BridgeNodeType::Donor) {
        let cash = cash_of(to).unwrap_or_else(|| ctx.default_cash_account());
        let income = if category == "funding" || g.is_some() {
            account_id("4100")
        } else {
            account_id("4200")
        };
        return entry(vec![
            LineInput {
                grant_id: grant_id.clone(),
                flow_node_id: node_of(to),
                ..line(cash, true, &fund_id)
            },
            LineInput {
                grant_id,
                dimension_ids: Some(dims(&[Some(from)])),
                flow_node_id: Nullish::Value(from.id.clone()),
                ..line(income, false, &fund_id)
            },
        ]);
    }

    let from_cash = cash_of(from);
    let to_cash = cash_of(to);
    // `if (fromCash && toCash && ...)`: an empty account id is false (bridge.ts:111).
    let truthy = |a: &Option<AccountId>| a.as_ref().filter(|a| !a.as_str().is_empty()).cloned();
    if let (Some(from_cash), Some(to_cash), Some(from), Some(to)) =
        (truthy(&from_cash), truthy(&to_cash), from, to)
        && category != "salaries"
    {
        return entry(vec![
            LineInput {
                flow_node_id: Nullish::Value(to.id.clone()),
                ..line(to_cash, true, &fund_id)
            },
            LineInput {
                flow_node_id: Nullish::Value(from.id.clone()),
                ..line(from_cash, false, &fund_id)
            },
        ]);
    }

    let credit = from_cash.unwrap_or_else(|| ctx.default_cash_account());
    let debit = match to {
        None => account_id("1400"),
        // `??`, so an empty override is kept (bridge.ts:126-128).
        Some(to) => ctx
            .expense_account_for(t)
            .or_else(|| expense_account_for_category(t.category.as_deref(), Some(to)))
            .unwrap_or_else(|| account_id("1400")),
    };
    entry(vec![
        LineInput {
            grant_id,
            dimension_ids: Some(dims(&[from, to])),
            flow_node_id: node_of(to),
            ..line(debit, true, &fund_id)
        },
        LineInput {
            flow_node_id: node_of(from),
            ..line(credit, false, &fund_id)
        },
    ])
}
