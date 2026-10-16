//! View models of the Spend screen's claims tab: the claims list with its status filter, the
//! new claim form with its coded lines, and the claim detail with its trail, the actions its
//! status allows, and the posted entry with the grants it charged. Plain structs, no GPUI; the
//! panel in `panels::spend` only draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/spend/claims.index.tsx`,
//! `claims.new.tsx`, `claims.$id.tsx` and `-shared.tsx`; the rules themselves stay in
//! `ledger-core` (`engine/claims.rs`), whose messages are shown as they come.

use ledger_core::types::{
    AccountId, Actor, ActorId, ClaimId, ClaimStatus, CodedLine, DimensionId, DimensionKind,
    EntryId, ExpenseClaim, FundId, GrantId, IsoDate, LedgerState, Timestamp,
};
use ledger_core::{LedgerCommand, Minor};

use crate::backend::BackendError;

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
pub fn claim_status_label(_status: ClaimStatus) -> &'static str {
    ""
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
pub fn claim_tone(_status: ClaimStatus) -> Tone {
    Tone::Quiet
}

/// `docTotal` (`-shared.tsx:133`): the sum of the lines.
#[must_use]
pub fn doc_total(_lines: &[CodedLine]) -> Minor {
    0
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

/// The claims `filter` shows, latest claim date first, then latest number
/// (`claims.index.tsx:51-58`).
#[must_use]
pub fn claim_rows(_state: &LedgerState, _filter: Option<ClaimStatus>) -> Vec<ClaimRow> {
    Vec::new()
}

/// Each filter with the number of claims it shows (`claims.index.tsx:42-49`).
#[must_use]
pub fn filter_counts(_state: &LedgerState) -> Vec<(ClaimFilter, usize)> {
    Vec::new()
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
pub fn list_notice(_state: &LedgerState, _filter: Option<ClaimStatus>) -> Option<Notice> {
    None
}

/// `isoToDayMonthYear` (`flowtrace/src/components/ui/date-field.tsx:36-39`): `04/02/2026`.
#[must_use]
pub fn iso_to_day_month_year(_date: &IsoDate) -> String {
    String::new()
}

/// `parseDayMonthYear` (`date-field.tsx:50-80`): `None` for a blank field, otherwise the date
/// or the message to show.
#[must_use]
pub fn parse_day_month_year(_text: &str) -> Option<Result<IsoDate, String>> {
    None
}

/// `AccountSelect types={['expense']}` (`flowtrace/src/components/ledger/pickers.tsx:25-44`):
/// active, non-header expense accounts in code order, as `(id, "code · name")`.
#[must_use]
pub fn cost_accounts(_state: &LedgerState) -> Vec<(AccountId, String)> {
    Vec::new()
}

/// `AccountSelect cashOnly` (`pickers.tsx:25-44`): active, non-header cash accounts in code
/// order.
#[must_use]
pub fn cash_accounts(_state: &LedgerState) -> Vec<(AccountId, String)> {
    Vec::new()
}

/// `FundSelect` (`pickers.tsx:46-54`): every fund in the books' order.
#[must_use]
pub fn fund_choices(_state: &LedgerState) -> Vec<(FundId, String)> {
    Vec::new()
}

/// `GrantSelect fundId` (`pickers.tsx:57-69`) without its `No grant` choice: the grants on
/// `fund`, or every grant when no fund is chosen.
#[must_use]
pub fn grant_choices(_state: &LedgerState, _fund: Option<&FundId>) -> Vec<(GrantId, String)> {
    Vec::new()
}

/// The dimension values of `kind` (`-shared.tsx:381-388`), for the project and place pickers.
#[must_use]
pub fn dimension_choices(_state: &LedgerState, _kind: DimensionKind) -> Vec<(DimensionId, String)> {
    Vec::new()
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
    pub fn new(_state: &LedgerState) -> Self {
        Self::default()
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
        ""
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
    pub fn line(&self, _index: usize, _field: LineField) -> Option<&str> {
        None
    }

    /// `ErrorSummary` (`-shared.tsx:552-597`): `Fix this before saving` or `Fix these before
    /// saving`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        ""
    }

    /// The summary lines, `Label: message`, date and purpose first.
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        Vec::new()
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
}

impl ClaimForm {
    /// The form on `as_of` with one blank line (`claims.new.tsx:50-63`).
    #[must_use]
    pub fn new(_state: &LedgerState, _as_of: &IsoDate) -> Self {
        Self {
            date: String::new(),
            purpose: String::new(),
            lines: Vec::new(),
        }
    }

    /// `Add another line` (`-shared.tsx:516-534`): a blank line carrying the last line's
    /// account, fund, grant, project and place.
    pub fn add_line(&mut self, _state: &LedgerState) {}

    /// Removes line `index` when more than one is left (`-shared.tsx:404-415`).
    pub fn remove_line(&mut self, _index: usize) -> bool {
        false
    }

    /// `withFund` (`-shared.tsx:356-361`): a new fund drops a grant from another fund and picks
    /// the only grant when there is one.
    pub fn set_fund(&mut self, _state: &LedgerState, _index: usize, _fund: Option<FundId>) {}

    /// `linesTotal` (`-shared.tsx:351-353`): the amounts that parse, negatives as zero.
    #[must_use]
    pub fn total(&self) -> Minor {
        0
    }

    /// The `claim.create` command, submitting the claim when `submit` is set, or what to fix
    /// first (`claims.new.tsx:68-113`). The id is left to the engine's env.
    ///
    /// # Errors
    ///
    /// [`FormErrors`] for a missing or late date, a blank purpose or a line that is not
    /// complete (`validateLines`, `-shared.tsx:322-345`).
    pub fn command(
        &self,
        _state: &LedgerState,
        _as_of: &IsoDate,
        _submit: bool,
    ) -> Result<LedgerCommand, FormErrors> {
        Err(FormErrors::default())
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
    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        ""
    }

    /// The notice once it is done (`claims.$id.tsx:127-133`, 377-397).
    #[must_use]
    pub fn done_notice(self) -> &'static str {
        ""
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

impl ClaimDetail {
    /// The detail of claim `id` as `actor` sees it, or `None` when it is not in the books.
    #[must_use]
    pub fn new(_state: &LedgerState, _id: &ClaimId, _actor: Option<&Actor>) -> Option<Self> {
        None
    }

    /// The lede: `EC-0002 · Achieng Odhiambo · spent 3 Feb 2026` (`claims.$id.tsx:57-63`).
    #[must_use]
    pub fn lede(&self) -> String {
        String::new()
    }

    /// The `claim.submit` command.
    #[must_use]
    pub fn submit_command(&self) -> LedgerCommand {
        LedgerCommand::ClaimSubmit {
            claim_id: ClaimId::from(""),
        }
    }

    /// The `claim.approve` command with the note trimmed (none when blank) and an override
    /// reason (`claims.$id.tsx:368-375`).
    #[must_use]
    pub fn approve_command(&self, _note: &str, _override_reason: Option<&str>) -> LedgerCommand {
        LedgerCommand::ClaimSubmit {
            claim_id: ClaimId::from(""),
        }
    }

    /// The `claim.reject` command (`claims.$id.tsx:389-396`).
    ///
    /// # Errors
    ///
    /// The message under the reason when it is blank.
    pub fn reject_command(&self, _note: &str) -> Result<LedgerCommand, String> {
        Err(String::new())
    }

    /// The confirmation of `action`, `None` for a submit, which needs none.
    #[must_use]
    pub fn dialog(&self, _action: ClaimAction) -> Option<Dialog> {
        None
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
        String::new()
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
    pub fn new(_as_of: &IsoDate) -> Self {
        Self {
            date: String::new(),
            cash_account: None,
            reference: String::new(),
        }
    }

    /// The `claim.reimburse` command (`claims.$id.tsx:419-428`).
    ///
    /// # Errors
    ///
    /// [`ReimburseErrors`] for a missing date or account.
    pub fn command(&self, _claim: &ClaimId) -> Result<LedgerCommand, ReimburseErrors> {
        Err(ReimburseErrors::default())
    }
}

/// `hasOverridable` (`flowtrace/src/components/ledger/issues.ts`): a ledger refusal whose
/// every issue an override reason lets through.
#[must_use]
pub fn has_overridable(_error: &BackendError) -> bool {
    false
}

/// `personName` (`-shared.tsx:139-147`): the viewer as `Name (you)`, the claimant by name,
/// `system` as FlowTrace, anyone else by id.
#[must_use]
pub fn person_name(_id: &ActorId, _actor: Option<&Actor>, _claimant: (&ActorId, &str)) -> String {
    String::new()
}

/// `formatDateTime` (`flowtrace/src/lib/format.ts:31-39`): `4 Feb 2026, 09:00`, in UTC.
#[must_use]
pub fn format_date_time(_at: &Timestamp) -> String {
    String::new()
}

/// `formatMinor(v, { decimals: 0 })` (`money.tsx:8-21`): whole units with separators, half
/// away from zero, zero as an en dash.
#[must_use]
pub fn format_whole(_minor: Minor) -> String {
    String::new()
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
        assert_eq!(err("31/02/2026"), "Feb 2026 has only 28 days.");
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
            Some("Feb 2026 has only 28 days.")
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
