//! Read access to the transparency tables: per-caller `my_*` views, like the ledger's
//! (`crate::views`).
//!
//! Seven are query views over membership: the rows of every organisation the caller is a member
//! of, which is what the Supabase policies give an organisation's users (`organization_id =
//! get_current_org_id()`, `flowtrace/supabase/migrations/002_rls_policies.sql`, `006`, `012`).
//! Three are procedural views that read the JSON
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.view.html>, `Vec<T>` returns):
//!
//! - [`my_verification`]: every proof of the caller's organisations, cut down to what their
//!   access groups let them see ([`crate::transparency::plan::redact`] at
//!   [`crate::transparency::plan::caller_visibility`], i.e. `getVerificationVisibility`,
//!   `flowtrace/src/lib/supabase.ts:573-588`); hidden ones are left out.
//! - [`my_verification_access_log`]: the caller's own looks at proofs (policy "Users can view own
//!   access logs", `008_trust_points.sql:121-123`).
//! - [`my_profile`]: the caller's profile and those of everyone who shares an organisation with
//!   them (policies "Users can view profiles in own organization" and "Users can view own
//!   profile", `002_rls_policies.sql:50-56`).

use std::collections::BTreeSet;

use spacetimedb::{Identity, Query, ViewContext};

use super::OrgTransparency;
use super::plan::{caller_visibility, redact};
use super::tables::{
    AccessGroupMemberRow, AccessGroupRow, CampaignRow, CampaignStatRow, DonationRow,
    DonorUpdateRow, OrganizationProfileRow, ProfileRow, VerificationAccessRow, VerificationRow,
};
// The query-builder and read-only handle traits the table macro generates.
use super::tables::{
    access_group__query, access_group__view, access_group_member__query, access_group_member__view,
    campaign__query, campaign_stat__query, donation__query, donor_update__query,
    organization_profile__query, profile__view, verification__view, verification_access_log__view,
};
use crate::codec::{doc_to_row, row_to_doc};
use crate::tables::{Member, member__query, member__view};

/// Declares one view per line: the rows of `$table` in the caller's organisations.
macro_rules! member_views {
    ($($(#[$doc:meta])* $view:ident => $table:ident: $row:ty;)+) => {$(
        $(#[$doc])*
        #[spacetimedb::view(accessor = $view, public)]
        pub fn $view(ctx: &ViewContext) -> impl Query<$row> {
            ctx.from
                .member()
                .r#where(|m| m.identity.eq(ctx.sender()))
                .right_semijoin(ctx.from.$table(), |m, r| m.org_id.eq(r.org_id))
                .build()
        }
    )+};
}

member_views! {
    /// The caller's organisations' profiles.
    my_organization_profile => organization_profile: OrganizationProfileRow;
    /// Access groups (policy "Users can view org access groups", `006_access_groups_rls.sql:16-18`).
    my_access_group => access_group: AccessGroupRow;
    /// Access group members (policy "Users can view group members", `006_access_groups_rls.sql:72-79`).
    my_access_group_member => access_group_member: AccessGroupMemberRow;
    /// Campaigns (policy "Users can view org campaigns", `012_campaigns_rls.sql:15-17`).
    my_campaign => campaign: CampaignRow;
    /// Campaign figures.
    my_campaign_stat => campaign_stat: CampaignStatRow;
    /// Donations (policy "Users can view org donations", `012_campaigns_rls.sql:52-58`).
    my_donation => donation: DonationRow;
    /// Donor updates (policy "Users can view org donor updates", `012_campaigns_rls.sql:109-121`).
    my_donor_update => donor_update: DonorUpdateRow;
}

/// Every proof of the caller's organisations, as much of each as their access groups allow.
#[spacetimedb::view(accessor = my_verification, public)]
#[must_use]
pub fn my_verification(ctx: &ViewContext) -> Vec<VerificationRow> {
    let mine: Vec<Member> = ctx.db.member().identity().filter(ctx.sender()).collect();
    visible_verifications(
        &mine,
        |org| ctx.db.access_group().org_id().filter(org).collect(),
        |org| ctx.db.access_group_member().org_id().filter(org).collect(),
        |org| ctx.db.verification().org_id().filter(org).collect(),
    )
}

/// The proofs `mine` (the caller's member rows) may see, redacted to each organisation's level.
/// A row whose JSON does not read is left out, as is an access group or member that does not.
#[must_use]
pub fn visible_verifications(
    mine: &[Member],
    groups_of: impl Fn(&str) -> Vec<AccessGroupRow>,
    members_of: impl Fn(&str) -> Vec<AccessGroupMemberRow>,
    proofs_of: impl Fn(&str) -> Vec<VerificationRow>,
) -> Vec<VerificationRow> {
    let mut out = Vec::new();
    for m in mine {
        let access = OrgTransparency {
            access_groups: groups_of(&m.org_id)
                .iter()
                .filter_map(|r| row_to_doc(r).ok())
                .collect(),
            group_members: members_of(&m.org_id)
                .iter()
                .filter_map(|r| row_to_doc(r).ok())
                .collect(),
            ..OrgTransparency::default()
        };
        let level = caller_visibility(&access, Some(m));
        for row in proofs_of(&m.org_id) {
            let Ok(proof) = row_to_doc(&row) else {
                continue;
            };
            if let Some(seen) = redact(&proof, level)
                && let Ok(seen_row) = doc_to_row::<VerificationRow>(&m.org_id, &seen, row.seq)
            {
                out.push(seen_row);
            }
        }
    }
    out
}

/// The caller's own looks at proofs.
#[spacetimedb::view(accessor = my_verification_access_log, public)]
#[must_use]
pub fn my_verification_access_log(ctx: &ViewContext) -> Vec<VerificationAccessRow> {
    ctx.db
        .member()
        .identity()
        .filter(ctx.sender())
        .flat_map(|m| {
            ctx.db
                .verification_access_log()
                .org_id()
                .filter(&m.org_id)
                .filter(|r| row_to_doc(r).is_ok_and(|a| a.accessed_by == m.user_id))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The caller's profile and those of everyone in their organisations.
#[spacetimedb::view(accessor = my_profile, public)]
#[must_use]
pub fn my_profile(ctx: &ViewContext) -> Vec<ProfileRow> {
    profiles_visible_to(
        ctx.sender(),
        |identity| ctx.db.member().identity().filter(identity).collect(),
        |org_id| ctx.db.member().org_id().filter(org_id).collect(),
        |identity| ctx.db.profile().identity().find(identity),
    )
}

/// The profiles `sender` may see: their own and every co-member's, each once, in identity order.
#[must_use]
pub fn profiles_visible_to(
    sender: Identity,
    members_of_identity: impl Fn(Identity) -> Vec<Member>,
    members_of_org: impl Fn(&str) -> Vec<Member>,
    profile_of: impl Fn(Identity) -> Option<ProfileRow>,
) -> Vec<ProfileRow> {
    let mut identities = BTreeSet::from([sender]);
    for mine in members_of_identity(sender) {
        identities.extend(members_of_org(&mine.org_id).iter().map(|m| m.identity));
    }
    identities.into_iter().filter_map(profile_of).collect()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::codec::doc_to_row;
    use crate::transparency::types::{
        AccessGroup, AccessGroupPermissions, GroupMember, NodeScope, Verification,
        VerificationStatus, VerificationType, Visibility,
    };

    const CALLER: Identity = Identity::from_byte_array([0xab; 32]);

    fn semijoin_sql(table: &str) -> String {
        format!(
            r#"SELECT "{table}".* FROM "member" JOIN "{table}" ON "member"."org_id" = "{table}"."org_id" WHERE ("member"."identity" = 0x{})"#,
            "ab".repeat(32)
        )
    }

    #[test]
    fn every_membership_view_returns_only_rows_of_the_callers_organisations() {
        let ctx = ViewContext::new(CALLER);
        let views = [
            (
                "organization_profile",
                my_organization_profile(&ctx).sql().to_owned(),
            ),
            ("access_group", my_access_group(&ctx).sql().to_owned()),
            (
                "access_group_member",
                my_access_group_member(&ctx).sql().to_owned(),
            ),
            ("campaign", my_campaign(&ctx).sql().to_owned()),
            ("campaign_stat", my_campaign_stat(&ctx).sql().to_owned()),
            ("donation", my_donation(&ctx).sql().to_owned()),
            ("donor_update", my_donor_update(&ctx).sql().to_owned()),
        ];
        for (table, sql) in views {
            assert_eq!(sql, semijoin_sql(table), "my_{table}");
        }
    }

    fn member(identity: Identity, org_id: &str, user_id: &str) -> Member {
        Member {
            key: format!("{org_id}:{user_id}"),
            identity,
            org_id: org_id.to_owned(),
            user_id: user_id.to_owned(),
            name: user_id.to_owned(),
            can_post: false,
            can_approve: false,
            can_close: false,
        }
    }

    fn group(org_id: &str, id: &str, perms: AccessGroupPermissions) -> AccessGroupRow {
        let g = AccessGroup {
            id: id.to_owned(),
            organization_id: org_id.to_owned(),
            name: id.to_owned(),
            slug: id.to_owned(),
            description: None,
            icon: "users".to_owned(),
            color: "#000000".to_owned(),
            permissions: perms,
            node_scope: NodeScope::All,
            is_default: false,
            is_public: false,
            priority: 1,
            created_at: String::new(),
            updated_at: String::new(),
        };
        doc_to_row(org_id, &g, 0).unwrap()
    }

    fn in_group(org_id: &str, group_id: &str, user_id: &str) -> AccessGroupMemberRow {
        let m = GroupMember {
            id: format!("{group_id}-{user_id}"),
            group_id: group_id.to_owned(),
            user_id: Some(user_id.to_owned()),
            user_email: None,
            granted_by: None,
            valid_from: String::new(),
            valid_until: None,
            notes: None,
            created_at: String::new(),
        };
        doc_to_row(org_id, &m, 0).unwrap()
    }

    fn proof(org_id: &str, id: &str) -> VerificationRow {
        let v = Verification {
            id: id.to_owned(),
            transaction_id: Some("t".to_owned()),
            claim_id: None,
            bill_id: None,
            kind: VerificationType::Photo,
            object_key: Some(format!("{org_id}/{id}.jpg")),
            sha256: None,
            ocr_data: None,
            metadata: serde_json::Map::new(),
            status: VerificationStatus::Verified,
            verified_by: None,
            verified_at: None,
            created_at: String::new(),
            trust_points: 8,
            visibility_level: Visibility::Full,
            thumbnail_key: Some(format!("{org_id}/{id}-small.jpg")),
            executor_name: None,
            gps_latitude: None,
            gps_longitude: None,
            gps_accuracy: None,
            location_name: None,
            description: None,
            created_by: None,
            visible_as: None,
        };
        doc_to_row(org_id, &v, 3).unwrap()
    }

    /// Two organisations: in `org-a` the caller is in a thumbnail group, in `org-b` in none; a
    /// third organisation the caller is not in has a proof too.
    fn seen(mine: &[Member]) -> Vec<(String, Option<Visibility>, bool)> {
        let thumbs = AccessGroupPermissions {
            can_view_proofs: true,
            ..AccessGroupPermissions::default()
        };
        visible_verifications(
            mine,
            |org| match org {
                "org-a" => vec![group("org-a", "g", thumbs)],
                _ => Vec::new(),
            },
            |org| match org {
                "org-a" => vec![in_group("org-a", "g", "me-a")],
                _ => Vec::new(),
            },
            |org| vec![proof(org, &format!("{org}-proof"))],
        )
        .into_iter()
        .map(|r| {
            let v: Verification = row_to_doc(&r).unwrap();
            (r.key, v.visible_as, v.object_key.is_some())
        })
        .collect()
    }

    #[test]
    fn my_verification_cuts_each_organisations_proofs_to_the_callers_groups() {
        let mine = [
            member(CALLER, "org-a", "me-a"),
            member(CALLER, "org-b", "me-b"),
        ];
        assert_eq!(
            seen(&mine),
            [(
                "org-a:org-a-proof".to_owned(),
                Some(Visibility::Thumbnail),
                false
            )]
        );
    }

    #[test]
    fn my_verification_shows_a_stranger_nothing() {
        assert_eq!(seen(&[]), []);
    }

    #[test]
    fn my_profile_is_the_callers_and_their_co_members() {
        let other = Identity::from_byte_array([1; 32]);
        let third = Identity::from_byte_array([2; 32]);
        let all = [
            member(CALLER, "org-a", "me"),
            member(other, "org-a", "other"),
            member(third, "org-c", "third"),
        ];
        let profiles = profiles_visible_to(
            CALLER,
            |id| all.iter().filter(|m| m.identity == id).cloned().collect(),
            |org| all.iter().filter(|m| m.org_id == org).cloned().collect(),
            |identity| {
                Some(ProfileRow {
                    identity,
                    data: String::new(),
                })
            },
        );
        let ids: Vec<Identity> = profiles.into_iter().map(|p| p.identity).collect();
        assert_eq!(ids, [other, CALLER]);
    }
}
