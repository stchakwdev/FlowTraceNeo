//! The public tables through [`SpacetimeClient`] with no identity of its own, a step of the MOD2
//! local proof (`rust/tools/transparency-proof.ts`): a connection with no token (a fresh
//! anonymous identity, a member of nothing) must read the published organisation, badge and
//! campaigns exactly as the script read them over HTTP SQL without an `Authorization` header, be
//! told by `ClientEvent::TransparencyChanged { public: true, .. }`, and read nothing else.
//!
//! Ignored by default and a no-op unless the script sets `FT_MOD2_DB` and `FT_MOD2_PUBLIC` (the
//! public rows it read). Run the whole proof with
//! `cd rust/tools && ~/.bun/bin/bun run transparency-proof`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::time::{Duration, Instant};

use ledger_client::{ClientConfig, ClientEvent, SpacetimeClient};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::public::{PublicBadge, PublicCampaign, PublicOrganization};
use serde_json::Value;

const LOCAL: &str = "http://127.0.0.1:3100";

#[test]
#[ignore = "a step of rust/tools/transparency-proof.ts; needs the database it publishes"]
fn an_anonymous_connection_reads_the_public_tables_and_nothing_else() {
    let (Ok(db), Ok(path)) = (std::env::var("FT_MOD2_DB"), std::env::var("FT_MOD2_PUBLIC")) else {
        eprintln!("FT_MOD2_DB and FT_MOD2_PUBLIC are not set; skipping");
        return;
    };
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let orgs: Vec<PublicOrganization> =
        serde_json::from_value(expected["public_organization"].clone()).unwrap();
    let badges: Vec<PublicBadge> =
        serde_json::from_value(expected["public_badge"].clone()).unwrap();
    let campaigns: Vec<PublicCampaign> =
        serde_json::from_value(expected["public_campaign"].clone()).unwrap();
    let org = expected["org_id"].as_str().unwrap();

    let client = SpacetimeClient::connect(ClientConfig::new(LOCAL, db)).unwrap();
    let events = client.subscribe();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut told = false;
    loop {
        while let Ok(e) = events.try_recv() {
            if matches!(e, ClientEvent::TransparencyChanged { public: true, .. }) {
                told = true;
            }
        }
        if told && client.public_campaigns().unwrap().len() == campaigns.len() {
            break;
        }
        assert!(Instant::now() < deadline, "the public rows did not arrive");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(client.public_organizations().unwrap(), orgs);
    assert_eq!(client.public_badges().unwrap(), badges);
    let mut got = client.public_campaigns().unwrap();
    got.sort_by(|a, b| a.campaign.id.cmp(&b.campaign.id));
    assert_eq!(got, campaigns);
    assert_eq!(
        client.transparency(org).unwrap(),
        OrgTransparency::default()
    );
    assert_eq!(client.chains().unwrap(), []);
    assert!(client.service().is_none());
    println!(
        "client: anonymous read {} public organisation(s), {} badge(s) (score {}), {} public campaign(s); no member rows, no chains",
        orgs.len(),
        badges.len(),
        badges.first().map_or(0, |b| b.score),
        campaigns.len()
    );
    client.disconnect();
}
