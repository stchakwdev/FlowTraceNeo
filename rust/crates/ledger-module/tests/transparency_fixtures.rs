//! The demo API's transparency calls, replayed against the module.
//!
//! `rust/fixtures/transparency-dataset.json` is the demo dataset and `transparency-calls.json` a
//! scripted run of the demo API over it (`flowtrace/src/lib/demo/api.ts`, exported by
//! `rust/tools/src/transparency.ts` under a frozen clock and counted ids). The dataset is loaded
//! through the module's TS mapping and row codec; each call then runs through the module's
//! planning function with the same arguments, ids and time, and its result, its error and the
//! records it leaves must equal the demo API's, compared in the module's shape (minor units,
//! object keys; `ledger_module::transparency::ts`). The demo API checks no permissions, so the
//! caller is a member who can close periods; the permission rules have their own unit tests.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ledger_core::env::LedgerEnv;
use ledger_core::money::js_round;
use ledger_core::types::Timestamp;
use ledger_module::tables::Member;
use ledger_module::transparency::plan::{self, Ctx, INVITE_NOTE, LedgerLinks};
use ledger_module::transparency::rows::{apply_write, from_rows, plan_write, to_rows};
use ledger_module::transparency::stats::{days_active, days_remaining, iso_ms};
use ledger_module::transparency::types::{
    AccessGroupPatch, AccessGroupPermissions, GroupMember, PaymentStatus, Visibility,
};
use ledger_module::transparency::{OrgTransparency, ts};
use serde_json::Value;
use spacetimedb::Identity;

const NOW: &str = "2026-02-04T09:00:00.000Z";

/// The note the demo writes on an invite (`api.ts:666-683`); the module writes [`INVITE_NOTE`].
const DEMO_INVITE_NOTE: &str = "Invited in the demo. No email was sent.";

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/{name}"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The exporter's frozen env (`transparency-calls.json#/env`).
struct FrozenEnv(u64);

impl LedgerEnv for FrozenEnv {
    fn now(&self) -> Timestamp {
        Timestamp::parse(NOW).unwrap()
    }

    fn new_id(&mut self) -> String {
        self.0 += 1;
        format!("00000000-0000-4000-8000-{:012}", self.0)
    }
}

/// No ledger behind the replay: the demo's proofs link to transactions only.
struct NoBooks;

impl LedgerLinks for NoBooks {
    fn has_claim(&self, _: &str) -> bool {
        false
    }

    fn has_bill(&self, _: &str) -> bool {
        false
    }
}

struct Replay {
    org_id: String,
    caller: Member,
    env: FrozenEnv,
    /// Public links of the other organisations in the dataset.
    other_slugs: Vec<String>,
}

impl Replay {
    fn ctx(&mut self) -> Ctx<'_> {
        Ctx {
            org_id: &self.org_id,
            identity: self.caller.identity,
            member: Some(&self.caller),
            env: &mut self.env,
        }
    }
}

/// The demo org's records from the dataset, through the module's rows and back.
fn load(dataset: &Value) -> OrgTransparency {
    let org_id = dataset["demo_org_id"].as_str().unwrap();
    let t = ts::dataset(org_id, &dataset["store"]).unwrap();
    let back = from_rows(&to_rows(org_id, &t).unwrap()).unwrap();
    assert_eq!(back, t, "the dataset round-trips through the rows");
    back
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}

/// The demo's text for an error, as the module words it.
fn module_error(ts_error: &str, org_id: &str) -> String {
    if ts_error == "Organization not found in the demo data." {
        plan::no_org(org_id)
    } else {
        ts_error.replace(" in the demo data.", ".")
    }
}

fn to_json<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// A planning function's outcome as JSON.
fn json<T: serde::Serialize>(v: Result<T, String>) -> Result<Option<Value>, String> {
    v.map(|x| Some(to_json(x)))
}

/// The module's outcome of one call, as the JSON of the record it returns (module shape), or the
/// error text; `None` result for calls that return nothing.
fn run(r: &mut Replay, t: &mut OrgTransparency, call: &Value) -> Result<Option<Value>, String> {
    let a = &call["args"];
    match s(&call["fn"]) {
        "updateOrganization" => {
            if s(&a[0]) != r.org_id {
                return Err(plan::no_org(s(&a[0])));
            }
            let slugs = r.other_slugs.clone();
            let patch = ts::organization_patch(&a[1]).unwrap();
            json(plan::update_organization(
                t,
                &mut r.ctx(),
                "KES",
                patch,
                &|x| slugs.iter().any(|y| y == x),
            ))
        }
        "createAccessGroup" => json(plan::create_access_group(
            t,
            &mut r.ctx(),
            ts::new_access_group(&a[0]).unwrap(),
        )),
        "updateAccessGroup" => {
            let patch: AccessGroupPatch = serde_json::from_value(a[1].clone()).unwrap();
            json(plan::update_access_group(t, &mut r.ctx(), s(&a[0]), patch))
        }
        "addGroupMember" => json(plan::add_group_member(
            t,
            &mut r.ctx(),
            ts::new_group_member(&a[0]).unwrap(),
        )),
        "inviteMemberByEmail" => json(plan::invite_group_member(
            t,
            &mut r.ctx(),
            s(&a[0]),
            s(&a[1]),
            a[3].as_str().map(str::to_owned),
        )),
        "removeGroupMember" => plan::remove_group_member(t, &mut r.ctx(), s(&a[0])).map(|()| None),
        "deleteAccessGroup" => plan::delete_access_group(t, &mut r.ctx(), s(&a[0])).map(|()| None),
        "getUserPermissions" => json(Ok(plan::user_permissions(t, s(&a[0])))),
        "getVerificationVisibility" => {
            let perms: AccessGroupPermissions = serde_json::from_value(a[1].clone()).unwrap();
            json(Ok(plan::visibility(perms)))
        }
        "createVerification" => json(plan::create_verification(
            t,
            &mut r.ctx(),
            ts::new_verification(&a[0]).unwrap(),
            &NoBooks,
        )),
        "createCampaign" => json(plan::create_campaign(
            t,
            &mut r.ctx(),
            ts::new_campaign(&a[0]).unwrap(),
            &|_| false,
        )),
        "updateCampaign" => json(plan::update_campaign(
            t,
            &mut r.ctx(),
            s(&a[0]),
            ts::campaign_patch(&a[2]).unwrap(),
            &|_| false,
        )),
        "deleteCampaign" => plan::delete_campaign(t, &mut r.ctx(), s(&a[0])).map(|()| None),
        "createDonation" => json(plan::record_donation(
            t,
            &mut r.ctx(),
            ts::new_donation(&a[0]).unwrap(),
        )),
        "updateDonationStatus" => {
            let status: PaymentStatus = serde_json::from_value(a[1].clone()).unwrap();
            json(plan::update_donation_status(
                t,
                &mut r.ctx(),
                s(&a[0]),
                status,
                a.get(2).and_then(Value::as_str).map(str::to_owned),
            ))
        }
        "getCampaignStats" => t
            .campaign_stats
            .iter()
            .find(|c| c.campaign_id == s(&a[0]))
            .map(|c| Some(serde_json::to_value(c).unwrap()))
            .ok_or_else(|| "Campaign not found.".to_owned()),
        "createDonorUpdate" => json(plan::post_donor_update(
            t,
            &mut r.ctx(),
            ts::new_donor_update(&a[0]).unwrap(),
        )),
        "markUpdateSent" => json(plan::mark_update_sent(t, &mut r.ctx(), s(&a[0]))),
        other => panic!("no replay for {other}"),
    }
}

/// A group member as the module writes it: the module's invite note.
fn as_module(mut m: GroupMember) -> GroupMember {
    if m.notes.as_deref() == Some(DEMO_INVITE_NOTE) {
        m.notes = Some(INVITE_NOTE.to_owned());
    }
    m
}

/// The demo API's result in the module's shape, for the record each call returns.
fn expected_result(call: &Value, org_id: &str) -> Option<Value> {
    let r = &call["result"];
    let json = |v: Option<Value>| v;
    match s(&call["fn"]) {
        "updateOrganization" => json(ts::organization(r).ok().map(to_json)),
        "createAccessGroup" | "updateAccessGroup" => json(ts::access_group(r).ok().map(to_json)),
        "addGroupMember" | "inviteMemberByEmail" => {
            json(Some(to_json(as_module(ts::group_member(r).unwrap()))))
        }
        "createVerification" => json(ts::verification(r).ok().map(to_json)),
        "createCampaign" | "updateCampaign" => json(ts::campaign(r).ok().map(to_json)),
        "createDonation" | "updateDonationStatus" => json(ts::donation(r).ok().map(to_json)),
        "createDonorUpdate" | "markUpdateSent" => json(ts::donor_update(r).ok().map(to_json)),
        "getUserPermissions" | "getVerificationVisibility" => Some(r.clone()),
        "removeGroupMember" | "deleteAccessGroup" | "deleteCampaign" | "getCampaignStats" => None,
        other => panic!("no expected result for {other} in {org_id}"),
    }
}

/// `getCampaignStats` (`api.ts:1130-1165`) against the module's stat row: counts and the
/// percentage exactly, money in minor units, the mean to the major unit (P4,
/// `rust/fixtures/README.md`), and the two day counts from the stats functions at the frozen now.
fn check_stats(t: &OrgTransparency, stat: &Value, ts_stats: &Value, campaign_id: &str) {
    let minor = |k: &str| stat[k].as_i64().unwrap();
    let major = |k: &str| ts_stats[k].as_f64().unwrap();
    #[expect(
        clippy::cast_precision_loss,
        reason = "fixture amounts are small integers"
    )]
    let as_major = |m: i64| m as f64 / 100.0;
    assert_eq!(
        stat["donation_count"], ts_stats["donation_count"],
        "{campaign_id}"
    );
    assert_eq!(
        stat["unique_donors"], ts_stats["unique_donors"],
        "{campaign_id}"
    );
    assert_eq!(
        stat["funding_percentage"].as_f64(),
        ts_stats["funding_percentage"].as_f64(),
        "{campaign_id}"
    );
    for k in ["total_raised", "largest_donation", "smallest_donation"] {
        assert_eq!(as_major(minor(k)), major(k), "{campaign_id} {k}");
    }
    assert_eq!(
        js_round(as_major(minor("avg_donation"))),
        major("avg_donation"),
        "{campaign_id} avg"
    );
    let c = t.campaigns.iter().find(|c| c.id == campaign_id).unwrap();
    let now = iso_ms(NOW).unwrap();
    assert_eq!(
        days_remaining(c.ends_at.as_deref(), now).map_or(Value::Null, Value::from),
        ts_stats["days_remaining"],
        "{campaign_id} days_remaining"
    );
    assert_eq!(
        Value::from(days_active(&c.starts_at, now)),
        ts_stats["days_active"],
        "{campaign_id} days_active"
    );
}

/// The demo store's delta for one call, applied to the expected records in the module's shape.
/// `transactions` belongs to MOD2 and is skipped.
fn apply_delta(expected: &mut OrgTransparency, delta: &Value, org_id: &str) {
    fn by_id<T>(rows: &mut Vec<T>, d: &Value, id: impl Fn(&T) -> &str, map: impl Fn(&Value) -> T) {
        for removed in d["removed"].as_array().into_iter().flatten() {
            rows.retain(|r| id(r) != removed.as_str().unwrap());
        }
        for changed in d["changed"].as_array().into_iter().flatten() {
            let new = map(changed);
            let at = rows.iter().position(|r| id(r) == id(&new)).unwrap();
            rows[at] = new;
        }
        for added in d["added"].as_array().into_iter().flatten() {
            rows.push(map(added));
        }
    }
    for (key, d) in delta.as_object().unwrap() {
        match key.as_str() {
            "organizations" => {
                for o in d["changed"].as_array().into_iter().flatten() {
                    if o["id"] == org_id {
                        expected.profile = Some(ts::organization(o).unwrap());
                    }
                }
            }
            "accessGroups" => by_id(
                &mut expected.access_groups,
                d,
                |g| &g.id,
                |v| ts::access_group(v).unwrap(),
            ),
            "groupMembers" => by_id(
                &mut expected.group_members,
                d,
                |m| &m.id,
                |v| as_module(ts::group_member(v).unwrap()),
            ),
            "verifications" => by_id(
                &mut expected.verifications,
                d,
                |v| &v.id,
                |v| ts::verification(v).unwrap(),
            ),
            "campaigns" => by_id(
                &mut expected.campaigns,
                d,
                |c| &c.id,
                |v| ts::campaign(v).unwrap(),
            ),
            "donations" => by_id(
                &mut expected.donations,
                d,
                |x| &x.id,
                |v| ts::donation(v).unwrap(),
            ),
            "donorUpdates" => by_id(
                &mut expected.donor_updates,
                d,
                |u| &u.id,
                |v| ts::donor_update(v).unwrap(),
            ),
            "transactions" => {}
            other => panic!("delta of {other}: not a MOD1 collection"),
        }
    }
}

/// Each collection keyed by id, so the comparison ignores order (the demo snapshot sorts by id;
/// the module keeps insertion order).
fn keyed(t: &OrgTransparency) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut put = |kind: &str, id: &str, v: Value| {
        out.insert(format!("{kind}/{id}"), v);
    };
    if let Some(p) = &t.profile {
        put("organization", &p.id, serde_json::to_value(p).unwrap());
    }
    for g in &t.access_groups {
        put("access_group", &g.id, serde_json::to_value(g).unwrap());
    }
    for m in &t.group_members {
        put("group_member", &m.id, serde_json::to_value(m).unwrap());
    }
    for v in &t.verifications {
        put("verification", &v.id, serde_json::to_value(v).unwrap());
    }
    for c in &t.campaigns {
        put("campaign", &c.id, serde_json::to_value(c).unwrap());
    }
    for d in &t.donations {
        put("donation", &d.id, serde_json::to_value(d).unwrap());
    }
    for u in &t.donor_updates {
        put("donor_update", &u.id, serde_json::to_value(u).unwrap());
    }
    out
}

#[test]
fn the_dataset_loads_through_the_module_codec_with_every_record() {
    let t = load(&fixture("transparency-dataset.json"));
    assert_eq!(
        (
            t.access_groups.len(),
            t.group_members.len(),
            t.verifications.len(),
            t.campaigns.len(),
            t.campaign_stats.len(),
            t.donations.len(),
            t.donor_updates.len(),
            t.profile.as_ref().map(|p| p.name.as_str())
        ),
        (5, 8, 21, 4, 4, 25, 3, Some("Demo NGO Kenya"))
    );
}

#[test]
fn the_seed_campaign_figures_match_their_raised_amounts() {
    let t = load(&fixture("transparency-dataset.json"));
    for c in &t.campaigns {
        let stat = t
            .campaign_stats
            .iter()
            .find(|s| s.campaign_id == c.id)
            .unwrap();
        assert_eq!(stat.total_raised, c.current_amount, "{}", c.name);
    }
}

#[test]
fn every_demo_api_call_replays_against_the_planning_functions() {
    let dataset = fixture("transparency-dataset.json");
    let calls = fixture("transparency-calls.json");
    let org_id = s(&dataset["demo_org_id"]).to_owned();
    let mut t = load(&dataset);
    let mut expected = t.clone();
    let mut r = Replay {
        caller: Member {
            key: format!("{org_id}:caller"),
            identity: Identity::from_byte_array([7; 32]),
            org_id: org_id.clone(),
            user_id: s(&dataset["demo_user_id"]).to_owned(),
            name: "Demo User".to_owned(),
            can_post: true,
            can_approve: true,
            can_close: true,
        },
        org_id: org_id.clone(),
        env: FrozenEnv(0),
        other_slugs: dataset["store"]["organizations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["id"] != org_id.as_str())
            .filter_map(|o| o["public_slug"].as_str().map(str::to_owned))
            .collect(),
    };
    let rows_before = to_rows(&org_id, &t).unwrap();
    let mut replayed = 0;
    for (i, call) in calls["calls"].as_array().unwrap().iter().enumerate() {
        let name = format!("call {i} {}", s(&call["fn"]));
        let outcome = run(&mut r, &mut t, call);
        match s(&call["outcome"]) {
            "ok" => {
                let got = outcome.unwrap_or_else(|e| panic!("{name}: refused: {e}"));
                if s(&call["fn"]) == "getCampaignStats" {
                    check_stats(
                        &t,
                        got.as_ref().unwrap(),
                        &call["result"],
                        s(&call["args"][0]),
                    );
                } else {
                    assert_eq!(got, expected_result(call, &org_id), "{name}: result");
                }
            }
            "error" => assert_eq!(
                outcome.unwrap_err(),
                module_error(s(&call["error"]), s(&call["args"][0])),
                "{name}: error"
            ),
            other => panic!("{name}: outcome {other}"),
        }
        apply_delta(&mut expected, &call["delta"], &org_id);
        assert_eq!(
            keyed(&t),
            keyed(&expected),
            "{name}: records after the call"
        );
        replayed += 1;
    }
    assert_eq!(replayed, 47, "every recorded call replayed");

    // The run's changes, written as rows over the dataset's rows, read back as the records.
    let mut rows = rows_before.clone();
    apply_write(&mut rows, plan_write(&org_id, &rows_before, &t).unwrap());
    assert_eq!(
        from_rows(&rows).unwrap(),
        t,
        "the rows after the run read back as its records"
    );
    let visible = plan::caller_visibility(&t, Some(&r.caller));
    assert_eq!(visible, Visibility::Full, "the demo user sits on the Board");
}
