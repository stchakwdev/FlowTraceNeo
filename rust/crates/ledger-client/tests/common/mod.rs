//! Fixtures and the checks `tests/live.rs` runs against the client.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use ledger_client::{ClientError, ClientEvent, ConnectionStatus, SpacetimeClient};
use ledger_core::reports::{Basis, financial_position, trial_balance};
use ledger_core::types::{IsoDate, LedgerState};
use ledger_core::{CommandResult, IssueCode, LedgerCommand, sanitize_command};
use serde_json::{Value, json};

/// `rust/fixtures/<name>.json`.
pub fn fixture(name: &str) -> Value {
    let path = format!("{}/../../fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// `demo-books.json#/state`.
pub fn demo_state() -> LedgerState {
    serde_json::from_value(fixture("demo-books")["state"].clone()).unwrap()
}

/// The demo organisation's id, from its settings.
pub fn demo_org_id() -> String {
    demo_state().settings.organization_id.to_string()
}

/// A command from its TS JSON, through the sanitiser as the desktop's forms will build them.
pub fn command(raw: &Value) -> LedgerCommand {
    sanitize_command(raw).unwrap()
}

/// The fixture's demo-books cases of `report-<file>.json`, each checked with `run`.
fn check_report(file: &str, run: impl Fn(&Value) -> Value) -> usize {
    let f = fixture(&format!("report-{file}"));
    let mut checked = 0;
    for case in f["cases"].as_array().unwrap() {
        if case["state"] != "demo-books" {
            continue;
        }
        assert_eq!(
            run(&case["args"]),
            case["result"],
            "{file}({})",
            case["args"]
        );
        checked += 1;
    }
    assert!(checked > 0, "{file}: no demo-books cases");
    checked
}

/// `trial_balance` and `financial_position` over `state` equal every demo-books case of
/// `report-trial-balance.json` and `report-financial-position.json`. Returns the case counts.
pub fn reports_match_the_demo_fixtures(state: &LedgerState) -> (usize, usize) {
    let tb = check_report("trial-balance", |a| {
        let as_of: Option<IsoDate> = serde_json::from_value(a[0].clone()).unwrap();
        let basis: Basis = serde_json::from_value(a[1].clone()).unwrap_or(Basis::Accrual);
        serde_json::to_value(trial_balance(state, as_of.as_ref(), basis)).unwrap()
    });
    let fp = check_report("financial-position", |a| {
        let as_of: IsoDate = serde_json::from_value(a[0].clone()).unwrap();
        let basis: Basis = serde_json::from_value(a[1].clone()).unwrap_or(Basis::Accrual);
        serde_json::to_value(financial_position(state, &as_of, basis)).unwrap()
    });
    (tb, fp)
}

/// Waits on `rx` until `pick` accepts an event, failing after `within`.
pub fn wait_for<T>(
    rx: &async_channel::Receiver<ClientEvent>,
    within: Duration,
    mut pick: impl FnMut(&ClientEvent) -> Option<T>,
) -> T {
    let deadline = Instant::now() + within;
    loop {
        match rx.try_recv() {
            Ok(event) => {
                if let Some(t) = pick(&event) {
                    return t;
                }
            }
            Err(async_channel::TryRecvError::Empty) => {
                assert!(Instant::now() < deadline, "no matching event in {within:?}");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(async_channel::TryRecvError::Closed) => panic!("the event channel closed"),
        }
    }
}

/// Every event already queued on `rx`.
pub fn queued(rx: &async_channel::Receiver<ClientEvent>) -> Vec<ClientEvent> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// A vendor the demo books do not have.
pub fn new_vendor() -> LedgerCommand {
    command(&json!({
        "type": "vendor.add",
        "input": {"name": "Contract Test Supplies", "node_id": null, "payment_details": "Paybill 111111"}
    }))
}

/// An entry whose debits and credits differ, which the engine refuses as `UNBALANCED`.
pub fn unbalanced_entry() -> LedgerCommand {
    command(&json!({
        "type": "entry.post",
        "input": {
            "date": "2026-02-04",
            "memo": "Contract test: does not balance",
            "lines": [
                {"account_id": "acc-7100", "debit": 10_000, "fund_id": "fund-unrestricted"},
                {"account_id": "acc-1110", "credit": 5_000, "fund_id": "fund-unrestricted"}
            ]
        }
    }))
}

/// What the contract measured, for the transcript.
#[derive(Debug)]
pub struct ContractRun {
    /// The vendor id the accepted command returned.
    pub vendor_id: String,
    /// The events a subscriber saw for the accepted command.
    pub events: Vec<ClientEvent>,
    /// The refusal's issue codes.
    pub refused: Vec<IssueCode>,
}

/// What the client must do on books equal to the demo books:
///
/// 1. the subscriber's first event is the current status, which is `Ready`;
/// 2. the organisation is listed and its books equal the demo books, and the reports over them
///    equal the fixtures;
/// 3. an accepted command resolves with its result after the change is readable, and a
///    subscriber gets one `Changed` naming the organisation;
/// 4. a refused command is a [`ClientError::Ledger`] with the engine's issues and changes
///    nothing.
pub fn client_contract(backend: &SpacetimeClient, org_id: &str) -> ContractRun {
    let rx = backend.subscribe();
    let first = wait_for(&rx, Duration::from_secs(5), |e| Some(e.clone()));
    assert!(
        matches!(first, ClientEvent::Status(ConnectionStatus::Ready { .. })),
        "first event {first:?}"
    );

    let demo = demo_state();
    assert!(
        backend.organisations().iter().any(|o| o.org_id == org_id),
        "{org_id} is not listed"
    );
    let before = backend.state(org_id).unwrap();
    assert_eq!(before, demo, "the books equal demo-books.json#/state");
    reports_match_the_demo_fixtures(&before);

    let result = backend.submit(org_id, &new_vendor()).unwrap();
    let CommandResult::Id { id: vendor_id } = result else {
        panic!("vendor.add returned {result:?}");
    };
    let after = backend.state(org_id).unwrap();
    assert_eq!(after.vendors.len(), before.vendors.len() + 1);
    assert!(
        after
            .vendors
            .iter()
            .any(|v| v.id.as_str() == vendor_id && v.name == "Contract Test Supplies")
    );
    let events = queued(&rx);
    let changed: Vec<&BTreeSet<String>> = events
        .iter()
        .filter_map(|e| match e {
            ClientEvent::Changed { orgs } => Some(orgs),
            _ => None,
        })
        .collect();
    assert_eq!(
        changed,
        [&BTreeSet::from([org_id.to_owned()])],
        "events {events:?}"
    );

    let refused = backend.submit(org_id, &unbalanced_entry());
    let Err(ClientError::Ledger(e)) = refused else {
        panic!("the unbalanced entry came back {refused:?}");
    };
    assert_eq!(e.code(), IssueCode::Unbalanced, "{e}");
    assert_eq!(
        backend.state(org_id).unwrap(),
        after,
        "a refusal changes nothing"
    );

    ContractRun {
        vendor_id,
        events,
        refused: e.issues().iter().map(|i| i.code).collect(),
    }
}
