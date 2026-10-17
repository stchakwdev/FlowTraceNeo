//! View models of the Spend screen's claims tab: the claims list with its status filter, the
//! new claim form with its coded lines, and the claim detail with its trail, the actions its
//! status allows, and the posted entry with the grants it charged. Plain structs, no GPUI; the
//! panel in `panels::spend` only draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/spend/claims.index.tsx`,
//! `claims.new.tsx`, `claims.$id.tsx` and `-shared.tsx`; the rules themselves stay in
//! `ledger-core` (`engine/claims.rs`), whose messages are shown as they come.

use std::collections::HashSet;

use ledger_core::engine::{NewClaim, NewCodedLine};
use ledger_core::money::parse_money;
use ledger_core::types::{
    AccountId, AccountType, Actor, ActorId, AppliesTo, ApprovalAction, ClaimId, ClaimStatus,
    CodedLine, DimensionId, DimensionKind, EntryId, ExpenseClaim, FundId, FundRestriction, GrantId,
    IsoDate, LedgerState, Nullish, Timestamp,
};
use ledger_core::{CommandError, LedgerCommand, Minor, engine};

use crate::accounts::{format_date, format_minor};
use crate::backend::BackendError;

/// The en dash the web app shows for an empty cell (`money.tsx:12`).
const DASH: &str = "\u{2013}";

/// The middle dot between parts of a line.
const DOT: &str = "\u{b7}";

/// The unrestricted fund every chart starts with (`ledger_core::coa_template`).
const UNRESTRICTED: &str = ledger_core::coa_template::UNRESTRICTED_FUND_ID;

/// One choice of the status filter (`claims.index.tsx:16-23`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimFilter {
    /// The status shown; `None` for all.
    pub status: Option<ClaimStatus>,
    /// The label.
    pub label: &'static str,
}

/// `FILTERS` (`claims.index.tsx:16-23`): All, Draft, Submitted, To reimburse, Reimbursed,
/// Rejected.
pub const CLAIM_FILTERS: [ClaimFilter; 6] = [
    ClaimFilter {
        status: None,
        label: "All",
    },
    ClaimFilter {
        status: Some(ClaimStatus::Draft),
        label: "Draft",
    },
    ClaimFilter {
        status: Some(ClaimStatus::Submitted),
        label: "Submitted",
    },
    ClaimFilter {
        status: Some(ClaimStatus::Posted),
        label: "To reimburse",
    },
    ClaimFilter {
        status: Some(ClaimStatus::Reimbursed),
        label: "Reimbursed",
    },
    ClaimFilter {
        status: Some(ClaimStatus::Rejected),
        label: "Rejected",
    },
];

/// `CLAIM_STATUS_LABEL` (`-shared.tsx:41-48`).
#[must_use]
pub fn claim_status_label(status: ClaimStatus) -> &'static str {
    match status {
        ClaimStatus::Draft => "Draft",
        ClaimStatus::Submitted => "Submitted",
        ClaimStatus::Approved => "Approved",
        ClaimStatus::Posted => "Posted, to reimburse",
        ClaimStatus::Reimbursed => "Reimbursed",
        ClaimStatus::Rejected => "Rejected",
    }
}

/// How a status tag is drawn (`-shared.tsx:58-65`, `DocStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Dashed and muted: a draft.
    Quiet,
    /// Ink outline: waiting on someone.
    Open,
    /// Filled ink: in the books, money still owed.
    Booked,
    /// Filled report blue: settled.
    Done,
    /// Dotted red: rejected.
    Rejected,
}

/// `CLAIM_TONE` (`-shared.tsx:58-65`).
#[must_use]
pub fn claim_tone(status: ClaimStatus) -> Tone {
    match status {
        ClaimStatus::Draft => Tone::Quiet,
        ClaimStatus::Submitted | ClaimStatus::Approved => Tone::Open,
        ClaimStatus::Posted => Tone::Booked,
        ClaimStatus::Reimbursed => Tone::Done,
        ClaimStatus::Rejected => Tone::Rejected,
    }
}

/// `docTotal` (`-shared.tsx:133`): the sum of the lines.
#[must_use]
pub fn doc_total(lines: &[CodedLine]) -> Minor {
    lines.iter().map(|l| l.amount).sum()
}

/// One row of the claims list.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimRow {
    /// The claim.
    pub claim: ExpenseClaim,
    /// Its total.
    pub total: Minor,
    /// `grantNames` (`claims.index.tsx:200-204`): the grants of its lines, or `No grant`.
    pub grants: String,
    /// The funds of its lines.
    pub funds: String,
}

/// The distinct values of `ids` in first-seen order.
fn distinct<'a, T: Eq + std::hash::Hash + 'a>(ids: impl IntoIterator<Item = &'a T>) -> Vec<&'a T> {
    let mut seen = HashSet::new();
    ids.into_iter().filter(|id| seen.insert(*id)).collect()
}

/// `grantNames` (`claims.index.tsx:200-204`).
fn grant_names(state: &LedgerState, claim: &ExpenseClaim) -> String {
    let ids = distinct(claim.lines.iter().filter_map(|l| l.grant_id.as_ref()));
    if ids.is_empty() {
        return "No grant".to_owned();
    }
    ids.into_iter()
        .map(|id| grant_name(state, id).unwrap_or("Grant"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The funds of a claim's lines, as `grantNames` lists grants.
fn fund_names(state: &LedgerState, claim: &ExpenseClaim) -> String {
    distinct(claim.lines.iter().map(|l| &l.fund_id))
        .into_iter()
        .map(|id| fund_name(state, id).unwrap_or("Fund"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn grant_name<'a>(state: &'a LedgerState, id: &GrantId) -> Option<&'a str> {
    state
        .grants
        .iter()
        .find(|g| &g.id == id)
        .map(|g| g.name.as_str())
}

fn fund_name<'a>(state: &'a LedgerState, id: &FundId) -> Option<&'a str> {
    state
        .funds
        .iter()
        .find(|f| &f.id == id)
        .map(|f| f.name.as_str())
}

/// The claims `filter` shows, latest claim date first, then latest number
/// (`claims.index.tsx:51-58`). Dates and numbers compare by bytes, which is what
/// `localeCompare` gives for ISO dates and `EC-` numbers.
#[must_use]
pub fn claim_rows(state: &LedgerState, filter: Option<ClaimStatus>) -> Vec<ClaimRow> {
    let mut claims: Vec<&ExpenseClaim> = state
        .claims
        .iter()
        .filter(|c| filter.is_none_or(|s| c.status == s))
        .collect();
    claims.sort_by(|a, b| {
        b.claim_date
            .cmp(&a.claim_date)
            .then_with(|| b.number.cmp(&a.number))
    });
    claims
        .into_iter()
        .map(|c| ClaimRow {
            claim: c.clone(),
            total: doc_total(&c.lines),
            grants: grant_names(state, c),
            funds: fund_names(state, c),
        })
        .collect()
}

/// Each filter with the number of claims it shows (`claims.index.tsx:42-49`).
#[must_use]
pub fn filter_counts(state: &LedgerState) -> Vec<(ClaimFilter, usize)> {
    CLAIM_FILTERS
        .iter()
        .map(|f| {
            let n = state
                .claims
                .iter()
                .filter(|c| f.status.is_none_or(|s| c.status == s))
                .count();
            (*f, n)
        })
        .collect()
}

/// A notice in place of an empty list: a title and one line (`claims.index.tsx:157-182`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The title.
    pub title: String,
    /// The line under it.
    pub body: String,
}

/// The notice when the list is empty: no claims at all, or none under `filter`.
#[must_use]
pub fn list_notice(state: &LedgerState, filter: Option<ClaimStatus>) -> Option<Notice> {
    if state.claims.is_empty() {
        return Some(Notice {
            title: "No claims yet".to_owned(),
            body: "When staff pay for work costs themselves, they claim the money back here \
                   with the receipts."
                .to_owned(),
        });
    }
    let shown = state
        .claims
        .iter()
        .any(|c| filter.is_none_or(|s| c.status == s));
    (!shown).then(|| Notice {
        title: "No claims match".to_owned(),
        body: format!(
            "No claims are {}.",
            filter.map_or("here".to_owned(), |s| claim_status_label(s).to_lowercase())
        ),
    })
}

/// `isoToDayMonthYear` (`flowtrace/src/components/ui/date-field.tsx:36-39`): `04/02/2026`.
#[must_use]
pub fn iso_to_day_month_year(date: &IsoDate) -> String {
    let s = date.as_str();
    match (s.get(0..4), s.get(5..7), s.get(8..10)) {
        (Some(y), Some(m), Some(d)) => format!("{d}/{m}/{y}"),
        _ => String::new(),
    }
}

/// The month names of the date field's messages (`date-field.tsx:12-25`).
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Days in `month` of `year`, Gregorian.
fn days_in(year: u32, month: u32) -> u32 {
    match month {
        2 if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `parseDayMonthYear` (`date-field.tsx:50-80`): `None` for a blank field, otherwise the date
/// or the message to show. Reads `d/m/yyyy` with `/`, `.` or `-` between the parts.
#[must_use]
pub fn parse_day_month_year(text: &str) -> Option<Result<IsoDate, String>> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let parts: Vec<&str> = t.split(['/', '.', '-']).collect();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let [d, m, y] = parts.as_slice() else {
        return Some(Err(format_message()));
    };
    if !(digits(d) && d.len() <= 2 && digits(m) && m.len() <= 2 && digits(y)) {
        return Some(Err(format_message()));
    }
    if y.len() != 4 {
        return Some(Err("Write the year in full, for example 2026.".to_owned()));
    }
    let (Ok(day), Ok(month), Ok(year)) = (d.parse::<u32>(), m.parse::<u32>(), y.parse::<u32>())
    else {
        return Some(Err(format_message()));
    };
    if !(1..=12).contains(&month) {
        return Some(Err(format!(
            "There is no month {month}. Put the day first, then the month: dd/mm/yyyy."
        )));
    }
    let days = days_in(year, month);
    if day < 1 {
        return Some(Err("There is no day 0. Enter a day from 1.".to_owned()));
    }
    if day > days {
        return Some(Err(format!(
            "{} {year} has only {days} days.",
            MONTHS[month as usize - 1]
        )));
    }
    Some(IsoDate::parse(&format!("{year:04}-{month:02}-{day:02}")).map_err(|_| format_message()))
}

fn format_message() -> String {
    "Enter the date as day/month/year, for example 18/01/2026.".to_owned()
}

/// Active, non-header accounts passing `keep`, in code order, as `(id, "code · name")`.
fn account_choices(
    state: &LedgerState,
    keep: impl Fn(&ledger_core::types::Account) -> bool,
) -> Vec<(AccountId, String)> {
    let mut accounts: Vec<_> = state
        .accounts
        .iter()
        .filter(|a| a.is_active && !a.is_group && keep(a))
        .collect();
    accounts.sort_by(|a, b| a.code.cmp(&b.code));
    accounts
        .into_iter()
        .map(|a| (a.id.clone(), format!("{} {DOT} {}", a.code, a.name)))
        .collect()
}

/// `AccountSelect types={['expense']}` (`flowtrace/src/components/ledger/pickers.tsx:25-44`):
/// active, non-header expense accounts in code order, as `(id, "code · name")`.
#[must_use]
pub fn cost_accounts(state: &LedgerState) -> Vec<(AccountId, String)> {
    account_choices(state, |a| a.account_type == AccountType::Expense)
}

/// `AccountSelect cashOnly` (`pickers.tsx:25-44`): active, non-header cash accounts in type
/// order, then code order.
#[must_use]
pub fn cash_accounts(state: &LedgerState) -> Vec<(AccountId, String)> {
    account_choices(state, |a| a.is_cash)
}

/// `FundSelect` (`pickers.tsx:46-54`): every fund in the books' order.
#[must_use]
pub fn fund_choices(state: &LedgerState) -> Vec<(FundId, String)> {
    state
        .funds
        .iter()
        .map(|f| (f.id.clone(), f.name.clone()))
        .collect()
}

/// `GrantSelect fundId` (`pickers.tsx:57-69`) without its `No grant` choice: the grants on
/// `fund`, or every grant when no fund is chosen.
#[must_use]
pub fn grant_choices(state: &LedgerState, fund: Option<&FundId>) -> Vec<(GrantId, String)> {
    state
        .grants
        .iter()
        .filter(|g| fund.is_none_or(|f| &g.fund_id == f))
        .map(|g| (g.id.clone(), g.name.clone()))
        .collect()
}

/// The dimension values of `kind` (`-shared.tsx:381-388`), for the project and place pickers.
#[must_use]
pub fn dimension_choices(state: &LedgerState, kind: DimensionKind) -> Vec<(DimensionId, String)> {
    state
        .dimensions
        .iter()
        .filter(|d| d.kind == kind)
        .map(|d| (d.id.clone(), d.name.clone()))
        .collect()
}

/// One coded line as typed (`LineDraft`, `-shared.tsx:276-288`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineDraft {
    /// What it was for.
    pub description: String,
    /// Amount as typed.
    pub amount: String,
    /// Kind of cost.
    pub account: Option<AccountId>,
    /// The fund that pays.
    pub fund: Option<FundId>,
    /// The grant it comes from.
    pub grant: Option<GrantId>,
    /// Project.
    pub project: Option<DimensionId>,
    /// Place.
    pub location: Option<DimensionId>,
}

impl LineDraft {
    /// `newLine` (`-shared.tsx:291-307`): blank, on the unrestricted fund when the books have
    /// it, else the first fund.
    #[must_use]
    pub fn new(state: &LedgerState) -> Self {
        let fund = state
            .funds
            .iter()
            .find(|f| f.id.as_str() == UNRESTRICTED)
            .or_else(|| state.funds.first())
            .map(|f| f.id.clone());
        Self {
            fund,
            ..Self::default()
        }
    }
}

/// A line field that can carry an error (`LineField`, `-shared.tsx:309`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineField {
    /// What it was for.
    Description,
    /// Amount.
    Amount,
    /// Kind of cost.
    Account,
    /// Fund.
    Fund,
    /// Grant.
    Grant,
}

impl LineField {
    /// `LINE_FIELD_LABEL` (`-shared.tsx:313-319`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::Amount => "amount",
            Self::Account => "account",
            Self::Fund => "fund",
            Self::Grant => "grant",
        }
    }
}

/// What the new claim form found wrong before sending (`claims.new.tsx:68-88`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormErrors {
    /// Under the date.
    pub date: Option<String>,
    /// Under the purpose.
    pub purpose: Option<String>,
    /// Under line fields: line index, field, message, in the order found.
    pub lines: Vec<(usize, LineField, String)>,
}

impl FormErrors {
    /// The message under one line field.
    #[must_use]
    pub fn line(&self, index: usize, field: LineField) -> Option<&str> {
        self.lines
            .iter()
            .find(|(i, f, _)| *i == index && *f == field)
            .map(|(_, _, m)| m.as_str())
    }

    fn is_empty(&self) -> bool {
        self.date.is_none() && self.purpose.is_none() && self.lines.is_empty()
    }

    /// `ErrorSummary` (`-shared.tsx:552-597`): `Fix this before saving` or `Fix these before
    /// saving`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        if self.summary().len() == 1 {
            "Fix this before saving"
        } else {
            "Fix these before saving"
        }
    }

    /// The summary lines, `Label: message`, date and purpose first (`claims.new.tsx:81-86`).
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(m) = &self.date {
            out.push(format!("Date: {m}"));
        }
        if let Some(m) = &self.purpose {
            out.push(format!("Purpose: {m}"));
        }
        out.extend(
            self.lines
                .iter()
                .map(|(i, f, m)| format!("Line {} {}: {m}", i + 1, f.label())),
        );
        out
    }
}

/// The new claim form (`claims.new.tsx`): date spent, purpose and the coded lines. The
/// claimant is whoever sends it (`engine/claims.rs`, `create_claim`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimForm {
    /// Date spent as typed, `dd/mm/yyyy`.
    pub date: String,
    /// Purpose as typed.
    pub purpose: String,
    /// The lines; at least one.
    pub lines: Vec<LineDraft>,
    /// Names of the receipts attached and in the proof store (`crate::proofs::AttachList`),
    /// sent as `receipt_names` (`claims.new.tsx:119`).
    pub receipts: Vec<String>,
}

impl ClaimForm {
    /// The form on `as_of` with one blank line (`claims.new.tsx:50-63`).
    #[must_use]
    pub fn new(state: &LedgerState, as_of: &IsoDate) -> Self {
        Self {
            date: iso_to_day_month_year(as_of),
            purpose: String::new(),
            lines: vec![LineDraft::new(state)],
            receipts: Vec::new(),
        }
    }

    /// `Add another line` (`-shared.tsx:516-534`): a blank line carrying the last line's
    /// account, fund, grant, project and place.
    pub fn add_line(&mut self, state: &LedgerState) {
        let line = match self.lines.last() {
            Some(last) => LineDraft {
                description: String::new(),
                amount: String::new(),
                ..last.clone()
            },
            None => LineDraft::new(state),
        };
        self.lines.push(line);
    }

    /// Removes line `index` when more than one is left (`-shared.tsx:404-415`).
    pub fn remove_line(&mut self, index: usize) -> bool {
        if self.lines.len() > 1 && index < self.lines.len() {
            self.lines.remove(index);
            true
        } else {
            false
        }
    }

    /// `withFund` (`-shared.tsx:356-361`): a new fund drops a grant from another fund and picks
    /// the only grant when there is one.
    pub fn set_fund(&mut self, state: &LedgerState, index: usize, fund: Option<FundId>) {
        let Some(line) = self.lines.get_mut(index) else {
            return;
        };
        let grants: Vec<&GrantId> = fund.as_ref().map_or_else(Vec::new, |f| {
            state
                .grants
                .iter()
                .filter(|g| &g.fund_id == f)
                .map(|g| &g.id)
                .collect()
        });
        let keep = line.grant.as_ref().is_some_and(|g| grants.contains(&g));
        if !keep {
            line.grant = match grants.as_slice() {
                [only] => Some((*only).clone()),
                _ => None,
            };
        }
        line.fund = fund;
    }

    /// `linesTotal` (`-shared.tsx:351-353`): the amounts that parse, negatives as zero.
    #[must_use]
    pub fn total(&self) -> Minor {
        self.lines
            .iter()
            .map(|l| parse_money(&l.amount).unwrap_or(0).max(0))
            .sum()
    }

    /// `validateLines` (`-shared.tsx:322-345`).
    fn line_errors(&self, state: &LedgerState) -> Vec<(usize, LineField, String)> {
        let mut out = Vec::new();
        for (n, l) in self.lines.iter().enumerate() {
            let mut add = |f, m: &str| out.push((n, f, m.to_owned()));
            if l.description.trim().is_empty() {
                add(LineField::Description, "Say what this was for.");
            }
            if l.amount.trim().is_empty() {
                add(LineField::Amount, "Enter the amount.");
            } else {
                match parse_money(&l.amount) {
                    None => add(LineField::Amount, "Enter a number, like 1250 or 1,250.50."),
                    Some(a) if a <= 0 => {
                        add(LineField::Amount, "The amount must be more than zero.");
                    }
                    Some(_) => {}
                }
            }
            if l.account.is_none() {
                add(LineField::Account, "Choose what kind of cost this is.");
            }
            match l
                .fund
                .as_ref()
                .and_then(|id| state.funds.iter().find(|f| &f.id == id))
            {
                None => add(LineField::Fund, "Choose the fund that pays for this."),
                Some(f) if f.restriction != FundRestriction::Unrestricted && l.grant.is_none() => {
                    add(
                        LineField::Grant,
                        "Restricted money needs the grant it comes from.",
                    );
                }
                Some(_) => {}
            }
        }
        out
    }

    /// The `claim.create` command, submitting the claim when `submit` is set, or what to fix
    /// first (`claims.new.tsx:68-113`). The claim is in the base currency at rate 1 with no
    /// receipts; the id is left to the engine's env (the web draws `crypto.randomUUID()`).
    ///
    /// # Errors
    ///
    /// [`FormErrors`] for a missing or late date, a blank purpose or a line that is not
    /// complete (`validateLines`, `-shared.tsx:322-345`).
    pub fn command(
        &self,
        state: &LedgerState,
        as_of: &IsoDate,
        submit: bool,
    ) -> Result<LedgerCommand, FormErrors> {
        let mut errors = FormErrors::default();
        let date = match parse_day_month_year(&self.date) {
            None => {
                errors.date = Some("Enter the date you spent the money.".to_owned());
                None
            }
            Some(Err(m)) => {
                errors.date = Some(m);
                None
            }
            Some(Ok(d)) if d > *as_of => {
                errors.date = Some("The date cannot be after today.".to_owned());
                None
            }
            Some(Ok(d)) => Some(d),
        };
        if self.purpose.trim().is_empty() {
            errors.purpose = Some(
                "Say what the trip or task was, e.g. Borehole site inspection, Lodwar.".to_owned(),
            );
        }
        errors.lines = self.line_errors(state);
        let (Some(date), true) = (date, errors.is_empty()) else {
            return Err(errors);
        };
        let lines = self
            .lines
            .iter()
            .filter_map(|l| {
                // validateLines above guarantees every field; toCodedLines (-shared.tsx:340-349).
                let (Some(account_id), Some(fund_id)) = (l.account.clone(), l.fund.clone()) else {
                    return None;
                };
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "the engine takes amounts as JS numbers; minor units are safe integers"
                )]
                let amount = parse_money(&l.amount).unwrap_or(0) as f64;
                Some(NewCodedLine {
                    description: l.description.trim().to_owned(),
                    account_id,
                    amount,
                    fund_id,
                    grant_id: l.grant.clone(),
                    dimension_ids: [&l.project, &l.location]
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect(),
                })
            })
            .collect();
        Ok(LedgerCommand::ClaimCreate {
            input: NewClaim {
                id: None,
                claim_date: date,
                currency: state.settings.base_currency.clone(),
                fx_rate: 1.0,
                purpose: self.purpose.trim().to_owned(),
                lines,
                receipt_names: self.receipts.clone(),
                captured_offline: false,
                submit: Some(submit),
            },
        })
    }
}

/// What a claim's status lets the viewer do (`claims.$id.tsx:177-232`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimAction {
    /// The claimant submits a draft.
    Submit,
    /// An approver approves and posts a submitted claim.
    Approve,
    /// An approver rejects a submitted claim with a note.
    Reject,
    /// Pay back a posted claim.
    Reimburse,
}

impl ClaimAction {
    /// The button label (`claims.$id.tsx:182-213`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Submit => "Submit for approval",
            Self::Approve => "Approve and post",
            Self::Reject => "Reject",
            Self::Reimburse => "Record the reimbursement",
        }
    }

    /// The notice once it is done (`claims.$id.tsx:127-133`, 377-397, 428).
    #[must_use]
    pub fn done_notice(self) -> &'static str {
        match self {
            Self::Submit => "Submitted. A manager can now approve it.",
            Self::Approve => "Approved and posted to the books.",
            Self::Reject => "Rejected. The claimant can see your reason.",
            Self::Reimburse => "Reimbursement recorded.",
        }
    }
}

/// One line of the claim as shown (`CodedLines`, `-shared.tsx:175-262`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineView {
    /// Description, or `No description`.
    pub description: String,
    /// `code name`, or `Unknown account`.
    pub account: String,
    /// The fund's name, or `Fund`.
    pub fund: String,
    /// The grant's name, or `No grant`.
    pub grant: String,
    /// Project and place names, or an en dash.
    pub dimensions: String,
    /// Amount.
    pub amount: Minor,
}

/// One step of the approval trail (`claims.$id.tsx:265-300`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrailStep {
    /// `STEP_LABEL` (`claims.$id.tsx:40-46`).
    pub label: &'static str,
    /// `personName` of who did it.
    pub who: String,
    /// When, `4 Feb 2026, 09:00`.
    pub at: String,
    /// The note shown in quotes (not for posted and reimbursed).
    pub note: Option<String>,
    /// The payment reference of a reimbursed step.
    pub reference: Option<String>,
    /// A rejection is drawn in red.
    pub rejected: bool,
}

/// `STEP_LABEL` (`claims.$id.tsx:40-46`).
fn step_label(action: ApprovalAction) -> &'static str {
    match action {
        ApprovalAction::Submitted => "Submitted for approval",
        ApprovalAction::Approved => "Approved",
        ApprovalAction::Rejected => "Rejected",
        ApprovalAction::Posted => "Posted to the books",
        ApprovalAction::Reimbursed => "Paid back",
    }
}

/// One row of "In the books" (`claims.$id.tsx:303-330`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookRow {
    /// What it is.
    pub label: String,
    /// The entry.
    pub entry_id: EntryId,
    /// Its number, or `Journal entry` (`EntryLink`, `-shared.tsx:151-171`).
    pub number: String,
}

/// One grant the posted entry charged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantHit {
    /// The grant.
    pub grant_id: GrantId,
    /// Its name.
    pub name: String,
    /// Net expense of the entry on it, in base currency.
    pub charged: Minor,
    /// All posted spend on it now (`grantSpend`, `engine.ts:447-464`).
    pub spend: Minor,
    /// The award.
    pub award: Minor,
}

/// The accrual entry an approval posted and the grants it charged, so the screen shows that
/// the posting reached the right grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    /// The entry.
    pub entry_id: EntryId,
    /// Its number.
    pub number: String,
    /// The grants its expense lines charged, in line order.
    pub grants: Vec<GrantHit>,
}

/// The posted entry `entry_id` and the grants its expense lines charge: debit less credit in
/// base currency per grant, as `grantSpend` counts it (`engine.ts:447-464`).
fn posting(state: &LedgerState, entry_id: &EntryId) -> Option<Posting> {
    let entry = state.entries.iter().find(|e| &e.id == entry_id)?;
    let mut grants: Vec<GrantHit> = Vec::new();
    for l in state.lines.iter().filter(|l| &l.entry_id == entry_id) {
        let (Some(grant_id), true) = (
            l.grant_id.as_ref(),
            state
                .accounts
                .iter()
                .any(|a| a.id == l.account_id && a.account_type == AccountType::Expense),
        ) else {
            continue;
        };
        if let Some(hit) = grants.iter_mut().find(|g| &g.grant_id == grant_id) {
            hit.charged += l.base_amount;
        } else {
            let grant = state.grants.iter().find(|g| &g.id == grant_id);
            grants.push(GrantHit {
                grant_id: grant_id.clone(),
                name: grant.map_or_else(|| "Grant".to_owned(), |g| g.name.clone()),
                charged: l.base_amount,
                spend: state.grant_spend(grant_id, None),
                award: grant.map_or(0, |g| g.award),
            });
        }
    }
    Some(Posting {
        entry_id: entry.id.clone(),
        number: entry.number.clone(),
        grants,
    })
}

/// One claim as its detail screen shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimDetail {
    /// The claim.
    pub claim: ExpenseClaim,
    /// Its total.
    pub total: Minor,
    /// Still owed: `outstanding` while posted, else zero (`claims.$id.tsx:88`).
    pub outstanding: Minor,
    /// Its lines.
    pub lines: Vec<LineView>,
    /// Its approval trail.
    pub trail: Vec<TrailStep>,
    /// "What happens next" (`claims.$id.tsx:177-232`).
    pub next: String,
    /// The buttons under it.
    pub actions: Vec<ClaimAction>,
    /// "In the books": the posted entry and the payments.
    pub books: Vec<BookRow>,
    /// The accrual entry and the grants it hit, once approved.
    pub posting: Option<Posting>,
}

/// `currency 1,234.56`, as `<Money currency>` writes it.
fn money(currency: &str, minor: Minor) -> String {
    format!("{currency} {}", format_minor(minor))
}

/// The next step and its actions (`claims.$id.tsx:177-232`).
fn next_step(
    c: &ExpenseClaim,
    outstanding: Minor,
    actor: Option<&Actor>,
) -> (String, Vec<ClaimAction>) {
    let is_claimant = actor.is_some_and(|a| a.id == c.claimant);
    let can_approve = actor.is_some_and(|a| a.can_approve);
    match c.status {
        ClaimStatus::Draft if is_claimant => (
            "This is a draft. Submit it when the lines and receipts are complete.".to_owned(),
            vec![ClaimAction::Submit],
        ),
        ClaimStatus::Draft => (
            format!(
                "This is {}\u{2019}s draft. Only they can submit it.",
                c.claimant_name
            ),
            vec![],
        ),
        ClaimStatus::Submitted if is_claimant => (
            "You made this claim, so someone else has to approve it. The books do not let \
             anyone approve their own spending."
                .to_owned(),
            vec![],
        ),
        ClaimStatus::Submitted if !can_approve => (
            "Waiting for a manager with approval rights.".to_owned(),
            vec![],
        ),
        ClaimStatus::Submitted => (
            format!(
                "Approving posts the claim to the books straight away: the costs land on their \
                 grants and budget lines, and the money is owed to {}.",
                c.claimant_name
            ),
            vec![ClaimAction::Approve, ClaimAction::Reject],
        ),
        ClaimStatus::Posted => (
            format!(
                "Approved and posted. {} is owed to {}.",
                money(&c.currency, outstanding),
                c.claimant_name
            ),
            vec![ClaimAction::Reimburse],
        ),
        ClaimStatus::Reimbursed => ("Paid back in full. Nothing more to do.".to_owned(), vec![]),
        ClaimStatus::Rejected => (
            "Rejected. The reason is in the approval trail below; the claimant can make a new \
             claim."
                .to_owned(),
            vec![],
        ),
        ClaimStatus::Approved => ("Approved, waiting to be posted.".to_owned(), vec![]),
    }
}

/// The lines of `c` as `CodedLines` shows them (`-shared.tsx:175-262`).
fn line_views(state: &LedgerState, c: &ExpenseClaim) -> Vec<LineView> {
    c.lines
        .iter()
        .map(|l| {
            let account = state.accounts.iter().find(|a| a.id == l.account_id);
            let dims: Vec<&str> = l
                .dimension_ids
                .iter()
                .map(|d| {
                    state
                        .dimensions
                        .iter()
                        .find(|x| &x.id == d)
                        .map_or(d.as_str(), |x| x.name.as_str())
                })
                .collect();
            LineView {
                description: if l.description.is_empty() {
                    "No description".to_owned()
                } else {
                    l.description.clone()
                },
                account: account.map_or_else(
                    || "Unknown account".to_owned(),
                    |a| format!("{} {}", a.code, a.name),
                ),
                fund: fund_name(state, &l.fund_id).unwrap_or("Fund").to_owned(),
                grant: l
                    .grant_id
                    .as_ref()
                    .and_then(|g| grant_name(state, g))
                    .unwrap_or("No grant")
                    .to_owned(),
                dimensions: if dims.is_empty() {
                    DASH.to_owned()
                } else {
                    dims.join(", ")
                },
                amount: l.amount,
            }
        })
        .collect()
}

/// The approval trail of `c` (`claims.$id.tsx:265-300`).
fn trail_steps(c: &ExpenseClaim, actor: Option<&Actor>) -> Vec<TrailStep> {
    let claimant = (&c.claimant, c.claimant_name.as_str());
    c.approvals
        .iter()
        .map(|s| {
            let quoted = !matches!(
                s.action,
                ApprovalAction::Posted | ApprovalAction::Reimbursed
            );
            TrailStep {
                label: step_label(s.action),
                who: person_name(&s.by, actor, claimant),
                at: format_date_time(&s.at),
                note: s.note.clone().filter(|_| quoted),
                reference: s
                    .note
                    .clone()
                    .filter(|_| s.action == ApprovalAction::Reimbursed),
                rejected: s.action == ApprovalAction::Rejected,
            }
        })
        .collect()
}

/// "In the books" of `c`: the posted entry, then the payments (`claims.$id.tsx:303-330`).
fn book_rows(state: &LedgerState, c: &ExpenseClaim) -> Vec<BookRow> {
    let number_of = |entry: &EntryId| {
        state
            .entries
            .iter()
            .find(|e| &e.id == entry)
            .map_or_else(|| "Journal entry".to_owned(), |e| e.number.clone())
    };
    let mut books = Vec::new();
    if let Some(entry) = &c.entry_id {
        books.push(BookRow {
            label: "Claim posted: costs and the amount owed".to_owned(),
            entry_id: entry.clone(),
            number: number_of(entry),
        });
    }
    books.extend(
        state
            .payments
            .iter()
            .filter(|p| p.applies_to == AppliesTo::ExpenseClaim(c.id.clone()))
            .map(|p| {
                let mut label = format!(
                    "Paid back {} {DOT} {}",
                    format_date(&p.date),
                    money(&p.currency, p.amount)
                );
                if let Some(r) = p.reference.as_deref().filter(|r| !r.is_empty()) {
                    label.push(' ');
                    label.push_str(r);
                }
                BookRow {
                    label,
                    entry_id: p.entry_id.clone(),
                    number: number_of(&p.entry_id),
                }
            }),
    );
    books
}

impl ClaimDetail {
    /// The detail of claim `id` as `actor` sees it, or `None` when it is not in the books.
    #[must_use]
    pub fn new(state: &LedgerState, id: &ClaimId, actor: Option<&Actor>) -> Option<Self> {
        let c = state.claims.iter().find(|c| &c.id == id)?;
        let total = doc_total(&c.lines);
        let outstanding = if c.status == ClaimStatus::Posted {
            state
                .outstanding(&AppliesTo::ExpenseClaim(c.id.clone()))
                .unwrap_or(0)
        } else {
            0
        };
        let lines = line_views(state, c);
        let trail = trail_steps(c, actor);
        let books = book_rows(state, c);
        let (next, actions) = next_step(c, outstanding, actor);
        Some(Self {
            claim: c.clone(),
            total,
            outstanding,
            lines,
            trail,
            next,
            actions,
            books,
            posting: c.entry_id.as_ref().and_then(|e| posting(state, e)),
        })
    }

    /// The lede: `EC-0002 · Achieng Odhiambo · spent 3 Feb 2026` (`claims.$id.tsx:57-63`).
    #[must_use]
    pub fn lede(&self) -> String {
        let c = &self.claim;
        format!(
            "{} {DOT} {} {DOT} spent {}",
            c.number,
            c.claimant_name,
            format_date(&c.claim_date)
        )
    }

    /// The `claim.submit` command.
    #[must_use]
    pub fn submit_command(&self) -> LedgerCommand {
        LedgerCommand::ClaimSubmit {
            claim_id: self.claim.id.clone(),
        }
    }

    /// The `claim.approve` command with the note trimmed (none when blank) and an override
    /// reason (`claims.$id.tsx:368-375`).
    #[must_use]
    pub fn approve_command(&self, note: &str, override_reason: Option<&str>) -> LedgerCommand {
        let note = note.trim();
        LedgerCommand::ClaimApprove {
            claim_id: self.claim.id.clone(),
            note: (!note.is_empty()).then(|| note.to_owned()),
            override_reason: override_reason.map(str::to_owned),
        }
    }

    /// The `claim.reject` command (`claims.$id.tsx:389-396`).
    ///
    /// # Errors
    ///
    /// The message under the reason when it is blank.
    pub fn reject_command(&self, note: &str) -> Result<LedgerCommand, String> {
        let note = note.trim();
        if note.is_empty() {
            return Err("Tell the claimant why, so they can put it right.".to_owned());
        }
        Ok(LedgerCommand::ClaimReject {
            claim_id: self.claim.id.clone(),
            note: note.to_owned(),
        })
    }

    /// The confirmation of `action`, `None` for a submit, which needs none
    /// (`claims.$id.tsx:356-445`).
    #[must_use]
    pub fn dialog(&self, action: ClaimAction) -> Option<Dialog> {
        let c = &self.claim;
        let (title, description, confirm) = match action {
            ClaimAction::Submit => return None,
            ClaimAction::Approve => (
                "Approve this claim",
                format!(
                    "{} {DOT} {} {DOT} {}",
                    c.number,
                    c.claimant_name,
                    money(&c.currency, self.total)
                ),
                "Approve and post",
            ),
            ClaimAction::Reject => (
                "Reject this claim",
                "The claimant sees your reason. Nothing is posted to the books.".to_owned(),
                "Reject claim",
            ),
            ClaimAction::Reimburse => (
                "Record the reimbursement",
                format!(
                    "Pays {} the full {} still owed.",
                    c.claimant_name,
                    money(&c.currency, self.outstanding)
                ),
                "Record payment",
            ),
        };
        Some(Dialog {
            action,
            title: title.to_owned(),
            description,
            confirm,
        })
    }
}

/// A confirmation box (`CommandDialog`, `-shared.tsx:604-700`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    /// The action it confirms.
    pub action: ClaimAction,
    /// Title.
    pub title: String,
    /// The line under it.
    pub description: String,
    /// The confirm button.
    pub confirm: &'static str,
}

impl Dialog {
    /// The confirm button once the ledger flagged an overridable issue: `Override and approve
    /// and post` (`-shared.tsx:671`).
    #[must_use]
    pub fn override_label(&self) -> String {
        format!("Override and {}", self.confirm.to_lowercase())
    }
}

/// What the reimbursement dialog found wrong (`claims.$id.tsx:410-418`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReimburseErrors {
    /// Under the date.
    pub date: Option<String>,
    /// Under the account.
    pub cash: Option<String>,
}

/// The reimbursement dialog's fields (`claims.$id.tsx:107-110`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReimburseForm {
    /// Date paid as typed, `dd/mm/yyyy`.
    pub date: String,
    /// Paid from.
    pub cash_account: Option<AccountId>,
    /// Reference as typed.
    pub reference: String,
}

impl ReimburseForm {
    /// The form on `as_of`.
    #[must_use]
    pub fn new(as_of: &IsoDate) -> Self {
        Self {
            date: iso_to_day_month_year(as_of),
            cash_account: None,
            reference: String::new(),
        }
    }

    /// The `claim.reimburse` command (`claims.$id.tsx:410-428`).
    ///
    /// # Errors
    ///
    /// [`ReimburseErrors`] for a missing date or account.
    pub fn command(&self, claim: &ClaimId) -> Result<LedgerCommand, ReimburseErrors> {
        let mut errors = ReimburseErrors::default();
        let date = match parse_day_month_year(&self.date) {
            None => {
                errors.date = Some("Enter the date the money was sent.".to_owned());
                None
            }
            Some(Err(m)) => {
                errors.date = Some(m);
                None
            }
            Some(Ok(d)) => Some(d),
        };
        if self.cash_account.is_none() {
            errors.cash = Some("Choose the account the money left from.".to_owned());
        }
        match (date, self.cash_account.clone()) {
            (Some(date), Some(cash_account_id)) => {
                let reference = self.reference.trim();
                Ok(LedgerCommand::ClaimReimburse {
                    claim_id: claim.clone(),
                    date: date.as_str().to_owned(),
                    cash_account_id,
                    reference: if reference.is_empty() {
                        Nullish::Null
                    } else {
                        Nullish::Value(reference.to_owned())
                    },
                })
            }
            _ => Err(errors),
        }
    }
}

/// `hasOverridable` (`flowtrace/src/components/ledger/issues.ts`): a ledger refusal whose
/// every issue an override reason lets through.
#[must_use]
pub fn has_overridable(error: &BackendError) -> bool {
    match error {
        BackendError::Command(CommandError::Engine(engine::Error::Ledger(e))) => {
            !e.issues().is_empty() && e.issues().iter().all(|i| i.overridable == Some(true))
        }
        _ => false,
    }
}

/// `personName` (`-shared.tsx:139-147`): the viewer as `Name (you)`, the claimant by name,
/// `system` as FlowTrace, anyone else by id.
#[must_use]
pub fn person_name(id: &ActorId, actor: Option<&Actor>, claimant: (&ActorId, &str)) -> String {
    match actor {
        Some(a) if &a.id == id => format!("{} (you)", a.name),
        _ if claimant.0 == id => claimant.1.to_owned(),
        _ if id.as_str() == "system" => "FlowTrace".to_owned(),
        _ => id.as_str().to_owned(),
    }
}

/// `formatDateTime` (`flowtrace/src/lib/format.ts:31-39`): `4 Feb 2026, 09:00`. The web shows
/// the viewer's local time; the desktop shows UTC so the figures do not move with the machine.
#[must_use]
pub fn format_date_time(at: &Timestamp) -> String {
    let s = at.as_str();
    let date = IsoDate::parse(s.get(..10).unwrap_or_default())
        .map(|d| format_date(&d))
        .unwrap_or_default();
    format!("{date}, {}", s.get(11..16).unwrap_or_default())
}

/// `formatMinor(v, { decimals: 0 })` (`money.tsx:8-21`): whole units with separators, half
/// away from zero as `Intl.NumberFormat` rounds, negatives in parentheses, zero as an en dash.
#[must_use]
pub fn format_whole(minor: Minor) -> String {
    if minor == 0 {
        return DASH.to_owned();
    }
    let whole = (minor.unsigned_abs() + 50) / 100 * 100;
    let text = format_minor(Minor::try_from(whole).unwrap_or(Minor::MAX));
    let text = text.trim_end_matches(".00").to_owned();
    if minor < 0 { format!("({text})") } else { text }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::accounts::Failure;
    use crate::backend::{InProcessBackend, LedgerBackend as _, demo_env_after};
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger, demo_actor};
    use ledger_core::engine::{NewClaim, NewCodedLine};
    use ledger_core::types::{Nullish, OrganizationId};
    use ledger_core::{CommandResult, Ledger, apply_command};

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

    fn id(s: &str) -> ClaimId {
        ClaimId::from(s)
    }

    /// EC-0002, submitted by Achieng Odhiambo, on the Kisima WASH grant.
    const SUBMITTED: &str = "demo-004x";
    /// EC-0001, Kiprop Mutai's, approved, posted and paid back.
    const REIMBURSED: &str = "demo-004g";
    const KISIMA: &str = "grant-kisima-wash";

    /// The demo organisation of a fresh in-process backend.
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

        fn detail(&self, claim: &ClaimId) -> ClaimDetail {
            ClaimDetail::new(&self.state(), claim, Some(&demo_actor())).unwrap()
        }
    }

    fn created(result: &CommandResult) -> ClaimId {
        match result {
            CommandResult::Id { id } => ClaimId::from(id.as_str()),
            other => panic!("no id: {other:?}"),
        }
    }

    /// A complete form: one line of fuel on the Kisima WASH grant.
    fn filled(state: &LedgerState) -> ClaimForm {
        let mut form = ClaimForm::new(state, &as_of());
        form.purpose = " Borehole follow-up, Lodwar ".into();
        let line = &mut form.lines[0];
        line.description = " Fuel ".into();
        line.amount = "1,250.50".into();
        line.account = Some(AccountId::from("acc-5100"));
        form.set_fund(state, 0, Some(FundId::from("fund-wash")));
        form.lines[0].project = Some(DimensionId::from("dim-a35"));
        form.lines[0].location = Some(DimensionId::from("dim-loc-turkana"));
        form
    }

    #[test]
    fn statuses_read_as_the_web_screen() {
        use ClaimStatus as S;
        let labels: Vec<_> = [
            S::Draft,
            S::Submitted,
            S::Approved,
            S::Posted,
            S::Reimbursed,
            S::Rejected,
        ]
        .into_iter()
        .map(|s| (claim_status_label(s), claim_tone(s)))
        .collect();
        assert_eq!(
            labels,
            [
                ("Draft", Tone::Quiet),
                ("Submitted", Tone::Open),
                ("Approved", Tone::Open),
                ("Posted, to reimburse", Tone::Booked),
                ("Reimbursed", Tone::Done),
                ("Rejected", Tone::Rejected),
            ]
        );
        let filters: Vec<_> = CLAIM_FILTERS.iter().map(|f| f.label).collect();
        assert_eq!(
            filters,
            [
                "All",
                "Draft",
                "Submitted",
                "To reimburse",
                "Reimbursed",
                "Rejected"
            ]
        );
    }

    #[test]
    fn the_list_runs_latest_first_with_grants_funds_and_totals() {
        let state = demo();
        let rows = claim_rows(&state, None);
        let numbers: Vec<_> = rows.iter().map(|r| r.claim.number.as_str()).collect();
        assert_eq!(numbers, ["EC-0002", "EC-0001"]);
        let ec2 = &rows[0];
        assert_eq!(ec2.claim.claimant_name, "Achieng Odhiambo");
        assert_eq!(ec2.total, 1_850_000);
        assert_eq!(ec2.total, doc_total(&ec2.claim.lines));
        assert_eq!(ec2.grants, "Kisima WASH: Turkana boreholes");
        assert_eq!(ec2.funds, "Water and sanitation (restricted)");
        assert_eq!(ec2.claim.status, ClaimStatus::Submitted);
        assert_eq!(rows[1].total, 1_230_000);
        assert_eq!(rows[1].grants, "Northlight maternal health 2026");
    }

    #[test]
    fn ties_on_the_date_fall_back_to_the_latest_number() {
        let mut state = demo();
        let date = state.claims[1].claim_date.clone();
        state.claims[0].claim_date = date;
        let rows = claim_rows(&state, None);
        let numbers: Vec<_> = rows.iter().map(|r| r.claim.number.as_str()).collect();
        assert_eq!(numbers, ["EC-0002", "EC-0001"]);
        state.claims[0].lines[0].grant_id = None;
        state.claims[0].lines[1].grant_id = None;
        let no_grant = claim_rows(&state, None);
        assert_eq!(no_grant[1].grants, "No grant");
    }

    #[test]
    fn the_filter_counts_and_narrows_the_list() {
        let state = demo();
        let counts: Vec<_> = filter_counts(&state)
            .into_iter()
            .map(|(f, n)| (f.label, n))
            .collect();
        assert_eq!(
            counts,
            [
                ("All", 2),
                ("Draft", 0),
                ("Submitted", 1),
                ("To reimburse", 0),
                ("Reimbursed", 1),
                ("Rejected", 0)
            ]
        );
        let submitted = claim_rows(&state, Some(ClaimStatus::Submitted));
        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].claim.number, "EC-0002");
        assert_eq!(list_notice(&state, None), None);
        assert_eq!(list_notice(&state, Some(ClaimStatus::Submitted)), None);
        assert_eq!(
            list_notice(&state, Some(ClaimStatus::Posted)),
            Some(Notice {
                title: "No claims match".into(),
                body: "No claims are posted, to reimburse.".into(),
            })
        );
        let mut empty = state.clone();
        empty.claims.clear();
        assert_eq!(
            list_notice(&empty, Some(ClaimStatus::Draft)),
            Some(Notice {
                title: "No claims yet".into(),
                body: "When staff pay for work costs themselves, they claim the money back here \
                       with the receipts."
                    .into(),
            })
        );
    }

    #[test]
    fn dates_are_typed_day_first() {
        assert_eq!(iso_to_day_month_year(&as_of()), "04/02/2026");
        let ok = |s: &str| parse_day_month_year(s).unwrap().unwrap();
        let err = |s: &str| parse_day_month_year(s).unwrap().unwrap_err();
        assert_eq!(parse_day_month_year("  "), None);
        assert_eq!(ok("4/2/2026"), IsoDate::parse("2026-02-04").unwrap());
        assert_eq!(ok("29.02.2028"), IsoDate::parse("2028-02-29").unwrap());
        assert_eq!(ok(" 01-12-2025 "), IsoDate::parse("2025-12-01").unwrap());
        assert_eq!(
            err("4 Feb"),
            "Enter the date as day/month/year, for example 18/01/2026."
        );
        assert_eq!(err("4/2/26"), "Write the year in full, for example 2026.");
        assert_eq!(
            err("4/13/2026"),
            "There is no month 13. Put the day first, then the month: dd/mm/yyyy."
        );
        assert_eq!(err("0/1/2026"), "There is no day 0. Enter a day from 1.");
        assert_eq!(err("31/02/2026"), "February 2026 has only 28 days.");
    }

    #[test]
    fn pickers_list_what_the_web_pickers_list() {
        let state = demo();
        let costs = cost_accounts(&state);
        assert_eq!(
            costs[0],
            (
                AccountId::from("acc-5100"),
                "5100 \u{b7} Programme: Water and sanitation".into()
            )
        );
        assert!(costs.windows(2).all(|w| w[0].1 < w[1].1));
        for (id, _) in &costs {
            let a = state.accounts.iter().find(|a| &a.id == id).unwrap();
            assert_eq!(a.account_type, ledger_core::types::AccountType::Expense);
            assert!(a.is_active && !a.is_group);
        }
        let cash: Vec<_> = cash_accounts(&state)
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        assert_eq!(cash.len(), 6);
        assert_eq!(cash[0], "1110 \u{b7} Bank: Main operating account");
        assert_eq!(cash[5], "1160 \u{b7} Petty cash");
        assert_eq!(fund_choices(&state).len(), 4);
        assert_eq!(fund_choices(&state)[0].1, "Unrestricted");
        let wash = grant_choices(&state, Some(&FundId::from("fund-wash")));
        assert_eq!(
            wash,
            [(
                GrantId::from(KISIMA),
                "Kisima WASH: Turkana boreholes".into()
            )]
        );
        assert_eq!(grant_choices(&state, None).len(), 4);
        let places: Vec<_> = dimension_choices(&state, DimensionKind::Location)
            .into_iter()
            .map(|(_, n)| n)
            .collect();
        assert_eq!(places, ["Turkana", "Kilifi", "Nairobi HQ"]);
        assert_eq!(dimension_choices(&state, DimensionKind::Project).len(), 2);
    }

    #[test]
    fn a_new_form_starts_on_today_with_one_unrestricted_line() {
        let state = demo();
        let form = ClaimForm::new(&state, &as_of());
        assert_eq!(form.date, "04/02/2026");
        assert_eq!(form.purpose, "");
        assert_eq!(form.lines.len(), 1);
        assert_eq!(form.lines[0].fund, Some(FundId::from("fund-unrestricted")));
        assert_eq!(form.lines[0].grant, None);
        assert_eq!(form.total(), 0);
    }

    #[test]
    fn lines_add_remove_and_follow_their_fund() {
        let state = demo();
        let mut form = filled(&state);
        assert_eq!(
            form.lines[0].grant,
            Some(GrantId::from(KISIMA)),
            "the only grant on the fund is picked"
        );
        assert!(!form.remove_line(0), "the last line stays");
        form.add_line(&state);
        assert_eq!(form.lines.len(), 2);
        let (first, second) = (&form.lines[0], &form.lines[1]);
        assert_eq!(second.description, "");
        assert_eq!(second.amount, "");
        assert_eq!(second.account, first.account);
        assert_eq!(second.fund, first.fund);
        assert_eq!(second.grant, first.grant);
        assert_eq!(second.project, first.project);
        assert_eq!(second.location, first.location);

        form.set_fund(&state, 1, Some(FundId::from("fund-wash")));
        assert_eq!(form.lines[1].grant, Some(GrantId::from(KISIMA)), "kept");
        form.set_fund(&state, 1, Some(FundId::from("fund-unrestricted")));
        assert_eq!(
            form.lines[1].grant,
            Some(GrantId::from("grant-harambee-2026")),
            "dropped, then the only grant on the new fund"
        );
        form.lines[1].grant = None;
        form.set_fund(&state, 1, None);
        assert_eq!(form.lines[1].grant, None);

        form.lines[1].amount = "abc".into();
        assert_eq!(form.total(), 125_050);
        form.lines[1].amount = "-5".into();
        assert_eq!(form.total(), 125_050, "negatives count as zero");
        form.lines[1].amount = "99.50".into();
        assert_eq!(form.total(), 135_000);
        assert!(form.remove_line(0));
        assert_eq!(form.lines.len(), 1);
        assert_eq!(form.total(), 9_950);
    }

    #[test]
    fn a_blank_form_names_every_missing_field() {
        let state = demo();
        let mut form = ClaimForm::new(&state, &as_of());
        form.date = String::new();
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.date.as_deref(),
            Some("Enter the date you spent the money.")
        );
        assert_eq!(
            errors.purpose.as_deref(),
            Some("Say what the trip or task was, e.g. Borehole site inspection, Lodwar.")
        );
        assert_eq!(
            errors.line(0, LineField::Description),
            Some("Say what this was for.")
        );
        assert_eq!(errors.line(0, LineField::Amount), Some("Enter the amount."));
        assert_eq!(
            errors.line(0, LineField::Account),
            Some("Choose what kind of cost this is.")
        );
        assert_eq!(errors.line(0, LineField::Fund), None);
        assert_eq!(errors.title(), "Fix these before saving");
        assert_eq!(
            errors.summary(),
            [
                "Date: Enter the date you spent the money.",
                "Purpose: Say what the trip or task was, e.g. Borehole site inspection, Lodwar.",
                "Line 1 description: Say what this was for.",
                "Line 1 amount: Enter the amount.",
                "Line 1 account: Choose what kind of cost this is.",
            ]
        );
    }

    #[test]
    fn the_form_checks_dates_amounts_and_restricted_funds() {
        let state = demo();
        let mut form = filled(&state);
        form.date = "05/02/2026".into();
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.date.as_deref(),
            Some("The date cannot be after today.")
        );
        assert_eq!(errors.title(), "Fix this before saving");
        form.date = "31/02/2026".into();
        assert_eq!(
            form.command(&state, &as_of(), true)
                .unwrap_err()
                .date
                .as_deref(),
            Some("February 2026 has only 28 days.")
        );
        form.date = "03/02/2026".into();

        form.lines[0].amount = "12.345".into();
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.line(0, LineField::Amount),
            Some("Enter a number, like 1250 or 1,250.50.")
        );
        form.lines[0].amount = "0".into();
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.line(0, LineField::Amount),
            Some("The amount must be more than zero.")
        );
        form.lines[0].amount = "100".into();

        form.lines[0].grant = None;
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.line(0, LineField::Grant),
            Some("Restricted money needs the grant it comes from.")
        );
        assert_eq!(
            errors.summary(),
            ["Line 1 grant: Restricted money needs the grant it comes from."]
        );
        form.lines[0].fund = None;
        let errors = form.command(&state, &as_of(), true).unwrap_err();
        assert_eq!(
            errors.line(0, LineField::Fund),
            Some("Choose the fund that pays for this.")
        );
    }

    #[test]
    fn a_complete_form_builds_claim_create() {
        let state = demo();
        let form = filled(&state);
        let command = form.command(&state, &as_of(), true).unwrap();
        assert_eq!(
            command,
            LedgerCommand::ClaimCreate {
                input: NewClaim {
                    id: None,
                    claim_date: as_of(),
                    currency: "KES".into(),
                    fx_rate: 1.0,
                    purpose: "Borehole follow-up, Lodwar".into(),
                    lines: vec![NewCodedLine {
                        description: "Fuel".into(),
                        account_id: AccountId::from("acc-5100"),
                        amount: 125_050.0,
                        fund_id: FundId::from("fund-wash"),
                        grant_id: Some(GrantId::from(KISIMA)),
                        dimension_ids: vec![
                            DimensionId::from("dim-a35"),
                            DimensionId::from("dim-loc-turkana")
                        ],
                    }],
                    receipt_names: vec![],
                    captured_offline: false,
                    submit: Some(true),
                },
            }
        );
        let LedgerCommand::ClaimCreate { input } = form.command(&state, &as_of(), false).unwrap()
        else {
            panic!("not claim.create");
        };
        assert_eq!(input.submit, Some(false));
    }

    #[test]
    fn attached_receipts_go_out_as_receipt_names() {
        let state = demo();
        let mut form = filled(&state);
        form.receipts = vec!["fuel.jpg".into(), "lodging.pdf".into()];
        let LedgerCommand::ClaimCreate { input } = form.command(&state, &as_of(), true).unwrap()
        else {
            panic!("not claim.create");
        };
        assert_eq!(input.receipt_names, vec!["fuel.jpg", "lodging.pdf"]);
    }

    #[test]
    fn a_sent_claim_joins_the_list_as_the_viewers() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let result = demo_org
            .submit(filled(&state).command(&state, &as_of(), true).unwrap())
            .unwrap();
        let claim = created(&result);
        let after = demo_org.state();
        let rows = claim_rows(&after, None);
        assert_eq!(rows[0].claim.id, claim);
        assert_eq!(rows[0].claim.number, "EC-0003");
        assert_eq!(rows[0].claim.claimant_name, "Demo User");
        assert_eq!(rows[0].claim.status, ClaimStatus::Submitted);
        assert_eq!(rows[0].total, 125_050);

        let detail = demo_org.detail(&claim);
        assert_eq!(detail.actions, []);
        assert_eq!(
            detail.next,
            "You made this claim, so someone else has to approve it. The books do not let \
             anyone approve their own spending."
        );
        assert_eq!(detail.trail.len(), 1);
        assert_eq!(detail.trail[0].label, "Submitted for approval");
        assert_eq!(detail.trail[0].who, "Demo User (you)");
        // The engine refuses self-approval with its own message.
        let err = demo_org
            .submit(detail.approve_command("", None))
            .unwrap_err();
        assert_eq!(
            Failure::new("Not done", &err).messages,
            ["You cannot approve your own claim."]
        );
    }

    #[test]
    fn a_draft_is_submitted_by_its_claimant() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let result = demo_org
            .submit(filled(&state).command(&state, &as_of(), false).unwrap())
            .unwrap();
        let claim = created(&result);
        let detail = demo_org.detail(&claim);
        assert_eq!(detail.claim.status, ClaimStatus::Draft);
        assert_eq!(detail.actions, [ClaimAction::Submit]);
        assert_eq!(
            detail.next,
            "This is a draft. Submit it when the lines and receipts are complete."
        );
        assert_eq!(detail.dialog(ClaimAction::Submit), None);
        assert_eq!(
            detail.submit_command(),
            LedgerCommand::ClaimSubmit {
                claim_id: claim.clone()
            }
        );
        demo_org.submit(detail.submit_command()).unwrap();
        assert_eq!(demo_org.detail(&claim).claim.status, ClaimStatus::Submitted);
        assert_eq!(
            ClaimAction::Submit.done_notice(),
            "Submitted. A manager can now approve it."
        );

        // Someone else's draft cannot be submitted from here.
        let mut other = demo_org.state();
        let i = other.claims.iter().position(|c| c.id == claim).unwrap();
        other.claims[i].status = ClaimStatus::Draft;
        other.claims[i].claimant = ActorId::from("staff-achieng");
        other.claims[i].claimant_name = "Achieng Odhiambo".into();
        let detail = ClaimDetail::new(&other, &claim, Some(&demo_actor())).unwrap();
        assert_eq!(detail.actions, []);
        assert_eq!(
            detail.next,
            "This is Achieng Odhiambo\u{2019}s draft. Only they can submit it."
        );
    }

    #[test]
    fn a_submitted_claim_waits_for_an_approver() {
        let state = demo();
        let detail = ClaimDetail::new(&state, &id(SUBMITTED), Some(&demo_actor())).unwrap();
        assert_eq!(
            detail.lede(),
            "EC-0002 \u{b7} Achieng Odhiambo \u{b7} spent 3 Feb 2026"
        );
        assert_eq!(detail.actions, [ClaimAction::Approve, ClaimAction::Reject]);
        assert_eq!(
            detail.next,
            "Approving posts the claim to the books straight away: the costs land on their \
             grants and budget lines, and the money is owed to Achieng Odhiambo."
        );
        assert_eq!(detail.total, 1_850_000);
        assert_eq!(detail.outstanding, 0);
        assert_eq!(detail.posting, None);
        assert_eq!(detail.books, []);
        assert_eq!(detail.lines.len(), 2);
        assert_eq!(detail.lines[0].fund, "Water and sanitation (restricted)");
        assert_eq!(detail.lines[0].grant, "Kisima WASH: Turkana boreholes");
        assert!(detail.lines[0].account.starts_with("5700 "));
        assert_eq!(detail.lines[0].amount, 1_420_000);

        let mut viewer = demo_actor();
        viewer.can_approve = false;
        let detail = ClaimDetail::new(&state, &id(SUBMITTED), Some(&viewer)).unwrap();
        assert_eq!(detail.actions, []);
        assert_eq!(detail.next, "Waiting for a manager with approval rights.");
        assert_eq!(ClaimDetail::new(&state, &id("gone"), None), None);
    }

    #[test]
    fn approving_posts_to_the_claims_grant() {
        let mut demo_org = Demo::new();
        let before = demo_org.state();
        let spend_before = before.grant_spend(&GrantId::from(KISIMA), None);
        let detail = demo_org.detail(&id(SUBMITTED));
        let dialog = detail.dialog(ClaimAction::Approve).unwrap();
        assert_eq!(dialog.title, "Approve this claim");
        assert_eq!(
            dialog.description,
            "EC-0002 \u{b7} Achieng Odhiambo \u{b7} KES 18,500.00"
        );
        assert_eq!(dialog.confirm, "Approve and post");
        assert_eq!(dialog.override_label(), "Override and approve and post");
        assert_eq!(
            detail.approve_command("  Receipts checked ", None),
            LedgerCommand::ClaimApprove {
                claim_id: id(SUBMITTED),
                note: Some("Receipts checked".into()),
                override_reason: None,
            }
        );
        assert_eq!(
            detail.approve_command("   ", Some("Board approved")),
            LedgerCommand::ClaimApprove {
                claim_id: id(SUBMITTED),
                note: None,
                override_reason: Some("Board approved".into()),
            }
        );

        demo_org
            .submit(detail.approve_command("Receipts checked", None))
            .unwrap();
        let after = demo_org.state();
        let detail = demo_org.detail(&id(SUBMITTED));
        assert_eq!(detail.claim.status, ClaimStatus::Posted);
        let entry_id = detail.claim.entry_id.clone().unwrap();
        let entry = after.entries.iter().find(|e| e.id == entry_id).unwrap();
        let posting = detail.posting.clone().unwrap();
        assert_eq!(posting.entry_id, entry_id);
        assert_eq!(posting.number, entry.number);
        assert_eq!(
            posting.grants,
            [GrantHit {
                grant_id: GrantId::from(KISIMA),
                name: "Kisima WASH: Turkana boreholes".into(),
                charged: 1_850_000,
                spend: spend_before + 1_850_000,
                award: after
                    .grants
                    .iter()
                    .find(|g| g.id.as_str() == KISIMA)
                    .unwrap()
                    .award,
            }]
        );
        assert_eq!(
            after.grant_spend(&GrantId::from(KISIMA), None) - spend_before,
            detail.total,
            "the grant's spend moved by the claim total"
        );
        assert_eq!(
            detail.books,
            [BookRow {
                label: "Claim posted: costs and the amount owed".into(),
                entry_id: entry_id.clone(),
                number: entry.number.clone(),
            }]
        );
        let labels: Vec<_> = detail.trail.iter().map(|s| s.label).collect();
        assert_eq!(
            labels,
            ["Submitted for approval", "Approved", "Posted to the books"]
        );
        assert_eq!(detail.trail[0].who, "Achieng Odhiambo");
        assert_eq!(detail.trail[1].who, "Demo User (you)");
        assert_eq!(detail.trail[1].note.as_deref(), Some("Receipts checked"));
        assert_eq!(detail.trail[2].note, None, "the entry number is not quoted");
        assert_eq!(detail.outstanding, 1_850_000);
        assert_eq!(detail.actions, [ClaimAction::Reimburse]);
        assert_eq!(
            detail.next,
            "Approved and posted. KES 18,500.00 is owed to Achieng Odhiambo."
        );
        assert_eq!(
            ClaimAction::Approve.done_notice(),
            "Approved and posted to the books."
        );
    }

    #[test]
    fn a_grant_that_has_not_started_can_be_overridden() {
        // A claim by someone else, dated before the PAEF grant starts.
        let mut state = demo();
        let mut env = demo_env_after(&state);
        let staff = Actor {
            id: ActorId::from("staff-achieng"),
            name: "Achieng Odhiambo".into(),
            can_post: false,
            can_approve: false,
            can_close: false,
        };
        let mut form = ClaimForm::new(&state, &as_of());
        form.date = "20/01/2026".into();
        form.purpose = "School visit".into();
        form.lines[0].description = "Bus fare".into();
        form.lines[0].amount = "800".into();
        form.lines[0].account = Some(AccountId::from("acc-5300"));
        form.set_fund(&state, 0, Some(FundId::from("fund-education")));
        let command = form.command(&state, &as_of(), true).unwrap();
        let mut ledger = Ledger::new(&mut state, &mut env);
        let claim = created(&apply_command(&mut ledger, &staff, command).unwrap());
        let detail = ClaimDetail::new(ledger.state(), &claim, Some(&demo_actor())).unwrap();
        let err = BackendError::Command(
            apply_command(&mut ledger, &demo_actor(), detail.approve_command("", None))
                .unwrap_err(),
        );
        assert!(has_overridable(&err));
        let failure = Failure::new("Not done", &err);
        assert_eq!(failure.messages.len(), 1);
        assert!(
            failure.messages[0].ends_with("PAEF education grant starts on 2026-02-01."),
            "{failure:?}"
        );
        apply_command(
            &mut ledger,
            &demo_actor(),
            detail.approve_command("", Some("Agreed with the donor")),
        )
        .unwrap();
        let detail = ClaimDetail::new(ledger.state(), &claim, Some(&demo_actor())).unwrap();
        assert_eq!(detail.claim.status, ClaimStatus::Posted);
        assert_eq!(detail.posting.unwrap().grants[0].charged, 80_000);

        let refused = BackendError::Connection("down".into());
        assert!(!has_overridable(&refused));
    }

    #[test]
    fn rejecting_needs_a_reason() {
        let mut demo_org = Demo::new();
        let detail = demo_org.detail(&id(SUBMITTED));
        let dialog = detail.dialog(ClaimAction::Reject).unwrap();
        assert_eq!(dialog.title, "Reject this claim");
        assert_eq!(
            dialog.description,
            "The claimant sees your reason. Nothing is posted to the books."
        );
        assert_eq!(dialog.confirm, "Reject claim");
        assert_eq!(
            detail.reject_command("  "),
            Err("Tell the claimant why, so they can put it right.".into())
        );
        let command = detail.reject_command(" Receipts missing ").unwrap();
        assert_eq!(
            command,
            LedgerCommand::ClaimReject {
                claim_id: id(SUBMITTED),
                note: "Receipts missing".into(),
            }
        );
        let spend = demo_org.state().grant_spend(&GrantId::from(KISIMA), None);
        demo_org.submit(command).unwrap();
        let detail = demo_org.detail(&id(SUBMITTED));
        assert_eq!(detail.claim.status, ClaimStatus::Rejected);
        assert_eq!(detail.actions, []);
        assert_eq!(
            detail.next,
            "Rejected. The reason is in the approval trail below; the claimant can make a new \
             claim."
        );
        let last = detail.trail.last().unwrap();
        assert_eq!(last.label, "Rejected");
        assert!(last.rejected);
        assert_eq!(last.note.as_deref(), Some("Receipts missing"));
        assert_eq!(
            demo_org.state().grant_spend(&GrantId::from(KISIMA), None),
            spend,
            "nothing posted"
        );
        assert_eq!(
            ClaimAction::Reject.done_notice(),
            "Rejected. The claimant can see your reason."
        );
    }

    #[test]
    fn reimbursing_pays_what_is_owed_from_a_cash_account() {
        let mut demo_org = Demo::new();
        let detail = demo_org.detail(&id(SUBMITTED));
        demo_org.submit(detail.approve_command("", None)).unwrap();
        let detail = demo_org.detail(&id(SUBMITTED));
        let dialog = detail.dialog(ClaimAction::Reimburse).unwrap();
        assert_eq!(dialog.title, "Record the reimbursement");
        assert_eq!(
            dialog.description,
            "Pays Achieng Odhiambo the full KES 18,500.00 still owed."
        );
        assert_eq!(dialog.confirm, "Record payment");

        let mut form = ReimburseForm::new(&as_of());
        assert_eq!(form.date, "04/02/2026");
        assert_eq!(
            form.command(&id(SUBMITTED)),
            Err(ReimburseErrors {
                date: None,
                cash: Some("Choose the account the money left from.".into()),
            })
        );
        form.date = " ".into();
        assert_eq!(
            form.command(&id(SUBMITTED)).unwrap_err().date.as_deref(),
            Some("Enter the date the money was sent.")
        );
        form.date = "4/2/2026".into();
        form.cash_account = Some(AccountId::from("acc-1150"));
        assert_eq!(
            form.command(&id(SUBMITTED)),
            Ok(LedgerCommand::ClaimReimburse {
                claim_id: id(SUBMITTED),
                date: "2026-02-04".into(),
                cash_account_id: AccountId::from("acc-1150"),
                reference: Nullish::Null,
            })
        );
        form.reference = " QX7 ".into();
        let command = form.command(&id(SUBMITTED)).unwrap();
        let LedgerCommand::ClaimReimburse { reference, .. } = &command else {
            panic!("not claim.reimburse");
        };
        assert_eq!(reference, &Nullish::Value("QX7".into()));

        demo_org.submit(command).unwrap();
        let detail = demo_org.detail(&id(SUBMITTED));
        assert_eq!(detail.claim.status, ClaimStatus::Reimbursed);
        assert_eq!(detail.outstanding, 0);
        assert_eq!(detail.actions, []);
        assert_eq!(detail.next, "Paid back in full. Nothing more to do.");
        assert_eq!(detail.books.len(), 2);
        assert!(
            detail.books[1]
                .label
                .starts_with("Paid back 4 Feb 2026 \u{b7} KES 18,500.00"),
            "{:?}",
            detail.books[1]
        );
        let last = detail.trail.last().unwrap();
        assert_eq!(last.label, "Paid back");
        assert_eq!(last.note, None);
        assert_eq!(last.reference.as_deref(), Some("QX7"));
        assert_eq!(
            ClaimAction::Reimburse.done_notice(),
            "Reimbursement recorded."
        );
    }

    #[test]
    fn a_paid_back_claim_shows_its_whole_story() {
        let state = demo();
        let detail = ClaimDetail::new(&state, &id(REIMBURSED), Some(&demo_actor())).unwrap();
        let labels: Vec<_> = detail.trail.iter().map(|s| s.label).collect();
        assert_eq!(
            labels,
            [
                "Submitted for approval",
                "Approved",
                "Posted to the books",
                "Paid back"
            ]
        );
        assert_eq!(detail.trail[0].who, "Kiprop Mutai");
        assert_eq!(detail.trail[0].at, "4 Feb 2026, 09:00");
        assert_eq!(
            detail.trail[1].note.as_deref(),
            Some("Receipts match the survey plan.")
        );
        assert_eq!(detail.trail[3].reference.as_deref(), Some("RAN4K2PQ7Z"));
        let posting = detail.posting.unwrap();
        assert_eq!(posting.number, "JE-0020");
        assert_eq!(posting.grants.len(), 1);
        assert_eq!(posting.grants[0].name, "Northlight maternal health 2026");
        assert_eq!(posting.grants[0].charged, 1_230_000);
        assert_eq!(
            detail.books[0],
            BookRow {
                label: "Claim posted: costs and the amount owed".into(),
                entry_id: EntryId::from("demo-004k"),
                number: "JE-0020".into(),
            }
        );
        assert!(detail.books[1].label.starts_with("Paid back 23 Jan 2026"));
        assert!(detail.books[1].label.ends_with("RAN4K2PQ7Z"));
    }

    #[test]
    fn people_and_figures_read_as_the_web_screen() {
        let me = demo_actor();
        let claimant = (&ActorId::from("staff-kiprop"), "Kiprop Mutai");
        assert_eq!(person_name(&me.id, Some(&me), claimant), "Demo User (you)");
        assert_eq!(
            person_name(&ActorId::from("staff-kiprop"), Some(&me), claimant),
            "Kiprop Mutai"
        );
        assert_eq!(
            person_name(&ActorId::from("system"), None, claimant),
            "FlowTrace"
        );
        assert_eq!(
            person_name(&ActorId::from("someone"), None, claimant),
            "someone"
        );
        let at = Timestamp::parse("2026-01-09T14:05:00.000Z").unwrap();
        assert_eq!(format_date_time(&at), "9 Jan 2026, 14:05");
        assert_eq!(format_whole(1_850_000), "18,500");
        assert_eq!(format_whole(1_850_050), "18,501");
        assert_eq!(format_whole(-150), "(2)");
        assert_eq!(format_whole(0), "\u{2013}");
        assert_eq!(LineField::Account.label(), "account");
        assert_eq!(ClaimAction::Approve.label(), "Approve and post");
        assert_eq!(ClaimAction::Reject.label(), "Reject");
        assert_eq!(ClaimAction::Submit.label(), "Submit for approval");
        assert_eq!(ClaimAction::Reimburse.label(), "Record the reimbursement");
    }
}
