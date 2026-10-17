//! The demo API's MOD2 calls, replayed against the module.
//!
//! `rust/fixtures/transparency-flows.json` is a scripted run of the demo API
//! (`flowtrace/src/lib/demo/api.ts`) over `transparency-dataset.json` for flow nodes,
//! transactions, proof counting, node visibility, the embed config, chains and the trust score,
//! exported by `rust/tools/src/transparency-flows.ts` under the same frozen clock and counted ids
//! as MOD1's. The dataset is loaded through the module's TS mapping and row codec; each call runs
//! through the module's rules with the same arguments, ids and time, and its result, its error and
//! the records it leaves must equal the demo API's in the module's shape (minor units). The trust
//! row is computed again after every call, as the reducers do, and must never write a history
//! row of its own. The demo API checks no permissions, so the caller is a member who can close
//! periods in every organisation of the dataset.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ledger_core::env::LedgerEnv;
use ledger_core::money::from_major;
use ledger_core::types::Timestamp;
use ledger_module::tables::Member;
use ledger_module::transparency::chains::{self, ChainActivity, Chains};
use ledger_module::transparency::flows::{self, EntryLinks, NodeRef, TransactionWithNodes};
use ledger_module::transparency::plan::{Ctx, LedgerLinks, no_org};
use ledger_module::transparency::public::report_organization;
use ledger_module::transparency::rows::{
    ChainRows, apply_write, chains_from_rows, from_rows, plan_chain_write, plan_write, to_rows,
};
use ledger_module::transparency::trust::{self, TrustHistory};
use ledger_module::transparency::{OrgTransparency, ts};
use serde_json::{Map, Value};
use spacetimedb::Identity;

const NOW: &str = "2026-02-04T09:00:00.000Z";

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/{name}"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The exporter's frozen env (`transparency-flows.json#/env`).
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

/// No books behind the replay: no entries, claims or bills.
struct NoBooks;

impl LedgerLinks for NoBooks {
    fn has_claim(&self, _: &str) -> bool {
        false
    }

    fn has_bill(&self, _: &str) -> bool {
        false
    }
}

impl EntryLinks for NoBooks {
    fn has_entry(&self, _: &str) -> bool {
        false
    }
}

struct Replay {
    org_id: String,
    user_id: String,
    env: FrozenEnv,
    /// Every other organisation's stored trust score (`Organization.trust_score`).
    other_scores: BTreeMap<String, i64>,
}

impl Replay {
    fn member(&self, org_id: &str) -> Member {
        Member {
            key: format!("{org_id}:caller"),
            identity: Identity::from_byte_array([7; 32]),
            org_id: org_id.to_owned(),
            user_id: self.user_id.clone(),
            name: "Demo User".to_owned(),
            can_post: true,
            can_approve: true,
            can_close: true,
        }
    }

    fn with_ctx<T>(&mut self, f: impl FnOnce(&mut Ctx<'_>) -> T) -> T {
        let m = self.member(&self.org_id.clone());
        let mut c = Ctx {
            org_id: &self.org_id,
            identity: m.identity,
            member: Some(&m),
            env: &mut self.env,
        };
        f(&mut c)
    }

    fn score_of(&self, t: &OrgTransparency, org_id: &str) -> i64 {
        if org_id == self.org_id {
            t.trust.as_ref().unwrap().score
        } else {
            self.other_scores[org_id]
        }
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}

fn to_json<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// Money keys of the reads, in major units in the TS: to minor units, recursively.
fn minor(v: &Value, keys: &[&str]) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| {
                    let x = match x.as_f64() {
                        Some(major) if keys.contains(&k.as_str()) => {
                            Value::from(from_major(major).unwrap())
                        }
                        _ => minor(x, keys),
                    };
                    (k.clone(), x)
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(|x| minor(x, keys)).collect()),
        other => other.clone(),
    }
}

/// A module history row as the TS writes it: without the module's model score, grade and
/// `model` breakdown entry.
fn as_ts_history(h: &TrustHistory) -> Value {
    let mut h = h.clone();
    h.model_score = None;
    h.grade = None;
    h.breakdown.remove("model");
    to_json(h)
}

/// A TS transaction with nodes in the module's shape.
fn with_nodes(v: &Value) -> TransactionWithNodes {
    let node = |n: &Value| -> Option<NodeRef> { serde_json::from_value(n.clone()).ok() };
    TransactionWithNodes {
        transaction: ts::transaction(v).unwrap(),
        from_node: node(&v["from_node"]),
        to_node: node(&v["to_node"]),
    }
}

/// The module's outcome of one call as the JSON of what it returns, or the error text.
#[expect(clippy::too_many_lines, reason = "one arm per demo API call")]
fn run(
    r: &mut Replay,
    t: &mut OrgTransparency,
    c: &mut Chains,
    call: &Value,
) -> Result<Value, String> {
    let a = &call["args"];
    let other_org = |i: usize| (s(&a[i]) != r.org_id).then(|| no_org(s(&a[i])));
    match s(&call["fn"]) {
        "createNode" => r
            .with_ctx(|x| flows::create_flow_node(t, x, ts::new_flow_node(&a[0]).unwrap()))
            .map(to_json),
        "createTransaction" => r
            .with_ctx(|x| flows::create_transaction(t, x, ts::new_transaction(&a[0]).unwrap()))
            .map(to_json),
        "updateTransaction" => r
            .with_ctx(|x| {
                flows::update_transaction(t, x, s(&a[0]), ts::transaction_patch(&a[2]).unwrap())
            })
            .map(to_json),
        "createVerification" => r
            .with_ctx(|x| {
                ledger_module::transparency::plan::create_verification(
                    t,
                    x,
                    ts::new_verification(&a[0]).unwrap(),
                    &NoBooks,
                )
            })
            .map(to_json),
        "getFlowDataAPI" => Ok(to_json(flows::flow_data(
            t,
            a.get(1).and_then(Value::as_str),
            a.get(2).and_then(Value::as_str),
            "KES",
        ))),
        "getOrganizationStats" => Ok(to_json(flows::org_stats(t))),
        "setNodeVisibility" => r
            .with_ctx(|x| {
                flows::set_node_visibility(
                    t,
                    x,
                    s(&a[0]),
                    s(&a[1]),
                    ts::node_visibility_settings(&a[2]).unwrap(),
                )
            })
            .map(to_json),
        "removeNodeVisibility" => r
            .with_ctx(|x| flows::remove_node_visibility(t, x, s(&a[0]), s(&a[1])))
            .map(|()| Value::Null),
        "updatePublicEmbedConfig" => match other_org(0) {
            Some(e) => Err(e),
            None => r
                .with_ctx(|x| flows::update_embed_config(t, x, ts::embed_patch(&a[1]).unwrap()))
                .map(to_json),
        },
        "requestChain" => {
            let caller = r.member(s(&a[0]));
            chains::request_chain(
                c,
                &mut r.env,
                Some(&caller),
                s(&a[0]),
                s(&a[1]),
                true,
                a.get(3).and_then(Value::as_str).map(str::to_owned),
            )
            .map(to_json)
        }
        "approveChain" => {
            let caller = target_member(r, c, s(&a[0]));
            chains::approve_chain(
                c,
                &mut r.env,
                caller.as_ref(),
                t,
                s(&a[0]),
                s(&a[2]),
                a.get(3).and_then(Value::as_str).map(str::to_owned),
            )
            .map(to_json)
        }
        "rejectChain" => {
            let caller = target_member(r, c, s(&a[0]));
            chains::reject_chain(
                c,
                &mut r.env,
                caller.as_ref(),
                s(&a[0]),
                a.get(2).and_then(Value::as_str).map(str::to_owned),
            )
            .map(to_json)
        }
        "revokeChain" => {
            let caller = target_member(r, c, s(&a[0]));
            chains::revoke_chain(c, &mut r.env, caller.as_ref(), s(&a[0])).map(to_json)
        }
        "updateChainPermissions" => {
            let caller = target_member(r, c, s(&a[0]));
            chains::update_chain_permissions(
                c,
                &mut r.env,
                caller.as_ref(),
                t,
                s(&a[0]),
                s(&a[1]),
                a.get(2).and_then(Value::as_str).map(str::to_owned),
            )
            .map(to_json)
        }
        "cancelChainRequest" => {
            let source = c
                .chains
                .iter()
                .find(|x| x.id == s(&a[0]))
                .map(|x| r.member(&x.source_org_id));
            chains::cancel_chain_request(c, source.as_ref(), s(&a[0])).map(|()| Value::Null)
        }
        "getChainCounts" => Ok(to_json(chains::chain_counts(&c.chains, s(&a[0])))),
        "getChainActivityLog" => Ok(to_json(chains::activity_of(&c.log, s(&a[0])))),
        "getChainTrustScore" => Ok(c.chains.iter().find(|x| x.id == s(&a[0])).map_or(
            Value::Null,
            |x| {
                to_json(chains::chain_trust_score(
                    r.score_of(t, &x.source_org_id),
                    r.score_of(t, &x.target_org_id),
                ))
            },
        )),
        "calculateTrustScoreAPI" => match other_org(0) {
            Some(e) => Err(e),
            None => {
                let org = r.org_id.clone();
                let user = r.user_id.clone();
                Ok(to_json(trust::recompute(t, &org, &mut r.env, &user)))
            }
        },
        "getTrustScoreHistory" => {
            let limit = a.get(1).and_then(Value::as_u64).unwrap_or(12);
            Ok(Value::Array(
                trust::history(t, usize::try_from(limit).unwrap())
                    .iter()
                    .map(as_ts_history)
                    .collect(),
            ))
        }
        "getCampaignReport" => Ok(to_json(
            flows::campaign_report(t, s(&a[0]), report_organization(&r.org_id, t)).unwrap(),
        )),
        other => panic!("no replay for {other}"),
    }
}

/// The caller's member row in the target of a chain (any status).
fn target_member(r: &Replay, c: &Chains, chain_id: &str) -> Option<Member> {
    c.chains
        .iter()
        .find(|x| x.id == chain_id)
        .map(|x| r.member(&x.target_org_id))
}

/// The demo API's result in the module's shape.
fn expected_result(call: &Value) -> Value {
    let v = &call["result"];
    match s(&call["fn"]) {
        "createNode" => to_json(ts::flow_node(v).unwrap()),
        "createTransaction" | "updateTransaction" => to_json(ts::transaction(v).unwrap()),
        "createVerification" => to_json(ts::verification(v).unwrap()),
        "getFlowDataAPI" => minor(
            v,
            &["inflow", "outflow", "balance", "value", "total_flow"],
        ),
        "setNodeVisibility" => to_json(ts::node_visibility(v).unwrap()),
        "updatePublicEmbedConfig" => to_json(ts::embed_config(v).unwrap()),
        "requestChain" | "approveChain" | "rejectChain" | "revokeChain"
        | "updateChainPermissions" => to_json(ts::chain(v).unwrap()),
        "getChainActivityLog" => Value::Array(
            v.as_array()
                .unwrap()
                .iter()
                .map(|l| to_json(ts::chain_activity(l).unwrap()))
                .collect::<Vec<_>>(),
        ),
        "getTrustScoreHistory" => Value::Array(
            v.as_array()
                .unwrap()
                .iter()
                .map(|h| as_ts_history(&ts::trust_history(h).unwrap()))
                .collect(),
        ),
        "getCampaignReport" => {
            let mut out = minor(v, &["evidenced_amount", "received", "evidenced"]);
            let campaign = &v["campaign"];
            let mut c = to_json(ts::campaign(campaign).unwrap());
            c["target_node"] = campaign["target_node"].clone();
            out["campaign"] = c;
            out["spending"] = Value::Array(
                v["spending"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| to_json(with_nodes(x)))
                    .collect(),
            );
            out
        }
        _ => v.clone(),
    }
}

/// The demo's text for an error, as the module words it: a call for an organisation the module
/// has no books for says so (`plan::no_org`).
fn module_error(call: &Value, org_id: &str) -> String {
    let first = call["args"][0].as_str().unwrap_or_default();
    let by_org = matches!(
        s(&call["fn"]),
        "updatePublicEmbedConfig" | "calculateTrustScoreAPI"
    );
    if by_org && first != org_id {
        return no_org(first);
    }
    call["error"]
        .as_str()
        .unwrap()
        .replace(" in the demo data.", ".")
}

/// The store's delta for one call, applied to the expected records and chains.
fn apply_delta(
    expected: &mut OrgTransparency,
    chains_expected: &mut Chains,
    delta: &Value,
    org_id: &str,
) -> Option<i64> {
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
    let mut score = None;
    for (key, d) in delta.as_object().unwrap() {
        match key.as_str() {
            "organizations" => {
                for o in d["changed"].as_array().into_iter().flatten() {
                    assert_eq!(o["id"], org_id, "only the demo organisation changes");
                    score = o["trust_score"].as_i64();
                }
            }
            "flowNodes" => by_id(
                &mut expected.flow_nodes,
                d,
                |n| &n.id,
                |v| ts::flow_node(v).unwrap(),
            ),
            "transactions" => by_id(
                &mut expected.transactions,
                d,
                |x| &x.id,
                |v| ts::transaction(v).unwrap(),
            ),
            "verifications" => by_id(
                &mut expected.verifications,
                d,
                |v| &v.id,
                |v| ts::verification(v).unwrap(),
            ),
            "nodeVisibility" => by_id(
                &mut expected.node_visibility,
                d,
                |v| &v.id,
                |v| ts::node_visibility(v).unwrap(),
            ),
            "trustScoreHistory" => by_id(
                &mut expected.trust_history,
                d,
                |h| &h.id,
                |v| ts::trust_history(v).unwrap(),
            ),
            "publicEmbedConfigs" => {
                for c in d["changed"].as_array().into_iter().flatten() {
                    expected.embed = Some(ts::embed_config(c).unwrap());
                }
            }
            "organizationChains" => by_id(
                &mut chains_expected.chains,
                d,
                |c| &c.id,
                |v| ts::chain(v).unwrap(),
            ),
            "chainActivityLog" => by_id(
                &mut chains_expected.log,
                d,
                |l: &ChainActivity| &l.id,
                |v| ts::chain_activity(v).unwrap(),
            ),
            other => panic!("delta of {other}: not a MOD2 collection"),
        }
    }
    score
}

/// The MOD2 records and the chains keyed by id, so the comparison ignores order.
fn keyed(t: &OrgTransparency, c: &Chains) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut put = |kind: &str, id: &str, v: Value| {
        out.insert(format!("{kind}/{id}"), v);
    };
    for n in &t.flow_nodes {
        put("flow_node", &n.id, to_json(n));
    }
    for x in &t.transactions {
        put("transaction", &x.id, to_json(x));
    }
    for v in &t.verifications {
        put("verification", &v.id, to_json(v));
    }
    for v in &t.node_visibility {
        put("node_visibility", &v.id, to_json(v));
    }
    for h in &t.trust_history {
        put("trust_history", &h.id, as_ts_history(h));
    }
    if let Some(e) = &t.embed {
        put("embed", &e.id, to_json(e));
    }
    for x in &c.chains {
        put("chain", &x.id, to_json(x));
    }
    for l in &c.log {
        put("chain_log", &l.id, to_json(l));
    }
    out
}

#[test]
fn the_dataset_loads_the_mod2_records_through_the_module_codec() {
    let dataset = fixture("transparency-dataset.json");
    let org = s(&dataset["demo_org_id"]);
    let t = ts::dataset(org, &dataset["store"]).unwrap();
    let back = from_rows(&to_rows(org, &t).unwrap()).unwrap();
    assert_eq!(back, t, "the dataset round-trips through the rows");
    let c = ts::chains(org, &dataset["store"]).unwrap();
    let rows = plan_chain_write(&ChainRows::default(), &c, 0, 0).unwrap();
    let read = chains_from_rows(&ChainRows {
        chains: rows.chains.insert,
        log: rows.log.insert,
    })
    .unwrap();
    assert_eq!(read, c, "the chains round-trip through their rows");
    assert_eq!(
        (
            t.flow_nodes.len(),
            t.transactions.len(),
            t.node_visibility.len(),
            t.trust_history.len(),
            t.embed.is_some(),
            c.chains.len(),
            c.log.len()
        ),
        (22, 16, 1, 8, true, 4, 6)
    );
}

#[test]
fn every_mod2_demo_api_call_replays_against_the_module_rules() {
    let dataset = fixture("transparency-dataset.json");
    let calls = fixture("transparency-flows.json");
    let org_id = s(&dataset["demo_org_id"]).to_owned();
    let store = &dataset["store"];
    let mut t = from_rows(&to_rows(&org_id, &ts::dataset(&org_id, store).unwrap()).unwrap()).unwrap();
    let mut c = ts::chains(&org_id, store).unwrap();
    trust::refresh(&mut t, &org_id, NOW, "module");
    let mut expected = t.clone();
    let mut expected_chains = c.clone();
    let mut r = Replay {
        org_id: org_id.clone(),
        user_id: s(&dataset["demo_user_id"]).to_owned(),
        env: FrozenEnv(0),
        other_scores: store["organizations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| (s(&o["id"]).to_owned(), o["trust_score"].as_i64().unwrap()))
            .collect(),
    };
    let rows_before = to_rows(&org_id, &t).unwrap();
    let mut replayed = 0;
    for (i, call) in calls["calls"].as_array().unwrap().iter().enumerate() {
        let name = format!("call {i} {}", s(&call["fn"]));
        let history_before = t.trust_history.len();
        let outcome = run(&mut r, &mut t, &mut c, call);
        if s(&call["fn"]) != "calculateTrustScoreAPI" {
            trust::refresh(&mut t, &org_id, NOW, "user_dummy");
            assert_eq!(
                t.trust_history.len(),
                history_before,
                "{name}: an automatic recompute writes no history row"
            );
        }
        match s(&call["outcome"]) {
            "ok" => assert_eq!(
                outcome.unwrap_or_else(|e| panic!("{name}: refused: {e}")),
                expected_result(call),
                "{name}: result"
            ),
            "error" => assert_eq!(
                outcome.unwrap_err(),
                module_error(call, &org_id),
                "{name}: error"
            ),
            other => panic!("{name}: outcome {other}"),
        }
        if let Some(score) = apply_delta(&mut expected, &mut expected_chains, &call["delta"], &org_id)
        {
            assert_eq!(t.trust.as_ref().unwrap().score, score, "{name}: trust score");
        }
        assert_eq!(
            keyed(&t, &c),
            keyed(&expected, &expected_chains),
            "{name}: records after the call"
        );
        replayed += 1;
    }
    assert_eq!(replayed, 45, "every recorded call replayed");

    // The run's changes, written as rows over the dataset's rows, read back as the records.
    let mut rows = rows_before.clone();
    apply_write(&mut rows, plan_write(&org_id, &rows_before, &t).unwrap());
    assert_eq!(from_rows(&rows).unwrap(), t, "the rows read back as the records");
}
