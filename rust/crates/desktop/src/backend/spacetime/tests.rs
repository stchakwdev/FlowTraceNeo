#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::time::Duration;

use ledger_core::IssueCode;
use ledger_core::{Issue, LedgerError};

use super::fake::{Fake, ME, member, ready};
use super::*;

fn connected() -> ConnectionStatus {
    ConnectionStatus::Connected {
        identity: ME.to_owned(),
    }
}

fn kinds(events: &[BackendEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            BackendEvent::Ledger { org, .. } => format!("ledger {org}"),
            BackendEvent::Organisations(orgs) => format!(
                "orgs {}",
                orgs.iter()
                    .map(|o| o.name.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            BackendEvent::Status(s) => format!("status {s:?}"),
        })
        .collect()
}

fn add_vendor() -> LedgerCommand {
    ledger_core::sanitize_command(&serde_json::json!({
        "type": "vendor.add",
        "input": { "name": "Acme", "node_id": null, "payment_details": "Paybill 000000" }
    }))
    .unwrap()
}

/// Receives the next event, failing after a second.
fn next(rx: &Receiver<BackendEvent>) -> BackendEvent {
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        if let Ok(event) = rx.try_recv() {
            return event;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no event within a second"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn client_statuses_map_onto_the_desktop_ones() {
    assert_eq!(
        status_of(&ClientStatus::Connecting),
        ConnectionStatus::Connecting
    );
    assert_eq!(
        status_of(&ClientStatus::Connected {
            identity: ME.to_owned()
        }),
        ConnectionStatus::Connecting,
        "rows are not readable until Ready"
    );
    assert_eq!(
        status_of(&ClientStatus::Ready {
            identity: ME.to_owned()
        }),
        connected()
    );
    assert_eq!(
        status_of(&ClientStatus::Disconnected { error: None }),
        ConnectionStatus::Disconnected
    );
    assert_eq!(
        status_of(&ClientStatus::Disconnected {
            error: Some("refused".to_owned())
        }),
        ConnectionStatus::Error {
            message: "refused".to_owned()
        }
    );
}

#[test]
fn ready_sends_the_organisations_then_every_books_then_the_status() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![]);
    fake.add_org("org-b", vec![]);
    let mut live = Live::default();

    let out = live.apply(ready(), fake.as_ref());
    assert_eq!(
        kinds(&out),
        [
            "orgs org-a,org-b",
            "ledger org-a",
            "ledger org-b",
            &format!("status {:?}", connected()),
        ]
    );
    let BackendEvent::Ledger { state, .. } = &out[1] else {
        panic!("ledger event");
    };
    assert_eq!(state.settings.organization_id.as_str(), "org-a");
    assert_eq!(live.books["org-a"], *state, "the snapshot is kept");
}

#[test]
fn a_change_rereads_only_the_named_books_and_the_list_only_when_it_moved() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![]);
    fake.add_org("org-b", vec![]);
    let mut live = Live::default();
    live.apply(ready(), fake.as_ref());

    let changed = |orgs: &[&str]| ClientEvent::Changed {
        orgs: orgs.iter().map(|o| (*o).to_owned()).collect(),
    };
    let out = live.apply(changed(&["org-b"]), fake.as_ref());
    assert_eq!(kinds(&out), ["ledger org-b"]);

    fake.add_org("org-c", vec![]);
    let out = live.apply(changed(&["org-c"]), fake.as_ref());
    assert_eq!(kinds(&out), ["orgs org-a,org-b,org-c", "ledger org-c"]);
}

#[test]
fn errors_and_statuses_become_status_events_once() {
    let fake = Fake::new();
    let mut live = Live::default();
    let out = live.apply(ClientEvent::Status(ClientStatus::Connecting), fake.as_ref());
    assert_eq!(kinds(&out), ["status Connecting"]);
    let again = live.apply(
        ClientEvent::Status(ClientStatus::Connected {
            identity: ME.to_owned(),
        }),
        fake.as_ref(),
    );
    assert!(again.is_empty(), "still connecting: {again:?}");

    let out = live.apply(
        ClientEvent::Error("socket closed".to_owned()),
        fake.as_ref(),
    );
    assert_eq!(
        out,
        [BackendEvent::Status(ConnectionStatus::Error {
            message: "socket closed".to_owned()
        })]
    );
}

#[test]
fn the_backend_forwards_a_recorded_stream_to_every_subscriber() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.attach(fake.clone()).unwrap();

    fake.emit(ClientEvent::Status(ClientStatus::Connecting));
    fake.emit(ready());
    assert_eq!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connecting)
    );
    assert!(matches!(next(&rx), BackendEvent::Organisations(o) if o.len() == 1));
    let BackendEvent::Ledger { org, state } = next(&rx) else {
        panic!("ledger event");
    };
    assert_eq!(org.as_str(), "org-a");
    assert_eq!(next(&rx), BackendEvent::Status(connected()));

    assert_eq!(backend.status(), connected());
    assert_eq!(backend.state(&org), Some(state));
    assert_eq!(
        backend.organisations(),
        [Organisation {
            id: OrganizationId::from("org-a"),
            name: "org-a".to_owned(),
            base_currency: "KES".to_owned(),
        }]
    );
}

#[test]
fn the_actor_is_the_identitys_member_row() {
    let fake = Fake::new();
    let other = "ab".repeat(32);
    // `spacetime call` writes identities as 0x plus hex; the SDK as bare hex.
    fake.add_org(
        "org-a",
        vec![
            member(&other, "someone-else", false),
            member(&format!("0x{}", ME.to_uppercase()), "me", true),
        ],
    );
    fake.add_org("org-b", vec![member(&other, "someone-else", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while next(&rx) != BackendEvent::Status(connected()) {}

    let state = backend.state(&OrganizationId::from("org-a")).unwrap();
    assert!(!state.accounts.is_empty(), "built books");
    assert_eq!(
        backend.actor(&OrganizationId::from("org-a")),
        Some(Actor {
            id: ActorId::from("me"),
            name: "Name of me".to_owned(),
            can_post: true,
            can_approve: false,
            can_close: true,
        })
    );
    assert_eq!(backend.actor(&OrganizationId::from("org-b")), None);
    assert_eq!(backend.actor(&OrganizationId::from("nowhere")), None);
}

#[test]
fn writes_go_out_only_while_connected() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let org = OrganizationId::from("org-a");

    let mut idle = SpacetimeBackend::new();
    let refused = idle.send(&org, add_vendor()).now().unwrap();
    assert_eq!(refused, Err(BackendError::Offline));
    assert!(idle.is_remote());

    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.attach(fake.clone()).unwrap();
    fake.emit(ClientEvent::Status(ClientStatus::Connecting));
    assert_eq!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connecting)
    );
    assert_eq!(
        backend.send(&org, add_vendor()).now().unwrap(),
        Err(BackendError::Offline),
        "no writes before the books are loaded"
    );

    fake.emit(ready());
    while next(&rx) != BackendEvent::Status(connected()) {}
    assert_eq!(
        backend.send(&org, add_vendor()).now().unwrap(),
        Ok(CommandResult::None)
    );
    assert_eq!(backend.submit(&org, add_vendor()), Ok(CommandResult::None));
    assert_eq!(lock(&fake.sent).len(), 2);
    assert_eq!(
        backend
            .send(&OrganizationId::from("elsewhere"), add_vendor())
            .now()
            .unwrap(),
        Err(BackendError::UnknownOrganisation("elsewhere".to_owned()))
    );

    // The connection drops: the last books stay readable, writes are refused, nothing hangs.
    let books = backend.state(&org).unwrap();
    fake.emit(ClientEvent::Status(ClientStatus::Disconnected {
        error: Some("connection reset".to_owned()),
    }));
    assert_eq!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Error {
            message: "connection reset".to_owned()
        })
    );
    assert_eq!(backend.state(&org), Some(books));
    assert_eq!(
        backend.send(&org, add_vendor()).now().unwrap(),
        Err(BackendError::Offline)
    );
    assert_eq!(lock(&fake.sent).len(), 2, "nothing more went out");
    assert!(
        backend.actor(&org).is_some(),
        "the actor is still known offline"
    );
}

#[test]
fn disconnect_reports_disconnected_and_keeps_the_books() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while next(&rx) != BackendEvent::Status(connected()) {}

    backend.disconnect();
    assert_eq!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Disconnected)
    );
    assert_eq!(backend.status(), ConnectionStatus::Disconnected);
    assert!(backend.state(&OrganizationId::from("org-a")).is_some());

    // A late event of the closed connection changes nothing.
    fake.emit(ready());
    std::thread::sleep(Duration::from_millis(50));
    assert!(rx.try_recv().is_err());
    assert_eq!(backend.status(), ConnectionStatus::Disconnected);
}

#[test]
fn a_broken_rule_reads_as_the_in_process_refusal() {
    let rule = LedgerError::new(vec![Issue::new(
        IssueCode::Unbalanced,
        "Debits and credits differ by KES 1.00.",
    )])
    .unwrap();
    assert_eq!(
        backend_error(ClientError::Ledger(rule.clone())),
        BackendError::Command(CommandError::Engine(engine::Error::Ledger(rule)))
    );
    assert_eq!(
        backend_error(ClientError::Disconnected),
        BackendError::Offline
    );
    assert_eq!(
        backend_error(ClientError::Timeout),
        BackendError::Connection("The ledger did not confirm the command in time.".to_owned())
    );
}

#[test]
fn the_token_path_reaches_the_client() {
    let settings = ConnectSettings {
        host: "https://maincloud.spacetimedb.com".to_owned(),
        database: "flowtrace-ledger-dev".to_owned(),
        token_path: "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com".to_owned(),
        access_token: None,
    };
    let config = SpacetimeBackend::client_config(&settings);
    assert_eq!(config.uri, settings.host);
    assert_eq!(config.database, settings.database);
    assert_eq!(
        config.token_file.unwrap().path(),
        std::path::Path::new(&settings.token_path)
    );

    assert!(config.access_token.is_none(), "a device identity");

    let signed_in = SpacetimeBackend::client_config(&ConnectSettings {
        access_token: Some(ledger_client::AccessToken::new("eyJ.access.sig")),
        ..settings.clone()
    });
    assert_eq!(
        signed_in.presented_token().unwrap(),
        (Some("eyJ.access.sig".to_owned()), false),
        "a WorkOS sign-in presents its access token and saves nothing in the token file"
    );

    let anonymous = SpacetimeBackend::client_config(&ConnectSettings {
        token_path: String::new(),
        ..settings
    });
    assert!(anonymous.token_file.is_none());
}
