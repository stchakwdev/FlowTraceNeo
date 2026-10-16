//! View models of the Journal screen's manual entry form and CSV journal import: the lines as
//! typed with the live balance check, the field and engine problems beside their fields, the
//! commands Save draft and Post send, and the import preview with each draft's issues. Plain
//! structs, no GPUI; `panels::journal` draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/ledger/new.tsx` and
//! `import.tsx`; the rules stay in `ledger-core` (`engine/validate.rs`, `importers.rs`).

use std::path::Path;

use ledger_core::types::{
    AccountId, DimensionId, EntryId, EntryInput, FlowNodeId, FundId, GrantId, IsoDate, LedgerState,
    Nullish,
};
use ledger_core::{Issue, LedgerCommand, Minor};

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

/// A side of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Debit.
    Debit,
    /// Credit.
    Credit,
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
    pub fn line(&self, _index: usize, _side: Side) -> Option<&str> {
        None
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

impl Balance {
    /// True when debits equal credits.
    #[must_use]
    pub fn is_balanced(&self) -> bool {
        false
    }

    /// Debits, `0.00` for none.
    #[must_use]
    pub fn debits_text(&self) -> String {
        String::new()
    }

    /// Credits, `0.00` for none.
    #[must_use]
    pub fn credits_text(&self) -> String {
        String::new()
    }

    /// `Balanced`, or the size of the gap.
    #[must_use]
    pub fn difference_text(&self) -> String {
        String::new()
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
        String::new()
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

impl EntryForm {
    /// A new entry dated `as_of` with two blank lines on the default fund.
    #[must_use]
    pub fn new(_state: &LedgerState, _as_of: &IsoDate) -> Self {
        Self {
            draft: None,
            date: String::new(),
            memo: String::new(),
            override_reason: String::new(),
            lines: Vec::new(),
            next_key: 0,
        }
    }

    /// The draft `id` loaded into the form (`new.tsx`, the load effect).
    ///
    /// # Errors
    ///
    /// The notice to show when `id` is not a draft in the books.
    pub fn from_draft(_state: &LedgerState, _id: &EntryId) -> Result<Self, Notice> {
        Err(Notice {
            title: String::new(),
            body: String::new(),
        })
    }

    /// `New journal entry` or `Edit draft entry`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        ""
    }

    /// Adds a blank line on the default fund.
    pub fn add_line(&mut self, _state: &LedgerState) {}

    /// Removes line `index` while more than two remain.
    pub fn remove_line(&mut self, _index: usize) -> bool {
        false
    }

    /// Sets line `index`'s fund, dropping a grant of another fund.
    pub fn set_fund(&mut self, _state: &LedgerState, _index: usize, _fund: Option<FundId>) {}

    /// Sets a debit; a non-empty one clears the credit.
    pub fn set_debit(&mut self, _index: usize, _text: &str) {}

    /// Sets a credit; a non-empty one clears the debit.
    pub fn set_credit(&mut self, _index: usize, _text: &str) {}

    /// The form's own problems.
    #[must_use]
    pub fn field_errors(&self) -> FieldErrors {
        FieldErrors::default()
    }

    /// The entry as the engine takes it (`new.tsx`, `input`).
    #[must_use]
    pub fn input(&self) -> EntryInput {
        EntryInput {
            date: String::new(),
            memo: String::new(),
            lines: Vec::new(),
            source_type: None,
            source_id: Nullish::Undefined,
            override_reason: Nullish::Undefined,
        }
    }

    /// The live balance check.
    #[must_use]
    pub fn balance(&self, _state: &LedgerState) -> Balance {
        Balance::default()
    }

    /// The engine's issues with the entry (`ledger.validateEntry`).
    #[must_use]
    pub fn issues(&self, _state: &LedgerState) -> Vec<Issue> {
        Vec::new()
    }

    /// The engine's message for line `index`'s `field`, without its `Line n:` prefix.
    #[must_use]
    pub fn line_issue(
        &self,
        _issues: &[Issue],
        _index: usize,
        _field: LineField,
    ) -> Option<String> {
        None
    }

    /// The summary box.
    #[must_use]
    pub fn summary(&self, _state: &LedgerState) -> Summary {
        Summary::default()
    }

    /// Save draft: `entry.saveDraft`, or `entry.updateDraft` for a loaded draft.
    ///
    /// # Errors
    ///
    /// The form's own problems.
    pub fn save_command(&self) -> Result<LedgerCommand, FieldErrors> {
        Err(FieldErrors::default())
    }

    /// Post: `entry.post`, or for a loaded draft `entry.updateDraft` then `entry.postDraft`.
    ///
    /// # Errors
    ///
    /// Why nothing is sent.
    pub fn post_commands(
        &self,
        _state: &LedgerState,
        _with_override: bool,
    ) -> Result<Vec<LedgerCommand>, Blocked> {
        Err(Blocked::default())
    }
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
    pub fn new(_text: &str, _state: &LedgerState, _fund: Option<&FundId>) -> Option<Self> {
        None
    }

    /// `QuickBooks journal export · 2 entries · 1 ready to post`.
    #[must_use]
    pub fn meta(&self) -> String {
        String::new()
    }

    /// `Create 2 drafts`.
    #[must_use]
    pub fn button_label(&self) -> String {
        String::new()
    }

    /// `No entries found` when nothing was read.
    #[must_use]
    pub fn notice(&self) -> Option<(String, String)> {
        None
    }

    /// `entry.import` of every draft, or `None` when there are none.
    #[must_use]
    pub fn command(&self) -> Option<LedgerCommand> {
        None
    }
}

/// The notice after an import (`import.tsx`, `created`).
#[must_use]
pub fn import_done(_count: usize) -> (String, String) {
    (String::new(), String::new())
}

/// Reads a CSV file for the import, up to [`MAX_IMPORT_BYTES`] (`import.tsx`, `readFile`).
///
/// # Errors
///
/// The message to show when the file is too large or cannot be read.
pub fn read_import_file(_path: &Path) -> Result<String, String> {
    Err(String::new())
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
        assert!(!issues.is_empty());
        let grant = form.line_issue(&issues, 0, LineField::Grant).unwrap();
        assert!(!grant.starts_with("Line "), "{grant}");
        assert!(form.line_issue(&issues, 1, LineField::Account).is_some());
        let summary = form.summary(&state);
        assert_eq!(summary.items.len(), issues.len());
        assert_eq!(summary.title().ends_with("stop the entry posting"), true);
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
        assert!(summary.items[0].contains("balance"), "{summary:?}");
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
            p.entries[0].issues[0].contains("balance"),
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
