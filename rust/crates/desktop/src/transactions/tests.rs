#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;
use crate::backend::demo_transparency;

fn demo() -> OrgTransparency {
    demo_transparency().unwrap()
}

fn day(s: &str) -> IsoDate {
    IsoDate::parse(s).unwrap()
}

#[test]
fn the_demo_list_reads_as_the_web_page() {
    let list = List::of(&demo(), Filter::All, "");
    assert_eq!(
        list.lede,
        "16 movements recorded \u{b7} 3 awaiting proof \u{b7} 1 flagged"
    );
    assert_eq!(Filter::ALL.map(|f| list.counts.of(f)), [16, 12, 3, 1]);
    let first = &list.rows[0];
    assert_eq!(first.date, "4 Feb 2026");
    assert_eq!(
        first.description,
        "Pan-African Education Fund - Education Grant"
    );
    assert_eq!(first.reference, "PAEF-2026-01");
    assert_eq!(first.amount, "6,000,000");
    assert_eq!(first.status, TransactionStatus::Pending);
    assert_eq!(
        list.footer,
        Some(Footer {
            label: "Total: 16 transactions".into(),
            currency: "KES".into(),
            amount: "32,432,500".into(),
        })
    );
    assert_eq!(list.notice, None);
}

#[test]
fn a_filter_and_a_search_narrow_the_rows_and_the_footer_says_shown() {
    let t = demo();
    let flagged = List::of(&t, Filter::Flagged, "");
    assert_eq!(flagged.rows.len(), 1);
    assert_eq!(flagged.rows[0].description, "Unreconciled Cash Withdrawal");
    assert_eq!(
        flagged.footer.as_ref().map(|f| f.label.as_str()),
        Some("Shown: 1 transaction")
    );
    let found = List::of(&t, Filter::All, "  mpesa-2026 ");
    assert!(
        found
            .rows
            .iter()
            .all(|r| r.reference.contains("MPESA-2026")),
        "{:?}",
        found.rows
    );
    assert_ne!(found.rows.len(), 0);
    let by_node = List::of(&t, Filter::All, "harambee");
    assert!(
        by_node
            .rows
            .iter()
            .any(|r| r.from == "Harambee Health Fund")
    );
}

#[test]
fn nothing_matching_and_nothing_recorded_have_their_notices() {
    let t = demo();
    let none = List::of(&t, Filter::Flagged, "zzz");
    assert_eq!(
        none.notice,
        Some(Notice {
            title: "No transactions match".into(),
            body: "Nothing flagged matches \u{201c}zzz\u{201d}.".into(),
            action: "Clear filters",
        })
    );
    assert_eq!(none.footer, None, "no rows, no currency");
    let mut only_verified = t.clone();
    only_verified
        .transactions
        .retain(|x| x.verification_status == TransactionStatus::Verified);
    let empty_status = List::of(&only_verified, Filter::Flagged, "");
    assert_eq!(
        empty_status.notice.unwrap().body,
        "No transactions have this status."
    );
    let empty = List::of(&OrgTransparency::default(), Filter::All, "");
    let notice = empty.notice.unwrap();
    assert_eq!(notice.title, "No transactions recorded yet");
    assert_eq!(notice.action, "Record a transaction");
    assert_eq!(
        empty.lede,
        "0 movements recorded \u{b7} 0 awaiting proof \u{b7} 0 flagged"
    );
}

#[test]
fn two_currencies_shown_have_no_total() {
    let mut t = demo();
    t.transactions[0].currency = "USD".into();
    assert_eq!(List::of(&t, Filter::All, "").footer, None);
}

#[test]
fn filters_take_the_webs_aliases() {
    assert_eq!(Filter::from_alias("evidenced"), Some(Filter::Evidenced));
    assert_eq!(
        Filter::from_alias("Awaiting-Proof"),
        Some(Filter::AwaitingProof)
    );
    assert_eq!(Filter::from_alias("pending"), Some(Filter::AwaitingProof));
    assert_eq!(Filter::from_alias("rejected"), None);
}

#[test]
fn a_rows_missing_nodes_and_reference_read_as_the_web() {
    let mut t = demo();
    t.transactions[0].reference = None;
    t.transactions[0].from_node_id = None;
    let id = t.transactions[0].id.clone();
    let list = List::of(&t, Filter::All, "");
    let row = list.rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(row.reference, "No reference");
    assert_eq!(row.from, "Unassigned");
}

#[test]
fn a_detail_page_lists_its_record_and_movement() {
    let mut t = demo();
    let id = "b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b15";
    let d = Detail::of(&t, id).unwrap();
    assert_eq!(d.title, "Pan-African Education Fund - Education Grant");
    assert_eq!(
        (d.currency.as_str(), d.amount.as_str()),
        ("KES", "6,000,000")
    );
    assert_eq!(d.about_usd, None);
    assert_eq!(d.from.role, "From");
    assert_eq!(d.to.role, "To");
    assert!(d.from.kind.is_some());
    let terms: Vec<&str> = d.facts.iter().map(|f| f.term).collect();
    assert_eq!(
        terms,
        ["Date", "Reference", "Category", "Status", "Recorded"]
    );
    assert_eq!(d.facts[3].value, "Awaiting proof");
    assert_eq!(d.facts[4].value, "4 Feb 2026, 12:00 EAT");
    assert_eq!(d.notes, "First tranche of education support grant");

    let x = t.transactions.iter_mut().find(|x| x.id == id).unwrap();
    x.reference = None;
    x.category = None;
    x.notes = None;
    x.amount_usd = Some(4_650_000);
    x.updated_at = "2026-02-05T08:00:00Z".into();
    x.to_node_id = None;
    let d = Detail::of(&t, id).unwrap();
    assert_eq!(d.about_usd.as_deref(), Some("about USD 46,500"));
    assert_eq!(
        (d.facts[1].value.as_str(), d.facts[1].muted),
        ("None recorded", true)
    );
    assert_eq!(d.facts[2].value, "Uncategorised");
    assert_eq!(d.facts.last().unwrap().term, "Last changed");
    assert_eq!(d.notes, "No notes recorded.");
    assert!(!d.has_notes);
    assert_eq!((d.to.name.as_str(), d.to.kind), ("Not assigned", None));
    assert_eq!(Detail::of(&t, "nope"), None);
}

#[test]
fn failed_uploads_are_counted_in_words() {
    assert_eq!(failed_uploads_title(1), "1 file did not upload");
    assert_eq!(failed_uploads_title(3), "3 files did not upload");
}

#[test]
fn amounts_parse_into_minor_units_from_the_text() {
    assert_eq!(parse_amount("250000"), Some(25_000_000));
    assert_eq!(parse_amount("1,250.50"), Some(125_050));
    assert_eq!(parse_amount(" 1 250.5 "), Some(125_050));
    assert_eq!(parse_amount("0"), Some(0));
    assert_eq!(parse_amount("12.345"), None);
    assert_eq!(parse_amount("12."), None);
    assert_eq!(parse_amount(".5"), None);
    assert_eq!(parse_amount("-4"), None);
    assert_eq!(parse_amount("ten"), None);
    assert_eq!(format_amount_exact(125_050), "1,250.50");
    assert_eq!(format_amount_exact(25_000_000), "250,000");
}

fn filled() -> NewForm {
    NewForm {
        description: "Borehole drilling, phase 1".into(),
        category: "Programs".into(),
        from: "n-1".into(),
        to: "n-2".into(),
        amount: "1,250.50".into(),
        currency: "KES".into(),
        date: "03/02/2026".into(),
        reference: " MP-1 ".into(),
        notes: String::new(),
    }
}

#[test]
fn an_empty_form_lists_every_problem_in_field_order() {
    let today = day("2026-02-04");
    let form = NewForm::new(None, &today);
    assert_eq!(form.currency, "KES");
    assert_eq!(form.date, "04/02/2026");
    let blank = NewForm {
        date: String::new(),
        ..form
    };
    let e = blank.validate(&today);
    assert_eq!(
        e.lines(),
        [
            "Description: Describe what this money was for.",
            "From: Choose where the money came from.",
            "To: Choose where the money went.",
            "Amount: Enter the amount that moved.",
            "Date: Enter the date the money moved.",
        ]
    );
    assert_eq!(e.title(), "Fix these before saving");
}

#[test]
fn each_check_has_the_webs_words() {
    let today = day("2026-02-04");
    let check = |f: NewForm| f.validate(&today).lines();
    assert_eq!(
        check(NewForm {
            description: "ab".into(),
            ..filled()
        }),
        ["Description: Use at least 3 characters so it can be found later."]
    );
    assert_eq!(
        check(NewForm {
            to: "n-1".into(),
            ..filled()
        }),
        ["To: Money must move between two different nodes."]
    );
    assert_eq!(
        check(NewForm {
            amount: "1.234".into(),
            ..filled()
        }),
        ["Amount: Enter a number, like 250000 or 1,250.50."]
    );
    assert_eq!(
        check(NewForm {
            amount: "0.00".into(),
            ..filled()
        }),
        ["Amount: The amount must be more than zero."]
    );
    assert_eq!(
        check(NewForm {
            date: "05/02/2026".into(),
            ..filled()
        }),
        ["Date: The date cannot be in the future."]
    );
    assert_eq!(
        check(NewForm {
            reference: "x".repeat(81),
            ..filled()
        }),
        ["Reference: Keep the reference under 80 characters."]
    );
    let one = NewForm {
        description: String::new(),
        ..filled()
    }
    .validate(&today);
    assert_eq!(one.title(), "Fix this before saving");
}

#[test]
fn a_valid_form_becomes_the_create_input() {
    let today = day("2026-02-04");
    let form = filled();
    assert_eq!(form.amount_hint().as_deref(), Some("KES 1,250.50"));
    let input = form.input(&today).unwrap();
    assert_eq!(input.amount, 125_050);
    assert_eq!(input.transaction_date, "2026-02-03");
    assert_eq!(input.reference.as_deref(), Some("MP-1"));
    assert_eq!(input.notes, None);
    assert_eq!(input.category.as_deref(), Some("Programs"));
    assert_eq!(input.verification_status, TransactionStatus::Pending);
    assert_eq!(
        (input.from_node_id.as_deref(), input.to_node_id.as_deref()),
        (Some("n-1"), Some("n-2"))
    );
    let uncategorised = NewForm {
        category: String::new(),
        ..filled()
    };
    assert_eq!(uncategorised.input(&today).unwrap().category, None);
    assert_eq!(
        NewForm {
            amount: String::new(),
            ..filled()
        }
        .input(&today),
        None
    );
    assert_eq!(
        save_failure("Not a member."),
        "The transaction was not saved: Not a member.. Your entries are still here; try again."
    );
    assert_eq!(save_label(false, false), "Save transaction");
    assert_eq!(save_label(false, true), "Uploading proof\u{2026}");
}

#[test]
fn the_node_lists_group_by_type_in_name_order() {
    let t = demo();
    let groups = node_groups(&t.flow_nodes);
    assert_eq!(groups[0].0, "Departments", "Admin sorts first");
    assert_eq!(groups[0].1[0].1, "Admin");
    assert_eq!(groups[1].0, "Vendors");
    let total: usize = groups.iter().map(|(_, l)| l.len()).sum();
    assert_eq!(total, t.flow_nodes.len());
    let labels: Vec<&str> = groups.iter().map(|(l, _)| *l).collect();
    let mut unique = labels.clone();
    unique.dedup();
    assert_eq!(labels, unique, "each type once");
}

#[test]
fn the_demo_defaults_to_its_latest_movement_and_a_live_org_to_today() {
    let t = demo();
    let today = day("2026-10-30");
    assert_eq!(default_date(Some(&t), true, &today), day("2026-02-04"));
    assert_eq!(default_date(Some(&t), false, &today), today);
    assert_eq!(default_date(None, true, &today), today);
}

#[test]
fn the_saved_transaction_is_found_when_the_records_come_back() {
    let mut t = demo();
    let before = transaction_ids(Some(&t));
    let saved = filled().input(&day("2026-02-04")).unwrap();
    assert_eq!(added_transaction(&before, &t, &saved), None);
    let mut row = t.transactions[0].clone();
    row.id = "new-1".into();
    row.description.clone_from(&saved.description);
    row.amount = saved.amount;
    row.transaction_date.clone_from(&saved.transaction_date);
    t.transactions.push(row);
    assert_eq!(
        added_transaction(&before, &t, &saved).as_deref(),
        Some("new-1")
    );
}
