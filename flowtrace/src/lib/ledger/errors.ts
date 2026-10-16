// Every rule the ledger enforces has a code, so a UI can point at the field
// and the SpacetimeDB module can return the same message the demo does.

export type IssueCode =
  | 'PERMISSION'
  | 'NOT_FOUND'
  | 'INVALID_STATE'
  | 'TOO_FEW_LINES'
  | 'LINE_BOTH_SIDES'
  | 'LINE_ZERO'
  | 'NEGATIVE_AMOUNT'
  | 'NOT_MINOR_UNITS'
  | 'UNKNOWN_ACCOUNT'
  | 'GROUP_ACCOUNT'
  | 'INACTIVE_ACCOUNT'
  | 'UNKNOWN_FUND'
  | 'UNKNOWN_GRANT'
  | 'UNKNOWN_DIMENSION'
  | 'FX_RATE_REQUIRED'
  | 'NO_PERIOD'
  | 'PERIOD_CLOSED'
  | 'PERIOD_LOCKED'
  | 'UNBALANCED'
  | 'FUND_UNBALANCED'
  | 'RESTRICTED_NEEDS_GRANT'
  | 'GRANT_FUND_MISMATCH'
  | 'GRANT_NOT_STARTED'
  | 'GRANT_ENDED'
  | 'GRANT_OVER_BUDGET'
  | 'SELF_APPROVAL'
  | 'DUPLICATE'
  | 'INVALID_INPUT'

export interface Issue {
  code: IssueCode
  message: string
  /** Index of the offending line, when the issue belongs to one. */
  line?: number
  /** True when `override_reason` lets the entry post anyway. */
  overridable?: boolean
}

export class LedgerError extends Error {
  readonly issues: Array<Issue>
  constructor(issues: Array<Issue> | Issue) {
    const list = Array.isArray(issues) ? issues : [issues]
    super(list.map((i) => i.message).join(' '))
    this.name = 'LedgerError'
    this.issues = list
  }
  get code(): IssueCode {
    return this.issues[0].code
  }
}

export const fail = (code: IssueCode, message: string): never => {
  throw new LedgerError({ code, message })
}
