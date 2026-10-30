//! The Dashboard's view models (DT1), plain Rust with no GPUI: the web's
//! `flowtrace/src/routes/_authenticated/dashboard.tsx` over the organisation's transparency
//! records, its money movements between flow nodes. Key figures, "Funding received, by donor"
//! (`summarise`, `dashboard.tsx:38-95`) and the eight latest movements. Amounts are minor
//! units; text follows `lib/format.ts` through [`crate::web_format`].

use std::cmp::Ordering;

use ledger_core::Minor;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::{
    NodeType, TransactionStatus, TransactionWithNodes, with_nodes,
};
use ledger_module::transparency::trust::trust_grade;

use crate::web_format::{format_amount, format_compact, format_date};

/// How many movements "Recent movements" lists (`RECENT_COUNT`, `dashboard.tsx:28`).
pub const RECENT_COUNT: usize = 8;

/// One donor's money in (`DonorLine`, `dashboard.tsx:30-36`), minor units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorLine {
    /// The donor node.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Received and not refused, proven or not.
    pub received: Minor,
    /// Received with verified proof.
    pub evidenced: Minor,
    /// Recorded and awaiting proof.
    pub awaiting: Minor,
}

/// `summarise(transactions)` (`dashboard.tsx:38-95`), minor units.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// The latest movement's date, `YYYY-MM-DD`.
    pub latest: Option<String>,
    /// Movements.
    pub total: usize,
    /// Verified movements.
    pub verified: usize,
    /// Movements awaiting proof.
    pub pending: usize,
    /// Flagged movements.
    pub flagged: usize,
    /// Money from donor nodes, evidenced first.
    pub donor_lines: Vec<DonorLine>,
    /// Their evidenced total.
    pub evidenced_total: Minor,
    /// Their received and awaiting total.
    pub committed_total: Minor,
    /// The largest donor's received and awaiting, the bars' full width.
    pub largest: Minor,
}

/// The organisation's movements as `getTransactions` lists them (`api.ts:202-209`): newest
/// `transaction_date` first, ties in record order, each with its node references.
#[must_use]
pub fn transactions(t: &OrgTransparency) -> Vec<TransactionWithNodes> {
    let mut list: Vec<TransactionWithNodes> =
        t.transactions.iter().map(|x| with_nodes(t, x)).collect();
    list.sort_by(|a, b| {
        b.transaction
            .transaction_date
            .cmp(&a.transaction.transaction_date)
    });
    list
}

/// `summarise` (`dashboard.tsx:38-95`): counts by proof state and each donor's money, donors
/// with evidence first, then most evidenced, then most received and awaiting (a stable sort,
/// as `Array.prototype.sort`).
#[must_use]
pub fn summarise(transactions: &[TransactionWithNodes]) -> Summary {
    let mut s = Summary {
        total: transactions.len(),
        ..Summary::default()
    };
    for x in transactions {
        let t = &x.transaction;
        if s.latest.as_ref().is_none_or(|l| &t.transaction_date > l) {
            s.latest = Some(t.transaction_date.clone());
        }
        match t.verification_status {
            TransactionStatus::Verified => s.verified += 1,
            TransactionStatus::Pending => s.pending += 1,
            TransactionStatus::Flagged => s.flagged += 1,
            TransactionStatus::Rejected => {}
        }
        let Some(from) = x
            .from_node
            .as_ref()
            .filter(|n| n.node_type == NodeType::Donor)
        else {
            continue;
        };
        let at = if let Some(i) = s.donor_lines.iter().position(|d| d.id == from.id) {
            i
        } else {
            s.donor_lines.push(DonorLine {
                id: from.id.clone(),
                name: from.name.clone(),
                received: 0,
                evidenced: 0,
                awaiting: 0,
            });
            s.donor_lines.len() - 1
        };
        let line = &mut s.donor_lines[at];
        match t.verification_status {
            TransactionStatus::Pending => line.awaiting += t.amount,
            TransactionStatus::Rejected => {}
            _ => line.received += t.amount,
        }
        if t.verification_status == TransactionStatus::Verified {
            line.evidenced += t.amount;
        }
    }
    s.donor_lines.sort_by(by_evidence);
    s.evidenced_total = s.donor_lines.iter().map(|d| d.evidenced).sum();
    s.committed_total = s.donor_lines.iter().map(|d| d.received + d.awaiting).sum();
    s.largest = s
        .donor_lines
        .iter()
        .map(|d| d.received + d.awaiting)
        .max()
        .unwrap_or(0)
        .max(0);
    s
}

/// One key figure as the web's `KeyFigure` shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Figure {
    /// The figure.
    pub value: String,
    /// What it counts.
    pub label: String,
    /// A flagged note under it, if any.
    pub detail: Option<String>,
}

/// A coverage bar (`CoverageBar`): a full width, the filled part and the evidenced part of it,
/// minor units; `pending` draws the filled part as awaiting proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    /// The full width.
    pub total: Minor,
    /// The filled part.
    pub filled: Minor,
    /// The darker, evidenced part.
    pub evidenced: Option<Minor>,
    /// Drawn as awaiting proof.
    pub pending: bool,
}

/// A row of "Funding received, by donor" (`DonorRow`, `dashboard.tsx:296-344`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonorRow {
    /// The donor's name.
    pub name: String,
    /// `KES 15.7M`.
    pub amount: String,
    /// `evidenced`, `awaiting proof` or `KES 1,200 evidenced`.
    pub state: String,
    /// The bar against the largest donor.
    pub bar: Bar,
}

impl DonorRow {
    /// The row of `donor` with bars against `largest`.
    #[must_use]
    pub fn of(donor: &DonorLine, largest: Minor) -> Self {
        let only_pending = donor.received == 0 && donor.awaiting > 0;
        let amount = donor.received + donor.awaiting;
        let state = if only_pending {
            "awaiting proof".to_owned()
        } else if donor.evidenced >= donor.received {
            "evidenced".to_owned()
        } else {
            format!("KES {} evidenced", format_amount(donor.evidenced))
        };
        Self {
            name: donor.name.clone(),
            amount: format!("KES {}", format_compact(amount)),
            state,
            bar: Bar {
                total: largest,
                filled: if only_pending { amount } else { donor.received },
                evidenced: (!only_pending).then_some(donor.evidenced),
                pending: only_pending,
            },
        }
    }
}

/// `PROOF_STATUS_LABEL` (`components/report/status-line.tsx`).
#[must_use]
pub fn proof_label(status: TransactionStatus) -> &'static str {
    match status {
        TransactionStatus::Verified => "Evidenced",
        TransactionStatus::Pending => "Awaiting proof",
        TransactionStatus::Flagged => "Flagged",
        TransactionStatus::Rejected => "Rejected",
    }
}

/// A row of "Recent movements" (`RecentMovements`, `dashboard.tsx:346-462`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementRow {
    /// The transaction.
    pub id: String,
    /// `From → To`, `Unassigned` for a missing node.
    pub movement: String,
    /// What it was.
    pub description: String,
    /// `18 Jan 2026`.
    pub date: String,
    /// `KES 1,450,000`.
    pub amount: String,
    /// Its proof state.
    pub status: TransactionStatus,
}

impl MovementRow {
    /// The row of one movement.
    #[must_use]
    pub fn of(x: &TransactionWithNodes) -> Self {
        let name = |n: Option<&ledger_module::transparency::flows::NodeRef>| {
            n.map_or_else(|| "Unassigned".to_owned(), |n| n.name.clone())
        };
        let t = &x.transaction;
        Self {
            id: t.id.clone(),
            movement: format!(
                "{} \u{2192} {}",
                name(x.from_node.as_ref()),
                name(x.to_node.as_ref())
            ),
            description: t.description.clone(),
            date: format_date(&t.transaction_date),
            amount: format!("{} {}", t.currency, format_amount(t.amount)),
            status: t.verification_status,
        }
    }
}

/// Everything the Dashboard shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    /// `as of 4 Feb 2026`, from the latest movement.
    pub as_of: Option<String>,
    /// No movements recorded: the empty notice instead of the report.
    pub empty: bool,
    /// The four key figures.
    pub figures: Vec<Figure>,
    /// Donor rows, evidenced first.
    pub donors: Vec<DonorRow>,
    /// `KES 157,000 evidenced of 217,000 committed`.
    pub donor_total: String,
    /// The eight latest movements.
    pub recent: Vec<MovementRow>,
}

impl Dashboard {
    /// The Dashboard of `t`. The Trust Score is the trust row's (`organization.trust_score ??
    /// 0` on the web, `dashboard.tsx:104`); its grade is `getTrustGrade`.
    #[must_use]
    pub fn of(t: &OrgTransparency) -> Self {
        let list = transactions(t);
        let s = summarise(&list);
        let score = t.trust.as_ref().map_or(0, |r| r.score);
        let figures = vec![
            Figure {
                value: format_compact(s.evidenced_total),
                label: "KES received with proof".to_owned(),
                detail: None,
            },
            Figure {
                value: format!("{}/{}", s.verified, s.total),
                label: "movements evidenced".to_owned(),
                detail: None,
            },
            Figure {
                value: s.pending.to_string(),
                label: "awaiting proof".to_owned(),
                detail: (s.flagged > 0).then(|| format!("{} flagged", s.flagged)),
            },
            Figure {
                value: score.to_string(),
                label: format!("Trust Score, grade {}", trust_grade(score)),
                detail: None,
            },
        ];
        Self {
            as_of: s
                .latest
                .as_deref()
                .map(|d| format!("as of {}", format_date(d))),
            empty: s.total == 0,
            figures,
            donors: s
                .donor_lines
                .iter()
                .map(|d| DonorRow::of(d, s.largest))
                .collect(),
            donor_total: format!(
                "KES {} evidenced of {} committed",
                format_amount(s.evidenced_total),
                format_amount(s.committed_total)
            ),
            recent: list
                .iter()
                .take(RECENT_COUNT)
                .map(MovementRow::of)
                .collect(),
        }
    }
}

/// The donor order of `summarise` (`dashboard.tsx:69-76`): with evidence first, most evidenced,
/// then most received and awaiting.
fn by_evidence(a: &DonorLine, b: &DonorLine) -> Ordering {
    (b.evidenced > 0)
        .cmp(&(a.evidenced > 0))
        .then(b.evidenced.cmp(&a.evidenced))
        .then((b.received + b.awaiting).cmp(&(a.received + a.awaiting)))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::backend::demo_transparency;

    /// The figures below are the web's `summarise` run over the same dataset
    /// (`transparency-dataset.json#/store`, major units), times 100.
    #[test]
    fn the_demo_summary_matches_the_webs() {
        let t = demo_transparency().unwrap();
        let s = summarise(&transactions(&t));
        assert_eq!(s.latest.as_deref(), Some("2026-02-04"));
        assert_eq!((s.total, s.verified, s.pending, s.flagged), (16, 12, 3, 1));
        let lines: Vec<(&str, Minor, Minor, Minor)> = s
            .donor_lines
            .iter()
            .map(|d| (d.name.as_str(), d.received, d.evidenced, d.awaiting))
            .collect();
        assert_eq!(
            lines,
            [
                ("Harambee Health Fund", 850_000_000, 850_000_000, 0),
                ("Kisima Water Trust", 400_000_000, 400_000_000, 0),
                ("Northlight Foundation", 320_000_000, 320_000_000, 0),
                ("Pan-African Education Fund", 0, 0, 600_000_000),
            ]
        );
        assert_eq!(
            (s.evidenced_total, s.committed_total, s.largest),
            (1_570_000_000, 2_170_000_000, 850_000_000)
        );
    }

    #[test]
    fn the_demo_dashboard_reads_as_the_web_page() {
        let t = demo_transparency().unwrap();
        let d = Dashboard::of(&t);
        let score = t.trust.as_ref().unwrap().score;
        assert_eq!(d.as_of.as_deref(), Some("as of 4 Feb 2026"));
        assert!(!d.empty);
        let figures: Vec<(&str, &str, Option<&str>)> = d
            .figures
            .iter()
            .map(|f| (f.value.as_str(), f.label.as_str(), f.detail.as_deref()))
            .collect();
        let trust_label = format!("Trust Score, grade {}", trust_grade(score));
        assert_eq!(
            figures,
            [
                ("15.7M", "KES received with proof", None),
                ("12/16", "movements evidenced", None),
                ("3", "awaiting proof", Some("1 flagged")),
                (score.to_string().as_str(), trust_label.as_str(), None),
            ]
        );
        assert_eq!(
            d.donor_total,
            "KES 15,700,000 evidenced of 21,700,000 committed"
        );
        assert_eq!(d.donors[0].amount, "KES 8.5M");
        assert_eq!(d.donors[0].state, "evidenced");
        let pending = &d.donors[3];
        assert_eq!(pending.state, "awaiting proof");
        assert_eq!(
            pending.bar,
            Bar {
                total: 850_000_000,
                filled: 600_000_000,
                evidenced: None,
                pending: true
            }
        );

        assert_eq!(d.recent.len(), RECENT_COUNT);
        let first = &d.recent[0];
        assert_eq!(
            first.movement,
            "Pan-African Education Fund \u{2192} Project Fund"
        );
        assert_eq!(first.date, "4 Feb 2026");
        assert_eq!(first.amount, "KES 6,000,000");
        assert_eq!(proof_label(first.status), "Awaiting proof");
        assert_eq!(d.recent[4].movement, "Project Fund \u{2192} Unassigned");
        assert_eq!(proof_label(d.recent[4].status), "Flagged");
    }

    #[test]
    fn a_partly_evidenced_donor_names_the_evidenced_part() {
        let line = DonorLine {
            id: "n".to_owned(),
            name: "Donor".to_owned(),
            received: 500_000,
            evidenced: 120_000,
            awaiting: 0,
        };
        assert_eq!(DonorRow::of(&line, 500_000).state, "KES 1,200 evidenced");
    }

    #[test]
    fn no_movements_is_the_empty_dashboard() {
        let d = Dashboard::of(&OrgTransparency::default());
        assert!(d.empty);
        assert_eq!(d.as_of, None);
        assert_eq!(d.figures[3].value, "0");
        assert_eq!(d.figures[3].label, "Trust Score, grade F");
    }
}
