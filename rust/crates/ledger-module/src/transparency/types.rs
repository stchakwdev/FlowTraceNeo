//! The transparency records the module stores, and the inputs its reducers take.
//!
//! Each record has the field names and literals of its TS type in `flowtrace/src/lib/supabase.ts`
//! (`Organization`, `Profile`, `AccessGroup`, `GroupMember`, `Verification`,
//! `VerificationAccessLog`, `Campaign`, `CampaignStats`, `Donation`, `DonorUpdate`), so a doc row's
//! JSON reads like the TS record. Where the module differs, the field says so:
//!
//! - **Money is integer minor units**, as everywhere in the ledger (`flowtrace/src/lib/ledger/types.ts:5-6`).
//!   The TS transparency types hold major units (whole shillings in the demo dataset); [`super::ts`]
//!   converts with `fromMajor` (`flowtrace/src/lib/ledger/money.ts:26-31`).
//! - **Files are R2 object keys and a SHA-256** (pre-decision P13): `logo_url`, `file_url` and
//!   `thumbnail_url` become `logo_key`, `object_key` and `thumbnail_key`, and a verification gains
//!   `sha256`.
//! - **Who acted is the caller's member row**, never an argument: `created_by`, `granted_by`,
//!   `verified_by` and `accessed_by` are the member's user id (the TS takes them from the client).
//! - Organisation-level `trust_score` is MOD2's `trust_score` row; flows, transactions and chains
//!   are MOD2's too, so their ids (`target_node_id`, `transaction_id`, ...) are kept as opaque text.

use ledger_core::types::{Minor, Nullish};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Free-form JSON object (`Record<string, unknown>` in TS).
pub type Metadata = Map<String, Value>;

/// `Organization.type` (`supabase.ts:31`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrgType {
    /// `NGO`.
    Ngo,
    /// `SME`.
    Sme,
    /// `SOCIAL_ENTERPRISE`.
    SocialEnterprise,
    /// `GOVERNMENT`.
    Government,
}

/// An organisation's public profile (`Organization`, `supabase.ts:28-40`), one per organisation:
/// the ledger knows organisations only by id until now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfile {
    /// The organisation id (the ledger's `org_id`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// `NGO`, `SME`, `SOCIAL_ENTERPRISE` or `GOVERNMENT`.
    #[serde(rename = "type")]
    pub org_type: Option<OrgType>,
    /// ISO 3166 country code.
    pub country: String,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Checked by FlowTrace; members cannot set it (`OrganizationProfileUpdate`, `api.ts:158-160`).
    pub verified: bool,
    /// The public link (`/donate`, badge and embed pages), unique across organisations.
    pub public_slug: Option<String>,
    /// About the organisation; module addition.
    #[serde(default)]
    pub description: Option<String>,
    /// R2 object key of the logo; the TS `logo_url` (P13).
    #[serde(default)]
    pub logo_key: Option<String>,
    /// Shown on public pages; module addition. The TS rule it starts from is "verified and has a
    /// public slug" (`flowtrace/supabase/migrations/002_rls_policies.sql:40-42`).
    #[serde(default)]
    pub is_public: bool,
    /// When the profile was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// One person's profile, per identity (`Profile`, `supabase.ts:42-50`): only the display name and a
/// hash of the email are kept; the organisation and role come from the member rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// The identity, lowercase hex.
    pub identity: String,
    /// The JWT subject (the WorkOS user id), else the identity hex, as for an owner
    /// (`crate::rules::new_org`).
    pub user_id: String,
    /// `full_name`; `None` when blank.
    pub display_name: Option<String>,
    /// SHA-256 of the trimmed, lowercased email, as lowercase hex; `None` without an email.
    pub email_hash: Option<String>,
    /// When the profile was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// What an access group lets its members see and do (`AccessGroupPermissions`, `supabase.ts:179-189`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the nine TS permission flags, field for field"
)]
pub struct AccessGroupPermissions {
    /// See flow nodes.
    pub can_view_nodes: bool,
    /// See amounts.
    pub can_view_amounts: bool,
    /// See proofs (thumbnails).
    pub can_view_proofs: bool,
    /// See who handled each payment.
    pub can_view_executor: bool,
    /// See original proof files.
    pub can_view_full_proofs: bool,
    /// Edit flow nodes.
    pub can_edit_nodes: bool,
    /// Edit transactions.
    pub can_edit_transactions: bool,
    /// Invite and remove people.
    pub can_manage_members: bool,
    /// Create groups and change access.
    pub can_manage_groups: bool,
}

impl AccessGroupPermissions {
    /// Every flag that either side grants (the union `getUserPermissions` takes, `api.ts:772-791`).
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            can_view_nodes: self.can_view_nodes || other.can_view_nodes,
            can_view_amounts: self.can_view_amounts || other.can_view_amounts,
            can_view_proofs: self.can_view_proofs || other.can_view_proofs,
            can_view_executor: self.can_view_executor || other.can_view_executor,
            can_view_full_proofs: self.can_view_full_proofs || other.can_view_full_proofs,
            can_edit_nodes: self.can_edit_nodes || other.can_edit_nodes,
            can_edit_transactions: self.can_edit_transactions || other.can_edit_transactions,
            can_manage_members: self.can_manage_members || other.can_manage_members,
            can_manage_groups: self.can_manage_groups || other.can_manage_groups,
        }
    }
}

/// `AccessGroup.node_scope` (`supabase.ts:200`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeScope {
    /// Every node.
    All,
    /// Nodes given to the group.
    Assigned,
    /// Nodes downstream of the group's.
    Downstream,
}

/// An access group (`AccessGroup`, `supabase.ts:191-206`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessGroup {
    /// Group id.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Name.
    pub name: String,
    /// Short id.
    pub slug: String,
    /// What the group is for.
    pub description: Option<String>,
    /// Icon name.
    pub icon: String,
    /// Colour, `#rrggbb`.
    pub color: String,
    /// What members see and do.
    pub permissions: AccessGroupPermissions,
    /// Which flow nodes they see.
    pub node_scope: NodeScope,
    /// One of the five groups every organisation starts with.
    pub is_default: bool,
    /// Shown on public pages.
    pub is_public: bool,
    /// Sort order, lowest first.
    pub priority: i64,
    /// When it was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// A person in an access group, by user id or by an emailed invite (`GroupMember`, `supabase.ts:208-218`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMember {
    /// Membership id.
    pub id: String,
    /// The group.
    pub group_id: String,
    /// The member's user id; `None` for an invite not yet taken up.
    pub user_id: Option<String>,
    /// The invited address.
    pub user_email: Option<String>,
    /// Who added them: the adding member's user id.
    pub granted_by: Option<String>,
    /// From when.
    pub valid_from: String,
    /// Until when; `None` for no end.
    pub valid_until: Option<String>,
    /// Notes.
    pub notes: Option<String>,
    /// When it was made.
    pub created_at: String,
}

/// `VerificationType` (`supabase.ts:95-107`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationType {
    /// `receipt`.
    Receipt,
    /// `photo`.
    Photo,
    /// `gps`.
    Gps,
    /// `signature`.
    Signature,
    /// `biometric`.
    Biometric,
    /// `bank_statement`.
    BankStatement,
    /// `document`.
    Document,
    /// `invoice`.
    Invoice,
    /// `video`.
    Video,
    /// `audit`.
    Audit,
    /// `contract`.
    Contract,
    /// `government_doc`.
    GovernmentDoc,
}

impl VerificationType {
    /// `VERIFICATION_TRUST_POINTS` (`supabase.ts:110-123`).
    #[must_use]
    pub const fn trust_points(self) -> i64 {
        match self {
            Self::Receipt => 5,
            Self::BankStatement | Self::Video => 10,
            Self::Invoice | Self::Document => 4,
            Self::Photo | Self::GovernmentDoc => 8,
            Self::Gps => 3,
            Self::Biometric | Self::Signature => 7,
            Self::Audit => 15,
            Self::Contract => 6,
        }
    }
}

/// `VerificationVisibility` (`supabase.ts:126`): how much of a proof a viewer gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Nothing.
    Hidden,
    /// That proof is attached, not the file.
    Summary,
    /// A low-resolution version.
    Thumbnail,
    /// The original file.
    Full,
}

/// `Verification.status` (`supabase.ts:135`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// `pending`.
    Pending,
    /// `verified`.
    Verified,
    /// `rejected`.
    Rejected,
}

/// A proof attached to a transaction, an expense claim or a bill (`Verification`, `supabase.ts:128-151`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    /// Proof id.
    pub id: String,
    /// The transaction it proves (MOD2's transactions); one of the three links is set.
    pub transaction_id: Option<String>,
    /// The expense claim it proves (`ledger_core` `ExpenseClaim.id`); module addition.
    #[serde(default)]
    pub claim_id: Option<String>,
    /// The bill it proves (`ledger_core` `Bill.id`); module addition.
    #[serde(default)]
    pub bill_id: Option<String>,
    /// What kind of proof.
    #[serde(rename = "type")]
    pub kind: VerificationType,
    /// R2 object key of the file; the TS `file_url` (P13).
    #[serde(default)]
    pub object_key: Option<String>,
    /// SHA-256 of the file, lowercase hex (P13); module addition.
    #[serde(default)]
    pub sha256: Option<String>,
    /// What was read from the file.
    pub ocr_data: Option<Metadata>,
    /// Anything else.
    pub metadata: Metadata,
    /// `pending`, `verified` or `rejected`.
    pub status: VerificationStatus,
    /// Who verified it.
    pub verified_by: Option<String>,
    /// When.
    pub verified_at: Option<String>,
    /// When it was made.
    pub created_at: String,
    /// The type's trust points.
    pub trust_points: i64,
    /// The widest audience the uploader meant it for (shown as a label in the web).
    pub visibility_level: Visibility,
    /// R2 object key of the low-resolution version; the TS `thumbnail_url` (P13).
    #[serde(default)]
    pub thumbnail_key: Option<String>,
    /// Who carried out the payment.
    pub executor_name: Option<String>,
    /// Latitude.
    pub gps_latitude: Option<f64>,
    /// Longitude.
    pub gps_longitude: Option<f64>,
    /// Accuracy in metres.
    pub gps_accuracy: Option<f64>,
    /// Place name.
    pub location_name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Who attached it.
    pub created_by: Option<String>,
    /// Shown to anyone at the proofs Worker's `GET /public/<sha256>` (P13, STO1); set by
    /// [`crate::transparency::plan::mark_published`]. Module addition: absent in TS and in rows
    /// written before STO1, and left out of the JSON while false so those rows stay byte-equal.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub published: bool,
    /// How much of this row the reader was given: set only on rows of the `my_verification` view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_as: Option<Visibility>,
}

/// `VerificationAccessLog.access_type` (`supabase.ts:157`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessType {
    /// Opened.
    View,
    /// Downloaded.
    Download,
}

/// One look at a proof (`VerificationAccessLog`, `supabase.ts:153-162`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationAccess {
    /// Entry id.
    pub id: String,
    /// The proof.
    pub verification_id: String,
    /// The viewer's user id.
    pub accessed_by: String,
    /// `view` or `download`.
    pub access_type: AccessType,
    /// What the viewer was given (never `hidden`).
    pub access_level: Visibility,
    /// Not known to the module; kept for the TS shape.
    pub ip_address: Option<String>,
    /// Not known to the module; kept for the TS shape.
    pub user_agent: Option<String>,
    /// When.
    pub accessed_at: String,
}

/// `CampaignStatus` (`supabase.ts:1377`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    /// Not yet open.
    Draft,
    /// Open to donors.
    Active,
    /// Paused.
    Paused,
    /// Reached its end.
    Completed,
    /// Called off.
    Cancelled,
}

/// A fundraising campaign (`Campaign`, `supabase.ts:1390-1415`); amounts in minor units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Campaign {
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
    /// Confirmed donations, minor units; follows the donations, never set directly.
    pub current_amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// The flow node the money goes to (MOD2).
    pub target_node_id: Option<String>,
    /// Status.
    pub status: CampaignStatus,
    /// May take more than its target.
    pub allow_overflow: bool,
    /// Where overflow goes.
    pub overflow_campaign_id: Option<String>,
    /// Asked of donors before overflow.
    pub overflow_consent_message: Option<String>,
    /// Opens.
    pub starts_at: String,
    /// Closes; `None` for open-ended.
    pub ends_at: Option<String>,
    /// Has a public page.
    pub is_public: bool,
    /// The public page's link, unique across organisations.
    pub public_slug: Option<String>,
    /// Category.
    pub category: Option<String>,
    /// Tags.
    pub tags: Option<Vec<String>>,
    /// Anything else.
    pub metadata: Metadata,
    /// Who made it.
    pub created_by: Option<String>,
    /// When it was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// A campaign's figures (`CampaignStats`, `supabase.ts:1417-1427`) without the two that depend on
/// the reading day, which [`super::stats::days_remaining`] and [`super::stats::days_active`] give.
/// Kept current by every write to the campaign or its donations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignStat {
    /// The campaign.
    pub campaign_id: String,
    /// Confirmed donations.
    pub donation_count: u64,
    /// Distinct donors among them.
    pub unique_donors: u64,
    /// Their sum, minor units.
    pub total_raised: Minor,
    /// Raised over target, percent with one decimal.
    pub funding_percentage: f64,
    /// Mean donation, rounded to the minor unit.
    pub avg_donation: Minor,
    /// Largest, minor units.
    pub largest_donation: Minor,
    /// Smallest, minor units.
    pub smallest_donation: Minor,
}

/// `PaymentStatus` (`supabase.ts:1378`), a plain recorded field: no payment processor (P19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    /// Promised, not yet received.
    Pending,
    /// Received.
    Confirmed,
    /// Did not go through.
    Failed,
    /// Paid back.
    Refunded,
}

/// A donation record (`Donation`, `supabase.ts:1442-1469`); amounts in minor units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Donation {
    /// Donation id.
    pub id: String,
    /// The campaign.
    pub campaign_id: String,
    /// Donor email, as given.
    pub donor_email: Option<String>,
    /// Donor name.
    pub donor_name: Option<String>,
    /// Donor phone.
    pub donor_phone: Option<String>,
    /// Shown without the donor's name.
    pub is_anonymous: bool,
    /// The donor's identity (hex) when a signed-in non-member recorded their own, named gift;
    /// module addition.
    #[serde(default)]
    pub donor_profile: Option<String>,
    /// Amount, minor units.
    pub amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// In US dollars, minor units.
    pub amount_usd: Option<Minor>,
    /// The rate used.
    pub exchange_rate: Option<f64>,
    /// The node the donor earmarked (MOD2).
    pub earmarked_node_id: Option<String>,
    /// Earmark notes.
    pub earmark_notes: Option<String>,
    /// May be moved elsewhere.
    pub allow_reallocation: bool,
    /// The donor's flow node (MOD2).
    pub flow_node_id: Option<String>,
    /// Share of impact (MOD2).
    pub impact_percentage: Option<f64>,
    /// How it was paid.
    pub payment_method: Option<String>,
    /// The payment's reference.
    pub payment_reference: Option<String>,
    /// As recorded.
    pub payment_status: PaymentStatus,
    /// The transaction it became (MOD2).
    pub transaction_id: Option<String>,
    /// Where it came from.
    pub source: Option<String>,
    /// UTM source.
    pub utm_source: Option<String>,
    /// UTM medium.
    pub utm_medium: Option<String>,
    /// UTM campaign.
    pub utm_campaign: Option<String>,
    /// Anything else.
    pub metadata: Metadata,
    /// When it was recorded.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// `DonorUpdateType` (`supabase.ts:1379-1387`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DonorUpdateType {
    /// `donation_received`.
    DonationReceived,
    /// `funds_allocated`.
    FundsAllocated,
    /// `milestone_reached`.
    MilestoneReached,
    /// `project_update`.
    ProjectUpdate,
    /// `impact_report`.
    ImpactReport,
    /// `campaign_complete`.
    CampaignComplete,
    /// `thank_you`.
    ThankYou,
    /// `custom`.
    Custom,
}

/// `DeliveryMethod` (`supabase.ts:1388`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMethod {
    /// `email`.
    Email,
    /// `sms`.
    Sms,
    /// `push`.
    Push,
    /// `in_app`.
    InApp,
}

/// A message to a donor (`DonorUpdate`, `supabase.ts:1476-1490`). Nothing is delivered; sending
/// only stamps `sent_at` (`api.ts:1343-1349`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DonorUpdate {
    /// Update id.
    pub id: String,
    /// The donation it is about.
    pub donation_id: String,
    /// The campaign.
    pub campaign_id: Option<String>,
    /// What kind.
    pub update_type: DonorUpdateType,
    /// Title.
    pub title: String,
    /// Message.
    pub message: String,
    /// Image.
    pub image_url: Option<String>,
    /// Related flow node (MOD2).
    pub related_node_id: Option<String>,
    /// Related transaction (MOD2).
    pub related_transaction_id: Option<String>,
    /// When it was marked sent.
    pub sent_at: Option<String>,
    /// How it would go out.
    pub delivery_method: DeliveryMethod,
    /// Anything else.
    pub metadata: Metadata,
    /// When it was written.
    pub created_at: String,
}

// ---------------------------------------------------------------------------------------------
// Inputs: the JSON arguments of the reducers. Unknown fields are refused; who acted, ids and
// times are the module's.
// ---------------------------------------------------------------------------------------------

/// `updateOrganization`'s `OrganizationProfileUpdate` (`api.ts:158-160`) plus the module's
/// profile fields. A field left out is kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationPatch {
    /// New name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New type, or `null`.
    #[serde(
        rename = "type",
        default,
        skip_serializing_if = "Nullish::is_undefined"
    )]
    pub org_type: Nullish<OrgType>,
    /// New country.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// New currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// New public link, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub public_slug: Nullish<String>,
    /// New description, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub description: Nullish<String>,
    /// New logo object key, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub logo_key: Nullish<String>,
    /// Show on public pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
}

/// `createAccessGroup`'s input (`api.ts:585-596`) less the organisation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccessGroup {
    /// Name.
    pub name: String,
    /// Short id.
    pub slug: String,
    /// What it is for.
    #[serde(default)]
    pub description: Option<String>,
    /// Icon name.
    pub icon: String,
    /// Colour.
    pub color: String,
    /// What members see and do.
    pub permissions: AccessGroupPermissions,
    /// Which nodes they see.
    pub node_scope: NodeScope,
    /// A default group.
    #[serde(default)]
    pub is_default: bool,
    /// Shown publicly.
    #[serde(default)]
    pub is_public: bool,
    /// Sort order.
    pub priority: i64,
}

/// `updateAccessGroup`'s patch (`api.ts:598-609`). A field left out is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessGroupPatch {
    /// New name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New short id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// New description, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub description: Nullish<String>,
    /// New icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// New colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// New permissions, whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<AccessGroupPermissions>,
    /// New node scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_scope: Option<NodeScope>,
    /// Default flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    /// Public flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    /// New sort order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
}

/// `addGroupMember`'s input (`api.ts:654-658`) less `granted_by`, which is the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewGroupMember {
    /// The group.
    pub group_id: String,
    /// The member's user id.
    #[serde(default)]
    pub user_id: Option<String>,
    /// Their email.
    #[serde(default)]
    pub user_email: Option<String>,
    /// From when; now when left out.
    #[serde(default)]
    pub valid_from: Option<String>,
    /// Until when.
    #[serde(default)]
    pub valid_until: Option<String>,
    /// Notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// `createVerification`'s input (`api.ts:284-317`) less what the module sets: id, times, trust
/// points, and `created_by` and `verified_by`, which are the caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewVerification {
    /// The transaction it proves.
    #[serde(default)]
    pub transaction_id: Option<String>,
    /// The expense claim it proves.
    #[serde(default)]
    pub claim_id: Option<String>,
    /// The bill it proves.
    #[serde(default)]
    pub bill_id: Option<String>,
    /// What kind of proof.
    #[serde(rename = "type")]
    pub kind: VerificationType,
    /// R2 object key.
    #[serde(default)]
    pub object_key: Option<String>,
    /// SHA-256 of the file.
    #[serde(default)]
    pub sha256: Option<String>,
    /// R2 object key of the thumbnail.
    #[serde(default)]
    pub thumbnail_key: Option<String>,
    /// What was read from it.
    #[serde(default)]
    pub ocr_data: Option<Metadata>,
    /// Anything else.
    #[serde(default)]
    pub metadata: Metadata,
    /// `pending` when left out.
    #[serde(default = "pending_proof")]
    pub status: VerificationStatus,
    /// `full` when left out.
    #[serde(default = "full_visibility")]
    pub visibility_level: Visibility,
    /// Who carried out the payment.
    #[serde(default)]
    pub executor_name: Option<String>,
    /// Latitude.
    #[serde(default)]
    pub gps_latitude: Option<f64>,
    /// Longitude.
    #[serde(default)]
    pub gps_longitude: Option<f64>,
    /// Accuracy in metres.
    #[serde(default)]
    pub gps_accuracy: Option<f64>,
    /// Place name.
    #[serde(default)]
    pub location_name: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
}

const fn pending_proof() -> VerificationStatus {
    VerificationStatus::Pending
}

const fn full_visibility() -> Visibility {
    Visibility::Full
}

/// `createCampaign`'s input (`api.ts:1080-1098`) less the organisation and `created_by` (the
/// caller); amounts in minor units. Defaults follow the table defaults of
/// `flowtrace/supabase/migrations/011_campaigns.sql:12-50`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCampaign {
    /// Name.
    pub name: String,
    /// Long description.
    #[serde(default)]
    pub description: Option<String>,
    /// One line.
    #[serde(default)]
    pub short_description: Option<String>,
    /// Image.
    #[serde(default)]
    pub image_url: Option<String>,
    /// Target, minor units.
    pub target_amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// Flow node.
    #[serde(default)]
    pub target_node_id: Option<String>,
    /// `draft` when left out.
    #[serde(default = "draft")]
    pub status: CampaignStatus,
    /// May take more than its target.
    #[serde(default)]
    pub allow_overflow: bool,
    /// Where overflow goes.
    #[serde(default)]
    pub overflow_campaign_id: Option<String>,
    /// Asked before overflow.
    #[serde(default)]
    pub overflow_consent_message: Option<String>,
    /// Opens.
    pub starts_at: String,
    /// Closes.
    #[serde(default)]
    pub ends_at: Option<String>,
    /// Has a public page.
    #[serde(default)]
    pub is_public: bool,
    /// The link; made from the name for a public campaign when left out.
    #[serde(default)]
    pub public_slug: Option<String>,
    /// Category.
    #[serde(default)]
    pub category: Option<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Anything else.
    #[serde(default)]
    pub metadata: Metadata,
}

const fn draft() -> CampaignStatus {
    CampaignStatus::Draft
}

/// `updateCampaign`'s patch (`api.ts:1100-1114`); amounts in minor units. A field left out is
/// kept. `current_amount` is not in it: it follows the confirmed donations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPatch {
    /// New name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New description, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub description: Nullish<String>,
    /// New one-liner, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub short_description: Nullish<String>,
    /// New image, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub image_url: Nullish<String>,
    /// New target, minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_amount: Option<Minor>,
    /// New currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// New flow node, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub target_node_id: Nullish<String>,
    /// New status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CampaignStatus>,
    /// Overflow flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_overflow: Option<bool>,
    /// Overflow campaign, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub overflow_campaign_id: Nullish<String>,
    /// Overflow message, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub overflow_consent_message: Nullish<String>,
    /// New start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    /// New end, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub ends_at: Nullish<String>,
    /// Public flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    /// New link, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub public_slug: Nullish<String>,
    /// New category, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub category: Nullish<String>,
    /// New tags, or `null`.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub tags: Nullish<Vec<String>>,
    /// New metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// `createDonation`'s input (`api.ts:1256-1272`); amounts in minor units. Defaults follow
/// `flowtrace/supabase/migrations/011_campaigns.sql:55-100`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewDonation {
    /// The campaign.
    pub campaign_id: String,
    /// Donor email.
    #[serde(default)]
    pub donor_email: Option<String>,
    /// Donor name.
    #[serde(default)]
    pub donor_name: Option<String>,
    /// Donor phone.
    #[serde(default)]
    pub donor_phone: Option<String>,
    /// Shown without a name.
    #[serde(default)]
    pub is_anonymous: bool,
    /// Amount, minor units.
    pub amount: Minor,
    /// ISO 4217 code.
    pub currency: String,
    /// In US dollars, minor units.
    #[serde(default)]
    pub amount_usd: Option<Minor>,
    /// The rate used.
    #[serde(default)]
    pub exchange_rate: Option<f64>,
    /// Earmarked node.
    #[serde(default)]
    pub earmarked_node_id: Option<String>,
    /// Earmark notes.
    #[serde(default)]
    pub earmark_notes: Option<String>,
    /// May be moved elsewhere; `true` when left out.
    #[serde(default = "yes")]
    pub allow_reallocation: bool,
    /// The donor's flow node.
    #[serde(default)]
    pub flow_node_id: Option<String>,
    /// How it was paid.
    #[serde(default)]
    pub payment_method: Option<String>,
    /// Payment reference.
    #[serde(default)]
    pub payment_reference: Option<String>,
    /// `pending` when left out.
    #[serde(default = "pending_payment")]
    pub payment_status: PaymentStatus,
    /// Transaction (MOD2).
    #[serde(default)]
    pub transaction_id: Option<String>,
    /// Where it came from.
    #[serde(default)]
    pub source: Option<String>,
    /// UTM source.
    #[serde(default)]
    pub utm_source: Option<String>,
    /// UTM medium.
    #[serde(default)]
    pub utm_medium: Option<String>,
    /// UTM campaign.
    #[serde(default)]
    pub utm_campaign: Option<String>,
    /// Anything else.
    #[serde(default)]
    pub metadata: Metadata,
}

const fn yes() -> bool {
    true
}

const fn pending_payment() -> PaymentStatus {
    PaymentStatus::Pending
}

/// `createDonorUpdate`'s input (`api.ts:1329-1341`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewDonorUpdate {
    /// The donation.
    pub donation_id: String,
    /// The campaign.
    #[serde(default)]
    pub campaign_id: Option<String>,
    /// What kind.
    pub update_type: DonorUpdateType,
    /// Title.
    pub title: String,
    /// Message.
    pub message: String,
    /// Image.
    #[serde(default)]
    pub image_url: Option<String>,
    /// Related node.
    #[serde(default)]
    pub related_node_id: Option<String>,
    /// Related transaction.
    #[serde(default)]
    pub related_transaction_id: Option<String>,
    /// How it would go out; `email` when left out.
    #[serde(default = "email")]
    pub delivery_method: DeliveryMethod,
    /// Anything else.
    #[serde(default)]
    pub metadata: Metadata,
}

const fn email() -> DeliveryMethod {
    DeliveryMethod::Email
}
