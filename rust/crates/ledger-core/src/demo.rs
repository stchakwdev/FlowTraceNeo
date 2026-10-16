//! The demo organisation's books, mirroring `flowtrace/src/lib/demo/ledger.ts`: the bundled
//! transactions run through the bridge (the backfill), with what a real NGO's books would hold
//! around them: grants, budgets, bills, staff claims and bank statements. Everything goes
//! through the engine, so the demo obeys the same rules a live organisation does
//! (`demo/ledger.ts:1-5`).
//!
//! The flow nodes and transactions are the demo dataset's (`flowtrace/src/lib/demo/dataset.ts`,
//! `flowNodes` and `transactions`), reduced to what the bridge reads. All names are fictional.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::bridge::{
    BridgeContext, BridgeNode, BridgeNodeType, BridgeResult, BridgeTransaction, GrantRef,
    VerificationStatus, bridge_transaction,
};
use crate::coa_template::{UNRESTRICTED_FUND_ID, account_id};
use crate::commands::{CommandError, CommandResult, LedgerCommand, apply_command};
use crate::engine::{EntryOptions, Error, Ledger, StatementCounter, StatementRow};
use crate::env::fixed;
use crate::errors::{IssueCode, LedgerError};
use crate::money::{Minor, from_major};
use crate::reconcile::{DEFAULT_WINDOW_DAYS, auto_match};
use crate::setup::{create_org_ledger, system_actor};
use crate::types::{
    AccountId, Actor, ActorId, BankAccountId, DimensionId, FlowNodeId, LedgerState, PeriodId,
    SourceType,
};

/// `DEMO_AS_OF` (`demo/ledger.ts:36`): the demo is a snapshot as of its latest movement.
pub const DEMO_AS_OF: &str = "2026-02-04";

/// `DEMO_ORG_ID` (`demo/dataset.ts:27`).
pub const DEMO_ORG_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

/// `DEMO_USER_ID` (`demo/dataset.ts:28`).
pub const DEMO_USER_ID: &str = "user_dummy";

/// `DEMO_ACTOR` (`demo/ledger.ts:38-44`): the demo user, with every permission.
#[must_use]
pub fn demo_actor() -> Actor {
    actor(DEMO_USER_ID, "Demo User", true)
}

fn actor(id: &str, name: &str, all: bool) -> Actor {
    Actor {
        id: ActorId::from(id),
        name: name.to_owned(),
        can_post: all,
        can_approve: all,
        can_close: all,
    }
}

/// `N(suffix)` (`demo/ledger.ts:32`): a flow node id.
fn n(suffix: &str) -> FlowNodeId {
    FlowNodeId::from(format!("a0eebc99-9c0b-4ef8-bb6d-6bb9bd380{suffix}"))
}

/// `T(suffix)` (`demo/ledger.ts:33`): a transaction id.
fn t(suffix: &str) -> String {
    format!("b0eebc99-9c0b-4ef8-bb6d-6bb9bd380{suffix}")
}

/// `flowNodes` (`demo/dataset.ts:133-156`): suffix, name and type, in order.
const NODES: [(&str, &str, BridgeNodeType); 22] = [
    ("a11", "General Fund", BridgeNodeType::Account),
    ("a13", "Operations Budget", BridgeNodeType::Account),
    ("a14", "Project Fund", BridgeNodeType::Account),
    ("a19", "Payroll", BridgeNodeType::Account),
    ("a12", "Harambee Health Fund", BridgeNodeType::Donor),
    ("a20", "Northlight Foundation", BridgeNodeType::Donor),
    ("a30", "Kisima Water Trust", BridgeNodeType::Donor),
    ("a31", "Pan-African Education Fund", BridgeNodeType::Donor),
    ("a15", "Training Department", BridgeNodeType::Department),
    ("a16", "Field Operations", BridgeNodeType::Department),
    ("a21", "Admin", BridgeNodeType::Department),
    ("a32", "M&E Unit", BridgeNodeType::Department),
    ("a17", "Staples Kenya", BridgeNodeType::Vendor),
    ("a18", "AutoXpress Ltd", BridgeNodeType::Vendor),
    ("a22", "Training Co Ltd", BridgeNodeType::Vendor),
    ("a33", "Safaricom M-Pesa", BridgeNodeType::Vendor),
    ("a34", "Kenya Power", BridgeNodeType::Vendor),
    ("a35", "Clean Water Turkana", BridgeNodeType::Project),
    ("a36", "Maternal Health Kilifi", BridgeNodeType::Project),
    ("a23", "Communities", BridgeNodeType::Beneficiary),
    ("a37", "Turkana Schools", BridgeNodeType::Beneficiary),
    ("a38", "Kilifi Health Centers", BridgeNodeType::Beneficiary),
];

/// The demo flow nodes as the bridge sees them (`demo/ledger.ts:96`).
#[must_use]
pub fn flow_nodes() -> Vec<BridgeNode> {
    NODES
        .iter()
        .map(|(suffix, name, node_type)| BridgeNode {
            id: n(suffix),
            name: (*name).to_owned(),
            node_type: *node_type,
        })
        .collect()
}

/// One row of `transactions` (`demo/dataset.ts:199-448`): suffix, from, to, amount in major
/// units, description, category, reference, status and date. All in KES, no exchange rate.
type Txn = (
    &'static str,
    &'static str,
    Option<&'static str>,
    f64,
    &'static str,
    &'static str,
    Option<&'static str>,
    VerificationStatus,
    &'static str,
);

/// `transactions` (`demo/dataset.ts:199-448`), in dataset order.
const TRANSACTIONS: [Txn; 16] = {
    use VerificationStatus::{Flagged, Pending, Verified};
    [
        (
            "b01",
            "a12",
            Some("a11"),
            8_500_000.0,
            "Harambee Health Fund Q1 2026 Disbursement",
            "Funding",
            Some("HHF-2026-Q1-001"),
            Verified,
            "2026-01-05",
        ),
        (
            "b02",
            "a20",
            Some("a11"),
            3_200_000.0,
            "Northlight Foundation Annual Grant",
            "Funding",
            Some("NLF-2026-001"),
            Verified,
            "2026-01-08",
        ),
        (
            "b03",
            "a30",
            Some("a14"),
            4_000_000.0,
            "Kisima Water Trust - Water & Sanitation Grant",
            "Funding",
            Some("KWT-WS-2026-01"),
            Verified,
            "2026-01-10",
        ),
        (
            "b04",
            "a11",
            Some("a13"),
            2_000_000.0,
            "Operations Budget Allocation - January",
            "Budget",
            Some("INT-OPS-2026-01"),
            Verified,
            "2026-01-12",
        ),
        (
            "b05",
            "a11",
            Some("a16"),
            1_800_000.0,
            "Field Operations Q1 Budget",
            "Budget",
            Some("INT-FIELD-2026-Q1"),
            Verified,
            "2026-01-12",
        ),
        (
            "b06",
            "a11",
            Some("a19"),
            1_450_000.0,
            "Staff Salaries - January 2026",
            "Salaries",
            Some("PAY-2026-01"),
            Verified,
            "2026-01-31",
        ),
        (
            "b07",
            "a13",
            Some("a17"),
            28_500.0,
            "Office Supplies - Nairobi HQ",
            "Operations",
            Some("PO-2026-0042"),
            Verified,
            "2026-01-15",
        ),
        (
            "b08",
            "a13",
            Some("a18"),
            67_000.0,
            "Vehicle Maintenance - 3 Field Vehicles",
            "Transport",
            Some("PO-2026-0051"),
            Verified,
            "2026-01-20",
        ),
        (
            "b09",
            "a13",
            Some("a34"),
            42_000.0,
            "Electricity Bill - January",
            "Utilities",
            Some("UTIL-2026-01"),
            Verified,
            "2026-01-25",
        ),
        (
            "b10",
            "a14",
            Some("a35"),
            2_500_000.0,
            "Clean Water Turkana - Borehole Phase 1",
            "Programs",
            Some("PROJ-WATER-001"),
            Verified,
            "2026-01-18",
        ),
        (
            "b11",
            "a16",
            Some("a23"),
            750_000.0,
            "Community Health Worker Stipends",
            "Programs",
            Some("CHW-2026-01"),
            Verified,
            "2026-01-28",
        ),
        (
            "b12",
            "a15",
            Some("a22"),
            380_000.0,
            "Financial Literacy Workshop - Mombasa",
            "Training",
            Some("TRN-2026-003"),
            Verified,
            "2026-01-22",
        ),
        (
            "b13",
            "a16",
            Some("a38"),
            1_200_000.0,
            "Maternal Health Equipment - Kilifi",
            "Programs",
            Some("PROJ-MH-002"),
            Pending,
            "2026-02-01",
        ),
        (
            "b14",
            "a13",
            Some("a33"),
            15_000.0,
            "M-Pesa Float Top-up",
            "Operations",
            Some("MPESA-2026-02"),
            Pending,
            "2026-02-03",
        ),
        (
            "b15",
            "a31",
            Some("a14"),
            6_000_000.0,
            "Pan-African Education Fund - Education Grant",
            "Funding",
            Some("PAEF-2026-01"),
            Pending,
            "2026-02-04",
        ),
        (
            "b16",
            "a14",
            None,
            500_000.0,
            "Unreconciled Cash Withdrawal",
            "Unclassified",
            None,
            Flagged,
            "2026-01-29",
        ),
    ]
};

/// The demo transactions as the bridge sees them, in dataset order.
#[must_use]
pub fn transactions() -> Vec<BridgeTransaction> {
    TRANSACTIONS
        .iter()
        .map(
            |&(id, from, to, amount, description, category, reference, status, date)| {
                BridgeTransaction {
                    id: t(id),
                    from_node_id: Some(n(from)),
                    to_node_id: to.map(n),
                    amount,
                    currency: "KES".to_owned(),
                    exchange_rate: None,
                    description: description.to_owned(),
                    category: Some(category.to_owned()),
                    reference: reference.map(str::to_owned),
                    verification_status: status,
                    transaction_date: date.to_owned(),
                }
            },
        )
        .collect()
}

/// Funds (`demo/ledger.ts:51-56`).
const FUND_MATERNAL: &str = "fund-maternal-health";
const FUND_WASH: &str = "fund-wash";
const FUND_EDUCATION: &str = "fund-education";

/// Grants (`demo/ledger.ts:57-62`).
const GRANT_HARAMBEE: &str = "grant-harambee-2026";
const GRANT_NORTHLIGHT: &str = "grant-northlight-2026";
const GRANT_KISIMA: &str = "grant-kisima-wash";
const GRANT_PAEF: &str = "grant-paef-education";

/// `cashByNode` (`demo/ledger.ts:65-70`): the GL cash account of each account node.
const CASH_BY_NODE: [(&str, &str); 4] = [
    ("a11", "1110"),
    ("a13", "1120"),
    ("a14", "1130"),
    ("a19", "1140"),
];

/// `grantByDonor` (`demo/ledger.ts:72-77`): income follows the donor's grant.
const GRANT_BY_DONOR: [(&str, &str); 4] = [
    ("a12", GRANT_HARAMBEE),
    ("a20", GRANT_NORTHLIGHT),
    ("a30", GRANT_KISIMA),
    ("a31", GRANT_PAEF),
];

/// `grantBySpend` (`demo/ledger.ts:78-83`): known spend follows its programme.
const GRANT_BY_SPEND: [(&str, &str); 4] = [
    ("b10", GRANT_KISIMA),
    ("b11", GRANT_HARAMBEE),
    ("b13", GRANT_NORTHLIGHT),
    ("b16", GRANT_KISIMA),
];

/// `grantByProject` (`demo/ledger.ts:85-89`): new spend through a project node.
const GRANT_BY_PROJECT: [(&str, &str); 3] = [
    ("a35", GRANT_KISIMA),
    ("a36", GRANT_NORTHLIGHT),
    ("a37", GRANT_PAEF),
];

/// `expenseOverrides` (`demo/ledger.ts:90-95`).
const EXPENSE_OVERRIDES: [(&str, &str); 4] = [
    ("b10", "5100"),
    ("b11", "5400"),
    ("b13", "5500"),
    ("b14", "1150"),
];

/// The value of `key` in a table keyed by a full id (built from the suffix with `id_of`).
fn lookup<'a>(table: &[(&str, &'a str)], key: &str, id_of: fn(&str) -> String) -> Option<&'a str> {
    table.iter().find(|(k, _)| id_of(k) == key).map(|(_, v)| *v)
}

fn node_id(suffix: &str) -> String {
    n(suffix).as_str().to_owned()
}

/// `demoBridgeContext(ledger)` (`demo/ledger.ts:97-116`), reading the books it is given.
pub struct DemoBridgeContext<'a> {
    state: &'a LedgerState,
    nodes: Vec<BridgeNode>,
}

impl<'a> DemoBridgeContext<'a> {
    /// The context over these books.
    #[must_use]
    pub fn new(state: &'a LedgerState) -> Self {
        Self {
            state,
            nodes: flow_nodes(),
        }
    }
}

impl BridgeContext for DemoBridgeContext<'_> {
    fn base_currency(&self) -> &str {
        &self.state.settings.base_currency
    }

    fn node(&self, id: &FlowNodeId) -> Option<BridgeNode> {
        self.nodes.iter().find(|x| &x.id == id).cloned()
    }

    fn cash_account_for_node(&self, id: &FlowNodeId) -> Option<AccountId> {
        lookup(&CASH_BY_NODE, id.as_str(), node_id).map(account_id)
    }

    fn default_cash_account(&self) -> AccountId {
        account_id("1110")
    }

    /// The donor's grant, else the spend's, else the project's (to, then from); only a grant
    /// the books hold (`demo/ledger.ts:104-111`).
    fn grant_for(&self, tx: &BridgeTransaction) -> Option<GrantRef> {
        let from = tx.from_node_id.as_ref().map(FlowNodeId::as_str);
        let to = tx.to_node_id.as_ref().map(FlowNodeId::as_str);
        let g = from
            .and_then(|f| lookup(&GRANT_BY_DONOR, f, node_id))
            .or_else(|| lookup(&GRANT_BY_SPEND, &tx.id, t))
            .or_else(|| to.and_then(|x| lookup(&GRANT_BY_PROJECT, x, node_id)))
            .or_else(|| from.and_then(|x| lookup(&GRANT_BY_PROJECT, x, node_id)))?;
        let grant = self.state.grant(&g.into())?;
        Some(GrantRef {
            grant_id: grant.id.clone(),
            fund_id: grant.fund_id.clone(),
        })
    }

    fn dimension_for_node(&self, id: &FlowNodeId) -> Option<DimensionId> {
        self.state
            .dimensions
            .iter()
            .find(|d| d.node_id.as_ref() == Some(id))
            .map(|d| d.id.clone())
    }

    fn expense_account_for(&self, tx: &BridgeTransaction) -> Option<AccountId> {
        lookup(&EXPENSE_OVERRIDES, &tx.id, t).map(account_id)
    }
}

/// A typed engine input from its TS literal. The literals below are fixed, so a failure is a
/// bug in this module; it surfaces as `INVALID_INPUT` rather than a panic.
fn from_json<T: DeserializeOwned>(v: Value) -> Result<T, Error> {
    serde_json::from_value(v)
        .map_err(|e| LedgerError::fail(IssueCode::InvalidInput, format!("Demo input: {e}.")).into())
}

/// `fromMajor(major)` as the JS number the inputs carry.
fn major(x: f64) -> Result<f64, Error> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a safe integer, as the TS number"
    )]
    Ok(from_major(x)? as f64)
}

/// `run(actor, cmd)` and `idOf` (`demo/ledger.ts:216-217`).
fn run(ledger: &mut Ledger<'_>, actor: &Actor, cmd: Value) -> Result<String, Error> {
    let cmd: LedgerCommand = from_json(cmd)?;
    match apply_command(ledger, actor, cmd) {
        Ok(CommandResult::Id { id }) => Ok(id),
        Ok(_) => Ok(String::new()),
        Err(CommandError::Engine(e)) => Err(e),
    }
}

/// `buildDemoLedger()` (`demo/ledger.ts:126-344`), with the demo env (`seedEnv`,
/// `demo/ledger.ts:118-124`; [`fixed::demo`]): `createOrgLedger` for the demo organisation in
/// KES for 2026, the 2025 year opened, funds, grants, dimensions, vendors and bank accounts,
/// the 2025 opening balances with 2025 closed (locked to November), the backfill of every
/// transaction through the bridge in date order, the FY2026 budget, bills, staff claims, the
/// January statement of the main account auto-matched with its bank fee posted, and an
/// unmatched M-Pesa statement. Equal to `demo-books.json#/state`.
///
/// # Errors
///
/// None in practice: the demo obeys every rule. An engine refusal is passed on.
#[expect(
    clippy::too_many_lines,
    reason = "one sequence of engine calls, as buildDemoLedger (demo/ledger.ts:126-344)"
)]
pub fn build_demo_ledger() -> Result<LedgerState, Error> {
    let mut env = fixed::demo();
    let mut state = create_org_ledger(DEMO_ORG_ID, "KES", 2026, &mut env)?;
    let mut ledger = Ledger::new(&mut state, &mut env);
    let l = &mut ledger;
    let sys = system_actor();
    let me = demo_actor();
    l.ensure_fiscal_year(2025)?;

    // Funds, donors and grants (demo/ledger.ts:131-153).
    for (id, name) in [
        (FUND_MATERNAL, "Maternal health (restricted)"),
        (FUND_WASH, "Water and sanitation (restricted)"),
        (FUND_EDUCATION, "Education (restricted)"),
    ] {
        let input = json!({ "id": id, "name": name, "restriction": "temporarily_restricted" });
        l.add_fund(&sys, from_json(input)?)?;
    }
    let nodes = flow_nodes();
    let name_of = |suffix: &str| {
        nodes
            .iter()
            .find(|x| x.id == n(suffix))
            .map(|x| x.name.clone())
            .unwrap_or_default()
    };
    for (id, name, donor, fund, award, start, end, terms) in [
        (
            GRANT_HARAMBEE,
            "Harambee core support 2026",
            "a12",
            UNRESTRICTED_FUND_ID,
            34_000_000.0,
            "2026-01-01",
            "2026-12-31",
            "Quarterly disbursements of KES 8.5M for community health programmes. Unrestricted within the programme.",
        ),
        (
            GRANT_NORTHLIGHT,
            "Northlight maternal health 2026",
            "a20",
            FUND_MATERNAL,
            3_200_000.0,
            "2026-01-01",
            "2026-12-31",
            "Restricted to maternal health initiatives in Kilifi.",
        ),
        (
            GRANT_KISIMA,
            "Kisima WASH: Turkana boreholes",
            "a30",
            FUND_WASH,
            4_000_000.0,
            "2026-01-01",
            "2027-06-30",
            "Restricted to the Clean Water Turkana project.",
        ),
        (
            GRANT_PAEF,
            "PAEF education grant",
            "a31",
            FUND_EDUCATION,
            6_000_000.0,
            "2026-02-01",
            "2027-01-31",
            "Restricted to the Turkana Schools programme.",
        ),
    ] {
        let input = json!({
            "id": id, "name": name, "donor_node_id": n(donor), "donor_name": name_of(donor),
            "fund_id": fund, "award": major(award)?, "currency": "KES",
            "start_date": start, "end_date": end, "terms": terms,
        });
        l.add_grant(&sys, from_json(input)?)?;
    }

    // Dimensions: departments, projects and donors from the flow nodes (demo/ledger.ts:155-165).
    for node in &nodes {
        let kind = match node.node_type {
            BridgeNodeType::Department => "department",
            BridgeNodeType::Project => "project",
            BridgeNodeType::Donor => "donor",
            _ => continue,
        };
        let input = json!({
            "id": format!("dim-{}", suffix3(node.id.as_str())), "kind": kind,
            "name": node.name, "node_id": node.id,
        });
        l.add_dimension(&sys, from_json(input)?)?;
    }
    for (id, name) in [
        ("dim-loc-turkana", "Turkana"),
        ("dim-loc-kilifi", "Kilifi"),
        ("dim-loc-nairobi", "Nairobi HQ"),
    ] {
        let input = json!({ "id": id, "kind": "location", "name": name, "node_id": null });
        l.add_dimension(&sys, from_json(input)?)?;
    }

    // Vendors from vendor nodes (demo/ledger.ts:167-172).
    for node in nodes
        .iter()
        .filter(|x| x.node_type == BridgeNodeType::Vendor)
    {
        let input = json!({
            "id": format!("vendor-{}", suffix3(node.id.as_str())), "name": node.name,
            "node_id": node.id, "payment_details": null,
        });
        l.add_vendor(&sys, from_json(input)?)?;
    }

    // Bank and mobile-money accounts (demo/ledger.ts:174-178).
    for (id, name, kind, provider, code, masked) in [
        (
            "bank-main",
            "Main operating account",
            "bank",
            "Equity Bank",
            "1110",
            "\u{2022}\u{2022}\u{2022}\u{2022} 4821",
        ),
        (
            "bank-ops",
            "Operations account",
            "bank",
            "Equity Bank",
            "1120",
            "\u{2022}\u{2022}\u{2022}\u{2022} 4839",
        ),
        (
            "bank-projects",
            "Restricted projects account",
            "bank",
            "KCB",
            "1130",
            "\u{2022}\u{2022}\u{2022}\u{2022} 0917",
        ),
        (
            "mpesa-till",
            "M-Pesa till",
            "mobile_money",
            "Safaricom M-Pesa",
            "1150",
            "Till 5\u{2022}\u{2022}\u{2022}12",
        ),
    ] {
        let input = json!({
            "id": id, "name": name, "kind": kind, "provider": provider,
            "account_id": account_id(code), "currency": "KES", "number_masked": masked,
        });
        l.add_bank_account(&sys, from_json(input)?)?;
    }

    // Opening balances, then close and lock 2025 (demo/ledger.ts:180-196).
    let opening = |code: &str, debit: bool, amount: f64| -> Result<Value, Error> {
        let side = if debit { "debit" } else { "credit" };
        Ok(
            json!({ "account_id": account_id(code), side: major(amount)?, "fund_id": UNRESTRICTED_FUND_ID }),
        )
    };
    let input = json!({
        "date": "2025-12-31",
        "memo": "Opening balances carried forward from the 2025 books",
        "source_type": "opening",
        "lines": [
            opening("1110", true, 3_000_000.0)?,
            opening("1120", true, 400_000.0)?,
            opening("1150", true, 60_000.0)?,
            opening("1510", true, 2_400_000.0)?,
            opening("2200", false, 85_000.0)?,
            opening("3100", false, 5_775_000.0)?,
        ],
    });
    l.post(&sys, &from_json(input)?, EntryOptions::default())?;
    let periods_2025 = |l: &Ledger<'_>, before: Option<&str>| -> Vec<PeriodId> {
        l.state()
            .periods
            .iter()
            .filter(|p| p.fiscal_year == 2025 && before.is_none_or(|b| p.end_date.as_str() < b))
            .map(|p| p.id.clone())
            .collect()
    };
    for p in periods_2025(l, None) {
        l.close_period(&sys, &p)?;
    }
    for p in periods_2025(l, Some("2025-12-01")) {
        l.lock_period(&sys, &p)?;
    }

    // Backfill: every bundled transaction through the bridge, by date (demo/ledger.ts:198-209).
    // A stable sort, as Array.prototype.sort; localeCompare orders ISO dates as bytes.
    let mut txns = transactions();
    txns.sort_by(|a, b| a.transaction_date.cmp(&b.transaction_date));
    let mut allocations: Vec<(FlowNodeId, f64)> = Vec::new();
    for tx in &txns {
        let result = bridge_transaction(tx, &DemoBridgeContext::new(l.state()))?;
        match result {
            // The flagged withdrawal is unexplained, so it sits in suspense and is not spend.
            BridgeResult::Entry { input } => {
                l.post(&sys, &input, EntryOptions::default())?;
            }
            BridgeResult::Allocation {
                dimension_node_id,
                amount,
            } => allocations.push((dimension_node_id, amount)),
            BridgeResult::Skip { .. } => {}
        }
    }

    // Budget FY2026 (demo/ledger.ts:211-243).
    let input = json!({ "id": "budget-2026", "name": "FY2026 budget", "fiscal_year": 2026, "status": "approved" });
    let budget = l.add_budget(&sys, from_json(input)?)?.id;
    let line = |l: &mut Ledger<'_>,
                account: Option<&str>,
                amount: f64,
                fund: Option<&str>,
                grant: Option<&str>,
                dim: Option<&str>|
     -> Result<(), Error> {
        let input = json!({
            "budget_id": budget, "account_id": account.map(account_id), "fund_id": fund,
            "grant_id": grant, "dimension_id": dim, "period_id": null, "amount": major(amount)?,
        });
        l.set_budget_line(&sys, from_json(input)?)?;
        Ok(())
    };
    // Grant budgets, from the award letters.
    for (code, amount, grant) in [
        ("5100", 3_400_000.0, GRANT_KISIMA),
        ("5700", 300_000.0, GRANT_KISIMA),
        ("7400", 50_000.0, GRANT_KISIMA),
        ("5500", 2_400_000.0, GRANT_NORTHLIGHT),
        ("5600", 800_000.0, GRANT_NORTHLIGHT),
        ("5300", 5_200_000.0, GRANT_PAEF),
        ("5600", 800_000.0, GRANT_PAEF),
        ("5400", 9_000_000.0, GRANT_HARAMBEE),
    ] {
        line(l, Some(code), amount, None, Some(grant), None)?;
    }
    // Running costs, unrestricted.
    for (code, amount) in [
        ("6100", 17_400_000.0),
        ("7100", 300_000.0),
        ("7200", 600_000.0),
        ("7300", 480_000.0),
        ("7400", 180_000.0),
        ("5600", 1_200_000.0),
    ] {
        line(
            l,
            Some(code),
            amount,
            Some(UNRESTRICTED_FUND_ID),
            None,
            None,
        )?;
    }
    // Department allocations from the budget transfers.
    for (node, amount) in &allocations {
        let dim = format!("dim-{}", suffix3(node.as_str()));
        line(l, None, *amount, None, None, Some(&dim))?;
    }

    // Accounts payable (demo/ledger.ts:245-293).
    let coded = |description: &str,
                 code: &str,
                 amount: f64,
                 fund: &str,
                 grant: Option<&str>,
                 dims: &[&str]|
     -> Result<Value, Error> {
        Ok(json!({
            "description": description, "account_id": account_id(code), "amount": major(amount)?,
            "fund_id": fund, "grant_id": grant, "dimension_ids": dims,
        }))
    };
    let bill = |vendor: &str, number: &str, bill_date: &str, due_date: &str, lines: Vec<Value>| {
        json!({ "type": "bill.create", "input": {
            "vendor_id": format!("vendor-{vendor}"), "number": number, "bill_date": bill_date,
            "due_date": due_date, "currency": "KES", "fx_rate": 1, "proof_ids": [], "lines": lines,
        }})
    };
    let power = run(
        l,
        &sys,
        bill(
            "a34",
            "KPLC-0226-77140",
            "2026-02-02",
            "2026-02-16",
            vec![coded(
                "Electricity, January usage, Nairobi HQ",
                "7300",
                39_800.0,
                UNRESTRICTED_FUND_ID,
                None,
                &["dim-loc-nairobi"],
            )?],
        ),
    )?;
    run(l, &me, json!({ "type": "bill.approve", "billId": power }))?;
    run(l, &me, json!({ "type": "bill.post", "billId": power }))?;

    let staples = run(
        l,
        &sys,
        bill(
            "a17",
            "SK-INV-30418",
            "2026-02-03",
            "2026-03-05",
            vec![
                coded(
                    "Exercise books and stationery, Turkana Schools",
                    "5300",
                    186_000.0,
                    FUND_EDUCATION,
                    Some(GRANT_PAEF),
                    &["dim-loc-turkana"],
                )?,
                coded(
                    "Printer toner, HQ",
                    "7100",
                    14_500.0,
                    UNRESTRICTED_FUND_ID,
                    None,
                    &["dim-loc-nairobi"],
                )?,
            ],
        ),
    )?;
    run(l, &me, json!({ "type": "bill.approve", "billId": staples }))?;

    let autox = run(
        l,
        &sys,
        bill(
            "a18",
            "AX-2026-0112",
            "2026-01-26",
            "2026-02-09",
            vec![coded(
                "Tyres, field vehicle KDA 412M",
                "7200",
                48_000.0,
                UNRESTRICTED_FUND_ID,
                None,
                &["dim-a16"],
            )?],
        ),
    )?;
    run(l, &me, json!({ "type": "bill.approve", "billId": autox }))?;
    run(l, &me, json!({ "type": "bill.post", "billId": autox }))?;
    run(
        l,
        &me,
        json!({
            "type": "bill.pay", "billId": autox, "date": "2026-02-02", "cashAccountId": account_id("1120"),
            "amount": major(48_000.0)?, "reference": "EFT 88213",
        }),
    )?;

    // Staff expense claims (demo/ledger.ts:295-331).
    let me_officer = actor("staff-kiprop", "Kiprop Mutai", false);
    let field_officer = actor("staff-achieng", "Achieng Odhiambo", false);
    let claim = |date: &str, purpose: &str, receipts: &[&str], offline: bool, lines: Vec<Value>| {
        json!({ "type": "claim.create", "input": {
            "claim_date": date, "currency": "KES", "fx_rate": 1, "purpose": purpose,
            "receipt_names": receipts, "captured_offline": offline, "submit": true, "lines": lines,
        }})
    };
    let reimbursed = run(
        l,
        &me_officer,
        claim(
            "2026-01-21",
            "Baseline survey, Kilifi health centres",
            &["matatu-receipts.jpg", "lodging-kilifi.pdf"],
            false,
            vec![
                coded(
                    "Transport, Mombasa to Kilifi and back",
                    "5700",
                    6_400.0,
                    FUND_MATERNAL,
                    Some(GRANT_NORTHLIGHT),
                    &["dim-a32", "dim-loc-kilifi"],
                )?,
                coded(
                    "Lodging, 2 nights",
                    "5700",
                    5_900.0,
                    FUND_MATERNAL,
                    Some(GRANT_NORTHLIGHT),
                    &["dim-a32", "dim-loc-kilifi"],
                )?,
            ],
        ),
    )?;
    run(
        l,
        &me,
        json!({ "type": "claim.approve", "claimId": reimbursed, "note": "Receipts match the survey plan." }),
    )?;
    run(
        l,
        &me,
        json!({
            "type": "claim.reimburse", "claimId": reimbursed, "date": "2026-01-23",
            "cashAccountId": account_id("1150"), "reference": "RAN4K2PQ7Z",
        }),
    )?;
    run(
        l,
        &field_officer,
        claim(
            "2026-02-03",
            "Borehole site inspection, Lodwar",
            &["fuel-lodwar.jpg", "meals.jpg"],
            true,
            vec![
                coded(
                    "Fuel, motorbike hire",
                    "5700",
                    14_200.0,
                    FUND_WASH,
                    Some(GRANT_KISIMA),
                    &["dim-a35", "dim-loc-turkana"],
                )?,
                coded(
                    "Meals, 2 days",
                    "5700",
                    4_300.0,
                    FUND_WASH,
                    Some(GRANT_KISIMA),
                    &["dim-a35", "dim-loc-turkana"],
                )?,
            ],
        ),
    )?;

    // Bank statement for the main account: January, reconciled (demo/ledger.ts:333-352).
    let main = account_id("1110");
    let mut rows: Vec<StatementRow> = Vec::new();
    for jl in &l.state().lines {
        let Some(e) = l.state().entry(&jl.entry_id) else {
            continue;
        };
        if jl.account_id != main
            || e.source_type == SourceType::Opening
            || e.date.as_str() > "2026-01-31"
        {
            continue;
        }
        let mut parts = e.memo.split(" \u{b7} ");
        let description = parts.next().unwrap_or("").to_uppercase();
        let reference = parts.next().map(str::to_owned);
        // Banks clear a day or two after the books record a movement.
        let shift = i64::from(e.source_type == SourceType::Transaction);
        rows.push(StatementRow {
            date: crate::dates::add_days(&e.date, shift),
            description,
            reference,
            amount: minor_f64(jl.debit - jl.credit),
        });
    }
    rows.push(statement_row(
        "2026-01-31",
        "LEDGER FEE JAN 2026",
        None,
        -major(1_250.0)?,
    )?);
    rows.push(statement_row(
        "2026-01-31",
        "INTEREST CREDIT",
        None,
        major(3_412.0)?,
    )?);
    let bank_main = BankAccountId::from("bank-main");
    l.import_statement(&sys, &bank_main, &rows)?;
    auto_match(l, &sys, &bank_main, DEFAULT_WINDOW_DAYS)?;
    // The bank fee is explained and posted; the interest is left for the demo.
    let fee = l
        .state()
        .statement_lines
        .iter()
        .find(|s| s.description == "LEDGER FEE JAN 2026")
        .map(|s| s.id.clone());
    if let Some(fee) = fee {
        let counter = StatementCounter {
            account_id: Some(account_id("7700")),
            fund_id: UNRESTRICTED_FUND_ID.into(),
            grant_id: crate::types::Nullish::Undefined,
            memo: Some("Equity Bank ledger fee, January".to_owned()),
        };
        l.post_from_statement_line(&sys, &fee, &counter)?;
    }

    // M-Pesa till statement, early February: not yet matched (demo/ledger.ts:354-359).
    let mpesa = [
        statement_row(
            "2026-01-23",
            "Business Payment to KIPROP MUTAI",
            Some("RAN4K2PQ7Z"),
            -major(12_300.0)?,
        )?,
        statement_row(
            "2026-02-01",
            "Customer Transfer Fuliza charge",
            Some("RB17Q0XT3M"),
            -major(410.0)?,
        )?,
        statement_row(
            "2026-02-03",
            "Funds received from Equity Bank",
            Some("RB34NNA81C"),
            major(15_000.0)?,
        )?,
    ];
    l.import_statement(&sys, &BankAccountId::from("mpesa-till"), &mpesa)?;

    Ok(state)
}

/// The last three characters of an id (`n.id.slice(-3)`, `demo/ledger.ts:159`).
fn suffix3(id: &str) -> &str {
    id.get(id.len().saturating_sub(3)..).unwrap_or(id)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a safe integer, as the TS number"
)]
fn minor_f64(x: Minor) -> f64 {
    x as f64
}

fn statement_row(
    date: &str,
    description: &str,
    reference: Option<&str>,
    amount: f64,
) -> Result<StatementRow, Error> {
    from_json(json!({
        "date": date, "description": description, "reference": reference, "amount": amount,
    }))
}
