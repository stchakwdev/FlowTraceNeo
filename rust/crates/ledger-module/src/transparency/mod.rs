//! The transparency side of the module (task MOD1, pre-decision P12: in the same module as the
//! ledger): organisation and person profiles, access groups and their members, proofs and who
//! looked at them, campaigns with their figures, donation records and donor updates.
//!
//! - [`types`]: the records (the TS types of `flowtrace/src/lib/supabase.ts`, money in minor
//!   units, files as R2 object keys) and the reducers' inputs.
//! - [`plan`]: the rules of each write as pure functions over one organisation's records
//!   ([`OrgTransparency`]), and who may see which proof.
//! - [`stats`]: campaign figures.
//! - [`ts`]: TS-shaped JSON into the module's records (datasets, the demo API's arguments).
//! - [`rows`]: records to table rows and back, and the row writes a change needs.
//! - [`tables`], [`views`], [`reducers`]: the SpacetimeDB side.
//!
//! Row shapes and the rules' sources: `docs/adr/0004-transparency-tables.md`.

pub mod plan;
pub mod reducers;
pub mod rows;
pub mod stats;
pub mod tables;
pub mod ts;
pub mod types;
pub mod views;

#[cfg(test)]
mod plan_tests;

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
}
