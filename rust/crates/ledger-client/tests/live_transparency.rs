//! The transparency views through [`SpacetimeClient`], step 5 of the MOD1 local proof
//! (`rust/tools/transparency-proof.ts`): the client joins the demo organisation as a member of
//! the Donors group and must read the same records the script read through SQL as a Board member,
//! with every proof cut to a thumbnail (`getVerificationVisibility`,
//! `flowtrace/src/lib/supabase.ts:573-588`), announced by `ClientEvent::TransparencyChanged`.
//!
//! Ignored by default and a no-op unless the script sets `FT_MOD1_DB` (the database it published
//! on the local server) and `FT_MOD1_EXPECTED` (the records it read). Run the whole proof with
//! `cd rust/tools && ~/.bun/bin/bun run transparency-proof`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use ledger_client::{ClientConfig, ClientEvent, SpacetimeClient};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::plan::redact;
use ledger_module::transparency::types::{
    AccessGroup, Campaign, CampaignStat, Donation, DonorUpdate, GroupMember, OrganizationProfile,
    Verification, Visibility,
};
use serde_json::Value;

const LOCAL: &str = "http://127.0.0.1:3100";

fn spacetime(args: &[&str]) -> String {
    let cli = std::env::var_os("SPACETIME").map_or_else(
        || PathBuf::from(std::env::var("HOME").unwrap()).join(".local/bin/spacetime"),
        PathBuf::from,
    );
    let out = Command::new(cli).args(args).output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "spacetime {args:?} failed:\n{text}");
    text
}

/// The records of one collection of the expected JSON, typed where they are used.
macro_rules! records {
    ($v:expr) => {
        serde_json::from_value($v.clone()).unwrap()
    };
}

/// The first event `pick` takes within `within`.
fn wait_for<T>(
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

/// Waits until `ready` holds for the organisation's records, or panics after `within`.
fn wait_until(
    client: &SpacetimeClient,
    org: &str,
    within: Duration,
    ready: impl Fn(&OrgTransparency) -> bool,
) -> OrgTransparency {
    let deadline = Instant::now() + within;
    loop {
        let t = client.transparency(org).unwrap();
        if ready(&t) {
            return t;
        }
        assert!(
            Instant::now() < deadline,
            "the records did not arrive in {within:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Makes `identity` a read-only member of `org` and puts it in the Donors group, through the CLI
/// as the books' owner.
fn join_donors(db: &str, org: &str, donors: &str, identity: &str) {
    let org_json = serde_json::to_string(org).unwrap();
    spacetime(&[
        "call",
        "-s",
        "local",
        db,
        "add_member",
        &org_json,
        &format!("\"0x{identity}\""),
        "\"user_client\"",
        "\"Client Reader\"",
        "false",
        "false",
        "false",
    ]);
    let member = serde_json::json!({ "group_id": donors, "user_id": "user_client" }).to_string();
    spacetime(&[
        "call",
        "-s",
        "local",
        db,
        "add_access_group_member",
        &org_json,
        &serde_json::to_string(&member).unwrap(),
    ]);
}

/// The client's records against the script's: equal, but for the client's own group membership
/// and the proofs, which it gets as thumbnails. Returns the client's membership.
fn compare(got: &OrgTransparency, expected: &Value) -> GroupMember {
    let profile: OrganizationProfile = records!(expected["organization_profile"]);
    let groups: Vec<AccessGroup> = records!(expected["access_groups"]);
    let campaigns: Vec<Campaign> = records!(expected["campaigns"]);
    let stats: Vec<CampaignStat> = records!(expected["campaign_stats"]);
    let donations: Vec<Donation> = records!(expected["donations"]);
    let updates: Vec<DonorUpdate> = records!(expected["donor_updates"]);
    let proofs: Vec<Verification> = records!(expected["verifications"]);
    let mut members: Vec<GroupMember> = records!(expected["group_members"]);
    assert_eq!(got.profile.as_ref(), Some(&profile));
    assert_eq!(got.access_groups, groups);
    assert_eq!(got.campaigns, campaigns);
    assert_eq!(got.campaign_stats, stats);
    assert_eq!(got.donations, donations);
    assert_eq!(got.donor_updates, updates);
    let mine = got
        .group_members
        .iter()
        .find(|m| m.user_id.as_deref() == Some("user_client"))
        .cloned()
        .unwrap();
    members.push(mine.clone());
    assert_eq!(got.group_members, members);
    let thumbs: Vec<Verification> = proofs
        .iter()
        .map(|v| {
            let stored = Verification {
                visible_as: None,
                ..v.clone()
            };
            redact(&stored, Visibility::Thumbnail).unwrap()
        })
        .collect();
    assert_eq!(got.verifications, thumbs);
    assert!(got.verifications.iter().all(|v| v.object_key.is_none()));
    assert!(
        got.access_log.is_empty(),
        "the client has looked at nothing"
    );
    mine
}

#[test]
#[ignore = "step 5 of rust/tools/transparency-proof.ts; needs the database it publishes"]
fn the_client_reads_the_transparency_views_as_a_donors_group_member() {
    let (Ok(db), Ok(expected_path)) = (
        std::env::var("FT_MOD1_DB"),
        std::env::var("FT_MOD1_EXPECTED"),
    ) else {
        eprintln!("FT_MOD1_DB and FT_MOD1_EXPECTED are not set; skipping");
        return;
    };
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(expected_path).unwrap()).unwrap();
    let org = expected["org_id"].as_str().unwrap();

    let client = SpacetimeClient::connect(ClientConfig::new(LOCAL, db.clone())).unwrap();
    let events = client.subscribe();
    let identity = wait_for(&events, Duration::from_secs(30), |e| match e {
        ClientEvent::Status(s) => s.identity().map(str::to_owned),
        _ => None,
    });
    assert_eq!(
        client.transparency(org).unwrap(),
        OrgTransparency::default(),
        "a stranger reads nothing"
    );
    join_donors(
        &db,
        org,
        expected["donors_group"].as_str().unwrap(),
        &identity,
    );

    let (campaigns, proofs) = (
        expected["campaigns"].as_array().unwrap().len(),
        expected["verifications"].as_array().unwrap().len(),
    );
    let got = wait_until(&client, org, Duration::from_secs(30), |t| {
        t.campaigns.len() == campaigns
            && t.verifications.len() == proofs
            && t.verifications
                .iter()
                .all(|v| v.visible_as == Some(Visibility::Thumbnail))
    });
    wait_for(&events, Duration::from_secs(5), |e| match e {
        ClientEvent::TransparencyChanged { orgs, .. } if orgs.contains(org) => Some(()),
        _ => None,
    });
    let mine = compare(&got, &expected);
    println!(
        "client: {} campaigns, {} stats, {} donations, {} donor updates, {} groups, {} group members, {} proofs as thumbnails without object keys, TransparencyChanged received, membership granted by {}",
        got.campaigns.len(),
        got.campaign_stats.len(),
        got.donations.len(),
        got.donor_updates.len(),
        got.access_groups.len(),
        got.group_members.len(),
        got.verifications.len(),
        mine.granted_by.unwrap_or_default(),
    );
    client.disconnect();
}
