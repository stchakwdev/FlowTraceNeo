//! The transparency rules as pure functions over one organisation's records ([`OrgTransparency`]):
//! what each demo API write checks and changes (`flowtrace/src/lib/demo/api.ts`), with who may
//! make it.
//!
//! **Behaviour** follows the demo API, the ground truth the brief names: the fields each call
//! sets, its ids and times, how donations move a campaign's raised amount, which deletes are
//! silent. **Permissions** follow the web's backend, since the single-user demo API checks none:
//! the row-level security policies of `flowtrace/supabase/migrations/` (`002`, `006`, `012`), with
//! the caller's organisation and role taken from their member row, never from the call. A
//! profile `role` of `admin` or `manager` is a member who can close periods, the same mapping the
//! TS ledger makes from access-group rights to ledger rights (`actorFromPermissions`,
//! `flowtrace/src/lib/ledger/setup.ts:28-43`: `can_close` is `can_manage_groups`). Proof
//! visibility is `getVerificationVisibility` (`flowtrace/src/lib/supabase.ts:573-588`) over the
//! union of the caller's access groups (`getUserPermissions`, `api.ts:772-791`).
//!
//! Every function checks everything before it changes anything, so an `Err` leaves the records
//! as they were. Error texts are the demo API's without "in the demo data", or the module's own
//! where the demo has no such check.

use ledger_core::env::LedgerEnv;
use ledger_core::money::MAX_SAFE_MINOR;
use ledger_core::types::{Minor, Nullish};
use spacetimedb::Identity;

use super::OrgTransparency;
use super::stats::{campaign_stat, iso_ms};
use super::types::{
    AccessGroup, AccessGroupPatch, AccessGroupPermissions, AccessType, Campaign, CampaignPatch,
    CampaignStatus, Donation, DonorUpdate, GroupMember, NewAccessGroup, NewCampaign, NewDonation,
    NewDonorUpdate, NewGroupMember, NewVerification, NodeScope, OrganizationPatch,
    OrganizationProfile, PaymentStatus, Profile, Verification, VerificationAccess,
    VerificationStatus, Visibility,
};
use crate::rules::js_trim;
use crate::tables::Member;

/// Who is calling and when: the reducer's caller with their member row in the organisation, and
/// the ids and clock of the reducer (`ModuleEnv`, as `run_command` uses).
pub struct Ctx<'a> {
    /// The organisation.
    pub org_id: &'a str,
    /// The caller.
    pub identity: Identity,
    /// The caller's member row in the organisation; `None` for anyone else.
    pub member: Option<&'a Member>,
    /// Ids and time.
    pub env: &'a mut dyn LedgerEnv,
}

impl Ctx<'_> {
    fn now(&self) -> String {
        self.env.now().to_string()
    }

    fn new_id(&mut self) -> String {
        self.env.new_id()
    }

    /// The caller as a member (`actorOf`, `spacetimedb/src/index.ts:507-511`; the policies'
    /// `organization_id = get_current_org_id()`).
    fn member(&self) -> Result<&Member, String> {
        self.member
            .ok_or_else(|| format!("You are not a member of organisation {}.", self.org_id))
    }

    /// The caller as an admin or manager: a member who can close periods (module docs above).
    fn manager(&self, what: &str) -> Result<&Member, String> {
        let m = self.member()?;
        if m.can_close {
            Ok(m)
        } else {
            Err(format!("Only members who can close periods can {what}."))
        }
    }
}

/// `"${what} not found in the demo data."` (`api.ts:85-87`), without the demo.
fn not_found(what: &str) -> String {
    format!("{what} not found.")
}

/// The text a reducer returns when the organisation has no books (the module's organisations are
/// its ledgers); the demo's "Organization not found in the demo data." (`api.ts:166-167`).
#[must_use]
pub fn no_org(org_id: &str) -> String {
    format!("No ledger for organisation {org_id}.")
}

/// `updateOrganization`'s refusal (`api.ts:168-177`), also used for a campaign link that is taken
/// (`public_slug TEXT UNIQUE`, `flowtrace/supabase/migrations/011_campaigns.sql:42`).
pub const SLUG_TAKEN: &str = "That public link is already taken. Choose another.";

/// The note on an emailed invite (`api.ts:666-683` writes "Invited in the demo. No email was
/// sent."; the module sends none either).
pub const INVITE_NOTE: &str = "Invited by email. No email was sent.";

/// Every campaign's figures again from the campaigns and donations (`campaign_stat`).
fn refresh_stats(t: &mut OrgTransparency) {
    t.campaign_stats = t
        .campaigns
        .iter()
        .map(|c| campaign_stat(c, &t.donations))
        .collect();
}

fn positive_minor(amount: Minor, message: &str) -> Result<Minor, String> {
    if amount > 0 && amount <= MAX_SAFE_MINOR {
        Ok(amount)
    } else {
        Err(message.to_owned())
    }
}

// ---------------------------------------------------------------------------------------------
// Organisation and person profiles
// ---------------------------------------------------------------------------------------------

/// `updateOrganization` (`api.ts:162-180`): only an admin (policy "Admins can update own
/// organization", `002_rls_policies.sql:44-47`); a non-empty public link may not be another
/// organisation's; the update is applied field by field and `updated_at` stamped. The first
/// update makes the profile, named after the organisation id, in the books' currency.
///
/// # Errors
///
/// Not an admin, or the link is taken.
pub fn update_organization(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    base_currency: &str,
    patch: OrganizationPatch,
    slug_taken_elsewhere: &dyn Fn(&str) -> bool,
) -> Result<OrganizationProfile, String> {
    ctx.manager("change the organisation profile")?;
    if let Nullish::Value(slug) = &patch.public_slug
        && !slug.is_empty()
        && slug_taken_elsewhere(slug)
    {
        return Err(SLUG_TAKEN.to_owned());
    }
    let now = ctx.now();
    let mut p = t.profile.take().unwrap_or_else(|| OrganizationProfile {
        id: ctx.org_id.to_owned(),
        name: ctx.org_id.to_owned(),
        org_type: None,
        country: String::new(),
        currency: base_currency.to_owned(),
        verified: false,
        public_slug: None,
        description: None,
        logo_key: None,
        is_public: false,
        created_at: now.clone(),
        updated_at: now.clone(),
    });
    let OrganizationPatch {
        name,
        org_type,
        country,
        currency,
        public_slug,
        description,
        logo_key,
        is_public,
    } = patch;
    set(&mut p.name, name);
    set_nullish(&mut p.org_type, org_type);
    set(&mut p.country, country);
    set(&mut p.currency, currency);
    set_nullish(&mut p.public_slug, public_slug);
    set_nullish(&mut p.description, description);
    set_nullish(&mut p.logo_key, logo_key);
    set(&mut p.is_public, is_public);
    p.updated_at = now;
    t.profile = Some(p.clone());
    Ok(p)
}

/// `Object.assign` of one field that may be left out.
fn set<T>(field: &mut T, value: Option<T>) {
    if let Some(v) = value {
        *field = v;
    }
}

/// `Object.assign` of one field that may be left out or set to `null`.
fn set_nullish<T>(field: &mut Option<T>, value: Nullish<T>) {
    match value {
        Nullish::Undefined => {}
        Nullish::Null => *field = None,
        Nullish::Value(v) => *field = Some(v),
    }
}

/// `createOrUpdateProfile` (`supabase.ts:311-330`) for the caller's own identity (policy "Users
/// can update own profile", `002_rls_policies.sql:58-61`): the name trimmed (`None` when blank),
/// the email kept only as its hash, the user id the caller's JWT subject or identity.
#[must_use]
pub fn set_profile(
    existing: Option<Profile>,
    identity: Identity,
    user_id: &str,
    display_name: &str,
    email: &str,
    now: &str,
) -> Profile {
    let name = js_trim(display_name);
    Profile {
        identity: identity.to_hex().to_string(),
        user_id: user_id.to_owned(),
        display_name: (!name.is_empty()).then(|| name.to_owned()),
        email_hash: super::ts::email_hash(email),
        created_at: existing.map_or_else(|| now.to_owned(), |p| p.created_at),
        updated_at: now.to_owned(),
    }
}

// ---------------------------------------------------------------------------------------------
// Access groups and members
// ---------------------------------------------------------------------------------------------

const MANAGE_GROUPS: &str = "manage access groups";

/// One of the default groups: what the trigger inserts for it.
struct DefaultGroup {
    name: &'static str,
    slug: &'static str,
    description: &'static str,
    icon: &'static str,
    color: &'static str,
    node_scope: NodeScope,
    is_public: bool,
    priority: i64,
    permissions: AccessGroupPermissions,
}

/// The five groups every organisation starts with (`flowtrace/src/lib/demo/dataset.ts:698-789`,
/// the `organizations_create_default_groups` trigger of `005_access_groups.sql:198-334`), all
/// marked default as the dataset marks them.
fn default_groups() -> [DefaultGroup; 5] {
    let none = AccessGroupPermissions::default();
    [
        DefaultGroup {
            name: "Board of Directors",
            slug: "board",
            description: "Full oversight access for board members",
            icon: "shield-check",
            color: "#10b981",
            node_scope: NodeScope::All,
            is_public: false,
            priority: 10,
            permissions: AccessGroupPermissions {
                can_view_nodes: true,
                can_view_amounts: true,
                can_view_proofs: true,
                can_view_executor: true,
                can_view_full_proofs: true,
                can_edit_nodes: true,
                can_edit_transactions: true,
                can_manage_members: true,
                can_manage_groups: true,
            },
        },
        DefaultGroup {
            name: "Staff",
            slug: "staff",
            description: "Internal staff with operational access",
            icon: "users",
            color: "#3b82f6",
            node_scope: NodeScope::All,
            is_public: false,
            priority: 20,
            permissions: AccessGroupPermissions {
                can_view_nodes: true,
                can_view_amounts: true,
                can_view_proofs: true,
                can_view_executor: true,
                can_edit_nodes: true,
                can_edit_transactions: true,
                ..none
            },
        },
        DefaultGroup {
            name: "Donors",
            slug: "donors",
            description: "Donor organizations and individuals with downstream visibility",
            icon: "heart",
            color: "#ec4899",
            node_scope: NodeScope::Downstream,
            is_public: false,
            priority: 30,
            permissions: AccessGroupPermissions {
                can_view_nodes: true,
                can_view_amounts: true,
                can_view_proofs: true,
                ..none
            },
        },
        DefaultGroup {
            name: "Partners",
            slug: "partners",
            description: "Partner organizations with assigned node access",
            icon: "handshake",
            color: "#f59e0b",
            node_scope: NodeScope::Assigned,
            is_public: false,
            priority: 40,
            permissions: AccessGroupPermissions {
                can_view_nodes: true,
                can_view_amounts: true,
                ..none
            },
        },
        DefaultGroup {
            name: "Public",
            slug: "public",
            description: "Public view for transparency reporting",
            icon: "globe",
            color: "#8b5cf6",
            node_scope: NodeScope::All,
            is_public: true,
            priority: 100,
            permissions: AccessGroupPermissions {
                can_view_nodes: true,
                ..none
            },
        },
    ]
}

/// Gives the organisation the five default groups, as the database trigger does for a new
/// organisation (`005_access_groups.sql:198-334`); an admin calls it once, since the module's
/// organisations are opened by `create_org_ledger`, which has no TS trigger. Nobody is put in a
/// group: the admin adds people, themselves included.
///
/// # Errors
///
/// Not an admin, or the organisation already has default groups.
pub fn create_default_access_groups(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
) -> Result<Vec<AccessGroup>, String> {
    ctx.manager(MANAGE_GROUPS)?;
    if t.access_groups.iter().any(|g| g.is_default) {
        return Err("This organisation already has its default access groups.".to_owned());
    }
    let mut made = Vec::new();
    for g in default_groups() {
        let now = ctx.now();
        made.push(AccessGroup {
            id: ctx.new_id(),
            organization_id: ctx.org_id.to_owned(),
            name: g.name.to_owned(),
            slug: g.slug.to_owned(),
            description: Some(g.description.to_owned()),
            icon: g.icon.to_owned(),
            color: g.color.to_owned(),
            permissions: g.permissions,
            node_scope: g.node_scope,
            is_default: true,
            is_public: g.is_public,
            priority: g.priority,
            created_at: now.clone(),
            updated_at: now,
        });
    }
    t.access_groups.extend(made.iter().cloned());
    Ok(made)
}

/// `createAccessGroup` (`api.ts:585-596`): only an admin or manager (policy "Admins can create
/// access groups", `006_access_groups_rls.sql:26-36`).
///
/// # Errors
///
/// Not an admin.
pub fn create_access_group(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewAccessGroup,
) -> Result<AccessGroup, String> {
    ctx.manager(MANAGE_GROUPS)?;
    let NewAccessGroup {
        name,
        slug,
        description,
        icon,
        color,
        permissions,
        node_scope,
        is_default,
        is_public,
        priority,
    } = input;
    let id = ctx.new_id();
    let now = ctx.now();
    let row = AccessGroup {
        id,
        organization_id: ctx.org_id.to_owned(),
        name,
        slug,
        description,
        icon,
        color,
        permissions,
        node_scope,
        is_default,
        is_public,
        priority,
        created_at: now.clone(),
        updated_at: now,
    };
    t.access_groups.push(row.clone());
    Ok(row)
}

/// `updateAccessGroup` (`api.ts:598-609`): only an admin or manager (`006_access_groups_rls.sql:39-51`).
/// Default groups can be edited, as the demo API and the web form allow ("You can change its
/// depth, but not delete it", `access-groups.tsx:918-921`), though the policy refuses it.
///
/// # Errors
///
/// Not an admin, or no such group.
pub fn update_access_group(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    patch: AccessGroupPatch,
) -> Result<AccessGroup, String> {
    ctx.manager(MANAGE_GROUPS)?;
    let now = ctx.now();
    let g = t
        .access_groups
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or_else(|| not_found("Access group"))?;
    let AccessGroupPatch {
        name,
        slug,
        description,
        icon,
        color,
        permissions,
        node_scope,
        is_default,
        is_public,
        priority,
    } = patch;
    set(&mut g.name, name);
    set(&mut g.slug, slug);
    set_nullish(&mut g.description, description);
    set(&mut g.icon, icon);
    set(&mut g.color, color);
    set(&mut g.permissions, permissions);
    set(&mut g.node_scope, node_scope);
    set(&mut g.is_default, is_default);
    set(&mut g.is_public, is_public);
    set(&mut g.priority, priority);
    g.updated_at = now;
    Ok(g.clone())
}

/// `deleteAccessGroup` (`api.ts:611-616`): removes the group and its members (its node
/// visibility rows are MOD2's); an unknown id changes nothing, as in the demo. Only an admin, and
/// never a default group: the web offers no delete for one (`access-groups.tsx:402-413`) and the
/// policy refuses it (`006_access_groups_rls.sql:54-65`), though the demo API would delete it.
///
/// # Errors
///
/// Not an admin, or a default group.
pub fn delete_access_group(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
) -> Result<(), String> {
    ctx.manager(MANAGE_GROUPS)?;
    if t.access_groups.iter().any(|g| g.id == id && g.is_default) {
        return Err("Default access groups cannot be deleted.".to_owned());
    }
    t.access_groups.retain(|g| g.id != id);
    t.group_members.retain(|m| m.group_id != id);
    Ok(())
}

fn group_in_org(t: &OrgTransparency, group_id: &str) -> Result<(), String> {
    if t.access_groups.iter().any(|g| g.id == group_id) {
        Ok(())
    } else {
        Err(not_found("Access group"))
    }
}

/// `addGroupMember` (`api.ts:654-658`): only an admin or manager (policy "Admins can add group
/// members", `006_access_groups_rls.sql:82-95`); `granted_by` is the caller. The group must be
/// the organisation's (the demo would add a member to any id).
///
/// # Errors
///
/// Not an admin, or no such group in the organisation.
pub fn add_group_member(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewGroupMember,
) -> Result<GroupMember, String> {
    let granted_by = ctx.manager(MANAGE_GROUPS)?.user_id.clone();
    group_in_org(t, &input.group_id)?;
    let id = ctx.new_id();
    let now = ctx.now();
    let NewGroupMember {
        group_id,
        user_id,
        user_email,
        valid_from,
        valid_until,
        notes,
    } = input;
    let row = GroupMember {
        id,
        group_id,
        user_id,
        user_email,
        granted_by: Some(granted_by),
        valid_from: valid_from.unwrap_or_else(|| now.clone()),
        valid_until,
        notes,
        created_at: now,
    };
    t.group_members.push(row.clone());
    Ok(row)
}

/// `inviteMemberByEmail` (`api.ts:666-683`): a member with only an email, valid from now, until
/// `valid_until` when given; no email goes out. Same rights as [`add_group_member`].
///
/// # Errors
///
/// Not an admin, or no such group in the organisation.
pub fn invite_group_member(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    group_id: &str,
    email: &str,
    valid_until: Option<String>,
) -> Result<GroupMember, String> {
    let granted_by = ctx.manager(MANAGE_GROUPS)?.user_id.clone();
    group_in_org(t, group_id)?;
    let id = ctx.new_id();
    let now = ctx.now();
    let row = GroupMember {
        id,
        group_id: group_id.to_owned(),
        user_id: None,
        user_email: Some(email.to_owned()),
        granted_by: Some(granted_by),
        valid_from: now.clone(),
        valid_until: valid_until.filter(|v| !v.is_empty()),
        notes: Some(INVITE_NOTE.to_owned()),
        created_at: now,
    };
    t.group_members.push(row.clone());
    Ok(row)
}

/// `removeGroupMember` (`api.ts:660-663`): an unknown id changes nothing. Only an admin or
/// manager (`006_access_groups_rls.sql:114-127`).
///
/// # Errors
///
/// Not an admin.
pub fn remove_group_member(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
) -> Result<(), String> {
    ctx.manager(MANAGE_GROUPS)?;
    t.group_members.retain(|m| m.id != id);
    Ok(())
}

/// `getUserPermissions` (`api.ts:772-791`): every flag any of the user's groups in the
/// organisation grants. As in the demo, `valid_from` and `valid_until` are not checked (the
/// Supabase function does check them, `005_access_groups.sql:356-359`; views have no clock).
#[must_use]
pub fn user_permissions(t: &OrgTransparency, user_id: &str) -> AccessGroupPermissions {
    t.access_groups
        .iter()
        .filter(|g| {
            t.group_members
                .iter()
                .any(|m| m.group_id == g.id && m.user_id.as_deref() == Some(user_id))
        })
        .fold(AccessGroupPermissions::default(), |acc, g| {
            acc.union(g.permissions)
        })
}

/// `getVerificationVisibility` (`supabase.ts:573-588`): full proofs, else proofs as thumbnails,
/// else nodes as a summary, else hidden. Like the TS it does not look at the proof itself.
#[must_use]
pub const fn visibility(perms: AccessGroupPermissions) -> Visibility {
    if perms.can_view_full_proofs {
        Visibility::Full
    } else if perms.can_view_proofs {
        Visibility::Thumbnail
    } else if perms.can_view_nodes {
        Visibility::Summary
    } else {
        Visibility::Hidden
    }
}

/// How much of the organisation's proofs a caller sees: nothing for a non-member, else
/// [`visibility`] of their [`user_permissions`].
#[must_use]
pub fn caller_visibility(t: &OrgTransparency, member: Option<&Member>) -> Visibility {
    member.map_or(Visibility::Hidden, |m| {
        visibility(user_permissions(t, &m.user_id))
    })
}

/// A proof as a viewer at `level` gets it (`VISIBILITY_DESCRIPTION`,
/// `flowtrace/src/components/verification-viewer.tsx:24-29`): `full` the original file,
/// `thumbnail` only the low-resolution version, `summary` that proof is attached and not the
/// file, `hidden` nothing. The row says which in `visible_as`.
#[must_use]
pub fn redact(v: &Verification, level: Visibility) -> Option<Verification> {
    let mut out = v.clone();
    match level {
        Visibility::Hidden => return None,
        Visibility::Summary => {
            out.object_key = None;
            out.thumbnail_key = None;
        }
        Visibility::Thumbnail => out.object_key = None,
        Visibility::Full => {}
    }
    out.visible_as = Some(level);
    Some(out)
}

// ---------------------------------------------------------------------------------------------
// Verifications
// ---------------------------------------------------------------------------------------------

/// What a proof may link to besides a transaction: the ledger's expense claims and bills.
pub trait LedgerLinks {
    /// The organisation has an expense claim with this id.
    fn has_claim(&self, id: &str) -> bool;
    /// The organisation has a bill with this id.
    fn has_bill(&self, id: &str) -> bool;
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `createVerification` (`insertVerification`, `api.ts:284-317`): any member (policy "Users can
/// create org verifications", `002_rls_policies.sql:108-114`). Trust points from the type,
/// `verified_at` and `verified_by` only for a proof made verified, `created_by` the caller. The
/// module adds: one link, to a transaction (MOD2), or an expense claim or bill of the books; an
/// R2 object key comes with its SHA-256 (P13). The demo's transaction side effects (count and
/// auto-verify) are MOD2's.
///
/// # Errors
///
/// Not a member, not exactly one link, an unknown claim or bill, or a file without its hash.
pub fn create_verification(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewVerification,
    ledger: &dyn LedgerLinks,
) -> Result<Verification, String> {
    let user_id = ctx.member()?.user_id.clone();
    let links = [&input.transaction_id, &input.claim_id, &input.bill_id];
    if links.iter().filter(|l| l.is_some()).count() != 1 {
        return Err("A proof belongs to one transaction, expense claim or bill.".to_owned());
    }
    if let Some(id) = &input.claim_id
        && !ledger.has_claim(id)
    {
        return Err(format!("Expense claim {id} not found."));
    }
    if let Some(id) = &input.bill_id
        && !ledger.has_bill(id)
    {
        return Err(format!("Bill {id} not found."));
    }
    match (&input.object_key, &input.sha256) {
        (None, None) => {}
        (Some(key), Some(hash)) if !key.is_empty() && is_sha256(hash) => {}
        (_, Some(hash)) if !is_sha256(hash) => {
            return Err("A file hash must be a SHA-256 in lowercase hex.".to_owned());
        }
        _ => return Err("A proof file needs both its object key and its SHA-256.".to_owned()),
    }
    let id = ctx.new_id();
    let now = ctx.now();
    let verified = input.status == VerificationStatus::Verified;
    let NewVerification {
        transaction_id,
        claim_id,
        bill_id,
        kind,
        object_key,
        sha256,
        thumbnail_key,
        ocr_data,
        metadata,
        status,
        visibility_level,
        executor_name,
        gps_latitude,
        gps_longitude,
        gps_accuracy,
        location_name,
        description,
    } = input;
    let row = Verification {
        id,
        transaction_id,
        claim_id,
        bill_id,
        kind,
        object_key,
        sha256,
        ocr_data,
        metadata,
        status,
        verified_by: verified.then(|| user_id.clone()),
        verified_at: verified.then(|| now.clone()),
        created_at: now,
        trust_points: kind.trust_points(),
        visibility_level,
        thumbnail_key,
        executor_name,
        gps_latitude,
        gps_longitude,
        gps_accuracy,
        location_name,
        description,
        created_by: Some(user_id),
        visible_as: None,
    };
    t.verifications.push(row.clone());
    Ok(row)
}

/// `logVerificationAccess` (`supabase.ts:554-571`; the demo has none): one entry by the caller at
/// the level they are given, never taken from the call. A member who cannot see the proof
/// cannot log a look at it.
///
/// # Errors
///
/// Not a member, no such proof, or the caller cannot see it.
pub fn log_verification_access(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    verification_id: &str,
    access_type: AccessType,
) -> Result<VerificationAccess, String> {
    let member = ctx.member()?.clone();
    if !t.verifications.iter().any(|v| v.id == verification_id) {
        return Err(not_found("Verification"));
    }
    let level = caller_visibility(t, Some(&member));
    if level == Visibility::Hidden {
        return Err("You cannot see this proof.".to_owned());
    }
    let row = VerificationAccess {
        id: ctx.new_id(),
        verification_id: verification_id.to_owned(),
        accessed_by: member.user_id,
        access_type,
        access_level: level,
        ip_address: None,
        user_agent: None,
        accessed_at: ctx.now(),
    };
    t.access_log.push(row.clone());
    Ok(row)
}

// ---------------------------------------------------------------------------------------------
// Campaigns
// ---------------------------------------------------------------------------------------------

const TARGET: &str = "A campaign needs a target above zero.";

/// `slugify` (`api.ts:1068-1078`): the name lowercased with each run of other characters a
/// hyphen and the ends trimmed, `campaign` when nothing is left, then `-2`, `-3`, ... until the
/// link is free.
#[must_use]
pub fn slugify(name: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let mut base = String::new();
    let mut gap = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if gap && !base.is_empty() {
                base.push('-');
            }
            base.push(c);
            gap = false;
        } else {
            gap = true;
        }
    }
    // `.replace(/^-|-$/g, '')`: a leading run never got its hyphen above; a trailing one is
    // dropped by not pushing it.
    let mut slug = if base.is_empty() {
        "campaign".to_owned()
    } else {
        base.clone()
    };
    let mut i = 2;
    while taken(&slug) {
        slug = format!("{base}-{i}");
        i += 1;
    }
    slug
}

/// `createCampaign` (`api.ts:1080-1098`): any member (policy "Users can create org campaigns",
/// `012_campaigns_rls.sql:29-31`). Nothing raised yet; a public campaign without a link gets one
/// from its name ([`slugify`]) unique across every organisation's campaigns. The module also
/// refuses a target that is not above zero and a link that is taken (`011_campaigns.sql:20,42`).
///
/// # Errors
///
/// Not a member, a target not above zero, or a taken link.
pub fn create_campaign(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewCampaign,
    slug_taken_elsewhere: &dyn Fn(&str) -> bool,
) -> Result<Campaign, String> {
    let created_by = ctx.member()?.user_id.clone();
    positive_minor(input.target_amount, TARGET)?;
    let taken = |s: &str| {
        slug_taken_elsewhere(s)
            || t.campaigns
                .iter()
                .any(|c| c.public_slug.as_deref() == Some(s))
    };
    let public_slug = match input.public_slug {
        Some(given) if taken(&given) => return Err(SLUG_TAKEN.to_owned()),
        Some(given) => Some(given),
        None if input.is_public => Some(slugify(&input.name, &taken)),
        None => None,
    };
    let id = ctx.new_id();
    let now = ctx.now();
    let NewCampaign {
        name,
        description,
        short_description,
        image_url,
        target_amount,
        currency,
        target_node_id,
        status,
        allow_overflow,
        overflow_campaign_id,
        overflow_consent_message,
        starts_at,
        ends_at,
        is_public,
        public_slug: _,
        category,
        tags,
        metadata,
    } = input;
    let row = Campaign {
        id,
        organization_id: ctx.org_id.to_owned(),
        name,
        description,
        short_description,
        image_url,
        target_amount,
        current_amount: 0,
        currency,
        target_node_id,
        status,
        allow_overflow,
        overflow_campaign_id,
        overflow_consent_message,
        starts_at,
        ends_at,
        is_public,
        public_slug,
        category,
        tags,
        metadata,
        created_by: Some(created_by),
        created_at: now.clone(),
        updated_at: now,
    };
    t.campaigns.push(row.clone());
    refresh_stats(t);
    Ok(row)
}

/// `updateCampaign` (`api.ts:1100-1114`): any member (`012_campaigns_rls.sql:34-37`); the
/// update applied field by field and `updated_at` stamped. The module refuses a target that is
/// not above zero and a link another campaign has (`011_campaigns.sql:20,42`); `current_amount`
/// is not in the patch.
///
/// # Errors
///
/// Not a member, no such campaign, a target not above zero, or a taken link.
pub fn update_campaign(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    patch: CampaignPatch,
    slug_taken_elsewhere: &dyn Fn(&str) -> bool,
) -> Result<Campaign, String> {
    ctx.member()?;
    if !t.campaigns.iter().any(|c| c.id == id) {
        return Err(not_found("Campaign"));
    }
    if let Some(target) = patch.target_amount {
        positive_minor(target, TARGET)?;
    }
    if let Nullish::Value(slug) = &patch.public_slug
        && (slug_taken_elsewhere(slug)
            || t.campaigns
                .iter()
                .any(|c| c.id != id && c.public_slug.as_deref() == Some(slug.as_str())))
    {
        return Err(SLUG_TAKEN.to_owned());
    }
    let now = ctx.now();
    let Some(c) = t.campaigns.iter_mut().find(|c| c.id == id) else {
        return Err(not_found("Campaign"));
    };
    let CampaignPatch {
        name,
        description,
        short_description,
        image_url,
        target_amount,
        currency,
        target_node_id,
        status,
        allow_overflow,
        overflow_campaign_id,
        overflow_consent_message,
        starts_at,
        ends_at,
        is_public,
        public_slug,
        category,
        tags,
        metadata,
    } = patch;
    set(&mut c.name, name);
    set_nullish(&mut c.description, description);
    set_nullish(&mut c.short_description, short_description);
    set_nullish(&mut c.image_url, image_url);
    set(&mut c.target_amount, target_amount);
    set(&mut c.currency, currency);
    set_nullish(&mut c.target_node_id, target_node_id);
    set(&mut c.status, status);
    set(&mut c.allow_overflow, allow_overflow);
    set_nullish(&mut c.overflow_campaign_id, overflow_campaign_id);
    set_nullish(&mut c.overflow_consent_message, overflow_consent_message);
    set(&mut c.starts_at, starts_at);
    set_nullish(&mut c.ends_at, ends_at);
    set(&mut c.is_public, is_public);
    set_nullish(&mut c.public_slug, public_slug);
    set_nullish(&mut c.category, category);
    set_nullish(&mut c.tags, tags);
    set(&mut c.metadata, metadata);
    c.updated_at = now;
    let row = c.clone();
    refresh_stats(t);
    Ok(row)
}

/// `deleteCampaign` (`api.ts:1116-1126`): only a draft goes; anything else, or an unknown id,
/// changes nothing, as in the demo (policy "Users can delete draft campaigns",
/// `012_campaigns_rls.sql:40-45`). Its donations stay, as in the demo.
///
/// # Errors
///
/// Not a member.
pub fn delete_campaign(t: &mut OrgTransparency, ctx: &mut Ctx<'_>, id: &str) -> Result<(), String> {
    ctx.member()?;
    t.campaigns
        .retain(|c| !(c.id == id && c.status == CampaignStatus::Draft));
    refresh_stats(t);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Donations and donor updates
// ---------------------------------------------------------------------------------------------

/// `applyToCampaign` (`api.ts:1245-1254`): a donation that becomes confirmed adds to the
/// campaign's raised amount, one that stops being confirmed takes it off (no floor at zero, as
/// in the demo; the Supabase trigger floors it, `011_campaigns.sql:206`).
fn apply_to_campaign(
    t: &mut OrgTransparency,
    d: &Donation,
    from: Option<PaymentStatus>,
    to: PaymentStatus,
) {
    let Some(c) = t.campaigns.iter_mut().find(|c| c.id == d.campaign_id) else {
        return;
    };
    let was = from == Some(PaymentStatus::Confirmed);
    let is = to == PaymentStatus::Confirmed;
    if !was && is {
        c.current_amount += d.amount;
    }
    if was && !is {
        c.current_amount -= d.amount;
    }
}

/// `createDonation` (`api.ts:1256-1272`): a record only, no payment is processed (P19). A member
/// records any donation to the organisation's campaigns (policy "Org members can create
/// donations", `012_campaigns_rls.sql:82-88`). Anyone else may record one only to an active
/// public campaign that has not ended and has room or takes overflow ("Anyone can create
/// donations", `012_campaigns_rls.sql:69-79`), only as pending (no processor confirms it, and a
/// member confirms with [`update_donation_status`]), and a named gift keeps their identity as
/// `donor_profile`. A confirmed donation adds to the raised amount ([`apply_to_campaign`]).
///
/// # Errors
///
/// No such campaign, an amount not above zero, or a non-member's donation the rules refuse.
pub fn record_donation(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewDonation,
) -> Result<Donation, String> {
    let now = ctx.now();
    let c = t
        .campaigns
        .iter()
        .find(|c| c.id == input.campaign_id)
        .ok_or_else(|| not_found("Campaign"))?;
    positive_minor(input.amount, "A donation needs an amount above zero.")?;
    let donor_profile = if ctx.member.is_some() {
        None
    } else {
        let now_ms = iso_ms(&now).unwrap_or(i64::MAX);
        let open = c.status == CampaignStatus::Active
            && c.is_public
            && c.ends_at
                .as_deref()
                .is_none_or(|end| iso_ms(end).is_some_and(|e| e > now_ms))
            && (c.allow_overflow || c.current_amount < c.target_amount);
        if !open {
            return Err("This campaign is not open for donations.".to_owned());
        }
        if input.payment_status != PaymentStatus::Pending {
            return Err(
                "A donation from outside the organisation is recorded as pending.".to_owned(),
            );
        }
        (!input.is_anonymous).then(|| ctx.identity.to_hex().to_string())
    };
    let id = ctx.new_id();
    let NewDonation {
        campaign_id,
        donor_email,
        donor_name,
        donor_phone,
        is_anonymous,
        amount,
        currency,
        amount_usd,
        exchange_rate,
        earmarked_node_id,
        earmark_notes,
        allow_reallocation,
        flow_node_id,
        payment_method,
        payment_reference,
        payment_status,
        transaction_id,
        source,
        utm_source,
        utm_medium,
        utm_campaign,
        metadata,
    } = input;
    let row = Donation {
        id,
        campaign_id,
        donor_email,
        donor_name,
        donor_phone,
        is_anonymous,
        donor_profile,
        amount,
        currency,
        amount_usd,
        exchange_rate,
        earmarked_node_id,
        earmark_notes,
        allow_reallocation,
        flow_node_id,
        impact_percentage: None,
        payment_method,
        payment_reference,
        payment_status,
        transaction_id,
        source,
        utm_source,
        utm_medium,
        utm_campaign,
        metadata,
        created_at: now.clone(),
        updated_at: now,
    };
    t.donations.push(row.clone());
    apply_to_campaign(t, &row, None, row.payment_status);
    refresh_stats(t);
    Ok(row)
}

/// `updateDonationStatus` (`api.ts:1274-1289`): any member (policy "Users can update org
/// donations", `012_campaigns_rls.sql:91-102`); moves the raised amount ([`apply_to_campaign`]),
/// sets the status, the reference (`None` when blank) and `updated_at`.
///
/// # Errors
///
/// Not a member, or no such donation.
pub fn update_donation_status(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    status: PaymentStatus,
    payment_reference: Option<String>,
) -> Result<Donation, String> {
    ctx.member()?;
    let now = ctx.now();
    let d = t
        .donations
        .iter()
        .find(|d| d.id == id)
        .cloned()
        .ok_or_else(|| not_found("Donation"))?;
    apply_to_campaign(t, &d, Some(d.payment_status), status);
    let Some(row) = t.donations.iter_mut().find(|d| d.id == id) else {
        return Err(not_found("Donation"));
    };
    row.payment_status = status;
    row.payment_reference = payment_reference.filter(|r| !r.is_empty());
    row.updated_at = now;
    let row = row.clone();
    refresh_stats(t);
    Ok(row)
}

/// `createDonorUpdate` (`api.ts:1329-1341`): any member (policy "Users can create donor updates",
/// `012_campaigns_rls.sql:135-147`); not sent yet. The donation must be the organisation's (the
/// demo takes any id).
///
/// # Errors
///
/// Not a member, or no such donation.
pub fn post_donor_update(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    input: NewDonorUpdate,
) -> Result<DonorUpdate, String> {
    ctx.member()?;
    if !t.donations.iter().any(|d| d.id == input.donation_id) {
        return Err(not_found("Donation"));
    }
    let id = ctx.new_id();
    let now = ctx.now();
    let NewDonorUpdate {
        donation_id,
        campaign_id,
        update_type,
        title,
        message,
        image_url,
        related_node_id,
        related_transaction_id,
        delivery_method,
        metadata,
    } = input;
    let row = DonorUpdate {
        id,
        donation_id,
        campaign_id,
        update_type,
        title,
        message,
        image_url,
        related_node_id,
        related_transaction_id,
        sent_at: None,
        delivery_method,
        metadata,
        created_at: now,
    };
    t.donor_updates.push(row.clone());
    Ok(row)
}

/// `markUpdateSent` (`api.ts:1343-1349`): stamps `sent_at`; nothing is delivered. Any member
/// (policy "Users can update org donor updates", `012_campaigns_rls.sql:150-156`).
///
/// # Errors
///
/// Not a member, or no such update.
pub fn mark_update_sent(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
) -> Result<DonorUpdate, String> {
    ctx.member()?;
    let now = ctx.now();
    let u = t
        .donor_updates
        .iter_mut()
        .find(|u| u.id == id)
        .ok_or_else(|| "Update not found.".to_owned())?;
    u.sent_at = Some(now);
    Ok(u.clone())
}
