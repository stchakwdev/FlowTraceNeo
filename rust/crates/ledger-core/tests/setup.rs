//! Task C6 acceptance for `coa-template.ts`, `setup.ts` and `bridge.ts`: every case of
//! `setup.json` and `bridge.json` matches exactly, and `createOrgLedger('org', 'KES', 2026)` with
//! the test env builds the books every test scenario starts from.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeMap;

use common::replay::{assert_json_eq, to_json};
use common::{cases, fixture};
use ledger_core::bridge::{
    BridgeContext, BridgeNode, BridgeTransaction, GrantRef, bridge_transaction,
    expense_account_for_category,
};
use ledger_core::coa_template::{
    UNRESTRICTED_FUND_ID, account_id, default_settings, ngo_chart_of_accounts,
};
use ledger_core::env::fixed;
use ledger_core::setup::{
    ActorUser, MemberPermissions, actor_from_permissions, create_org_ledger, system_actor,
};
use ledger_core::types::{AccountId, DimensionId, FlowNodeId};
use serde_json::Value;

#[test]
fn create_org_ledger_matches() {
    let f = fixture("setup");
    let c = &f["createOrgLedger"];
    assert_eq!(c["args"], serde_json::json!(["org", "KES", 2026]));
    let state = create_org_ledger("org", "KES", 2026, &mut fixed::test()).unwrap();
    assert_json_eq(
        &to_json(&state),
        &c["state"],
        "createOrgLedger('org', 'KES', 2026)",
    );
}

#[test]
fn chart_of_accounts_matches() {
    let f = fixture("setup");
    assert_json_eq(
        &to_json(&ngo_chart_of_accounts()),
        &f["ngoChartOfAccounts"],
        "ngoChartOfAccounts()",
    );
}

#[test]
fn setup_exports_match() {
    let f = fixture("setup");
    for case in cases(&f, "defaultSettings") {
        let org = case["args"][0].as_str().unwrap();
        let currency = case["args"][1].as_str().unwrap_or("KES");
        assert_json_eq(
            &to_json(&default_settings(org, currency)),
            &case["result"],
            &format!("defaultSettings({})", case["args"]),
        );
    }
    for case in cases(&f, "accountId") {
        let got = account_id(case["args"][0].as_str().unwrap());
        assert_eq!(to_json(&got), case["result"]);
    }
    for case in cases(&f, "actorFromPermissions") {
        let user: ActorUser = serde_json::from_value(case["args"][0].clone()).unwrap();
        let perms: MemberPermissions = serde_json::from_value(case["args"][1].clone()).unwrap();
        assert_json_eq(
            &to_json(&actor_from_permissions(&user, &perms)),
            &case["result"],
            &format!("actorFromPermissions({})", case["args"]),
        );
    }
    assert_json_eq(
        &to_json(&system_actor()),
        &f["SYSTEM_ACTOR"],
        "SYSTEM_ACTOR",
    );
    assert_eq!(f["UNRESTRICTED_FUND_ID"], UNRESTRICTED_FUND_ID);
}

/// The demo bridge context as `bridge.json` records it: the node maps as data, `grantFor` and
/// `expenseAccountFor` as evaluated for the case at hand.
struct FixtureContext {
    base_currency: String,
    nodes: Vec<BridgeNode>,
    cash: BTreeMap<FlowNodeId, AccountId>,
    default_cash: AccountId,
    dims: BTreeMap<FlowNodeId, DimensionId>,
    grant: Option<GrantRef>,
    expense: Option<AccountId>,
}

impl FixtureContext {
    fn new(ctx: &Value, case: &Value) -> Self {
        fn from<T: serde::de::DeserializeOwned>(v: &Value) -> T {
            serde_json::from_value(v.clone()).unwrap()
        }
        Self {
            base_currency: from(&ctx["base_currency"]),
            nodes: from(&ctx["nodes"]),
            cash: from(&ctx["cash_account_for_node"]),
            default_cash: from(&ctx["default_cash_account"]),
            dims: from(&ctx["dimension_for_node"]),
            grant: from(&case["grant_for"]),
            expense: from(&case["expense_account_for"]),
        }
    }
}

impl BridgeContext for FixtureContext {
    fn base_currency(&self) -> &str {
        &self.base_currency
    }
    fn node(&self, id: &FlowNodeId) -> Option<BridgeNode> {
        self.nodes.iter().find(|n| &n.id == id).cloned()
    }
    fn cash_account_for_node(&self, id: &FlowNodeId) -> Option<AccountId> {
        self.cash.get(id).cloned()
    }
    fn default_cash_account(&self) -> AccountId {
        self.default_cash.clone()
    }
    fn grant_for(&self, _t: &BridgeTransaction) -> Option<GrantRef> {
        self.grant.clone()
    }
    fn dimension_for_node(&self, id: &FlowNodeId) -> Option<DimensionId> {
        self.dims.get(id).cloned()
    }
    fn expense_account_for(&self, _t: &BridgeTransaction) -> Option<AccountId> {
        self.expense.clone()
    }
}

#[test]
fn bridge_transaction_matches() {
    let f = fixture("bridge");
    for case in cases(&f, "cases") {
        let t: BridgeTransaction = serde_json::from_value(case["transaction"].clone()).unwrap();
        let ctx = FixtureContext::new(&f["context"], case);
        let got = bridge_transaction(&t, &ctx).unwrap();
        assert_json_eq(
            &to_json(&got),
            &case["result"],
            &format!("bridgeTransaction({})", t.id),
        );
    }
}

#[test]
fn expense_account_for_category_matches() {
    let f = fixture("bridge");
    for case in cases(&f, "expense_account_for_category") {
        let category: Option<String> = serde_json::from_value(case["args"][0].clone()).unwrap();
        let to: Option<BridgeNode> = serde_json::from_value(case["args"][1].clone()).unwrap();
        let got = expense_account_for_category(category.as_deref(), to.as_ref());
        assert_eq!(to_json(&got), case["result"], "{}", case["args"]);
    }
}
