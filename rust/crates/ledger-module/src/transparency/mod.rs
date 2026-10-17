//! The transparency side of the module (tasks MOD1 and MOD2, pre-decision P12: in the same module
//! as the ledger): organisation and person profiles, access groups and their members, proofs and
//! who looked at them, campaigns with their figures, donation records and donor updates (MOD1);
//! flow nodes, transactions and their visibility, the public embed config, organisation chains,
//! the trust score with its history, flags, services and the public projection (MOD2).
//!
//! - [`types`]: the records (the TS types of `flowtrace/src/lib/supabase.ts`, money in minor
//!   units, files as R2 object keys) and the reducers' inputs.
//! - [`plan`]: the rules of each write as pure functions over one organisation's records
//!   ([`OrgTransparency`]), and who may see which proof.
//! - [`stats`]: campaign figures.
//! - [`flows`], [`chains`], [`trust`], [`services`], [`public`]: the MOD2 records and rules.
//! - [`ts`]: TS-shaped JSON into the module's records (datasets, the demo API's arguments).
//! - [`rows`]: records to table rows and back, and the row writes a change needs.
//! - [`tables`], [`views`], [`reducers`]: the SpacetimeDB side.
//!
//! Row shapes and the rules' sources: `docs/adr/0004-transparency-tables.md` (MOD1) and
//! `docs/adr/0005-transparency-tables-part-2.md` (MOD2).

pub mod chain_reducers;
pub mod chains;
pub mod flow_reducers;
pub mod flows;
pub mod plan;
pub mod public;
pub mod reducers;
pub mod rows;
pub mod services;
pub mod stats;
pub mod tables;
pub mod trust;
pub mod trust_reducers;
pub mod ts;
pub mod types;
pub mod views;

#[cfg(test)]
mod mod2_tests;
#[cfg(test)]
mod plan_tests;

use flows::{FlowNode, NodeVisibility, PublicEmbedConfig, Transaction, TransactionVisibility};
use trust::{Flag, TrustHistory, TrustScore};
use types::{
    AccessGroup, Campaign, CampaignStat, Donation, DonorUpdate, GroupMember, OrganizationProfile,
    Verification, VerificationAccess,
};

/// One organisation's transparency records, each collection in row (`seq`) order, as the demo
/// store holds them (`flowtrace/src/lib/demo/api.ts:49-68`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrgTransparency {
    /// `organization_profile`; `None` until the first update.
    pub profile: Option<OrganizationProfile>,
    /// `access_group`.
    pub access_groups: Vec<AccessGroup>,
    /// `access_group_member`.
    pub group_members: Vec<GroupMember>,
    /// `verification`.
    pub verifications: Vec<Verification>,
    /// `verification_access_log`.
    pub access_log: Vec<VerificationAccess>,
    /// `campaign`.
    pub campaigns: Vec<Campaign>,
    /// `campaign_stat`, one per campaign.
    pub campaign_stats: Vec<CampaignStat>,
    /// `donation`.
    pub donations: Vec<Donation>,
    /// `donor_update`.
    pub donor_updates: Vec<DonorUpdate>,
    /// `flow_node` (MOD2).
    pub flow_nodes: Vec<FlowNode>,
    /// `transaction` (MOD2).
    pub transactions: Vec<Transaction>,
    /// `node_visibility` (MOD2).
    pub node_visibility: Vec<NodeVisibility>,
    /// `transaction_visibility` (MOD2).
    pub transaction_visibility: Vec<TransactionVisibility>,
    /// `trust_score` (MOD2); `None` until the first write computes it.
    pub trust: Option<TrustScore>,
    /// `trust_score_history` (MOD2).
    pub trust_history: Vec<TrustHistory>,
    /// `flag` (MOD2).
    pub flags: Vec<Flag>,
    /// `public_embed_config` (MOD2); `None` until the first update.
    pub embed: Option<PublicEmbedConfig>,
}
