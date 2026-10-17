//! The published projection (pre-decision P20): what the web shows to people who are not
//! members, kept in public tables that anyone can read without an identity, and rewritten from an
//! organisation's records by every write to them.
//!
//! - **Organisation** (`public_organization`): the profile fields the public pages show and the
//!   trust score with its grade, for an organisation whose profile is public (`is_public`, which
//!   starts as "verified and has a public slug", the policy "Public can view verified
//!   organizations", `flowtrace/supabase/migrations/002_rls_policies.sql:40-42`).
//! - **Badge** (`public_badge`): what the trust badge draws (`TrustBadgePreview` through
//!   `useTrustScore`, `flowtrace/src/lib/queries.ts:266-290`): score, grade, breakdown and when,
//!   for the same organisations. The web reads the latest history snapshot; the module's row is
//!   kept current, so the badge shows the current score (P16: the breakdown is shown on the badge).
//! - **Campaign** (`public_campaign`): what the donate page and the campaign embed show
//!   (`flowtrace/src/routes/donate/$slug.tsx`, `components/campaign-embed-widget.tsx`): the
//!   campaign, its figures and its report (`getCampaignReport`, `api.ts:1381-1443`), for a campaign
//!   open to the public (`getCampaignBySlug`, `api.ts:1059-1066`). Donors and their gifts, the
//!   campaign's metadata and who made it are not in it; spending lines carry what the page draws
//!   (`components/public-campaign-header.tsx:113-136`).

use std::collections::BTreeMap;

use ledger_core::types::Minor;
use serde::{Deserialize, Serialize};

use super::OrgTransparency;
use super::flows::{
    Earmark, NodeRef, ReportOrganization, TransactionStatus, campaign_report, is_open_to_public,
    stat_of,
};
use super::types::{CampaignStat, CampaignStatus, OrgType};

/// A published organisation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicOrganization {
    /// Organisation id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Kind.
    #[serde(rename = "type")]
    pub org_type: Option<OrgType>,
    /// Country.
    pub country: String,
    /// Currency.
    pub currency: String,
    /// Checked by FlowTrace.
    pub verified: bool,
    /// Public link.
    pub public_slug: Option<String>,
    /// About it.
    pub description: Option<String>,
    /// Logo object key.
    pub logo_key: Option<String>,
    /// Trust score, when computed.
    pub trust_score: Option<i64>,
    /// Its grade.
    pub grade: Option<String>,
    /// When the profile last changed.
    pub updated_at: String,
}

/// A published trust badge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicBadge {
    /// Organisation id.
    pub organization_id: String,
    /// Name.
    pub name: String,
    /// Public link.
    pub public_slug: Option<String>,
    /// Score.
    pub score: i64,
    /// Grade.
    pub grade: String,
    /// The rounded components by name.
    pub breakdown: BTreeMap<String, i64>,
    /// When the score was computed.
    pub calculated_at: String,
}

/// The campaign fields a donate page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicCampaignFields {
    /// Campaign id.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Name.
    pub name: String,
    /// Long description.
    pub description: Option<String>,
    /// One line.
    pub short_description: Option<String>,
    /// Image.
    pub image_url: Option<String>,
    /// Target, minor units.
    pub target_amount: Minor,
    /// Raised, minor units.
    pub current_amount: Minor,
    /// Currency.
    pub currency: String,
    /// Where the money goes.
    pub target_node: Option<NodeRef>,
    /// Status.
    pub status: CampaignStatus,
    /// Takes more than its target.
    pub allow_overflow: bool,
    /// Asked of donors before overflow.
    pub overflow_consent_message: Option<String>,
    /// Opens.
    pub starts_at: String,
    /// Closes.
    pub ends_at: Option<String>,
    /// Link.
    pub public_slug: Option<String>,
    /// Category.
    pub category: Option<String>,
    /// Tags.
    pub tags: Option<Vec<String>>,
}

/// One spending line on a donate page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicSpending {
    /// Transaction id.
    pub id: String,
    /// What it was.
    pub description: String,
    /// Amount, minor units.
    pub amount: Minor,
    /// Currency.
    pub currency: String,
    /// Date.
    pub transaction_date: String,
    /// Proof state.
    pub verification_status: TransactionStatus,
    /// Proofs attached.
    pub verification_count: u64,
    /// From.
    pub from_node: Option<NodeRef>,
    /// To.
    pub to_node: Option<NodeRef>,
}

/// A published campaign: its fields, figures and report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicCampaign {
    /// The campaign.
    pub campaign: PublicCampaignFields,
    /// Its figures (without the two day counts, which readers compute).
    pub stats: Option<CampaignStat>,
    /// Who runs it.
    pub organization: ReportOrganization,
    /// Verified money into the target node, capped at the raised amount.
    pub evidenced_amount: Option<Minor>,
    /// Programme nodes.
    pub earmarks: Vec<Earmark>,
    /// The six latest programme transactions.
    pub spending: Vec<PublicSpending>,
}

/// One organisation's published rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Projection {
    /// `public_organization`, when public.
    pub organization: Option<PublicOrganization>,
    /// `public_badge`, when public.
    pub badge: Option<PublicBadge>,
    /// `public_campaign`, one per campaign open to the public, in campaign order.
    pub campaigns: Vec<PublicCampaign>,
}

/// The organisation as a campaign report names it: its profile name (the id without one) and the
/// trust row's score (0 without one).
#[must_use]
pub fn report_organization(org_id: &str, t: &OrgTransparency) -> ReportOrganization {
    ReportOrganization {
        id: org_id.to_owned(),
        name: t
            .profile
            .as_ref()
            .map_or_else(|| org_id.to_owned(), |p| p.name.clone()),
        trust_score: t.trust.as_ref().map_or(0, |s| s.score),
        verified: t.profile.as_ref().is_some_and(|p| p.verified),
    }
}

/// What an organisation publishes.
#[must_use]
pub fn project(org_id: &str, t: &OrgTransparency) -> Projection {
    let public_profile = t.profile.as_ref().filter(|p| p.is_public);
    let organization = public_profile.map(|p| PublicOrganization {
        id: org_id.to_owned(),
        name: p.name.clone(),
        org_type: p.org_type,
        country: p.country.clone(),
        currency: p.currency.clone(),
        verified: p.verified,
        public_slug: p.public_slug.clone(),
        description: p.description.clone(),
        logo_key: p.logo_key.clone(),
        trust_score: t.trust.as_ref().map(|s| s.score),
        grade: t.trust.as_ref().map(|s| s.grade.clone()),
        updated_at: p.updated_at.clone(),
    });
    let badge = public_profile
        .zip(t.trust.as_ref())
        .map(|(p, s)| PublicBadge {
            organization_id: org_id.to_owned(),
            name: p.name.clone(),
            public_slug: p.public_slug.clone(),
            score: s.score,
            grade: s.grade.clone(),
            breakdown: s.breakdown.clone(),
            calculated_at: s.computed_at.clone(),
        });
    let who = report_organization(org_id, t);
    let campaigns = t
        .campaigns
        .iter()
        .filter(|c| is_open_to_public(c))
        .filter_map(|c| {
            let r = campaign_report(t, &c.id, who.clone())?;
            Some(PublicCampaign {
                campaign: PublicCampaignFields {
                    id: c.id.clone(),
                    organization_id: c.organization_id.clone(),
                    name: c.name.clone(),
                    description: c.description.clone(),
                    short_description: c.short_description.clone(),
                    image_url: c.image_url.clone(),
                    target_amount: c.target_amount,
                    current_amount: c.current_amount,
                    currency: c.currency.clone(),
                    target_node: r.campaign.target_node.clone(),
                    status: c.status,
                    allow_overflow: c.allow_overflow,
                    overflow_consent_message: c.overflow_consent_message.clone(),
                    starts_at: c.starts_at.clone(),
                    ends_at: c.ends_at.clone(),
                    public_slug: c.public_slug.clone(),
                    category: c.category.clone(),
                    tags: c.tags.clone(),
                },
                stats: stat_of(t, &c.id).cloned(),
                organization: r.organization,
                evidenced_amount: r.evidenced_amount,
                earmarks: r.earmarks,
                spending: r
                    .spending
                    .into_iter()
                    .map(|s| PublicSpending {
                        id: s.transaction.id,
                        description: s.transaction.description,
                        amount: s.transaction.amount,
                        currency: s.transaction.currency,
                        transaction_date: s.transaction.transaction_date,
                        verification_status: s.transaction.verification_status,
                        verification_count: s.transaction.verification_count,
                        from_node: s.from_node,
                        to_node: s.to_node,
                    })
                    .collect(),
            })
        })
        .collect();
    Projection {
        organization,
        badge,
        campaigns,
    }
}
