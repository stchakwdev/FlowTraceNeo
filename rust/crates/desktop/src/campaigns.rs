//! The Campaigns screen's view models (DT1), plain Rust with no GPUI: the web's campaign list
//! (`flowtrace/src/routes/_authenticated/campaigns/index.tsx`, `components/campaign-card.tsx`),
//! the new-campaign form (`components/campaign-create-modal.tsx`), one campaign's manage page
//! (`campaigns/$id.tsx`, `campaign-progress-bar.tsx`, `campaign-status-badge.tsx`), the
//! record-donation form (`components/public-donation-form.tsx` as `donation-modal.tsx` uses it),
//! the preview card (`components/campaign-embed-widget.tsx`) and the link and embed code
//! (`components/social-share-buttons.tsx`, `components/embed-code-generator.tsx`).
//!
//! Amounts are minor units; forms take major units and send minor (`fromMajor`). Text follows
//! `lib/format.ts` ([`crate::web_format`]).

use ledger_core::Minor;
use ledger_core::money::from_major;
use ledger_core::types::IsoDate;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::{
    CampaignReport, Earmark, NodeType, ReportOrganization, TransactionStatus, campaign_report,
    is_open_to_public, node_ref, stat_of,
};
use ledger_module::transparency::stats::iso_ms;
use ledger_module::transparency::types::{
    Campaign, CampaignPatch, CampaignStatus, Donation, Metadata, NewCampaign, NewDonation,
    PaymentStatus,
};

use crate::backend::CampaignWrite;
use crate::claims::parse_day_month_year;
use crate::dashboard::{Bar, Figure};
use crate::web_format::{format_compact, format_date, format_money, format_short_date};

/// Where links and embeds point when no web address is set: `ORIGIN_FALLBACK`
/// (`embed-code-generator.tsx:19`), also `config.toml`'s `web_url` default.
pub const ORIGIN_FALLBACK: &str = "https://flowtrace.app";

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

// ---------------------------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------------------------

/// `CAMPAIGN_STATUS[status].label` (`campaign-status-badge.tsx:9-37`).
#[must_use]
pub fn status_label(status: CampaignStatus) -> &'static str {
    match status {
        CampaignStatus::Active => "Active",
        CampaignStatus::Completed => "Completed",
        CampaignStatus::Draft => "Draft",
        CampaignStatus::Paused => "Paused",
        CampaignStatus::Cancelled => "Cancelled",
    }
}

/// `PAYMENT_STATUS[status].label` (`campaign-status-badge.tsx:69-74`).
#[must_use]
pub fn payment_label(status: PaymentStatus) -> &'static str {
    match status {
        PaymentStatus::Confirmed => "Confirmed",
        PaymentStatus::Pending => "Awaiting payment",
        PaymentStatus::Failed => "Failed",
        PaymentStatus::Refunded => "Refunded",
    }
}

/// The proof line a payment status is drawn with (`PAYMENT_STATUS[status].status`).
#[must_use]
pub fn payment_line(status: PaymentStatus) -> TransactionStatus {
    match status {
        PaymentStatus::Confirmed => TransactionStatus::Verified,
        PaymentStatus::Pending => TransactionStatus::Pending,
        PaymentStatus::Failed => TransactionStatus::Flagged,
        PaymentStatus::Refunded => TransactionStatus::Rejected,
    }
}

/// `METHOD_LABEL[method] ?? method` (`campaigns/$id.tsx:57-62`).
#[must_use]
pub fn method_label(method: Option<&str>) -> String {
    match method {
        Some("mpesa") => "M-Pesa".to_owned(),
        Some("card") => "Card".to_owned(),
        Some("bank") => "Bank transfer".to_owned(),
        Some("paypal") => "PayPal".to_owned(),
        Some(other) => other.to_owned(),
        None => String::new(),
    }
}

/// `Math.round((raised / target) * 100)`, or 0 without a target (`pctOf`,
/// `campaign-card.tsx:20-21`).
#[must_use]
pub fn percent(raised: Minor, target: Minor) -> i64 {
    if target <= 0 {
        return 0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a JS division, as the web makes it"
    )]
    let ratio = raised as f64 / target as f64 * 100.0;
    #[expect(clippy::cast_possible_truncation, reason = "a percentage fits an i64")]
    let rounded = (ratio + 0.5).floor() as i64;
    rounded
}

// ---------------------------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------------------------

/// Where a campaign stands in time (`campaignTiming`, `campaign-status-badge.tsx:88-125`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    /// The end has passed, or an open-ended campaign is closed.
    pub ended: bool,
    /// `63 days left, ends 31 Dec 2026`, `Ended 30 Jun 2026`, `No end date`.
    pub label: String,
    /// Whole days left, rounded up; `None` once ended or without an end.
    pub days_left: Option<i64>,
}

/// `campaignTiming(campaign, now)`: "Ended 30 Jun 2026" once the end has passed, never
/// "0 days left". An end the web cannot read (`new Date(..)` is `NaN`) reads as no end date.
#[must_use]
pub fn campaign_timing(c: &Campaign, now_ms: i64) -> Timing {
    let end = c
        .ends_at
        .as_deref()
        .and_then(|e| iso_ms(e).map(|ms| (e, ms)));
    let Some((ends_at, end)) = end else {
        return Timing {
            ended: matches!(
                c.status,
                CampaignStatus::Completed | CampaignStatus::Cancelled
            ),
            label: "No end date".to_owned(),
            days_left: None,
        };
    };
    if end <= now_ms {
        return Timing {
            ended: true,
            label: format!("Ended {}", format_date(ends_at)),
            days_left: None,
        };
    }
    let days = -(-(end - now_ms)).div_euclid(DAY_MS);
    let label = if days <= 1 {
        format!("Last day, ends {}", format_date(ends_at))
    } else {
        format!("{days} days left, ends {}", format_date(ends_at))
    };
    Timing {
        ended: false,
        label,
        days_left: Some(days),
    }
}

// ---------------------------------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------------------------------

/// The campaigns as `getCampaigns` lists them (`api.ts:1041-1048`): newest `created_at` first,
/// ties in record order.
#[must_use]
pub fn campaigns_newest_first(t: &OrgTransparency) -> Vec<&Campaign> {
    let mut list: Vec<&Campaign> = t.campaigns.iter().collect();
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    list
}

/// One row of `CampaignTable` (`campaign-card.tsx:27-140`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRow {
    /// Campaign id.
    pub id: String,
    /// Name.
    pub name: String,
    /// `Funds Turkana Boreholes · public page`.
    pub subtitle: String,
    /// Status.
    pub status: CampaignStatus,
    /// `KES 3,200,000`.
    pub raised: String,
    /// `of KES 5,000,000 · 64%`.
    pub of_target: String,
    /// Raised against the larger of target and raised.
    pub bar: Bar,
    /// `campaignTiming(c).label`.
    pub timing: String,
    /// The row menu offers "Delete draft".
    pub is_draft: bool,
}

impl CampaignRow {
    /// The row of `c`.
    #[must_use]
    pub fn of(t: &OrgTransparency, c: &Campaign, now_ms: i64) -> Self {
        let target = node_ref(t, c.target_node_id.as_deref());
        let pct = percent(c.current_amount, c.target_amount);
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            subtitle: format!(
                "{}{}",
                target.map_or_else(
                    || "Not tied to one project".to_owned(),
                    |n| format!("Funds {}", n.name)
                ),
                if c.is_public {
                    " \u{b7} public page"
                } else {
                    " \u{b7} private"
                }
            ),
            status: c.status,
            raised: format_money(c.current_amount, &c.currency),
            of_target: format!(
                "of {} \u{b7} {pct}%",
                format_money(c.target_amount, &c.currency)
            ),
            bar: Bar {
                total: c.target_amount.max(c.current_amount),
                filled: c.current_amount,
                evidenced: None,
                pending: false,
            },
            timing: campaign_timing(c, now_ms).label,
            is_draft: c.status == CampaignStatus::Draft,
        }
    }
}

/// `getCampaignCounts` (`api.ts:1167-1181`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CampaignCounts {
    /// Campaigns.
    pub total_campaigns: usize,
    /// Active ones.
    pub active_campaigns: usize,
    /// Completed ones.
    pub completed_campaigns: usize,
    /// Their raised amounts, minor units.
    pub total_raised: Minor,
    /// Confirmed donations to them.
    pub total_donations: usize,
}

/// The organisation's campaign counts.
#[must_use]
pub fn campaign_counts(t: &OrgTransparency) -> CampaignCounts {
    let count = |s: CampaignStatus| t.campaigns.iter().filter(|c| c.status == s).count();
    CampaignCounts {
        total_campaigns: t.campaigns.len(),
        active_campaigns: count(CampaignStatus::Active),
        completed_campaigns: count(CampaignStatus::Completed),
        total_raised: t.campaigns.iter().map(|c| c.current_amount).sum(),
        total_donations: t
            .donations
            .iter()
            .filter(|d| {
                d.payment_status == PaymentStatus::Confirmed
                    && t.campaigns.iter().any(|c| c.id == d.campaign_id)
            })
            .count(),
    }
}

/// The list page's key figures (`campaigns/index.tsx:101-118`).
#[must_use]
pub fn counts_figures(c: &CampaignCounts) -> Vec<Figure> {
    vec![
        Figure {
            value: format!("KES {}", format_compact(c.total_raised)),
            label: "Raised across campaigns".to_owned(),
            detail: None,
        },
        Figure {
            value: c.active_campaigns.to_string(),
            label: "Open to donors".to_owned(),
            detail: Some(format!("of {} campaigns", c.total_campaigns)),
        },
        Figure {
            value: c.total_donations.to_string(),
            label: "Confirmed donations".to_owned(),
            detail: None,
        },
        Figure {
            value: c.completed_campaigns.to_string(),
            label: "Completed".to_owned(),
            detail: None,
        },
    ]
}

/// The status filter (`STATUS_FILTERS`, `campaigns/index.tsx:45-52`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    /// Every status.
    #[default]
    All,
    /// One status.
    Only(CampaignStatus),
}

impl StatusFilter {
    /// The filters in menu order.
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Only(CampaignStatus::Active),
        Self::Only(CampaignStatus::Draft),
        Self::Only(CampaignStatus::Paused),
        Self::Only(CampaignStatus::Completed),
        Self::Only(CampaignStatus::Cancelled),
    ];

    /// The menu label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All statuses",
            Self::Only(CampaignStatus::Active) => "Active",
            Self::Only(CampaignStatus::Draft) => "Drafts",
            Self::Only(CampaignStatus::Paused) => "Paused",
            Self::Only(CampaignStatus::Completed) => "Completed",
            Self::Only(CampaignStatus::Cancelled) => "Cancelled",
        }
    }
}

/// The list's search and filter (`campaigns/index.tsx:66-73`): the query, trimmed and lower
/// case, in the name, description or category, and the status.
#[must_use]
pub fn matches(c: &Campaign, query: &str, filter: StatusFilter) -> bool {
    let q = query.trim().to_lowercase();
    let has = |s: Option<&str>| s.is_some_and(|s| s.to_lowercase().contains(&q));
    let searched = q.is_empty()
        || has(Some(&c.name))
        || has(c.description.as_deref())
        || has(c.category.as_deref());
    searched && (filter == StatusFilter::All || filter == StatusFilter::Only(c.status))
}

/// What the list page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignList {
    /// The four key figures.
    pub figures: Vec<Figure>,
    /// The rows that match, newest first.
    pub rows: Vec<CampaignRow>,
    /// `2 of 4`.
    pub meta: String,
    /// The organisation has campaigns but none match.
    pub no_match: Option<String>,
    /// The organisation has no campaigns.
    pub empty: bool,
}

impl CampaignList {
    /// The list of `t` for `query` and `filter` at `now_ms`.
    #[must_use]
    pub fn of(t: &OrgTransparency, query: &str, filter: StatusFilter, now_ms: i64) -> Self {
        let all = campaigns_newest_first(t);
        let rows: Vec<CampaignRow> = all
            .iter()
            .filter(|c| matches(c, query, filter))
            .map(|c| CampaignRow::of(t, c, now_ms))
            .collect();
        let no_match = (rows.is_empty() && !all.is_empty()).then(|| {
            let what = if query.is_empty() {
                filter.label()
            } else {
                query
            };
            format!("Nothing matches \u{201c}{what}\u{201d}.")
        });
        Self {
            figures: counts_figures(&campaign_counts(t)),
            meta: format!("{} of {}", rows.len(), all.len()),
            rows,
            no_match,
            empty: all.is_empty(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The new-campaign form
// ---------------------------------------------------------------------------------------------

/// `CATEGORIES` (`campaign-create-modal.tsx:36-45`): value and label.
pub const CATEGORIES: [(&str, &str); 8] = [
    ("water", "Water and sanitation"),
    ("education", "Education"),
    ("health", "Health"),
    ("infrastructure", "Infrastructure"),
    ("emergency", "Emergency relief"),
    ("environment", "Environment"),
    ("community", "Community development"),
    ("other", "Other"),
];

/// `CURRENCIES` (`campaign-create-modal.tsx:47-52`).
pub const CURRENCIES: [&str; 4] = ["KES", "USD", "EUR", "GBP"];

/// The one-line summary's `maxLength` (`campaign-create-modal.tsx:185`).
pub const SUMMARY_MAX: usize = 150;

/// The projects a campaign can fund: project and department nodes in record order
/// (`campaign-create-modal.tsx:91-93`), as `(id, name)`.
#[must_use]
pub fn project_choices(t: &OrgTransparency) -> Vec<(String, String)> {
    t.flow_nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Project | NodeType::Department))
        .map(|n| (n.id.clone(), n.name.clone()))
        .collect()
}

/// The new-campaign form's fields (`EMPTY_FORM`, `campaign-create-modal.tsx:54-65`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCampaignForm {
    /// Campaign name.
    pub name: String,
    /// The story.
    pub description: String,
    /// One-line summary.
    pub short_description: String,
    /// Target in major units, as typed.
    pub target: String,
    /// Currency code.
    pub currency: String,
    /// The project it funds.
    pub target_node: Option<String>,
    /// Category value.
    pub category: Option<String>,
    /// End date, day/month/year as typed.
    pub ends_at: String,
    /// Public donate page.
    pub is_public: bool,
    /// Keep accepting gifts after the target.
    pub allow_overflow: bool,
}

impl Default for NewCampaignForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            short_description: String::new(),
            target: String::new(),
            currency: "KES".to_owned(),
            target_node: None,
            category: None,
            ends_at: String::new(),
            is_public: true,
            allow_overflow: false,
        }
    }
}

/// The form's field errors (`FormErrors`, `campaign-create-modal.tsx:67`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CampaignErrors {
    /// Under the name.
    pub name: Option<String>,
    /// Under the target.
    pub target: Option<String>,
    /// Under the end date.
    pub ends_at: Option<String>,
}

impl CampaignErrors {
    /// Whether any field has an error.
    #[must_use]
    pub fn any(&self) -> bool {
        self.name.is_some() || self.target.is_some() || self.ends_at.is_some()
    }
}

/// `Number(text)` for the number fields: blank is 0, otherwise the decimal it spells; `None`
/// where JS gives `NaN` or an infinity.
fn js_number(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return Some(0.0);
    }
    let plain = t
        .trim_start_matches(['+', '-'])
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'));
    if !plain {
        return None;
    }
    t.parse::<f64>().ok().filter(|n| n.is_finite())
}

impl NewCampaignForm {
    /// `validate(form)` (`campaign-create-modal.tsx:73-85`) with `today` as `localTodayISO()`.
    #[must_use]
    pub fn validate(&self, today: &IsoDate) -> CampaignErrors {
        let mut e = CampaignErrors::default();
        if self.name.trim().is_empty() {
            e.name = Some("Give the campaign a name donors will recognise.".to_owned());
        }
        if self.target.is_empty() {
            e.target = Some("Enter the amount you need to raise.".to_owned());
        } else if js_number(&self.target).is_none_or(|n| n <= 0.0) {
            e.target = Some("The target must be more than zero.".to_owned());
        }
        match parse_day_month_year(&self.ends_at) {
            Some(Err(message)) => e.ends_at = Some(message),
            Some(Ok(end)) if end.as_str() < today.as_str() => {
                e.ends_at = Some("The end date has passed. Pick today or a later date.".to_owned());
            }
            _ => {}
        }
        e
    }

    /// The campaign `submit(asDraft)` sends (`campaign-create-modal.tsx:113-138`): trimmed text,
    /// blanks as none, the target in minor units, `draft` or `active`, starting `now` (an ISO
    /// timestamp), ending at 23:59:59 UTC on the end date, no link (the module makes one from
    /// the name for a public campaign).
    ///
    /// # Errors
    ///
    /// The field errors of [`NewCampaignForm::validate`], or a target too large for minor units.
    pub fn to_input(
        &self,
        as_draft: bool,
        today: &IsoDate,
        now: &str,
    ) -> Result<NewCampaign, Box<CampaignErrors>> {
        let mut errors = self.validate(today);
        let target = js_number(&self.target)
            .and_then(|n| from_major(n).ok())
            .unwrap_or(0);
        if errors.target.is_none() && target <= 0 {
            errors.target = Some("The target must be more than zero.".to_owned());
        }
        if errors.any() {
            return Err(Box::new(errors));
        }
        let text = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        let summary: String = self.short_description.chars().take(SUMMARY_MAX).collect();
        let ends_at = match parse_day_month_year(&self.ends_at) {
            Some(Ok(end)) => Some(format!("{}T23:59:59Z", end.as_str())),
            _ => None,
        };
        Ok(NewCampaign {
            name: self.name.trim().to_owned(),
            description: text(&self.description),
            short_description: text(&summary),
            image_url: None,
            target_amount: target,
            currency: self.currency.clone(),
            target_node_id: self.target_node.clone(),
            status: if as_draft {
                CampaignStatus::Draft
            } else {
                CampaignStatus::Active
            },
            allow_overflow: self.allow_overflow,
            overflow_campaign_id: None,
            overflow_consent_message: None,
            starts_at: now.to_owned(),
            ends_at,
            is_public: self.is_public,
            public_slug: None,
            category: self.category.clone(),
            tags: None,
            metadata: Metadata::new(),
        })
    }
}

/// The create form's submit error (`campaign-create-modal.tsx:140-144`).
#[must_use]
pub fn not_saved(message: &str) -> String {
    format!("The campaign was not saved: {message}")
}

// ---------------------------------------------------------------------------------------------
// One campaign
// ---------------------------------------------------------------------------------------------

/// What the status menu offers (`campaigns/$id.tsx:203-243`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAction {
    /// A draft opens: `Open to donors`.
    Open,
    /// An active campaign pauses: `Pause campaign`.
    Pause,
    /// A paused campaign resumes: `Resume campaign`.
    Resume,
    /// A draft goes: `Delete draft`. The module silently keeps any other campaign
    /// (`plan::delete_campaign`), so only drafts offer it.
    DeleteDraft,
}

impl StatusAction {
    /// The menu item's text.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open to donors",
            Self::Pause => "Pause campaign",
            Self::Resume => "Resume campaign",
            Self::DeleteDraft => "Delete draft",
        }
    }

    /// The write it makes on campaign `id`.
    #[must_use]
    pub fn write(self, id: &str) -> CampaignWrite {
        let to = |status| CampaignWrite::Update {
            id: id.to_owned(),
            patch: CampaignPatch {
                status: Some(status),
                ..CampaignPatch::default()
            },
        };
        match self {
            Self::Open | Self::Resume => to(CampaignStatus::Active),
            Self::Pause => to(CampaignStatus::Paused),
            Self::DeleteDraft => CampaignWrite::Delete { id: id.to_owned() },
        }
    }

    /// The error when the write fails (`setStatus`, `handleDelete`, `campaigns/$id.tsx:82-103`).
    #[must_use]
    pub fn failed(self, message: &str) -> String {
        match self {
            Self::DeleteDraft => format!("The draft was not deleted: {message}"),
            _ => format!("The campaign status did not change: {message} Try again."),
        }
    }
}

/// The actions of a campaign in `status`.
#[must_use]
pub fn status_actions(status: CampaignStatus) -> Vec<StatusAction> {
    match status {
        CampaignStatus::Draft => vec![StatusAction::Open, StatusAction::DeleteDraft],
        CampaignStatus::Active => vec![StatusAction::Pause],
        CampaignStatus::Paused => vec![StatusAction::Resume],
        CampaignStatus::Completed | CampaignStatus::Cancelled => Vec::new(),
    }
}

/// What a closed campaign's menu says (`campaigns/$id.tsx:240-242`).
pub const NO_ACTIONS: &str = "No actions for a closed campaign";

/// `CampaignCoverage` (`campaign-progress-bar.tsx`): raised against target, the spent-with-proof
/// share and the key under the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// `KES 3,200,000`.
    pub raised: String,
    /// `KES 5,000,000`.
    pub target: String,
    /// `64%`.
    pub percent: String,
    /// The bar.
    pub bar: Bar,
    /// The key: label and amount, in order.
    pub key: Vec<(String, String)>,
}

impl Coverage {
    /// The coverage of `raised` against `target`, with `evidenced` spent with proof when known.
    #[must_use]
    pub fn of(raised: Minor, target: Minor, evidenced: Option<Minor>, currency: &str) -> Self {
        let money = |m| format_money(m, currency);
        let mut key = Vec::new();
        if let Some(e) = evidenced {
            key.push(("Spent with proof".to_owned(), money(e)));
        }
        key.push(match evidenced {
            Some(e) => (
                "Raised, not yet spent with proof".to_owned(),
                money((raised - e).max(0)),
            ),
            None => ("Raised".to_owned(), money(raised)),
        });
        let over = raised - target;
        key.push(if over > 0 {
            ("Over target".to_owned(), money(over))
        } else {
            ("Still to raise".to_owned(), money((target - raised).max(0)))
        });
        Self {
            raised: money(raised),
            target: money(target),
            percent: format!("{}%", percent(raised, target)),
            bar: Bar {
                total: target.max(raised),
                filled: raised,
                evidenced,
                pending: false,
            },
            key,
        }
    }
}

/// One row of the Donations table (`campaigns/$id.tsx:296-321`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonationRow {
    /// Donation id.
    pub id: String,
    /// The donor's name or `Anonymous`.
    pub donor: String,
    /// `For Turkana Boreholes` when earmarked.
    pub earmark: Option<String>,
    /// `18 Jan`.
    pub date: String,
    /// Payment status.
    pub status: PaymentStatus,
    /// `M-Pesa`.
    pub method: String,
    /// `KES 2,500`.
    pub amount: String,
}

impl DonationRow {
    /// The row of `d`.
    #[must_use]
    pub fn of(t: &OrgTransparency, d: &Donation) -> Self {
        let named = d.donor_name.as_deref().filter(|n| !n.is_empty());
        Self {
            id: d.id.clone(),
            donor: match (d.is_anonymous, named) {
                (false, Some(name)) => name.to_owned(),
                _ => "Anonymous".to_owned(),
            },
            earmark: node_ref(t, d.earmarked_node_id.as_deref()).map(|n| format!("For {}", n.name)),
            date: format_short_date(&d.created_at),
            status: d.payment_status,
            method: method_label(d.payment_method.as_deref()),
            amount: format_money(d.amount, &d.currency),
        }
    }
}

/// One line of "Recent programme spending" (`ProofSpendingList`,
/// `public-campaign-header.tsx:104-145`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendingRow {
    /// What it was.
    pub description: String,
    /// `KES 750,000`.
    pub amount: String,
    /// `28 Jan · Field Operations to Communities`.
    pub detail: String,
    /// Proof state.
    pub status: TransactionStatus,
    /// `3 proofs`, when any.
    pub proofs: Option<String>,
}

/// One node of "Where gifts can go" (`EarmarkList`, `public-campaign-header.tsx:50-101`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarmarkRow {
    /// Node name.
    pub name: String,
    /// The campaign's own node.
    pub is_target: bool,
    /// `KES 1,200,000`.
    pub received: String,
    /// Evidenced against received.
    pub bar: Bar,
    /// `KES 900,000 of it with verified proof (75%)` or `Nothing received yet`.
    pub note: String,
}

impl EarmarkRow {
    /// The row of `e`.
    #[must_use]
    pub fn of(e: &Earmark, currency: &str) -> Self {
        Self {
            name: e.name.clone(),
            is_target: e.is_target,
            received: format_money(e.received, currency),
            bar: Bar {
                total: e.received.max(1),
                filled: e.received,
                evidenced: Some(e.evidenced),
                pending: false,
            },
            note: if e.received > 0 {
                format!(
                    "{} of it with verified proof ({}%)",
                    format_money(e.evidenced, currency),
                    percent(e.evidenced, e.received)
                )
            } else {
                "Nothing received yet".to_owned()
            },
        }
    }
}

/// The organisation as a campaign report names it (`getCampaignReport`, `api.ts:1368-1373`):
/// the profile's name (else `fallback`, the picker's), the trust row's score.
#[must_use]
pub fn report_organization(
    t: &OrgTransparency,
    org_id: &str,
    fallback: &str,
) -> ReportOrganization {
    ReportOrganization {
        id: org_id.to_owned(),
        name: t
            .profile
            .as_ref()
            .map_or_else(|| fallback.to_owned(), |p| p.name.clone()),
        trust_score: t.trust.as_ref().map_or(0, |r| r.score),
        verified: t.profile.as_ref().is_some_and(|p| p.verified),
    }
}

/// One campaign's manage page (`campaigns/$id.tsx`).
#[derive(Debug, Clone, PartialEq)]
pub struct CampaignPage {
    /// The campaign.
    pub campaign: Campaign,
    /// `getCampaignReport` of it: evidenced amount, earmarks, spending.
    pub report: CampaignReport,
    /// Timing at the reading time.
    pub timing: Timing,
    /// `Funds Turkana Boreholes`.
    pub funds: Option<String>,
    /// "Record donation" shows: active and not ended (`canReceive`).
    pub can_receive: bool,
    /// The status menu.
    pub actions: Vec<StatusAction>,
    /// Funding coverage.
    pub coverage: Coverage,
    /// Confirmed donations, donors, average gift, days.
    pub figures: Vec<Figure>,
    /// Every donation, newest first.
    pub donations: Vec<DonationRow>,
    /// The six latest programme movements.
    pub spending: Vec<SpendingRow>,
    /// The details list: label and value.
    pub details: Vec<(&'static str, String)>,
    /// Projects with money or the campaign's own.
    pub earmarks: Vec<EarmarkRow>,
    /// The donate page's address, for a public campaign with a link.
    pub donate_url: Option<String>,
}

impl CampaignPage {
    /// The page of campaign `id` of `t`, `None` when there is no such campaign.
    #[must_use]
    pub fn of(
        t: &OrgTransparency,
        id: &str,
        org: ReportOrganization,
        now_ms: i64,
        origin: &str,
    ) -> Option<Self> {
        let report = campaign_report(t, id, org)?;
        let c = report.campaign.campaign.clone();
        let timing = campaign_timing(&c, now_ms);
        let stats = stat_of(t, id);
        let cur = c.currency.as_str();
        let dash = || "\u{2013}".to_owned();
        let days = match (timing.days_left, timing.ended, c.ends_at.as_deref()) {
            (Some(n), _, Some(end)) => (
                n.to_string(),
                format!("Days left, to {}", format_short_date(end)),
            ),
            (None, true, Some(end)) => ("Ended".to_owned(), format!("on {}", format_date(end))),
            (None, true, None) => ("Ended".to_owned(), "No end date".to_owned()),
            _ => ("Open".to_owned(), "No end date".to_owned()),
        };
        let figures = vec![
            figure(
                stats.map_or_else(dash, |s| s.donation_count.to_string()),
                "Confirmed donations",
            ),
            figure(
                stats.map_or_else(dash, |s| s.unique_donors.to_string()),
                "Donors",
            ),
            figure(
                stats.map_or_else(dash, |s| format_money(s.avg_donation, cur)),
                "Average gift",
            ),
            Figure {
                value: days.0,
                label: days.1,
                detail: None,
            },
        ];
        let mut donations: Vec<&Donation> = t
            .donations
            .iter()
            .filter(|d| d.campaign_id == c.id)
            .collect();
        donations.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let category = c.category.as_deref().map_or_else(
            || "None".to_owned(),
            |k| {
                let mut chars = k.chars();
                chars
                    .next()
                    .map_or_else(String::new, |f| f.to_uppercase().chain(chars).collect())
            },
        );
        let funds = report.campaign.target_node.as_ref().map(|n| n.name.clone());
        let details = vec![
            ("Target", format_money(c.target_amount, cur)),
            (
                "Funds",
                funds
                    .clone()
                    .unwrap_or_else(|| "Not tied to one project".to_owned()),
            ),
            ("Category", category),
            ("Started", format_date(&c.starts_at)),
            (
                "Ends",
                c.ends_at
                    .as_deref()
                    .map_or_else(|| "No end date".to_owned(), format_date),
            ),
            (
                "Visibility",
                if c.is_public {
                    "Public donate page"
                } else {
                    "Private"
                }
                .to_owned(),
            ),
            (
                "After target",
                if c.allow_overflow {
                    "Keeps accepting gifts"
                } else {
                    "Closes to new gifts"
                }
                .to_owned(),
            ),
        ];
        let spending = report
            .spending
            .iter()
            .map(|x| {
                let t = &x.transaction;
                let route = match (&x.from_node, &x.to_node) {
                    (Some(f), Some(to)) => format!(" \u{b7} {} to {}", f.name, to.name),
                    (None, Some(to)) => format!(" \u{b7} to {}", to.name),
                    _ => String::new(),
                };
                SpendingRow {
                    description: t.description.clone(),
                    amount: format_money(t.amount, &t.currency),
                    detail: format!("{}{route}", format_short_date(&t.transaction_date)),
                    status: t.verification_status,
                    proofs: (t.verification_count > 0).then(|| {
                        let unit = if t.verification_count == 1 {
                            "proof"
                        } else {
                            "proofs"
                        };
                        format!("{} {unit}", t.verification_count)
                    }),
                }
            })
            .collect();
        Some(Self {
            can_receive: c.status == CampaignStatus::Active && !timing.ended,
            actions: status_actions(c.status),
            coverage: Coverage::of(
                c.current_amount,
                c.target_amount,
                report.evidenced_amount,
                cur,
            ),
            figures,
            donations: donations.iter().map(|d| DonationRow::of(t, d)).collect(),
            spending,
            details,
            earmarks: report
                .earmarks
                .iter()
                .filter(|e| e.received > 0 || e.is_target)
                .map(|e| EarmarkRow::of(e, cur))
                .collect(),
            donate_url: c
                .public_slug
                .as_deref()
                .filter(|_| c.is_public)
                .map(|slug| donate_url(origin, slug)),
            funds,
            timing,
            report,
            campaign: c,
        })
    }

    /// Whether the donate page is live: public, active and with a link
    /// (`getCampaignBySlug`, `api.ts:1059-1066`).
    #[must_use]
    pub fn is_open_to_public(&self) -> bool {
        is_open_to_public(&self.campaign)
    }
}

fn figure(value: String, label: &str) -> Figure {
    Figure {
        value,
        label: label.to_owned(),
        detail: None,
    }
}

// ---------------------------------------------------------------------------------------------
// Recording a donation
// ---------------------------------------------------------------------------------------------

/// `PAYMENT_METHODS` (`public-donation-form.tsx:24-29`): value and label.
pub const PAYMENT_METHODS: [(&str, &str); 4] = [
    ("mpesa", "M-Pesa"),
    ("card", "Debit or credit card"),
    ("bank", "Bank transfer"),
    ("paypal", "PayPal"),
];

/// `SUGGESTED_AMOUNTS` (`public-donation-form.tsx:31`), major units.
pub const SUGGESTED_AMOUNTS: [u32; 5] = [500, 1000, 2500, 5000, 10000];

/// The record-donation form's fields (`EMPTY`, `public-donation-form.tsx:33-42`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonationForm {
    /// Amount in major units, as typed.
    pub amount: String,
    /// Donor name.
    pub donor_name: String,
    /// Donor email.
    pub donor_email: String,
    /// M-Pesa phone.
    pub donor_phone: String,
    /// Shown as Anonymous.
    pub is_anonymous: bool,
    /// `mpesa`, `card`, `bank` or `paypal`.
    pub payment_method: String,
    /// The project the gift is for.
    pub earmark: Option<String>,
    /// May go elsewhere once the target is met.
    pub allow_reallocation: bool,
}

impl Default for DonationForm {
    fn default() -> Self {
        Self {
            amount: String::new(),
            donor_name: String::new(),
            donor_email: String::new(),
            donor_phone: String::new(),
            is_anonymous: false,
            payment_method: "mpesa".to_owned(),
            earmark: None,
            allow_reallocation: true,
        }
    }
}

/// The donation form's field errors (`Errors`, `public-donation-form.tsx:44`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DonationErrors {
    /// Under the amount.
    pub amount: Option<String>,
    /// Under the email.
    pub donor_email: Option<String>,
    /// Under the phone.
    pub donor_phone: Option<String>,
}

impl DonationErrors {
    /// Whether any field has an error.
    #[must_use]
    pub fn any(&self) -> bool {
        self.amount.is_some() || self.donor_email.is_some() || self.donor_phone.is_some()
    }
}

/// `/^\S+@\S+\.\S+$/`: no white space, and an `@` with something before it followed later by a
/// `.` with something on both sides.
#[must_use]
pub fn looks_like_email(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || chars.iter().any(|c| c.is_whitespace()) {
        return false;
    }
    let last = chars.len() - 1;
    chars.iter().enumerate().any(|(at, &c)| {
        c == '@'
            && at > 0
            && chars
                .iter()
                .enumerate()
                .any(|(dot, &d)| d == '.' && dot > at + 1 && dot < last)
    })
}

impl DonationForm {
    /// `validate()` (`public-donation-form.tsx:80-93`) as the in-app form runs it
    /// (`requireEmail={false}`, `donation-modal.tsx:41`).
    #[must_use]
    pub fn validate(&self, currency: &str) -> DonationErrors {
        let mut e = DonationErrors::default();
        if self.amount.is_empty() {
            e.amount = Some("Enter how much you would like to give.".to_owned());
        } else if js_number(&self.amount).is_none_or(|n| n < 1.0) {
            e.amount = Some(format!(
                "The smallest gift is {}.",
                format_money(100, currency)
            ));
        }
        if !self.donor_email.is_empty() && !looks_like_email(&self.donor_email) {
            e.donor_email = Some("This email address looks incomplete.".to_owned());
        }
        if self.payment_method == "mpesa" && self.donor_phone.trim().is_empty() {
            e.donor_phone = Some(
                "M-Pesa needs the phone number that will receive the payment prompt.".to_owned(),
            );
        }
        e
    }

    /// The amount in minor units, when it reads.
    #[must_use]
    pub fn amount_minor(&self) -> Option<Minor> {
        js_number(&self.amount)
            .and_then(|n| from_major(n).ok())
            .filter(|m| *m > 0)
    }

    /// The donation `handleSubmit` records (`public-donation-form.tsx:95-131`): pending, from
    /// the website, the name dropped when anonymous, blanks as none. A record only (P19).
    ///
    /// # Errors
    ///
    /// The field errors of [`DonationForm::validate`].
    pub fn to_input(&self, c: &Campaign) -> Result<NewDonation, Box<DonationErrors>> {
        let mut errors = self.validate(&c.currency);
        let amount = self.amount_minor();
        if errors.amount.is_none() && amount.is_none() {
            errors.amount = Some(format!(
                "The smallest gift is {}.",
                format_money(100, &c.currency)
            ));
        }
        if errors.any() {
            return Err(Box::new(errors));
        }
        let text = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        Ok(NewDonation {
            campaign_id: c.id.clone(),
            donor_email: text(&self.donor_email),
            donor_name: if self.is_anonymous {
                None
            } else {
                text(&self.donor_name)
            },
            donor_phone: text(&self.donor_phone),
            is_anonymous: self.is_anonymous,
            amount: amount.unwrap_or(0),
            currency: c.currency.clone(),
            amount_usd: None,
            exchange_rate: None,
            earmarked_node_id: self.earmark.clone(),
            earmark_notes: None,
            allow_reallocation: self.allow_reallocation,
            flow_node_id: None,
            payment_method: Some(self.payment_method.clone()),
            payment_reference: None,
            payment_status: PaymentStatus::Pending,
            transaction_id: None,
            source: Some("website".to_owned()),
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            metadata: Metadata::new(),
        })
    }

    /// The submit button's text: `Give KES 2,500`, or `Give` before an amount.
    #[must_use]
    pub fn button_label(&self, currency: &str) -> String {
        match self.amount_minor() {
            Some(m) => format!("Give {}", format_money(m, currency)),
            None => "Give".to_owned(),
        }
    }
}

/// The thank-you line after a gift is recorded (`public-donation-form.tsx:136-138`).
#[must_use]
pub fn pledged(amount: Minor, currency: &str) -> String {
    format!("Thank you. {} pledged.", format_money(amount, currency))
}

/// The body under it for `method` (`public-donation-form.tsx:140-147`).
#[must_use]
pub fn pledged_note(method: &str) -> String {
    let first = if method == "mpesa" {
        "Check your phone for the M-Pesa prompt. "
    } else {
        "Payment instructions are on their way to your email. "
    };
    format!(
        "{first}Once the payment is confirmed it joins the campaign total, and you will get an \
         update each time your gift is spent, with the proof attached."
    )
}

/// The donation form's submit error (`public-donation-form.tsx:124-129`).
#[must_use]
pub fn not_recorded(message: &str) -> String {
    format!("Your gift was not recorded: {message}")
}

/// The desktop's way to mark a recorded gift received: `update_donation_status` to
/// `confirmed`, which adds it to the raised amount (`plan::update_donation_status`). The web
/// page has no such control (P19 keeps payment status as data); the desktop adds it so a gift
/// recorded by hand can be confirmed without a processor (`rust/docs/handoff.md`).
#[must_use]
pub fn confirm_payment(donation_id: &str) -> CampaignWrite {
    CampaignWrite::DonationStatus {
        id: donation_id.to_owned(),
        status: PaymentStatus::Confirmed,
        reference: None,
    }
}

// ---------------------------------------------------------------------------------------------
// The preview card, the link and the embed code
// ---------------------------------------------------------------------------------------------

/// The donate page's address (`/donate/{slug}` on the web's origin, as the share buttons copy
/// it on `donate/$slug.tsx:99`).
#[must_use]
pub fn donate_url(origin: &str, slug: &str) -> String {
    format!("{}/donate/{slug}", origin.trim_end_matches('/'))
}

/// The embed generator's options (`CampaignWidgetEmbed`, `embed-code-generator.tsx:127-131`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbedOptions {
    /// Dark theme instead of light.
    pub dark: bool,
    /// Show the raised amount.
    pub show_amount: bool,
    /// Show the Trust Score and timing.
    pub show_donors: bool,
    /// The one-line widget.
    pub compact: bool,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            dark: false,
            show_amount: true,
            show_donors: true,
            compact: false,
        }
    }
}

impl EmbedOptions {
    /// The widget's query string, as `URLSearchParams` writes it (`embed-code-generator.tsx:
    /// 133-140`): the theme always, the other options only when not their default.
    #[must_use]
    pub fn query(self) -> String {
        let mut params = vec![format!(
            "theme={}",
            if self.dark { "dark" } else { "light" }
        )];
        if !self.show_amount {
            params.push("showAmount=false".to_owned());
        }
        if !self.show_donors {
            params.push("showDonors=false".to_owned());
        }
        if self.compact {
            params.push("compact=true".to_owned());
        }
        params.join("&")
    }
}

/// The embed code (`embed-code-generator.tsx:164-186`), byte for byte: an iframe on the
/// widget's path and the script that sizes it to the height the widget reports.
#[must_use]
pub fn embed_snippet(origin: &str, id: &str, name: Option<&str>, opts: EmbedOptions) -> String {
    let origin = origin.trim_end_matches('/');
    let height = if opts.compact { 80 } else { 440 };
    let title = name.unwrap_or("Campaign").replace('"', "&quot;");
    let query = opts.query();
    format!(
        "<iframe\n  src=\"{origin}/embed/campaign/{id}?{query}\"\n  width=\"100%\"\n  \
         height=\"{height}\"\n  style=\"border: 0;\"\n  loading=\"lazy\"\n  \
         title=\"{title} on FlowTrace\"\n></iframe>\n<script>\n  \
         window.addEventListener('message', function (e) {{\n    \
         if (e.data && e.data.type === 'flowtrace-embed-height') {{\n      \
         var frame = document.querySelector('iframe[src*=\"/embed/campaign/{id}\"]');\n      \
         if (frame) frame.style.height = e.data.height + 'px';\n    }}\n  }});\n</script>"
    )
}

/// The preview card (`CampaignEmbedWidget`, `campaign-embed-widget.tsx`), full and compact, from
/// the same report the widget reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCard {
    /// Who runs it.
    pub organization: String,
    /// Campaign name.
    pub name: String,
    /// One-line summary.
    pub summary: Option<String>,
    /// `KES 3,200,000`.
    pub raised: String,
    /// `64%`.
    pub percent: String,
    /// `raised of KES 5,000,000`.
    pub of_target: String,
    /// The bar, with the spent-with-proof share.
    pub bar: Bar,
    /// `KES 900,000` spent with verified proof, when the campaign funds a node.
    pub evidenced: Option<String>,
    /// The Trust Score, `/100`.
    pub trust_score: i64,
    /// Timing up to its first comma: `63 days left`.
    pub timing: String,
    /// The compact card's line: `KES 3,200,000 · 64% of target`.
    pub compact_line: String,
    /// The Give link, while the campaign is open with a public link.
    pub give_url: Option<String>,
}

impl PreviewCard {
    /// The card of `page` with links on `origin`.
    #[must_use]
    pub fn of(page: &CampaignPage, origin: &str) -> Self {
        let c = &page.campaign;
        let pct = percent(c.current_amount, c.target_amount);
        let open = c.status == CampaignStatus::Active && !page.timing.ended;
        let raised = format_money(c.current_amount, &c.currency);
        Self {
            organization: page.report.organization.name.clone(),
            name: c.name.clone(),
            summary: c.short_description.clone().filter(|s| !s.is_empty()),
            percent: format!("{pct}%"),
            of_target: format!("raised of {}", format_money(c.target_amount, &c.currency)),
            bar: Bar {
                total: c.target_amount.max(c.current_amount),
                filled: c.current_amount,
                evidenced: page.report.evidenced_amount,
                pending: false,
            },
            evidenced: page
                .report
                .evidenced_amount
                .map(|e| format_money(e, &c.currency)),
            trust_score: page.report.organization.trust_score,
            timing: page
                .timing
                .label
                .split(',')
                .next()
                .unwrap_or_default()
                .to_owned(),
            compact_line: format!("{raised} \u{b7} {pct}% of target"),
            raised,
            give_url: c
                .public_slug
                .as_deref()
                .filter(|_| c.is_public && open)
                .map(|slug| donate_url(origin, slug)),
        }
    }
}

#[cfg(test)]
mod tests;
