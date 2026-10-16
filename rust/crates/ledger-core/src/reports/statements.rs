//! The trial balance, general ledger and the financial statements (`reports.ts:152-451`).

use std::collections::BTreeMap;

use serde::Serialize;

use super::{Basis, LineFilter, ReportError, ReportLine};
use crate::money::Minor;
use crate::types::{Account, AccountId, Fund, IsoDate, LedgerState};

/// `TrialBalanceRow` (`reports.ts:156-160`): one account's balance on its debit or credit side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrialBalanceRow {
    /// The account.
    pub account: Account,
    /// A debit balance, else 0.
    pub debit: Minor,
    /// A credit balance, else 0.
    pub credit: Minor,
}

/// `trialBalance` result (`reports.ts:162-173`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrialBalance {
    /// Every account with a non-zero balance, by code.
    pub rows: Vec<TrialBalanceRow>,
    /// Total of the debit column.
    pub debit: Minor,
    /// Total of the credit column.
    pub credit: Minor,
    /// Debit minus credit. Always 0.
    pub difference: Minor,
}

/// `GeneralLedgerRow` (`reports.ts:175-179`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneralLedgerRow {
    /// The posted line.
    pub line: ReportLine,
    /// Its account; `None` (left out of the JSON, as TS leaves it `undefined`) for an account not
    /// in the chart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    /// The account's running balance after this line, signed debit minus credit.
    pub running: Minor,
}

/// `generalLedger` result (`reports.ts:181-199`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneralLedger {
    /// Lines by date, then entry number.
    pub rows: Vec<GeneralLedgerRow>,
    /// Balance per account before `from` (empty without `from`).
    pub opening: BTreeMap<AccountId, Minor>,
    /// Balance per account after the last row.
    pub closing: BTreeMap<AccountId, Minor>,
}

/// `StatementRow` (`reports.ts:213-216`): an account and its balance the natural way round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatementRow {
    /// The account.
    pub account: Account,
    /// Its balance, natural side positive.
    pub amount: Minor,
}

/// `FinancialPosition['netAssets']` (`reports.ts:222-226`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetAssets {
    /// Equity, income and expense on unrestricted funds.
    pub unrestricted: Minor,
    /// Equity, income and expense on restricted funds.
    pub restricted: Minor,
    /// The equity accounts with a balance, by code.
    pub rows: Vec<StatementRow>,
}

/// `FinancialPosition` (`reports.ts:218-232`): the Statement of Financial Position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialPosition {
    /// The date it is drawn up at.
    pub as_of: IsoDate,
    /// Asset accounts with a balance, by code.
    pub assets: Vec<StatementRow>,
    /// Liability accounts with a balance, by code.
    pub liabilities: Vec<StatementRow>,
    /// Net assets by restriction.
    pub net_assets: NetAssets,
    /// Sum of `assets`.
    pub total_assets: Minor,
    /// Sum of `liabilities`.
    pub total_liabilities: Minor,
    /// Unrestricted plus restricted net assets.
    pub total_net_assets: Minor,
    /// Assets minus (liabilities plus net assets). Always 0.
    pub difference: Minor,
}

/// `ActivityRow` (`reports.ts:280-285`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivityRow {
    /// The income or expense account.
    pub account: Account,
    /// On unrestricted funds, natural side positive.
    pub unrestricted: Minor,
    /// On restricted funds, natural side positive.
    pub restricted: Minor,
    /// Unrestricted plus restricted.
    pub total: Minor,
}

/// A total split by restriction (`reports.ts:292-294`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RestrictionTotals {
    /// Unrestricted funds.
    pub unrestricted: Minor,
    /// Restricted funds.
    pub restricted: Minor,
    /// Both.
    pub total: Minor,
}

/// One fund's line in the fund schedule (`reports.ts:296`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FundActivity {
    /// The fund.
    pub fund: Fund,
    /// Its income.
    pub income: Minor,
    /// Its expenses.
    pub expenses: Minor,
    /// Income minus expenses.
    pub change: Minor,
}

/// `Activities` (`reports.ts:287-297`): the Statement of Activities, by fund restriction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activities {
    /// First date.
    pub from: IsoDate,
    /// Last date.
    pub to: IsoDate,
    /// Income accounts with activity, by code.
    pub income: Vec<ActivityRow>,
    /// Expense accounts with activity, by code.
    pub expenses: Vec<ActivityRow>,
    /// Sum of `income`.
    pub total_income: RestrictionTotals,
    /// Sum of `expenses`.
    pub total_expenses: RestrictionTotals,
    /// Income minus expenses.
    pub change_in_net_assets: RestrictionTotals,
    /// Per fund with activity, in fund order: income, expense and change.
    pub by_fund: Vec<FundActivity>,
}

/// `FunctionalRow` (`reports.ts:361-367`): one expense account split by function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunctionalRow {
    /// The expense account.
    pub account: Account,
    /// Programme spend.
    pub program: Minor,
    /// Management and general.
    pub management: Minor,
    /// Fundraising.
    pub fundraising: Minor,
    /// All three.
    pub total: Minor,
}

/// `functionalExpenses(...).totals` (`reports.ts:383-388`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FunctionalTotals {
    /// Programme spend.
    pub program: Minor,
    /// Management and general.
    pub management: Minor,
    /// Fundraising.
    pub fundraising: Minor,
    /// All three.
    pub total: Minor,
}

/// `functionalExpenses` result (`reports.ts:369-395`): the Statement of Functional Expenses.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionalExpenses {
    /// Expense accounts with a non-zero total, by code.
    pub rows: Vec<FunctionalRow>,
    /// Column totals.
    pub totals: FunctionalTotals,
    /// Share of spend that went to programmes, 0 to 1 (0 when there is no spend).
    #[serde(with = "crate::js_number")]
    pub program_ratio: f64,
}

/// `CashFlow['operating']` (`reports.ts:404`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OperatingCash {
    /// Cash in.
    pub receipts: Minor,
    /// Cash out, negative.
    pub payments: Minor,
    /// Receipts plus payments.
    pub net: Minor,
}

/// `CashFlow` (`reports.ts:401-410`): the cash flow statement, direct method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CashFlow {
    /// Cash at the start of `from`.
    pub opening: Minor,
    /// Cash at the end of `to`.
    pub closing: Minor,
    /// Operating activities.
    pub operating: OperatingCash,
    /// Investing activities.
    pub investing: Minor,
    /// Financing activities.
    pub financing: Minor,
    /// Operating plus investing plus financing.
    pub net: Minor,
    /// Opening plus net minus closing. Always 0.
    pub difference: Minor,
}

/// `trialBalance(state, asOf, basis)` (`reports.ts:162-173`).
#[must_use]
pub fn trial_balance(state: &LedgerState, as_of: Option<&IsoDate>, basis: Basis) -> TrialBalance {
    let _ = (state, as_of, basis);
    todo!("reports.ts:162-173")
}

/// `generalLedger(state, f)` (`reports.ts:181-199`).
///
/// # Errors
///
/// [`ReportError::InvalidDate`] when `f.from` is not a calendar date.
pub fn general_ledger(state: &LedgerState, f: &LineFilter) -> Result<GeneralLedger, ReportError> {
    let _ = (state, f);
    todo!("reports.ts:181-199")
}

/// `financialPosition(state, asOf, basis)` (`reports.ts:238-274`).
#[must_use]
pub fn financial_position(state: &LedgerState, as_of: &IsoDate, basis: Basis) -> FinancialPosition {
    let _ = (state, as_of, basis);
    todo!("reports.ts:238-274")
}

/// `activities(state, from, to, basis)` (`reports.ts:299-355`).
#[must_use]
pub fn activities(state: &LedgerState, from: &IsoDate, to: &IsoDate, basis: Basis) -> Activities {
    let _ = (state, from, to, basis);
    todo!("reports.ts:299-355")
}

/// `functionalExpenses(state, from, to, basis)` (`reports.ts:369-395`).
#[must_use]
pub fn functional_expenses(
    state: &LedgerState,
    from: &IsoDate,
    to: &IsoDate,
    basis: Basis,
) -> FunctionalExpenses {
    let _ = (state, from, to, basis);
    todo!("reports.ts:369-395")
}

/// `cashFlow(state, from, to)` (`reports.ts:412-451`).
///
/// # Errors
///
/// [`ReportError::InvalidDate`] when `from` is not a calendar date.
pub fn cash_flow(
    state: &LedgerState,
    from: &IsoDate,
    to: &IsoDate,
) -> Result<CashFlow, ReportError> {
    let _ = (state, from, to);
    todo!("reports.ts:412-451")
}
