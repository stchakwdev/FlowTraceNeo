//! DT3's live check: the Transactions screen and the Proofs section's own path (the view models
//! of `desktop::transactions` and `desktop::verification`, `desktop::proofs::ProofsClient`, and
//! `SpacetimeBackend::write_flow`, `attach_proof`, `log_verification_access` and
//! `mark_published`) against a real database and the proofs Worker. Ignored; it runs only by
//! hand with these set:
//!
//! - `DT3_LIVE_DB`: the database, a local `ft-dt3-<stamp>` (never the production one).
//! - `DT3_LIVE_ORG`: an organisation the token's identity is a member of, able to close periods
//!   and in an access group that sees full proofs (the Board of `create_default_access_groups`).
//! - `DT3_LIVE_TOKEN`: the token file of that identity (made on first run), an absolute path
//!   under the gitignored `rust/out/`.
//! - `DT3_LIVE_HOST`: the server, `http://127.0.0.1:3100` by default.
//! - `DT3_LIVE_PROOFS`: the Worker, `http://localhost:8787` by default (`bun run dev`).
//!
//! `identity` prints the token's identity for `add_member`; `transactions_and_proofs` adds two
//! nodes, records a transaction as the new-transaction form does, finds it on the list and its
//! page, uploads a PNG receipt and records it, records a GPS reading without a file, fetches the
//! receipt's bytes from a signed link as the viewer does and logs the view, publishes the receipt
//! as a manager and reads it back at `GET /public/<sha256>` with no token. `read_only` lists the
//! transactions and proofs of `DT3_LIVE_ORG` without writing (for `flowtrace-ledger-dev`).
//!
//! ```sh
//! DT3_LIVE_DB=ft-dt3-20261030 DT3_LIVE_ORG=dt3-org DT3_LIVE_TOKEN=$PWD/out/dt3-token \
//!   cargo test -p desktop --test live_proofs -- --ignored --nocapture --test-threads 1
//! ```

#![expect(clippy::unwrap_used, reason = "a hand-run check may panic")]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use desktop::backend::{
    BackendEvent, ConnectSettings, ConnectionStatus, FlowWrite, LedgerBackend, SpacetimeBackend,
    Submission, SubmitResult,
};
use desktop::proofs::{
    Attachment, Gps, LinkKind, ProofDetails, ProofsClient, check, new_verification_for,
};
use desktop::statements::today_utc;
use desktop::transactions::{Detail, Filter, List, NewForm, added_transaction, transaction_ids};
use desktop::verification::{ProofRow, proofs_meta, viewer_facts};
use ledger_core::types::OrganizationId;
use ledger_module::transparency::types::{AccessType, VerificationType, Visibility};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

fn settings() -> ConnectSettings {
    let settings = ConnectSettings {
        host: std::env::var("DT3_LIVE_HOST").unwrap_or_else(|_| "http://127.0.0.1:3100".to_owned()),
        database: env("DT3_LIVE_DB"),
        token_path: env("DT3_LIVE_TOKEN"),
        access_token: None,
    };
    assert_ne!(
        settings.database, "flowtrace-ledger",
        "never the production database"
    );
    settings
}

fn connect() -> (SpacetimeBackend, async_channel::Receiver<BackendEvent>) {
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.connect(settings()).unwrap();
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

/// A 1x1 PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0,
    0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
#[ignore = "live: needs DT3_LIVE_DB and a token file"]
fn identity() {
    let (backend, rx) = connect();
    until(&rx, "the connection", || {
        matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let ConnectionStatus::Connected { identity } = backend.status() else {
        unreachable!("checked above");
    };
    println!("dt3: identity {identity}");
}

#[test]
#[ignore = "live: needs DT3_LIVE_DB, DT3_LIVE_ORG, a member's token file and the Worker"]
#[expect(clippy::too_many_lines, reason = "one hand-run scenario, step by step")]
fn transactions_and_proofs() {
    let (mut backend, rx) = connect();
    let org = OrganizationId::from(env("DT3_LIVE_ORG"));
    until(&rx, "the organisation's records", || {
        backend.transparency(&org).is_some()
            && matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let started = Instant::now();

    for (name, kind) in [("DT3 Donor", "donor"), ("DT3 Clinic", "project")] {
        let node =
            serde_json::from_value(serde_json::json!({ "name": name, "type": kind })).unwrap();
        settle(backend.write_flow(&org, FlowWrite::CreateNode(node))).unwrap();
    }
    let node_id = |b: &SpacetimeBackend, name: &str| {
        b.transparency(&org).and_then(|t| {
            t.flow_nodes
                .iter()
                .find(|n| n.name == name)
                .map(|n| n.id.clone())
        })
    };
    until(&rx, "the two nodes", || {
        node_id(&backend, "DT3 Donor").is_some() && node_id(&backend, "DT3 Clinic").is_some()
    });

    // The new-transaction form's input, then the row found as the screen finds it.
    let today = today_utc();
    let form = NewForm {
        description: "DT3 clinic supplies".to_owned(),
        category: "Supplies".to_owned(),
        from: node_id(&backend, "DT3 Donor").unwrap(),
        to: node_id(&backend, "DT3 Clinic").unwrap(),
        amount: "1,250.50".to_owned(),
        currency: "KES".to_owned(),
        date: desktop::claims::iso_to_day_month_year(&today),
        reference: "DT3-LIVE-1".to_owned(),
        notes: String::new(),
    };
    let input = form.input(&today).unwrap();
    let before = transaction_ids(backend.transparency(&org).as_deref());
    settle(backend.write_flow(&org, FlowWrite::CreateTransaction(input.clone()))).unwrap();
    let found = |b: &SpacetimeBackend| {
        b.transparency(&org)
            .and_then(|t| added_transaction(&before, &t, &input))
    };
    until(&rx, "the new transaction", || found(&backend).is_some());
    let txn = found(&backend).unwrap();
    let records = backend.transparency(&org).unwrap();
    let list = List::of(&records, Filter::AwaitingProof, "dt3-live");
    assert_eq!(list.rows.len(), 1, "{:?}", list.rows);
    assert_eq!(list.rows[0].amount, "1,251");
    let detail = Detail::of(&records, &txn).unwrap();
    println!(
        "dt3: recorded {txn}: {} | {} | {}",
        detail.title, list.lede, detail.facts[4].value
    );

    // The receipt through the Worker, recorded on the transaction; then a GPS reading.
    let client = ProofsClient::new(
        &std::env::var("DT3_LIVE_PROOFS").unwrap_or_else(|_| "http://localhost:8787".to_owned()),
        backend.proofs_token().unwrap(),
    );
    let file = check("dt3-receipt.png", PNG.to_vec()).unwrap();
    let uploaded = client
        .upload(org.as_str(), LinkKind::Transaction, &txn, &file)
        .unwrap();
    println!(
        "dt3: uploaded {} ({} bytes, thumbnail {:?})",
        uploaded.key, uploaded.size, uploaded.thumbnail_key
    );
    let receipt = Attachment {
        name: file.name.clone(),
        uploaded: uploaded.clone(),
    };
    let mut details = ProofDetails::of(VerificationType::Receipt);
    details.description = "Clinic supplies receipt".to_owned();
    details.executor = "Achieng".to_owned();
    settle(backend.attach_proof(
        &org,
        new_verification_for(LinkKind::Transaction, &txn, &details, Some(&receipt)),
    ))
    .unwrap();
    let gps = ProofDetails {
        kind: VerificationType::Gps,
        visibility: Visibility::Summary,
        description: String::new(),
        executor: "Achieng".to_owned(),
        gps: Some(Gps {
            latitude: -3.119_5,
            longitude: 35.597_2,
            accuracy: Some(12.0),
            location_name: Some("Lokichar".to_owned()),
        }),
    };
    settle(backend.attach_proof(
        &org,
        new_verification_for(LinkKind::Transaction, &txn, &gps, None),
    ))
    .unwrap();
    let proofs = |b: &SpacetimeBackend| {
        desktop::verification::newest_first(
            b.proofs(&org)
                .into_iter()
                .filter(|v| v.transaction_id.as_deref() == Some(txn.as_str()))
                .collect(),
        )
    };
    until(&rx, "both proofs", || proofs(&backend).len() == 2);
    let shown = proofs(&backend);
    println!(
        "dt3: section meta {:?}; rows {:?}",
        proofs_meta(&shown),
        shown
            .iter()
            .map(|v| ProofRow::of(v).added)
            .collect::<Vec<_>>()
    );
    let image = shown
        .iter()
        .find(|v| v.object_key.is_some())
        .unwrap()
        .clone();
    let reading = shown.iter().find(|v| v.object_key.is_none()).unwrap();
    println!(
        "dt3: reading facts {:?}",
        viewer_facts(reading)
            .iter()
            .map(|f| (f.term, f.value.clone(), f.note.clone()))
            .collect::<Vec<_>>()
    );
    let records = backend.transparency(&org).unwrap();
    let after = records.transactions.iter().find(|x| x.id == txn).unwrap();
    assert_eq!(after.verification_count, 2);
    println!("dt3: transaction now {:?}", after.verification_status);

    // The viewer: a signed link, its bytes, the view logged.
    let url = client
        .signed_url(image.object_key.as_deref().unwrap())
        .unwrap();
    let bytes = client.fetch_bytes(&url).unwrap();
    assert_eq!(bytes, PNG, "the viewer draws the uploaded bytes");
    settle(backend.log_verification_access(&org, &image.id, AccessType::View)).unwrap();
    until(&rx, "the view in the access log", || {
        backend.transparency(&org).is_some_and(|t| {
            t.access_log
                .iter()
                .any(|a| a.verification_id == image.id && a.access_type == AccessType::View)
        })
    });
    println!(
        "dt3: viewer fetched {} bytes and logged the view",
        bytes.len()
    );

    // Publish as a manager, then the public path with no token.
    settle(backend.mark_published(&org, &image.id, true)).unwrap();
    until(&rx, "the published flag", || {
        proofs(&backend)
            .iter()
            .any(|v| v.id == image.id && v.published)
    });
    let base =
        std::env::var("DT3_LIVE_PROOFS").unwrap_or_else(|_| "http://localhost:8787".to_owned());
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut public = None;
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let mut response = agent
            .get(format!("{base}/public/{}", uploaded.sha256))
            .call()
            .unwrap();
        if response.status() == 200 {
            public = Some(response.body_mut().read_to_vec().unwrap());
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    assert_eq!(
        public.as_deref(),
        Some(PNG),
        "GET /public/<sha256> serves the receipt"
    );
    println!(
        "dt3: published receipt served at /public/{} ({:.1} s for the scenario)",
        uploaded.sha256,
        started.elapsed().as_secs_f64()
    );
}

#[test]
#[ignore = "live: needs DT3_LIVE_DB, DT3_LIVE_ORG and a member's token file"]
fn read_only() {
    let (backend, rx) = connect();
    let org = OrganizationId::from(env("DT3_LIVE_ORG"));
    until(&rx, "the organisation's records", || {
        backend.transparency(&org).is_some()
            && matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let records = backend.transparency(&org).unwrap();
    let list = List::of(&records, Filter::All, "");
    println!("dt3: {}", list.lede);
    for row in list.rows.iter().take(20) {
        println!(
            "dt3:   {} | {} | {} -> {} | {} {} | {} proofs",
            row.date, row.description, row.from, row.to, row.currency, row.amount, row.proofs
        );
    }
    let proofs = backend.proofs(&org);
    println!("dt3: {} proofs visible to this identity", proofs.len());
    for v in proofs.iter().take(20) {
        let row = ProofRow::of(v);
        println!(
            "dt3:   {} {} | {} | {:?} | file {}",
            row.label,
            row.points,
            row.added,
            row.limited,
            v.object_key.is_some()
        );
    }
}
