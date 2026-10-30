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
            BackendEvent::Transparency { org } => format!("transparency {org}"),
            BackendEvent::Chains => "chains".to_owned(),
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

fn transparency_changed(orgs: &[&str]) -> ClientEvent {
    ClientEvent::TransparencyChanged {
        orgs: orgs.iter().map(|o| (*o).to_owned()).collect(),
        profiles: false,
        chains: false,
        public: false,
        service: false,
    }
}

#[test]
fn a_transparency_change_rereads_the_named_records_and_keeps_them() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![]);
    let records = crate::backend::demo_transparency().unwrap();
    lock(&fake.transparency).insert("org-a".to_owned(), records.clone());
    let mut live = Live::default();
    live.apply(ready(), fake.as_ref());

    let out = live.apply(transparency_changed(&["org-a", "org-gone"]), fake.as_ref());
    assert_eq!(
        kinds(&out),
        ["transparency org-a"],
        "unreadable records send nothing"
    );
    assert_eq!(*live.transparency["org-a"], records);
}

#[test]
fn campaign_writes_go_to_the_client_only_while_connected() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    let org = OrganizationId::from("org-a");
    let write = CampaignWrite::Delete {
        id: "c1".to_owned(),
    };
    let offline = backend.write_campaign(&org, write.clone()).now().unwrap();
    assert_eq!(offline, Err(BackendError::Offline));

    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while !matches!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connected { .. })
    ) {}
    lock(&fake.transparency).insert(
        "org-a".to_owned(),
        crate::backend::demo_transparency().unwrap(),
    );
    fake.emit(transparency_changed(&["org-a"]));
    assert_eq!(next(&rx), BackendEvent::Transparency { org: org.clone() });
    assert_eq!(backend.transparency(&org).unwrap().campaigns.len(), 4);

    let sent = backend.write_campaign(&org, write.clone()).now().unwrap();
    assert_eq!(sent, Ok(CommandResult::None));
    assert_eq!(*lock(&fake.campaign_writes), [("org-a".to_owned(), write)]);
}

fn chains_changed() -> ClientEvent {
    ClientEvent::TransparencyChanged {
        orgs: std::collections::BTreeSet::new(),
        profiles: false,
        chains: true,
        public: false,
        service: false,
    }
}

#[test]
fn chains_are_read_at_ready_and_again_when_they_change_and_kept_offline() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![]);
    let demo = crate::backend::InProcessBackend::with_demo_and_empty()
        .unwrap()
        .chains();
    *lock(&fake.chains) = demo[..1].to_vec();
    let mut live = Live::default();
    let out = live.apply(ready(), fake.as_ref());
    assert!(
        kinds(&out).contains(&"chains".to_owned()),
        "{:?}",
        kinds(&out)
    );
    assert_eq!(live.chains.len(), 1);

    let out = live.apply(chains_changed(), fake.as_ref());
    assert_eq!(
        kinds(&out),
        Vec::<String>::new(),
        "unchanged chains send nothing"
    );
    *lock(&fake.chains) = demo.clone();
    let out = live.apply(chains_changed(), fake.as_ref());
    assert_eq!(kinds(&out), ["chains"]);
    let out = live.apply(transparency_changed(&[]), fake.as_ref());
    assert_eq!(
        kinds(&out),
        Vec::<String>::new(),
        "only a chains change rereads them"
    );
    assert_eq!(*live.chains, demo);
}

#[test]
fn flow_and_chain_writes_go_to_the_client_only_while_connected() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    let org = OrganizationId::from("org-a");
    let flow = FlowWrite::LinkEntries {
        id: "t1".to_owned(),
        entry_ids: vec!["e1".to_owned()],
    };
    let chain = ChainWrite::Revoke {
        id: "c1".to_owned(),
    };
    assert_eq!(
        backend.write_flow(&org, flow.clone()).now().unwrap(),
        Err(BackendError::Offline)
    );
    assert_eq!(
        backend.write_chain(chain.clone()).now().unwrap(),
        Err(BackendError::Offline)
    );

    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while !matches!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connected { .. })
    ) {}
    assert_eq!(
        backend
            .write_flow(&OrganizationId::from("org-b"), flow.clone())
            .now()
            .unwrap(),
        Err(BackendError::UnknownOrganisation("org-b".to_owned()))
    );
    assert_eq!(
        backend.write_flow(&org, flow.clone()).now().unwrap(),
        Ok(CommandResult::None)
    );
    assert_eq!(
        backend.write_chain(chain.clone()).now().unwrap(),
        Ok(CommandResult::None)
    );
    assert_eq!(*lock(&fake.flow_writes), [("org-a".to_owned(), flow)]);
    assert_eq!(*lock(&fake.chain_writes), [chain]);

    let public = crate::backend::InProcessBackend::with_demo_and_empty()
        .unwrap()
        .public_organizations();
    *lock(&fake.public) = public.clone();
    assert_eq!(backend.public_organizations(), public);
}

#[test]
fn settings_writes_go_to_the_client_only_while_connected() {
    let fake = Fake::new();
    fake.add_org("org-a", vec![member(ME, "me", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    let org = OrganizationId::from("org-a");
    let write = SettingsWrite::InviteMember {
        group: "g1".to_owned(),
        email: "a@example.org".to_owned(),
        valid_until: None,
    };
    assert_eq!(
        backend.write_settings(&org, write.clone()).now().unwrap(),
        Err(BackendError::Offline)
    );

    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while !matches!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connected { .. })
    ) {}
    assert_eq!(
        backend
            .write_settings(
                &OrganizationId::from("org-b"),
                SettingsWrite::RecomputeTrust
            )
            .now()
            .unwrap(),
        Err(BackendError::UnknownOrganisation("org-b".to_owned()))
    );
    for w in [write.clone(), SettingsWrite::RecomputeTrust] {
        assert_eq!(
            backend.write_settings(&org, w).now().unwrap(),
            Ok(CommandResult::None)
        );
    }
    assert_eq!(
        *lock(&fake.settings_writes),
        [
            ("org-a".to_owned(), write),
            ("org-a".to_owned(), SettingsWrite::RecomputeTrust)
        ]
    );

    let badges = crate::backend::InProcessBackend::with_demo_and_empty()
        .unwrap()
        .public_badges();
    *lock(&fake.badges) = badges.clone();
    assert_eq!(backend.public_badges(), badges);
}

#[test]
fn org_members_come_from_the_member_rows_by_name() {
    let fake = Fake::new();
    let mut zed = member("c200bb", "user_z", false);
    zed.name = "Zawadi".to_owned();
    fake.add_org("org-a", vec![zed, member(ME, "me", true)]);
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.attach(fake.clone()).unwrap();
    fake.emit(ready());
    while !matches!(
        next(&rx),
        BackendEvent::Status(ConnectionStatus::Connected { .. })
    ) {}
    let people: Vec<(String, String)> = backend
        .org_members(&OrganizationId::from("org-a"))
        .into_iter()
        .map(|p| (p.user_id, p.name))
        .collect();
    assert_eq!(
        people,
        [
            ("me".to_owned(), "Name of me".to_owned()),
            ("user_z".to_owned(), "Zawadi".to_owned())
        ]
    );
    assert_eq!(
        backend.org_members(&OrganizationId::from("org-b")),
        Vec::<OrgMember>::new()
    );
}
