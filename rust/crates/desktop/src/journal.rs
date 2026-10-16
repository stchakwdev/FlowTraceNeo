//! View models of the Journal screen: the entry list with its status, source, period and text
//! filters, and one entry's detail with its lines, its audit trail and the actions its status
//! allows (post, delete, reverse). Plain structs, no GPUI; the panel in `panels::journal` only
//! draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/ledger/index.tsx`,
//! `entries.$id.tsx` and `-shared.tsx`; the rules stay in `ledger-core` (`engine/entries.rs`),
//! whose messages are shown as they come.

use std::collections::HashSet;

use ledger_core::demo::demo_actor;
use ledger_core::js_number::to_js_string;
use ledger_core::types::{
    ActorId, EntryId, EntryStatus, IsoDate, JournalEntry, JournalLine, LedgerState, PeriodId,
    SourceType,
};
use ledger_core::{CommandResult, LedgerCommand, Minor};

use crate::accounts::{format_date, format_minor};
use crate::claims::{format_date_time, iso_to_day_month_year, parse_day_month_year};

/// The en dash the web app shows for an empty cell (`money.tsx:12`).
const DASH: &str = "\u{2013}";

/// `SOURCE_LABEL` (`-shared.tsx:15-27`).
#[must_use]
pub fn source_label(source: SourceType) -> &'static str {
    match source {
        SourceType::Manual => "Manual entry",
        SourceType::Transaction => "Transaction",
        SourceType::Bill => "Bill",
        SourceType::Payment => "Bill payment",
        SourceType::ExpenseClaim => "Expense claim",
        SourceType::Reimbursement => "Claim reimbursement",
        SourceType::Statement => "Bank statement",
        SourceType::Opening => "Opening balances",
        SourceType::YearEndClose => "Year-end close",
        SourceType::Reversal => "Reversal",
        SourceType::Import => "Import",
    }
}

/// The sources in the web's `SOURCE_LABEL` order, for the source filter.
pub const SOURCES: [SourceType; 11] = [
    SourceType::Manual,
    SourceType::Transaction,
    SourceType::Bill,
    SourceType::Payment,
    SourceType::ExpenseClaim,
    SourceType::Reimbursement,
    SourceType::Statement,
    SourceType::Opening,
    SourceType::YearEndClose,
    SourceType::Reversal,
    SourceType::Import,
];

/// `ENTRY_STATUS_LABEL` (`-shared.tsx:29-33`).
#[must_use]
pub fn entry_status_label(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Draft => "Draft",
        EntryStatus::Posted => "Posted",
        EntryStatus::Reversed => "Reversed",
    }
}

/// The status filter's choices (`index.tsx:39`, `[undefined, ...STATUSES]`).
pub const STATUS_FILTERS: [Option<EntryStatus>; 4] = [
    None,
    Some(EntryStatus::Draft),
    Some(EntryStatus::Posted),
    Some(EntryStatus::Reversed),
];

/// `entryTotal` (`-shared.tsx:104-109`): the debits of an entry's lines in base currency.
#[must_use]
pub fn entry_total<'a>(lines: impl IntoIterator<Item = &'a JournalLine>) -> Minor {
    lines.into_iter().map(|l| l.base_amount.max(0)).sum()
}

/// `formatMinor(v, { signed: true, zero })` (`money.tsx:8-21`): negatives with a minus sign
/// (U+2212) rather than parentheses, as the base column of the lines table shows them.
#[must_use]
pub fn format_signed(minor: Minor, zero: &str) -> String {
    match minor {
        0 => zero.to_owned(),
        m if m < 0 => format!("\u{2212}{}", format_minor(-m)),
        m => format_minor(m),
    }
}

/// One row of the journal list (`index.tsx`, `Row`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRow {
    /// The entry.
    pub entry: JournalEntry,
    /// Its debits in base currency.
    pub total: Minor,
    /// How many lines it has.
    pub line_count: usize,
    /// The funds its lines charge, in line order, by name.
    pub funds: String,
}

/// True when the entry's override reason is its own, not the reason a reversal or a
/// year-end close carries (`index.tsx`, `EntryTable`; `entries.$id.tsx`).
fn shows_override(entry: &JournalEntry) -> bool {
    entry.override_reason.is_some()
        && !matches!(
            entry.source_type,
            SourceType::Reversal | SourceType::YearEndClose
        )
}

impl EntryRow {
    /// `3 lines`, then ` · posted with an override` when an override reason is kept on an
    /// entry that is not a reversal or a year-end close (`index.tsx`, `EntryTable`).
    #[must_use]
    pub fn detail(&self) -> String {
        let n = self.line_count;
        let lines = format!("{n} {}", if n == 1 { "line" } else { "lines" });
        if shows_override(&self.entry) {
            format!("{lines} \u{b7} posted with an override")
        } else {
            lines
        }
    }
}

/// Every entry of `state`, latest date first, then latest number (`index.tsx`, `all`).
#[must_use]
pub fn entry_rows(state: &LedgerState) -> Vec<EntryRow> {
    let mut rows: Vec<EntryRow> = state
        .entries
        .iter()
        .map(|entry| {
            let lines: Vec<&JournalLine> = state
                .lines
                .iter()
                .filter(|l| l.entry_id == entry.id)
                .collect();
            let mut seen = HashSet::new();
            let funds: Vec<&str> = lines
                .iter()
                .filter(|l| seen.insert(&l.fund_id))
                .map(|l| {
                    state
                        .funds
                        .iter()
                        .find(|f| f.id == l.fund_id)
                        .map_or("Unknown fund", |f| f.name.as_str())
                })
                .collect();
            EntryRow {
                entry: entry.clone(),
                total: entry_total(lines.iter().copied()),
                line_count: lines.len(),
                funds: funds.join(", "),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.entry
            .date
            .cmp(&a.entry.date)
            .then_with(|| b.entry.number.cmp(&a.entry.number))
    });
    rows
}

/// Entries per status (`index.tsx`, `counts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatusCounts {
    /// All entries.
    pub all: usize,
    /// Drafts.
    pub draft: usize,
    /// Posted.
    pub posted: usize,
    /// Reversed.
    pub reversed: usize,
}

impl StatusCounts {
    /// The counts of `rows`.
    #[must_use]
    pub fn of(rows: &[EntryRow]) -> Self {
        let mut c = Self {
            all: rows.len(),
            ..Self::default()
        };
        for r in rows {
            match r.entry.status {
                EntryStatus::Draft => c.draft += 1,
                EntryStatus::Posted => c.posted += 1,
                EntryStatus::Reversed => c.reversed += 1,
            }
        }
        c
    }

    /// The count behind a status filter choice.
    #[must_use]
    pub fn get(&self, status: Option<EntryStatus>) -> usize {
        match status {
            None => self.all,
            Some(EntryStatus::Draft) => self.draft,
            Some(EntryStatus::Posted) => self.posted,
            Some(EntryStatus::Reversed) => self.reversed,
        }
    }
}

/// What the filter bar holds (`index.tsx`, `JournalSearch`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JournalFilter {
    /// Draft, posted or reversed; `None` for all.
    pub status: Option<EntryStatus>,
    /// One source; `None` for all.
    pub source: Option<SourceType>,
    /// One period; `None` for all.
    pub period: Option<PeriodId>,
    /// Searched in the number, memo and override reason, ignoring case.
    pub query: String,
}

impl JournalFilter {
    /// True when anything is filtered (`index.tsx`, `filtered`).
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        self.status.is_some()
            || self.source.is_some()
            || self.period.is_some()
            || !self.query.is_empty()
    }

    /// The rows of `all` this filter keeps, in order (`index.tsx`, `rows`).
    #[must_use]
    pub fn apply(&self, all: &[EntryRow]) -> Vec<EntryRow> {
        let needle = self.query.trim().to_lowercase();
        all.iter()
            .filter(|r| {
                let e = &r.entry;
                self.status.is_none_or(|s| e.status == s)
                    && self.source.is_none_or(|s| e.source_type == s)
                    && self.period.as_ref().is_none_or(|p| &e.period_id == p)
                    && (needle.is_empty()
                        || [Some(&e.number), Some(&e.memo), e.override_reason.as_ref()]
                            .into_iter()
                            .flatten()
                            .any(|f| f.to_lowercase().contains(&needle)))
            })
            .cloned()
            .collect()
    }
}

/// The source filter's choices: All sources, then each source present in the books
/// (`index.tsx`, `sourceOptions`).
#[must_use]
pub fn source_choices(all: &[EntryRow]) -> Vec<(Option<SourceType>, &'static str)> {
    let present: HashSet<SourceType> = all.iter().map(|r| r.entry.source_type).collect();
    std::iter::once((None, "All sources"))
        .chain(
            SOURCES
                .into_iter()
                .filter(|s| present.contains(s))
                .map(|s| (Some(s), source_label(s))),
        )
        .collect()
}

/// The period filter's choices: All periods, then the periods latest first (`index.tsx`,
/// `periodOptions`).
#[must_use]
pub fn period_choices(state: &LedgerState) -> Vec<(Option<PeriodId>, String)> {
    let mut periods: Vec<_> = state.periods.iter().collect();
    periods.sort_by(|a, b| b.start_date.cmp(&a.start_date));
    std::iter::once((None, "All periods".to_owned()))
        .chain(
            periods
                .into_iter()
                .map(|p| (Some(p.id.clone()), p.name.clone())),
        )
        .collect()
}

/// A notice in place of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The heading.
    pub title: String,
    /// The text under it.
    pub body: String,
}

/// `No entries yet` or `No entries match` (`index.tsx`), or `None` when rows show.
#[must_use]
pub fn list_notice(all: usize, shown: usize, filter: &JournalFilter) -> Option<Notice> {
    if all == 0 {
        return Some(Notice {
            title: "No entries yet".to_owned(),
            body: "Entries appear here as transactions, bills and claims post, or when you \
                   write one by hand."
                .to_owned(),
        });
    }
    if shown > 0 {
        return None;
    }
    Some(Notice {
        title: "No entries match".to_owned(),
        body: if filter.query.is_empty() {
            "No entries have this combination of status, source and period.".to_owned()
        } else {
            format!(
                "Nothing matches \u{201c}{}\u{201d} with these filters.",
                filter.query
            )
        },
    })
}

/// The line under the table: `Total: 22 entries` or `Shown: 1 entry` (`index.tsx`).
#[must_use]
pub fn list_footer(shown: usize, filter: &JournalFilter) -> String {
    format!(
        "{}: {shown} {}",
        if filter.is_filtered() {
            "Shown"
        } else {
            "Total"
        },
        if shown == 1 { "entry" } else { "entries" }
    )
}

/// `actorName` (`-shared.tsx:120-126`): who acted, for the audit trail.
#[must_use]
pub fn actor_name(id: &ActorId) -> String {
    let id = id.as_str();
    let demo = demo_actor();
    if id == "system" {
        "FlowTrace (automatic)".to_owned()
    } else if id == demo.id.as_str() {
        demo.name
    } else if let Some(name) = id.strip_prefix("staff-") {
        let mut chars = name.chars();
        chars
            .next()
            .map(|c| c.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    } else {
        format!("Member {}", id.chars().take(8).collect::<String>())
    }
}

/// `AUDIT_ACTION_LABEL` (`-shared.tsx:128-140`), the action itself when it has no label.
#[must_use]
pub fn audit_label(action: &str) -> String {
    match action {
        "entry.draft" => "Saved as draft",
        "entry.update" => "Draft edited",
        "entry.post" => "Posted",
        "entry.delete" => "Draft deleted",
        "entry.reverse" => "Reversed",
        "account.create" => "Account added",
        "account.update" => "Account changed",
        "period.close" => "Period closed",
        "period.reopen" => "Period reopened",
        "period.lock" => "Period locked",
        "year.close" => "Year closed",
        other => other,
    }
    .to_owned()
}

/// What can be done to an entry (`entries.$id.tsx`, `actions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryAction {
    /// Open the draft in the entry form.
    Edit,
    /// Delete the draft.
    Delete,
    /// Post the draft.
    Post,
    /// Reverse the posted entry.
    Reverse,
}

impl EntryAction {
    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Edit => "Edit",
            Self::Delete => "Delete",
            Self::Post => "Post",
            Self::Reverse => "Reverse",
        }
    }
}

/// The actions of an entry by its status: a draft is edited, deleted or posted, a posted entry
/// reversed, a reversed one left alone (`entries.$id.tsx`, `actions`).
#[must_use]
pub fn actions_for(status: EntryStatus) -> Vec<EntryAction> {
    match status {
        EntryStatus::Draft => vec![EntryAction::Edit, EntryAction::Delete, EntryAction::Post],
        EntryStatus::Posted => vec![EntryAction::Reverse],
        EntryStatus::Reversed => Vec::new(),
    }
}

/// One line of the entry detail (`entries.$id.tsx`, the lines table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineView {
    /// `1110 · Bank: Main operating account`, or `Unknown account`.
    pub account: String,
    /// The line's own memo.
    pub memo: Option<String>,
    /// Fund name, or `Unknown fund`.
    pub fund: String,
    /// Grant name, `No grant`, or `Unknown grant`.
    pub grant: String,
    /// Dimension names joined by commas, or an en dash.
    pub tags: String,
    /// Debit in the line's currency.
    pub debit: Minor,
    /// Credit in the line's currency.
    pub credit: Minor,
    /// The line's currency, shown only when some line is not in base currency.
    pub currency: Option<String>,
    /// Signed base-currency amount.
    pub base_amount: Minor,
    /// `at 129.5` for a line not in base currency.
    pub rate: Option<String>,
}

/// One event of the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRow {
    /// `4 Feb 2026, 09:00`.
    pub at: String,
    /// `Posted`.
    pub label: String,
    /// `FlowTrace (automatic)`.
    pub by: String,
    /// The reason of a reversal.
    pub reason: Option<String>,
}

/// A linked entry: its id and how the link reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryLink {
    /// The entry to open.
    pub id: EntryId,
    /// `JE-0003 Rent for February` or `JE-0024 on 4 Feb 2026`.
    pub text: String,
}

/// One entry as its detail page shows it (`entries.$id.tsx`, `EntryDetail`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    /// The entry.
    pub entry: JournalEntry,
    /// Its lines in book order.
    pub lines: Vec<LineView>,
    /// Debits in base currency, the key figure.
    pub total: Minor,
    /// Sum of the base debits, for the totals row.
    pub debits: Minor,
    /// Sum of the base credits.
    pub credits: Minor,
    /// `Date · Feb 2026`, or `Date` when the period is unknown.
    pub date_label: String,
    /// `Manual entry`.
    pub source: &'static str,
    /// `4 Feb 2026, 09:00 by Demo User`.
    pub created: String,
    /// When and by whom it was posted, or why it counts nowhere yet.
    pub posted: String,
    /// The override reason, except on reversals and year-end closes.
    pub override_reason: Option<String>,
    /// The entry this one reverses.
    pub reverses: Option<EntryLink>,
    /// The entry that reversed this one.
    pub reversed_by: Option<EntryLink>,
    /// Audit events on the entry, oldest first.
    pub audit: Vec<AuditRow>,
    /// What can be done to it.
    pub actions: Vec<EntryAction>,
}

/// The lines of `entry` as the detail table shows them (`entries.$id.tsx`, `accountName`,
/// `fundName`, `grantName`, `dimNames`, `foreign`).
fn line_views(state: &LedgerState, lines: &[&JournalLine]) -> Vec<LineView> {
    let base = &state.settings.base_currency;
    let foreign = lines.iter().any(|l| &l.currency != base);
    lines
        .iter()
        .map(|l| {
            let account = state
                .accounts
                .iter()
                .find(|a| a.id == l.account_id)
                .map_or_else(
                    || "Unknown account".to_owned(),
                    |a| format!("{} \u{b7} {}", a.code, a.name),
                );
            let fund = state
                .funds
                .iter()
                .find(|f| f.id == l.fund_id)
                .map_or_else(|| "Unknown fund".to_owned(), |f| f.name.clone());
            let grant = l.grant_id.as_ref().map_or_else(
                || "No grant".to_owned(),
                |id| {
                    state
                        .grants
                        .iter()
                        .find(|g| &g.id == id)
                        .map_or_else(|| "Unknown grant".to_owned(), |g| g.name.clone())
                },
            );
            let tags: Vec<&str> = l
                .dimension_ids
                .iter()
                .filter_map(|id| {
                    state
                        .dimensions
                        .iter()
                        .find(|d| &d.id == id)
                        .map(|d| d.name.as_str())
                })
                .collect();
            LineView {
                account,
                memo: l.memo.clone(),
                fund,
                grant,
                tags: if tags.is_empty() {
                    DASH.to_owned()
                } else {
                    tags.join(", ")
                },
                debit: l.debit,
                credit: l.credit,
                currency: foreign.then(|| l.currency.clone()),
                base_amount: l.base_amount,
                rate: (&l.currency != base).then(|| format!("at {}", to_js_string(l.fx_rate))),
            }
        })
        .collect()
}

impl EntryView {
    /// The entry `id` of `state`, or `None` when it is not in the books.
    #[must_use]
    pub fn new(state: &LedgerState, id: &EntryId) -> Option<Self> {
        let entry = state.entries.iter().find(|e| &e.id == id)?;
        let lines: Vec<&JournalLine> = state.lines.iter().filter(|l| &l.entry_id == id).collect();
        let debits = lines.iter().map(|l| l.base_amount.max(0)).sum();
        let credits = lines.iter().map(|l| (-l.base_amount).max(0)).sum();
        let find = |id: &EntryId| state.entries.iter().find(|e| &e.id == id);
        let reverses = entry
            .reversal_of
            .as_ref()
            .and_then(find)
            .map(|e| EntryLink {
                id: e.id.clone(),
                text: format!("{} {}", e.number, e.memo),
            });
        let reversed_by = entry
            .reversed_by
            .as_ref()
            .and_then(find)
            .map(|e| EntryLink {
                id: e.id.clone(),
                text: format!("{} on {}", e.number, format_date(&e.date)),
            });
        let mut audit: Vec<_> = state
            .audit
            .iter()
            .filter(|a| a.subject == id.as_str())
            .collect();
        audit.sort_by(|a, b| a.at.cmp(&b.at));
        let audit = audit
            .into_iter()
            .map(|a| AuditRow {
                at: format_date_time(&a.at),
                label: audit_label(&a.action),
                by: actor_name(&a.by),
                reason: (!a.detail.is_empty() && a.action == "entry.reverse")
                    .then(|| a.detail.clone()),
            })
            .collect();
        let posted = match (&entry.posted_at, &entry.posted_by) {
            (Some(at), Some(by)) => format!("{} by {}", format_date_time(at), actor_name(by)),
            _ => "Not posted. Drafts do not count in any balance or report.".to_owned(),
        };
        Some(Self {
            lines: line_views(state, &lines),
            total: entry_total(lines.iter().copied()),
            debits,
            credits,
            date_label: state
                .periods
                .iter()
                .find(|p| p.id == entry.period_id)
                .map_or_else(|| "Date".to_owned(), |p| format!("Date \u{b7} {}", p.name)),
            source: source_label(entry.source_type),
            created: format!(
                "{} by {}",
                format_date_time(&entry.created_at),
                actor_name(&entry.created_by)
            ),
            posted,
            override_reason: shows_override(entry)
                .then(|| entry.override_reason.clone())
                .flatten(),
            reverses,
            reversed_by,
            audit,
            actions: actions_for(entry.status),
            entry: entry.clone(),
        })
    }

    /// The title of a refusal on this entry: `Not posted` for a draft, else `Not done`.
    #[must_use]
    pub fn failure_title(&self) -> &'static str {
        if self.entry.status == EntryStatus::Draft {
            "Not posted"
        } else {
            "Not done"
        }
    }

    /// `entry.postDraft`, with an override reason when given (`entries.$id.tsx`, `post`).
    #[must_use]
    pub fn post_command(&self, override_reason: Option<&str>) -> LedgerCommand {
        LedgerCommand::EntryPostDraft {
            entry_id: self.entry.id.clone(),
            override_reason: override_reason.map(str::to_owned),
        }
    }

    /// `entry.deleteDraft` (`entries.$id.tsx`, `remove`).
    #[must_use]
    pub fn delete_command(&self) -> LedgerCommand {
        LedgerCommand::EntryDeleteDraft {
            entry_id: self.entry.id.clone(),
        }
    }
}

/// The override reason the Post with override box asks for (`entries.$id.tsx`,
/// `postWithOverride`): the trimmed text, or the message when it is blank.
///
/// # Errors
///
/// The message to show under the field when the reason is blank.
pub fn override_reason(text: &str) -> Result<String, String> {
    let reason = text.trim();
    if reason.is_empty() {
        Err("Say why this entry should post despite the rule.".to_owned())
    } else {
        Ok(reason.to_owned())
    }
}

/// The confirmation of Delete (`entries.$id.tsx`, the delete `ReportDialog`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDialog {
    /// `Delete JE-0023?`
    pub title: String,
    /// What deleting does.
    pub description: String,
    /// The memo of the draft.
    pub memo: String,
}

impl DeleteDialog {
    /// The dialog for `entry`.
    #[must_use]
    pub fn of(entry: &JournalEntry) -> Self {
        Self {
            title: format!("Delete {}?", entry.number),
            description: "The draft and its lines are removed. Nothing posted is affected."
                .to_owned(),
            memo: entry.memo.clone(),
        }
    }
}

/// The messages under the reversal fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReverseErrors {
    /// Under the date.
    pub date: Option<String>,
    /// Under the reason.
    pub reason: Option<String>,
}

/// The Reverse dialog (`entries.$id.tsx`, `ReverseDialog`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseForm {
    /// `Reverse JE-0016`.
    pub title: String,
    /// What reversing does.
    pub description: String,
    /// The date as typed, `dd/mm/yyyy`; starts on the as-of date.
    pub date: String,
    /// Why, as typed.
    pub reason: String,
}

impl ReverseForm {
    /// The dialog for `entry`, dated `as_of`.
    #[must_use]
    pub fn new(entry: &JournalEntry, as_of: &IsoDate) -> Self {
        Self {
            title: format!("Reverse {}", entry.number),
            description: "A mirror-image entry is posted on the date you choose. The original \
                          stays in the books, marked reversed."
                .to_owned(),
            date: iso_to_day_month_year(as_of),
            reason: String::new(),
        }
    }

    /// `entry.reverse` with the parsed date and the trimmed reason (`ReverseDialog.submit`).
    ///
    /// # Errors
    ///
    /// The field messages when the date or the reason is missing or wrong.
    pub fn command(&self, entry: &EntryId) -> Result<LedgerCommand, ReverseErrors> {
        let mut errors = ReverseErrors::default();
        let date = match parse_day_month_year(&self.date) {
            None => {
                errors.date = Some("Enter the date of the reversal.".to_owned());
                None
            }
            Some(Err(message)) => {
                errors.date = Some(message);
                None
            }
            Some(Ok(date)) => Some(date),
        };
        let reason = self.reason.trim();
        if reason.is_empty() {
            errors.reason = Some("Say why the entry is being reversed.".to_owned());
        }
        match date {
            Some(date) if errors == ReverseErrors::default() => Ok(LedgerCommand::EntryReverse {
                entry_id: entry.clone(),
                date: date.as_str().to_owned(),
                reason: reason.to_owned(),
            }),
            _ => Err(errors),
        }
    }
}

/// The entry a command created, to open after it (`ReverseDialog`, `onDone`; `createdId`).
#[must_use]
pub fn created_entry(result: &CommandResult) -> Option<EntryId> {
    match result {
        CommandResult::Id { id } => Some(EntryId::from(id.as_str())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::accounts::Failure;
    use crate::backend::{BackendError, InProcessBackend, LedgerBackend as _};
    use crate::claims::has_overridable;
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger, demo_actor};
    use ledger_core::types::{AccountId, EntryInput, FundId, LineInput, Nullish, OrganizationId};

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

    /// JE-0016, the demo's latest entry: the Pan-African Education Fund grant received.
    const LATEST: &str = "demo-0032";

    struct Demo {
        backend: InProcessBackend,
        org: OrganizationId,
    }

    impl Demo {
        fn new() -> Self {
            let backend = InProcessBackend::with_demo_and_empty().unwrap();
            let org = backend.organisations()[0].id.clone();
            Self { backend, org }
        }

        fn submit(&mut self, command: LedgerCommand) -> Result<CommandResult, BackendError> {
            self.backend.submit(&self.org, command)
        }

        fn state(&self) -> LedgerState {
            LedgerState::clone(&self.backend.state(&self.org).unwrap())
        }

        fn view(&self, id: &EntryId) -> EntryView {
            EntryView::new(&self.state(), id).unwrap()
        }

        /// Saves a balanced manual draft of office supplies paid from the bank.
        fn draft(&mut self, memo: &str) -> EntryId {
            let r = self
                .submit(LedgerCommand::EntrySaveDraft {
                    input: supplies(memo, 2_500.0, None),
                })
                .unwrap();
            created_entry(&r).unwrap()
        }
    }

    fn line(account: &str, debit: f64, credit: f64, fund: &str) -> LineInput {
        LineInput {
            account_id: AccountId::from(account),
            debit: Some(debit),
            credit: Some(credit),
            currency: None,
            fx_rate: None,
            fund_id: FundId::from(fund),
            grant_id: Nullish::Undefined,
            dimension_ids: None,
            flow_node_id: Nullish::Undefined,
            memo: Nullish::Undefined,
        }
    }

    /// Office supplies (7100) paid from the main bank account (1110), on `fund` when given.
    fn supplies(memo: &str, amount: f64, fund: Option<&str>) -> EntryInput {
        let fund = fund.unwrap_or("fund-unrestricted");
        EntryInput {
            date: DEMO_AS_OF.to_owned(),
            memo: memo.to_owned(),
            lines: vec![
                line("acc-7100", amount, 0.0, fund),
                line("acc-1110", 0.0, amount, fund),
            ],
            source_type: None,
            source_id: Nullish::Undefined,
            override_reason: Nullish::Undefined,
        }
    }

    #[test]
    fn labels_read_as_the_web_screen() {
        let sources: Vec<_> = SOURCES.into_iter().map(source_label).collect();
        assert_eq!(
            sources,
            [
                "Manual entry",
                "Transaction",
                "Bill",
                "Bill payment",
                "Expense claim",
                "Claim reimbursement",
                "Bank statement",
                "Opening balances",
                "Year-end close",
                "Reversal",
                "Import",
            ]
        );
        let statuses: Vec<_> = STATUS_FILTERS
            .into_iter()
            .flatten()
            .map(entry_status_label)
            .collect();
        assert_eq!(statuses, ["Draft", "Posted", "Reversed"]);
        assert_eq!(audit_label("entry.draft"), "Saved as draft");
        assert_eq!(audit_label("entry.update"), "Draft edited");
        assert_eq!(audit_label("entry.post"), "Posted");
        assert_eq!(audit_label("entry.reverse"), "Reversed");
        assert_eq!(audit_label("bill.create"), "bill.create");
        assert_eq!(
            actor_name(&ActorId::from("system")),
            "FlowTrace (automatic)"
        );
        assert_eq!(actor_name(&demo_actor().id), demo_actor().name);
        assert_eq!(actor_name(&ActorId::from("staff-achieng")), "Achieng");
        assert_eq!(
            actor_name(&ActorId::from("0123456789abcdef")),
            "Member 01234567"
        );
    }

    #[test]
    fn the_list_runs_latest_first_with_totals_and_funds() {
        let state = demo();
        let rows = entry_rows(&state);
        assert_eq!(rows.len(), state.entries.len());
        let first: Vec<_> = rows
            .iter()
            .take(4)
            .map(|r| (r.entry.number.as_str(), r.entry.date.as_str()))
            .collect();
        // Same date: the later number first (JE-0019 before JE-0017).
        assert_eq!(
            first,
            [
                ("JE-0016", "2026-02-04"),
                ("JE-0015", "2026-02-03"),
                ("JE-0019", "2026-02-02"),
                ("JE-0017", "2026-02-02"),
            ]
        );
        assert_eq!(rows.last().unwrap().entry.number, "JE-0001");
        let latest = &rows[0];
        assert_eq!(latest.total, 600_000_000);
        assert_eq!(latest.line_count, 2);
        assert_eq!(latest.funds, "Education (restricted)");
        assert_eq!(latest.detail(), "2 lines");
        for r in &rows {
            let lines = state.lines.iter().filter(|l| l.entry_id == r.entry.id);
            assert_eq!(r.total, entry_total(lines));
        }
    }

    #[test]
    fn an_override_shows_on_the_row_but_not_on_a_reversal() {
        let state = demo();
        let mut row = entry_rows(&state).remove(0);
        row.entry.override_reason = Some("Board approved".into());
        assert_eq!(row.detail(), "2 lines \u{b7} posted with an override");
        row.entry.source_type = SourceType::Reversal;
        assert_eq!(row.detail(), "2 lines");
        row.line_count = 1;
        assert_eq!(row.detail(), "1 line");
    }

    #[test]
    fn filters_narrow_by_status_source_period_and_text() {
        let mut demo_org = Demo::new();
        demo_org.draft("Printer paper for the Lodwar office");
        let state = demo_org.state();
        let all = entry_rows(&state);
        let counts = StatusCounts::of(&all);
        assert_eq!(
            counts,
            StatusCounts {
                all: 23,
                draft: 1,
                posted: 22,
                reversed: 0
            }
        );
        assert_eq!(counts.get(None), 23);
        assert_eq!(counts.get(Some(EntryStatus::Draft)), 1);

        let none = JournalFilter::default();
        assert!(!none.is_filtered());
        assert_eq!(none.apply(&all), all);

        let drafts = JournalFilter {
            status: Some(EntryStatus::Draft),
            ..JournalFilter::default()
        };
        assert!(drafts.is_filtered());
        let shown = drafts.apply(&all);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].entry.memo, "Printer paper for the Lodwar office");
        assert_eq!(shown[0].entry.source_type, SourceType::Manual);

        let bills = JournalFilter {
            source: Some(SourceType::Bill),
            ..JournalFilter::default()
        };
        let numbers: Vec<_> = bills
            .apply(&all)
            .into_iter()
            .map(|r| r.entry.source_type)
            .collect();
        assert_eq!(numbers, [SourceType::Bill, SourceType::Bill]);

        let december = JournalFilter {
            period: Some(PeriodId::from("period-2025-12")),
            ..JournalFilter::default()
        };
        let shown = december.apply(&all);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].entry.number, "JE-0001");

        // Text matches the number, the memo or the override reason, ignoring case.
        let text = |q: &str| JournalFilter {
            query: q.into(),
            ..JournalFilter::default()
        };
        assert_eq!(text("je-0016").apply(&all).len(), 1);
        assert_eq!(text("  PRINTER  ").apply(&all).len(), 1);
        assert_eq!(text("nothing like this").apply(&all).len(), 0);
        assert!(text("x").is_filtered());
    }

    #[test]
    fn filter_choices_follow_the_books() {
        let state = demo();
        let all = entry_rows(&state);
        let sources = source_choices(&all);
        assert_eq!(
            sources,
            [
                (None, "All sources"),
                (Some(SourceType::Transaction), "Transaction"),
                (Some(SourceType::Bill), "Bill"),
                (Some(SourceType::Payment), "Bill payment"),
                (Some(SourceType::ExpenseClaim), "Expense claim"),
                (Some(SourceType::Reimbursement), "Claim reimbursement"),
                (Some(SourceType::Statement), "Bank statement"),
                (Some(SourceType::Opening), "Opening balances"),
            ]
        );
        let periods = period_choices(&state);
        assert_eq!(periods.len(), state.periods.len() + 1);
        assert_eq!(periods[0], (None, "All periods".to_owned()));
        // Latest period first.
        let latest = state
            .periods
            .iter()
            .max_by(|a, b| a.start_date.cmp(&b.start_date))
            .unwrap();
        assert_eq!(periods[1], (Some(latest.id.clone()), latest.name.clone()));
    }

    #[test]
    fn notices_and_footer_say_what_shows() {
        let none = JournalFilter::default();
        assert_eq!(
            list_notice(0, 0, &none),
            Some(Notice {
                title: "No entries yet".into(),
                body: "Entries appear here as transactions, bills and claims post, or when you \
                       write one by hand."
                    .into(),
            })
        );
        let drafts = JournalFilter {
            status: Some(EntryStatus::Draft),
            ..JournalFilter::default()
        };
        assert_eq!(
            list_notice(22, 0, &drafts),
            Some(Notice {
                title: "No entries match".into(),
                body: "No entries have this combination of status, source and period.".into(),
            })
        );
        let text = JournalFilter {
            query: "rent".into(),
            ..JournalFilter::default()
        };
        assert_eq!(
            list_notice(22, 0, &text).unwrap().body,
            "Nothing matches \u{201c}rent\u{201d} with these filters."
        );
        assert_eq!(list_notice(22, 3, &text), None);
        assert_eq!(list_footer(22, &none), "Total: 22 entries");
        assert_eq!(list_footer(1, &drafts), "Shown: 1 entry");
    }

    #[test]
    fn base_amounts_are_signed() {
        assert_eq!(
            format_signed(-600_000_000, "\u{2013}"),
            "\u{2212}6,000,000.00"
        );
        assert_eq!(format_signed(125_050, "\u{2013}"), "1,250.50");
        assert_eq!(format_signed(0, "0.00"), "0.00");
    }

    #[test]
    fn status_maps_to_actions() {
        assert_eq!(
            actions_for(EntryStatus::Draft),
            [EntryAction::Edit, EntryAction::Delete, EntryAction::Post]
        );
        assert_eq!(actions_for(EntryStatus::Posted), [EntryAction::Reverse]);
        assert_eq!(actions_for(EntryStatus::Reversed), []);
        let labels: Vec<_> = [
            EntryAction::Edit,
            EntryAction::Delete,
            EntryAction::Post,
            EntryAction::Reverse,
        ]
        .into_iter()
        .map(EntryAction::label)
        .collect();
        assert_eq!(labels, ["Edit", "Delete", "Post", "Reverse"]);
    }

    #[test]
    fn a_posted_entry_shows_lines_figures_and_trail() {
        let state = demo();
        let view = EntryView::new(&state, &EntryId::from(LATEST)).unwrap();
        assert_eq!(view.entry.number, "JE-0016");
        assert_eq!(view.total, 600_000_000);
        assert_eq!((view.debits, view.credits), (600_000_000, 600_000_000));
        assert_eq!(view.date_label, "Date \u{b7} Feb 2026");
        assert_eq!(view.source, "Transaction");
        assert_eq!(view.created, "4 Feb 2026, 09:00 by FlowTrace (automatic)");
        assert_eq!(view.posted, "4 Feb 2026, 09:00 by FlowTrace (automatic)");
        assert_eq!(view.override_reason, None);
        assert_eq!(
            (view.reverses.clone(), view.reversed_by.clone()),
            (None, None)
        );
        assert_eq!(view.actions, [EntryAction::Reverse]);
        assert_eq!(view.failure_title(), "Not done");
        assert_eq!(
            view.lines,
            [
                LineView {
                    account: "1130 \u{b7} Bank: Restricted projects account".into(),
                    memo: None,
                    fund: "Education (restricted)".into(),
                    grant: "PAEF education grant".into(),
                    tags: "\u{2013}".into(),
                    debit: 600_000_000,
                    credit: 0,
                    currency: None,
                    base_amount: 600_000_000,
                    rate: None,
                },
                LineView {
                    account: "4100 \u{b7} Grant income".into(),
                    memo: None,
                    fund: "Education (restricted)".into(),
                    grant: "PAEF education grant".into(),
                    tags: "Pan-African Education Fund".into(),
                    debit: 0,
                    credit: 600_000_000,
                    currency: None,
                    base_amount: -600_000_000,
                    rate: None,
                },
            ]
        );
        let trail: Vec<_> = view
            .audit
            .iter()
            .map(|a| (a.at.as_str(), a.label.as_str(), a.by.as_str()))
            .collect();
        assert_eq!(
            trail,
            [
                (
                    "4 Feb 2026, 09:00",
                    "Saved as draft",
                    "FlowTrace (automatic)"
                ),
                ("4 Feb 2026, 09:00", "Posted", "FlowTrace (automatic)"),
            ]
        );
        assert_eq!(EntryView::new(&state, &EntryId::from("nope")), None);
    }

    #[test]
    fn a_foreign_line_shows_its_currency_and_rate() {
        let mut state = demo();
        let i = state
            .lines
            .iter()
            .position(|l| l.entry_id.as_str() == LATEST)
            .unwrap();
        state.lines[i].currency = "USD".into();
        state.lines[i].fx_rate = 129.5;
        state.lines[i].debit = 4_633_204;
        let view = EntryView::new(&state, &EntryId::from(LATEST)).unwrap();
        assert_eq!(view.lines[0].currency.as_deref(), Some("USD"));
        assert_eq!(view.lines[0].rate.as_deref(), Some("at 129.5"));
        // Every line names its currency once one is foreign.
        assert_eq!(view.lines[1].currency.as_deref(), Some("KES"));
        assert_eq!(view.lines[1].rate, None);
    }

    #[test]
    fn a_draft_is_posted_from_its_detail() {
        let mut demo_org = Demo::new();
        let id = demo_org.draft("Printer paper");
        let view = demo_org.view(&id);
        assert_eq!(view.entry.status, EntryStatus::Draft);
        assert_eq!(
            view.actions,
            [EntryAction::Edit, EntryAction::Delete, EntryAction::Post]
        );
        assert_eq!(view.source, "Manual entry");
        assert_eq!(
            view.posted,
            "Not posted. Drafts do not count in any balance or report."
        );
        assert_eq!(view.created, "4 Feb 2026, 09:00 by Demo User");
        assert_eq!(view.failure_title(), "Not posted");
        assert_eq!(view.audit.len(), 1);
        assert_eq!(view.audit[0].label, "Saved as draft");
        assert_eq!(
            view.post_command(None),
            LedgerCommand::EntryPostDraft {
                entry_id: id.clone(),
                override_reason: None
            }
        );
        demo_org.submit(view.post_command(None)).unwrap();
        let view = demo_org.view(&id);
        assert_eq!(view.entry.status, EntryStatus::Posted);
        assert_eq!(view.posted, "4 Feb 2026, 09:00 by Demo User");
        assert_eq!(view.actions, [EntryAction::Reverse]);
        let labels: Vec<_> = view.audit.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(labels, ["Saved as draft", "Posted"]);
    }

    #[test]
    fn a_restricted_draft_needs_an_override_to_post() {
        let mut demo_org = Demo::new();
        // Spend on the PAEF grant before it starts: the engine flags it, and an override lets
        // it through.
        let mut input = supplies("School books", 1_000.0, Some("fund-education"));
        input.date = "2026-01-20".into();
        for l in &mut input.lines {
            l.grant_id = Nullish::Value(ledger_core::types::GrantId::from("grant-paef-education"));
        }
        let r = demo_org
            .submit(LedgerCommand::EntrySaveDraft { input })
            .unwrap();
        let id = created_entry(&r).unwrap();
        let view = demo_org.view(&id);
        let err = demo_org.submit(view.post_command(None)).unwrap_err();
        assert!(has_overridable(&err), "{err:?}");
        let failure = Failure::new(view.failure_title(), &err);
        assert_eq!(failure.title, "Not posted");
        assert_eq!(failure.messages.len(), 1);
        assert!(
            failure.messages[0].ends_with("PAEF education grant starts on 2026-02-01."),
            "{failure:?}"
        );

        assert_eq!(
            override_reason("   "),
            Err("Say why this entry should post despite the rule.".to_owned())
        );
        let reason = override_reason("  Donor agreed by email  ").unwrap();
        assert_eq!(reason, "Donor agreed by email");
        assert_eq!(
            view.post_command(Some(&reason)),
            LedgerCommand::EntryPostDraft {
                entry_id: id.clone(),
                override_reason: Some(reason.clone())
            }
        );
        demo_org.submit(view.post_command(Some(&reason))).unwrap();
        let view = demo_org.view(&id);
        assert_eq!(view.entry.status, EntryStatus::Posted);
        assert_eq!(
            view.override_reason.as_deref(),
            Some("Donor agreed by email")
        );
    }

    #[test]
    fn a_draft_is_deleted_after_a_confirmation() {
        let mut demo_org = Demo::new();
        let id = demo_org.draft("Printer paper");
        let view = demo_org.view(&id);
        let dialog = DeleteDialog::of(&view.entry);
        assert_eq!(dialog.title, format!("Delete {}?", view.entry.number));
        assert_eq!(
            dialog.description,
            "The draft and its lines are removed. Nothing posted is affected."
        );
        assert_eq!(dialog.memo, "Printer paper");
        assert_eq!(
            view.delete_command(),
            LedgerCommand::EntryDeleteDraft {
                entry_id: id.clone()
            }
        );
        demo_org.submit(view.delete_command()).unwrap();
        assert_eq!(EntryView::new(&demo_org.state(), &id), None);
    }

    #[test]
    fn reversing_needs_a_date_and_a_reason() {
        let state = demo();
        let entry = state
            .entries
            .iter()
            .find(|e| e.id.as_str() == LATEST)
            .unwrap();
        let mut form = ReverseForm::new(entry, &as_of());
        assert_eq!(form.title, "Reverse JE-0016");
        assert_eq!(
            form.description,
            "A mirror-image entry is posted on the date you choose. The original stays in the \
             books, marked reversed."
        );
        assert_eq!(form.date, "04/02/2026");
        assert_eq!(form.reason, "");
        let id = EntryId::from(LATEST);
        assert_eq!(
            form.command(&id),
            Err(ReverseErrors {
                date: None,
                reason: Some("Say why the entry is being reversed.".into()),
            })
        );
        form.date = String::new();
        form.reason = "Wrong grant".into();
        assert_eq!(
            form.command(&id).unwrap_err().date.as_deref(),
            Some("Enter the date of the reversal.")
        );
        form.date = "31/02/2026".into();
        assert_eq!(
            form.command(&id).unwrap_err().date.as_deref(),
            Some("February 2026 has only 28 days.")
        );
        form.date = "5/2/2026".into();
        form.reason = "  Wrong grant  ".into();
        assert_eq!(
            form.command(&id),
            Ok(LedgerCommand::EntryReverse {
                entry_id: id.clone(),
                date: "2026-02-05".into(),
                reason: "Wrong grant".into(),
            })
        );
    }

    #[test]
    fn a_reversal_links_both_entries() {
        let mut demo_org = Demo::new();
        let id = EntryId::from(LATEST);
        let view = demo_org.view(&id);
        let mut form = ReverseForm::new(&view.entry, &as_of());
        form.reason = "Booked to the wrong grant".into();
        let r = demo_org.submit(form.command(&id).unwrap()).unwrap();
        let reversal = created_entry(&r).unwrap();

        let original = demo_org.view(&id);
        assert_eq!(original.entry.status, EntryStatus::Reversed);
        assert_eq!(original.actions, []);
        let rev = demo_org.view(&reversal);
        assert_eq!(
            original.reversed_by,
            Some(EntryLink {
                id: reversal.clone(),
                text: format!("{} on 4 Feb 2026", rev.entry.number),
            })
        );
        let trail: Vec<_> = original
            .audit
            .iter()
            .map(|a| (a.label.as_str(), a.reason.as_deref()))
            .collect();
        assert_eq!(
            trail,
            [
                ("Saved as draft", None),
                ("Posted", None),
                ("Reversed", Some("Booked to the wrong grant")),
            ]
        );

        assert_eq!(rev.entry.source_type, SourceType::Reversal);
        assert_eq!(rev.source, "Reversal");
        assert_eq!(
            rev.reverses,
            Some(EntryLink {
                id: id.clone(),
                text: format!("JE-0016 {}", original.entry.memo),
            })
        );
        // A reversal is posted and could only be reversed by the engine's refusal.
        assert_eq!(rev.actions, [EntryAction::Reverse]);
        let err = demo_org
            .submit(LedgerCommand::EntryReverse {
                entry_id: reversal.clone(),
                date: DEMO_AS_OF.into(),
                reason: "again".into(),
            })
            .unwrap_err();
        assert_eq!(
            Failure::new(rev.failure_title(), &err).messages,
            [format!(
                "{} is itself a reversal. Post a new entry instead.",
                rev.entry.number
            )]
        );

        // The list shows both, with the counts moved.
        let counts = StatusCounts::of(&entry_rows(&demo_org.state()));
        assert_eq!((counts.posted, counts.reversed), (22, 1));
        assert_eq!(created_entry(&CommandResult::None), None);
    }
}
