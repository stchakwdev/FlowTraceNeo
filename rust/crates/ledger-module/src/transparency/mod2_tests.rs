//! The MOD2 rules the demo API does not have, so the fixture replay
//! (`tests/transparency_flows.rs`) cannot reach them: who may write, which links, amounts and
//! values a write takes, how flags and the trust row move, what each side of a chain sees, and
//! what the public tables hold.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use ledger_core::env::LedgerEnv;
use ledger_core::types::Timestamp;
use serde_json::json;
use spacetimedb::Identity;

use super::OrgTransparency;
use super::chains::{self, ChainStatus, Chains};
use super::flows::{
    self, EmbedConfigPatch, EntryLinks, FlowNode, NewFlowNode, NewTransaction, NodeType,
    NodeVisibilitySettings, Transaction, TransactionPatch, TransactionStatus,
    TransactionVisibilitySettings,
};
use super::plan::{self, Ctx, LedgerLinks};
use super::public::project;
use super::rows::{OneRow, plan_public_write, public_rows};
use super::trust::{
    self, FlagBatch, FlagState, FlagSubject, ModelSource, NewFlag, Resolution, RubricInput,
};
use super::types::{
    AccessGroup, AccessGroupPermissions, Campaign, CampaignStatus, NodeScope, OrganizationProfile,
};
use super::views::visible_chains;
use crate::tables::Member;

const ORG: &str = "org-1";
const NOW: &str = "2026-02-04T09:00:00.000Z";

struct Clock(u64);

impl LedgerEnv for Clock {
    fn now(&self) -> Timestamp {
        Timestamp::parse(NOW).unwrap()
    }

    fn new_id(&mut self) -> String {
        self.0 += 1;
        format!("id-{}", self.0)
    }
}

/// Books with entry `je-1` and `je-2`, claim `claim-1` and bill `bill-1`.
struct Books;

impl LedgerLinks for Books {
    fn has_claim(&self, id: &str) -> bool {
        id == "claim-1"
    }

    fn has_bill(&self, id: &str) -> bool {
        id == "bill-1"
    }
}

impl EntryLinks for Books {
    fn has_entry(&self, id: &str) -> bool {
        id == "je-1" || id == "je-2"
    }
}

fn member(org_id: &str, user_id: &str, can_approve: bool, can_close: bool) -> Member {
    Member {
        key: format!("{org_id}:{user_id}"),
        identity: Identity::from_byte_array([3; 32]),
        org_id: org_id.to_owned(),
        user_id: user_id.to_owned(),
        name: user_id.to_owned(),
        can_post: true,
        can_approve,
        can_close,
    }
}

fn clerk() -> Member {
    member(ORG, "clerk", false, false)
}

fn approver() -> Member {
    member(ORG, "approver", true, false)
}

fn admin() -> Member {
    member(ORG, "admin", true, true)
}

fn as_caller<T>(
    who: Option<&Member>,
    t: &mut OrgTransparency,
    f: impl FnOnce(&mut OrgTransparency, &mut Ctx<'_>) -> T,
) -> T {
    let mut env = Clock(100);
    let mut c = Ctx {
        org_id: ORG,
        identity: Identity::from_byte_array([3; 32]),
        member: who,
        env: &mut env,
    };
    f(t, &mut c)
}

fn node(id: &str, node_type: NodeType) -> FlowNode {
    FlowNode {
        id: id.to_owned(),
        organization_id: ORG.to_owned(),
        name: id.to_owned(),
        node_type,
        balance: 0,
        metadata: serde_json::Map::new(),
        created_at: NOW.to_owned(),
    }
}

fn txn(id: &str, status: TransactionStatus) -> Transaction {
    Transaction {
        id: id.to_owned(),
        organization_id: ORG.to_owned(),
        from_node_id: Some("n-donor".to_owned()),
        to_node_id: Some("n-project".to_owned()),
        amount: 500_000,
        currency: "KES".to_owned(),
        exchange_rate: None,
        amount_usd: None,
        description: format!("Transaction {id}"),
        category: None,
        reference: None,
        notes: None,
        verification_status: status,
        verification_count: 0,
        transaction_date: "2026-01-15".to_owned(),
        created_by: None,
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
        journal_entry_ids: Vec::new(),
    }
}

fn group(id: &str) -> AccessGroup {
    AccessGroup {
        id: id.to_owned(),
        organization_id: ORG.to_owned(),
        name: id.to_owned(),
        slug: id.to_owned(),
        description: None,
        icon: "users".to_owned(),
        color: "#000000".to_owned(),
        permissions: AccessGroupPermissions::default(),
        node_scope: NodeScope::All,
        is_default: false,
        is_public: false,
        priority: 1,
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
    }
}

/// Two nodes, two transactions (one verified), one group.
fn org() -> OrgTransparency {
    OrgTransparency {
        flow_nodes: vec![
            node("n-donor", NodeType::Donor),
            node("n-project", NodeType::Project),
        ],
        transactions: vec![
            txn("t-1", TransactionStatus::Pending),
            txn("t-2", TransactionStatus::Verified),
        ],
        access_groups: vec![group("g-1")],
        ..OrgTransparency::default()
    }
}

fn new_txn() -> NewTransaction {
    NewTransaction {
        from_node_id: Some("n-donor".to_owned()),
        to_node_id: Some("n-project".to_owned()),
        amount: 250_000,
        currency: "KES".to_owned(),
        exchange_rate: None,
        amount_usd: None,
        description: "Grant tranche".to_owned(),
        category: Some("Funding".to_owned()),
        reference: None,
        notes: None,
        verification_status: TransactionStatus::Pending,
        transaction_date: "2026-02-01".to_owned(),
    }
}

// ---------------------------------------------------------------------------------------------
// Flow nodes and transactions
// ---------------------------------------------------------------------------------------------

#[test]
fn a_stranger_can_neither_add_a_node_nor_record_a_transaction() {
    let mut t = org();
    let input = NewFlowNode {
        name: "Clinic".to_owned(),
        node_type: NodeType::Beneficiary,
        metadata: serde_json::Map::new(),
    };
    assert_eq!(
        as_caller(None, &mut t, |t, c| flows::create_flow_node(t, c, input)).unwrap_err(),
        "You are not a member of organisation org-1."
    );
    assert!(
        as_caller(None, &mut t, |t, c| flows::create_transaction(
            t,
            c,
            new_txn()
        ))
        .is_err()
    );
    assert_eq!(t, org());
}

#[test]
fn a_transaction_is_recorded_by_the_caller_with_no_proofs_and_no_entries() {
    let mut t = org();
    let x = as_caller(Some(&clerk()), &mut t, |t, c| {
        flows::create_transaction(t, c, new_txn())
    })
    .unwrap();
    assert_eq!(
        (
            x.id.as_str(),
            x.created_by.as_deref(),
            x.verification_count,
            x.journal_entry_ids.len()
        ),
        ("id-101", Some("clerk"), 0, 0)
    );
    assert_eq!(t.transactions.len(), 3);
}

#[test]
fn a_transaction_needs_an_amount_a_date_and_the_organisations_nodes() {
    let cases = [
        (
            NewTransaction {
                amount: 0,
                ..new_txn()
            },
            "A transaction amount must be above zero.",
        ),
        (
            NewTransaction {
                transaction_date: "1 Feb 2026".to_owned(),
                ..new_txn()
            },
            "Transaction date 1 Feb 2026 is not a date (YYYY-MM-DD).",
        ),
        (
            NewTransaction {
                to_node_id: Some("n-elsewhere".to_owned()),
                ..new_txn()
            },
            "Flow node n-elsewhere not found.",
        ),
    ];
    for (input, message) in cases {
        let mut t = org();
        assert_eq!(
            as_caller(Some(&clerk()), &mut t, |t, c| flows::create_transaction(
                t, c, input
            ))
            .unwrap_err(),
            message
        );
        assert_eq!(t, org());
    }
}

#[test]
fn an_update_is_checked_like_a_new_transaction_and_keeps_its_proof_count() {
    let mut t = org();
    let patch = TransactionPatch {
        from_node_id: ledger_core::types::Nullish::Value("n-elsewhere".to_owned()),
        ..TransactionPatch::default()
    };
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| flows::update_transaction(
            t, c, "t-1", patch
        ))
        .unwrap_err(),
        "Flow node n-elsewhere not found."
    );
    let refused: Result<TransactionPatch, _> =
        serde_json::from_value(json!({ "verification_count": 9 }));
    assert!(refused.is_err(), "the proof count is not in an update");
}

#[test]
fn entries_link_only_from_the_books_and_to_one_transaction_each() {
    let mut t = org();
    let link = |t: &mut OrgTransparency, id: &str, entries: &[&str]| {
        as_caller(Some(&clerk()), t, |t, c| {
            flows::link_transaction_entries(
                t,
                c,
                id,
                entries.iter().map(|e| (*e).to_owned()).collect(),
                &Books,
            )
        })
    };
    assert_eq!(
        link(&mut t, "t-1", &["je-9"]).unwrap_err(),
        "Journal entry je-9 not found."
    );
    let x = link(&mut t, "t-1", &["je-1", "je-2", "je-1"]).unwrap();
    assert_eq!(x.journal_entry_ids, ["je-1", "je-2"]);
    assert_eq!(
        link(&mut t, "t-2", &["je-2"]).unwrap_err(),
        "Journal entry je-2 already records transaction t-1."
    );
    assert_eq!(
        link(&mut t, "t-1", &[]).unwrap().journal_entry_ids,
        Vec::<String>::new()
    );
    assert!(link(&mut t, "t-2", &["je-2"]).is_ok());
}

// ---------------------------------------------------------------------------------------------
// Visibility and the embed config
// ---------------------------------------------------------------------------------------------

#[test]
fn only_an_admin_or_manager_changes_what_groups_see() {
    let mut t = org();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| flows::set_node_visibility(
            t,
            c,
            "n-project",
            "g-1",
            NodeVisibilitySettings::default()
        ))
        .unwrap_err(),
        "Only members who can close periods can change what access groups see."
    );
    assert!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            flows::remove_transaction_visibility(t, c, "t-1", "g-1")
        })
        .is_err()
    );
}

#[test]
fn visibility_needs_the_organisations_node_or_transaction_and_group() {
    let mut t = org();
    let set_node = |t: &mut OrgTransparency, n: &str, g: &str| {
        as_caller(Some(&admin()), t, |t, c| {
            flows::set_node_visibility(t, c, n, g, NodeVisibilitySettings::default())
        })
    };
    assert_eq!(
        set_node(&mut t, "n-9", "g-1").unwrap_err(),
        "Flow node n-9 not found."
    );
    assert_eq!(
        set_node(&mut t, "n-project", "g-9").unwrap_err(),
        "Access group not found."
    );
    assert_eq!(
        as_caller(Some(&admin()), &mut t, |t, c| {
            flows::set_transaction_visibility(
                t,
                c,
                "t-9",
                "g-1",
                TransactionVisibilitySettings::default(),
            )
        })
        .unwrap_err(),
        "Transaction t-9 not found."
    );
}

#[test]
fn transaction_visibility_is_one_row_per_transaction_and_group() {
    let mut t = org();
    let set = |t: &mut OrgTransparency, s: TransactionVisibilitySettings| {
        as_caller(Some(&admin()), t, |t, c| {
            flows::set_transaction_visibility(t, c, "t-1", "g-1", s)
        })
        .unwrap()
    };
    let first = set(
        &mut t,
        TransactionVisibilitySettings {
            can_view_amount: ledger_core::types::Nullish::Value(false),
            ..TransactionVisibilitySettings::default()
        },
    );
    let second = set(
        &mut t,
        TransactionVisibilitySettings {
            can_view: Some(false),
            ..TransactionVisibilitySettings::default()
        },
    );
    assert_eq!(t.transaction_visibility.len(), 1);
    assert_eq!(first.id, second.id);
    assert_eq!(
        (second.can_view, second.can_view_amount),
        (false, Some(false))
    );
}

#[test]
fn deleting_a_group_drops_what_it_was_shown() {
    let mut t = org();
    as_caller(Some(&admin()), &mut t, |t, c| {
        flows::set_node_visibility(t, c, "n-project", "g-1", NodeVisibilitySettings::default())?;
        flows::set_transaction_visibility(
            t,
            c,
            "t-1",
            "g-1",
            TransactionVisibilitySettings::default(),
        )?;
        plan::delete_access_group(t, c, "g-1")
    })
    .unwrap();
    assert!(t.node_visibility.is_empty() && t.transaction_visibility.is_empty());
}

#[test]
fn the_first_embed_update_makes_the_config_from_the_column_defaults() {
    let mut t = org();
    let patch = EmbedConfigPatch {
        show_amounts: Some(false),
        ..EmbedConfigPatch::default()
    };
    assert!(
        as_caller(Some(&clerk()), &mut t, |t, c| flows::update_embed_config(
            t,
            c,
            patch.clone()
        ))
        .is_err()
    );
    assert_eq!(
        as_caller(Some(&admin()), &mut t, flows::regenerate_embed_token).unwrap_err(),
        "Embed settings not found."
    );
    let made = as_caller(Some(&admin()), &mut t, |t, c| {
        flows::update_embed_config(t, c, patch)
    })
    .unwrap();
    assert_eq!(
        (
            made.show_amounts,
            made.amount_format,
            made.theme.primary_color.as_str(),
            made.visible_node_types.len(),
            !made.embed_token.is_empty() && made.embed_token.chars().all(|c| c.is_ascii_hexdigit())
        ),
        (false, flows::AmountFormat::Ranges, "#6366f1", 4, true)
    );
    let again = as_caller(Some(&admin()), &mut t, |t, c| {
        flows::regenerate_embed_token(t, c)
    })
    .unwrap();
    assert_ne!(again.embed_token, made.embed_token);
}

// ---------------------------------------------------------------------------------------------
// Flags and the trust row
// ---------------------------------------------------------------------------------------------

fn finding(subject_type: FlagSubject, subject_id: &str, key: &str, p: f64) -> NewFlag {
    NewFlag {
        subject_type,
        subject_id: subject_id.to_owned(),
        question_key: key.to_owned(),
        answer: json!(p),
        probability: Some(p),
        confidence: Some(0.9),
    }
}

fn record(t: &mut OrgTransparency, flags: Vec<NewFlag>) -> Result<Vec<trust::Flag>, String> {
    let mut env = Clock(200);
    trust::record_flags(
        t,
        ORG,
        &mut env,
        &Books,
        "service:jev",
        FlagBatch {
            model_version: "jev-2026-10".to_owned(),
            flags,
        },
    )
}

#[test]
fn a_flag_batch_opens_new_flags_and_refreshes_open_ones() {
    let mut t = org();
    let made = record(
        &mut t,
        vec![
            finding(FlagSubject::Transaction, "t-1", "round_amount", 0.7),
            finding(FlagSubject::Bill, "bill-1", "duplicate_invoice", 0.4),
        ],
    )
    .unwrap();
    assert_eq!(made.len(), 2);
    assert!(
        made.iter()
            .all(|f| f.state == FlagState::Open && f.raised_by == "service:jev")
    );
    let again = record(
        &mut t,
        vec![finding(
            FlagSubject::Transaction,
            "t-1",
            "round_amount",
            0.8,
        )],
    )
    .unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(t.flags.len(), 2, "upserted by subject and question");
    assert_eq!(t.flags[0].probability, Some(0.8));
}

#[test]
fn a_flag_batch_never_changes_a_resolved_flag() {
    let mut t = org();
    let f = record(
        &mut t,
        vec![finding(
            FlagSubject::Transaction,
            "t-1",
            "round_amount",
            0.7,
        )],
    )
    .unwrap()
    .remove(0);
    as_caller(Some(&approver()), &mut t, |t, c| {
        trust::resolve_flag(t, c, &f.id, Resolution::Confirmed, None)
    })
    .unwrap();
    let resolved = t.flags[0].clone();
    let written = record(
        &mut t,
        vec![finding(
            FlagSubject::Transaction,
            "t-1",
            "round_amount",
            0.1,
        )],
    )
    .unwrap();
    assert_eq!(written, []);
    assert_eq!(t.flags, [resolved]);
}

#[test]
fn a_flag_batch_checks_its_subjects_and_values() {
    let cases = [
        (
            finding(FlagSubject::Transaction, "t-9", "q", 0.5),
            "Transaction t-9 not found.",
        ),
        (
            finding(FlagSubject::Claim, "claim-9", "q", 0.5),
            "Expense claim claim-9 not found.",
        ),
        (
            finding(FlagSubject::Transaction, "t-1", "q", 1.5),
            "A flag probability must be from 0 to 1.",
        ),
        (
            finding(FlagSubject::Transaction, "t-1", " ", 0.5),
            "A flag needs its question key.",
        ),
    ];
    for (f, message) in cases {
        let mut t = org();
        assert_eq!(record(&mut t, vec![f]).unwrap_err(), message);
        assert_eq!(t.flags, []);
    }
}

#[test]
fn only_an_approver_resolves_a_flag_and_a_dismissal_needs_a_reason() {
    let mut t = org();
    let f = record(
        &mut t,
        vec![finding(FlagSubject::Claim, "claim-1", "split_invoice", 0.6)],
    )
    .unwrap()
    .remove(0);
    let resolve = |t: &mut OrgTransparency, who: &Member, r: Resolution, why: Option<&str>| {
        as_caller(Some(who), t, |t, c| {
            trust::resolve_flag(t, c, &f.id, r, why)
        })
    };
    assert_eq!(
        resolve(&mut t, &clerk(), Resolution::Dismissed, Some("fine")).unwrap_err(),
        "Only members who can approve spend can resolve flags."
    );
    assert_eq!(
        resolve(&mut t, &approver(), Resolution::Dismissed, Some("  ")).unwrap_err(),
        "Say why the flag is dismissed."
    );
    let done = resolve(
        &mut t,
        &approver(),
        Resolution::Dismissed,
        Some(" Two invoices for two deliveries "),
    )
    .unwrap();
    assert_eq!(
        (
            done.state,
            done.reason.as_deref(),
            done.resolved_by.as_deref()
        ),
        (
            FlagState::Dismissed,
            Some("Two invoices for two deliveries"),
            Some("approver")
        )
    );
    assert_eq!(
        resolve(&mut t, &approver(), Resolution::Confirmed, None).unwrap_err(),
        "This flag is already dismissed."
    );
}

#[test]
fn a_confirmed_flag_counts_against_the_score_and_an_open_one_does_not() {
    let mut t = org();
    trust::refresh(&mut t, ORG, NOW, "module");
    let before = t.trust.clone().unwrap();
    let f = record(
        &mut t,
        vec![finding(
            FlagSubject::Transaction,
            "t-1",
            "round_amount",
            0.7,
        )],
    )
    .unwrap()
    .remove(0);
    assert!(
        trust::refresh(&mut t, ORG, NOW, "module"),
        "open flags are an input"
    );
    assert_eq!(t.trust.as_ref().unwrap().score, before.score);
    as_caller(Some(&approver()), &mut t, |t, c| {
        trust::resolve_flag(t, c, &f.id, Resolution::Confirmed, None)
    })
    .unwrap();
    trust::refresh(&mut t, ORG, NOW, "module");
    let after = t.trust.as_ref().unwrap();
    assert_eq!(after.model.confirmed_flags, 1);
    assert!((after.components.model - (before.components.model - 5.0)).abs() < 1e-9);
    assert_eq!(after.score, before.score - 1);
}

#[test]
fn an_automatic_recompute_changes_nothing_when_no_input_moved() {
    let mut t = org();
    assert!(trust::refresh(&mut t, ORG, NOW, "module"));
    let row = t.trust.clone();
    assert!(!trust::refresh(
        &mut t,
        ORG,
        "2026-02-05T09:00:00.000Z",
        "someone"
    ));
    assert_eq!(t.trust, row, "not even the time moves");
    assert_eq!(t.trust_history, []);
}

#[test]
fn a_rubric_makes_the_model_count_and_stays_through_later_recomputes() {
    let mut t = org();
    let input = |scores: &[(&str, f64)]| RubricInput {
        model_version: "jev-2026-10".to_owned(),
        scores: scores.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    };
    assert_eq!(
        trust::record_rubric(&mut t, ORG, NOW, "service:jev", input(&[("q", 101.0)])).unwrap_err(),
        "Rubric score q must be from 0 to 100."
    );
    assert_eq!(
        trust::record_rubric(&mut t, ORG, NOW, "service:jev", input(&[])).unwrap_err(),
        "A rubric needs at least one score."
    );
    let row = trust::record_rubric(
        &mut t,
        ORG,
        NOW,
        "service:jev",
        input(&[("receipts", 90.0), ("timing", 70.0)]),
    )
    .unwrap();
    assert_eq!(
        (
            row.model.source,
            row.model.rubric_mean,
            row.components.model
        ),
        (ModelSource::Rubric, Some(80.0), 80.0)
    );
    t.transactions.push(txn("t-3", TransactionStatus::Verified));
    assert!(trust::refresh(&mut t, ORG, NOW, "clerk"));
    assert_eq!(t.trust.as_ref().unwrap().model.rubric_mean, Some(80.0));
}

#[test]
fn a_recompute_appends_a_snapshot_against_the_latest_one() {
    let mut t = org();
    let mut env = Clock(300);
    let first = trust::recompute(&mut t, ORG, &mut env, "clerk");
    assert_eq!((first.previous_score, first.delta), (0, first.trust_score));
    t.transactions.push(txn("t-3", TransactionStatus::Verified));
    let second = trust::recompute(&mut t, ORG, &mut env, "clerk");
    assert_eq!(second.previous_score, first.trust_score);
    assert_eq!(t.trust_history.len(), 2);
    let h = &t.trust_history[1];
    assert_eq!(
        (h.id.as_str(), h.grade.as_deref(), h.breakdown.len()),
        ("id-302", Some(trust::trust_grade(second.trust_score)), 4)
    );
}

// ---------------------------------------------------------------------------------------------
// Chains
// ---------------------------------------------------------------------------------------------

fn chain_env() -> Clock {
    Clock(400)
}

#[test]
fn a_chain_is_asked_by_a_member_of_the_source_of_another_organisation_with_books() {
    let mut c = Chains::default();
    let mut env = chain_env();
    let source = member("org-a", "asker", false, false);
    let ask = |c: &mut Chains, env: &mut Clock, who: Option<&Member>, target: &str, exists| {
        chains::request_chain(
            c,
            env,
            who,
            "org-a",
            target,
            exists,
            Some("Please".to_owned()),
        )
    };
    assert_eq!(
        ask(&mut c, &mut env, None, "org-b", true).unwrap_err(),
        "You are not a member of organisation org-a."
    );
    assert_eq!(
        ask(&mut c, &mut env, Some(&source), "org-a", true).unwrap_err(),
        "An organisation cannot chain to itself."
    );
    assert_eq!(
        ask(&mut c, &mut env, Some(&source), "org-z", false).unwrap_err(),
        "No ledger for organisation org-z."
    );
    let made = ask(&mut c, &mut env, Some(&source), "org-b", true).unwrap();
    assert_eq!(
        (made.status, made.requested_by.as_str()),
        (ChainStatus::Pending, "asker")
    );
    assert_eq!(
        ask(&mut c, &mut env, Some(&source), "org-b", true).unwrap_err(),
        "There is already a pending or active chain to that organisation."
    );
    let target = member("org-b", "boss", true, true);
    chains::reject_chain(&mut c, &mut env, Some(&target), &made.id, None).unwrap();
    assert!(
        ask(&mut c, &mut env, Some(&source), "org-b", true).is_ok(),
        "may ask again"
    );
}

#[test]
fn only_a_manager_of_the_target_answers_and_grants_only_its_own_group_and_node() {
    let mut c = Chains::default();
    let mut env = chain_env();
    let id = chains::request_chain(
        &mut c,
        &mut env,
        Some(&member("org-a", "asker", true, true)),
        "org-a",
        "org-b",
        true,
        None,
    )
    .unwrap()
    .id;
    let target_records = OrgTransparency {
        access_groups: vec![group("g-b")],
        flow_nodes: vec![node("n-b", NodeType::Project)],
        ..OrgTransparency::default()
    };
    let approve =
        |c: &mut Chains, env: &mut Clock, who: Option<&Member>, g: &str, n: Option<&str>| {
            chains::approve_chain(c, env, who, &target_records, &id, g, n.map(str::to_owned))
        };
    assert_eq!(
        approve(&mut c, &mut env, None, "g-b", None).unwrap_err(),
        "You are not a member of organisation org-b."
    );
    assert_eq!(
        approve(
            &mut c,
            &mut env,
            Some(&member("org-b", "clerk", true, false)),
            "g-b",
            None
        )
        .unwrap_err(),
        "Only members who can close periods can answer for their organisation's chains."
    );
    let boss = member("org-b", "boss", true, true);
    assert_eq!(
        approve(&mut c, &mut env, Some(&boss), "g-a", None).unwrap_err(),
        "Access group g-a not found."
    );
    assert_eq!(
        approve(&mut c, &mut env, Some(&boss), "g-b", Some("n-a")).unwrap_err(),
        "Flow node n-a not found."
    );
    let done = approve(&mut c, &mut env, Some(&boss), "g-b", Some("n-b")).unwrap();
    assert_eq!(
        (
            done.status,
            done.approved_by.as_deref(),
            done.link_node_id.as_deref()
        ),
        (ChainStatus::Active, Some("boss"), Some("n-b"))
    );
    let entry = c.log.last().unwrap();
    assert_eq!(
        (entry.actor_id.as_str(), entry.actor_org_id.as_str()),
        ("boss", "org-b")
    );
}

#[test]
fn a_cancel_needs_a_member_of_the_source_and_leaves_other_chains_alone() {
    let mut c = Chains::default();
    let mut env = chain_env();
    let id = chains::request_chain(
        &mut c,
        &mut env,
        Some(&member("org-a", "asker", false, false)),
        "org-a",
        "org-b",
        true,
        None,
    )
    .unwrap()
    .id;
    assert_eq!(
        chains::cancel_chain_request(&mut c, None, &id).unwrap_err(),
        "You are not a member of organisation org-a."
    );
    chains::cancel_chain_request(&mut c, Some(&member("org-a", "x", false, false)), &id).unwrap();
    assert!(
        c.chains.is_empty() && c.log.is_empty(),
        "the log goes with it"
    );
    assert!(chains::cancel_chain_request(&mut c, None, "chain-missing").is_ok());
}

#[test]
fn each_side_sees_its_chains_once_with_both_names_and_a_stranger_sees_none() {
    let mut c = Chains::default();
    let mut env = chain_env();
    chains::request_chain(
        &mut c,
        &mut env,
        Some(&member("org-a", "asker", false, false)),
        "org-a",
        "org-b",
        true,
        None,
    )
    .unwrap();
    let rows = super::rows::plan_chain_write(&super::rows::ChainRows::default(), &c, 0, 0)
        .unwrap()
        .chains
        .insert;
    let side = |org: &str| OrgTransparency {
        profile: Some(OrganizationProfile {
            id: org.to_owned(),
            name: format!("Name of {org}"),
            org_type: None,
            country: "KE".to_owned(),
            currency: "KES".to_owned(),
            verified: false,
            public_slug: None,
            description: None,
            logo_key: None,
            is_public: false,
            created_at: NOW.to_owned(),
            updated_at: NOW.to_owned(),
        }),
        ..OrgTransparency::default()
    };
    let seen = |mine: &[Member]| {
        visible_chains(
            mine,
            |org| {
                rows.iter()
                    .filter(|r| r.source_org_id == org || r.target_org_id == org)
                    .cloned()
                    .collect()
            },
            side,
        )
    };
    let both = seen(&[
        member("org-a", "x", false, false),
        member("org-b", "x", false, false),
    ]);
    assert_eq!(both.len(), 1, "a member of both sides sees it once");
    let details: chains::ChainDetails = serde_json::from_str(&both[0].data).unwrap();
    assert_eq!(
        (
            details.source_org.name.as_str(),
            details.target_org.name.as_str()
        ),
        ("Name of org-a", "Name of org-b")
    );
    assert_eq!(seen(&[member("org-b", "y", false, false)]).len(), 1);
    assert_eq!(seen(&[member("org-c", "z", false, false)]), []);
}

// ---------------------------------------------------------------------------------------------
// The public projection
// ---------------------------------------------------------------------------------------------

fn public_profile(is_public: bool) -> OrganizationProfile {
    OrganizationProfile {
        id: ORG.to_owned(),
        name: "Demo NGO".to_owned(),
        org_type: None,
        country: "KE".to_owned(),
        currency: "KES".to_owned(),
        verified: true,
        public_slug: Some("demo-ngo".to_owned()),
        description: None,
        logo_key: None,
        is_public,
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
    }
}

fn campaign(id: &str, is_public: bool, status: CampaignStatus, slug: Option<&str>) -> Campaign {
    Campaign {
        id: id.to_owned(),
        organization_id: ORG.to_owned(),
        name: id.to_owned(),
        description: None,
        short_description: None,
        image_url: None,
        target_amount: 1_000_000,
        current_amount: 0,
        currency: "KES".to_owned(),
        target_node_id: Some("n-project".to_owned()),
        status,
        allow_overflow: false,
        overflow_campaign_id: None,
        overflow_consent_message: None,
        starts_at: NOW.to_owned(),
        ends_at: None,
        is_public,
        public_slug: slug.map(str::to_owned),
        category: None,
        tags: None,
        metadata: serde_json::Map::new(),
        created_by: Some("clerk".to_owned()),
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
    }
}

#[test]
fn a_private_organisation_publishes_nothing_but_its_open_public_campaigns() {
    let mut t = org();
    t.profile = Some(public_profile(false));
    t.campaigns = vec![
        campaign("c-open", true, CampaignStatus::Active, Some("open")),
        campaign("c-draft", true, CampaignStatus::Draft, Some("draft")),
        campaign("c-quiet", false, CampaignStatus::Active, Some("quiet")),
        campaign("c-noslug", true, CampaignStatus::Active, None),
    ];
    trust::refresh(&mut t, ORG, NOW, "module");
    let p = project(ORG, &t);
    assert!(p.organization.is_none() && p.badge.is_none());
    assert_eq!(
        p.campaigns
            .iter()
            .map(|c| c.campaign.id.as_str())
            .collect::<Vec<_>>(),
        ["c-open"]
    );
    let text = serde_json::to_string(&p.campaigns[0]).unwrap();
    for private in [
        "\"created_by\"",
        "\"metadata\"",
        "donor_name",
        "donor_email",
        "\"notes\"",
    ] {
        assert!(!text.contains(private), "{private} is not published");
    }
}

#[test]
fn a_public_organisation_publishes_its_profile_score_and_badge() {
    let mut t = org();
    t.profile = Some(public_profile(true));
    trust::refresh(&mut t, ORG, NOW, "module");
    let p = project(ORG, &t);
    let score = t.trust.as_ref().unwrap().score;
    let o = p.organization.unwrap();
    assert_eq!(
        (o.name.as_str(), o.trust_score, o.grade.as_deref()),
        ("Demo NGO", Some(score), Some(trust::trust_grade(score)))
    );
    let b = p.badge.unwrap();
    assert_eq!((b.score, b.breakdown.len()), (score, 4));
}

#[test]
fn the_public_rows_follow_the_projection_and_go_when_the_organisation_turns_private() {
    let mut t = org();
    t.profile = Some(public_profile(true));
    t.campaigns = vec![campaign(
        "c-open",
        true,
        CampaignStatus::Active,
        Some("open"),
    )];
    trust::refresh(&mut t, ORG, NOW, "module");
    let before = public_rows(ORG, &project(ORG, &t)).unwrap();
    let same = plan_public_write(&before, public_rows(ORG, &project(ORG, &t)).unwrap());
    assert!(same.is_empty());
    t.profile = Some(public_profile(false));
    t.campaigns[0].status = CampaignStatus::Paused;
    let w = plan_public_write(&before, public_rows(ORG, &project(ORG, &t)).unwrap());
    assert_eq!((w.organization, w.badge), (OneRow::Delete, OneRow::Delete));
    assert_eq!(w.campaigns.delete, ["c-open"]);
}
