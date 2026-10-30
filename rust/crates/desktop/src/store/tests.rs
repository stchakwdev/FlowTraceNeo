#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use ledger_client::{ClientEvent, ConnectionStatus as ClientStatus};
use ledger_core::CommandResult;

use super::*;
use crate::backend::spacetime_fake::{Fake, ME, member, ready};
use crate::backend::{ConnectionStatus, InProcessBackend, SpacetimeBackend};

fn add_vendor() -> LedgerCommand {
    ledger_core::sanitize_command(&serde_json::json!({
        "type": "vendor.add",
        "input": { "name": "Acme", "node_id": null, "payment_details": "Paybill 000000" }
    }))
    .unwrap()
}

fn settings() -> ConnectSettings {
    ConnectSettings {
        host: "https://maincloud.spacetimedb.com".to_owned(),
        database: "flowtrace-ledger-dev".to_owned(),
        token_path: "/tmp/ft-u10-unused-token".to_owned(),
        access_token: None,
    }
}

/// A connector handing out a [`SpacetimeBackend`] on `fake`, as the app's does on a server.
fn connector_to(fake: &Arc<Fake>) -> Connector {
    let fake = Arc::clone(fake);
    Box::new(move |_| {
        let mut backend = SpacetimeBackend::new();
        backend.attach(fake.clone())?;
        Ok(Box::new(backend) as Box<dyn LedgerBackend>)
    })
}

fn refusing() -> Connector {
    Box::new(|_| Err(BackendError::Connection("Server unreachable.".to_owned())))
}

fn demo_store() -> (StoreCore, Receiver<BackendEvent>) {
    StoreCore::new(
        Box::new(InProcessBackend::with_demo_and_empty().unwrap()),
        refusing(),
    )
}

/// Drains `rx` into the store, as the pump does, until `done` holds or a second passes.
fn pump_until(
    store: &mut StoreCore,
    rx: &Receiver<BackendEvent>,
    done: impl Fn(&StoreCore) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !done(store) {
        let batch: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        if !batch.is_empty() {
            store.apply(batch);
        }
        assert!(Instant::now() < deadline, "store never reached the state");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_demo_store_writes_at_once_and_is_never_offline() {
    let (mut store, rx) = demo_store();
    assert!(!store.view().is_remote());
    assert_eq!(store.view().offline_notice(), None);
    let before = store.view().revision();

    let result = store.submit(add_vendor()).now().unwrap();
    assert!(matches!(result, Ok(CommandResult::Id { .. })), "{result:?}");
    store.apply(vec![rx.try_recv().unwrap()]);
    assert_eq!(store.view().revision(), before + 1);
}

#[test]
fn connecting_swaps_in_the_live_books_and_writes_go_to_the_server() {
    let fake = Fake::new();
    fake.add_org("org-live", vec![member(ME, "me", true)]);
    let (mut store, _demo_rx) = StoreCore::new(
        Box::new(InProcessBackend::with_demo_and_empty().unwrap()),
        connector_to(&fake),
    );

    let rx = store.connect(settings()).unwrap();
    assert!(store.view().is_remote());
    assert_eq!(store.view().organisations(), &[], "the demo books are gone");
    assert!(store.view().offline_notice().is_some());
    assert!(
        store.submit(add_vendor()).now().unwrap().is_err(),
        "no write before the books load"
    );

    fake.emit(ClientEvent::Status(ClientStatus::Connecting));
    pump_until(&mut store, &rx, |s| {
        s.view().status() == &ConnectionStatus::Connecting
    });
    assert!(
        store
            .view()
            .offline_notice()
            .unwrap()
            .starts_with("Connecting")
    );

    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());
    assert_eq!(store.view().selected().unwrap().name, "org-live");
    assert!(store.view().snapshot().is_some(), "the books are visible");
    assert_eq!(store.actor().unwrap().id.as_str(), "me");

    assert_eq!(
        store.submit(add_vendor()).now().unwrap(),
        Ok(CommandResult::None)
    );
    assert_eq!(fake.sent.lock().unwrap().len(), 1);
}

#[test]
fn a_dropped_connection_keeps_the_books_shows_the_notice_and_refuses_writes() {
    let fake = Fake::new();
    fake.add_org("org-live", vec![member(ME, "me", true)]);
    let (mut store, _) = StoreCore::new(Box::new(SpacetimeBackend::new()), connector_to(&fake));
    let rx = store.connect(settings()).unwrap();
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());
    let books = Arc::clone(store.view().snapshot().unwrap());

    fake.emit(ClientEvent::Error("connection reset".to_owned()));
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_some());
    assert!(Arc::ptr_eq(store.view().snapshot().unwrap(), &books));
    assert!(
        store
            .view()
            .offline_notice()
            .unwrap()
            .starts_with("You are offline. These are the last books seen")
    );
    let refused = store.submit(add_vendor()).now().unwrap().unwrap_err();
    assert_eq!(refused, BackendError::Offline);
    assert!(
        refused
            .to_string()
            .starts_with("You are offline, so nothing was saved.")
    );
    assert_eq!(fake.sent.lock().unwrap().len(), 0);

    // Reconnecting: the server is back.
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());
}

#[test]
fn disconnect_goes_offline_with_the_last_books() {
    let fake = Fake::new();
    fake.add_org("org-live", vec![member(ME, "me", true)]);
    let (mut store, _) = StoreCore::new(Box::new(SpacetimeBackend::new()), connector_to(&fake));
    let rx = store.connect(settings()).unwrap();
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());

    store.disconnect();
    pump_until(&mut store, &rx, |s| {
        s.view().status() == &ConnectionStatus::Disconnected
    });
    assert!(store.view().snapshot().is_some());
    assert!(store.view().status_text().ends_with("Offline"));
}

#[test]
fn a_refused_connect_keeps_the_books_open() {
    let (mut store, _) = demo_store();
    let err = store.connect(settings()).unwrap_err();
    assert_eq!(
        err,
        BackendError::Connection("Server unreachable.".to_owned())
    );
    assert!(!store.view().is_remote());
    assert!(store.view().snapshot().is_some());
}

#[test]
fn nothing_selected_refuses_at_once() {
    let (mut store, _) = StoreCore::new(Box::new(SpacetimeBackend::new()), refusing());
    assert_eq!(
        store.submit(add_vendor()).now().unwrap(),
        Err(BackendError::UnknownOrganisation(String::new()))
    );
}

fn proof_of(kind: crate::proofs::LinkKind, id: &str) -> NewVerification {
    let sha = "b".repeat(64);
    crate::proofs::new_verification(
        kind,
        id,
        &crate::proofs::Attachment {
            name: "fuel.jpg".to_owned(),
            uploaded: crate::proofs::Uploaded {
                key: format!("org/o/{sha}"),
                sha256: sha,
                size: 10,
                content_type: "image/jpeg".to_owned(),
                thumbnail_key: None,
            },
        },
    )
}

#[test]
fn the_demo_books_hold_no_worker_token_so_attaching_says_to_connect() {
    let (mut store, _rx) = demo_store();
    let missing = store.proofs_client().unwrap_err();
    assert!(missing.to_string().contains("proofs_url"), "{missing}");
    store.set_proofs_url(Some("  ".to_owned()));
    assert!(
        store
            .proofs_client()
            .unwrap_err()
            .to_string()
            .contains("proofs_url")
    );
    store.set_proofs_url(Some("https://proofs.example.org".to_owned()));
    assert_eq!(
        store.proofs_client().unwrap_err(),
        ProofsError::Unavailable(crate::backend::NO_PROOFS.to_owned())
    );
}

#[test]
fn a_proof_is_recorded_on_a_demo_claim_by_the_modules_rules() {
    let (mut store, _rx) = demo_store();
    let claim = store.view().snapshot().unwrap().claims[0].id.clone();
    let made = store
        .attach_proof(proof_of(LinkKind::Claim, claim.as_str()))
        .now()
        .unwrap();
    assert_eq!(made, Ok(CommandResult::None));
    let proofs = store.proofs_of(LinkKind::Claim, claim.as_str());
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].sha256.as_deref(), Some("b".repeat(64).as_str()));
    assert_eq!(store.proofs_of(LinkKind::Bill, claim.as_str()).len(), 0);

    let refused = store
        .attach_proof(proof_of(LinkKind::Claim, "EC-9999"))
        .now()
        .unwrap();
    assert_eq!(
        refused,
        Err(BackendError::Connection(
            "Expense claim EC-9999 not found.".to_owned()
        ))
    );
}

#[test]
fn a_live_store_sends_proofs_to_the_server_and_hands_out_a_worker_client() {
    let dir = std::env::temp_dir().join(format!("ft-sto1-store-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let token_path = dir.join("token");
    ledger_client::TokenFile::new(&token_path)
        .save("tok-live")
        .unwrap();
    let fake = Fake::new();
    fake.add_org("org-live", vec![member(ME, "me", true)]);
    let connector: Connector = {
        let fake = Arc::clone(&fake);
        Box::new(move |s| {
            let mut backend = SpacetimeBackend::new();
            backend.remember(s);
            backend.attach(fake.clone())?;
            Ok(Box::new(backend) as Box<dyn LedgerBackend>)
        })
    };
    let (mut store, _demo_rx) = StoreCore::new(
        Box::new(InProcessBackend::with_demo_and_empty().unwrap()),
        connector,
    );
    let mut live = settings();
    live.token_path = token_path.to_string_lossy().into_owned();
    let rx = store.connect(live).unwrap();
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());
    store.set_proofs_url(Some("https://proofs.example.org/".to_owned()));

    let (client, org) = store.proofs_client().unwrap();
    assert_eq!(org.as_str(), "org-live");
    let shown = format!("{client:?}");
    assert!(shown.contains("https://proofs.example.org") && !shown.contains("tok-live"));

    assert_eq!(
        store
            .attach_proof(proof_of(LinkKind::Bill, "bill-1"))
            .now()
            .unwrap(),
        Ok(CommandResult::None)
    );
    let sent = fake.proofs_sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        (sent[0].0.as_str(), sent[0].1.bill_id.as_deref()),
        ("org-live", Some("bill-1"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_transparency_snapshot_has_its_own_revision() {
    let (mut store, rx) = demo_store();
    let records = store.view().transparency().unwrap().clone();
    assert_eq!(records.campaigns.len(), 4, "the demo's campaigns at open");
    let seen = store.view().transparency_revision();

    store.submit(add_vendor()).now().unwrap().unwrap();
    store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect());
    assert_eq!(
        store.view().transparency_revision(),
        seen,
        "a ledger change leaves the records alone"
    );

    let campaign = serde_json::from_value(serde_json::json!({
        "name": "Boreholes", "target_amount": 100_000, "currency": "KES",
        "starts_at": "2026-02-04T09:00:00.000Z",
    }))
    .unwrap();
    let books = store.view().revision();
    store
        .write_campaign(CampaignWrite::Create(campaign))
        .now()
        .unwrap()
        .unwrap();
    assert!(store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect()));
    assert_eq!(store.view().transparency_revision(), seen + 1);
    assert_eq!(
        store.view().revision(),
        books,
        "the books are not re-derived"
    );
    assert_eq!(store.view().transparency().unwrap().campaigns.len(), 5);
}

#[test]
fn selecting_another_organisation_reads_its_records() {
    let (mut store, _rx) = demo_store();
    let empty = store.view().organisations()[1].id.clone();
    assert!(store.select_org(&empty));
    assert_eq!(store.view().transparency().unwrap().campaigns, []);
}

#[test]
fn a_live_store_reads_records_when_the_server_says_they_changed() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let (mut store, _rx) = StoreCore::new(Box::new(SpacetimeBackend::new()), connector_to(&fake));
    let rx = store.connect(settings()).unwrap();
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().snapshot().is_some());
    assert!(store.view().transparency().is_none(), "nothing read yet");

    fake.transparency.lock().unwrap().insert(
        "org-a".to_owned(),
        crate::backend::demo_transparency().unwrap(),
    );
    fake.emit(ClientEvent::TransparencyChanged {
        orgs: std::iter::once("org-a".to_owned()).collect(),
        profiles: false,
        chains: false,
        public: true,
        service: false,
    });
    pump_until(&mut store, &rx, |s| s.view().transparency().is_some());
    assert_eq!(store.view().transparency().unwrap().donations.len(), 25);

    let write = CampaignWrite::Delete {
        id: "c1".to_owned(),
    };
    assert_eq!(
        store.write_campaign(write.clone()).now().unwrap(),
        Ok(CommandResult::None)
    );
    assert_eq!(
        *fake.campaign_writes.lock().unwrap(),
        [("org-a".to_owned(), write)]
    );
}

#[test]
fn the_store_reads_chains_at_open_and_again_after_a_chain_write() {
    use ledger_module::transparency::chains::ChainStatus;
    let (mut store, rx) = demo_store();
    assert_eq!(store.view().chains().len(), 4, "the demo's chains at open");
    let seen = store.view().chains_revision();
    let pending = store
        .view()
        .chains()
        .iter()
        .find(|c| c.chain.status == ChainStatus::Pending)
        .unwrap()
        .chain
        .id
        .clone();
    store
        .write_chain(crate::backend::ChainWrite::Reject {
            id: pending.clone(),
            reason: Some("Not now.".to_owned()),
        })
        .now()
        .unwrap()
        .unwrap();
    let records = store.view().transparency_revision();
    assert!(store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect()));
    assert_eq!(store.view().chains_revision(), seen + 1);
    assert_eq!(
        store.view().transparency_revision(),
        records,
        "a chain change leaves the records alone"
    );
    let c = store
        .view()
        .chains()
        .iter()
        .find(|c| c.chain.id == pending)
        .unwrap();
    assert_eq!(c.chain.status, ChainStatus::Rejected);
    assert_ne!(store.public_organizations().len(), 0);
    assert_eq!(store.chain_activity(&pending)[0].chain_id, pending);
}

#[test]
fn a_flow_write_reaches_the_selected_organisations_records() {
    let (mut store, rx) = demo_store();
    let before = store.view().transparency().unwrap().flow_nodes.len();
    let node =
        serde_json::from_value(serde_json::json!({ "name": "Clinic", "type": "project" })).unwrap();
    store
        .write_flow(crate::backend::FlowWrite::CreateNode(node))
        .now()
        .unwrap()
        .unwrap();
    assert!(store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect()));
    assert_eq!(
        store.view().transparency().unwrap().flow_nodes.len(),
        before + 1
    );
}

#[test]
fn a_transaction_proof_is_attached_published_and_its_view_logged_in_process() {
    let (mut store, rx) = demo_store();
    let txn = store.view().transparency().unwrap().transactions[0]
        .id
        .clone();
    let before = store.proofs_of(LinkKind::Transaction, &txn).len();
    store
        .attach_proof(proof_of(LinkKind::Transaction, &txn))
        .now()
        .unwrap()
        .unwrap();
    store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect());
    let proofs = store.proofs_of(LinkKind::Transaction, &txn);
    assert_eq!(proofs.len(), before + 1);
    let mine = proofs
        .iter()
        .find(|v| v.sha256.as_deref() == Some("b".repeat(64).as_str()))
        .unwrap()
        .clone();
    assert!(!mine.published);
    assert!(
        store.actor().unwrap().can_close,
        "the demo user is a manager"
    );
    store.mark_published(&mine.id, true).now().unwrap().unwrap();
    store
        .log_verification_access(&mine.id, AccessType::View)
        .now()
        .unwrap()
        .unwrap();
    store.apply(std::iter::from_fn(|| rx.try_recv().ok()).collect());
    let records = store.view().transparency().unwrap().clone();
    let now = records
        .verifications
        .iter()
        .find(|v| v.id == mine.id)
        .unwrap();
    assert!(now.published);
    let logged = records.access_log.last().unwrap();
    assert_eq!(
        (logged.verification_id.as_str(), logged.access_type),
        (mine.id.as_str(), AccessType::View)
    );
    assert_eq!(
        store
            .log_verification_access("nope", AccessType::View)
            .now()
            .unwrap(),
        Err(BackendError::Connection(
            "Verification not found.".to_owned()
        ))
    );
}

fn signed_in_session(token: &str) -> crate::signin::Session {
    crate::signin::Session {
        access_token: ledger_client::AccessToken::new(token),
        claims: crate::signin::TokenClaims {
            issuer: "https://api.workos.com/user_management/client_1".to_owned(),
            subject: "user_1".to_owned(),
            audience: None,
            expires_at: Some(1_900_000_000),
        },
        user: crate::signin::SignedInUser {
            id: "user_1".to_owned(),
            name: "Alice Achieng".to_owned(),
            email: "alice@example.org".to_owned(),
        },
        organization_id: None,
    }
}

#[test]
fn a_workos_connection_sends_its_access_token_to_the_worker_and_the_refreshed_one_after() {
    let dir = std::env::temp_dir().join(format!("ft-dt3-store-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let token_path = dir.join("token");
    ledger_client::TokenFile::new(&token_path)
        .save("device-token")
        .unwrap();
    let fake = Fake::new();
    fake.add_org("org-live", vec![member(ME, "me", true)]);
    let connector: Connector = {
        let fake = Arc::clone(&fake);
        Box::new(move |s| {
            let mut backend = SpacetimeBackend::new();
            backend.remember(s);
            backend.attach(fake.clone())?;
            Ok(Box::new(backend) as Box<dyn LedgerBackend>)
        })
    };
    let (mut store, _demo_rx) = StoreCore::new(
        Box::new(InProcessBackend::with_demo_and_empty().unwrap()),
        connector,
    );
    let mut live = settings();
    live.token_path = token_path.to_string_lossy().into_owned();
    live.access_token = Some(ledger_client::AccessToken::new("workos-at-connect"));
    let rx = store.connect(live).unwrap();
    fake.emit(ready());
    pump_until(&mut store, &rx, |s| s.view().offline_notice().is_none());
    store.set_proofs_url(Some("https://proofs.example.org".to_owned()));
    let bearer = |store: &StoreCore| {
        let (client, _) = store.proofs_client().unwrap();
        client.bearer_for_test().to_owned()
    };
    assert_eq!(bearer(&store), "workos-at-connect", "not the device token");

    store
        .signin_mut()
        .signed_in(signed_in_session("workos-first"));
    assert_eq!(bearer(&store), "workos-first");
    store
        .signin_mut()
        .refreshed(signed_in_session("workos-refreshed"));
    assert_eq!(bearer(&store), "workos-refreshed");
    store.signin_mut().signed_out();
    assert_eq!(bearer(&store), "workos-at-connect");

    store.mark_published("v1", true).now().unwrap().unwrap();
    store
        .log_verification_access("v1", AccessType::Download)
        .now()
        .unwrap()
        .unwrap();
    assert_eq!(
        fake.published.lock().unwrap().clone(),
        [("org-live".to_owned(), "v1".to_owned(), true)]
    );
    assert_eq!(
        fake.accesses.lock().unwrap().clone(),
        [("org-live".to_owned(), "v1".to_owned(), AccessType::Download)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_device_connection_sends_the_token_file() {
    let dir = std::env::temp_dir().join(format!("ft-dt3-device-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let token_path = dir.join("token");
    ledger_client::TokenFile::new(&token_path)
        .save("device-token")
        .unwrap();
    let mut backend = SpacetimeBackend::new();
    let mut s = settings();
    s.token_path = token_path.to_string_lossy().into_owned();
    backend.remember(s);
    assert_eq!(backend.proofs_token().as_deref(), Some("device-token"));
    std::fs::remove_dir_all(&dir).unwrap();
}
