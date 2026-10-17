//! The transparency tables (MOD1). Additive: no ledger table changes.
//!
//! Every table is private, like the ledger's; clients read through the `my_*` views of
//! [`super::views`]. The eight organisation tables are doc rows in the ledger's shape
//! (`crate::tables::DocRow`, `docs/adr/0003-module-tables.md`): `key` (`{org_id}:{id}`), `org_id`,
//! `id`, `seq` (insertion order within the organisation) and the record as JSON in `data`, so a
//! record can gain fields without a schema change. No view needs a typed column beyond `org_id`:
//! the membership views join on it, and the proof, access-log and profile views read the JSON.
//! `organization_profile` has one row per organisation and `profile` one per identity.
//!
//! MOD2 adds, additively: six more organisation doc tables (`flow_node`, `transaction`,
//! `node_visibility`, `transaction_visibility`, `trust_score_history`, `flag`), two more
//! one-per-organisation rows (`trust_score`, `public_embed_config`), the two-organisation chain
//! tables (`organization_chain` with both organisations as typed columns, `chain_activity_log`),
//! `service`, and three public tables anyone may read without an identity (`public_organization`,
//! `public_campaign`, `public_badge`; P20). Shapes: `docs/adr/0005-transparency-tables-part-2.md`.

use spacetimedb::Identity;

use super::flows::{FlowNode, NodeVisibility, Transaction, TransactionVisibility};
use super::trust::{Flag, TrustHistory};
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

/// A node of the flow map (`FlowNode`, `supabase.ts:52-66`), balance in minor units (MOD2).
#[spacetimedb::table(accessor = flow_node)]
#[derive(Debug, Clone, PartialEq)]
pub struct FlowNodeRow {
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

/// A transaction (`Transaction`, `supabase.ts:68-87`), amounts in minor units, with the journal
/// entries that record it (MOD2).
#[spacetimedb::table(accessor = transaction)]
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionRow {
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

/// What one access group sees of one node (`NodeVisibility`, `supabase.ts:225-235`; MOD2).
#[spacetimedb::table(accessor = node_visibility)]
#[derive(Debug, Clone, PartialEq)]
pub struct NodeVisibilityRow {
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

/// What one access group sees of one transaction (`TransactionVisibility`,
/// `supabase.ts:237-246`; MOD2).
#[spacetimedb::table(accessor = transaction_visibility)]
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionVisibilityRow {
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

/// A trust score snapshot (`TrustScoreHistory`, `supabase.ts:164-173`; MOD2).
#[spacetimedb::table(accessor = trust_score_history)]
#[derive(Debug, Clone, PartialEq)]
pub struct TrustHistoryRow {
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

/// A finding of the model service on a transaction, claim or bill (`Flag`; MOD2, P15).
#[spacetimedb::table(accessor = flag)]
#[derive(Debug, Clone, PartialEq)]
pub struct FlagRow {
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

/// The organisation's trust score (`super::trust::TrustScore`), one row per organisation (P16).
#[spacetimedb::table(accessor = trust_score)]
#[derive(Debug, Clone, PartialEq)]
pub struct TrustScoreRow {
    /// The organisation.
    #[primary_key]
    pub org_id: String,
    /// `TrustScore` as JSON.
    pub data: String,
}

/// The organisation's public embed settings (`PublicEmbedConfig`, `supabase.ts:248-271`), one
/// row per organisation.
#[spacetimedb::table(accessor = public_embed_config)]
#[derive(Debug, Clone, PartialEq)]
pub struct EmbedConfigRow {
    /// The organisation.
    #[primary_key]
    pub org_id: String,
    /// `PublicEmbedConfig` as JSON.
    pub data: String,
}

/// A chain from one organisation to another (`OrganizationChain`, `supabase.ts:1088-1105`); both
/// organisations are typed columns so each side's view can find it.
#[spacetimedb::table(accessor = organization_chain)]
#[derive(Debug, Clone, PartialEq)]
pub struct ChainRow {
    /// The chain id.
    #[primary_key]
    pub id: String,
    /// The organisation that asked.
    #[index(btree)]
    pub source_org_id: String,
    /// The organisation asked.
    #[index(btree)]
    pub target_org_id: String,
    /// Insertion order among all chains.
    pub seq: u64,
    /// `OrganizationChain` as JSON (`ChainDetails` on `my_organization_chain` rows).
    pub data: String,
}

/// One step of a chain (`ChainActivityLog`, `supabase.ts:1122-1130`).
#[spacetimedb::table(accessor = chain_activity_log)]
#[derive(Debug, Clone, PartialEq)]
pub struct ChainActivityRow {
    /// The entry id.
    #[primary_key]
    pub id: String,
    /// The chain.
    #[index(btree)]
    pub chain_id: String,
    /// Insertion order among all entries.
    pub seq: u64,
    /// `ChainActivity` as JSON.
    pub data: String,
}

/// A registered service (P17): an identity that may call the reducers its scopes name.
#[spacetimedb::table(accessor = service)]
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceRow {
    /// The service's identity.
    #[primary_key]
    pub identity: Identity,
    /// What it is, for example `jev-worker`.
    pub label: String,
    /// `flags`, `rubric`, `trust` (`super::services::SCOPES`).
    pub scopes: Vec<String>,
    /// The operator who registered it.
    pub registered_by: Identity,
    /// When, ISO 8601 with milliseconds.
    pub registered_at: String,
}

/// A published organisation (`super::public::PublicOrganization`); anyone may read it.
#[spacetimedb::table(accessor = public_organization, public)]
#[derive(Debug, Clone, PartialEq)]
pub struct PublicOrganizationRow {
    /// The organisation.
    #[primary_key]
    pub org_id: String,
    /// Its public link, `""` for none.
    #[index(btree)]
    pub slug: String,
    /// `PublicOrganization` as JSON.
    pub data: String,
}

/// A published campaign (`super::public::PublicCampaign`); anyone may read it.
#[spacetimedb::table(accessor = public_campaign, public)]
#[derive(Debug, Clone, PartialEq)]
pub struct PublicCampaignRow {
    /// The campaign.
    #[primary_key]
    pub campaign_id: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// Its public link (`/donate/{slug}`).
    #[index(btree)]
    pub slug: String,
    /// `PublicCampaign` as JSON.
    pub data: String,
}

/// A published trust badge (`super::public::PublicBadge`); anyone may read it.
#[spacetimedb::table(accessor = public_badge, public)]
#[derive(Debug, Clone, PartialEq)]
pub struct PublicBadgeRow {
    /// The organisation.
    #[primary_key]
    pub org_id: String,
    /// `PublicBadge` as JSON.
    pub data: String,
}

transparency_doc_row! {
    "flow_node", FlowNodeRow, FlowNode, id;
    "transaction", TransactionRow, Transaction, id;
    "node_visibility", NodeVisibilityRow, NodeVisibility, id;
    "transaction_visibility", TransactionVisibilityRow, TransactionVisibility, id;
    "trust_score_history", TrustHistoryRow, TrustHistory, id;
    "flag", FlagRow, Flag, id;
    "verification", VerificationRow, Verification, id;
    "verification_access_log", VerificationAccessRow, VerificationAccess, id;
    "access_group", AccessGroupRow, AccessGroup, id;
    "access_group_member", AccessGroupMemberRow, GroupMember, id;
    "campaign", CampaignRow, Campaign, id;
    "campaign_stat", CampaignStatRow, CampaignStat, campaign_id;
    "donation", DonationRow, Donation, id;
    "donor_update", DonorUpdateRow, DonorUpdate, id;
}
