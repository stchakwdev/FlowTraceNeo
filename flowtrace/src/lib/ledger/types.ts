// Ledger domain types. Plain data, no framework imports: the same types are
// used by the in-memory demo ledger and by the SpacetimeDB module, so both
// enforce one set of rules.
//
// Money is always an integer count of minor units (cents) in a `number`,
// checked with Number.isSafeInteger. There are no floats in stored amounts.

export type Minor = number
export type ISODate = string // '2026-01-31'

export type AccountType = 'asset' | 'liability' | 'equity' | 'income' | 'expense'
export type NormalBalance = 'debit' | 'credit'

/** Functional classification, for the Statement of Functional Expenses. */
export type ExpenseFunction = 'program' | 'management' | 'fundraising'

/** Cash-flow classification, for the cash flow statement. */
export type CashFlowClass = 'operating' | 'investing' | 'financing'

export interface Account {
  id: string
  code: string
  name: string
  type: AccountType
  normal_balance: NormalBalance
  parent_id: string | null
  /** Header accounts group others and cannot be posted to. */
  is_group: boolean
  is_active: boolean
  /** Bank, mobile money or petty cash: counts as cash. */
  is_cash: boolean
  expense_function: ExpenseFunction | null
  cash_flow_class: CashFlowClass | null
}

export type FundRestriction =
  | 'unrestricted'
  | 'temporarily_restricted'
  | 'permanently_restricted'

export interface Fund {
  id: string
  name: string
  restriction: FundRestriction
}

export interface Grant {
  id: string
  name: string
  /** The donor flow node, so the flow map and the ledger meet. */
  donor_node_id: string | null
  donor_name: string
  fund_id: string
  award: Minor
  currency: string
  start_date: ISODate
  end_date: ISODate
  terms: string
}

export type DimensionKind = 'project' | 'department' | 'location' | 'donor'

export interface DimensionValue {
  id: string
  kind: DimensionKind
  name: string
  /** The flow node this value stands for, if any. */
  node_id: string | null
}

export type PeriodStatus = 'open' | 'closed' | 'locked'

export interface FiscalPeriod {
  id: string
  name: string // 'Jan 2026'
  fiscal_year: number
  start_date: ISODate
  end_date: ISODate
  status: PeriodStatus
}

export type EntryStatus = 'draft' | 'posted' | 'reversed'

export type SourceType =
  | 'manual'
  | 'transaction'
  | 'bill'
  | 'payment'
  | 'expense_claim'
  | 'reimbursement'
  | 'statement'
  | 'opening'
  | 'year_end_close'
  | 'reversal'
  | 'import'

export interface JournalLine {
  id: string
  entry_id: string
  account_id: string
  /** One side is > 0 and the other is 0, in transaction currency. */
  debit: Minor
  credit: Minor
  currency: string
  /** Units of base currency per unit of `currency`. 1 for base currency. */
  fx_rate: number
  /** Signed debit-minus-credit in base currency, frozen when posted. */
  base_amount: Minor
  fund_id: string
  grant_id: string | null
  dimension_ids: Array<string>
  flow_node_id: string | null
  memo: string | null
}

export interface JournalEntry {
  id: string
  number: string // 'JE-0001'
  date: ISODate
  period_id: string
  memo: string
  source_type: SourceType
  source_id: string | null
  status: EntryStatus
  reversal_of: string | null
  reversed_by: string | null
  /** Set when a restricted-fund rule was overridden. */
  override_reason: string | null
  created_by: string
  created_at: string
  posted_by: string | null
  posted_at: string | null
}

export interface BudgetLine {
  id: string
  budget_id: string
  /** Null means every expense account. */
  account_id: string | null
  fund_id: string | null
  grant_id: string | null
  dimension_id: string | null
  /** Null means the whole fiscal year. */
  period_id: string | null
  amount: Minor
}

export interface Budget {
  id: string
  name: string
  fiscal_year: number
  status: 'draft' | 'approved'
}

export interface Vendor {
  id: string
  name: string
  node_id: string | null
  payment_details: string | null
}

export type BillStatus = 'draft' | 'approved' | 'posted' | 'paid' | 'void'

export interface CodedLine {
  id: string
  description: string
  account_id: string
  amount: Minor
  fund_id: string
  grant_id: string | null
  dimension_ids: Array<string>
}

export interface Bill {
  id: string
  vendor_id: string
  number: string
  bill_date: ISODate
  due_date: ISODate
  currency: string
  fx_rate: number
  status: BillStatus
  lines: Array<CodedLine>
  entry_id: string | null
  /** Proof references, matching verifications on the source document. */
  proof_ids: Array<string>
  created_by: string
  approved_by: string | null
}

export interface Payment {
  id: string
  date: ISODate
  /** The cash account money left from. */
  cash_account_id: string
  amount: Minor
  currency: string
  fx_rate: number
  reference: string | null
  applies_to: { kind: 'bill' | 'expense_claim'; id: string }
  entry_id: string
  created_by: string
}

export type ClaimStatus =
  | 'draft'
  | 'submitted'
  | 'approved'
  | 'rejected'
  | 'posted'
  | 'reimbursed'

export interface ApprovalStep {
  at: string
  by: string
  action: 'submitted' | 'approved' | 'rejected' | 'posted' | 'reimbursed'
  note: string | null
}

export interface ExpenseClaim {
  id: string
  number: string
  claimant: string
  claimant_name: string
  claim_date: ISODate
  currency: string
  fx_rate: number
  purpose: string
  lines: Array<CodedLine>
  status: ClaimStatus
  approvals: Array<ApprovalStep>
  receipt_names: Array<string>
  entry_id: string | null
  /** Set when captured offline and synced later. */
  captured_offline: boolean
}

export interface BankAccount {
  id: string
  name: string
  kind: 'bank' | 'mobile_money' | 'cash'
  provider: string // 'Equity Bank', 'Safaricom M-Pesa'
  account_id: string // the GL cash account
  currency: string
  number_masked: string
}

export interface StatementLine {
  id: string
  bank_account_id: string
  date: ISODate
  description: string
  reference: string | null
  /** Signed: money in > 0, money out < 0, in the account's currency. */
  amount: Minor
  import_batch: string
  /** The journal line this statement line is matched to. */
  matched_line_id: string | null
  match_kind: 'auto' | 'manual' | null
}

export interface AuditEvent {
  id: string
  at: string
  by: string
  action: string
  subject: string
  detail: string
}

export interface OrgLedgerSettings {
  organization_id: string
  base_currency: string
  fiscal_year_start_month: number // 1 = January
  /** Recommended default: accrual books, with a cash-basis report view. */
  basis: 'accrual'
  /** Recommended default: IFRS for SMEs presentation. */
  reporting_standard: 'ifrs_sme' | 'ipsas'
  /** System accounts the engine posts to. */
  system_accounts: {
    accounts_payable: string
    claims_payable: string
    suspense: string
    net_assets_unrestricted: string
    net_assets_restricted: string
    fx_gain_loss: string
  }
}

export interface LedgerState {
  settings: OrgLedgerSettings
  accounts: Array<Account>
  funds: Array<Fund>
  grants: Array<Grant>
  dimensions: Array<DimensionValue>
  periods: Array<FiscalPeriod>
  entries: Array<JournalEntry>
  lines: Array<JournalLine>
  budgets: Array<Budget>
  budgetLines: Array<BudgetLine>
  vendors: Array<Vendor>
  bills: Array<Bill>
  payments: Array<Payment>
  claims: Array<ExpenseClaim>
  bankAccounts: Array<BankAccount>
  statementLines: Array<StatementLine>
  audit: Array<AuditEvent>
  counters: Record<string, number>
}

/** Who is acting, and what they may do. Checked by every command. */
export interface Actor {
  id: string
  name: string
  can_post: boolean
  can_approve: boolean
  can_close: boolean
}

export interface LineInput {
  account_id: string
  debit?: Minor
  credit?: Minor
  currency?: string
  fx_rate?: number
  fund_id: string
  grant_id?: string | null
  dimension_ids?: Array<string>
  flow_node_id?: string | null
  memo?: string | null
}

export interface EntryInput {
  date: ISODate
  memo: string
  lines: Array<LineInput>
  source_type?: SourceType
  source_id?: string | null
  /** Allows a restricted-fund rule to be overridden, with a reason. */
  override_reason?: string | null
}
