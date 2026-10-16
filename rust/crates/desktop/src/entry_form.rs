//! View models of the Journal screen's manual entry form and CSV journal import: the lines as
//! typed with the live balance check, the field and engine problems beside their fields, the
//! commands Save draft and Post send, and the import preview with each draft's issues. Plain
//! structs, no GPUI; `panels::journal` draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/ledger/new.tsx` and
//! `import.tsx`; the rules stay in `ledger-core` (`engine/validate.rs`, `importers.rs`).

use std::path::Path;

use ledger_core::coa_template::UNRESTRICTED_FUND_ID;
use ledger_core::engine::EntryOptions;
use ledger_core::importers::{JournalSource, parse_journal_csv};
use ledger_core::money::{assert_minor, format_major, js_round, parse_money};
use ledger_core::types::{
    AccountId, DimensionId, EntryId, EntryInput, EntryStatus, FlowNodeId, FundId, GrantId, IsoDate,
    LedgerState, LineInput, Nullish,
};
use ledger_core::{Issue, IssueCode, LedgerCommand, Minor};

use crate::accounts::{format_date, format_minor};
use crate::claims::{iso_to_day_month_year, parse_day_month_year};
use crate::journal::Notice;

/// The largest file the import reads (`import.tsx`, `readFile`).
pub const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;

/// What a loaded draft's line carries that the form does not show, so editing keeps it
/// (`new.tsx`, `LineDraft.keep`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Kept {
    /// The line's currency.
    pub currency: Option<String>,
    /// Its rate to base currency.
    pub fx_rate: Option<f64>,
    /// The flow node it moved through.
    pub flow_node_id: Nullish<FlowNodeId>,
    /// Dimension values after the first.
    pub extra_dims: Vec<DimensionId>,
}

/// One line as typed (`new.tsx`, `LineDraft`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryLineDraft {
    /// A stable key for the line's widgets.
    pub key: u64,
    /// Account.
    pub account: Option<AccountId>,
    /// Fund.
    pub fund: Option<FundId>,
    /// Grant.
    pub grant: Option<GrantId>,
    /// One tag.
    pub dimension: Option<DimensionId>,
    /// Debit as typed.
    pub debit: String,
    /// Credit as typed.
    pub credit: String,
    /// Line memo as typed.
    pub memo: String,
    /// Kept from a loaded draft.
    pub keep: Kept,
}

impl EntryLineDraft {
    /// The typed amount of `side` as the engine takes it: `Math.max(0, parseMoney(raw) ?? 0)`
    /// (`new.tsx`, `input`).
    fn amount(&self, side: Side) -> Minor {
        let raw = match side {
            Side::Debit => &self.debit,
            Side::Credit => &self.credit,
        };
        parse_money(raw).unwrap_or(0).max(0)
    }
}

/// A side of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Debit.
    Debit,
    /// Credit.
    Credit,
}

impl Side {
    fn word(self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Credit => "credit",
        }
    }
}

/// The control an engine issue belongs to (`new.tsx`, `ISSUE_FIELD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineField {
    /// Account picker.
    Account,
    /// Fund picker.
    Fund,
    /// Grant picker.
    Grant,
    /// Debit field.
    Debit,
    /// Credit field.
    Credit,
}

/// `ISSUE_FIELD` (`new.tsx`): which control an engine issue sits beside.
fn issue_field(code: IssueCode) -> Option<LineField> {
    use IssueCode as C;
    match code {
        C::UnknownAccount | C::GroupAccount | C::InactiveAccount => Some(LineField::Account),
        C::UnknownFund => Some(LineField::Fund),
        C::UnknownGrant
        | C::GrantFundMismatch
        | C::GrantNotStarted
        | C::GrantEnded
        | C::RestrictedNeedsGrant => Some(LineField::Grant),
        C::LineZero
        | C::LineBothSides
        | C::NegativeAmount
        | C::NotMinorUnits
        | C::FxRateRequired => Some(LineField::Debit),
        _ => None,
    }
}

/// `strip` (`new.tsx`): an engine line message without its `Line n: ` prefix.
fn strip(message: &str) -> &str {
    message
        .strip_prefix("Line ")
        .and_then(|rest| {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            (digits > 0)
                .then(|| rest[digits..].strip_prefix(": "))
                .flatten()
        })
        .unwrap_or(message)
}

/// An engine issue as the summary lists it: the overridable ones marked.
fn issue_text(issue: &Issue) -> String {
    if issue.overridable == Some(true) {
        format!("{} (can be overridden with a reason)", issue.message)
    } else {
        issue.message.clone()
    }
}

/// The problems the form finds itself, before the engine sees the entry (`new.tsx`,
/// `fieldErrors`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldErrors {
    /// Under the date.
    pub date: Option<String>,
    /// Under the memo.
    pub memo: Option<String>,
    /// Under a line's debit or credit.
    pub lines: Vec<(usize, Side, String)>,
}

impl FieldErrors {
    /// The message under line `index`'s `side`.
    #[must_use]
    pub fn line(&self, index: usize, side: Side) -> Option<&str> {
        self.lines
            .iter()
            .find(|(i, s, _)| *i == index && *s == side)
            .map(|(_, _, m)| m.as_str())
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The live totals under the lines (`new.tsx`, `totals`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Balance {
    /// Debits in base currency.
    pub debits: Minor,
    /// Credits in base currency.
    pub credits: Minor,
    /// Debits less credits.
    pub difference: Minor,
}

/// `formatMinor(v, { zero: '0.00' })`.
fn zero_as_digits(minor: Minor) -> String {
    if minor == 0 {
        "0.00".to_owned()
    } else {
        format_minor(minor)
    }
}

impl Balance {
    /// True when debits equal credits.
    #[must_use]
    pub fn is_balanced(&self) -> bool {
        self.difference == 0
    }

    /// Debits, `0.00` for none.
    #[must_use]
    pub fn debits_text(&self) -> String {
        zero_as_digits(self.debits)
    }

    /// Credits, `0.00` for none.
    #[must_use]
    pub fn credits_text(&self) -> String {
        zero_as_digits(self.credits)
    }

    /// `Balanced`, or the size of the gap.
    #[must_use]
    pub fn difference_text(&self) -> String {
        if self.is_balanced() {
            "Balanced".to_owned()
        } else {
            format_minor(self.difference.abs())
        }
    }
}

/// The box above the form (`new.tsx`, `summaryItems`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// Each problem, the form's first and then the engine's.
    pub items: Vec<String>,
}

impl Summary {
    /// `This stops the entry posting` or `3 things stop the entry posting`.
    #[must_use]
    pub fn title(&self) -> String {
        match self.items.len() {
            1 => "This stops the entry posting".to_owned(),
            n => format!("{n} things stop the entry posting"),
        }
    }
}

/// Why Post sent nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blocked {
    /// Only overridable grant rules stop the entry: open the override box.
    pub needs_override: bool,
    /// The override was asked for with no reason.
    pub override_error: Option<String>,
}

/// The manual entry form (`new.tsx`, `EntryForm`).
#[derive(Debug, Clone, PartialEq)]
pub struct EntryForm {
    /// The draft being edited, or `None` for a new entry.
    pub draft: Option<EntryId>,
    /// Date as typed, `dd/mm/yyyy`.
    pub date: String,
    /// Memo as typed.
    pub memo: String,
    /// Override reason as typed.
    pub override_reason: String,
    /// The lines.
    pub lines: Vec<EntryLineDraft>,
    next_key: u64,
}

/// `defaultFund` (`new.tsx`): the unrestricted fund when the books have it, else the first.
fn default_fund(state: &LedgerState) -> Option<FundId> {
    state
        .funds
        .iter()
        .find(|f| f.id.as_str() == UNRESTRICTED_FUND_ID)
        .or_else(|| state.funds.first())
        .map(|f| f.id.clone())
}

/// `minorToText` (`new.tsx`): `2500.00`, or empty for zero.
fn minor_to_text(minor: Option<f64>) -> String {
    match minor.and_then(|m| assert_minor(m, "amount").ok()) {
        Some(m) if m != 0 => format_major(m),
        _ => String::new(),
    }
}

/// A rounded f64 of minor units as `Minor`, saturating where no `Minor` holds it.
fn to_minor(x: f64) -> Minor {
    assert_minor(js_round(x), "amount").unwrap_or(if x < 0.0 { Minor::MIN } else { Minor::MAX })
}

impl EntryForm {
    /// A new entry dated `as_of` with two blank lines on the default fund.
    #[must_use]
    pub fn new(state: &LedgerState, as_of: &IsoDate) -> Self {
        let mut form = Self {
            draft: None,
            date: iso_to_day_month_year(as_of),
            memo: String::new(),
            override_reason: String::new(),
            lines: Vec::new(),
            next_key: 0,
        };
        form.add_line(state);
        form.add_line(state);
        form
    }

    /// The draft `id` loaded into the form (`new.tsx`, the load effect).
    ///
    /// # Errors
    ///
    /// The notice to show when `id` is not a draft in the books.
    pub fn from_draft(state: &LedgerState, id: &EntryId) -> Result<Self, Notice> {
        let entry = state.entry(id).ok_or_else(|| Notice {
            title: "This draft is not in the books".to_owned(),
            body: "It may have been deleted or posted from another screen.".to_owned(),
        })?;
        if entry.status != EntryStatus::Draft {
            let status = match entry.status {
                EntryStatus::Posted => "posted",
                EntryStatus::Reversed => "reversed",
                EntryStatus::Draft => "a draft",
            };
            return Err(Notice {
                title: format!("{} is {status}", entry.number),
                body: "Posted entries cannot be edited. Reverse it and post a correcting entry \
                       instead."
                    .to_owned(),
            });
        }
        let input = state.input_of(id).map_err(|e| Notice {
            title: "This draft is not in the books".to_owned(),
            body: e.to_string(),
        })?;
        let mut form = Self {
            draft: Some(id.clone()),
            date: IsoDate::parse(&input.date)
                .map(|d| iso_to_day_month_year(&d))
                .unwrap_or_default(),
            memo: input.memo,
            override_reason: input.override_reason.value().cloned().unwrap_or_default(),
            lines: Vec::new(),
            next_key: 0,
        };
        for l in input.lines {
            let mut dims = l.dimension_ids.unwrap_or_default().into_iter();
            let dimension = dims.next();
            form.next_key += 1;
            form.lines.push(EntryLineDraft {
                key: form.next_key,
                account: Some(l.account_id),
                fund: Some(l.fund_id),
                grant: l.grant_id.value().cloned(),
                dimension,
                debit: minor_to_text(l.debit),
                credit: minor_to_text(l.credit),
                memo: l.memo.value().cloned().unwrap_or_default(),
                keep: Kept {
                    currency: l.currency,
                    fx_rate: l.fx_rate,
                    flow_node_id: l.flow_node_id,
                    extra_dims: dims.collect(),
                },
            });
        }
        while form.lines.len() < 2 {
            form.add_line(state);
        }
        Ok(form)
    }

    /// `New journal entry` or `Edit draft entry`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        if self.draft.is_some() {
            "Edit draft entry"
        } else {
            "New journal entry"
        }
    }

    /// Adds a blank line on the default fund (`blankLine`).
    pub fn add_line(&mut self, state: &LedgerState) {
        self.next_key += 1;
        self.lines.push(EntryLineDraft {
            key: self.next_key,
            fund: default_fund(state),
            ..EntryLineDraft::default()
        });
    }

    /// Removes line `index` while more than two remain.
    pub fn remove_line(&mut self, index: usize) -> bool {
        if self.lines.len() <= 2 || index >= self.lines.len() {
            return false;
        }
        self.lines.remove(index);
        true
    }

    /// Sets line `index`'s fund, dropping a grant of another fund (`updateLine`).
    pub fn set_fund(&mut self, state: &LedgerState, index: usize, fund: Option<FundId>) {
        let Some(line) = self.lines.get_mut(index) else {
            return;
        };
        let other = line.grant.as_ref().is_some_and(|id| {
            state
                .grants
                .iter()
                .any(|g| &g.id == id && Some(&g.fund_id) != fund.as_ref())
        });
        if other {
            line.grant = None;
        }
        line.fund = fund;
    }

    /// Sets a debit; a non-empty one clears the credit.
    pub fn set_debit(&mut self, index: usize, text: &str) {
        if let Some(line) = self.lines.get_mut(index) {
            text.clone_into(&mut line.debit);
            if !text.is_empty() {
                line.credit.clear();
            }
        }
    }

    /// Sets a credit; a non-empty one clears the debit.
    pub fn set_credit(&mut self, index: usize, text: &str) {
        if let Some(line) = self.lines.get_mut(index) {
            text.clone_into(&mut line.credit);
            if !text.is_empty() {
                line.debit.clear();
            }
        }
    }

    /// The form's own problems (`new.tsx`, `fieldErrors`).
    #[must_use]
    pub fn field_errors(&self) -> FieldErrors {
        let mut e = FieldErrors {
            date: match parse_day_month_year(&self.date) {
                None => Some("Enter the entry date.".to_owned()),
                Some(Err(message)) => Some(message),
                Some(Ok(_)) => None,
            },
            memo: self
                .memo
                .trim()
                .is_empty()
                .then(|| "Describe the entry, as it should read in the journal.".to_owned()),
            lines: Vec::new(),
        };
        for (i, l) in self.lines.iter().enumerate() {
            for side in [Side::Debit, Side::Credit] {
                let raw = match side {
                    Side::Debit => l.debit.trim(),
                    Side::Credit => l.credit.trim(),
                };
                if raw.is_empty() {
                    continue;
                }
                match parse_money(raw) {
                    None => {
                        e.lines.push((
                            i,
                            side,
                            "Enter a number, like 1250 or 1,250.50.".to_owned(),
                        ));
                    }
                    Some(m) if m < 0 => e.lines.push((
                        i,
                        side,
                        "Enter a positive amount; use the other column for the opposite side."
                            .to_owned(),
                    )),
                    Some(_) => {}
                }
            }
        }
        e
    }

    /// The entry as the engine takes it (`new.tsx`, `input`). Amounts are minor units.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "typed amounts are safe integers, exact in f64, as the TS numbers are"
    )]
    pub fn input(&self) -> EntryInput {
        let date = match parse_day_month_year(&self.date) {
            Some(Ok(d)) => d.as_str().to_owned(),
            _ => String::new(),
        };
        let reason = self.override_reason.trim();
        EntryInput {
            date,
            memo: self.memo.clone(),
            override_reason: if reason.is_empty() {
                Nullish::Null
            } else {
                Nullish::Value(reason.to_owned())
            },
            source_type: None,
            source_id: Nullish::Undefined,
            lines: self
                .lines
                .iter()
                .map(|l| {
                    let mut dims: Vec<DimensionId> = l.dimension.iter().cloned().collect();
                    dims.extend(
                        l.keep
                            .extra_dims
                            .iter()
                            .filter(|d| Some(*d) != l.dimension.as_ref())
                            .cloned(),
                    );
                    let memo = l.memo.trim();
                    LineInput {
                        account_id: l.account.clone().unwrap_or_else(|| AccountId::from("")),
                        debit: Some(l.amount(Side::Debit) as f64),
                        credit: Some(l.amount(Side::Credit) as f64),
                        currency: l.keep.currency.clone(),
                        fx_rate: l.keep.fx_rate,
                        fund_id: l.fund.clone().unwrap_or_else(|| FundId::from("")),
                        grant_id: l.grant.clone().map_or(Nullish::Null, Nullish::Value),
                        dimension_ids: Some(dims),
                        flow_node_id: match &l.keep.flow_node_id {
                            Nullish::Value(v) => Nullish::Value(v.clone()),
                            _ => Nullish::Null,
                        },
                        memo: if memo.is_empty() {
                            Nullish::Null
                        } else {
                            Nullish::Value(memo.to_owned())
                        },
                    }
                })
                .collect(),
        }
    }

    /// The live balance check: each side at its line's rate when the line is not in base
    /// currency, rounded as `Math.round` (`new.tsx`, `totals`).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "typed amounts are safe integers, exact in f64, as the TS numbers are"
    )]
    pub fn balance(&self, state: &LedgerState) -> Balance {
        let base = &state.settings.base_currency;
        let (mut debits, mut credits) = (0.0, 0.0);
        for l in &self.lines {
            let rate = match &l.keep.currency {
                Some(c) if c != base => l.keep.fx_rate.unwrap_or(1.0),
                _ => 1.0,
            };
            debits += js_round(l.amount(Side::Debit) as f64 * rate);
            credits += js_round(l.amount(Side::Credit) as f64 * rate);
        }
        let (debits, credits) = (to_minor(debits), to_minor(credits));
        Balance {
            debits,
            credits,
            difference: debits.saturating_sub(credits),
        }
    }

    /// The engine's issues with the entry (`ledger.validateEntry(input, draftId)`).
    #[must_use]
    pub fn issues(&self, state: &LedgerState) -> Vec<Issue> {
        validate(state, &self.input(), self.draft.as_ref())
    }

    /// The engine's message for line `index`'s `field`, without its `Line n:` prefix; several
    /// joined by spaces (`new.tsx`, `fieldIssue`).
    #[must_use]
    pub fn line_issue(&self, issues: &[Issue], index: usize, field: LineField) -> Option<String> {
        let found: Vec<&str> = issues
            .iter()
            .filter(|x| x.line == Some(index) && issue_field(x.code) == Some(field))
            .map(|x| strip(&x.message))
            .collect();
        (!found.is_empty()).then(|| found.join(" "))
    }

    /// The engine's messages on line `index` that sit beside no field (`new.tsx`, `loose`).
    #[must_use]
    pub fn loose_issues(&self, issues: &[Issue], index: usize) -> Vec<String> {
        issues
            .iter()
            .filter(|x| x.line == Some(index) && issue_field(x.code).is_none())
            .map(|x| strip(&x.message).to_owned())
            .collect()
    }

    /// The summary box (`new.tsx`, `summaryItems`): the form's own problems, then the engine's,
    /// leaving out its date and memo messages when the fields already say so.
    #[must_use]
    pub fn summary(&self, state: &LedgerState) -> Summary {
        let e = self.field_errors();
        let mut items = Vec::new();
        if let Some(d) = &e.date {
            items.push(format!("Date: {d}"));
        }
        if let Some(m) = &e.memo {
            items.push(format!("Memo: {m}"));
        }
        for i in 0..self.lines.len() {
            for side in [Side::Debit, Side::Credit] {
                if let Some(m) = e.line(i, side) {
                    items.push(format!("Line {} {}: {m}", i + 1, side.word()));
                }
            }
        }
        let skip_input = e.date.is_some() || e.memo.is_some();
        items.extend(
            self.issues(state)
                .iter()
                .filter(|x| !(x.code == IssueCode::InvalidInput && skip_input))
                .map(issue_text),
        );
        Summary { items }
    }

    /// Save draft: `entry.saveDraft`, or `entry.updateDraft` for a loaded draft
    /// (`new.tsx`, `saveDraft`).
    ///
    /// # Errors
    ///
    /// The form's own problems.
    pub fn save_command(&self) -> Result<LedgerCommand, FieldErrors> {
        let errors = self.field_errors();
        if !errors.is_empty() {
            return Err(errors);
        }
        let input = self.input();
        Ok(match &self.draft {
            Some(id) => LedgerCommand::EntryUpdateDraft {
                entry_id: id.clone(),
                input,
            },
            None => LedgerCommand::EntrySaveDraft { input },
        })
    }

    /// Post: `entry.post`, or for a loaded draft `entry.updateDraft` then `entry.postDraft`
    /// (`new.tsx`, `post`). With `with_override`, the override reason is required and lets the
    /// overridable issues through.
    ///
    /// # Errors
    ///
    /// Why nothing is sent: the form's or the engine's problems, and whether only an override
    /// stands in the way.
    pub fn post_commands(
        &self,
        state: &LedgerState,
        with_override: bool,
    ) -> Result<Vec<LedgerCommand>, Blocked> {
        let reason = self.override_reason.trim();
        if with_override && reason.is_empty() {
            return Err(Blocked {
                needs_override: true,
                override_error: Some("Say why this entry should post despite the rule.".to_owned()),
            });
        }
        let reason = with_override.then(|| reason.to_owned());
        let mut input = self.input();
        input.override_reason = reason.clone().map_or(Nullish::Null, Nullish::Value);
        let found = validate(state, &input, self.draft.as_ref());
        let blocking: Vec<&Issue> = found
            .iter()
            .filter(|x| !(x.overridable == Some(true) && reason.is_some()))
            .collect();
        let fields_ok = self.field_errors().is_empty();
        if !fields_ok || !blocking.is_empty() {
            return Err(Blocked {
                needs_override: fields_ok && blocking.iter().all(|x| x.overridable == Some(true)),
                override_error: None,
            });
        }
        Ok(match &self.draft {
            Some(id) => vec![
                LedgerCommand::EntryUpdateDraft {
                    entry_id: id.clone(),
                    input,
                },
                LedgerCommand::EntryPostDraft {
                    entry_id: id.clone(),
                    override_reason: reason,
                },
            ],
            None => vec![LedgerCommand::EntryPost { input }],
        })
    }
}

/// `ledger.validateEntry(input, ignore)`; an amount past the safe range, which TS throws on,
/// shows as one issue with that error's text.
fn validate(state: &LedgerState, input: &EntryInput, ignore: Option<&EntryId>) -> Vec<Issue> {
    state
        .validate_entry(input, ignore, EntryOptions::default())
        .unwrap_or_else(|err| vec![Issue::new(IssueCode::InvalidInput, err.to_string())])
}

/// One draft of the import preview.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewEntry {
    /// What is imported.
    pub input: EntryInput,
    /// Its memo.
    pub memo: String,
    /// `15 Jan 2026`, or the date as read when it is not one.
    pub date: String,
    /// How many lines.
    pub line_count: usize,
    /// Its debits.
    pub total: Minor,
    /// The engine's issues, the overridable ones marked.
    pub issues: Vec<String>,
}

/// The import preview (`import.tsx`, `parsed`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPreview {
    /// `QuickBooks journal export` or `Xero manual journal`.
    pub source: &'static str,
    /// The drafts with at least one line.
    pub entries: Vec<PreviewEntry>,
    /// Rows that could not be read.
    pub problems: Vec<String>,
    /// Accounts not in the chart.
    pub unmapped: Vec<String>,
    /// Drafts with no issues.
    pub ready: usize,
}

impl ImportPreview {
    /// The preview of `text` with every line on `fund`; `None` with no text or no fund.
    #[must_use]
    pub fn new(text: &str, state: &LedgerState, fund: Option<&FundId>) -> Option<Self> {
        let fund = fund?;
        if text.trim().is_empty() {
            return None;
        }
        let parsed = parse_journal_csv(text, state, fund);
        let entries: Vec<PreviewEntry> = parsed
            .entries
            .into_iter()
            .filter(|e| !e.lines.is_empty())
            .map(|input| {
                let issues = validate(state, &input, None)
                    .iter()
                    .map(issue_text)
                    .collect();
                let total = to_minor(input.lines.iter().filter_map(|l| l.debit).sum());
                PreviewEntry {
                    memo: input.memo.clone(),
                    date: IsoDate::parse(&input.date)
                        .map_or_else(|_| input.date.clone(), |d| format_date(&d)),
                    line_count: input.lines.len(),
                    total,
                    issues,
                    input,
                }
            })
            .collect();
        let ready = entries.iter().filter(|e| e.issues.is_empty()).count();
        Some(Self {
            source: match parsed.source {
                JournalSource::Quickbooks => "QuickBooks journal export",
                JournalSource::Xero => "Xero manual journal",
            },
            entries,
            problems: parsed.problems,
            unmapped: parsed.unmapped_accounts,
            ready,
        })
    }

    /// `QuickBooks journal export · 2 entries · 1 ready to post`.
    #[must_use]
    pub fn meta(&self) -> String {
        format!(
            "{} \u{b7} {} entries \u{b7} {} ready to post",
            self.source,
            self.entries.len(),
            self.ready
        )
    }

    /// `Create 2 drafts`.
    #[must_use]
    pub fn button_label(&self) -> String {
        let n = self.entries.len();
        format!("Create {n} {}", if n == 1 { "draft" } else { "drafts" })
    }

    /// `No entries found` when nothing was read.
    #[must_use]
    pub fn notice(&self) -> Option<(String, String)> {
        self.entries.is_empty().then(|| {
            (
                "No entries found".to_owned(),
                "Check that the file is a QuickBooks Journal report or a Xero manual journal \
                 export with its header row."
                    .to_owned(),
            )
        })
    }

    /// `entry.import` of every draft, or `None` when there are none (`import.tsx`, `create`).
    #[must_use]
    pub fn command(&self) -> Option<LedgerCommand> {
        (!self.entries.is_empty()).then(|| LedgerCommand::EntryImport {
            inputs: self.entries.iter().map(|e| e.input.clone()).collect(),
        })
    }
}

/// The notice after an import (`import.tsx`, `created`).
#[must_use]
pub fn import_done(count: usize) -> (String, String) {
    (
        format!(
            "{count} draft {} created",
            if count == 1 {
                "entry was"
            } else {
                "entries were"
            }
        ),
        "Review each draft, fix anything the ledger flags, then post it.".to_owned(),
    )
}

/// Reads a CSV file for the import, up to [`MAX_IMPORT_BYTES`] (`import.tsx`, `readFile`).
///
/// # Errors
///
/// The message to show when the file is too large or cannot be read.
pub fn read_import_file(path: &Path) -> Result<String, String> {
    let unreadable = || "The file could not be read. Try exporting it again as CSV.".to_owned();
    let size = std::fs::metadata(path).map_err(|_| unreadable())?.len();
    if usize::try_from(size).map_or(true, |s| s > MAX_IMPORT_BYTES) {
        return Err("This file is larger than 5 MB. Export a shorter date range.".to_owned());
    }
    std::fs::read_to_string(path).map_err(|_| unreadable())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::backend::{BackendError, InProcessBackend, LedgerBackend as _};
    use crate::journal::{EntryView, created_entry};
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger};
    use ledger_core::types::{EntryStatus, OrganizationId, SourceType};
    use ledger_core::{CommandResult, LedgerCommand};

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

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
    }

    /// Office supplies 2,500 paid from the main bank account, on the unrestricted fund.
    fn filled(state: &LedgerState) -> EntryForm {
        let mut form = EntryForm::new(state, &as_of());
        form.memo = " Printer paper ".into();
        form.lines[0].account = Some(AccountId::from("acc-7100"));
        form.set_debit(0, "2,500");
        form.lines[1].account = Some(AccountId::from("acc-1110"));
        form.set_credit(1, "2500.00");
        form
    }

    #[test]
    fn a_new_form_starts_on_the_as_of_date_with_two_unrestricted_lines() {
        let state = demo();
        let form = EntryForm::new(&state, &as_of());
        assert_eq!(form.draft, None);
        assert_eq!(form.date, "04/02/2026");
        assert_eq!(form.memo, "");
        assert_eq!(form.lines.len(), 2);
        for l in &form.lines {
            assert_eq!(l.fund, Some(FundId::from("fund-unrestricted")));
            assert_eq!((l.debit.as_str(), l.credit.as_str()), ("", ""));
            assert_eq!(l.account, None);
        }
        assert_ne!(form.lines[0].key, form.lines[1].key);
        assert_eq!(form.title(), "New journal entry");
    }

    #[test]
    fn lines_add_remove_and_keep_one_side() {
        let state = demo();
        let mut form = EntryForm::new(&state, &as_of());
        // Two lines are the least an entry has.
        assert!(!form.remove_line(0));
        form.add_line(&state);
        assert_eq!(form.lines.len(), 3);
        assert_eq!(form.lines[2].fund, Some(FundId::from("fund-unrestricted")));
        assert!(form.remove_line(2));
        assert_eq!(form.lines.len(), 2);

        // Typing a debit clears the credit and the other way round (`updateLine`).
        form.set_credit(0, "10");
        form.set_debit(0, "25");
        assert_eq!(
            (form.lines[0].debit.as_str(), form.lines[0].credit.as_str()),
            ("25", "")
        );
        form.set_credit(0, "30");
        assert_eq!(
            (form.lines[0].debit.as_str(), form.lines[0].credit.as_str()),
            ("", "30")
        );
        // Clearing a side leaves the other alone.
        form.set_debit(0, "");
        assert_eq!(form.lines[0].credit, "30");
    }

    #[test]
    fn a_fund_change_drops_a_grant_of_another_fund() {
        let state = demo();
        let mut form = EntryForm::new(&state, &as_of());
        form.set_fund(&state, 0, Some(FundId::from("fund-education")));
        form.lines[0].grant = Some(GrantId::from("grant-paef-education"));
        form.set_fund(&state, 0, Some(FundId::from("fund-education")));
        assert_eq!(
            form.lines[0].grant,
            Some(GrantId::from("grant-paef-education"))
        );
        form.set_fund(&state, 0, Some(FundId::from("fund-wash")));
        assert_eq!(form.lines[0].grant, None);
    }

    #[test]
    fn the_balance_check_follows_every_edit() {
        let state = demo();
        let mut form = EntryForm::new(&state, &as_of());
        assert_eq!(
            form.balance(&state),
            Balance {
                debits: 0,
                credits: 0,
                difference: 0
            }
        );
        assert_eq!(form.balance(&state).difference_text(), "Balanced");
        form.set_debit(0, "2,500");
        let b = form.balance(&state);
        assert_eq!((b.debits, b.credits, b.difference), (250_000, 0, 250_000));
        assert_eq!(b.difference_text(), "2,500.00");
        assert!(!b.is_balanced());
        form.set_credit(1, "2,000");
        let b = form.balance(&state);
        assert_eq!(b.difference, 50_000);
        assert_eq!(b.debits_text(), "2,500.00");
        assert_eq!(b.credits_text(), "2,000.00");
        // Credits over debits read as the size of the gap.
        form.set_credit(1, "3,000.50");
        let b = form.balance(&state);
        assert_eq!(b.difference, -50_050);
        assert_eq!(b.difference_text(), "500.50");
        form.set_credit(1, "2500");
        assert!(form.balance(&state).is_balanced());
        // Unreadable and negative amounts count as nothing (`Math.max(0, parseMoney ?? 0)`).
        form.set_debit(0, "abc");
        assert_eq!(form.balance(&state).debits, 0);
        form.set_debit(0, "-5");
        assert_eq!(form.balance(&state).debits, 0);
        assert_eq!(form.balance(&state).debits_text(), "0.00");
    }

    #[test]
    fn the_balance_check_converts_kept_foreign_lines() {
        let state = demo();
        let mut form = EntryForm::new(&state, &as_of());
        form.set_debit(0, "100");
        form.lines[0].keep.currency = Some("USD".into());
        form.lines[0].keep.fx_rate = Some(129.455);
        form.set_credit(1, "12,945.50");
        // 10,000 minor x 129.455 = 1,294,550 minor, rounded as Math.round.
        let b = form.balance(&state);
        assert_eq!((b.debits, b.credits), (1_294_550, 1_294_550));
        assert!(b.is_balanced());
        // A line in base currency ignores its rate; a foreign line with no rate counts at 1.
        form.lines[0].keep.currency = Some("KES".into());
        assert_eq!(form.balance(&state).debits, 10_000);
        form.lines[0].keep.currency = Some("USD".into());
        form.lines[0].keep.fx_rate = None;
        assert_eq!(form.balance(&state).debits, 10_000);
    }

    #[test]
    fn a_blank_form_names_what_stops_it() {
        let state = demo();
        let mut form = EntryForm::new(&state, &as_of());
        form.date = String::new();
        form.set_debit(1, "1.2.3");
        form.set_credit(0, "-4");
        let errors = form.field_errors();
        assert_eq!(errors.date.as_deref(), Some("Enter the entry date."));
        assert_eq!(
            errors.memo.as_deref(),
            Some("Describe the entry, as it should read in the journal.")
        );
        assert_eq!(
            errors.line(0, Side::Credit),
            Some("Enter a positive amount; use the other column for the opposite side.")
        );
        assert_eq!(
            errors.line(1, Side::Debit),
            Some("Enter a number, like 1250 or 1,250.50.")
        );
        assert_eq!(errors.line(0, Side::Debit), None);
        let summary = form.summary(&state);
        assert_eq!(
            summary.items[..4],
            [
                "Date: Enter the entry date.".to_owned(),
                "Memo: Describe the entry, as it should read in the journal.".to_owned(),
                "Line 1 credit: Enter a positive amount; use the other column for the opposite \
                 side."
                    .to_owned(),
                "Line 2 debit: Enter a number, like 1250 or 1,250.50.".to_owned(),
            ]
        );
        // The engine's own date and memo messages are left out when the field already says so.
        assert!(
            !summary
                .items
                .iter()
                .any(|i| i == "Enter the entry date." || i == "Describe the entry."),
            "{summary:?}"
        );
        assert_eq!(
            summary.title(),
            format!("{} things stop the entry posting", summary.items.len())
        );
        assert!(matches!(form.save_command(), Err(e) if e == errors));
        form.date = "30/02/2026".into();
        assert_eq!(
            form.field_errors().date.as_deref(),
            Some("February 2026 has only 28 days.")
        );
    }

    #[test]
    fn engine_issues_sit_beside_their_fields() {
        let state = demo();
        let mut form = filled(&state);
        // No account on the second line, a grant of another fund on the first.
        form.lines[1].account = None;
        form.lines[0].grant = Some(GrantId::from("grant-paef-education"));
        let issues = form.issues(&state);
        assert_ne!(issues, []);
        let grant = form.line_issue(&issues, 0, LineField::Grant).unwrap();
        assert!(!grant.starts_with("Line "), "{grant}");
        assert!(form.line_issue(&issues, 1, LineField::Account).is_some());
        let summary = form.summary(&state);
        assert_eq!(summary.items.len(), issues.len());
        assert!(summary.title().ends_with("stop the entry posting"));
    }

    #[test]
    fn a_complete_form_saves_a_draft_and_posts() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let form = filled(&state);
        assert_eq!(form.field_errors(), FieldErrors::default());
        assert_eq!(form.issues(&state), []);
        assert_eq!(form.summary(&state).items, Vec::<String>::new());
        let input = form.input();
        assert_eq!(input.date, "2026-02-04");
        assert_eq!(input.memo, " Printer paper ");
        assert_eq!(input.override_reason, Nullish::Null);
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0].debit, Some(250_000.0));
        assert_eq!(input.lines[0].credit, Some(0.0));
        assert_eq!(input.lines[0].grant_id, Nullish::Null);
        assert_eq!(input.lines[0].dimension_ids, Some(vec![]));
        assert_eq!(input.lines[0].memo, Nullish::Null);
        assert_eq!(input.lines[1].credit, Some(250_000.0));

        let save = form.save_command().unwrap();
        assert_eq!(
            save,
            LedgerCommand::EntrySaveDraft {
                input: input.clone()
            }
        );
        let post = form.post_commands(&state, false).unwrap();
        assert_eq!(post, [LedgerCommand::EntryPost { input }]);
        let mut id = None;
        for c in post {
            id = created_entry(&demo_org.submit(c).unwrap());
        }
        let view = EntryView::new(&demo_org.state(), &id.unwrap()).unwrap();
        assert_eq!(view.entry.status, EntryStatus::Posted);
        assert_eq!(view.entry.memo, "Printer paper");
        assert_eq!(view.entry.source_type, SourceType::Manual);
        assert_eq!((view.debits, view.credits), (250_000, 250_000));
    }

    #[test]
    fn an_unbalanced_form_is_blocked_before_posting() {
        let state = demo();
        let mut form = filled(&state);
        form.set_credit(1, "2,000");
        let blocked = form.post_commands(&state, false).unwrap_err();
        assert_eq!(
            blocked,
            Blocked {
                needs_override: false,
                override_error: None
            }
        );
        let summary = form.summary(&state);
        assert_eq!(summary.items.len(), 1);
        assert!(
            summary.items[0].contains("Debits and credits differ"),
            "{summary:?}"
        );
        assert_eq!(summary.title(), "This stops the entry posting");
        // A draft can still be saved; it is checked again when posted.
        assert!(form.save_command().is_ok());
    }

    #[test]
    fn grant_rules_ask_for_an_override() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let mut form = filled(&state);
        form.date = "20/01/2026".into();
        for i in 0..2 {
            form.set_fund(&state, i, Some(FundId::from("fund-education")));
            form.lines[i].grant = Some(GrantId::from("grant-paef-education"));
        }
        let blocked = form.post_commands(&state, false).unwrap_err();
        assert!(blocked.needs_override);
        let summary = form.summary(&state);
        assert!(
            summary.items[0].ends_with(" (can be overridden with a reason)"),
            "{summary:?}"
        );
        // An override needs its reason.
        form.override_reason = "  ".into();
        assert_eq!(
            form.post_commands(&state, true)
                .unwrap_err()
                .override_error
                .as_deref(),
            Some("Say why this entry should post despite the rule.")
        );
        form.override_reason = " Donor agreed ".into();
        let commands = form.post_commands(&state, true).unwrap();
        let [LedgerCommand::EntryPost { input }] = commands.as_slice() else {
            panic!("{commands:?}");
        };
        assert_eq!(input.override_reason, Nullish::Value("Donor agreed".into()));
        let id = created_entry(&demo_org.submit(commands[0].clone()).unwrap()).unwrap();
        let view = EntryView::new(&demo_org.state(), &id).unwrap();
        assert_eq!(view.override_reason.as_deref(), Some("Donor agreed"));
    }

    #[test]
    fn a_draft_opens_in_the_form_and_saves_back() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let mut form = filled(&state);
        form.lines[0].memo = "A4, 10 reams".into();
        form.lines[0].dimension = Some(DimensionId::from("dim-a31"));
        let id = created_entry(&demo_org.submit(form.save_command().unwrap()).unwrap()).unwrap();
        let state = demo_org.state();

        let mut edit = EntryForm::from_draft(&state, &id).unwrap();
        assert_eq!(edit.draft, Some(id.clone()));
        assert_eq!(edit.title(), "Edit draft entry");
        assert_eq!(edit.date, "04/02/2026");
        assert_eq!(edit.memo, "Printer paper");
        assert_eq!(edit.lines.len(), 2);
        assert_eq!(edit.lines[0].debit, "2500.00");
        assert_eq!(edit.lines[0].credit, "");
        assert_eq!(edit.lines[1].credit, "2500.00");
        assert_eq!(edit.lines[0].memo, "A4, 10 reams");
        assert_eq!(edit.lines[0].dimension, Some(DimensionId::from("dim-a31")));
        assert_eq!(edit.lines[0].keep.currency.as_deref(), Some("KES"));
        assert_eq!(edit.lines[0].keep.fx_rate, Some(1.0));

        edit.memo = "Printer paper, February".into();
        let save = edit.save_command().unwrap();
        assert!(
            matches!(&save, LedgerCommand::EntryUpdateDraft { entry_id, .. } if *entry_id == id)
        );
        demo_org.submit(save).unwrap();
        let state = demo_org.state();
        assert_eq!(
            EntryView::new(&state, &id).unwrap().entry.memo,
            "Printer paper, February"
        );
        // Posting a draft from the form saves it, then posts it.
        let edit = EntryForm::from_draft(&state, &id).unwrap();
        let commands = edit.post_commands(&state, false).unwrap();
        assert!(matches!(
            commands.as_slice(),
            [
                LedgerCommand::EntryUpdateDraft { .. },
                LedgerCommand::EntryPostDraft {
                    override_reason: None,
                    ..
                }
            ]
        ));
        for c in commands {
            demo_org.submit(c).unwrap();
        }
        let state = demo_org.state();
        assert_eq!(
            EntryView::new(&state, &id).unwrap().entry.status,
            EntryStatus::Posted
        );

        // A posted entry does not open in the form.
        let notice = EntryForm::from_draft(&state, &id).unwrap_err();
        assert_eq!(
            notice.title,
            format!(
                "{} is posted",
                EntryView::new(&state, &id).unwrap().entry.number
            )
        );
        assert_eq!(
            notice.body,
            "Posted entries cannot be edited. Reverse it and post a correcting entry instead."
        );
        let gone = EntryForm::from_draft(&state, &EntryId::from("nope")).unwrap_err();
        assert_eq!(gone.title, "This draft is not in the books");
    }

    const QUICKBOOKS: &str = "Date,Transaction Type,Num,Name,Memo/Description,Account,Debit,Credit\n\
        01/15/2026,Journal Entry,12,,Office rent January,7500 Rent,\"45,000.00\",\n\
        ,,,,,Bank: Main operating account,,\"45,000.00\"\n\
        20/01/2026,Journal Entry,13,,Unknown,9999 Mystery,100,";

    const XERO: &str = "*Narration,*Date,Description,*AccountCode,*TaxRate,*Amount\n\
        Audit fee,31/01/2026,Audit,7600,Tax Exempt,25000\n\
        Audit fee,31/01/2026,Audit,2200,Tax Exempt,-25000";

    #[test]
    fn an_import_preview_lists_drafts_and_their_problems() {
        let state = demo();
        let fund = FundId::from("fund-unrestricted");
        assert_eq!(ImportPreview::new("  ", &state, Some(&fund)), None);
        assert_eq!(ImportPreview::new(XERO, &state, None), None);

        let p = ImportPreview::new(QUICKBOOKS, &state, Some(&fund)).unwrap();
        assert_eq!(p.source, "QuickBooks journal export");
        assert_eq!(p.unmapped, ["9999 Mystery"]);
        assert_eq!(p.problems, Vec::<String>::new());
        // A journal left with no lines is not offered (`entries.filter(lines.length > 0)`).
        assert_eq!(p.entries.len(), 1);
        let e = &p.entries[0];
        assert_eq!(e.memo, "Office rent January");
        assert_eq!(e.date, "15 Jan 2026");
        assert_eq!(e.line_count, 2);
        assert_eq!(e.total, 4_500_000);
        assert_eq!(e.issues, Vec::<String>::new());
        assert_eq!(p.ready, 1);
        assert_eq!(
            p.meta(),
            "QuickBooks journal export \u{b7} 1 entries \u{b7} 1 ready to post"
        );
        assert_eq!(p.button_label(), "Create 1 draft");
        assert_eq!(p.notice(), None);
        assert_eq!(
            p.command(),
            Some(LedgerCommand::EntryImport {
                inputs: vec![p.entries[0].input.clone()]
            })
        );

        let x = ImportPreview::new(XERO, &state, Some(&fund)).unwrap();
        assert_eq!(x.source, "Xero manual journal");
        assert_eq!(x.entries.len(), 1);
        assert_eq!(x.entries[0].total, 2_500_000);
    }

    #[test]
    fn import_rows_with_problems_show_them() {
        let state = demo();
        let fund = FundId::from("fund-unrestricted");
        let text = "Date,Num,Memo,Account,Debit,Credit\n\
            ,7,Accrual,1110,12.345,\n\
            ,,,Rent,,100\n\
            02/02/2026,8,Fuel,7200,\"1,000.00\",\n\
            ,,,1110 Bank: Main operating account,,\"1,000.00\"";
        let p = ImportPreview::new(text, &state, Some(&fund)).unwrap();
        assert_eq!(
            p.problems,
            [
                "Row 2: unreadable amount.",
                "\u{201c}Imported journal\u{201d} has no date."
            ]
        );
        let bad = ImportPreview::new(
            "Receipt No.,Completion Time,Details\nX,2026-01-01,Y",
            &state,
            Some(&fund),
        )
        .unwrap();
        assert_eq!(bad.entries.len(), 0);
        assert_eq!(bad.command(), None);
        assert_eq!(
            bad.notice(),
            Some((
                "No entries found".to_owned(),
                "Check that the file is a QuickBooks Journal report or a Xero manual journal \
                 export with its header row."
                    .to_owned()
            ))
        );

        // An entry the engine would refuse still imports as a draft, with its issues listed.
        let unbalanced = "*Narration,*Date,Description,*AccountCode,*TaxRate,*Amount\n\
            Audit fee,31/01/2026,Audit,7600,Tax Exempt,25000\n\
            Audit fee,31/01/2026,Audit,2200,Tax Exempt,-20000";
        let p = ImportPreview::new(unbalanced, &state, Some(&fund)).unwrap();
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.ready, 0);
        assert_eq!(p.entries[0].issues.len(), 1);
        assert!(
            p.entries[0].issues[0].contains("Debits and credits differ"),
            "{:?}",
            p.entries[0].issues
        );
    }

    #[test]
    fn importing_creates_drafts_in_one_command() {
        let mut demo_org = Demo::new();
        let state = demo_org.state();
        let fund = FundId::from("fund-unrestricted");
        let p = ImportPreview::new(XERO, &state, Some(&fund)).unwrap();
        demo_org.submit(p.command().unwrap()).unwrap();
        let state = demo_org.state();
        let imported: Vec<_> = state
            .entries
            .iter()
            .filter(|e| e.source_type == SourceType::Import)
            .collect();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].status, EntryStatus::Draft);
        assert_eq!(
            import_done(1),
            (
                "1 draft entry was created".to_owned(),
                "Review each draft, fix anything the ledger flags, then post it.".to_owned()
            )
        );
        assert_eq!(import_done(3).0, "3 draft entries were created");
    }

    #[test]
    fn an_import_file_is_read_with_a_size_limit() {
        let dir = std::env::temp_dir().join(format!("u3-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("journal.csv");
        std::fs::write(&path, XERO).unwrap();
        assert_eq!(read_import_file(&path).unwrap(), XERO);
        assert_eq!(
            read_import_file(&dir.join("missing.csv")).unwrap_err(),
            "The file could not be read. Try exporting it again as CSV."
        );
        let big = dir.join("big.csv");
        std::fs::write(&big, vec![b'a'; MAX_IMPORT_BYTES + 1]).unwrap();
        assert_eq!(
            read_import_file(&big).unwrap_err(),
            "This file is larger than 5 MB. Export a shorter date range."
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
