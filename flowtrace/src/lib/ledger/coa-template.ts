// Default chart of accounts for an NGO, presented the IFRS for SMEs way
// (Statement of Financial Position, Statement of Activities by fund) and
// tagged with functional classes for the Statement of Functional Expenses.
// Seeded for every new organisation; editable afterwards.
import { normalBalanceOf } from './engine'
import type {
  Account,
  AccountType,
  CashFlowClass,
  ExpenseFunction,
  OrgLedgerSettings,
} from './types'

type Row = [
  code: string,
  name: string,
  type: AccountType,
  opts?: {
    group?: boolean
    cash?: boolean
    fn?: ExpenseFunction
    cf?: CashFlowClass
  },
]

const ROWS: Array<Row> = [
  ['1000', 'Assets', 'asset', { group: true }],
  ['1100', 'Cash and cash equivalents', 'asset', { group: true }],
  ['1110', 'Bank: Main operating account', 'asset', { cash: true }],
  ['1120', 'Bank: Operations account', 'asset', { cash: true }],
  ['1130', 'Bank: Restricted projects account', 'asset', { cash: true }],
  ['1140', 'Bank: Payroll account', 'asset', { cash: true }],
  ['1150', 'M-Pesa till and float', 'asset', { cash: true }],
  ['1160', 'Petty cash', 'asset', { cash: true }],
  ['1200', 'Receivables', 'asset', { group: true }],
  ['1210', 'Grants receivable', 'asset'],
  ['1220', 'Staff advances', 'asset'],
  ['1300', 'Prepayments', 'asset'],
  ['1400', 'Suspense: unexplained movements', 'asset'],
  ['1500', 'Property and equipment', 'asset', { group: true }],
  ['1510', 'Vehicles', 'asset', { cf: 'investing' }],
  ['1520', 'Equipment', 'asset', { cf: 'investing' }],
  ['1590', 'Accumulated depreciation', 'asset', { cf: 'investing' }],

  ['2000', 'Liabilities', 'liability', { group: true }],
  ['2100', 'Accounts payable', 'liability'],
  ['2110', 'Staff claims payable', 'liability'],
  ['2200', 'Accrued expenses', 'liability'],
  ['2300', 'Deferred grant income', 'liability'],
  ['2400', 'Payroll liabilities (PAYE, NSSF, SHIF)', 'liability'],
  ['2500', 'Loans', 'liability', { cf: 'financing' }],

  ['3000', 'Net assets', 'equity', { group: true }],
  ['3100', 'Net assets without donor restrictions', 'equity'],
  ['3200', 'Net assets with donor restrictions', 'equity'],

  ['4000', 'Income', 'income', { group: true }],
  ['4100', 'Grant income', 'income'],
  ['4200', 'Donations', 'income'],
  ['4300', 'Other income', 'income'],
  ['4400', 'Exchange gains and losses', 'income'],

  ['5000', 'Programme expenses', 'expense', { group: true, fn: 'program' }],
  ['5100', 'Programme: Water and sanitation', 'expense', { fn: 'program' }],
  ['5200', 'Programme: Health', 'expense', { fn: 'program' }],
  ['5300', 'Programme: Education', 'expense', { fn: 'program' }],
  ['5400', 'Stipends and transfers to beneficiaries', 'expense', { fn: 'program' }],
  ['5500', 'Programme equipment and supplies', 'expense', { fn: 'program' }],
  ['5600', 'Training and workshops', 'expense', { fn: 'program' }],
  ['5700', 'Field travel', 'expense', { fn: 'program' }],

  ['6000', 'Staff costs', 'expense', { group: true, fn: 'management' }],
  ['6100', 'Salaries and wages', 'expense', { fn: 'management' }],
  ['6200', 'Staff benefits', 'expense', { fn: 'management' }],

  ['7000', 'Running costs', 'expense', { group: true, fn: 'management' }],
  ['7100', 'Office supplies', 'expense', { fn: 'management' }],
  ['7200', 'Vehicle running and maintenance', 'expense', { fn: 'management' }],
  ['7300', 'Utilities', 'expense', { fn: 'management' }],
  ['7400', 'Communication and mobile money fees', 'expense', { fn: 'management' }],
  ['7500', 'Rent', 'expense', { fn: 'management' }],
  ['7600', 'Audit and professional fees', 'expense', { fn: 'management' }],
  ['7700', 'Bank charges', 'expense', { fn: 'management' }],
  ['7800', 'Depreciation', 'expense', { fn: 'management' }],

  ['8000', 'Fundraising', 'expense', { group: true, fn: 'fundraising' }],
  ['8100', 'Fundraising and campaigns', 'expense', { fn: 'fundraising' }],
]

export const accountId = (code: string) => `acc-${code}`

/** Parent: the nearest shorter code that is a group (1110 -> 1100 -> 1000). */
function parentOf(code: string, groups: Set<string>) {
  for (const candidate of [
    code.slice(0, 2) + '00',
    code.slice(0, 1) + '000',
  ]) {
    if (candidate !== code && groups.has(candidate)) return candidate
  }
  return null
}

export function ngoChartOfAccounts(): Array<Account> {
  const groups = new Set(ROWS.filter((r) => r[3]?.group).map((r) => r[0]))
  return ROWS.map(([code, name, type, o = {}]) => {
    const parent = parentOf(code, groups)
    return {
      id: accountId(code),
      code,
      name,
      type,
      normal_balance: normalBalanceOf(type),
      parent_id: parent ? accountId(parent) : null,
      is_group: !!o.group,
      is_active: true,
      is_cash: !!o.cash,
      expense_function: type === 'expense' ? (o.fn ?? 'program') : null,
      cash_flow_class: o.cf ?? (type === 'equity' ? 'financing' : 'operating'),
    }
  })
}

export function defaultSettings(
  organizationId: string,
  baseCurrency = 'KES',
): OrgLedgerSettings {
  return {
    organization_id: organizationId,
    base_currency: baseCurrency,
    fiscal_year_start_month: 1,
    basis: 'accrual',
    reporting_standard: 'ifrs_sme',
    system_accounts: {
      accounts_payable: accountId('2100'),
      claims_payable: accountId('2110'),
      suspense: accountId('1400'),
      net_assets_unrestricted: accountId('3100'),
      net_assets_restricted: accountId('3200'),
      fx_gain_loss: accountId('4400'),
    },
  }
}

export const UNRESTRICTED_FUND_ID = 'fund-unrestricted'
