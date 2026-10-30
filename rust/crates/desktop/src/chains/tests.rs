#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use ledger_core::demo::DEMO_ORG_ID;
use ledger_module::transparency::chains::GroupRef;

use super::*;
use crate::backend::{InProcessBackend, LedgerBackend as _};

const MAJI: &str = "660e8400-e29b-41d4-a716-446655440001";
const TECH: &str = "660e8400-e29b-41d4-a716-446655440003";
const PWANI: &str = "660e8400-e29b-41d4-a716-446655440004";

fn demo_chains() -> Vec<ChainDetails> {
    InProcessBackend::with_demo_and_empty().unwrap().chains()
}

fn perms(
    nodes: bool,
    amounts: bool,
    proofs: bool,
    full: bool,
    executor: bool,
) -> AccessGroupPermissions {
    AccessGroupPermissions {
        can_view_nodes: nodes,
        can_view_amounts: amounts,
        can_view_proofs: proofs,
        can_view_executor: executor,
        can_view_full_proofs: full,
        can_edit_nodes: false,
        can_edit_transactions: false,
        can_manage_members: false,
        can_manage_groups: false,
    }
}

#[test]
fn describe_access_reads_as_the_web() {
    assert_eq!(
        describe_access(&perms(false, false, false, false, false)),
        "Sees nothing yet."
    );
    assert_eq!(
        describe_access(&perms(true, false, false, false, false)),
        "Sees the flow."
    );
    assert_eq!(
        describe_access(&perms(true, true, false, false, false)),
        "Sees the flow and amounts."
    );
    assert_eq!(
        describe_access(&perms(true, true, true, false, true)),
        "Sees the flow, amounts, proof summaries and who handled each payment."
    );
    assert_eq!(
        describe_access(&perms(false, false, true, true, false)),
        "Sees original proofs.",
        "full proofs win over summaries"
    );
}

#[test]
fn the_demo_splits_into_incoming_and_outgoing_with_the_webs_counts() {
    let v = ChainsView::of(&demo_chains(), DEMO_ORG_ID);
    assert_eq!((v.active, v.awaiting, v.requested, v.ended), (2, 2, 0, 0));
    let names: Vec<(&str, &str)> = v
        .incoming
        .iter()
        .map(|r| (r.source.as_str(), r.target.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            ("TechBridge Solutions", "Demo NGO Kenya"),
            ("Pwani County Health Office", "Demo NGO Kenya"),
        ],
        "partner names, not ids"
    );
    assert_eq!(v.outgoing.len(), 2);
    assert!(v.outgoing.iter().all(|r| r.status == ChainStatus::Active));
    assert_eq!(v.linked.len(), 2);
    assert!(v.linked.contains(&MAJI.to_owned()));
    assert_eq!(links_meta(1), "1 link");
    assert_eq!(links_meta(2), "2 links");
}

#[test]
fn a_row_carries_the_when_line_access_score_and_actions() {
    let chains = demo_chains();
    let v = ChainsView::of(&chains, DEMO_ORG_ID);
    let tech = v
        .incoming
        .iter()
        .find(|r| r.source == "TechBridge Solutions")
        .unwrap();
    assert_eq!(tech.when, "Requested 20 Jan 2026");
    assert_eq!(tech.access, "You choose on approval");
    assert_eq!(tech.access_note, None);
    assert!(
        tech.message
            .as_deref()
            .unwrap()
            .starts_with("We are developing")
    );
    assert_eq!(tech.actions, [ChainAction::Approve, ChainAction::Reject]);
    let demo_score = chains[0].source_org.trust_score;
    assert_eq!(tech.score_note, format!("Average of 78 and {demo_score}"));
    assert_eq!(
        tech.score,
        chain_trust_score(78, demo_score).aggregated_score
    );

    let maji = v
        .outgoing
        .iter()
        .find(|r| r.target == "Maji Safi East Africa")
        .unwrap();
    assert_eq!(maji.when, "Active since 18 Nov 2025");
    assert_eq!(maji.actions, [], "the source cannot revoke");
    assert_eq!(maji.message, None, "the message shows while pending only");
    assert!(!maji.ended);
    assert_eq!(ChainAction::Revoke.label(), "Revoke access");
}

#[test]
fn ended_rows_read_their_dates_and_reason_and_sort_last() {
    let mut chains = demo_chains();
    let pwani = chains
        .iter_mut()
        .find(|c| c.chain.source_org_id == PWANI)
        .unwrap();
    pwani.chain.status = ChainStatus::Rejected;
    pwani.chain.approved_at = Some("2026-02-03T09:00:00Z".to_owned());
    pwani.chain.rejection_reason = Some("Not a grantee.".to_owned());
    let tech = chains
        .iter_mut()
        .find(|c| c.chain.source_org_id == TECH)
        .unwrap();
    tech.chain.status = ChainStatus::Active;
    tech.chain.approved_at = Some("2026-02-02T09:00:00Z".to_owned());
    tech.granted_group = Some(GroupRef {
        id: "g".to_owned(),
        name: "Partners".to_owned(),
        slug: "partners".to_owned(),
        permissions: perms(true, true, false, false, false),
    });
    let v = ChainsView::of(&chains, DEMO_ORG_ID);
    let order: Vec<ChainStatus> = v.incoming.iter().map(|r| r.status).collect();
    assert_eq!(order, [ChainStatus::Active, ChainStatus::Rejected]);
    let rejected = &v.incoming[1];
    assert_eq!(rejected.when, "Rejected 3 Feb 2026");
    assert_eq!(rejected.reason.as_deref(), Some("Not a grantee."));
    assert_eq!(rejected.access, "None");
    assert!(rejected.ended);
    assert_eq!(rejected.actions, []);
    let active = &v.incoming[0];
    assert_eq!(active.access, "Partners");
    assert_eq!(
        active.access_note.as_deref(),
        Some("Sees the flow and amounts.")
    );
    assert_eq!(active.actions, [ChainAction::Revoke]);
    assert_eq!((v.active, v.awaiting, v.ended), (3, 0, 1));
}

#[test]
fn outgoing_pending_says_they_choose_and_offers_cancel() {
    let mut chains = demo_chains();
    let maji = chains
        .iter_mut()
        .find(|c| c.chain.target_org_id == MAJI)
        .unwrap();
    maji.chain.status = ChainStatus::Pending;
    let v = ChainsView::of(&chains, DEMO_ORG_ID);
    let row = &v.outgoing[0];
    assert_eq!(row.access, "They choose on approval");
    assert_eq!(row.actions, [ChainAction::Cancel]);
    assert_eq!(v.requested, 1);
    let c = Confirm::cancel(row);
    assert_eq!(c.title, "Cancel your request to Maji Safi East Africa?");
    assert_eq!((c.keep, c.confirm), ("Keep request", "Cancel request"));
    let r = Confirm::revoke(&v.incoming[0]);
    assert_eq!(r.title, "Revoke TechBridge Solutions's access?");
    assert_eq!((r.keep, r.confirm), ("Keep access", "Revoke access"));
}

#[test]
fn search_needs_two_letters_skips_the_org_itself_and_marks_linked() {
    let backend = InProcessBackend::with_demo_and_empty().unwrap();
    let orgs = backend.public_organizations();
    let linked = vec![MAJI.to_owned()];
    assert_eq!(search(&orgs, " m ", DEMO_ORG_ID, &linked), Search::TooShort);
    let Search::Results(found) = search(&orgs, "ma", DEMO_ORG_ID, &linked) else {
        panic!("Maji Safi matches");
    };
    assert_eq!(found[0].name, "Maji Safi East Africa");
    assert_eq!(found[0].line, "NGO \u{b7} KE \u{b7} Trust Score 92");
    assert!(found[0].linked);
    assert!(matches!(
        search(&orgs, "DEMO", DEMO_ORG_ID, &linked),
        Search::NoMatch(_)
    ));
    assert_eq!(
        search(&orgs, "zzz", DEMO_ORG_ID, &linked),
        Search::NoMatch(
            "No organisation matches \u{201c}zzz\u{201d}. Check the spelling, or ask them to \
             make their profile public on FlowTrace first."
                .to_owned()
        )
    );
    let Search::Results(gov) = search(&orgs, "pwani", DEMO_ORG_ID, &[]) else {
        panic!("Pwani matches");
    };
    assert_eq!(gov[0].line, "Government \u{b7} KE \u{b7} Trust Score 65");
    assert!(!gov[0].linked);
    assert_eq!(request_error("No."), "The request wasn't sent. No.");
}

#[test]
fn the_review_dialog_names_the_requester_and_the_hint_follows_priority() {
    let chains = demo_chains();
    let tech = chains
        .iter()
        .find(|c| c.chain.source_org_id == TECH)
        .unwrap();
    let r = Review::of(tech, ReviewMode::Approve);
    assert_eq!(r.title, "Approve TechBridge Solutions");
    assert_eq!(r.facts[0], ("From", "TechBridge Solutions".to_owned()));
    assert_eq!(r.facts[1], ("Trust Score", "78 of 100".to_owned()));
    assert_eq!(r.facts[2].0, "Their message");
    assert_eq!(r.submit, ("Approve and grant access", "Approving"));
    let r = Review::of(tech, ReviewMode::Reject);
    assert_eq!(r.title, "Reject TechBridge Solutions");
    assert_eq!(r.submit, ("Reject request", "Rejecting"));

    let p = perms(true, false, false, false, false);
    assert_eq!(
        group_hint(true, true, Some(&p)),
        "Choose an access group before approving."
    );
    assert_eq!(
        group_hint(false, true, Some(&p)),
        "Your access groups could not be loaded. Close this and try again."
    );
    assert_eq!(group_hint(false, false, Some(&p)), "Sees the flow.");
    assert_eq!(
        group_hint(false, false, None),
        "This sets what they can see in your flows."
    );
}

#[test]
fn the_lede_names_the_organisation() {
    assert!(lede(Some("Demo NGO Kenya")).starts_with("Demo NGO Kenya and the partners"));
    assert!(lede(None).starts_with("Your organisation and"));
    let backend = InProcessBackend::with_demo_and_empty().unwrap();
    let empty = backend.organisations()[1].id.clone();
    assert_eq!(
        ChainsView::of(&backend.chains(), empty.as_str()),
        ChainsView::default()
    );
}
