#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;
use crate::backend::demo_transparency;

/// A reading time after the demo: 29 Oct 2026, noon UTC.
fn now() -> i64 {
    iso_ms("2026-10-29T12:00:00.000Z").unwrap()
}

fn today() -> IsoDate {
    IsoDate::parse("2026-10-29").unwrap()
}

const TURKANA: &str = "e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01";
const DRAFT: &str = "e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e04";

fn org(t: &OrgTransparency) -> ReportOrganization {
    report_organization(t, ledger_core::demo::DEMO_ORG_ID, "Demo NGO Kenya")
}

fn campaign(t: &OrgTransparency, id: &str) -> Campaign {
    t.campaigns.iter().find(|c| c.id == id).unwrap().clone()
}

#[test]
fn the_list_is_newest_first_with_the_webs_rows_and_figures() {
    let t = demo_transparency().unwrap();
    let list = CampaignList::of(&t, "", StatusFilter::All, now());
    let names: Vec<&str> = list.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Emergency Drought Relief",
            "Maternal Health Kilifi",
            "Clean Water for Turkana",
            "Digital Literacy for Youth"
        ]
    );
    assert_eq!(list.meta, "4 of 4");
    let turkana = &list.rows[2];
    assert_eq!(
        turkana.subtitle,
        "Funds Clean Water Turkana \u{b7} public page"
    );
    assert_eq!(turkana.raised, "KES 3,200,000");
    assert_eq!(turkana.of_target, "of KES 5,000,000 \u{b7} 64%");
    assert_eq!(turkana.timing, "64 days left, ends 31 Dec 2026");
    assert!(!turkana.is_draft);
    let draft = &list.rows[0];
    assert_eq!(draft.subtitle, "Not tied to one project \u{b7} private");
    assert_eq!(draft.timing, "No end date");
    assert!(draft.is_draft);
    assert_eq!(list.rows[3].timing, "Ended 31 Dec 2025");

    let figures: Vec<(&str, &str, Option<&str>)> = list
        .figures
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str(), f.detail.as_deref()))
        .collect();
    assert_eq!(
        figures,
        [
            ("KES 7M", "Raised across campaigns", None),
            ("2", "Open to donors", Some("of 4 campaigns")),
            ("23", "Confirmed donations", None),
            ("1", "Completed", None),
        ]
    );
}

#[test]
fn search_and_status_filter_as_the_web() {
    let t = demo_transparency().unwrap();
    let names = |q: &str, f: StatusFilter| -> Vec<String> {
        CampaignList::of(&t, q, f, now())
            .rows
            .into_iter()
            .map(|r| r.name)
            .collect()
    };
    assert_eq!(
        names("  WATER ", StatusFilter::All),
        ["Emergency Drought Relief", "Clean Water for Turkana"],
        "the drought appeal's story mentions water"
    );
    assert_eq!(
        names("health", StatusFilter::All),
        ["Maternal Health Kilifi"]
    );
    assert_eq!(
        names("", StatusFilter::Only(CampaignStatus::Draft)),
        ["Emergency Drought Relief"]
    );
    let none = CampaignList::of(&t, "", StatusFilter::Only(CampaignStatus::Paused), now());
    assert_eq!(
        none.no_match.as_deref(),
        Some("Nothing matches \u{201c}Paused\u{201d}.")
    );
    assert_eq!(none.meta, "0 of 4");
    let typed = CampaignList::of(&t, "zebra", StatusFilter::All, now());
    assert_eq!(
        typed.no_match.as_deref(),
        Some("Nothing matches \u{201c}zebra\u{201d}.")
    );
    let empty = CampaignList::of(&OrgTransparency::default(), "", StatusFilter::All, now());
    assert!(empty.empty);
    assert_eq!(empty.no_match, None);
    let labels: Vec<&str> = StatusFilter::ALL.iter().map(|f| f.label()).collect();
    assert_eq!(
        labels,
        [
            "All statuses",
            "Active",
            "Drafts",
            "Paused",
            "Completed",
            "Cancelled"
        ]
    );
}

#[test]
fn timing_is_stated_honestly() {
    let t = demo_transparency().unwrap();
    let mut c = campaign(&t, TURKANA);
    c.ends_at = Some("2026-10-30T00:00:00Z".to_owned());
    let last = campaign_timing(&c, now());
    assert_eq!(last.label, "Last day, ends 30 Oct 2026");
    assert_eq!(last.days_left, Some(1));
    c.ends_at = Some("2026-10-29T12:00:00.000Z".to_owned());
    assert_eq!(
        campaign_timing(&c, now()).label,
        "Ended 29 Oct 2026",
        "never 0 days left"
    );
    c.ends_at = None;
    c.status = CampaignStatus::Completed;
    let closed = campaign_timing(&c, now());
    assert!(closed.ended);
    assert_eq!(closed.label, "No end date");
}

#[test]
fn the_new_campaign_form_validates_with_the_webs_texts() {
    let blank = NewCampaignForm::default().validate(&today());
    assert_eq!(
        blank.name.as_deref(),
        Some("Give the campaign a name donors will recognise.")
    );
    assert_eq!(
        blank.target.as_deref(),
        Some("Enter the amount you need to raise.")
    );
    assert_eq!(blank.ends_at, None, "no end date is allowed");

    let form = |target: &str, ends: &str| NewCampaignForm {
        name: "Boreholes".to_owned(),
        target: target.to_owned(),
        ends_at: ends.to_owned(),
        ..NewCampaignForm::default()
    };
    for bad in ["0", "-5", "abc", "1,000"] {
        assert_eq!(
            form(bad, "").validate(&today()).target.as_deref(),
            Some("The target must be more than zero."),
            "{bad}"
        );
    }
    assert_eq!(
        form("500000", "28/10/2026")
            .validate(&today())
            .ends_at
            .as_deref(),
        Some("The end date has passed. Pick today or a later date.")
    );
    assert!(
        form("500000", "29/10/2026")
            .validate(&today())
            .ends_at
            .is_none()
    );
    assert!(
        form("500000", "31/02/2027")
            .validate(&today())
            .ends_at
            .is_some()
    );
}

#[test]
fn the_new_campaign_is_sent_in_minor_units_as_the_web_sends_it() {
    let form = NewCampaignForm {
        name: "  Boreholes for Turkana ".to_owned(),
        description: "  ".to_owned(),
        short_description: "x".repeat(200),
        target: "500000.5".to_owned(),
        target_node: Some("node-1".to_owned()),
        category: Some("water".to_owned()),
        ends_at: "31/12/2026".to_owned(),
        ..NewCampaignForm::default()
    };
    let now = "2026-10-29T12:00:00.000Z";
    let draft = form.to_input(true, &today(), now).unwrap();
    assert_eq!(draft.name, "Boreholes for Turkana");
    assert_eq!(draft.description, None);
    assert_eq!(
        draft.short_description.as_deref().map(str::len),
        Some(SUMMARY_MAX)
    );
    assert_eq!(draft.target_amount, 50_000_050);
    assert_eq!(draft.currency, "KES");
    assert_eq!(draft.status, CampaignStatus::Draft);
    assert_eq!(draft.starts_at, now);
    assert_eq!(draft.ends_at.as_deref(), Some("2026-12-31T23:59:59Z"));
    assert!(draft.is_public && !draft.allow_overflow);
    assert_eq!(draft.public_slug, None);
    let open = form.to_input(false, &today(), now).unwrap();
    assert_eq!(open.status, CampaignStatus::Active);

    // The module reads the same JSON back (`reducers::create_campaign`, `parse`).
    let json = serde_json::to_string(&open).unwrap();
    assert_eq!(serde_json::from_str::<NewCampaign>(&json).unwrap(), open);

    let refused = NewCampaignForm::default()
        .to_input(false, &today(), now)
        .unwrap_err();
    assert!(refused.name.is_some() && refused.target.is_some());
    assert_eq!(
        not_saved("Only members can do that."),
        "The campaign was not saved: Only members can do that."
    );
}

#[test]
fn the_manage_page_matches_the_webs() {
    let t = demo_transparency().unwrap();
    let page = CampaignPage::of(&t, TURKANA, org(&t), now(), ORIGIN_FALLBACK).unwrap();
    assert_eq!(page.funds.as_deref(), Some("Clean Water Turkana"));
    assert!(page.can_receive);
    assert_eq!(page.actions, [StatusAction::Pause]);
    assert_eq!(page.coverage.raised, "KES 3,200,000");
    assert_eq!(page.coverage.target, "KES 5,000,000");
    assert_eq!(page.coverage.percent, "64%");
    let stat = stat_of(&t, TURKANA).unwrap();
    let figures: Vec<(&str, &str)> = page
        .figures
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str()))
        .collect();
    let (count, donors) = (
        stat.donation_count.to_string(),
        stat.unique_donors.to_string(),
    );
    assert_eq!(
        figures,
        [
            (count.as_str(), "Confirmed donations"),
            (donors.as_str(), "Donors"),
            ("KES 290,909", "Average gift"),
            ("64", "Days left, to 31 Dec"),
        ]
    );
    assert_eq!(page.donations.len(), 12);
    let first = &page.donations[0];
    assert_eq!(
        (
            first.donor.as_str(),
            first.date.as_str(),
            first.method.as_str(),
            first.amount.as_str()
        ),
        ("Daniel Mutua", "5 Feb", "M-Pesa", "KES 10,000")
    );
    assert_eq!(payment_label(first.status), "Awaiting payment");
    assert_eq!(page.donations[2].donor, "Anonymous");
    assert_eq!(page.donations[1].method, "Bank transfer");
    let details: Vec<(&str, &str)> = page.details.iter().map(|(k, v)| (*k, v.as_str())).collect();
    assert_eq!(
        details,
        [
            ("Target", "KES 5,000,000"),
            ("Funds", "Clean Water Turkana"),
            ("Category", "Water"),
            ("Started", "1 Dec 2025"),
            ("Ends", "31 Dec 2026"),
            ("Visibility", "Public donate page"),
            ("After target", "Closes to new gifts"),
        ]
    );
    assert_eq!(
        page.donate_url.as_deref(),
        Some("https://flowtrace.app/donate/clean-water-turkana")
    );
    assert!(page.is_open_to_public());
    assert!(page.spending.len() <= 6);
    assert!(
        page.earmarks.first().is_some_and(|e| e.is_target),
        "target first"
    );
}

#[test]
fn a_draft_offers_open_and_delete_and_a_closed_one_nothing() {
    let t = demo_transparency().unwrap();
    let draft = CampaignPage::of(&t, DRAFT, org(&t), now(), ORIGIN_FALLBACK).unwrap();
    assert_eq!(
        draft.actions,
        [StatusAction::Open, StatusAction::DeleteDraft]
    );
    assert!(!draft.can_receive);
    assert_eq!(draft.donate_url, None, "private, no link");
    assert_eq!(
        StatusAction::Open.write(DRAFT),
        CampaignWrite::Update {
            id: DRAFT.to_owned(),
            patch: CampaignPatch {
                status: Some(CampaignStatus::Active),
                ..CampaignPatch::default()
            }
        }
    );
    assert_eq!(
        StatusAction::DeleteDraft.write(DRAFT),
        CampaignWrite::Delete {
            id: DRAFT.to_owned()
        }
    );
    assert_eq!(status_actions(CampaignStatus::Completed), []);
    assert_eq!(
        status_actions(CampaignStatus::Paused),
        [StatusAction::Resume]
    );
    assert_eq!(
        StatusAction::Pause.failed("Offline."),
        "The campaign status did not change: Offline. Try again."
    );
    let figures = &draft.figures;
    assert_eq!(figures[2].value, "KES 0");
    assert_eq!(
        (figures[3].value.as_str(), figures[3].label.as_str()),
        ("Open", "No end date")
    );
}

#[test]
fn coverage_keys_follow_the_progress_bar() {
    let with_proof = Coverage::of(300_000, 500_000, Some(100_000), "KES");
    assert_eq!(
        with_proof.key,
        [
            ("Spent with proof".to_owned(), "KES 1,000".to_owned()),
            (
                "Raised, not yet spent with proof".to_owned(),
                "KES 2,000".to_owned()
            ),
            ("Still to raise".to_owned(), "KES 2,000".to_owned()),
        ]
    );
    let over = Coverage::of(600_000, 500_000, None, "USD");
    assert_eq!(over.percent, "120%");
    assert_eq!(
        over.key,
        [
            ("Raised".to_owned(), "USD 6,000".to_owned()),
            ("Over target".to_owned(), "USD 1,000".to_owned()),
        ]
    );
    assert_eq!(over.bar.total, 600_000);
}

#[test]
fn the_donation_form_validates_and_records_a_pending_gift() {
    let t = demo_transparency().unwrap();
    let c = campaign(&t, TURKANA);
    let blank = DonationForm::default().validate("KES");
    assert_eq!(
        blank.amount.as_deref(),
        Some("Enter how much you would like to give.")
    );
    assert_eq!(
        blank.donor_phone.as_deref(),
        Some("M-Pesa needs the phone number that will receive the payment prompt.")
    );
    assert_eq!(blank.donor_email, None, "email is optional in the app");
    let small = DonationForm {
        amount: "0.5".to_owned(),
        donor_email: "a@b".to_owned(),
        payment_method: "card".to_owned(),
        ..DonationForm::default()
    }
    .validate("KES");
    assert_eq!(small.amount.as_deref(), Some("The smallest gift is KES 1."));
    assert_eq!(
        small.donor_email.as_deref(),
        Some("This email address looks incomplete.")
    );
    assert_eq!(small.donor_phone, None);

    let form = DonationForm {
        amount: "2500".to_owned(),
        donor_name: "  Achieng ".to_owned(),
        donor_phone: "+254 712 345 678".to_owned(),
        earmark: Some("node-1".to_owned()),
        ..DonationForm::default()
    };
    assert_eq!(form.button_label("KES"), "Give KES 2,500");
    let d = form.to_input(&c).unwrap();
    assert_eq!(d.campaign_id, TURKANA);
    assert_eq!(d.amount, 250_000);
    assert_eq!(d.donor_name.as_deref(), Some("Achieng"));
    assert_eq!(d.donor_email, None);
    assert_eq!(d.payment_status, PaymentStatus::Pending);
    assert_eq!(d.payment_method.as_deref(), Some("mpesa"));
    assert_eq!(d.source.as_deref(), Some("website"));
    assert!(d.allow_reallocation);
    let anonymous = DonationForm {
        is_anonymous: true,
        ..form
    }
    .to_input(&c)
    .unwrap();
    assert_eq!(anonymous.donor_name, None);
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(serde_json::from_str::<NewDonation>(&json).unwrap(), d);

    assert_eq!(pledged(250_000, "KES"), "Thank you. KES 2,500 pledged.");
    assert!(pledged_note("mpesa").starts_with("Check your phone for the M-Pesa prompt. Once"));
    assert_eq!(
        confirm_payment("d1"),
        CampaignWrite::DonationStatus {
            id: "d1".to_owned(),
            status: PaymentStatus::Confirmed,
            reference: None
        }
    );
}

#[test]
fn emails_are_checked_as_the_webs_pattern() {
    for good in ["a@b.co", "x.y@mail.example.org", "a@@b.c"] {
        assert!(looks_like_email(good), "{good}");
    }
    for bad in ["a@b", "@b.co", "a@.co", "a@b.", "a b@c.d", ""] {
        assert!(!looks_like_email(bad), "{bad}");
    }
}

/// The literal the web's `CampaignWidgetEmbed` builds for campaign `e01` on
/// `https://flowtrace.app` with the default options and name "Clean Water for Turkana"
/// (`embed-code-generator.tsx:164-186`).
const WEB_SNIPPET: &str = r#"<iframe
  src="https://flowtrace.app/embed/campaign/e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01?theme=light"
  width="100%"
  height="440"
  style="border: 0;"
  loading="lazy"
  title="Clean Water for Turkana on FlowTrace"
></iframe>
<script>
  window.addEventListener('message', function (e) {
    if (e.data && e.data.type === 'flowtrace-embed-height') {
      var frame = document.querySelector('iframe[src*="/embed/campaign/e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01"]');
      if (frame) frame.style.height = e.data.height + 'px';
    }
  });
</script>"#;

#[test]
fn the_embed_snippet_is_the_webs_byte_for_byte() {
    assert_eq!(
        embed_snippet(
            ORIGIN_FALLBACK,
            TURKANA,
            Some("Clean Water for Turkana"),
            EmbedOptions::default()
        ),
        WEB_SNIPPET
    );
    let compact = embed_snippet(
        "https://books.example.org/",
        "c1",
        Some(r#"The "best" campaign"#),
        EmbedOptions {
            dark: true,
            show_amount: false,
            show_donors: false,
            compact: true,
        },
    );
    assert!(compact.contains(
        r#"src="https://books.example.org/embed/campaign/c1?theme=dark&showAmount=false&showDonors=false&compact=true""#
    ));
    assert!(compact.contains(r#"height="80""#));
    assert!(compact.contains(r#"title="The &quot;best&quot; campaign on FlowTrace""#));
    assert!(
        embed_snippet(ORIGIN_FALLBACK, "c1", None, EmbedOptions::default())
            .contains(r#"title="Campaign on FlowTrace""#)
    );
}

#[test]
fn the_donate_link_is_the_donate_page_on_the_web() {
    assert_eq!(
        donate_url("https://flowtrace.app", "clean-water-turkana"),
        "https://flowtrace.app/donate/clean-water-turkana"
    );
    assert_eq!(
        donate_url("http://localhost:3000/", "x"),
        "http://localhost:3000/donate/x"
    );
}

#[test]
fn the_preview_card_reads_as_the_widget() {
    let t = demo_transparency().unwrap();
    let page = CampaignPage::of(&t, TURKANA, org(&t), now(), ORIGIN_FALLBACK).unwrap();
    let card = PreviewCard::of(&page, ORIGIN_FALLBACK);
    assert_eq!(card.organization, "Demo NGO Kenya");
    assert_eq!(card.name, "Clean Water for Turkana");
    assert_eq!(
        card.summary.as_deref(),
        Some("Bringing clean water to 15,000 people in Turkana")
    );
    assert_eq!(card.raised, "KES 3,200,000");
    assert_eq!(card.percent, "64%");
    assert_eq!(card.of_target, "raised of KES 5,000,000");
    assert_eq!(card.compact_line, "KES 3,200,000 \u{b7} 64% of target");
    assert_eq!(card.timing, "64 days left");
    assert_eq!(card.trust_score, t.trust.as_ref().unwrap().score);
    assert_eq!(card.bar.evidenced, page.report.evidenced_amount);
    assert_eq!(
        card.give_url.as_deref(),
        Some("https://flowtrace.app/donate/clean-water-turkana")
    );
    let draft = CampaignPage::of(&t, DRAFT, org(&t), now(), ORIGIN_FALLBACK).unwrap();
    assert_eq!(PreviewCard::of(&draft, ORIGIN_FALLBACK).give_url, None);
}
