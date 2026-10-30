//! The Chains screen's view models (DT2), plain Rust with no GPUI: the web's
//! `flowtrace/src/routes/_authenticated/chains.tsx` with its dialogs
//! (`components/chain-request-modal.tsx`, `components/chain-approval-modal.tsx`) and labels
//! (`components/chain-status-badge.tsx`) over the chains the caller's organisations are on
//! (`my_organization_chain`, ledger-module `transparency::chains::ChainDetails`).

use ledger_module::transparency::chains::{ChainDetails, ChainStatus, chain_trust_score};
use ledger_module::transparency::public::PublicOrganization;
use ledger_module::transparency::types::{AccessGroupPermissions, OrgType};

use crate::web_format::format_date;

/// `CHAIN_STATUS_LABEL` (`chain-status-badge.tsx:4-9`).
#[must_use]
pub fn status_label(status: ChainStatus) -> &'static str {
    match status {
        ChainStatus::Active => "Active",
        ChainStatus::Pending => "Pending",
        ChainStatus::Revoked => "Revoked",
        ChainStatus::Rejected => "Rejected",
    }
}

/// `ChainLegend` (`chain-status-badge.tsx:113-131`): the three line patterns and their words.
pub const LEGEND: [(ChainStatus, &str); 3] = [
    (ChainStatus::Active, "Active"),
    (ChainStatus::Pending, "Awaiting approval"),
    (ChainStatus::Revoked, "Revoked or rejected"),
];

/// `describeAccess` (`chain-approval-modal.tsx:34-48`): one plain sentence for what a group's
/// permissions let the other organisation see.
#[must_use]
pub fn describe_access(p: &AccessGroupPermissions) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if p.can_view_nodes {
        parts.push("the flow");
    }
    if p.can_view_amounts {
        parts.push("amounts");
    }
    if p.can_view_full_proofs {
        parts.push("original proofs");
    } else if p.can_view_proofs {
        parts.push("proof summaries");
    }
    if p.can_view_executor {
        parts.push("who handled each payment");
    }
    match parts.as_slice() {
        [] => "Sees nothing yet.".to_owned(),
        [one] => format!("Sees {one}."),
        [init @ .., last] => format!("Sees {} and {last}.", init.join(", ")),
    }
}

/// Which side of a chain the organisation is on (`Direction`, `chains.tsx:45`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// It asked: "Outgoing: flows you follow".
    Outgoing,
    /// It was asked: "Incoming: who can see your flows".
    Incoming,
}

/// What a row offers (`chains.tsx:452-489`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainAction {
    /// `Approve`, incoming and pending.
    Approve,
    /// `Reject`, incoming and pending.
    Reject,
    /// `Revoke access`, incoming and active.
    Revoke,
    /// `Cancel request`, outgoing and pending.
    Cancel,
}

impl ChainAction {
    /// The button's label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Approve => "Approve",
            Self::Reject => "Reject",
            Self::Revoke => "Revoke access",
            Self::Cancel => "Cancel request",
        }
    }
}

/// One chain as its row shows it (`ChainRow`, `chains.tsx:323-492`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRow {
    /// Chain id.
    pub id: String,
    /// Where it stands.
    pub status: ChainStatus,
    /// The source's name; `You` under it when outgoing.
    pub source: String,
    /// The target's name; `You` under it when incoming.
    pub target: String,
    /// Which side the organisation is on.
    pub direction: Direction,
    /// `Requested 20 Jan 2026`, `Active since ...`, `Revoked ...`, `Rejected ...`.
    pub when: String,
    /// The request message, while pending.
    pub message: Option<String>,
    /// The reason, once rejected.
    pub reason: Option<String>,
    /// `Access granted`: the group, `You choose on approval`, `They choose on approval`, `None`.
    pub access: String,
    /// What the granted group sees, while active.
    pub access_note: Option<String>,
    /// The Chain Trust Score, the rounded mean.
    pub score: i64,
    /// `Average of 87 and 92`.
    pub score_note: String,
    /// The buttons.
    pub actions: Vec<ChainAction>,
    /// The revoked or rejected rows read muted.
    pub ended: bool,
}

impl ChainRow {
    /// The row of `chain` seen from `direction`.
    #[must_use]
    pub fn of(chain: &ChainDetails, direction: Direction) -> Self {
        let c = &chain.chain;
        let when = match (c.status, &c.approved_at, &c.revoked_at) {
            (ChainStatus::Active, Some(at), _) => format!("Active since {}", format_date(at)),
            (ChainStatus::Revoked, _, Some(at)) => format!("Revoked {}", format_date(at)),
            (ChainStatus::Rejected, Some(at), _) => format!("Rejected {}", format_date(at)),
            _ => format!("Requested {}", format_date(&c.requested_at)),
        };
        let active = c.status == ChainStatus::Active;
        let access = match (&chain.granted_group, c.status) {
            (Some(g), ChainStatus::Active) => g.name.clone(),
            (_, ChainStatus::Pending) => match direction {
                Direction::Incoming => "You choose on approval".to_owned(),
                Direction::Outgoing => "They choose on approval".to_owned(),
            },
            _ => "None".to_owned(),
        };
        let score = chain_trust_score(chain.source_org.trust_score, chain.target_org.trust_score);
        let actions = match (direction, c.status) {
            (Direction::Incoming, ChainStatus::Pending) => {
                vec![ChainAction::Approve, ChainAction::Reject]
            }
            (Direction::Incoming, ChainStatus::Active) => vec![ChainAction::Revoke],
            (Direction::Outgoing, ChainStatus::Pending) => vec![ChainAction::Cancel],
            _ => Vec::new(),
        };
        Self {
            id: c.id.clone(),
            status: c.status,
            source: chain.source_org.name.clone(),
            target: chain.target_org.name.clone(),
            direction,
            when,
            message: c
                .request_message
                .clone()
                .filter(|m| !m.is_empty() && c.status == ChainStatus::Pending),
            reason: c
                .rejection_reason
                .clone()
                .filter(|r| !r.is_empty() && c.status == ChainStatus::Rejected),
            access,
            access_note: chain
                .granted_group
                .as_ref()
                .filter(|_| active)
                .map(|g| describe_access(&g.permissions)),
            score: score.aggregated_score,
            score_note: format!(
                "Average of {} and {}",
                score.scores.source_org, score.scores.target_org
            ),
            actions,
            ended: matches!(c.status, ChainStatus::Revoked | ChainStatus::Rejected),
        }
    }
}

/// `Incoming: who can see your flows` and its empty texts (`chains.tsx:153-165`).
pub const INCOMING: Section = Section {
    title: "Incoming: who can see your flows",
    empty_title: "No one has asked to link with you yet",
    empty_body: "When a funder or partner requests a view of your flows, the request appears \
                 here for you to approve or reject.",
    empty_action: false,
};

/// `Outgoing: flows you follow` (`chains.tsx:167-182`), whose empty notice offers a request.
pub const OUTGOING: Section = Section {
    title: "Outgoing: flows you follow",
    empty_title: "You don't follow any partner's flows yet",
    empty_body: "Request a link to see how an implementing partner spends the money you pass on.",
    empty_action: true,
};

/// A section's fixed texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// Heading.
    pub title: &'static str,
    /// Empty notice title.
    pub empty_title: &'static str,
    /// Empty notice body.
    pub empty_body: &'static str,
    /// Whether the empty notice offers "Request a link".
    pub empty_action: bool,
}

/// `{n} link(s)` beside a section heading (`chains.tsx:271-276`).
#[must_use]
pub fn links_meta(n: usize) -> String {
    format!("{n} {}", if n == 1 { "link" } else { "links" })
}

/// The screen: the counts and the two sections, pending first (`chains.tsx:50-243`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChainsView {
    /// The incoming rows: pending, active, rejected, revoked.
    pub incoming: Vec<ChainRow>,
    /// The outgoing rows, in the same order.
    pub outgoing: Vec<ChainRow>,
    /// `Active links`.
    pub active: usize,
    /// `Awaiting your review`.
    pub awaiting: usize,
    /// `Your requests pending`.
    pub requested: usize,
    /// `Ended`.
    pub ended: usize,
    /// Targets of the organisation's pending or active requests (`linkedOrgIds`).
    pub linked: Vec<String>,
}

/// Pending first: they need a decision (`chains.tsx:262-264`).
fn order(status: ChainStatus) -> u8 {
    match status {
        ChainStatus::Pending => 0,
        ChainStatus::Active => 1,
        ChainStatus::Rejected => 2,
        ChainStatus::Revoked => 3,
    }
}

impl ChainsView {
    /// The chains `org_id` is on, from every chain the caller can read.
    #[must_use]
    pub fn of(chains: &[ChainDetails], org_id: &str) -> Self {
        let side = |direction: Direction| {
            let mut rows: Vec<ChainRow> = chains
                .iter()
                .filter(|c| match direction {
                    Direction::Outgoing => c.chain.source_org_id == org_id,
                    Direction::Incoming => c.chain.target_org_id == org_id,
                })
                .map(|c| ChainRow::of(c, direction))
                .collect();
            rows.sort_by_key(|r| order(r.status));
            rows
        };
        let (incoming, outgoing) = (side(Direction::Incoming), side(Direction::Outgoing));
        let all = || incoming.iter().chain(&outgoing);
        let count = |want: &dyn Fn(ChainStatus) -> bool| all().filter(|r| want(r.status)).count();
        let pending = |rows: &[ChainRow]| {
            rows.iter()
                .filter(|r| r.status == ChainStatus::Pending)
                .count()
        };
        Self {
            active: count(&|s| s == ChainStatus::Active),
            awaiting: pending(&incoming),
            requested: pending(&outgoing),
            ended: count(&|s| matches!(s, ChainStatus::Revoked | ChainStatus::Rejected)),
            linked: chains
                .iter()
                .filter(|c| {
                    c.chain.source_org_id == org_id
                        && matches!(c.chain.status, ChainStatus::Pending | ChainStatus::Active)
                })
                .map(|c| c.chain.target_org_id.clone())
                .collect(),
            incoming,
            outgoing,
        }
    }
}

/// The lede (`chains.tsx:101-105`).
#[must_use]
pub fn lede(org_name: Option<&str>) -> String {
    format!(
        "{} and the partners whose flows are linked to yours. A funder can follow a grant along \
         each active link, seeing only what that link grants.",
        org_name.unwrap_or("Your organisation")
    )
}

/// The fallback when a refusal has no text (`chains.tsx:224-227`).
pub const FALLBACK_ERROR: &str = "That didn\u{2019}t go through. Try again in a moment.";

// ---------------------------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------------------------

/// A revoke or cancel confirmation (`chains.tsx:200-243`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    /// Title.
    pub title: String,
    /// Body.
    pub body: &'static str,
    /// The button that keeps things as they are.
    pub keep: &'static str,
    /// The destructive button.
    pub confirm: &'static str,
}

impl Confirm {
    /// `Revoke {source}'s access?`.
    #[must_use]
    pub fn revoke(row: &ChainRow) -> Self {
        Self {
            title: format!("Revoke {}'s access?", row.source),
            body: "They stop seeing your flows straight away. The link stays on record as \
                   revoked, and they would need to send a new request.",
            keep: "Keep access",
            confirm: "Revoke access",
        }
    }

    /// `Cancel your request to {target}?`.
    #[must_use]
    pub fn cancel(row: &ChainRow) -> Self {
        Self {
            title: format!("Cancel your request to {}?", row.target),
            body: "The request is withdrawn before they review it. You can send a new one later.",
            keep: "Keep request",
            confirm: "Cancel request",
        }
    }
}

/// `ORG_TYPE_LABEL` (`chain-request-modal.tsx:16-21`).
#[must_use]
pub fn org_type_label(t: OrgType) -> &'static str {
    match t {
        OrgType::Ngo => "NGO",
        OrgType::Sme => "Business",
        OrgType::SocialEnterprise => "Social enterprise",
        OrgType::Government => "Government",
    }
}

/// The request dialog's fixed texts (`chain-request-modal.tsx:77-236`).
pub mod request {
    /// Title.
    pub const TITLE: &str = "Request a chain link";
    /// Description.
    pub const DESCRIPTION: &str = "Ask another organisation for a view of their flows. They \
                                   choose what depth you see, and either side can end the link \
                                   later.";
    /// Search placeholder.
    pub const PLACEHOLDER: &str = "Search by name, e.g. Maji Safi";
    /// Under the search field.
    pub const HINT: &str = "Type at least two letters.";
    /// Message placeholder.
    pub const MESSAGE_PLACEHOLDER: &str =
        "Say which programme or grant you want to follow, and why.";
    /// Under the message.
    pub const MESSAGE_HINT: &str = "They see this when they review your request.";
    /// The send button.
    pub const SEND: &str = "Send request";
    /// While sending.
    pub const SENDING: &str = "Sending request";
    /// On a result already linked.
    pub const LINKED: &str = "Already linked";
}

/// One search result (`OrgLine`, `chain-request-modal.tsx:238-249`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgOption {
    /// Organisation id.
    pub id: String,
    /// Name.
    pub name: String,
    /// `NGO · KE · Trust Score 92`.
    pub line: String,
    /// A pending or active request already goes there.
    pub linked: bool,
}

impl OrgOption {
    /// The line of a published organisation.
    #[must_use]
    pub fn of(o: &PublicOrganization, linked: &[String]) -> Self {
        let mut head: Vec<&str> = Vec::new();
        if let Some(t) = o.org_type {
            head.push(org_type_label(t));
        }
        if !o.country.is_empty() {
            head.push(&o.country);
        }
        Self {
            id: o.id.clone(),
            name: o.name.clone(),
            line: format!(
                "{} \u{b7} Trust Score {}",
                head.join(" \u{b7} "),
                o.trust_score.unwrap_or(0)
            ),
            linked: linked.contains(&o.id),
        }
    }
}

/// What the results area shows for a query (`chain-request-modal.tsx:153-207`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Search {
    /// Fewer than two letters: the hint only.
    TooShort,
    /// Matches.
    Results(Vec<OrgOption>),
    /// None.
    NoMatch(String),
}

/// `searchOrganizations(query, excludeOrgId)` (`api.ts:182-190`) over the published
/// organisations (`public_organization`; the module has no search, so an organisation must have
/// made its profile public): a name containing the query, any case, not the organisation itself,
/// at most ten. Fewer than two letters after trimming searches nothing.
#[must_use]
pub fn search(
    orgs: &[PublicOrganization],
    query: &str,
    own_org: &str,
    linked: &[String],
) -> Search {
    let trimmed = query.trim();
    if trimmed.chars().count() < 2 {
        return Search::TooShort;
    }
    let q = trimmed.to_lowercase();
    let found: Vec<OrgOption> = orgs
        .iter()
        .filter(|o| o.id != own_org && o.name.to_lowercase().contains(&q))
        .take(10)
        .map(|o| OrgOption::of(o, linked))
        .collect();
    if found.is_empty() {
        Search::NoMatch(format!(
            "No organisation matches \u{201c}{trimmed}\u{201d}. Check the spelling, or ask them \
             to make their profile public on FlowTrace first."
        ))
    } else {
        Search::Results(found)
    }
}

/// The request's refusal, `The request wasn't sent. {message}` (`chain-request-modal.tsx:225-233`).
#[must_use]
pub fn request_error(message: &str) -> String {
    format!("The request wasn't sent. {message}")
}

/// Approve or reject (`ChainApprovalModal`'s `mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    /// Grant an access group.
    Approve,
    /// Decline, with an optional reason.
    Reject,
}

/// The review dialog's texts for one chain (`chain-approval-modal.tsx:110-288`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    /// `Approve Pwani County Health Office`.
    pub title: String,
    /// What approving or rejecting means.
    pub description: &'static str,
    /// The facts: From, Trust Score and, when there is one, Their message.
    pub facts: Vec<(&'static str, String)>,
    /// The submit button, idle and pending.
    pub submit: (&'static str, &'static str),
}

/// The approve dialog's access group label.
pub const GROUP_LABEL: &str = "Access group";
/// Its empty choice.
pub const GROUP_NONE: &str = "Choose a group";
/// The node label (`Join at node`, with `(optional)` muted).
pub const NODE_LABEL: &str = "Join at node";
/// Its empty choice.
pub const NODE_NONE: &str = "No specific node";
/// Under the node select.
pub const NODE_HINT: &str =
    "Where their flow joins yours, so a funder can follow the money across both.";
/// The reject reason's label (`Reason`, with `(optional)` muted).
pub const REASON_LABEL: &str = "Reason";
/// Its placeholder.
pub const REASON_PLACEHOLDER: &str =
    "For example: we only link with organisations we fund directly.";
/// The cancel button of the review dialog.
pub const KEEP_PENDING: &str = "Keep pending";

impl Review {
    /// The dialog for `chain` in `mode`.
    #[must_use]
    pub fn of(chain: &ChainDetails, mode: ReviewMode) -> Self {
        let requester = &chain.source_org;
        let mut facts = vec![
            ("From", requester.name.clone()),
            ("Trust Score", format!("{} of 100", requester.trust_score)),
        ];
        if let Some(m) = chain
            .chain
            .request_message
            .as_ref()
            .filter(|m| !m.is_empty())
        {
            facts.push(("Their message", m.clone()));
        }
        match mode {
            ReviewMode::Approve => Self {
                title: format!("Approve {}", requester.name),
                description: "Choose the access group they join. They see your flows at that \
                              group\u{2019}s depth, and you can revoke the link at any time.",
                facts,
                submit: ("Approve and grant access", "Approving"),
            },
            ReviewMode::Reject => Self {
                title: format!("Reject {}", requester.name),
                description: "They will see that you declined. They can send a new request later.",
                facts,
                submit: ("Reject request", "Rejecting"),
            },
        }
    }
}

/// The hint under the access group select, by priority (`chain-approval-modal.tsx:191-206`):
/// the missing choice, groups that failed to load, what the chosen group sees, else what the
/// choice sets.
#[must_use]
pub fn group_hint(
    missing: bool,
    groups_failed: bool,
    chosen: Option<&AccessGroupPermissions>,
) -> String {
    if missing {
        "Choose an access group before approving.".to_owned()
    } else if groups_failed {
        "Your access groups could not be loaded. Close this and try again.".to_owned()
    } else if let Some(p) = chosen {
        describe_access(p)
    } else {
        "This sets what they can see in your flows.".to_owned()
    }
}

#[cfg(test)]
mod tests;
