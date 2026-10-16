//! The trial balance, general ledger and the financial statements (`reports.ts:152-451`).

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use super::{
    Basis, LineFilter, Posted, ReportError, ReportLine, Scope, account_map, balance_map, by_code,
    day_before, js, natural, owned_balances, posted, restricted_funds,
};
use crate::money::{Minor, sum};
use crate::types::{
    Account, AccountId, AccountType, CashFlowClass, EntryId, ExpenseFunction, Fund, FundId,
    IsoDate, LedgerState,
};

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

/// `trialBalance(state, asOf, basis)` (`reports.ts:162-173`): every account with a non-zero
/// balance up to `as_of` (all dates when `None`), by code, on its debit or credit side.
#[must_use]
pub fn trial_balance(state: &LedgerState, as_of: Option<&IsoDate>, basis: Basis) -> TrialBalance {
    let lines = posted(
        state,
        &Scope {
            to: as_of,
            basis,
            ..Scope::default()
        },
    );
    let bal = balance_map(&lines);
    let rows: Vec<TrialBalanceRow> = by_code(&state.accounts)
        .into_iter()
        .filter_map(|a| {
            let b = bal.get(&a.id).copied().unwrap_or(0);
            (b != 0).then(|| TrialBalanceRow {
                account: a.clone(),
                debit: b.max(0),
                credit: (-b).max(0),
            })
        })
        .collect();
    let debit = sum(rows.iter().map(|r| r.debit));
    let credit = sum(rows.iter().map(|r| r.credit));
    TrialBalance {
        rows,
        debit,
        credit,
        difference: debit - credit,
    }
}

/// `generalLedger(state, f)` (`reports.ts:181-199`): the posted lines of `f` by date, then entry
/// number (stable, byte order as `localeCompare` gives here), each with its account's running
/// balance. With `f.from`, running balances start from the balances of `f` without `from` up to
/// the day before it.
///
/// # Errors
///
/// [`ReportError::InvalidDate`] when `f.from` is not a calendar date.
pub fn general_ledger(state: &LedgerState, f: &LineFilter) -> Result<GeneralLedger, ReportError> {
    let scope = f.scope();
    let mut lines = posted(state, &scope);
    lines.sort_by(|a, b| {
        a.entry
            .date
            .cmp(&b.entry.date)
            .then_with(|| a.entry.number.cmp(&b.entry.number))
    });
    let accounts = account_map(state);
    let opening = match &f.from {
        Some(from) => {
            let to = day_before(from)?;
            let before = Scope {
                from: None,
                to: Some(&to),
                ..scope
            };
            owned_balances(&posted(state, &before))
        }
        None => BTreeMap::new(),
    };
    let mut running = opening.clone();
    let rows: Vec<GeneralLedgerRow> = lines
        .into_iter()
        .map(|p| {
            let id = &p.line.account_id;
            let r = running.get(id).copied().unwrap_or(0) + p.line.base_amount;
            running.insert(id.clone(), r);
            let account = accounts.get(id).map(|a| Account::clone(a));
            GeneralLedgerRow {
                line: p.into_report_line(),
                account,
                running: r,
            }
        })
        .collect();
    Ok(GeneralLedger {
        rows,
        opening,
        closing: running,
    })
}

/// `financialPosition(state, asOf, basis)` (`reports.ts:238-274`): assets, liabilities and
/// equity accounts with a balance at `as_of`, by code, natural side positive; net assets are
/// equity, income and expense together, split by the line's fund restriction.
#[must_use]
pub fn financial_position(state: &LedgerState, as_of: &IsoDate, basis: Basis) -> FinancialPosition {
    let lines = posted(
        state,
        &Scope {
            to: Some(as_of),
            basis,
            ..Scope::default()
        },
    );
    let accounts = account_map(state);
    let restricted = restricted_funds(state);
    let mut bal: HashMap<&AccountId, Minor> = HashMap::new();
    let (mut unrestricted, mut restricted_na) = (0, 0);
    for p in &lines {
        let Some(a) = accounts.get(&p.line.account_id) else {
            continue;
        };
        let base = p.line.base_amount;
        *bal.entry(&a.id).or_insert(0) += base;
        if matches!(
            a.account_type,
            AccountType::Equity | AccountType::Income | AccountType::Expense
        ) {
            if restricted.contains(&p.line.fund_id) {
                restricted_na -= base;
            } else {
                unrestricted -= base;
            }
        }
    }
    let chart = by_code(&state.accounts);
    let rows_of = |t: AccountType| -> Vec<StatementRow> {
        chart
            .iter()
            .filter(|a| a.account_type == t)
            .filter_map(|a| {
                let b = bal.get(&a.id).copied().unwrap_or(0);
                (b != 0).then(|| StatementRow {
                    account: Account::clone(a),
                    amount: natural(a.normal_balance, b),
                })
            })
            .collect()
    };
    let assets = rows_of(AccountType::Asset);
    let liabilities = rows_of(AccountType::Liability);
    let total_assets = sum(assets.iter().map(|r| r.amount));
    let total_liabilities = sum(liabilities.iter().map(|r| r.amount));
    let total_net_assets = unrestricted + restricted_na;
    FinancialPosition {
        as_of: as_of.clone(),
        assets,
        liabilities,
        net_assets: NetAssets {
            unrestricted,
            restricted: restricted_na,
            rows: rows_of(AccountType::Equity),
        },
        total_assets,
        total_liabilities,
        total_net_assets,
        difference: total_assets - total_liabilities - total_net_assets,
    }
}

/// `activities(state, from, to, basis)` (`reports.ts:299-355`): income and expense in the range,
/// closing entries left out, natural side positive, split by the line's fund restriction; rows
/// by code, without those whose total and unrestricted part are both 0; the fund schedule in
/// fund order.
#[must_use]
pub fn activities(state: &LedgerState, from: &IsoDate, to: &IsoDate, basis: Basis) -> Activities {
    let lines = posted(
        state,
        &Scope {
            from: Some(from),
            to: Some(to),
            basis,
            exclude_closing: true,
            ..Scope::default()
        },
    );
    let accounts = account_map(state);
    let restricted = restricted_funds(state);
    // (unrestricted, restricted) per account and (income, expenses) per fund.
    let mut cells: HashMap<&AccountId, (Minor, Minor)> = HashMap::new();
    let mut fund_totals: HashMap<&FundId, (Minor, Minor)> = HashMap::new();
    for p in &lines {
        let Some(a) = accounts.get(&p.line.account_id) else {
            continue;
        };
        let income = match a.account_type {
            AccountType::Income => true,
            AccountType::Expense => false,
            _ => continue,
        };
        let v = natural(a.normal_balance, p.line.base_amount);
        let cell = cells.entry(&a.id).or_insert((0, 0));
        if restricted.contains(&p.line.fund_id) {
            cell.1 += v;
        } else {
            cell.0 += v;
        }
        let fund = fund_totals.entry(&p.line.fund_id).or_insert((0, 0));
        if income {
            fund.0 += v;
        } else {
            fund.1 += v;
        }
    }
    let chart = by_code(&state.accounts);
    let rows_of = |t: AccountType| -> Vec<ActivityRow> {
        chart
            .iter()
            .filter(|a| a.account_type == t)
            .filter_map(|a| {
                let &(u, r) = cells.get(&a.id)?;
                (u + r != 0 || u != 0).then(|| ActivityRow {
                    account: Account::clone(a),
                    unrestricted: u,
                    restricted: r,
                    total: u + r,
                })
            })
            .collect()
    };
    let income = rows_of(AccountType::Income);
    let expenses = rows_of(AccountType::Expense);
    let total_income = restriction_totals(&income);
    let total_expenses = restriction_totals(&expenses);
    let by_fund = state
        .funds
        .iter()
        .filter_map(|fund| {
            let &(income, expenses) = fund_totals.get(&fund.id)?;
            Some(FundActivity {
                fund: fund.clone(),
                income,
                expenses,
                change: income - expenses,
            })
        })
        .collect();
    Activities {
        from: from.clone(),
        to: to.clone(),
        income,
        expenses,
        total_income,
        total_expenses,
        change_in_net_assets: RestrictionTotals {
            unrestricted: total_income.unrestricted - total_expenses.unrestricted,
            restricted: total_income.restricted - total_expenses.restricted,
            total: total_income.total - total_expenses.total,
        },
        by_fund,
    }
}

/// The `tot` helper of `activities` (`reports.ts:329-333`).
fn restriction_totals(rows: &[ActivityRow]) -> RestrictionTotals {
    RestrictionTotals {
        unrestricted: sum(rows.iter().map(|r| r.unrestricted)),
        restricted: sum(rows.iter().map(|r| r.restricted)),
        total: sum(rows.iter().map(|r| r.total)),
    }
}

/// `functionalExpenses(state, from, to, basis)` (`reports.ts:369-395`): expense in the range,
/// closing entries left out, by the account's function (`program` when it has none), signed
/// debit minus credit; rows with a non-zero total, by code.
#[must_use]
pub fn functional_expenses(
    state: &LedgerState,
    from: &IsoDate,
    to: &IsoDate,
    basis: Basis,
) -> FunctionalExpenses {
    let lines = posted(
        state,
        &Scope {
            from: Some(from),
            to: Some(to),
            basis,
            exclude_closing: true,
            ..Scope::default()
        },
    );
    let accounts = account_map(state);
    // Rows in the order accounts first appear, as the TS `Map` keeps them.
    let mut rows: Vec<FunctionalRow> = Vec::new();
    let mut index: HashMap<&AccountId, usize> = HashMap::new();
    for p in &lines {
        let Some(a) = accounts.get(&p.line.account_id) else {
            continue;
        };
        if a.account_type != AccountType::Expense {
            continue;
        }
        let i = *index.entry(&a.id).or_insert_with(|| {
            rows.push(FunctionalRow {
                account: Account::clone(a),
                program: 0,
                management: 0,
                fundraising: 0,
                total: 0,
            });
            rows.len() - 1
        });
        let Some(row) = rows.get_mut(i) else {
            continue;
        };
        let base = p.line.base_amount;
        match a.expense_function.unwrap_or(ExpenseFunction::Program) {
            ExpenseFunction::Program => row.program += base,
            ExpenseFunction::Management => row.management += base,
            ExpenseFunction::Fundraising => row.fundraising += base,
        }
        row.total += base;
    }
    rows.retain(|r| r.total != 0);
    rows.sort_by(|a, b| a.account.code.cmp(&b.account.code));
    let totals = FunctionalTotals {
        program: sum(rows.iter().map(|r| r.program)),
        management: sum(rows.iter().map(|r| r.management)),
        fundraising: sum(rows.iter().map(|r| r.fundraising)),
        total: sum(rows.iter().map(|r| r.total)),
    };
    let program_ratio = if totals.total > 0 {
        js(totals.program) / js(totals.total)
    } else {
        0.0
    };
    FunctionalExpenses {
        rows,
        totals,
        program_ratio,
    }
}

/// `cashFlow(state, from, to)` (`reports.ts:412-451`), direct method on accrual lines: each
/// entry that moves cash in the range is classed by its largest non-cash line (the first of
/// equal ones), by that account's cash-flow class, operating when it has none; operating cash
/// splits into receipts and payments by sign. Transfers between cash accounts move nothing.
///
/// # Errors
///
/// [`ReportError::InvalidDate`] when `from` is not a calendar date.
pub fn cash_flow(
    state: &LedgerState,
    from: &IsoDate,
    to: &IsoDate,
) -> Result<CashFlow, ReportError> {
    let accounts = account_map(state);
    let is_cash = |p: &Posted<'_>| accounts.get(&p.line.account_id).is_some_and(|a| a.is_cash);
    let cash_balance = |up_to: &IsoDate| {
        let lines = posted(
            state,
            &Scope {
                to: Some(up_to),
                ..Scope::default()
            },
        );
        sum(lines
            .iter()
            .filter(|p| is_cash(p))
            .map(|p| p.line.base_amount))
    };
    let opening = cash_balance(&day_before(from)?);
    let lines = posted(
        state,
        &Scope {
            from: Some(from),
            to: Some(to),
            ..Scope::default()
        },
    );
    // Lines grouped by entry, entries in the order they first appear.
    let mut groups: Vec<Vec<&Posted<'_>>> = Vec::new();
    let mut index: HashMap<&EntryId, usize> = HashMap::new();
    for p in &lines {
        let i = *index.entry(&p.line.entry_id).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        if let Some(group) = groups.get_mut(i) {
            group.push(p);
        }
    }
    let (mut receipts, mut payments, mut investing, mut financing) = (0, 0, 0, 0);
    for group in &groups {
        let cash_move = sum(group
            .iter()
            .filter(|p| is_cash(p))
            .map(|p| p.line.base_amount));
        if cash_move == 0 {
            continue;
        }
        let mut counter: Option<&Posted<'_>> = None;
        for &p in group.iter().filter(|p| !is_cash(p)) {
            if counter.is_none_or(|c| p.line.base_amount.abs() > c.line.base_amount.abs()) {
                counter = Some(p);
            }
        }
        let class = counter
            .and_then(|c| accounts.get(&c.line.account_id))
            .and_then(|a| a.cash_flow_class)
            .unwrap_or(CashFlowClass::Operating);
        match class {
            CashFlowClass::Investing => investing += cash_move,
            CashFlowClass::Financing => financing += cash_move,
            CashFlowClass::Operating if cash_move > 0 => receipts += cash_move,
            CashFlowClass::Operating => payments += cash_move,
        }
    }
    let operating = receipts + payments;
    let net = operating + investing + financing;
    let closing = cash_balance(to);
    Ok(CashFlow {
        opening,
        closing,
        operating: OperatingCash {
            receipts,
            payments,
            net: operating,
        },
        investing,
        financing,
        net,
        difference: opening + net - closing,
    })
}
