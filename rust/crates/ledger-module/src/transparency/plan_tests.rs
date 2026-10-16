//! The permission and integrity rules of [`super::plan`] that the demo API does not have, so the
//! fixture replay (`tests/transparency_fixtures.rs`) cannot reach them: who may do what, which
//! links and hashes a proof takes, what a non-member may record, and how much of a proof each
//! level shows.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use ledger_core::env::LedgerEnv;
use ledger_core::types::{Nullish, Timestamp};
use spacetimedb::Identity;

use super::OrgTransparency;
use super::plan::{self, Ctx, LedgerLinks};
use super::types::{
    AccessGroupPermissions, AccessType, Campaign, CampaignStatus, GroupMember, NewAccessGroup,
    NewCampaign, NewDonation, NewGroupMember, NewVerification, NodeScope, OrganizationPatch,
    PaymentStatus, Verification, VerificationStatus, VerificationType, Visibility,
};
use crate::tables::Member;

const ORG: &str = "org-1";
const NOW: &str = "2026-02-04T09:00:00.000Z";
const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

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

struct Books;

impl LedgerLinks for Books {
    fn has_claim(&self, id: &str) -> bool {
        id == "claim-1"
    }

    fn has_bill(&self, id: &str) -> bool {
        id == "bill-1"
    }
}

fn person(user_id: &str, can_close: bool) -> Member {
    Member {
        key: format!("{ORG}:{user_id}"),
        identity: Identity::from_byte_array([1; 32]),
        org_id: ORG.to_owned(),
        user_id: user_id.to_owned(),
        name: user_id.to_owned(),
        can_post: true,
        can_approve: false,
        can_close,
    }
}

/// Runs `f` as `member` (or as a non-member with `None`).
fn as_caller<T>(
    member: Option<&Member>,
    t: &mut OrgTransparency,
    f: impl FnOnce(&mut OrgTransparency, &mut Ctx<'_>) -> T,
) -> T {
    let mut env = Clock(0);
    let mut ctx = Ctx {
        org_id: ORG,
        identity: Identity::from_byte_array([9; 32]),
        member,
        env: &mut env,
    };
    f(t, &mut ctx)
}

fn admin() -> Member {
    person("admin", true)
}

fn clerk() -> Member {
    person("clerk", false)
}

fn new_group() -> NewAccessGroup {
    NewAccessGroup {
        name: "Auditors".to_owned(),
        slug: "auditors".to_owned(),
        description: None,
        icon: "users".to_owned(),
        color: "#1C5FA8".to_owned(),
        permissions: AccessGroupPermissions {
            can_view_proofs: true,
            ..AccessGroupPermissions::default()
        },
        node_scope: NodeScope::Assigned,
        is_default: false,
        is_public: false,
        priority: 50,
    }
}

fn proof(object_key: Option<&str>, sha256: Option<&str>) -> NewVerification {
    NewVerification {
        transaction_id: Some("txn-1".to_owned()),
        claim_id: None,
        bill_id: None,
        kind: VerificationType::Receipt,
        object_key: object_key.map(str::to_owned),
        sha256: sha256.map(str::to_owned),
        thumbnail_key: Some("org-1/thumb.jpg".to_owned()),
        ocr_data: None,
        metadata: serde_json::Map::new(),
        status: VerificationStatus::Pending,
        visibility_level: Visibility::Full,
        executor_name: None,
        gps_latitude: None,
        gps_longitude: None,
        gps_accuracy: None,
        location_name: None,
        description: None,
    }
}

fn campaign(status: CampaignStatus, current: i64, ends_at: Option<&str>) -> Campaign {
    Campaign {
        id: "c-1".to_owned(),
        organization_id: ORG.to_owned(),
        name: "Wells".to_owned(),
        description: None,
        short_description: None,
        image_url: None,
        target_amount: 100_000,
        current_amount: current,
        currency: "KES".to_owned(),
        target_node_id: None,
        status,
        allow_overflow: false,
        overflow_campaign_id: None,
        overflow_consent_message: None,
        starts_at: "2026-01-01T00:00:00Z".to_owned(),
        ends_at: ends_at.map(str::to_owned),
        is_public: true,
        public_slug: Some("wells".to_owned()),
        category: None,
        tags: None,
        metadata: serde_json::Map::new(),
        created_by: None,
        created_at: NOW.to_owned(),
        updated_at: NOW.to_owned(),
    }
}

fn gift(status: PaymentStatus, anonymous: bool) -> NewDonation {
    serde_json::from_value(serde_json::json!({
        "campaign_id": "c-1",
        "donor_name": "Ama",
        "is_anonymous": anonymous,
        "amount": 5_000,
        "currency": "KES",
        "payment_status": status,
    }))
    .unwrap()
}

fn with_campaign(c: Campaign) -> OrgTransparency {
    OrgTransparency {
        campaigns: vec![c],
        ..OrgTransparency::default()
    }
}

// Who may write what (policies of flowtrace/supabase/migrations/002, 006, 012).

#[test]
fn a_non_member_cannot_create_a_campaign() {
    let mut t = OrgTransparency::default();
    let input: NewCampaign = serde_json::from_value(serde_json::json!({
        "name": "Wells", "target_amount": 100, "currency": "KES", "starts_at": NOW
    }))
    .unwrap();
    assert_eq!(
        as_caller(None, &mut t, |t, c| plan::create_campaign(
            t,
            c,
            input,
            &|_| false
        ))
        .unwrap_err(),
        "You are not a member of organisation org-1."
    );
}

#[test]
fn any_member_creates_a_campaign_and_its_figures() {
    let mut t = OrgTransparency::default();
    let input: NewCampaign = serde_json::from_value(serde_json::json!({
        "name": "Wells", "target_amount": 100, "currency": "KES", "starts_at": NOW
    }))
    .unwrap();
    let made = as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::create_campaign(t, c, input, &|_| false)
    })
    .unwrap();
    assert_eq!(
        (
            made.created_by.as_deref(),
            made.public_slug,
            t.campaign_stats.len()
        ),
        (Some("clerk"), None, 1)
    );
}

#[test]
fn a_campaign_needs_a_target_above_zero() {
    let mut t = OrgTransparency::default();
    let input: NewCampaign = serde_json::from_value(serde_json::json!({
        "name": "Wells", "target_amount": 0, "currency": "KES", "starts_at": NOW
    }))
    .unwrap();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| plan::create_campaign(
            t,
            c,
            input,
            &|_| false
        ))
        .unwrap_err(),
        "A campaign needs a target above zero."
    );
}

#[test]
fn a_campaign_link_taken_elsewhere_is_refused() {
    let mut t = OrgTransparency::default();
    let input: NewCampaign = serde_json::from_value(serde_json::json!({
        "name": "Wells", "target_amount": 100, "currency": "KES", "starts_at": NOW,
        "public_slug": "wells"
    }))
    .unwrap();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            plan::create_campaign(t, c, input, &|s| s == "wells")
        })
        .unwrap_err(),
        plan::SLUG_TAKEN
    );
}

#[test]
fn a_clerk_cannot_manage_access_groups() {
    let mut t = OrgTransparency::default();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| plan::create_access_group(
            t,
            c,
            new_group()
        ))
        .unwrap_err(),
        "Only members who can close periods can manage access groups."
    );
}

#[test]
fn a_clerk_cannot_change_the_organisation_profile() {
    let mut t = OrgTransparency::default();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            plan::update_organization(t, c, "KES", OrganizationPatch::default(), &|_| false)
        })
        .unwrap_err(),
        "Only members who can close periods can change the organisation profile."
    );
}

#[test]
fn the_first_profile_update_makes_the_profile_in_the_books_currency() {
    let mut t = OrgTransparency::default();
    let patch = OrganizationPatch {
        name: Some("Wells for All".to_owned()),
        public_slug: Nullish::Value("wells-for-all".to_owned()),
        ..OrganizationPatch::default()
    };
    let p = as_caller(Some(&admin()), &mut t, |t, c| {
        plan::update_organization(t, c, "UGX", patch, &|_| false)
    })
    .unwrap();
    assert_eq!(
        (
            p.id.as_str(),
            p.name.as_str(),
            p.currency.as_str(),
            p.public_slug.as_deref()
        ),
        ("org-1", "Wells for All", "UGX", Some("wells-for-all"))
    );
}

#[test]
fn an_admin_gets_the_five_default_groups_once() {
    let mut t = OrgTransparency::default();
    let made = as_caller(Some(&admin()), &mut t, plan::create_default_access_groups).unwrap();
    let slugs: Vec<&str> = made.iter().map(|g| g.slug.as_str()).collect();
    assert_eq!(slugs, ["board", "staff", "donors", "partners", "public"]);
    assert_eq!(
        as_caller(Some(&admin()), &mut t, plan::create_default_access_groups).unwrap_err(),
        "This organisation already has its default access groups."
    );
}

#[test]
fn a_default_group_cannot_be_deleted() {
    let mut t = OrgTransparency::default();
    let made = as_caller(Some(&admin()), &mut t, plan::create_default_access_groups).unwrap();
    assert_eq!(
        as_caller(Some(&admin()), &mut t, |t, c| plan::delete_access_group(
            t,
            c,
            &made[0].id
        ))
        .unwrap_err(),
        "Default access groups cannot be deleted."
    );
}

#[test]
fn a_member_goes_only_into_a_group_of_the_organisation() {
    let mut t = OrgTransparency::default();
    let input = NewGroupMember {
        group_id: "elsewhere".to_owned(),
        user_id: Some("u".to_owned()),
        user_email: None,
        valid_from: None,
        valid_until: None,
        notes: None,
    };
    assert_eq!(
        as_caller(Some(&admin()), &mut t, |t, c| plan::add_group_member(
            t, c, input
        ))
        .unwrap_err(),
        "Access group not found."
    );
}

#[test]
fn granted_by_is_the_caller_not_the_call() {
    let mut t = OrgTransparency::default();
    let g = as_caller(Some(&admin()), &mut t, |t, c| {
        plan::create_access_group(t, c, new_group())
    })
    .unwrap();
    let m: GroupMember = as_caller(Some(&admin()), &mut t, |t, c| {
        plan::invite_group_member(t, c, &g.id, "a@example.org", None)
    })
    .unwrap();
    assert_eq!(
        (
            m.granted_by.as_deref(),
            m.valid_from.as_str(),
            m.notes.as_deref()
        ),
        (Some("admin"), NOW, Some(plan::INVITE_NOTE))
    );
}

// Proofs: links, hashes, visibility (getVerificationVisibility, supabase.ts:573-588).

#[test]
fn a_proof_needs_exactly_one_link() {
    let mut t = OrgTransparency::default();
    let mut input = proof(None, None);
    input.bill_id = Some("bill-1".to_owned());
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| plan::create_verification(
            t, c, input, &Books
        ))
        .unwrap_err(),
        "A proof belongs to one transaction, expense claim or bill."
    );
}

#[test]
fn a_proof_of_an_unknown_claim_is_refused() {
    let mut t = OrgTransparency::default();
    let mut input = proof(None, None);
    input.transaction_id = None;
    input.claim_id = Some("claim-9".to_owned());
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| plan::create_verification(
            t, c, input, &Books
        ))
        .unwrap_err(),
        "Expense claim claim-9 not found."
    );
}

#[test]
fn a_proof_file_needs_its_hash() {
    let mut t = OrgTransparency::default();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            plan::create_verification(t, c, proof(Some("org-1/r.pdf"), None), &Books)
        })
        .unwrap_err(),
        "A proof file needs both its object key and its SHA-256."
    );
}

#[test]
fn an_uppercase_hash_is_refused() {
    let mut t = OrgTransparency::default();
    let upper = HASH.to_uppercase();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            plan::create_verification(t, c, proof(Some("org-1/r.pdf"), Some(&upper)), &Books)
        })
        .unwrap_err(),
        "A file hash must be a SHA-256 in lowercase hex."
    );
}

#[test]
fn a_proof_of_a_bill_with_its_file_is_kept_with_key_hash_and_points() {
    let mut t = OrgTransparency::default();
    let mut input = proof(Some("org-1/r.pdf"), Some(HASH));
    input.transaction_id = None;
    input.bill_id = Some("bill-1".to_owned());
    input.status = VerificationStatus::Verified;
    let v: Verification = as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::create_verification(t, c, input, &Books)
    })
    .unwrap();
    assert_eq!(
        (
            v.object_key.as_deref(),
            v.sha256.as_deref(),
            v.trust_points,
            v.verified_by.as_deref(),
            v.created_by.as_deref()
        ),
        (
            Some("org-1/r.pdf"),
            Some(HASH),
            5,
            Some("clerk"),
            Some("clerk")
        )
    );
}

fn org_with_proof_and_group(perms: AccessGroupPermissions, user: &str) -> OrgTransparency {
    let mut t = OrgTransparency::default();
    let g = as_caller(Some(&admin()), &mut t, |t, c| {
        plan::create_access_group(
            t,
            c,
            NewAccessGroup {
                permissions: perms,
                ..new_group()
            },
        )
    })
    .unwrap();
    as_caller(Some(&admin()), &mut t, |t, c| {
        plan::add_group_member(
            t,
            c,
            NewGroupMember {
                group_id: g.id,
                user_id: Some(user.to_owned()),
                user_email: None,
                valid_from: None,
                valid_until: None,
                notes: None,
            },
        )
    })
    .unwrap();
    as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::create_verification(t, c, proof(Some("org-1/r.pdf"), Some(HASH)), &Books)
    })
    .unwrap();
    t
}

#[test]
fn a_member_in_no_group_does_not_see_proofs() {
    let t = org_with_proof_and_group(AccessGroupPermissions::default(), "someone-else");
    assert_eq!(
        plan::caller_visibility(&t, Some(&clerk())),
        Visibility::Hidden
    );
}

#[test]
fn a_non_member_does_not_see_proofs_whatever_the_groups() {
    let full = AccessGroupPermissions {
        can_view_full_proofs: true,
        ..AccessGroupPermissions::default()
    };
    let t = org_with_proof_and_group(full, "clerk");
    assert_eq!(plan::caller_visibility(&t, None), Visibility::Hidden);
}

#[test]
fn a_thumbnail_viewer_gets_no_original_file() {
    let t = org_with_proof_and_group(
        AccessGroupPermissions {
            can_view_proofs: true,
            ..AccessGroupPermissions::default()
        },
        "clerk",
    );
    let level = plan::caller_visibility(&t, Some(&clerk()));
    let seen = plan::redact(&t.verifications[0], level).unwrap();
    assert_eq!(
        (
            level,
            seen.object_key,
            seen.thumbnail_key.is_some(),
            seen.visible_as
        ),
        (
            Visibility::Thumbnail,
            None,
            true,
            Some(Visibility::Thumbnail)
        )
    );
}

#[test]
fn a_summary_viewer_gets_no_file_at_all() {
    let t = org_with_proof_and_group(
        AccessGroupPermissions {
            can_view_nodes: true,
            ..AccessGroupPermissions::default()
        },
        "clerk",
    );
    let seen = plan::redact(&t.verifications[0], Visibility::Summary).unwrap();
    assert_eq!(
        (
            plan::caller_visibility(&t, Some(&clerk())),
            seen.object_key,
            seen.thumbnail_key
        ),
        (Visibility::Summary, None, None)
    );
}

#[test]
fn a_full_viewer_gets_the_row_as_stored() {
    let t = org_with_proof_and_group(
        AccessGroupPermissions {
            can_view_full_proofs: true,
            ..AccessGroupPermissions::default()
        },
        "clerk",
    );
    let seen = plan::redact(&t.verifications[0], Visibility::Full).unwrap();
    assert_eq!(
        Verification {
            visible_as: None,
            ..seen
        },
        t.verifications[0]
    );
}

#[test]
fn a_hidden_proof_is_left_out() {
    let t = org_with_proof_and_group(AccessGroupPermissions::default(), "clerk");
    assert_eq!(plan::redact(&t.verifications[0], Visibility::Hidden), None);
}

#[test]
fn a_look_at_a_proof_is_logged_at_the_callers_level() {
    let mut t = org_with_proof_and_group(
        AccessGroupPermissions {
            can_view_proofs: true,
            ..AccessGroupPermissions::default()
        },
        "clerk",
    );
    let id = t.verifications[0].id.clone();
    let entry = as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::log_verification_access(t, c, &id, AccessType::Download)
    })
    .unwrap();
    assert_eq!(
        (entry.accessed_by.as_str(), entry.access_level),
        ("clerk", Visibility::Thumbnail)
    );
}

#[test]
fn a_member_who_cannot_see_a_proof_cannot_log_a_look() {
    let mut t = org_with_proof_and_group(AccessGroupPermissions::default(), "someone-else");
    let id = t.verifications[0].id.clone();
    assert_eq!(
        as_caller(Some(&clerk()), &mut t, |t, c| {
            plan::log_verification_access(t, c, &id, AccessType::View)
        })
        .unwrap_err(),
        "You cannot see this proof."
    );
}

// Donations from outside the organisation (policy "Anyone can create donations",
// 012_campaigns_rls.sql:69-79).

#[test]
fn a_non_member_records_a_pending_named_gift_with_their_identity() {
    let mut t = with_campaign(campaign(
        CampaignStatus::Active,
        0,
        Some("2026-12-31T23:59:59Z"),
    ));
    let d = as_caller(None, &mut t, |t, c| {
        plan::record_donation(t, c, gift(PaymentStatus::Pending, false))
    })
    .unwrap();
    assert_eq!(d.donor_profile, Some("09".repeat(32)));
}

#[test]
fn an_anonymous_gift_keeps_no_identity() {
    let mut t = with_campaign(campaign(CampaignStatus::Active, 0, None));
    let d = as_caller(None, &mut t, |t, c| {
        plan::record_donation(t, c, gift(PaymentStatus::Pending, true))
    })
    .unwrap();
    assert_eq!(d.donor_profile, None);
}

#[test]
fn a_non_member_cannot_record_a_confirmed_gift() {
    let mut t = with_campaign(campaign(CampaignStatus::Active, 0, None));
    assert_eq!(
        as_caller(None, &mut t, |t, c| plan::record_donation(
            t,
            c,
            gift(PaymentStatus::Confirmed, false)
        ))
        .unwrap_err(),
        "A donation from outside the organisation is recorded as pending."
    );
}

#[test]
fn a_non_member_cannot_give_to_a_paused_campaign() {
    let mut t = with_campaign(campaign(CampaignStatus::Paused, 0, None));
    assert_eq!(
        as_caller(None, &mut t, |t, c| plan::record_donation(
            t,
            c,
            gift(PaymentStatus::Pending, false)
        ))
        .unwrap_err(),
        "This campaign is not open for donations."
    );
}

#[test]
fn a_non_member_cannot_give_after_the_end() {
    let mut t = with_campaign(campaign(
        CampaignStatus::Active,
        0,
        Some("2026-02-01T00:00:00Z"),
    ));
    assert_eq!(
        as_caller(None, &mut t, |t, c| plan::record_donation(
            t,
            c,
            gift(PaymentStatus::Pending, false)
        ))
        .unwrap_err(),
        "This campaign is not open for donations."
    );
}

#[test]
fn a_non_member_cannot_give_to_a_full_campaign() {
    let mut t = with_campaign(campaign(CampaignStatus::Active, 100_000, None));
    assert_eq!(
        as_caller(None, &mut t, |t, c| plan::record_donation(
            t,
            c,
            gift(PaymentStatus::Pending, false)
        ))
        .unwrap_err(),
        "This campaign is not open for donations."
    );
}

#[test]
fn a_member_records_a_confirmed_gift_to_a_draft_and_it_counts() {
    let mut t = with_campaign(campaign(CampaignStatus::Draft, 0, None));
    as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::record_donation(t, c, gift(PaymentStatus::Confirmed, false))
    })
    .unwrap();
    assert_eq!(
        (
            t.campaigns[0].current_amount,
            t.campaign_stats[0].total_raised
        ),
        (5_000, 5_000)
    );
}

#[test]
fn a_non_member_cannot_change_a_donation_status() {
    let mut t = with_campaign(campaign(CampaignStatus::Active, 0, None));
    let d = as_caller(Some(&clerk()), &mut t, |t, c| {
        plan::record_donation(t, c, gift(PaymentStatus::Pending, false))
    })
    .unwrap();
    assert_eq!(
        as_caller(None, &mut t, |t, c| {
            plan::update_donation_status(t, c, &d.id, PaymentStatus::Confirmed, None)
        })
        .unwrap_err(),
        "You are not a member of organisation org-1."
    );
}

// slugify (api.ts:1068-1078).

#[test]
fn slugify_collapses_runs_and_trims_the_ends() {
    assert_eq!(
        plan::slugify("  Clean Water -- Turkana! ", &|_| false),
        "clean-water-turkana"
    );
}

#[test]
fn slugify_counts_up_from_two() {
    assert_eq!(
        plan::slugify("Wells", &|s| s == "wells" || s == "wells-2"),
        "wells-3"
    );
}

#[test]
fn slugify_of_nothing_usable_is_campaign_and_then_a_bare_number() {
    assert_eq!(
        (
            plan::slugify("!!!", &|_| false),
            plan::slugify("!!!", &|s| s == "campaign")
        ),
        ("campaign".to_owned(), "-2".to_owned())
    );
}

#[test]
fn slugify_drops_non_ascii_letters_as_the_ts_regex_does() {
    assert_eq!(plan::slugify("Café Ñandú", &|_| false), "caf-and");
}
