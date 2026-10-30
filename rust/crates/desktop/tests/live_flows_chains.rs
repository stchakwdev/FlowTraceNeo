//! DT2's live check: the Flows and Chains screens' own path (the view models of
//! `desktop::flows`, `desktop::chains` and `desktop::general_ledger` through
//! `SpacetimeBackend::write_flow` and `write_chain`) against a real database. Ignored; it runs
//! only by hand with these set:
//!
//! - `DT2_LIVE_DB`: the database, `flowtrace-ledger-dev` (never the production one).
//! - `DT2_LIVE_ORG` and `DT2_LIVE_ORG_B`: two organisations the token's identity is a member of,
//!   able to close periods in both; the second has its default access groups.
//! - `DT2_LIVE_TOKEN`: the token file of that identity (made on first run), an absolute path:
//!   cargo runs the test in the crate's folder. Keep it under the gitignored `rust/out/`.
//! - `DT2_LIVE_HOST`: the server, `https://maincloud.spacetimedb.com` by default.
//!
//! `identity` prints the token's identity for the operator's `add_member`; `flows_and_chains`
//! posts an entry in the first organisation, adds two flow nodes and a movement between them,
//! links the movement to the entry, reads the edge and its ledger lines as the screens do, then
//! requests a chain from the first to the second, approves it for the second with its first
//! access group, reads both sides, and revokes it.
//!
//! ```sh
//! DT2_LIVE_DB=flowtrace-ledger-dev DT2_LIVE_ORG=dt2-20261030 DT2_LIVE_ORG_B=dt2-20261030-b \
//!   DT2_LIVE_TOKEN=$PWD/out/dt2-token \
//!   cargo test -p desktop --test live_flows_chains -- --ignored --nocapture --test-threads 1
//! ```

#![expect(clippy::unwrap_used, reason = "a hand-run check may panic")]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use desktop::backend::{
    BackendEvent, ChainWrite, ConnectSettings, ConnectionStatus, FlowWrite, LedgerBackend,
    SpacetimeBackend, Submission, SubmitResult,
};
use desktop::chains::{ChainAction, ChainsView};
use desktop::flows::{Details, FlowPage, LedgerLink, PeriodId, Selection};
use desktop::general_ledger::GlTable;
use desktop::statements::today_utc;
use ledger_core::coa_template::UNRESTRICTED_FUND_ID;
use ledger_core::types::{AccountType, OrganizationId};
use ledger_core::{CommandResult, sanitize_command};
use ledger_module::transparency::chains::ChainStatus;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

fn connect() -> (SpacetimeBackend, async_channel::Receiver<BackendEvent>) {
    let settings = ConnectSettings {
        host: std::env::var("DT2_LIVE_HOST")
            .unwrap_or_else(|_| "https://maincloud.spacetimedb.com".to_owned()),
        database: env("DT2_LIVE_DB"),
        token_path: env("DT2_LIVE_TOKEN"),
        access_token: None,
    };
    assert_ne!(
        settings.database, "flowtrace-ledger",
        "never the production database"
    );
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.connect(settings).unwrap();
    (backend, rx)
}

/// Drains events until `done` holds, failing after 30 s.
fn until(rx: &async_channel::Receiver<BackendEvent>, what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        while rx.try_recv().is_ok() {}
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Waits for a submission's result off any async runtime.
fn settle(submission: Submission) -> SubmitResult {
    let mut future = pin!(submission.result());
    let mut cx = Context::from_waker(Waker::noop());
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Poll::Ready(result) = future.as_mut().poll(&mut cx) {
            return result;
        }
        assert!(Instant::now() < deadline, "no answer within 30 s");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "live: needs DT2_LIVE_DB and a token file"]
fn identity() {
    let (backend, rx) = connect();
    until(&rx, "the connection", || {
        matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let ConnectionStatus::Connected { identity } = backend.status() else {
        unreachable!("checked above");
    };
    println!("dt2: identity {identity}");
}

#[test]
#[ignore = "live: needs DT2_LIVE_DB, DT2_LIVE_ORG, DT2_LIVE_ORG_B and a member's token file"]
#[expect(clippy::too_many_lines, reason = "one hand-run scenario, step by step")]
fn flows_and_chains() {
    let (mut backend, rx) = connect();
    let org = OrganizationId::from(env("DT2_LIVE_ORG"));
    let org_b = OrganizationId::from(env("DT2_LIVE_ORG_B"));
    until(&rx, "both organisations' books and records", || {
        backend.state(&org).is_some()
            && backend.state(&org_b).is_some()
            && backend.transparency(&org_b).is_some()
            && matches!(backend.status(), ConnectionStatus::Connected { .. })
    });

    // An entry in the first organisation's books, as the Journal screen posts one.
    let books = backend.state(&org).unwrap();
    let leaf = |t: AccountType| {
        books
            .accounts
            .iter()
            .find(|a| a.account_type == t && !a.is_group && a.is_active)
            .unwrap()
            .id
            .clone()
    };
    let (expense, asset) = (leaf(AccountType::Expense), leaf(AccountType::Asset));
    let post = sanitize_command(&serde_json::json!({
        "type": "entry.post",
        "input": {
            "date": today_utc().as_str(),
            "memo": "DT2 live check: clinic supplies",
            "lines": [
                { "account_id": expense.as_str(), "debit": 150_000, "fund_id": UNRESTRICTED_FUND_ID },
                { "account_id": asset.as_str(), "credit": 150_000, "fund_id": UNRESTRICTED_FUND_ID },
            ],
        },
    }))
    .unwrap();
    let CommandResult::Id { id: entry } = settle(backend.send(&org, post)).unwrap() else {
        panic!("entry.post returns the entry id");
    };
    println!("dt2: posted entry {entry}");

    // Two nodes and a movement between them, then the link to the entry.
    for (name, kind) in [("DT2 Donor", "donor"), ("DT2 Clinic", "project")] {
        let node =
            serde_json::from_value(serde_json::json!({ "name": name, "type": kind })).unwrap();
        settle(backend.write_flow(&org, FlowWrite::CreateNode(node))).unwrap();
    }
    let node_id = |name: &str| {
        backend.transparency(&org).and_then(|t| {
            t.flow_nodes
                .iter()
                .find(|n| n.name == name)
                .map(|n| n.id.clone())
        })
    };
    until(&rx, "the two nodes", || {
        node_id("DT2 Donor").is_some() && node_id("DT2 Clinic").is_some()
    });
    let movement = serde_json::from_value(serde_json::json!({
        "from_node_id": node_id("DT2 Donor").unwrap(),
        "to_node_id": node_id("DT2 Clinic").unwrap(),
        "amount": 150_000, "currency": "KES", "description": "DT2 clinic supplies",
        "verification_status": "pending", "transaction_date": today_utc().as_str(),
    }))
    .unwrap();
    settle(backend.write_flow(&org, FlowWrite::CreateTransaction(movement))).unwrap();
    let txn = |backend: &SpacetimeBackend| {
        backend.transparency(&org).and_then(|t| {
            t.transactions
                .iter()
                .find(|x| x.description == "DT2 clinic supplies")
                .cloned()
        })
    };
    until(&rx, "the movement", || txn(&backend).is_some());
    let txn_id = txn(&backend).unwrap().id;
    settle(backend.write_flow(
        &org,
        FlowWrite::LinkEntries {
            id: txn_id.clone(),
            entry_ids: vec![entry.clone()],
        },
    ))
    .unwrap();
    until(&rx, "the link", || {
        txn(&backend).is_some_and(|x| x.journal_entry_ids == [entry.clone()])
    });

    // The map shows the edge; its ledger link opens the entry's lines only.
    let records = backend.transparency(&org).unwrap();
    let page = FlowPage::of(&records, PeriodId::Week);
    let edge = page
        .graph
        .edges
        .iter()
        .find(|e| e.transactions.iter().any(|t| t.transaction.id == txn_id))
        .unwrap();
    let details = Details::of(&page.graph, &Selection::Edge(edge.id.clone())).unwrap();
    println!(
        "dt2: edge {} ({}), {}, {}",
        details.heading,
        details.subtitle,
        details.figures[0].1,
        page.figures()[0].value
    );
    let (_, link) = details.ledger.clone().unwrap();
    assert_eq!(link, LedgerLink::Source(txn_id.clone()));
    let filter = link.filter(Some(&records));
    let table = GlTable::derive(&backend.state(&org).unwrap(), &filter);
    assert_eq!(table.rows.len(), 2);
    assert!(table.rows.iter().all(|r| r.entry_id.as_str() == entry));
    println!(
        "dt2: ledger lines {} ({:?})",
        table.rows.len(),
        filter.scope_note(&backend.state(&org).unwrap())
    );

    // A chain from the first to the second, approved and revoked for the second.
    settle(backend.write_chain(ChainWrite::Request {
        source: org.clone(),
        target: org_b.clone(),
        message: Some("DT2 live check: follow the clinic grant.".to_owned()),
    }))
    .unwrap();
    let chain = |backend: &SpacetimeBackend, status: ChainStatus| {
        backend.chains().into_iter().find(|c| {
            c.chain.source_org_id == org.as_str()
                && c.chain.target_org_id == org_b.as_str()
                && c.chain.status == status
        })
    };
    until(&rx, "the request", || {
        chain(&backend, ChainStatus::Pending).is_some()
    });
    let id = chain(&backend, ChainStatus::Pending).unwrap().chain.id;
    let incoming = ChainsView::of(&backend.chains(), org_b.as_str());
    let row = incoming.incoming.iter().find(|r| r.id == id).unwrap();
    assert_eq!(row.actions, [ChainAction::Approve, ChainAction::Reject]);
    let group = backend.transparency(&org_b).unwrap().access_groups[0].clone();
    settle(backend.write_chain(ChainWrite::Approve {
        id: id.clone(),
        group: group.id.clone(),
        node: None,
    }))
    .unwrap();
    until(&rx, "the approval", || {
        chain(&backend, ChainStatus::Active).is_some()
    });
    let chains = backend.chains();
    for (side, view) in [
        ("source", ChainsView::of(&chains, org.as_str())),
        ("target", ChainsView::of(&chains, org_b.as_str())),
    ] {
        let row = view
            .incoming
            .iter()
            .chain(&view.outgoing)
            .find(|r| r.id == id)
            .unwrap();
        println!(
            "dt2: {side} sees {} -> {}: {}, access {} ({:?}), Chain Trust Score {} / 100, {}",
            row.source,
            row.target,
            row.when,
            row.access,
            row.access_note,
            row.score,
            row.score_note
        );
        assert_eq!(row.access, group.name);
    }
    settle(backend.write_chain(ChainWrite::Revoke { id: id.clone() })).unwrap();
    until(&rx, "the revocation", || {
        chain(&backend, ChainStatus::Revoked).is_some()
    });
    println!("dt2: chain {id} revoked");
}
