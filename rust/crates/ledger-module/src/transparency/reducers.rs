//! The transparency reducers: one per demo API write (`flowtrace/src/lib/demo/api.ts`), plus
//! `set_profile`, `log_verification_access`, `create_default_access_groups` and the operator's
//! `import_transparency`. The MOD2 reducers are in [`super::flow_reducers`],
//! [`super::chain_reducers`] and [`super::trust_reducers`].
//!
//! Each one loads the organisation's transparency rows, lets a [`super::plan`] function change
//! the records with the caller's member row and the reducer's ids and clock, and writes the rows
//! that changed ([`super::rows::plan_write`]). The organisation must have books (the module's
//! organisations are its ledgers). Record-shaped arguments are JSON in the module's shape
//! ([`super::types`]: money in minor units, files as object keys); an `Err` rolls the call back
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.reducer.html>).
//!
//! After every change ([`change`]) the trust score is computed again when an input moved
//! ([`super::trust::refresh`]), and the organisation's public rows are rewritten from its records
//! ([`publish`]), so the projection never lags the records it comes from.

use ledger_core::types::OrgLedgerSettings;
use serde::de::DeserializeOwned;
use spacetimedb::{ReducerContext, Table};

use super::OrgTransparency;
use super::flows::EntryLinks;
use super::plan::{self, Ctx, LedgerLinks, no_org};
use super::rows::{
    OneRow, PublicRows, TransparencyRows, TransparencyWrite, from_rows, plan_public_write,
    plan_write, profile_of, public_rows,
};
use super::tables::{
    ProfileRow, access_group, access_group_member, campaign, campaign_stat, donation, donor_update,
    flag, flow_node, node_visibility, organization_profile, profile, public_badge, public_campaign,
    public_embed_config, public_organization, service, transaction, transaction_visibility,
    trust_score, trust_score_history, verification, verification_access_log,
};
use super::types::{
    AccessGroupPatch, AccessType, CampaignPatch, NewAccessGroup, NewCampaign, NewDonation,
    NewDonorUpdate, NewGroupMember, NewVerification, OrganizationPatch, PaymentStatus, Profile,
};
use crate::clock::reducer_time;
use crate::codec::{row_key, row_to_doc};
use crate::reducers::{is_operator, reducer_env, text};
use crate::rules::member_key;
use crate::tables::{bill, expense_claim, journal_entry, ledger_org, member};
use crate::write_back::TableOps;

/// The organisation's transparency rows.
pub(super) fn load_rows(ctx: &ReducerContext, org_id: &str) -> TransparencyRows {
    let org = org_id.to_owned();
    TransparencyRows {
        profile: ctx.db.organization_profile().org_id().find(&org),
        access_groups: ctx.db.access_group().org_id().filter(&org).collect(),
        group_members: ctx.db.access_group_member().org_id().filter(&org).collect(),
        verifications: ctx.db.verification().org_id().filter(&org).collect(),
        access_log: ctx
            .db
            .verification_access_log()
            .org_id()
            .filter(&org)
            .collect(),
        campaigns: ctx.db.campaign().org_id().filter(&org).collect(),
        campaign_stats: ctx.db.campaign_stat().org_id().filter(&org).collect(),
        donations: ctx.db.donation().org_id().filter(&org).collect(),
        donor_updates: ctx.db.donor_update().org_id().filter(&org).collect(),
        flow_nodes: ctx.db.flow_node().org_id().filter(&org).collect(),
        transactions: ctx.db.transaction().org_id().filter(&org).collect(),
        node_visibility: ctx.db.node_visibility().org_id().filter(&org).collect(),
        transaction_visibility: ctx
            .db
            .transaction_visibility()
            .org_id()
            .filter(&org)
            .collect(),
        trust: ctx.db.trust_score().org_id().find(&org),
        trust_history: ctx.db.trust_score_history().org_id().filter(&org).collect(),
        flags: ctx.db.flag().org_id().filter(&org).collect(),
        embed: ctx.db.public_embed_config().org_id().find(&org),
    }
}

/// An organisation's records, read from its rows.
pub(super) fn load(ctx: &ReducerContext, org_id: &str) -> Result<OrgTransparency, String> {
    from_rows(&load_rows(ctx, org_id)).map_err(text)
}

/// Inserts or replaces a row of a table keyed by `$key`.
macro_rules! upsert {
    ($table:expr, $key:ident, $row:expr) => {{
        let table = $table;
        let row = $row;
        if table.$key().find(&row.$key).is_some() {
            table.$key().update(row);
        } else {
            table.try_insert(row).map_err(text)?;
        }
    }};
}

/// Applies one table's writes through its `key` index.
macro_rules! write_table {
    ($table:expr, $ops:expr) => {{
        let table = $table;
        let ops: TableOps<_> = $ops;
        for key in &ops.delete {
            table.key().delete(key);
        }
        for row in ops.update {
            table.key().update(row);
        }
        for row in ops.insert {
            table.try_insert(row).map_err(text)?;
        }
    }};
}

fn write(ctx: &ReducerContext, w: TransparencyWrite) -> Result<(), String> {
    if let Some(row) = w.profile {
        upsert!(ctx.db.organization_profile(), org_id, row);
    }
    if let Some(row) = w.trust {
        upsert!(ctx.db.trust_score(), org_id, row);
    }
    if let Some(row) = w.embed {
        upsert!(ctx.db.public_embed_config(), org_id, row);
    }
    write_table!(ctx.db.flow_node(), w.flow_nodes);
    write_table!(ctx.db.transaction(), w.transactions);
    write_table!(ctx.db.node_visibility(), w.node_visibility);
    write_table!(ctx.db.transaction_visibility(), w.transaction_visibility);
    write_table!(ctx.db.trust_score_history(), w.trust_history);
    write_table!(ctx.db.flag(), w.flags);
    write_table!(ctx.db.access_group(), w.access_groups);
    write_table!(ctx.db.access_group_member(), w.group_members);
    write_table!(ctx.db.verification(), w.verifications);
    write_table!(ctx.db.verification_access_log(), w.access_log);
    write_table!(ctx.db.campaign(), w.campaigns);
    write_table!(ctx.db.campaign_stat(), w.campaign_stats);
    write_table!(ctx.db.donation(), w.donations);
    write_table!(ctx.db.donor_update(), w.donor_updates);
    Ok(())
}

/// Rewrites the organisation's public rows from its records ([`super::public::project`]).
pub(super) fn publish(
    ctx: &ReducerContext,
    org_id: &str,
    records: &OrgTransparency,
) -> Result<(), String> {
    let org = org_id.to_owned();
    let before = PublicRows {
        organization: ctx.db.public_organization().org_id().find(&org),
        badge: ctx.db.public_badge().org_id().find(&org),
        campaigns: ctx.db.public_campaign().org_id().filter(&org).collect(),
    };
    let after = public_rows(org_id, &super::public::project(org_id, records)).map_err(text)?;
    let w = plan_public_write(&before, after);
    match w.organization {
        OneRow::Keep => {}
        OneRow::Put(row) => upsert!(ctx.db.public_organization(), org_id, row),
        OneRow::Delete => {
            ctx.db.public_organization().org_id().delete(&org);
        }
    }
    match w.badge {
        OneRow::Keep => {}
        OneRow::Put(row) => upsert!(ctx.db.public_badge(), org_id, row),
        OneRow::Delete => {
            ctx.db.public_badge().org_id().delete(&org);
        }
    }
    for id in &w.campaigns.delete {
        ctx.db.public_campaign().campaign_id().delete(id);
    }
    for row in w.campaigns.update {
        ctx.db.public_campaign().campaign_id().update(row);
    }
    for row in w.campaigns.insert {
        ctx.db.public_campaign().try_insert(row).map_err(text)?;
    }
    Ok(())
}

/// Who a write is by, for the trust row's `computed_by`: the member's user id, a service's
/// signature, else the identity.
fn author(ctx: &ReducerContext, caller: Option<&crate::tables::Member>) -> String {
    caller.map_or_else(
        || {
            ctx.db.service().identity().find(ctx.sender()).map_or_else(
                || ctx.sender().to_hex().to_string(),
                |s| super::services::signature(&s.label),
            )
        },
        |m| m.user_id.clone(),
    )
}

/// Runs one [`super::plan`] function on the organisation's records as the caller and writes back
/// what changed, with the trust score computed again when an input moved and the public rows
/// rewritten. `base` is the books' settings, for the defaults of a first profile.
pub(super) fn change<T>(
    ctx: &ReducerContext,
    org_id: &str,
    f: impl FnOnce(&mut OrgTransparency, &mut Ctx<'_>, &OrgLedgerSettings) -> Result<T, String>,
) -> Result<T, String> {
    let org = ctx
        .db
        .ledger_org()
        .org_id()
        .find(org_id.to_owned())
        .ok_or_else(|| no_org(org_id))?;
    let settings: OrgLedgerSettings = serde_json::from_str(&org.settings_json).map_err(text)?;
    let caller = ctx.db.member().key().find(member_key(org_id, ctx.sender()));
    let rows = load_rows(ctx, org_id);
    let mut records = from_rows(&rows).map_err(text)?;
    let mut env = reducer_env(ctx)?;
    let out = {
        let mut c = Ctx {
            org_id,
            identity: ctx.sender(),
            member: caller.as_ref(),
            env: &mut env,
        };
        f(&mut records, &mut c, &settings)?
    };
    env.finish()?;
    let now = reducer_time(ctx.timestamp).map_err(text)?.to_string();
    super::trust::refresh(&mut records, org_id, &now, &author(ctx, caller.as_ref()));
    write(ctx, plan_write(org_id, &rows, &records).map_err(text)?)?;
    publish(ctx, org_id, &records)?;
    Ok(out)
}

/// A JSON argument as the module's input type.
pub(super) fn parse<T: DeserializeOwned>(what: &str, json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|e| format!("The {what} is malformed: {e}"))
}

/// Whether another organisation's campaign has this public link.
fn campaign_slug_elsewhere(ctx: &ReducerContext, org_id: &str) -> impl Fn(&str) -> bool {
    let slugs: Vec<String> = ctx
        .db
        .campaign()
        .iter()
        .filter(|r| r.org_id != org_id)
        .filter_map(|r| row_to_doc(&r).ok())
        .filter_map(|c| c.public_slug)
        .collect();
    move |s| slugs.iter().any(|x| x == s)
}

/// Whether another organisation's profile has this public link.
fn profile_slug_elsewhere(ctx: &ReducerContext, org_id: &str) -> impl Fn(&str) -> bool {
    let slugs: Vec<String> = ctx
        .db
        .organization_profile()
        .iter()
        .filter(|r| r.org_id != org_id)
        .filter_map(|r| profile_of(&r).ok())
        .filter_map(|p| p.public_slug)
        .collect();
    move |s| slugs.iter().any(|x| x == s)
}

/// The organisation's journal entries, expense claims and bills, for proof, flag and
/// transaction links.
pub(super) struct Books<'a> {
    /// The call.
    pub(super) ctx: &'a ReducerContext,
    /// The organisation.
    pub(super) org_id: &'a str,
}

impl EntryLinks for Books<'_> {
    fn has_entry(&self, id: &str) -> bool {
        self.ctx
            .db
            .journal_entry()
            .key()
            .find(row_key(self.org_id, id))
            .is_some()
    }
}

impl LedgerLinks for Books<'_> {
    fn has_claim(&self, id: &str) -> bool {
        self.ctx
            .db
            .expense_claim()
            .key()
            .find(row_key(self.org_id, id))
            .is_some()
    }

    fn has_bill(&self, id: &str) -> bool {
        self.ctx
            .db
            .bill()
            .key()
            .find(row_key(self.org_id, id))
            .is_some()
    }
}

/// Sets the caller's own profile (`createOrUpdateProfile`, `supabase.ts:311-330`): display name
/// and the hash of their email; their user id is their JWT subject, else their identity.
///
/// # Errors
///
/// A stored profile or the clock that does not read.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn set_profile(
    ctx: &ReducerContext,
    display_name: String,
    email: String,
) -> Result<(), String> {
    let existing = ctx.db.profile().identity().find(ctx.sender());
    let before: Option<Profile> = existing
        .as_ref()
        .map(|r| serde_json::from_str(&r.data))
        .transpose()
        .map_err(text)?;
    let user_id = ctx.sender_auth().jwt().map_or_else(
        || ctx.sender().to_hex().to_string(),
        |jwt| jwt.subject().to_owned(),
    );
    let now = reducer_time(ctx.timestamp).map_err(text)?.to_string();
    let p = plan::set_profile(before, ctx.sender(), &user_id, &display_name, &email, &now);
    let row = ProfileRow {
        identity: ctx.sender(),
        data: serde_json::to_string(&p).map_err(text)?,
    };
    if existing.is_some() {
        ctx.db.profile().identity().update(row);
    } else {
        ctx.db.profile().try_insert(row).map_err(text)?;
    }
    Ok(())
}

/// `updateOrganization` (`api.ts:162-180`), [`plan::update_organization`]; `patch_json` is an
/// [`OrganizationPatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`plan::update_organization`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_organization_profile(
    ctx: &ReducerContext,
    org_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: OrganizationPatch = parse("organisation update", &patch_json)?;
    let taken = profile_slug_elsewhere(ctx, &org_id);
    change(ctx, &org_id, |t, c, books| {
        plan::update_organization(t, c, &books.base_currency, patch, &taken).map(drop)
    })
}

/// The five default access groups, [`plan::create_default_access_groups`].
///
/// # Errors
///
/// No books, or a rule of [`plan::create_default_access_groups`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_default_access_groups(ctx: &ReducerContext, org_id: String) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::create_default_access_groups(t, c).map(drop)
    })
}

/// `createAccessGroup` (`api.ts:585-596`), [`plan::create_access_group`]; `group_json` is a
/// [`NewAccessGroup`].
///
/// # Errors
///
/// No books, a malformed group, or a rule of [`plan::create_access_group`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_access_group(
    ctx: &ReducerContext,
    org_id: String,
    group_json: String,
) -> Result<(), String> {
    let input: NewAccessGroup = parse("access group", &group_json)?;
    change(ctx, &org_id, |t, c, _| {
        plan::create_access_group(t, c, input).map(drop)
    })
}

/// `updateAccessGroup` (`api.ts:598-609`), [`plan::update_access_group`]; `patch_json` is an
/// [`AccessGroupPatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`plan::update_access_group`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_access_group(
    ctx: &ReducerContext,
    org_id: String,
    group_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: AccessGroupPatch = parse("access group update", &patch_json)?;
    change(ctx, &org_id, |t, c, _| {
        plan::update_access_group(t, c, &group_id, patch).map(drop)
    })
}

/// `deleteAccessGroup` (`api.ts:611-616`), [`plan::delete_access_group`].
///
/// # Errors
///
/// No books, or a rule of [`plan::delete_access_group`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn delete_access_group(
    ctx: &ReducerContext,
    org_id: String,
    group_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::delete_access_group(t, c, &group_id)
    })
}

/// `addGroupMember` (`api.ts:654-658`), [`plan::add_group_member`]; `member_json` is a
/// [`NewGroupMember`].
///
/// # Errors
///
/// No books, a malformed member, or a rule of [`plan::add_group_member`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn add_access_group_member(
    ctx: &ReducerContext,
    org_id: String,
    member_json: String,
) -> Result<(), String> {
    let input: NewGroupMember = parse("group member", &member_json)?;
    change(ctx, &org_id, |t, c, _| {
        plan::add_group_member(t, c, input).map(drop)
    })
}

/// `inviteMemberByEmail` (`api.ts:666-683`), [`plan::invite_group_member`].
///
/// # Errors
///
/// No books, or a rule of [`plan::invite_group_member`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn invite_access_group_member(
    ctx: &ReducerContext,
    org_id: String,
    group_id: String,
    email: String,
    valid_until: Option<String>,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::invite_group_member(t, c, &group_id, &email, valid_until).map(drop)
    })
}

/// `removeGroupMember` (`api.ts:660-663`), [`plan::remove_group_member`].
///
/// # Errors
///
/// No books, or a rule of [`plan::remove_group_member`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn remove_access_group_member(
    ctx: &ReducerContext,
    org_id: String,
    member_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::remove_group_member(t, c, &member_id)
    })
}

/// `createVerification` (`api.ts:309-317`), [`plan::create_verification`]; `verification_json`
/// is a [`NewVerification`].
///
/// # Errors
///
/// No books, a malformed proof, or a rule of [`plan::create_verification`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_verification(
    ctx: &ReducerContext,
    org_id: String,
    verification_json: String,
) -> Result<(), String> {
    let input: NewVerification = parse("proof", &verification_json)?;
    let books = Books {
        ctx,
        org_id: &org_id,
    };
    change(ctx, &org_id, |t, c, _| {
        plan::create_verification(t, c, input, &books).map(drop)
    })
}

/// [`plan::mark_published`]: shows a proof at the proofs Worker's public path, or takes it back.
///
/// # Errors
///
/// No books, or a rule of [`plan::mark_published`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn mark_published(
    ctx: &ReducerContext,
    org_id: String,
    verification_id: String,
    published: bool,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::mark_published(t, c, &verification_id, published).map(drop)
    })
}

/// `logVerificationAccess` (`supabase.ts:554-571`), [`plan::log_verification_access`];
/// `access_type` is `view` or `download`.
///
/// # Errors
///
/// No books, an unknown access type, or a rule of [`plan::log_verification_access`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn log_verification_access(
    ctx: &ReducerContext,
    org_id: String,
    verification_id: String,
    access_type: String,
) -> Result<(), String> {
    let kind: AccessType = parse(
        "access type",
        &serde_json::Value::String(access_type).to_string(),
    )?;
    change(ctx, &org_id, |t, c, _| {
        plan::log_verification_access(t, c, &verification_id, kind).map(drop)
    })
}

/// `createCampaign` (`api.ts:1080-1098`), [`plan::create_campaign`]; `campaign_json` is a
/// [`NewCampaign`].
///
/// # Errors
///
/// No books, a malformed campaign, or a rule of [`plan::create_campaign`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn create_campaign(
    ctx: &ReducerContext,
    org_id: String,
    campaign_json: String,
) -> Result<(), String> {
    let input: NewCampaign = parse("campaign", &campaign_json)?;
    let taken = campaign_slug_elsewhere(ctx, &org_id);
    change(ctx, &org_id, |t, c, _| {
        plan::create_campaign(t, c, input, &taken).map(drop)
    })
}

/// `updateCampaign` (`api.ts:1100-1114`), [`plan::update_campaign`]; `patch_json` is a
/// [`CampaignPatch`].
///
/// # Errors
///
/// No books, a malformed patch, or a rule of [`plan::update_campaign`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_campaign(
    ctx: &ReducerContext,
    org_id: String,
    campaign_id: String,
    patch_json: String,
) -> Result<(), String> {
    let patch: CampaignPatch = parse("campaign update", &patch_json)?;
    let taken = campaign_slug_elsewhere(ctx, &org_id);
    change(ctx, &org_id, |t, c, _| {
        plan::update_campaign(t, c, &campaign_id, patch, &taken).map(drop)
    })
}

/// `deleteCampaign` (`api.ts:1116-1126`), [`plan::delete_campaign`].
///
/// # Errors
///
/// No books, or a rule of [`plan::delete_campaign`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn delete_campaign(
    ctx: &ReducerContext,
    org_id: String,
    campaign_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::delete_campaign(t, c, &campaign_id)
    })
}

/// `createDonation` (`api.ts:1256-1272`), [`plan::record_donation`]; `donation_json` is a
/// [`NewDonation`].
///
/// # Errors
///
/// No books, a malformed donation, or a rule of [`plan::record_donation`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn record_donation(
    ctx: &ReducerContext,
    org_id: String,
    donation_json: String,
) -> Result<(), String> {
    let input: NewDonation = parse("donation", &donation_json)?;
    change(ctx, &org_id, |t, c, _| {
        plan::record_donation(t, c, input).map(drop)
    })
}

/// `updateDonationStatus` (`api.ts:1274-1289`), [`plan::update_donation_status`]; `status` is
/// `pending`, `confirmed`, `failed` or `refunded`.
///
/// # Errors
///
/// No books, an unknown status, or a rule of [`plan::update_donation_status`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn update_donation_status(
    ctx: &ReducerContext,
    org_id: String,
    donation_id: String,
    status: String,
    payment_reference: Option<String>,
) -> Result<(), String> {
    let status: PaymentStatus = parse(
        "payment status",
        &serde_json::Value::String(status).to_string(),
    )?;
    change(ctx, &org_id, |t, c, _| {
        plan::update_donation_status(t, c, &donation_id, status, payment_reference).map(drop)
    })
}

/// `createDonorUpdate` (`api.ts:1329-1341`), [`plan::post_donor_update`]; `update_json` is a
/// [`NewDonorUpdate`].
///
/// # Errors
///
/// No books, a malformed update, or a rule of [`plan::post_donor_update`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn post_donor_update(
    ctx: &ReducerContext,
    org_id: String,
    update_json: String,
) -> Result<(), String> {
    let input: NewDonorUpdate = parse("donor update", &update_json)?;
    change(ctx, &org_id, |t, c, _| {
        plan::post_donor_update(t, c, input).map(drop)
    })
}

/// `markUpdateSent` (`api.ts:1343-1349`), [`plan::mark_update_sent`].
///
/// # Errors
///
/// No books, or a rule of [`plan::mark_update_sent`].
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn mark_donor_update_sent(
    ctx: &ReducerContext,
    org_id: String,
    update_id: String,
) -> Result<(), String> {
    change(ctx, &org_id, |t, c, _| {
        plan::mark_update_sent(t, c, &update_id).map(drop)
    })
}

/// Replaces an organisation's transparency records with a dataset in the demo store's TS shape
/// (`rust/fixtures/transparency-dataset.json#/store`, mapped by [`super::ts::dataset`]); no TS
/// counterpart, like `import_books`, and only an operator may call it. Profiles stay. The
/// dataset's chains that the organisation is on either side of replace the chains it is on
/// ([`super::ts::chains`]); the trust row is computed and the public rows written.
///
/// # Errors
///
/// Not an operator, no books, or a dataset that does not map.
#[spacetimedb::reducer]
#[expect(
    clippy::needless_pass_by_value,
    reason = "reducer arguments arrive by value from the SpacetimeDB host"
)]
pub fn import_transparency(
    ctx: &ReducerContext,
    org_id: String,
    store_json: String,
) -> Result<(), String> {
    if !is_operator(ctx, ctx.sender()) {
        return Err("Only an operator can import transparency records.".to_owned());
    }
    if ctx.db.ledger_org().org_id().find(&org_id).is_none() {
        return Err(no_org(&org_id));
    }
    let store: serde_json::Value = parse("dataset", &store_json)?;
    let records = super::ts::dataset(&org_id, &store).map_err(text)?;
    ctx.db.organization_profile().org_id().delete(&org_id);
    ctx.db.access_group().org_id().delete(&org_id);
    ctx.db.access_group_member().org_id().delete(&org_id);
    ctx.db.verification().org_id().delete(&org_id);
    ctx.db.verification_access_log().org_id().delete(&org_id);
    ctx.db.campaign().org_id().delete(&org_id);
    ctx.db.campaign_stat().org_id().delete(&org_id);
    ctx.db.donation().org_id().delete(&org_id);
    ctx.db.donor_update().org_id().delete(&org_id);
    ctx.db.flow_node().org_id().delete(&org_id);
    ctx.db.transaction().org_id().delete(&org_id);
    ctx.db.node_visibility().org_id().delete(&org_id);
    ctx.db.transaction_visibility().org_id().delete(&org_id);
    ctx.db.trust_score().org_id().delete(&org_id);
    ctx.db.trust_score_history().org_id().delete(&org_id);
    ctx.db.flag().org_id().delete(&org_id);
    ctx.db.public_embed_config().org_id().delete(&org_id);
    let mut records = records;
    let now = reducer_time(ctx.timestamp).map_err(text)?.to_string();
    super::trust::refresh(&mut records, &org_id, &now, "module");
    write(
        ctx,
        plan_write(&org_id, &TransparencyRows::default(), &records).map_err(text)?,
    )?;
    publish(ctx, &org_id, &records)?;
    let before = super::chain_reducers::load_chains(ctx, &[org_id.as_str()]);
    let after = super::ts::chains(&org_id, &store).map_err(text)?;
    super::chain_reducers::write_chains(ctx, &before, &after)
}
