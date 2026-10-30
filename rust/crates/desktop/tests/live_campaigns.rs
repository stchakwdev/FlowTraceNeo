//! DT1's live check: the Campaigns screen's own path (the view models of `desktop::campaigns`
//! through `SpacetimeBackend::write_campaign`) against a real database. Ignored; it runs only by
//! hand with these set:
//!
//! - `DT1_LIVE_DB`: the database, `flowtrace-ledger-dev` (never the production one).
//! - `DT1_LIVE_ORG`: an organisation whose member the token's identity is.
//! - `DT1_LIVE_TOKEN`: the token file of that identity (made on first run), an absolute path:
//!   cargo runs the test in the crate's folder. Keep it under the gitignored `rust/out/`.
//! - `DT1_LIVE_HOST`: the server, `https://maincloud.spacetimedb.com` by default.
//!
//! `identity` prints the token's identity for the operator's `add_member`; `campaign_flow`
//! creates a draft as "Save as draft" does, opens it to donors, records a gift as "Record
//! donation" does, confirms its payment, and prints the raised amount the manage page shows.
//!
//! ```sh
//! DT1_LIVE_DB=flowtrace-ledger-dev DT1_LIVE_ORG=dt1-20261029 DT1_LIVE_TOKEN=$PWD/out/dt1-token \
//!   cargo test -p desktop --test live_campaigns -- --ignored --nocapture --test-threads 1
//! ```

#![expect(clippy::unwrap_used, reason = "a hand-run check may panic")]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use desktop::backend::{
    BackendEvent, CampaignWrite, ConnectSettings, ConnectionStatus, LedgerBackend,
    SpacetimeBackend, Submission, SubmitResult,
};
use desktop::campaigns::{
    CampaignPage, DonationForm, NewCampaignForm, ORIGIN_FALLBACK, StatusAction, confirm_payment,
    report_organization,
};
use desktop::statements::today_utc;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::types::CampaignStatus;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

fn connect() -> (
    SpacetimeBackend,
    OrganizationId,
    async_channel::Receiver<BackendEvent>,
) {
    let settings = ConnectSettings {
        host: std::env::var("DT1_LIVE_HOST")
            .unwrap_or_else(|_| "https://maincloud.spacetimedb.com".to_owned()),
        database: env("DT1_LIVE_DB"),
        token_path: env("DT1_LIVE_TOKEN"),
        access_token: None,
    };
    assert_ne!(
        settings.database, "flowtrace-ledger",
        "never the production database"
    );
    let mut backend = SpacetimeBackend::new();
    let rx = backend.subscribe();
    backend.connect(settings).unwrap();
    let org = OrganizationId::from(env("DT1_LIVE_ORG"));
    (backend, org, rx)
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
#[ignore = "live: needs DT1_LIVE_DB and a token file"]
fn identity() {
    let (backend, _, rx) = connect();
    until(&rx, "the connection", || {
        matches!(backend.status(), ConnectionStatus::Connected { .. })
    });
    let ConnectionStatus::Connected { identity } = backend.status() else {
        unreachable!("checked above");
    };
    println!("dt1: identity {identity}");
}

#[test]
#[ignore = "live: needs DT1_LIVE_DB, DT1_LIVE_ORG and a member's token file"]
fn campaign_flow() {
    let (mut backend, org, rx) = connect();
    until(&rx, "the organisation's books", || {
        backend.state(&org).is_some()
    });
    let books = backend.state(&org).unwrap();
    let name = format!("DT1 live check {}", books.settings.organization_id);

    let form = NewCampaignForm {
        name: name.clone(),
        short_description: "Desktop proof of the campaign path".to_owned(),
        target: "50000".to_owned(),
        ..NewCampaignForm::default()
    };
    let now = desktop::backend::iso_millis(
        u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap(),
    );
    let draft = form.to_input(true, &today_utc(), &now).unwrap();
    settle(backend.write_campaign(&org, CampaignWrite::Create(draft))).unwrap();
    let find = |backend: &SpacetimeBackend| {
        backend
            .transparency(&org)
            .and_then(|t| t.campaigns.iter().find(|c| c.name == name).cloned())
    };
    until(&rx, "the new campaign", || find(&backend).is_some());
    let campaign = find(&backend).unwrap();
    println!("dt1: created {} as {:?}", campaign.id, campaign.status);

    settle(backend.write_campaign(&org, StatusAction::Open.write(&campaign.id))).unwrap();
    until(&rx, "the campaign to open", || {
        find(&backend).is_some_and(|c| c.status == CampaignStatus::Active)
    });

    let gift = DonationForm {
        amount: "2500".to_owned(),
        donor_name: "DT1 donor".to_owned(),
        payment_method: "bank".to_owned(),
        ..DonationForm::default()
    }
    .to_input(&find(&backend).unwrap())
    .unwrap();
    settle(backend.write_campaign(&org, CampaignWrite::RecordDonation(gift))).unwrap();
    let donation = |backend: &SpacetimeBackend| {
        backend.transparency(&org).and_then(|t| {
            t.donations
                .iter()
                .find(|d| d.campaign_id == campaign.id)
                .map(|d| d.id.clone())
        })
    };
    until(&rx, "the donation", || donation(&backend).is_some());
    let gift_id = donation(&backend).unwrap();
    settle(backend.write_campaign(&org, confirm_payment(&gift_id))).unwrap();
    until(&rx, "the raised amount", || {
        find(&backend).is_some_and(|c| c.current_amount == 250_000)
    });

    let t = backend.transparency(&org).unwrap();
    let page = CampaignPage::of(
        &t,
        &campaign.id,
        report_organization(&t, org.as_str(), org.as_str()),
        0,
        ORIGIN_FALLBACK,
    )
    .unwrap();
    println!(
        "dt1: campaign {} {} raised {} of {} ({}), slug {:?}, donate {:?}",
        campaign.id,
        page.campaign.status == CampaignStatus::Active,
        page.coverage.raised,
        page.coverage.target,
        page.coverage.percent,
        page.campaign.public_slug,
        page.donate_url
    );
}
