#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;
use crate::backend::demo_transparency;

const BOARD: &str = "d0eebc99-9c0b-4ef8-bb6d-6bb9bd380d01";
const STAFF: &str = "d0eebc99-9c0b-4ef8-bb6d-6bb9bd380d02";
const NOW: &str = "2026-02-04T09:00:00.000Z";

fn today() -> IsoDate {
    IsoDate::parse("2026-02-04").unwrap()
}

#[test]
fn the_demo_table_reads_the_five_default_groups_in_priority_order() {
    let t = demo_transparency().unwrap();
    let r = rows(&t.access_groups, &t.group_members);
    let names: Vec<&str> = r.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Board of Directors",
            "Staff",
            "Donors",
            "Partners",
            "Public"
        ]
    );
    let board = &r[0];
    assert_eq!(board.kind, "Default");
    assert_eq!(board.sees, "All flows");
    assert_eq!(board.proofs, Cell::Text("Originals"));
    assert_eq!(board.manages, Cell::Text("Members and groups"));
    assert_eq!(board.members, 2);
    assert!(!board.deletable);
    assert_eq!(r[1].proofs, Cell::Text("Summaries"));
    assert_eq!(r[1].manages, Cell::No);
    assert_eq!(r[1].members, 3);
    assert_eq!(r[2].sees, "What their money funds");
    assert_eq!(r[3].sees, "Assigned nodes");
    let public = &r[4];
    assert_eq!(public.kind, "Default \u{b7} Public");
    assert_eq!(public.sees, "All flows, names only");
    assert_eq!(public.amounts, Cell::No);
    assert_eq!(public.proofs, Cell::No);
}

#[test]
fn a_custom_group_and_a_group_that_sees_nothing() {
    let mut g = demo_transparency().unwrap().access_groups[0].clone();
    g.is_default = false;
    g.permissions = AccessGroupPermissions {
        can_manage_members: true,
        ..AccessGroupPermissions::default()
    };
    let row = GroupRow::of(&g, 0);
    assert_eq!(row.kind, "Custom");
    assert_eq!(row.sees, "Nothing");
    assert_eq!(row.manages, Cell::Text("Members"));
    assert!(row.deletable);
}

#[test]
fn member_rows_name_people_and_their_term_newest_first() {
    let t = demo_transparency().unwrap();
    let people = [Person {
        user_id: "user_dummy".to_owned(),
        name: "Achieng Otieno".to_owned(),
    }];
    let board = member_rows(&t.group_members, BOARD, &people, NOW);
    assert_eq!(board.len(), 2);
    assert_eq!(board[0].name, "chair@demo-ngo.org", "newest first");
    assert_eq!(board[0].sub, "Invited since 10 Jun 2025");
    assert_eq!(board[0].chip, "No end date");
    assert_eq!(board[1].name, "Achieng Otieno");
    assert_eq!(
        board[1].sub,
        "demo@flowtrace.dev \u{b7} Member since 1 Jun 2025"
    );

    let unnamed = member_rows(&t.group_members, BOARD, &[], NOW);
    assert_eq!(
        unnamed[1].name, "demo@flowtrace.dev",
        "the address without a name"
    );
    assert_eq!(unnamed[1].sub, "Member since 1 Jun 2025");

    let staff = member_rows(&t.group_members, STAFF, &[], NOW);
    let auditor = &staff[0];
    assert_eq!(auditor.term, Term::Limited);
    assert_eq!(auditor.chip, "Until 31 Mar 2026");
    assert_eq!(
        auditor.notes.as_deref(),
        Some("Annual audit access, expires after fieldwork")
    );
    let later = member_rows(&t.group_members, STAFF, &[], "2026-04-01T00:00:00Z");
    assert_eq!(later[0].term, Term::Expired);
    assert_eq!(later[0].chip, "Expired 31 Mar 2026");
    assert_eq!(members_meta(1), "1 member");
    assert_eq!(members_meta(3), "3 members");
}

#[test]
fn a_member_with_neither_name_nor_email_reads_as_the_user_id() {
    let mut m = demo_transparency().unwrap().group_members[0].clone();
    m.user_email = None;
    assert_eq!(MemberRow::of(&m, &[], NOW).name, "user_dummy");
}

#[test]
fn people_already_in_the_group_are_not_offered() {
    let t = demo_transparency().unwrap();
    let people = [
        Person {
            user_id: "user_dummy".to_owned(),
            name: "Achieng".to_owned(),
        },
        Person {
            user_id: "user_b".to_owned(),
            name: "Baraka".to_owned(),
        },
    ];
    let open = addable(&people, &t.group_members, BOARD);
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].user_id, "user_b");
    assert_eq!(addable(&people, &t.group_members, STAFF).len(), 2);
}

#[test]
fn the_invite_form_checks_as_the_web_does() {
    let t = demo_transparency().unwrap();
    let check =
        |email: &str, until: &str| check_invite(email, until, &t.group_members, BOARD, &today());
    assert_eq!(
        check("  ", "").unwrap_err().email,
        Some("Enter an email address.")
    );
    for bad in ["auditor", "a@b", "a b@c.org", "a@@b.org", "a@.org", "a@b."] {
        assert_eq!(
            check(bad, "").unwrap_err().email,
            Some("That doesn\u{2019}t look like an email address. Check it and try again."),
            "{bad}"
        );
    }
    assert_eq!(
        check("Chair@Demo-NGO.org", "").unwrap_err().email,
        Some("This person is already in the group.")
    );
    assert_eq!(
        check("new@example.org", "3/2/2026")
            .unwrap_err()
            .until
            .as_deref(),
        Some("That date has passed. Choose today or a later day.")
    );
    assert!(
        check("new@example.org", "31/2/2026")
            .unwrap_err()
            .until
            .is_some()
    );
    assert_eq!(
        check(" new@example.org ", "4/2/2026").unwrap(),
        Invite {
            email: "new@example.org".to_owned(),
            valid_until: Some("2026-02-04T23:59:59Z".to_owned()),
        }
    );
    assert_eq!(check("new@example.org", "").unwrap().valid_until, None);
    assert!(is_email("a.b@c.d.e"));
    assert_eq!(
        invited("new@example.org", "Board of Directors"),
        "new@example.org is invited to Board of Directors."
    );
}

#[test]
fn a_new_group_takes_its_short_id_from_the_name_until_one_is_typed() {
    let mut f = GroupForm::new();
    assert_eq!(f.scope, NodeScope::Assigned);
    assert_eq!(f.permissions, NEW_PERMISSIONS);
    f.set_name("  Harambee Grant -- Team! ");
    assert_eq!(f.slug, "harambee-grant-team");
    f.set_slug("Auditors");
    assert_eq!(f.slug, "auditors");
    f.set_name("Something else");
    assert_eq!(f.slug, "auditors", "a typed short id stays");
    assert_eq!(slugify("\u{e9}t\u{e9} 2026"), "t-2026");
    assert_eq!(f.title(), "Create an access group");
    assert_eq!(f.submit_label(false), "Create group");
    assert_eq!(f.submit_label(true), "Creating");
}

#[test]
fn the_group_form_refuses_duplicates_and_bad_ids_with_the_web_texts() {
    let groups = demo_transparency().unwrap().access_groups;
    let mut f = GroupForm::new();
    f.set_name("A");
    assert_eq!(f.validate(&groups).name, Some("Give the group a name."));
    f.set_name("staff");
    assert_eq!(
        f.validate(&groups).name,
        Some("Another group already has this name.")
    );
    assert_eq!(
        f.validate(&groups).slug,
        Some("Another group already uses this ID.")
    );
    f.set_slug("");
    assert_eq!(f.validate(&groups).slug, Some("Enter a short ID."));
    f.set_slug("bad_id");
    assert_eq!(
        f.validate(&groups).slug,
        Some("Use lowercase letters, numbers and single hyphens.")
    );

    let edit = GroupForm::edit(&groups[1]);
    assert!(
        edit.validate(&groups).is_empty(),
        "a group may keep its own name"
    );
    assert_eq!(edit.title(), "Edit Staff");
    assert_eq!(
        edit.description_text(),
        "A default group. You can change its depth, but not delete it."
    );
    assert_eq!(edit.submit_label(false), "Save group");
}

#[test]
fn saving_sends_what_the_web_sends() {
    let mut f = GroupForm::new();
    f.set_name(" Auditors ");
    f.description = "  ".to_owned();
    f.permissions = ProofDepth::Full.apply(f.permissions);
    let g = f.new_group();
    assert_eq!(g.name, "Auditors");
    assert_eq!(g.slug, "auditors");
    assert_eq!(g.description, None);
    assert_eq!((g.icon.as_str(), g.color.as_str()), ("users", "#1C5FA8"));
    assert_eq!((g.is_default, g.is_public, g.priority), (false, false, 50));
    assert!(g.permissions.can_view_proofs && g.permissions.can_view_full_proofs);

    let groups = demo_transparency().unwrap().access_groups;
    let mut edit = GroupForm::edit(&groups[2]);
    edit.description = String::new();
    edit.permissions = Flag::Executor.set(edit.permissions, true);
    let json = serde_json::to_value(edit.patch()).unwrap();
    assert_eq!(json["description"], serde_json::Value::Null);
    assert_eq!(json["node_scope"], "downstream");
    assert_eq!(json["permissions"]["can_view_executor"], true);
    assert!(json.get("priority").is_none(), "only what the form edits");
    assert_eq!(
        ProofDepth::None.apply(edit.permissions).can_view_proofs,
        false
    );
    assert_eq!(
        ProofDepth::Summary.hint(),
        "Type, date and status of each proof."
    );
}

#[test]
fn dialog_texts_follow_the_web() {
    assert_eq!(delete_title("Auditors"), "Delete \u{201c}Auditors\u{201d}?");
    assert_eq!(
        remove_title("chair@demo-ngo.org", "Board of Directors"),
        "Remove chair@demo-ngo.org from Board of Directors?"
    );
    assert_eq!(
        lede(Some("Demo NGO Kenya")),
        "Who sees what in Demo NGO Kenya. Each group sees its own depth of your flows, and FlowTrace holds the line."
    );
    assert_eq!(
        scope_hint(NodeScope::Downstream),
        "The nodes downstream of the money they gave."
    );
}
