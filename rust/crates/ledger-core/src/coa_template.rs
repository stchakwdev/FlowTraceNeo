//! The default chart of accounts for an NGO and the default settings, mirroring
//! `flowtrace/src/lib/ledger/coa-template.ts`: presented the IFRS for SMEs way (Statement of
//! Financial Position, Statement of Activities by fund) and tagged with functional classes for
//! the Statement of Functional Expenses; seeded for every new organisation and editable
//! afterwards (`coa-template.ts:1-4`).

use crate::engine::normal_balance_of;
use crate::types::{
    Account, AccountId, AccountType, Basis, CashFlowClass, ExpenseFunction, OrgLedgerSettings,
    OrganizationId, ReportingStandard, SystemAccounts,
};

/// `UNRESTRICTED_FUND_ID` (`coa-template.ts:144`): the fund every new organisation starts with.
pub const UNRESTRICTED_FUND_ID: &str = "fund-unrestricted";

/// `accountId(code)` (`coa-template.ts:89`): `acc-<code>`.
#[must_use]
pub fn account_id(code: &str) -> AccountId {
    AccountId::from(format!("acc-{code}"))
}

/// One row of the template (`coa-template.ts:14-23`): code, name, type, header, cash, function
/// and cash-flow class.
struct Row {
    code: &'static str,
    name: &'static str,
    account_type: AccountType,
    group: bool,
    cash: bool,
    function: Option<ExpenseFunction>,
    cash_flow: Option<CashFlowClass>,
}

const fn row(
    code: &'static str,
    name: &'static str,
    account_type: AccountType,
    group: bool,
    cash: bool,
    function: Option<ExpenseFunction>,
    cash_flow: Option<CashFlowClass>,
) -> Row {
    Row {
        code,
        name,
        account_type,
        group,
        cash,
        function,
        cash_flow,
    }
}

/// `ROWS` (`coa-template.ts:25-87`), in order.
const ROWS: [Row; 54] = [
    row(
        "1000",
        "Assets",
        AccountType::Asset,
        true,
        false,
        None,
        None,
    ),
    row(
        "1100",
        "Cash and cash equivalents",
        AccountType::Asset,
        true,
        false,
        None,
        None,
    ),
    row(
        "1110",
        "Bank: Main operating account",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1120",
        "Bank: Operations account",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1130",
        "Bank: Restricted projects account",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1140",
        "Bank: Payroll account",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1150",
        "M-Pesa till and float",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1160",
        "Petty cash",
        AccountType::Asset,
        false,
        true,
        None,
        None,
    ),
    row(
        "1200",
        "Receivables",
        AccountType::Asset,
        true,
        false,
        None,
        None,
    ),
    row(
        "1210",
        "Grants receivable",
        AccountType::Asset,
        false,
        false,
        None,
        None,
    ),
    row(
        "1220",
        "Staff advances",
        AccountType::Asset,
        false,
        false,
        None,
        None,
    ),
    row(
        "1300",
        "Prepayments",
        AccountType::Asset,
        false,
        false,
        None,
        None,
    ),
    row(
        "1400",
        "Suspense: unexplained movements",
        AccountType::Asset,
        false,
        false,
        None,
        None,
    ),
    row(
        "1500",
        "Property and equipment",
        AccountType::Asset,
        true,
        false,
        None,
        None,
    ),
    row(
        "1510",
        "Vehicles",
        AccountType::Asset,
        false,
        false,
        None,
        Some(CashFlowClass::Investing),
    ),
    row(
        "1520",
        "Equipment",
        AccountType::Asset,
        false,
        false,
        None,
        Some(CashFlowClass::Investing),
    ),
    row(
        "1590",
        "Accumulated depreciation",
        AccountType::Asset,
        false,
        false,
        None,
        Some(CashFlowClass::Investing),
    ),
    row(
        "2000",
        "Liabilities",
        AccountType::Liability,
        true,
        false,
        None,
        None,
    ),
    row(
        "2100",
        "Accounts payable",
        AccountType::Liability,
        false,
        false,
        None,
        None,
    ),
    row(
        "2110",
        "Staff claims payable",
        AccountType::Liability,
        false,
        false,
        None,
        None,
    ),
    row(
        "2200",
        "Accrued expenses",
        AccountType::Liability,
        false,
        false,
        None,
        None,
    ),
    row(
        "2300",
        "Deferred grant income",
        AccountType::Liability,
        false,
        false,
        None,
        None,
    ),
    row(
        "2400",
        "Payroll liabilities (PAYE, NSSF, SHIF)",
        AccountType::Liability,
        false,
        false,
        None,
        None,
    ),
    row(
        "2500",
        "Loans",
        AccountType::Liability,
        false,
        false,
        None,
        Some(CashFlowClass::Financing),
    ),
    row(
        "3000",
        "Net assets",
        AccountType::Equity,
        true,
        false,
        None,
        None,
    ),
    row(
        "3100",
        "Net assets without donor restrictions",
        AccountType::Equity,
        false,
        false,
        None,
        None,
    ),
    row(
        "3200",
        "Net assets with donor restrictions",
        AccountType::Equity,
        false,
        false,
        None,
        None,
    ),
    row(
        "4000",
        "Income",
        AccountType::Income,
        true,
        false,
        None,
        None,
    ),
    row(
        "4100",
        "Grant income",
        AccountType::Income,
        false,
        false,
        None,
        None,
    ),
    row(
        "4200",
        "Donations",
        AccountType::Income,
        false,
        false,
        None,
        None,
    ),
    row(
        "4300",
        "Other income",
        AccountType::Income,
        false,
        false,
        None,
        None,
    ),
    row(
        "4400",
        "Exchange gains and losses",
        AccountType::Income,
        false,
        false,
        None,
        None,
    ),
    row(
        "5000",
        "Programme expenses",
        AccountType::Expense,
        true,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5100",
        "Programme: Water and sanitation",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5200",
        "Programme: Health",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5300",
        "Programme: Education",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5400",
        "Stipends and transfers to beneficiaries",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5500",
        "Programme equipment and supplies",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5600",
        "Training and workshops",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "5700",
        "Field travel",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Program),
        None,
    ),
    row(
        "6000",
        "Staff costs",
        AccountType::Expense,
        true,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "6100",
        "Salaries and wages",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "6200",
        "Staff benefits",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7000",
        "Running costs",
        AccountType::Expense,
        true,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7100",
        "Office supplies",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7200",
        "Vehicle running and maintenance",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7300",
        "Utilities",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7400",
        "Communication and mobile money fees",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7500",
        "Rent",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7600",
        "Audit and professional fees",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7700",
        "Bank charges",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "7800",
        "Depreciation",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Management),
        None,
    ),
    row(
        "8000",
        "Fundraising",
        AccountType::Expense,
        true,
        false,
        Some(ExpenseFunction::Fundraising),
        None,
    ),
    row(
        "8100",
        "Fundraising and campaigns",
        AccountType::Expense,
        false,
        false,
        Some(ExpenseFunction::Fundraising),
        None,
    ),
];

/// `parentOf(code, groups)` (`coa-template.ts:91-100`): the nearest shorter code that is a
/// header (`1110` to `1100` to `1000`).
fn parent_of(code: &str) -> Option<&'static str> {
    let prefix = |n: usize| code.get(..n).unwrap_or("");
    [format!("{}00", prefix(2)), format!("{}000", prefix(1))]
        .into_iter()
        .filter(|candidate| candidate != code)
        .find_map(|candidate| {
            ROWS.iter()
                .find(|r| r.group && r.code == candidate)
                .map(|r| r.code)
        })
}

/// `ngoChartOfAccounts()` (`coa-template.ts:102-121`): the 54 accounts, active, each under its
/// nearest header; an expense account's function defaults to `program`; the cash-flow class is
/// the row's, else `financing` for equity and `operating` for the rest.
#[must_use]
pub fn ngo_chart_of_accounts() -> Vec<Account> {
    ROWS.iter()
        .map(|r| Account {
            id: account_id(r.code),
            code: r.code.to_owned(),
            name: r.name.to_owned(),
            account_type: r.account_type,
            normal_balance: normal_balance_of(r.account_type),
            parent_id: parent_of(r.code).map(account_id),
            is_group: r.group,
            is_active: true,
            is_cash: r.cash,
            expense_function: (r.account_type == AccountType::Expense)
                .then(|| r.function.unwrap_or(ExpenseFunction::Program)),
            cash_flow_class: Some(r.cash_flow.unwrap_or(
                if r.account_type == AccountType::Equity {
                    CashFlowClass::Financing
                } else {
                    CashFlowClass::Operating
                },
            )),
        })
        .collect()
}

/// `defaultSettings(organizationId, baseCurrency = 'KES')` (`coa-template.ts:123-142`): accrual
/// basis, IFRS for SMEs, a January year, and the system accounts of the template.
#[must_use]
pub fn default_settings(organization_id: &str, base_currency: &str) -> OrgLedgerSettings {
    OrgLedgerSettings {
        organization_id: OrganizationId::from(organization_id),
        base_currency: base_currency.to_owned(),
        fiscal_year_start_month: 1,
        basis: Basis::Accrual,
        reporting_standard: ReportingStandard::IfrsSme,
        system_accounts: SystemAccounts {
            accounts_payable: account_id("2100"),
            claims_payable: account_id("2110"),
            suspense: account_id("1400"),
            net_assets_unrestricted: account_id("3100"),
            net_assets_restricted: account_id("3200"),
            fx_gain_loss: account_id("4400"),
        },
    }
}
