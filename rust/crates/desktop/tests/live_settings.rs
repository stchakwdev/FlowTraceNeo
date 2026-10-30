//! DT4's live check: the Trust Score, Settings, Access groups and badge screens' own path (the
//! view models of `desktop::settings`, `desktop::access_groups`, `desktop::trust` and
//! `desktop::badge` through `SpacetimeBackend::write_settings`) against a real database.
//! Ignored; it runs only by hand with these set:
//!
//! - `DT4_LIVE_DB`: the database, `flowtrace-ledger-dev` (never the production one).
//! - `DT4_LIVE_ORG`: an organisation the token's identity is a member of, able to close periods,
//!   with no access groups yet.
//! - `DT4_LIVE_TOKEN`: the token file of that identity (made on first run), an absolute path:
//!   cargo runs the test in the crate's folder. Keep it under the gitignored `rust/out/`.
//! - `DT4_LIVE_HOST`: the server, `https://maincloud.spacetimedb.com` by default.
//!
//! `identity` prints the token's identity for the operator's `create_org_ledger` and
//! `add_member`; `settings_groups_and_trust` saves the profile from the Settings form (name,
//! currency, public link) and turns "Show on public pages" on, reads the published badge,
//! creates the standard groups and a custom group from the group form, invites an address and
//! removes the invite, records a movement between two new flow nodes and recalculates the score,
//! seeing one more history point.
//!
//! ```sh
//! DT4_LIVE_DB=flowtrace-ledger-dev DT4_LIVE_ORG=dt4-20261030 DT4_LIVE_TOKEN=$PWD/out/dt4-token \
//!   cargo test -p desktop --test live_settings -- --ignored --nocapture --test-threads 1
//! ```

#![expect(clippy::unwrap_used, reason = "a hand-run check may panic")]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use desktop::access_groups::{GroupForm, ProofDepth, check_invite, member_rows, rows};
use desktop::backend::{
    BackendEvent, ConnectSettings, ConnectionStatus, FlowWrite, LedgerBackend, SettingsWrite,
    SpacetimeBackend, Submission, SubmitResult,
};
use desktop::badge::{BadgeStyle, markdown_snippet};
use desktop::settings::{Profile, public_patch};
use desktop::statements::today_utc;
use desktop::trust::TrustPage;
use ledger_core::types::OrganizationId;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

fn connect() -> (SpacetimeBackend, async_channel::Receiver<BackendEvent>) {
    let settings = ConnectSettings {
        host: std::env::var("DT4_LIVE_HOST")
            .unwrap_or_else(|_| "https://maincloud.spacetimedb.com".to_owned()),
        database: env("DT4_LIVE_DB"),
        token_path: env("DT4_LIVE_TOKEN"),
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
#[ignore = "live: needs DT4_LIVE_DB and a token file"]
fn identity() {
    let (backend, rx) = connect();
    until(&rx, "the connection", || {
        matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let ConnectionStatus::Connected { identity } = backend.status() else {
        unreachable!("checked above");
    };
    println!("dt4: identity {identity}");
}

#[test]
#[ignore = "live: needs DT4_LIVE_DB, DT4_LIVE_ORG and a member's token file"]
#[expect(clippy::too_many_lines, reason = "one hand-run scenario, step by step")]
fn settings_groups_and_trust() {
    let (mut backend, rx) = connect();
    let org = OrganizationId::from(env("DT4_LIVE_ORG"));
    until(&rx, "the organisation's books and records", || {
        backend.state(&org).is_some()
            && backend.transparency(&org).is_some()
            && matches!(backend.status(), ConnectionStatus::Connected { .. })
    });

    // The Settings form: name, currency and public link, then the switch.
    let slug = format!("{}-books", org.as_str());
    let t = backend.transparency(&org).unwrap();
    let mut form = Profile::of(&t, org.as_str(), "KES").form;
    form.name = "DT4 Live Check Trust".to_owned();
    form.currency = "KES".to_owned();
    form.country = "KE".to_owned();
    form.public_slug.clone_from(&slug);
    assert!(form.validate().is_empty());
    settle(backend.write_settings(&org, SettingsWrite::UpdateProfile(form.patch()))).unwrap();
    settle(backend.write_settings(&org, SettingsWrite::UpdateProfile(public_patch(true)))).unwrap();
    until(&rx, "the public profile", || {
        backend
            .transparency(&org)
            .and_then(|t| t.profile.clone())
            .is_some_and(|p| p.is_public && p.public_slug.as_deref() == Some(slug.as_str()))
    });
    until(&rx, "the published badge", || {
        backend
            .public_badges()
            .iter()
            .any(|b| b.organization_id == org.as_str())
    });
    let badge = backend
        .public_badges()
        .into_iter()
        .find(|b| b.organization_id == org.as_str())
        .unwrap();
    println!(
        "dt4: badge {} {} / 100 grade {}; {}",
        badge.name,
        badge.score,
        badge.grade,
        markdown_snippet(
            "https://flowtrace.app",
            org.as_str(),
            BadgeStyle::Flat,
            "Trust Score"
        )
    );

    // The standard groups, then a custom one from the group form.
    settle(backend.write_settings(&org, SettingsWrite::CreateDefaultGroups)).unwrap();
    until(&rx, "the five default groups", || {
        backend
            .transparency(&org)
            .is_some_and(|t| t.access_groups.len() >= 5)
    });
    let mut group = GroupForm::new();
    group.set_name("DT4 Auditors");
    group.permissions = ProofDepth::Full.apply(group.permissions);
    let groups = backend.transparency(&org).unwrap().access_groups.clone();
    assert!(group.validate(&groups).is_empty());
    settle(backend.write_settings(&org, SettingsWrite::CreateGroup(group.new_group()))).unwrap();
    let custom = |backend: &SpacetimeBackend| {
        backend.transparency(&org).and_then(|t| {
            t.access_groups
                .iter()
                .find(|g| g.slug == "dt4-auditors")
                .cloned()
        })
    };
    until(&rx, "the custom group", || custom(&backend).is_some());
    let auditors = custom(&backend).unwrap();
    let t = backend.transparency(&org).unwrap();
    for r in rows(&t.access_groups, &t.group_members) {
        println!(
            "dt4: group {} ({}), sees {}, members {}",
            r.name, r.kind, r.sees, r.members
        );
    }

    // An invite from the invite form, then its removal.
    let invite = check_invite(
        "auditor@dt4.example.org",
        "",
        &t.group_members,
        &auditors.id,
        &today_utc(),
    )
    .unwrap();
    settle(backend.write_settings(
        &org,
        SettingsWrite::InviteMember {
            group: auditors.id.clone(),
            email: invite.email.clone(),
            valid_until: invite.valid_until,
        },
    ))
    .unwrap();
    let membership = |backend: &SpacetimeBackend| {
        backend.transparency(&org).and_then(|t| {
            t.group_members
                .iter()
                .find(|m| m.user_email.as_deref() == Some(invite.email.as_str()))
                .cloned()
        })
    };
    until(&rx, "the invite", || membership(&backend).is_some());
    let t = backend.transparency(&org).unwrap();
    let rows_ = member_rows(&t.group_members, &auditors.id, &[], "2026-10-30T00:00:00Z");
    println!("dt4: invited {} ({})", rows_[0].name, rows_[0].sub);
    let id = membership(&backend).unwrap().id;
    settle(backend.write_settings(&org, SettingsWrite::RemoveMember { id })).unwrap();
    until(&rx, "the removal", || membership(&backend).is_none());
    println!("dt4: invite removed");

    // One movement, then Recalculate: one more history point.
    for (name, kind) in [("DT4 Donor", "donor"), ("DT4 Clinic", "project")] {
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
        node_id("DT4 Donor").is_some() && node_id("DT4 Clinic").is_some()
    });
    let movement = serde_json::from_value(serde_json::json!({
        "from_node_id": node_id("DT4 Donor").unwrap(),
        "to_node_id": node_id("DT4 Clinic").unwrap(),
        "amount": 250_000, "currency": "KES", "description": "DT4 clinic grant",
        "verification_status": "pending", "transaction_date": today_utc().as_str(),
    }))
    .unwrap();
    settle(backend.write_flow(&org, FlowWrite::CreateTransaction(movement))).unwrap();
    until(&rx, "the movement", || {
        backend.transparency(&org).is_some_and(|t| {
            t.transactions
                .iter()
                .any(|x| x.description == "DT4 clinic grant")
        })
    });
    let before = backend.transparency(&org).unwrap().trust_history.len();
    settle(backend.write_settings(&org, SettingsWrite::RecomputeTrust)).unwrap();
    until(&rx, "the new history point", || {
        backend
            .transparency(&org)
            .is_some_and(|t| t.trust_history.len() == before + 1)
    });
    let page = TrustPage::of(&backend.transparency(&org).unwrap());
    println!(
        "dt4: score {} grade {}, {}, {:?}",
        page.score,
        page.grade,
        page.history_meta.unwrap_or_default(),
        page.components
            .iter()
            .map(|c| (c.label, c.weight.clone(), c.value.clone()))
            .collect::<Vec<_>>()
    );
}
