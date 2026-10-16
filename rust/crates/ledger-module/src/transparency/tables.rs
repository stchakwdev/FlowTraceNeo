//! The transparency tables (MOD1). Additive: no ledger table changes.
//!
//! Every table is private, like the ledger's; clients read through the `my_*` views of
//! [`super::views`]. The eight organisation tables are doc rows in the ledger's shape
//! (`crate::tables::DocRow`, `docs/adr/0003-module-tables.md`): `key` (`{org_id}:{id}`), `org_id`,
//! `id`, `seq` (insertion order within the organisation) and the record as JSON in `data`, so a
//! record can gain fields without a schema change. No view needs a typed column beyond `org_id`:
//! the membership views join on it, and the proof, access-log and profile views read the JSON.
//! `organization_profile` has one row per organisation and `profile` one per identity.

use spacetimedb::Identity;

use super::types::{
    AccessGroup, Campaign, CampaignStat, Donation, DonorUpdate, GroupMember, Verification,
    VerificationAccess,
};
use crate::tables::{Doc, DocRow};

/// An organisation's public profile (`Organization`, `supabase.ts:28-40`): one row per
/// organisation, the record as JSON in `data`.
#[spacetimedb::table(accessor = organization_profile)]
#[derive(Debug, Clone, PartialEq)]
pub struct OrganizationProfileRow {
    /// The organisation.
    #[primary_key]
    pub org_id: String,
    /// `OrganizationProfile` as JSON.
    pub data: String,
}

/// One person's profile (`Profile`, `supabase.ts:42-50`): one row per identity, the record as
/// JSON in `data`. Written only by that identity (`set_profile`).
#[spacetimedb::table(accessor = profile)]
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileRow {
    /// The person.
    #[primary_key]
    pub identity: Identity,
    /// `Profile` as JSON.
    pub data: String,
}

// The eight organisation tables. Spelled out: the table attribute does not survive
// `macro_rules!` hygiene (`crate::tables`), so only the trait impls come from a macro.

/// A proof (`Verification`, `supabase.ts:128-151`); clients read it through `my_verification`, which gives each reader only what their access groups allow.
#[spacetimedb::table(accessor = verification)]
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// One look at a proof (`VerificationAccessLog`, `supabase.ts:153-162`).
#[spacetimedb::table(accessor = verification_access_log)]
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationAccessRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// An access group (`AccessGroup`, `supabase.ts:191-206`).
#[spacetimedb::table(accessor = access_group)]
#[derive(Debug, Clone, PartialEq)]
pub struct AccessGroupRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// A person in an access group (`GroupMember`, `supabase.ts:208-218`); `org_id` is the group's organisation.
#[spacetimedb::table(accessor = access_group_member)]
#[derive(Debug, Clone, PartialEq)]
pub struct AccessGroupMemberRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// A campaign (`Campaign`, `supabase.ts:1390-1415`), amounts in minor units.
#[spacetimedb::table(accessor = campaign)]
#[derive(Debug, Clone, PartialEq)]
pub struct CampaignRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// A campaign's figures (`CampaignStats`, `supabase.ts:1417-1427`), keyed by the campaign id.
#[spacetimedb::table(accessor = campaign_stat)]
#[derive(Debug, Clone, PartialEq)]
pub struct CampaignStatRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// A donation record (`Donation`, `supabase.ts:1442-1469`), amounts in minor units; `org_id` is the campaign's organisation.
#[spacetimedb::table(accessor = donation)]
#[derive(Debug, Clone, PartialEq)]
pub struct DonationRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// A message to a donor (`DonorUpdate`, `supabase.ts:1476-1490`).
#[spacetimedb::table(accessor = donor_update)]
#[derive(Debug, Clone, PartialEq)]
pub struct DonorUpdateRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position among the organisation's rows of this table.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// Implements [`DocRow`] for a table and [`Doc`] for its record, keyed by `$id`.
macro_rules! transparency_doc_row {
    ($($table:literal, $row:ident, $record:ty, $id:ident;)+) => {$(
        impl DocRow for $row {
            type Doc = $record;

            const TABLE: &'static str = $table;

            fn from_columns(key: String, org_id: String, id: String, seq: u64, data: String) -> Self {
                Self { key, org_id, id, seq, data }
            }

            fn key(&self) -> &str {
                &self.key
            }

            fn seq(&self) -> u64 {
                self.seq
            }

            fn data(&self) -> &str {
                &self.data
            }
        }

        impl Doc for $record {
            fn doc_id(&self) -> &str {
                &self.$id
            }
        }
    )+};
}

transparency_doc_row! {
    "verification", VerificationRow, Verification, id;
    "verification_access_log", VerificationAccessRow, VerificationAccess, id;
    "access_group", AccessGroupRow, AccessGroup, id;
    "access_group_member", AccessGroupMemberRow, GroupMember, id;
    "campaign", CampaignRow, Campaign, id;
    "campaign_stat", CampaignStatRow, CampaignStat, campaign_id;
    "donation", DonationRow, Donation, id;
    "donor_update", DonorUpdateRow, DonorUpdate, id;
}
