//! View models of the Accounts screen: the chart of accounts as a tree with balances, the add
//! and edit forms, the fiscal years and their periods, the period actions, and the mapping of a
//! refused command to the lines shown beside the form. Plain structs, no GPUI; the panel in
//! `panels::accounts` only draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/ledger/accounts.tsx`
//! and `periods.tsx`; the rules themselves stay in `ledger-core`, whose messages are shown as
//! they come.

use std::collections::{BTreeMap, HashMap, HashSet};

use ledger_core::engine::{AccountPatch, NewAccount};
use ledger_core::reports::{LineFilter, balances, natural};
use ledger_core::types::{
    Account, AccountId, AccountType, EntryStatus, ExpenseFunction, FiscalPeriod, IsoDate,
    NormalBalance, Nullish, PeriodId, PeriodStatus, SourceType,
};
use ledger_core::{CommandError, LedgerCommand, LedgerState, Minor, engine};

use crate::backend::BackendError;

/// The account types in the order the add form lists them (`accounts.tsx:27-33`).
pub const ACCOUNT_TYPES: [AccountType; 5] = [
    AccountType::Asset,
    AccountType::Liability,
    AccountType::Equity,
    AccountType::Income,
    AccountType::Expense,
];

/// The expense functions in the order the add form lists them (`accounts.tsx:35-39`).
pub const EXPENSE_FUNCTIONS: [ExpenseFunction; 3] = [
    ExpenseFunction::Program,
    ExpenseFunction::Management,
    ExpenseFunction::Fundraising,
];

/// The en dash the web app shows for an empty cell (`money.tsx:12`).
const DASH: &str = "\u{2013}";

/// The middle dot between parts of a meta line.
const DOT: &str = "\u{b7}";

/// `TYPE_LABEL` (`accounts.tsx:27-33`).
#[must_use]
pub fn type_label(t: AccountType) -> &'static str {
    match t {
        AccountType::Asset => "Asset",
        AccountType::Liability => "Liability",
        AccountType::Equity => "Net assets",
        AccountType::Income => "Income",
        AccountType::Expense => "Expense",
    }
}

/// `FUNCTION_LABEL` (`accounts.tsx:35-39`).
#[must_use]
pub fn function_label(f: ExpenseFunction) -> &'static str {
    match f {
        ExpenseFunction::Program => "Programme",
        ExpenseFunction::Management => "Management and general",
        ExpenseFunction::Fundraising => "Fundraising",
    }
}

/// The side that increases an account, for the normal balance column.
#[must_use]
pub fn normal_balance_label(n: NormalBalance) -> &'static str {
    match n {
        NormalBalance::Debit => "Debit",
        NormalBalance::Credit => "Credit",
    }
}

/// `formatMinor` (`flowtrace/src/components/ledger/money.tsx:8-21`) with its defaults: two
/// decimals with thousands separators (`en-KE` groups by three with commas), negatives in
/// parentheses, zero as an en dash. Minor units are whole cents, so the digits are exact.
#[must_use]
pub fn format_minor(minor: Minor) -> String {
    if minor == 0 {
        return DASH.to_owned();
    }
    let abs = minor.unsigned_abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let text = format!("{grouped}.{:02}", abs % 100);
    if minor < 0 { format!("({text})") } else { text }
}

/// The short English month names `en-GB` dates use.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Day and month name of a `YYYY-MM-DD` date, without leading zero on the day.
fn day_month(date: &IsoDate) -> (String, &'static str, &str) {
    let s = date.as_str();
    let (year, rest) = s.split_at(4);
    let month: usize = rest.get(1..3).and_then(|m| m.parse().ok()).unwrap_or(1);
    let day = rest.get(4..6).unwrap_or("01").trim_start_matches('0');
    (day.to_owned(), MONTHS[month.clamp(1, 12) - 1], year)
}

/// `formatDate` (`flowtrace/src/lib/format.ts:12-19`): `31 Jan 2026`.
#[must_use]
pub fn format_date(date: &IsoDate) -> String {
    let (day, month, year) = day_month(date);
    format!("{day} {month} {year}")
}

/// `formatShortDate` (`flowtrace/src/lib/format.ts:22-28`): `1 Jan`.
#[must_use]
pub fn format_short_date(date: &IsoDate) -> String {
    let (day, month, _) = day_month(date);
    format!("{day} {month}")
}

/// One row of the chart: an account at its depth in the tree with its balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    /// The account.
    pub account: Account,
    /// 0 for a top-level account.
    pub depth: usize,
    /// Own balance plus every account below, the natural way round (`accounts.tsx:43-68`,
    /// `natural`).
    pub balance: Minor,
    /// One of the settings' system accounts, which cannot be deactivated.
    pub is_system: bool,
}

impl AccountRow {
    /// `AccountTags` (`accounts.tsx:226-235`): Header, Cash, System, Inactive.
    #[must_use]
    pub fn tags(&self) -> Vec<&'static str> {
        let a = &self.account;
        [
            (a.is_group, "Header"),
            (a.is_cash, "Cash"),
            (self.is_system, "System"),
            (!a.is_active, "Inactive"),
        ]
        .into_iter()
        .filter_map(|(on, tag)| on.then_some(tag))
        .collect()
    }

    /// `ActiveToggle` (`accounts.tsx:237-264`): none for a system account.
    #[must_use]
    pub fn toggle_label(&self) -> Option<&'static str> {
        (!self.is_system).then_some(if self.account.is_active {
            "Deactivate"
        } else {
            "Reactivate"
        })
    }

    /// The function column: the label, or an en dash.
    #[must_use]
    pub fn function_text(&self) -> &'static str {
        self.account.expense_function.map_or(DASH, function_label)
    }

    /// The balance column.
    #[must_use]
    pub fn balance_text(&self) -> String {
        format_minor(self.balance)
    }

    /// The command of the row's Deactivate or Reactivate button (`accounts.tsx:80-90`).
    #[must_use]
    pub fn toggle_command(&self) -> LedgerCommand {
        LedgerCommand::AccountUpdate {
            account_id: self.account.id.clone(),
            patch: AccountPatch {
                is_active: Some(!self.account.is_active),
                ..AccountPatch::default()
            },
        }
    }
}

/// `buildTree` (`accounts.tsx:43-68`): accounts under their parent in code order, depth first;
/// an account whose parent is missing sits at the top level. Balances are posted lines up to
/// `as_of` (all of them when `None`). Codes compare by bytes, which is what `localeCompare`
/// gives for the digit codes of a chart (project memory, fixtures and TS engine facts).
#[must_use]
pub fn account_rows(state: &LedgerState, as_of: Option<&IsoDate>) -> Vec<AccountRow> {
    let posted = balances(
        state,
        &LineFilter {
            to: as_of.cloned(),
            ..LineFilter::default()
        },
    );
    let ids: HashSet<&AccountId> = state.accounts.iter().map(|a| &a.id).collect();
    let mut children: HashMap<Option<&AccountId>, Vec<&Account>> = HashMap::new();
    for a in &state.accounts {
        let key = a.parent_id.as_ref().filter(|p| ids.contains(p));
        children.entry(key).or_default().push(a);
    }
    for list in children.values_mut() {
        list.sort_by(|x, y| x.code.cmp(&y.code));
    }
    let system = system_ids(state);

    let mut out = Vec::with_capacity(state.accounts.len());
    walk(None, 0, &children, &posted, &system, &mut out);
    out
}

/// One level of `buildTree`'s walk: pushes the children of `parent` and everything below them,
/// and returns their summed balance.
fn walk<'a>(
    parent: Option<&'a AccountId>,
    depth: usize,
    children: &HashMap<Option<&'a AccountId>, Vec<&'a Account>>,
    posted: &BTreeMap<AccountId, Minor>,
    system: &HashSet<&AccountId>,
    out: &mut Vec<AccountRow>,
) -> Minor {
    let mut total = 0;
    for a in children.get(&parent).map_or(&[][..], Vec::as_slice) {
        let index = out.len();
        out.push(AccountRow {
            account: (*a).clone(),
            depth,
            balance: 0,
            is_system: system.contains(&a.id),
        });
        let own = posted.get(&a.id).copied().unwrap_or(0);
        let below = walk(Some(&a.id), depth + 1, children, posted, system, out);
        let balance = natural(a.normal_balance, own) + below;
        out[index].balance = balance;
        total += balance;
    }
    total
}

/// The settings' system accounts (`accounts.tsx:72-75`).
fn system_ids(state: &LedgerState) -> HashSet<&AccountId> {
    let s = &state.settings.system_accounts;
    [
        &s.accounts_payable,
        &s.claims_payable,
        &s.suspense,
        &s.net_assets_unrestricted,
        &s.net_assets_restricted,
        &s.fx_gain_loss,
    ]
    .into_iter()
    .collect()
}

/// The section meta: `54 accounts`, or `54 accounts · 2 inactive` (`accounts.tsx:117-119`).
#[must_use]
pub fn accounts_meta(rows: &[AccountRow]) -> String {
    let inactive = rows.iter().filter(|r| !r.account.is_active).count();
    if inactive == 0 {
        format!("{} accounts", rows.len())
    } else {
        format!("{} accounts {DOT} {inactive} inactive", rows.len())
    }
}

/// What the add form found wrong before sending (`accounts.tsx:312-318`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldErrors {
    /// Under the code field.
    pub code: Option<String>,
    /// Under the name field.
    pub name: Option<String>,
}

/// The add form (`accounts.tsx:266-276`, `EMPTY`): an expense account in programme by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccountForm {
    /// Code as typed.
    pub code: String,
    /// Name as typed.
    pub name: String,
    /// Type.
    pub account_type: AccountType,
    /// Header account above it; `None` for the top level.
    pub parent: Option<AccountId>,
    /// Header account.
    pub is_group: bool,
    /// Cash account; only kept for a non-header asset.
    pub is_cash: bool,
    /// Function; only kept for a non-header expense.
    pub expense_function: ExpenseFunction,
}

impl Default for NewAccountForm {
    fn default() -> Self {
        Self {
            code: String::new(),
            name: String::new(),
            account_type: AccountType::Expense,
            parent: None,
            is_group: false,
            is_cash: false,
            expense_function: ExpenseFunction::Program,
        }
    }
}

impl NewAccountForm {
    /// Changes the type; the parent resets, as a parent must have the same type
    /// (`accounts.tsx:298-303`).
    pub fn set_type(&mut self, t: AccountType) {
        self.account_type = t;
        self.parent = None;
    }

    /// The parent choices: header accounts of the form's type in code order, as
    /// `(id, "code · name")` (`accounts.tsx:294-296`).
    #[must_use]
    pub fn parents(&self, state: &LedgerState) -> Vec<(AccountId, String)> {
        let mut headers: Vec<&Account> = state
            .accounts
            .iter()
            .filter(|a| a.is_group && a.account_type == self.account_type)
            .collect();
        headers.sort_by(|x, y| x.code.cmp(&y.code));
        headers
            .into_iter()
            .map(|a| (a.id.clone(), format!("{} {DOT} {}", a.code, a.name)))
            .collect()
    }

    /// The cash checkbox shows for a non-header asset (`accounts.tsx:398`).
    #[must_use]
    pub fn shows_cash(&self) -> bool {
        self.account_type == AccountType::Asset && !self.is_group
    }

    /// The function choice shows for a non-header expense (`accounts.tsx:408`).
    #[must_use]
    pub fn shows_function(&self) -> bool {
        self.account_type == AccountType::Expense && !self.is_group
    }

    /// The `account.add` command, or what to fix first (`accounts.tsx:312-336`). A code
    /// already in use is left to the engine (`engine.ts` `addAccount`), so its message is the
    /// one shown; the web form checks it itself with its own wording.
    ///
    /// # Errors
    ///
    /// [`FieldErrors`] when the code or the name is blank.
    pub fn command(&self) -> Result<LedgerCommand, FieldErrors> {
        let code = self.code.trim();
        let name = self.name.trim();
        let errors = FieldErrors {
            code: code
                .is_empty()
                .then(|| "Enter a code, such as 5150.".to_owned()),
            name: name
                .is_empty()
                .then(|| "Enter the account name.".to_owned()),
        };
        if errors != FieldErrors::default() {
            return Err(errors);
        }
        Ok(LedgerCommand::AccountAdd {
            input: NewAccount {
                id: None,
                code: code.to_owned(),
                name: name.to_owned(),
                account_type: self.account_type,
                parent_id: self.parent.clone(),
                is_group: self.is_group,
                is_active: None,
                is_cash: self.shows_cash() && self.is_cash,
                expense_function: self.shows_function().then_some(self.expense_function),
                cash_flow_class: None,
            },
        })
    }
}

/// The edit form of one account: name, active and, for an expense account, its function.
/// Only changed fields go into the patch (`AccountPatch`, `engine.ts:203-207`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditAccountForm {
    /// The account as it was when the form opened.
    pub account: Account,
    /// Name as typed.
    pub name: String,
    /// Active.
    pub is_active: bool,
    /// Function, for an expense account.
    pub expense_function: Option<ExpenseFunction>,
}

impl EditAccountForm {
    /// The form filled from `account`.
    #[must_use]
    pub fn new(account: &Account) -> Self {
        Self {
            account: account.clone(),
            name: account.name.clone(),
            is_active: account.is_active,
            expense_function: account.expense_function,
        }
    }

    /// Only expense accounts have a function (`engine.ts:216-218`).
    #[must_use]
    pub fn shows_function(&self) -> bool {
        self.account.account_type == AccountType::Expense
    }

    /// The `account.update` command with the changed fields, or `None` when nothing changed.
    /// A blank name is sent as typed so the engine's message shows.
    #[must_use]
    pub fn command(&self) -> Option<LedgerCommand> {
        let a = &self.account;
        let patch = AccountPatch {
            name: (self.name != a.name).then(|| self.name.clone()),
            is_active: (self.is_active != a.is_active).then_some(self.is_active),
            expense_function: match self.expense_function {
                Some(f) if self.shows_function() && a.expense_function != Some(f) => {
                    Nullish::Value(f)
                }
                _ => Nullish::Undefined,
            },
        };
        (patch != AccountPatch::default()).then(|| LedgerCommand::AccountUpdate {
            account_id: a.id.clone(),
            patch,
        })
    }
}

/// A refused command as the screen shows it: a title and one line per issue, each the engine's
/// message as given (`IssueSummary`, `flowtrace/src/components/ledger/page.tsx:156-190`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// `Not added`, `Not changed`, `Not closed`, `Not done`.
    pub title: String,
    /// The messages.
    pub messages: Vec<String>,
}

impl Failure {
    /// The failure of `error` under `title`: the issues of a ledger refusal, otherwise the
    /// error's own text.
    #[must_use]
    pub fn new(title: &str, error: &BackendError) -> Self {
        let messages = match error {
            BackendError::Command(CommandError::Engine(engine::Error::Ledger(e))) => {
                e.issues().iter().map(|i| i.message.clone()).collect()
            }
            other => vec![other.to_string()],
        };
        Self {
            title: title.to_owned(),
            messages,
        }
    }
}

/// `PERIOD_STATUS_LABEL` (`-shared.tsx:35-39`).
#[must_use]
pub fn period_status_label(s: PeriodStatus) -> &'static str {
    match s {
        PeriodStatus::Open => "Open",
        PeriodStatus::Closed => "Closed",
        PeriodStatus::Locked => "Locked",
    }
}

/// What a period row offers (`periods.tsx:243-256`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodAction {
    /// An open period: close it.
    Close,
    /// A closed period: reopen it with a reason.
    Reopen,
    /// A closed period: lock it for good.
    Lock,
}

impl PeriodAction {
    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Reopen => "Reopen",
            Self::Lock => "Lock",
        }
    }
}

/// One period with its entry counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodRow {
    /// The period.
    pub period: FiscalPeriod,
    /// Posted and reversed entries dated in it.
    pub posted: usize,
    /// Drafts dated in it.
    pub drafts: usize,
}

/// `1 entry`, `2 entries` and the like.
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

impl PeriodRow {
    /// `1 Jan to 31 Jan 2026` (`periods.tsx:226-228`).
    #[must_use]
    pub fn dates_text(&self) -> String {
        format!(
            "{} to {}",
            format_short_date(&self.period.start_date),
            format_date(&self.period.end_date)
        )
    }

    /// `16 entries`, `1 entry · 2 drafts` (`periods.tsx:233-251`).
    #[must_use]
    pub fn counts_text(&self) -> String {
        let posted = count(self.posted, "entry", "entries");
        if self.drafts == 0 {
            posted
        } else {
            format!("{posted} {DOT} {}", count(self.drafts, "draft", "drafts"))
        }
    }

    /// The actions of its status: open closes, closed reopens or locks, locked has none.
    #[must_use]
    pub fn actions(&self) -> &'static [PeriodAction] {
        match self.period.status {
            PeriodStatus::Open => &[PeriodAction::Close],
            PeriodStatus::Closed => &[PeriodAction::Reopen, PeriodAction::Lock],
            PeriodStatus::Locked => &[],
        }
    }

    /// The `period.close` command (`periods.tsx:81-89`).
    #[must_use]
    pub fn close_command(&self) -> LedgerCommand {
        LedgerCommand::PeriodClose {
            period_id: self.period.id.clone(),
        }
    }

    /// The notice after a close (`periods.tsx:85`).
    #[must_use]
    pub fn closed_notice(&self) -> String {
        format!(
            "{} is closed. Entries can no longer be dated in it unless it is reopened.",
            self.period.name
        )
    }
}

/// One fiscal year (`periods.tsx:20-26`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearView {
    /// The fiscal year.
    pub year: i32,
    /// Its periods by start date.
    pub periods: Vec<PeriodRow>,
    /// The year-end close is posted.
    pub closed: bool,
    /// Not closed and its last period is open.
    pub can_close: bool,
    /// Drafts dated in the year.
    pub drafts: usize,
}

impl YearView {
    /// `Fiscal year 2026`.
    #[must_use]
    pub fn title(&self) -> String {
        format!("Fiscal year {}", self.year)
    }

    /// Why the year cannot close yet, when it cannot (`periods.tsx:208-212`).
    #[must_use]
    pub fn note(&self) -> Option<String> {
        (!self.closed && !self.can_close).then(|| {
            format!(
                "The last period of {} must be open to post the year-end close.",
                self.year
            )
        })
    }

    /// The confirmation of its year-end close.
    #[must_use]
    pub fn close_pending(&self) -> Option<Pending> {
        let last = self.periods.last()?;
        self.can_close.then(|| Pending::CloseYear {
            year: self.year,
            last_end: last.period.end_date.clone(),
            drafts: self.drafts,
        })
    }
}

/// `buildYears` (`periods.tsx:28-49`) with the per-period counts (`periods.tsx:69-78`):
/// latest year first, periods by start date, with the close state and the draft count of each
/// year. A year is closed when a year-end close entry is dated on its last day.
#[must_use]
pub fn fiscal_years(state: &LedgerState) -> Vec<YearView> {
    let mut counts: HashMap<&PeriodId, (usize, usize)> = HashMap::new();
    for e in &state.entries {
        let c = counts.entry(&e.period_id).or_default();
        if e.status == EntryStatus::Draft {
            c.1 += 1;
        } else {
            c.0 += 1;
        }
    }
    let mut by_year: Vec<(i32, Vec<PeriodRow>)> = Vec::new();
    for p in &state.periods {
        let (posted, drafts) = counts.get(&p.id).copied().unwrap_or_default();
        let row = PeriodRow {
            period: p.clone(),
            posted,
            drafts,
        };
        match by_year.iter_mut().find(|(y, _)| *y == p.fiscal_year) {
            Some((_, rows)) => rows.push(row),
            None => by_year.push((p.fiscal_year, vec![row])),
        }
    }
    by_year.sort_by_key(|(year, _)| std::cmp::Reverse(*year));
    by_year
        .into_iter()
        .map(|(year, mut periods)| {
            periods.sort_by(|a, b| a.period.start_date.cmp(&b.period.start_date));
            let last = periods.last().map(|p| &p.period);
            let closed = last.is_some_and(|last| {
                state
                    .entries
                    .iter()
                    .any(|e| e.source_type == SourceType::YearEndClose && e.date == last.end_date)
            });
            let can_close = !closed && last.is_some_and(|l| l.status == PeriodStatus::Open);
            let drafts = periods.iter().map(|p| p.drafts).sum();
            YearView {
                year,
                periods,
                closed,
                can_close,
                drafts,
            }
        })
        .collect()
}

/// The year the "Open fiscal year" button adds, one after the latest (`periods.tsx:92`).
#[must_use]
pub fn next_year(years: &[YearView]) -> Option<i32> {
    years.iter().map(|y| y.year).max().map(|y| y + 1)
}

/// The `year.open` command and its notice (`periods.tsx:93-104`).
#[must_use]
pub fn open_year(year: i32) -> (LedgerCommand, String) {
    (
        LedgerCommand::YearOpen { year },
        format!("Fiscal year {year} is open: twelve monthly periods were added."),
    )
}

/// An action waiting for confirmation (`periods.tsx:51-55`, the dialog at 266-347).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Reopen a closed period, with a reason.
    Reopen {
        /// The period.
        period: PeriodId,
        /// Its name.
        name: String,
    },
    /// Lock a closed period.
    Lock {
        /// The period.
        period: PeriodId,
        /// Its name.
        name: String,
    },
    /// Post the year-end close.
    CloseYear {
        /// The fiscal year.
        year: i32,
        /// The last day of the year, the closing entry's date.
        last_end: IsoDate,
        /// Drafts still in the year.
        drafts: usize,
    },
}

impl Pending {
    /// The pending action of `action` on `row`; `None` for a close, which needs no
    /// confirmation.
    #[must_use]
    pub fn of(action: PeriodAction, row: &PeriodRow) -> Option<Self> {
        let period = row.period.id.clone();
        let name = row.period.name.clone();
        match action {
            PeriodAction::Close => None,
            PeriodAction::Reopen => Some(Self::Reopen { period, name }),
            PeriodAction::Lock => Some(Self::Lock { period, name }),
        }
    }

    /// The dialog title (`periods.tsx:269-277`).
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::Reopen { name, .. } => format!("Reopen {name}"),
            Self::Lock { name, .. } => format!("Lock {name}?"),
            Self::CloseYear { year, .. } => format!("Close fiscal year {year}?"),
        }
    }

    /// The confirm button (`periods.tsx:283-291`).
    #[must_use]
    pub fn confirm_label(&self) -> &'static str {
        match self {
            Self::Reopen { .. } => "Reopen period",
            Self::Lock { .. } => "Lock period",
            Self::CloseYear { .. } => "Close the year",
        }
    }

    /// The explanation, one paragraph or list item per line (`periods.tsx:296-344`).
    #[must_use]
    pub fn body(&self) -> Vec<String> {
        match self {
            Self::Reopen { name, .. } => vec![format!(
                "Reopening lets entries be dated in {name} again, which can change figures \
                 already reported. Periods reopen latest first."
            )],
            Self::Lock { name, .. } => vec![format!(
                "A locked period can never be reopened or take entries. Lock {name} only once \
                 its accounts are signed off."
            )],
            Self::CloseYear {
                year,
                last_end,
                drafts,
            } => {
                let mut lines = vec![
                    "The year-end close does three things:".to_owned(),
                    format!(
                        "1. Posts one closing entry dated {} that brings every income and \
                         expense account for {year} to zero.",
                        format_date(last_end)
                    ),
                    "2. Moves the result into net assets fund by fund: unrestricted funds to \
                     unrestricted net assets, restricted funds to restricted net assets."
                        .to_owned(),
                    format!("3. Closes every period of {year} that is still open."),
                ];
                if *drafts > 0 {
                    let verb = if *drafts == 1 {
                        "entry is"
                    } else {
                        "entries are"
                    };
                    lines.push(format!(
                        "{drafts} draft {verb} still in {year}. Drafts are not included in the \
                         close; post or delete them first."
                    ));
                }
                lines.push(format!(
                    "A year can be closed only once. Check the trial balance for {year} before \
                     you continue."
                ));
                lines
            }
        }
    }

    /// Reopening asks for a reason.
    #[must_use]
    pub fn needs_reason(&self) -> bool {
        matches!(self, Self::Reopen { .. })
    }

    /// The command to send, with `reason` for a reopen, and the notice shown once it is done
    /// (`periods.tsx:114-136`).
    ///
    /// # Errors
    ///
    /// The message under the reason field when a reopen has a blank reason.
    pub fn command(&self, reason: &str) -> Result<(LedgerCommand, String), String> {
        match self {
            Self::Reopen { period, name } => {
                let reason = reason.trim();
                if reason.is_empty() {
                    return Err("Say why the period needs to take entries again.".to_owned());
                }
                Ok((
                    LedgerCommand::PeriodReopen {
                        period_id: period.clone(),
                        reason: reason.to_owned(),
                    },
                    format!("{name} is open again. The reason is in the audit trail."),
                ))
            }
            Self::Lock { period, name } => Ok((
                LedgerCommand::PeriodLock {
                    period_id: period.clone(),
                },
                format!("{name} is locked."),
            )),
            Self::CloseYear { year, .. } => Ok((
                LedgerCommand::YearClose { year: *year },
                format!("{year} is closed. Income and expense have moved to net assets."),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::backend::{InProcessBackend, LedgerBackend as _};
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger};
    use ledger_core::types::OrganizationId;

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

    fn row<'a>(rows: &'a [AccountRow], code: &str) -> &'a AccountRow {
        rows.iter().find(|r| r.account.code == code).unwrap()
    }

    /// Submits `command` to the demo organisation of a fresh in-process backend.
    fn submit_demo(command: LedgerCommand) -> Result<LedgerState, BackendError> {
        let mut backend = InProcessBackend::with_demo_and_empty().unwrap();
        let org: OrganizationId = backend.organisations()[0].id.clone();
        backend.submit(&org, command)?;
        Ok(LedgerState::clone(&backend.state(&org).unwrap()))
    }

    #[test]
    fn labels_follow_the_web_screen() {
        let types: Vec<_> = ACCOUNT_TYPES.into_iter().map(type_label).collect();
        assert_eq!(
            types,
            ["Asset", "Liability", "Net assets", "Income", "Expense"]
        );
        let functions: Vec<_> = EXPENSE_FUNCTIONS.into_iter().map(function_label).collect();
        assert_eq!(
            functions,
            ["Programme", "Management and general", "Fundraising"]
        );
        assert_eq!(normal_balance_label(NormalBalance::Debit), "Debit");
        assert_eq!(normal_balance_label(NormalBalance::Credit), "Credit");
        assert_eq!(period_status_label(PeriodStatus::Open), "Open");
        assert_eq!(period_status_label(PeriodStatus::Closed), "Closed");
        assert_eq!(period_status_label(PeriodStatus::Locked), "Locked");
    }

    #[test]
    fn money_and_dates_format_as_the_web_app() {
        assert_eq!(format_minor(125_050), "1,250.50");
        assert_eq!(format_minor(123_456_789), "1,234,567.89");
        assert_eq!(format_minor(5), "0.05");
        assert_eq!(format_minor(-150_000), "(1,500.00)");
        assert_eq!(format_minor(0), "\u{2013}");
        let d = IsoDate::parse("2026-01-31").unwrap();
        assert_eq!(format_date(&d), "31 Jan 2026");
        assert_eq!(
            format_short_date(&IsoDate::parse("2025-12-01").unwrap()),
            "1 Dec"
        );
    }

    #[test]
    fn the_tree_puts_children_under_their_parent_in_code_order() {
        let state = demo();
        let rows = account_rows(&state, Some(&as_of()));
        assert_eq!(rows.len(), state.accounts.len());
        let top: Vec<_> = rows
            .iter()
            .filter(|r| r.depth == 0)
            .map(|r| r.account.code.as_str())
            .collect();
        assert_eq!(
            top,
            [
                "1000", "2000", "3000", "4000", "5000", "6000", "7000", "8000"
            ]
        );
        let first: Vec<_> = rows
            .iter()
            .take(3)
            .map(|r| (r.account.code.as_str(), r.depth))
            .collect();
        assert_eq!(first, [("1000", 0), ("1100", 1), ("1110", 2)]);
        // Every row sits directly under the nearest shallower row, its parent, and siblings
        // run in code order.
        for (i, r) in rows.iter().enumerate() {
            let parent = rows[..i].iter().rev().find(|p| p.depth < r.depth);
            match r.depth {
                0 => assert!(r.account.parent_id.is_none()),
                d => {
                    let parent = parent.unwrap();
                    assert_eq!(parent.depth, d - 1);
                    assert_eq!(r.account.parent_id.as_ref(), Some(&parent.account.id));
                }
            }
            if let Some(next) = rows[i + 1..].iter().find(|n| n.depth <= r.depth)
                && next.depth == r.depth
            {
                assert!(r.account.code < next.account.code, "{}", r.account.code);
            }
        }
    }

    #[test]
    fn an_account_with_a_missing_parent_sits_at_the_top() {
        let mut state = demo();
        let i = state
            .accounts
            .iter()
            .position(|a| a.code == "1110")
            .unwrap();
        state.accounts[i].parent_id = Some(AccountId::from("acc-gone"));
        let rows = account_rows(&state, None);
        let r = row(&rows, "1110");
        assert_eq!(r.depth, 0);
        let top: Vec<_> = rows
            .iter()
            .filter(|r| r.depth == 0)
            .map(|r| r.account.code.as_str())
            .collect();
        assert_eq!(top[..2], ["1000", "1110"]);
    }

    #[test]
    fn balances_are_natural_and_headers_sum_their_children() {
        let state = demo();
        let rows = account_rows(&state, Some(&as_of()));
        let posted = balances(
            &state,
            &LineFilter {
                to: Some(as_of()),
                ..LineFilter::default()
            },
        );
        for r in rows.iter().filter(|r| !r.account.is_group) {
            let own = posted.get(&r.account.id).copied().unwrap_or(0);
            assert_eq!(r.balance, natural(r.account.normal_balance, own));
        }
        for (i, r) in rows.iter().enumerate().filter(|(_, r)| r.account.is_group) {
            let children: Minor = rows[i + 1..]
                .iter()
                .take_while(|c| c.depth > r.depth)
                .filter(|c| c.depth == r.depth + 1)
                .map(|c| c.balance)
                .sum();
            // Children of a header share its type, so the natural sign is the same.
            let own = posted.get(&r.account.id).copied().unwrap_or(0);
            assert_eq!(r.balance, children + natural(r.account.normal_balance, own));
        }
        // The demo holds money in the bank, and income shows positive.
        assert!(row(&rows, "1000").balance > 0);
        assert!(row(&rows, "4000").balance > 0);
        assert_eq!(
            row(&rows, "1000").balance_text(),
            format_minor(row(&rows, "1000").balance)
        );
    }

    #[test]
    fn tags_toggle_and_meta_follow_the_account() {
        let state = demo();
        let rows = account_rows(&state, Some(&as_of()));
        assert_eq!(row(&rows, "1000").tags(), ["Header"]);
        assert_eq!(row(&rows, "1110").tags(), ["Cash"]);
        let payable = row(&rows, "2100");
        assert!(payable.is_system);
        assert!(payable.tags().contains(&"System"));
        assert_eq!(payable.toggle_label(), None);
        assert_eq!(row(&rows, "1110").toggle_label(), Some("Deactivate"));
        assert_eq!(row(&rows, "1110").function_text(), "\u{2013}");
        assert_eq!(accounts_meta(&rows), format!("{} accounts", rows.len()));

        let after = submit_demo(row(&rows, "1160").toggle_command()).unwrap();
        let rows = account_rows(&after, Some(&as_of()));
        let petty = row(&rows, "1160");
        assert!(!petty.account.is_active);
        assert_eq!(petty.tags(), ["Cash", "Inactive"]);
        assert_eq!(petty.toggle_label(), Some("Reactivate"));
        assert_eq!(
            accounts_meta(&rows),
            format!("{} accounts \u{b7} 1 inactive", rows.len())
        );
    }

    #[test]
    fn the_add_form_checks_blanks_and_builds_the_command() {
        let state = demo();
        let mut form = NewAccountForm::default();
        assert!(form.shows_function());
        assert!(!form.shows_cash());
        assert_eq!(
            form.command(),
            Err(FieldErrors {
                code: Some("Enter a code, such as 5150.".into()),
                name: Some("Enter the account name.".into()),
            })
        );

        let parents = form.parents(&state);
        assert!(parents.len() >= 2);
        assert!(parents.windows(2).all(|w| w[0].1 < w[1].1));
        let (parent, label) = parents[0].clone();
        let header = state.accounts.iter().find(|a| a.id == parent).unwrap();
        assert!(header.is_group);
        assert_eq!(header.account_type, AccountType::Expense);
        assert_eq!(label, format!("{} \u{b7} {}", header.code, header.name));

        form.code = " 5150 ".into();
        form.name = " Field visits ".into();
        form.parent = Some(parent.clone());
        form.is_cash = true;
        let command = form.command().unwrap();
        let LedgerCommand::AccountAdd { input } = command else {
            panic!("not account.add");
        };
        assert_eq!(input.code, "5150");
        assert_eq!(input.name, "Field visits");
        assert_eq!(input.account_type, AccountType::Expense);
        assert_eq!(input.parent_id, Some(parent));
        assert!(!input.is_group);
        assert!(!input.is_cash, "only a non-header asset is cash");
        assert_eq!(input.expense_function, Some(ExpenseFunction::Program));
        assert_eq!(input.cash_flow_class, None);
        assert_eq!(input.id, None);

        form.set_type(AccountType::Asset);
        assert_eq!(form.parent, None, "a new type clears the parent");
        assert!(form.shows_cash());
        assert!(!form.shows_function());
        let LedgerCommand::AccountAdd { input } = form.command().unwrap() else {
            panic!("not account.add");
        };
        assert!(input.is_cash);
        assert_eq!(input.expense_function, None);

        form.is_group = true;
        assert!(!form.shows_cash());
        let LedgerCommand::AccountAdd { input } = form.command().unwrap() else {
            panic!("not account.add");
        };
        assert!(input.is_group);
        assert!(!input.is_cash);
    }

    #[test]
    fn an_added_account_joins_the_tree_under_its_parent() {
        let state = demo();
        let mut form = NewAccountForm {
            code: "5190".into(),
            name: "Field visits".into(),
            ..NewAccountForm::default()
        };
        form.parent = Some(form.parents(&state)[0].0.clone());
        let after = submit_demo(form.command().unwrap()).unwrap();
        let rows = account_rows(&after, Some(&as_of()));
        let added = row(&rows, "5190");
        assert_eq!(added.depth, 1);
        assert_eq!(added.function_text(), "Programme");
        assert_eq!(added.balance_text(), "\u{2013}");
    }

    #[test]
    fn a_code_in_use_shows_the_engine_message() {
        let form = NewAccountForm {
            code: "1110".into(),
            name: "Another bank".into(),
            ..NewAccountForm::default()
        };
        let err = submit_demo(form.command().unwrap()).unwrap_err();
        assert_eq!(
            Failure::new("Not added", &err),
            Failure {
                title: "Not added".into(),
                messages: vec!["Account code 1110 is already in use.".into()],
            }
        );
    }

    #[test]
    fn the_edit_form_sends_only_what_changed() {
        let state = demo();
        let rows = account_rows(&state, None);
        let bank = &row(&rows, "1110").account;
        let mut form = EditAccountForm::new(bank);
        assert_eq!(form.name, bank.name);
        assert!(form.is_active);
        assert!(!form.shows_function());
        assert_eq!(form.command(), None);

        form.name = "Bank: Main account".into();
        assert_eq!(
            form.command(),
            Some(LedgerCommand::AccountUpdate {
                account_id: bank.id.clone(),
                patch: AccountPatch {
                    name: Some("Bank: Main account".into()),
                    ..AccountPatch::default()
                },
            })
        );

        let expense = rows
            .iter()
            .find(|r| r.account.expense_function == Some(ExpenseFunction::Program))
            .unwrap();
        let mut form = EditAccountForm::new(&expense.account);
        assert!(form.shows_function());
        form.expense_function = Some(ExpenseFunction::Management);
        form.is_active = false;
        assert_eq!(
            form.command(),
            Some(LedgerCommand::AccountUpdate {
                account_id: expense.account.id.clone(),
                patch: AccountPatch {
                    name: None,
                    is_active: Some(false),
                    expense_function: Nullish::Value(ExpenseFunction::Management),
                },
            })
        );
        let after = submit_demo(form.command().unwrap()).unwrap();
        let changed = after
            .accounts
            .iter()
            .find(|a| a.id == expense.account.id)
            .unwrap();
        assert_eq!(changed.expense_function, Some(ExpenseFunction::Management));
        assert!(!changed.is_active);
    }

    #[test]
    fn a_blank_name_is_left_to_the_engine() {
        let state = demo();
        let bank = state.accounts.iter().find(|a| a.code == "1110").unwrap();
        let mut form = EditAccountForm::new(bank);
        form.name = "   ".into();
        let err = submit_demo(form.command().unwrap()).unwrap_err();
        assert_eq!(
            Failure::new("Not changed", &err).messages,
            ["An account needs a name."]
        );
    }

    #[test]
    fn a_non_ledger_error_shows_its_own_text() {
        let err = BackendError::Connection("The server is not reachable.".into());
        assert_eq!(
            Failure::new("Not done", &err).messages,
            ["The server is not reachable."]
        );
    }

    #[test]
    fn fiscal_years_run_latest_first_with_their_periods() {
        let state = demo();
        let years = fiscal_years(&state);
        let ids: Vec<_> = years.iter().map(|y| y.year).collect();
        assert_eq!(ids, [2026, 2025]);
        let (y2026, y2025) = (&years[0], &years[1]);
        assert_eq!(y2026.title(), "Fiscal year 2026");
        assert_eq!(y2026.periods.len(), 12);
        assert!(
            y2026
                .periods
                .windows(2)
                .all(|w| w[0].period.start_date < w[1].period.start_date)
        );
        assert!(!y2026.closed);
        assert!(y2026.can_close);
        assert_eq!(y2026.note(), None);
        assert_eq!(y2026.drafts, 0);

        let jan = &y2026.periods[0];
        assert_eq!(jan.period.name, "Jan 2026");
        assert_eq!(jan.dates_text(), "1 Jan to 31 Jan 2026");
        assert_eq!(jan.counts_text(), "16 entries");
        assert_eq!(jan.actions(), [PeriodAction::Close]);
        assert_eq!(y2026.periods[2].counts_text(), "0 entries");

        assert!(!y2025.closed);
        assert!(!y2025.can_close);
        assert_eq!(
            y2025.note().as_deref(),
            Some("The last period of 2025 must be open to post the year-end close.")
        );
        assert_eq!(y2025.close_pending(), None);
        let dec = y2025.periods.last().unwrap();
        assert_eq!(dec.counts_text(), "1 entry");
        assert_eq!(dec.actions(), [PeriodAction::Reopen, PeriodAction::Lock]);
        assert_eq!(y2025.periods[0].actions(), []);
        assert_eq!(next_year(&years), Some(2027));
        assert_eq!(next_year(&[]), None);
    }

    #[test]
    fn counts_name_drafts() {
        let state = demo();
        let mut r = fiscal_years(&state)[0].periods[0].clone();
        r.posted = 1;
        r.drafts = 1;
        assert_eq!(r.counts_text(), "1 entry \u{b7} 1 draft");
        r.drafts = 3;
        assert_eq!(r.counts_text(), "1 entry \u{b7} 3 drafts");
        assert_eq!(PeriodAction::Close.label(), "Close");
        assert_eq!(PeriodAction::Reopen.label(), "Reopen");
        assert_eq!(PeriodAction::Lock.label(), "Lock");
    }

    #[test]
    fn closing_a_period_sends_period_close() {
        let state = demo();
        let jan = &fiscal_years(&state)[0].periods[0];
        assert_eq!(
            jan.close_command(),
            LedgerCommand::PeriodClose {
                period_id: jan.period.id.clone(),
            }
        );
        assert_eq!(
            jan.closed_notice(),
            "Jan 2026 is closed. Entries can no longer be dated in it unless it is reopened."
        );
        assert_eq!(Pending::of(PeriodAction::Close, jan), None);
        let after = submit_demo(jan.close_command()).unwrap();
        let jan = &fiscal_years(&after)[0].periods[0];
        assert_eq!(jan.period.status, PeriodStatus::Closed);
    }

    #[test]
    fn closing_out_of_order_shows_the_engine_message() {
        let state = demo();
        let mar = &fiscal_years(&state)[0].periods[2];
        let err = submit_demo(mar.close_command()).unwrap_err();
        assert_eq!(
            Failure::new("Not closed", &err).messages,
            ["Close Jan 2026 first; periods close in order."]
        );
    }

    #[test]
    fn reopening_needs_a_reason() {
        let state = demo();
        let dec = fiscal_years(&state)[1].periods[11].clone();
        let pending = Pending::of(PeriodAction::Reopen, &dec).unwrap();
        assert_eq!(pending.title(), "Reopen Dec 2025");
        assert_eq!(pending.confirm_label(), "Reopen period");
        assert!(pending.needs_reason());
        assert_eq!(
            pending.body(),
            [
                "Reopening lets entries be dated in Dec 2025 again, which can change figures \
              already reported. Periods reopen latest first."
            ]
        );
        assert_eq!(
            pending.command("  "),
            Err("Say why the period needs to take entries again.".into())
        );
        let (command, notice) = pending.command("  Late invoice ").unwrap();
        assert_eq!(
            command,
            LedgerCommand::PeriodReopen {
                period_id: dec.period.id.clone(),
                reason: "Late invoice".into(),
            }
        );
        assert_eq!(
            notice,
            "Dec 2025 is open again. The reason is in the audit trail."
        );
        let after = submit_demo(command).unwrap();
        assert_eq!(
            fiscal_years(&after)[1].periods[11].period.status,
            PeriodStatus::Open
        );
    }

    #[test]
    fn locking_asks_first() {
        let state = demo();
        let dec = fiscal_years(&state)[1].periods[11].clone();
        let pending = Pending::of(PeriodAction::Lock, &dec).unwrap();
        assert_eq!(pending.title(), "Lock Dec 2025?");
        assert_eq!(pending.confirm_label(), "Lock period");
        assert!(!pending.needs_reason());
        assert_eq!(
            pending.body(),
            [
                "A locked period can never be reopened or take entries. Lock Dec 2025 only once \
              its accounts are signed off."
            ]
        );
        let (command, notice) = pending.command("").unwrap();
        assert_eq!(
            command,
            LedgerCommand::PeriodLock {
                period_id: dec.period.id.clone(),
            }
        );
        assert_eq!(notice, "Dec 2025 is locked.");
        let after = submit_demo(command).unwrap();
        let dec = &fiscal_years(&after)[1].periods[11];
        assert_eq!(dec.period.status, PeriodStatus::Locked);
        assert_eq!(dec.actions(), []);
    }

    #[test]
    fn the_year_end_close_is_confirmed_first() {
        let state = demo();
        let y2026 = &fiscal_years(&state)[0];
        let pending = y2026.close_pending().unwrap();
        assert_eq!(pending.title(), "Close fiscal year 2026?");
        assert_eq!(pending.confirm_label(), "Close the year");
        assert_eq!(
            pending.body(),
            [
                "The year-end close does three things:",
                "1. Posts one closing entry dated 31 Dec 2026 that brings every income and \
                 expense account for 2026 to zero.",
                "2. Moves the result into net assets fund by fund: unrestricted funds to \
                 unrestricted net assets, restricted funds to restricted net assets.",
                "3. Closes every period of 2026 that is still open.",
                "A year can be closed only once. Check the trial balance for 2026 before you \
                 continue.",
            ]
        );
        let with_drafts = Pending::CloseYear {
            year: 2026,
            last_end: IsoDate::parse("2026-12-31").unwrap(),
            drafts: 2,
        };
        assert!(
            with_drafts.body().contains(
                &"2 draft entries are still in 2026. Drafts are not included in the close; post \
              or delete them first."
                    .to_owned()
            )
        );
        let one = Pending::CloseYear {
            year: 2026,
            last_end: IsoDate::parse("2026-12-31").unwrap(),
            drafts: 1,
        };
        assert!(
            one.body().contains(
                &"1 draft entry is still in 2026. Drafts are not included in the close; post or \
              delete them first."
                    .to_owned()
            )
        );

        let (command, notice) = pending.command("").unwrap();
        assert_eq!(command, LedgerCommand::YearClose { year: 2026 });
        assert_eq!(
            notice,
            "2026 is closed. Income and expense have moved to net assets."
        );
        let after = submit_demo(command).unwrap();
        let y2026 = &fiscal_years(&after)[0];
        assert!(y2026.closed);
        assert!(!y2026.can_close);
        assert_eq!(y2026.note(), None);
        assert!(
            y2026
                .periods
                .iter()
                .all(|p| p.period.status == PeriodStatus::Closed)
        );
    }

    #[test]
    fn opening_the_next_year_adds_its_periods() {
        let (command, notice) = open_year(2027);
        assert_eq!(command, LedgerCommand::YearOpen { year: 2027 });
        assert_eq!(
            notice,
            "Fiscal year 2027 is open: twelve monthly periods were added."
        );
        let after = submit_demo(command).unwrap();
        let years = fiscal_years(&after);
        assert_eq!(years[0].year, 2027);
        assert_eq!(years[0].periods.len(), 12);
        assert_eq!(next_year(&years), Some(2028));
    }
}
