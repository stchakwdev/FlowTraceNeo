//! Reports over posted journal lines, mirroring `flowtrace/src/lib/ledger/reports.ts`.
//!
//! Every report reads `&LedgerState` only: no env, no writes. They read posted lines (reversed
//! entries count, offset by their reversals), and every one of them foots: the trial balance
//! nets to zero, the Statement of Financial Position balances, and the Statement of Activities'
//! change in net assets equals the change on the balance sheet (`reports.ts:1-4`).
//!
//! Outputs are owned values with the TS JSON shape: money in integer minor units, ratios and
//! months as `f64` written through [`crate::js_number`], keys camelCase where TS uses camelCase
//! (`asOf`, `totalAssets`) and records ([`Account`], [`JournalLine`]) in their `snake_case` form.
//! Sorting by account code or entry number is byte order, which is what `localeCompare` gives on
//! these keys (`rust/fixtures/README.md`, "Formatting notes").
//!
//! Where TS throws (a date that is not on the calendar, an unknown grant, an exchange rate
//! `convert` refuses), the Rust function returns [`ReportError`]. Where TS would read a field of
//! a missing account and throw a `TypeError` (a line on an account not in the chart, which the
//! engine never writes), the line is left out.

mod budgets;
mod statements;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::money::{Minor, MoneyError, js_round, sum};
use crate::types::{
    Account, AccountId, CodedLine, DimensionId, EntryId, EntryStatus, FundId, FundRestriction,
    GrantId, IsoDate, JournalEntry, JournalLine, LedgerState, LineId, NormalBalance, SourceType,
};

pub use budgets::{
    BudgetVsActualOptions, BudgetVsActualRow, GrantSummary, LedgerSignals, WarningLevel,
    budget_vs_actual, grant_summary, ledger_signals, warning_level,
};
pub use statements::{
    Activities, ActivityRow, CashFlow, FinancialPosition, FunctionalExpenses, FunctionalRow,
    FunctionalTotals, FundActivity, GeneralLedger, GeneralLedgerRow, NetAssets, OperatingCash,
    RestrictionTotals, StatementRow, TrialBalance, TrialBalanceRow, activities, cash_flow,
    financial_position, functional_expenses, general_ledger, trial_balance,
};

/// `Basis` (`reports.ts:18`): the books are accrual; `Cash` is the cash-basis view of them.
///
/// Not [`crate::types::Basis`], which is the books' own setting and has only `accrual`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// `accrual`, the default.
    #[default]
    Accrual,
    /// `cash`: accruals dropped, settlements re-coded to the expense lines they paid.
    Cash,
}

/// `LineFilter` (`reports.ts:20-30`): which posted lines a report reads.
///
/// TS tests each filter for truthiness, so an empty id or an absent one means no filter; an
/// empty `account_ids` list (truthy in TS) matches no account.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineFilter {
    /// First date, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<IsoDate>,
    /// Last date, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<IsoDate>,
    /// These accounts and every account below them in the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_ids: Option<Vec<AccountId>>,
    /// Lines charged to this fund.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fund_id: Option<FundId>,
    /// Lines charged to this grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<GrantId>,
    /// Lines tagged with this dimension value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_id: Option<DimensionId>,
    /// Accrual (default) or the cash-basis view.
    #[serde(default)]
    pub basis: Basis,
    /// Leave out the year-end closing entries (for activity reports).
    #[serde(default)]
    pub exclude_closing: bool,
}

/// `ReportLine` (`reports.ts:32-34`): a posted line with its entry, written as the line's fields
/// plus `entry`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportLine {
    /// The line. On a cash basis a re-coded settlement line has the id `<line>:<coded line>`.
    #[serde(flatten)]
    pub line: JournalLine,
    /// Its entry.
    pub entry: JournalEntry,
}

/// What a report cannot answer, where the TS function throws.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReportError {
    /// A date whose month is outside 1-12 or whose day is outside 1-31, or a result before year
    /// 0000. TS builds an `Invalid Date` from it (`reports.ts:202`, `reports.ts:550-551`):
    /// `dayBefore` throws a `RangeError` and `grantSummary` would return `NaN` months.
    #[error("{0} is not a calendar date.")]
    InvalidDate(IsoDate),
    /// `grantSummary` for a grant that does not exist; TS throws a `TypeError` reading its award
    /// (`reports.ts:556`).
    #[error("Grant {0} does not exist.")]
    UnknownGrant(GrantId),
    /// `convert` refused a document's rate or amount (`reports.ts:563`, `money.ts:36-51`).
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// `descendants(state, accountId)` (`reports.ts:39-52`): the account plus every account below it
/// in the tree, in the order TS adds them to its `Set` (the account first, then pass by pass in
/// chart order). An unknown id gives just itself.
#[must_use]
pub fn descendants(state: &LedgerState, account_id: &AccountId) -> Vec<AccountId> {
    descendant_ids(&state.accounts, account_id.as_str())
        .into_iter()
        .map(AccountId::from)
        .collect()
}

/// `postedLines(state, f)` (`reports.ts:54-82`): lines of every entry that is not a draft
/// (reversed entries count, offset by their reversals), in book order, then filtered by `f`.
///
/// With [`Basis::Cash`], accruals are dropped and each bill payment or claim reimbursement is
/// re-coded to the expense lines it settled, pro rata (`reports.ts:84-139`).
#[must_use]
pub fn posted_lines(state: &LedgerState, f: &LineFilter) -> Vec<ReportLine> {
    posted(state, &f.scope())
        .into_iter()
        .map(Posted::into_report_line)
        .collect()
}

/// `balances(state, f)` (`reports.ts:141-146`): signed debit-minus-credit balance per account in
/// base currency, for every account with a line in `f` (a zero balance included).
#[must_use]
pub fn balances(state: &LedgerState, f: &LineFilter) -> BTreeMap<AccountId, Minor> {
    owned_balances(&posted(state, &f.scope()))
}

/// `natural(a, signed)` (`reports.ts:148-150`): a balance shown the natural way round, credit
/// accounts positive.
#[must_use]
pub fn natural(normal_balance: NormalBalance, signed: Minor) -> Minor {
    match normal_balance {
        NormalBalance::Debit => signed,
        NormalBalance::Credit => -signed,
    }
}

/// `dayBefore(d)` (`reports.ts:201-205`): the previous calendar day, in UTC.
///
/// # Errors
///
/// [`ReportError::InvalidDate`] for a date that is not on the calendar (see
/// [`ReportError::InvalidDate`]) and for `0000-01-01`, whose day before TS cannot print as a date.
pub fn day_before(d: &IsoDate) -> Result<IsoDate, ReportError> {
    let invalid = || ReportError::InvalidDate(d.clone());
    let (year, month, day) = civil_from_days(utc_days(d).ok_or_else(invalid)? - 1);
    // `toISOString` writes years outside 0000-9999 with a sign and six digits, which the
    // `slice(0, 10)` then cuts to something that is not a date.
    if !(0..=9999).contains(&year) {
        return Err(invalid());
    }
    IsoDate::parse(&format!("{year:04}-{month:02}-{day:02}")).map_err(|_| invalid())
}

/// A posted line borrowed from the books, or owned when the cash basis re-coded it.
#[derive(Debug, Clone)]
struct Posted<'a> {
    line: Cow<'a, JournalLine>,
    entry: &'a JournalEntry,
}

impl Posted<'_> {
    fn into_report_line(self) -> ReportLine {
        ReportLine {
            line: self.line.into_owned(),
            entry: self.entry.clone(),
        }
    }
}

/// A [`LineFilter`] borrowed, with its truthiness applied.
#[derive(Debug, Clone, Copy, Default)]
struct Scope<'f> {
    from: Option<&'f IsoDate>,
    to: Option<&'f IsoDate>,
    account_ids: Option<&'f [AccountId]>,
    fund_id: Option<&'f FundId>,
    grant_id: Option<&'f GrantId>,
    dimension_id: Option<&'f DimensionId>,
    basis: Basis,
    exclude_closing: bool,
}

impl LineFilter {
    fn scope(&self) -> Scope<'_> {
        Scope {
            from: self.from.as_ref(),
            to: self.to.as_ref(),
            account_ids: self.account_ids.as_deref(),
            fund_id: truthy(self.fund_id.as_ref()),
            grant_id: truthy(self.grant_id.as_ref()),
            dimension_id: truthy(self.dimension_id.as_ref()),
            basis: self.basis,
            exclude_closing: self.exclude_closing,
        }
    }
}

/// An id as a JS condition reads it: the empty string is no id.
fn truthy<T: AsRef<str>>(id: Option<&T>) -> Option<&T> {
    id.filter(|id| !id.as_ref().is_empty())
}

/// The `descendants` loop (`reports.ts:41-50`) over ids: passes over the chart in order until
/// one adds nothing, each adding every account whose parent is already in.
fn descendant_ids<'s>(accounts: &'s [Account], root: &'s str) -> Vec<&'s str> {
    let mut order = vec![root];
    let mut seen = HashSet::from([root]);
    let mut grew = true;
    while grew {
        grew = false;
        for a in accounts {
            if let Some(parent) = truthy(a.parent_id.as_ref())
                && seen.contains(parent.as_str())
                && seen.insert(a.id.as_str())
            {
                order.push(a.id.as_str());
                grew = true;
            }
        }
    }
    order
}

/// `postedLines` over a [`Scope`] (`reports.ts:59-82`), borrowing from the books.
fn posted<'a>(state: &'a LedgerState, s: &Scope<'_>) -> Vec<Posted<'a>> {
    // `new Map(...)`: the last entry with an id wins.
    let entries: HashMap<&EntryId, &JournalEntry> =
        state.entries.iter().map(|e| (&e.id, e)).collect();
    let mut lines: Vec<Posted<'a>> = state
        .lines
        .iter()
        .filter_map(|l| {
            let entry = *entries.get(&l.entry_id)?;
            let skip = entry.status == EntryStatus::Draft
                || (s.exclude_closing && entry.source_type == SourceType::YearEndClose);
            (!skip).then_some(Posted {
                line: Cow::Borrowed(l),
                entry,
            })
        })
        .collect();
    if s.basis == Basis::Cash {
        lines = to_cash_basis(state, lines);
    }
    let accounts: Option<HashSet<&str>> = s.account_ids.map(|ids| {
        ids.iter()
            .flat_map(|id| descendant_ids(&state.accounts, id.as_str()))
            .collect()
    });
    lines.retain(|p| {
        let (l, date) = (&p.line, &p.entry.date);
        s.from.is_none_or(|from| date >= from)
            && s.to.is_none_or(|to| date <= to)
            && accounts
                .as_ref()
                .is_none_or(|set| set.contains(l.account_id.as_str()))
            && s.fund_id.is_none_or(|f| &l.fund_id == f)
            && s.grant_id.is_none_or(|g| l.grant_id.as_ref() == Some(g))
            && s.dimension_id.is_none_or(|d| l.dimension_ids.contains(d))
    });
    lines
}

/// `toCashBasis(state, lines)` (`reports.ts:84-139`).
///
/// A reversed entry and its reversal cancel out (on a cash basis the reversal of an accrual
/// would otherwise show as negative spend); bill and claim accruals are dropped; a payment's or
/// reimbursement's debit to accounts or claims payable is replaced by the settled document's
/// coded lines in that fund, pro rata. Other lines pass unchanged.
fn to_cash_basis<'a>(state: &'a LedgerState, lines: Vec<Posted<'a>>) -> Vec<Posted<'a>> {
    let mut cancelled: HashSet<&EntryId> = HashSet::new();
    for e in &state.entries {
        if e.status == EntryStatus::Reversed {
            cancelled.insert(&e.id);
            if let Some(by) = &e.reversed_by {
                cancelled.insert(by);
            }
        }
    }
    let sys = &state.settings.system_accounts;
    let mut out = Vec::with_capacity(lines.len());
    for p in lines {
        let source = p.entry.source_type;
        if cancelled.contains(&p.line.entry_id)
            || matches!(source, SourceType::Bill | SourceType::ExpenseClaim)
        {
            continue;
        }
        let settles = matches!(source, SourceType::Payment | SourceType::Reimbursement)
            && (p.line.account_id == sys.accounts_payable
                || p.line.account_id == sys.claims_payable);
        if !settles {
            out.push(p);
            continue;
        }
        // Payment entries carry the bill id, reimbursements the claim id (engine.ts).
        let source_id = p.entry.source_id.as_deref();
        let coded = if source == SourceType::Payment {
            state
                .bills
                .iter()
                .find(|b| Some(b.id.as_str()) == source_id)
                .map(|b| b.lines.as_slice())
        } else {
            state
                .claims
                .iter()
                .find(|c| Some(c.id.as_str()) == source_id)
                .map(|c| c.lines.as_slice())
        };
        match coded {
            Some(coded) => recode(&p, coded, &mut out),
            None => out.push(p),
        }
    }
    out
}

/// Replaces one settlement line with the document's coded lines in the same fund
/// (`reports.ts:116-136`): each takes `Math.round(base * amount / fundTotal)` of the line's base
/// amount and the last takes what is left, keeping the line's entry, fund, currency and rate.
fn recode<'a>(p: &Posted<'a>, coded: &[CodedLine], out: &mut Vec<Posted<'a>>) {
    let fund_lines: Vec<&CodedLine> = coded
        .iter()
        .filter(|d| d.fund_id == p.line.fund_id)
        .collect();
    let fund_total = sum(fund_lines.iter().map(|d| d.amount));
    let base = p.line.base_amount;
    let mut allocated = 0;
    for (i, d) in fund_lines.iter().enumerate() {
        let part = if i + 1 == fund_lines.len() {
            base - allocated
        } else {
            // In doubles, as TS; coded amounts are > 0, so the total is too.
            minor_of(js_round((js(base) * js(d.amount)) / js(fund_total)))
        };
        allocated += part;
        let mut line = JournalLine::clone(&p.line);
        line.id = LineId::from(format!("{}:{}", p.line.id, d.id));
        line.account_id = d.account_id.clone();
        line.grant_id.clone_from(&d.grant_id);
        line.dimension_ids.clone_from(&d.dimension_ids);
        line.debit = part.max(0);
        line.credit = (-part).max(0);
        line.base_amount = part;
        line.memo = Some(d.description.clone());
        out.push(Posted {
            line: Cow::Owned(line),
            entry: p.entry,
        });
    }
}

/// Signed balance per account of `lines` (`reports.ts:142-145`), borrowing the ids.
fn balance_map<'p>(lines: &'p [Posted<'_>]) -> HashMap<&'p AccountId, Minor> {
    let mut m = HashMap::new();
    for p in lines {
        *m.entry(&p.line.account_id).or_insert(0) += p.line.base_amount;
    }
    m
}

/// [`balance_map`] with owned ids, sorted by id, for the public results.
fn owned_balances(lines: &[Posted<'_>]) -> BTreeMap<AccountId, Minor> {
    balance_map(lines)
        .into_iter()
        .map(|(id, b)| (id.clone(), b))
        .collect()
}

/// `new Map(state.accounts.map((a) => [a.id, a]))`: the last account with an id wins, as in a
/// JS `Map` built from the chart.
fn account_map(state: &LedgerState) -> HashMap<&AccountId, &Account> {
    state.accounts.iter().map(|a| (&a.id, a)).collect()
}

/// `[...state.accounts].sort(byCode)` (`reports.ts:207`): the chart by code. A stable sort in
/// byte order, which is what `localeCompare` gives on account codes (fixtures README,
/// "Formatting notes").
fn by_code(accounts: &[Account]) -> Vec<&Account> {
    let mut chart: Vec<&Account> = accounts.iter().collect();
    chart.sort_by(|a, b| a.code.cmp(&b.code));
    chart
}

/// `restrictedFunds(state)` (`reports.ts:234-236`): every fund that is not unrestricted.
fn restricted_funds(state: &LedgerState) -> HashSet<&FundId> {
    state
        .funds
        .iter()
        .filter(|f| f.restriction != FundRestriction::Unrestricted)
        .map(|f| &f.id)
        .collect()
}

/// A whole number of minor units as the JS number TS holds; exact, as amounts are safe integers.
#[expect(
    clippy::cast_precision_loss,
    reason = "amounts are safe integers, exact in f64"
)]
fn js(minor: Minor) -> f64 {
    minor as f64
}

/// A `Math.round` result back in minor units: whole, and a share of a safe amount.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a rounded share of a safe amount is a safe integer"
)]
fn minor_of(rounded: f64) -> Minor {
    rounded as Minor
}

/// Days since 1970-01-01 of `new Date(`${d}T00:00:00Z`)` (`reports.ts:202`, `reports.ts:550`),
/// or `None` where JS makes an `Invalid Date`. The JS engine that wrote the fixtures (Bun's)
/// takes a month of 1-12 and a day of 1-31, and carries a day past the month's end into the next
/// month: `2026-02-30` is 2 March.
fn utc_days(d: &IsoDate) -> Option<i64> {
    let s = d.as_str();
    let part = |range: std::ops::Range<usize>| s.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (part(0..4)?, part(5..7)?, part(8..10)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, 1) + day - 1)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant, "chrono-Compatible Low-Level
/// Date Algorithms", `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let (era, yoe) = (y.div_euclid(400), y.rem_euclid(400));
    let mp = (month + 9) % 12; // March is 0
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of [`days_from_civil`]: `(year, month, day)`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// `monthsBetween(a, b)` (`reports.ts:549-553`): UTC milliseconds from `a` to `b` over an average
/// month of 30.4375 days.
fn months_between(a: &IsoDate, b: &IsoDate) -> Result<f64, ReportError> {
    let days = |d: &IsoDate| utc_days(d).ok_or_else(|| ReportError::InvalidDate(d.clone()));
    let ms = (days(b)? - days(a)?) * 86_400_000;
    Ok(js(ms) / (1000.0 * 60.0 * 60.0 * 24.0 * 30.4375))
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, days_from_civil};

    #[test]
    fn civil_days_round_trip_across_eras() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(0, 1, 1), -719_528);
        for days in [-719_528, -1, 0, 11_016, 11_017, 20_454, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        }
        assert_eq!(civil_from_days(2_932_896), (9999, 12, 31));
    }
}
