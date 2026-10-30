//! The Trust Score screen's view models (DT4), plain Rust with no GPUI: the web's
//! `flowtrace/src/routes/_authenticated/trust-score.tsx` over the module's trust row
//! (`ledger_module::transparency::trust::TrustScore`), its history snapshots, the organisation's
//! transaction counts (`flows::org_stats`) and the books' `ledger_signals`.
//!
//! The parts and weights are the module's four (verification 32, documentation 24, consistency
//! 24, model 20; pre-decision P16), as the web reads them since RW3 (`trustSummaryFrom`,
//! `flowtrace/src/lib/ledger/transparency-model.ts:1145-1162`).

use ledger_core::money::js_round;
use ledger_core::reports::ledger_signals;
use ledger_core::types::{IsoDate, LedgerState};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::{OrgStats, TransactionStatus, org_stats};
use ledger_module::transparency::trust::{
    ModelSource, TrustHistory, TrustScore, history, trust_grade,
};

use crate::web_format::format_date;

#[cfg(test)]
mod tests;

/// How many snapshots the History chart shows (`getTrustScoreHistory(orgId, limit = 12)`,
/// `flowtrace/src/lib/ledger/spacetime-transparency.ts:1500-1511`).
pub const HISTORY_LIMIT: usize = 12;

/// The header's lede (`trust-score.tsx:126-129`).
pub const LEDE: &str = "The score updates by itself each time a movement or a proof is recorded or changed. Recalculate adds today's score to the history.";

/// The recalculate failure (`trust-score.tsx:146-152`), with the reason when there is one.
#[must_use]
pub fn recalc_error(reason: &str) -> String {
    let reason = if reason.is_empty() {
        String::new()
    } else {
        format!(" ({reason})")
    };
    format!(
        "The score could not be recalculated{reason}. The figures below are still current; try again in a moment."
    )
}

/// One part of the score, as "How it is made" lists it (`COMPONENT_TEXT`,
/// `trust-score.tsx:39-62`, and the row at `:204-259`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentRow {
    /// `Verification`, `Documentation`, `Consistency`, `Independent review`.
    pub label: &'static str,
    /// `32% of score`.
    pub weight: String,
    /// `87`, rounded; `None` reads "Not yet scored".
    pub value: Option<String>,
    /// How much of the bar is filled, 0 to 100.
    pub filled: f64,
    /// The explanation and ` Adds n points.`
    pub explain: String,
}

/// The four parts in the web's order with their texts.
const PARTS: [(&str, &str); 4] = [
    (
        "Verification",
        "Share of movements whose proof has been checked and accepted.",
    ),
    (
        "Documentation",
        "Share of movements with at least one proof attached.",
    ),
    (
        "Consistency",
        "How well amounts, dates and parties agree across the records.",
    ),
    (
        "Independent review",
        "A review of the records against a fixed rubric, less 5 points for each confirmed flag. Until the first review it matches the other three, so it does not move the score.",
    ),
];

impl ComponentRow {
    fn of(part: usize, value: f64, weight: u8, scored: bool) -> Self {
        let (label, explain) = PARTS[part];
        Self {
            label,
            weight: format!("{weight}% of score"),
            value: scored.then(|| js_round(value).to_string()),
            filled: if scored { value.clamp(0.0, 100.0) } else { 0.0 },
            explain: format!(
                "{explain} Adds {} points.",
                js_round(value * f64::from(weight) / 100.0)
            ),
        }
    }

    /// The rows of `row`, verification first; the model part is scored once a rubric review
    /// stored scores (`scored: model.source === 'rubric'`).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "component values are 0 to 100")]
    pub fn all(row: &TrustScore) -> Vec<Self> {
        let c = &row.components;
        let w = &row.weights;
        vec![
            Self::of(0, c.verification as f64, w.verification, true),
            Self::of(1, c.documentation as f64, w.documentation, true),
            Self::of(2, c.consistency as f64, w.consistency, true),
            Self::of(3, c.model, w.model, row.model.source == ModelSource::Rubric),
        ]
    }
}

/// `ScoreChange` (`trust-score.tsx:311-349`): what the score moved by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreChange {
    /// Fewer than two snapshots.
    NotYet,
    /// Since the previous and since the first snapshot, then when the latest was taken.
    Rows {
        /// `Since 1 Jan 2026`, `+2 pts`.
        since_previous: (String, String),
        /// `Since 1 Jul 2025`, `+25 pts`.
        since_first: (String, String),
        /// `as of 1 Feb 2026`.
        last: String,
    },
}

/// The note shown while there is no change to show.
pub const CHANGE_NOT_YET: &str = "A change will show after the next calculation.";

impl ScoreChange {
    /// The change across `history`, oldest first.
    #[must_use]
    pub fn of(history: &[TrustHistory]) -> Self {
        let (Some(first), Some(last), true) = (history.first(), history.last(), history.len() > 1)
        else {
            return Self::NotYet;
        };
        let prev = &history[history.len() - 2];
        let signed = |n: i64| {
            if n > 0 {
                format!("+{n} pts")
            } else {
                format!("{n} pts")
            }
        };
        Self::Rows {
            since_previous: (
                format!("Since {}", format_date(&prev.calculated_at)),
                signed(last.score - prev.score),
            ),
            since_first: (
                format!("Since {}", format_date(&first.calculated_at)),
                signed(last.score - first.score),
            ),
            last: format!("as of {}", format_date(&last.calculated_at)),
        }
    }
}

/// The History chart's geometry (`HistoryChart`, `trust-score.tsx:361-495`): positions are
/// percent of the plot box, `x` from the left and `y` from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryChart {
    /// The axis floor: `max(0, floor((min - 5) / 10) * 10)`.
    pub lo: i64,
    /// The y ticks with their `y`, from `lo` up to 100.
    pub ticks: Vec<(i64, f64)>,
    /// Each snapshot's point.
    pub points: Vec<(f64, f64)>,
    /// The x labels shown, `Jul 25`, with their `x`; every second one past six points, ending
    /// on the latest.
    pub labels: Vec<(f64, String)>,
    /// The latest score, printed by the enlarged last dot.
    pub last_score: i64,
    /// The chart in words (the SVG's `aria-label`).
    pub summary: String,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `monthLabel(iso)` (`trust-score.tsx:352-355`): `Jul 25`, in UTC.
#[must_use]
pub fn month_label(iso: &str) -> String {
    let month: usize = iso.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    let year = iso.get(2..4).unwrap_or("");
    format!("{} {year}", MONTHS[month.clamp(1, 12) - 1])
}

impl HistoryChart {
    /// The chart of `history`, oldest first; `None` when there is none.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "scores are 0 to 100 and there are at most a dozen points"
    )]
    pub fn of(history: &[TrustHistory]) -> Option<Self> {
        let (first, last) = (history.first()?, history.last()?);
        let min = history.iter().map(|h| h.score).min().unwrap_or(0);
        let lo = ((min - 5).div_euclid(10) * 10).max(0);
        let hi = 100;
        let step = if lo <= 50 { 25 } else { 10 };
        let y = |v: i64| (hi - v) as f64 / (hi - lo) as f64 * 100.0;
        let mut ticks: Vec<(i64, f64)> = (lo..=hi).step_by(step).map(|t| (t, y(t))).collect();
        if ticks.last().map(|t| t.0) != Some(hi) {
            ticks.push((hi, y(hi)));
        }
        let n = history.len();
        let x = |i: usize| {
            if n == 1 {
                50.0
            } else {
                i as f64 / (n - 1) as f64 * 100.0
            }
        };
        let every = if n > 6 { 2 } else { 1 };
        Some(Self {
            lo,
            ticks,
            points: history
                .iter()
                .enumerate()
                .map(|(i, h)| (x(i), y(h.score)))
                .collect(),
            labels: history
                .iter()
                .enumerate()
                .filter(|(i, _)| i % every == (n - 1) % every)
                .map(|(i, h)| (x(i), month_label(&h.calculated_at)))
                .collect(),
            last_score: last.score,
            summary: format!(
                "Trust Score from {} on {} to {} on {}",
                first.score,
                format_date(&first.calculated_at),
                last.score,
                format_date(&last.calculated_at)
            ),
        })
    }
}

/// Where a "How to raise it" link goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaiseLink {
    /// The Transactions list filtered by status.
    Transactions(TransactionStatus),
    /// One transaction.
    Transaction(String),
}

/// One "How to raise it" item (`Action`, `trust-score.tsx:497-503`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseAction {
    /// `pending`, `flagged`, `unproven`, `proofs`.
    pub key: &'static str,
    /// The status line it is marked with.
    pub mark: TransactionStatus,
    /// The headline.
    pub title: String,
    /// The sentence under it.
    pub detail: String,
    /// The link text and where it goes.
    pub link: Option<(&'static str, RaiseLink)>,
}

/// The "Nothing outstanding" notice (`trust-score.tsx:603-610`).
pub const NOTHING_OUTSTANDING: (&str, &str) = (
    "Nothing outstanding",
    "Every movement is evidenced and nothing is flagged. Keep attaching proof as you record new transactions to hold the score.",
);

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `RaiseActions` (`trust-score.tsx:505-623`): what would lift the score, from the
/// organisation's counts and the two weights; `flagged_id` is the one flagged movement when
/// there is exactly one. Empty means "Nothing outstanding".
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "transaction counts")]
pub fn raise_actions(
    stats: &OrgStats,
    verification_weight: u8,
    documentation_weight: u8,
    flagged_id: Option<&str>,
) -> Vec<RaiseAction> {
    let total = stats.total;
    let proofs = &stats.proof_stats;
    let unproven = total.saturating_sub(proofs.transactions_with_proofs);
    let points_for = |count: u64| {
        if total > 0 {
            js_round(count as f64 / total as f64 * f64::from(verification_weight))
        } else {
            0.0
        }
    };
    let mut actions = Vec::new();
    if stats.pending > 0 {
        actions.push(RaiseAction {
            key: "pending",
            mark: TransactionStatus::Pending,
            title: format!(
                "{} awaiting proof",
                plural(stats.pending, "movement is", "movements are")
            ),
            detail: format!(
                "Attach a receipt, photo or statement to each. Evidencing all of them adds up to {} points.",
                points_for(stats.pending)
            ),
            link: Some((
                "Show movements awaiting proof",
                RaiseLink::Transactions(TransactionStatus::Pending),
            )),
        });
    }
    if stats.flagged > 0 {
        actions.push(RaiseAction {
            key: "flagged",
            mark: TransactionStatus::Flagged,
            title: format!(
                "{} flagged",
                plural(stats.flagged, "movement is", "movements are")
            ),
            detail: "Resolve the flag with a matching receipt, or record why the money moved."
                .to_owned(),
            link: Some(flagged_id.map_or(
                (
                    "Review flagged movements",
                    RaiseLink::Transactions(TransactionStatus::Flagged),
                ),
                |id| {
                    (
                        "Review the flagged movement",
                        RaiseLink::Transaction(id.to_owned()),
                    )
                },
            )),
        });
    }
    if unproven > 0 {
        actions.push(RaiseAction {
            key: "unproven",
            mark: TransactionStatus::Pending,
            title: format!(
                "{} no proof attached",
                plural(unproven, "movement has", "movements have")
            ),
            detail: format!(
                "Documentation is {}% covered and counts for {documentation_weight}% of the score.",
                proofs.proof_coverage_rate
            ),
            link: None,
        });
    }
    if proofs.pending_proofs > 0 {
        actions.push(RaiseAction {
            key: "proofs",
            mark: TransactionStatus::Pending,
            title: format!(
                "{} waiting for review",
                plural(proofs.pending_proofs, "proof is", "proofs are")
            ),
            detail: "A reviewer can accept them from each transaction\u{2019}s page.".to_owned(),
            link: None,
        });
    }
    actions
}

/// One "Ledger signals" figure (`LedgerSignals`, `trust-score.tsx:632-704`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    /// `64%`.
    pub value: String,
    /// The label.
    pub label: &'static str,
    /// The sentence under it.
    pub detail: &'static str,
    /// The link text.
    pub link: &'static str,
}

/// The "Ledger signals" lede.
pub const SIGNALS_LEDE: &str = "Read from the books, alongside the score. They do not change it.";

/// The three ledger signals of `state` up to `as_of` (`ledgerSignals`, `reports.ts:599-622`).
#[must_use]
pub fn signals(state: &LedgerState, as_of: &IsoDate) -> Vec<Signal> {
    let s = ledger_signals(state, Some(as_of));
    let pct = |n: f64| format!("{}%", js_round(n * 100.0));
    vec![
        Signal {
            value: pct(s.reconciled_share),
            label: "Cash movement reconciled",
            detail: "Share of bank and mobile-money movement, by value, matched to a statement line.",
            link: "Reconcile accounts",
        },
        Signal {
            value: pct(s.in_budget_share),
            label: "Spend within budget",
            detail: "Share of budgeted spend on budget lines that are not over.",
            link: "Budget against actual",
        },
        Signal {
            value: pct(s.grant_coded_share),
            label: "Spend coded to a grant",
            detail: "Share of spend charged to a named grant, so donors can trace it.",
            link: "See grants",
        },
    ]
}

/// Everything the Trust Score screen shows of one organisation.
#[derive(Debug, Clone, PartialEq)]
pub struct TrustPage {
    /// The score: the trust row's, else the latest snapshot's, else 0.
    pub score: i64,
    /// Its grade: the row's, else `getTrustGrade(score)`.
    pub grade: String,
    /// `calculated 4 Feb 2026`, when known.
    pub calculated: Option<String>,
    /// The change since earlier snapshots.
    pub change: ScoreChange,
    /// The four parts; empty reads "Not yet calculated".
    pub components: Vec<ComponentRow>,
    /// `last 8 calculations`, when there are any.
    pub history_meta: Option<String>,
    /// The chart; `None` reads "No history yet".
    pub chart: Option<HistoryChart>,
    /// What would raise the score; empty reads "Nothing outstanding".
    pub actions: Vec<RaiseAction>,
}

/// The "Not yet calculated" notice.
pub const NOT_CALCULATED: (&str, &str) = (
    "Not yet calculated",
    "The score is worked out when the first movement is recorded.",
);

/// The "No history yet" notice.
pub const NO_HISTORY: (&str, &str) = (
    "No history yet",
    "Each recalculation adds a point here, so you can see the score move. Use Recalculate score to record the first point; the score above is already current.",
);

impl TrustPage {
    /// The page of `t` (`TrustScore`, `trust-score.tsx:69-306`).
    #[must_use]
    pub fn of(t: &OrgTransparency) -> Self {
        let history = history(t, HISTORY_LIMIT);
        let latest = history.last();
        let row = t.trust.as_ref();
        let score = row
            .map(|r| r.score)
            .or(latest.map(|h| h.score))
            .unwrap_or(0);
        let grade = row.map_or_else(|| trust_grade(score).to_owned(), |r| r.grade.clone());
        let calculated = row
            .map(|r| r.computed_at.as_str())
            .or(latest.map(|h| h.calculated_at.as_str()))
            .map(|at| format!("calculated {}", format_date(at)));
        let weights = row.map(|r| r.weights);
        let flagged: Vec<&str> = t
            .transactions
            .iter()
            .filter(|x| x.verification_status == TransactionStatus::Flagged)
            .map(|x| x.id.as_str())
            .collect();
        let flagged_id = match flagged.as_slice() {
            [one] => Some(*one),
            _ => None,
        };
        Self {
            score,
            grade,
            calculated,
            change: ScoreChange::of(&history),
            components: row.map(ComponentRow::all).unwrap_or_default(),
            history_meta: (!history.is_empty())
                .then(|| format!("last {} calculations", history.len())),
            chart: HistoryChart::of(&history),
            actions: raise_actions(
                &org_stats(t),
                weights.map_or(0, |w| w.verification),
                weights.map_or(0, |w| w.documentation),
                flagged_id,
            ),
        }
    }
}
