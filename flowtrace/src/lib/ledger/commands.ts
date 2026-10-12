// Every ledger write as a serialisable command. The demo store applies them
// in memory; the SpacetimeDB module applies the same command in a reducer.
// One dispatcher means one set of rules, wherever the books live.
import { autoMatch } from './reconcile'
import type { AutoMatchResult } from './reconcile'
import type { Ledger } from './engine'
import type {
  Account,
  Actor,
  BankAccount,
  Bill,
  BudgetLine,
  CodedLine,
  EntryInput,
  ExpenseClaim,
  Grant,
  ISODate,
  Minor,
  StatementLine,
  Vendor,
} from './types'

type NewCoded = Array<Omit<CodedLine, 'id'>>

export type LedgerCommand =
  | { type: 'entry.post'; input: EntryInput }
  | { type: 'entry.saveDraft'; input: EntryInput }
  | { type: 'entry.updateDraft'; entryId: string; input: EntryInput }
  | { type: 'entry.postDraft'; entryId: string; overrideReason?: string }
  | { type: 'entry.deleteDraft'; entryId: string }
  | { type: 'entry.reverse'; entryId: string; date: ISODate; reason: string }
  | { type: 'entry.import'; inputs: Array<EntryInput> }
  | {
      type: 'account.add'
      input: Pick<Account, 'code' | 'name' | 'type' | 'parent_id' | 'is_group' | 'is_cash' | 'expense_function' | 'cash_flow_class'>
    }
  | { type: 'account.update'; accountId: string; patch: Partial<Pick<Account, 'name' | 'is_active' | 'expense_function'>> }
  | { type: 'grant.add'; input: Omit<Grant, 'id'> }
  | { type: 'budgetLine.set'; input: Omit<BudgetLine, 'id'> & { id?: string } }
  | { type: 'period.close'; periodId: string }
  | { type: 'period.reopen'; periodId: string; reason: string }
  | { type: 'period.lock'; periodId: string }
  | { type: 'year.close'; year: number }
  | { type: 'vendor.add'; input: Omit<Vendor, 'id'> }
  | {
      type: 'bill.create'
      input: Omit<Bill, 'id' | 'status' | 'entry_id' | 'approved_by' | 'created_by' | 'lines'> & { lines: NewCoded }
    }
  | { type: 'bill.approve'; billId: string }
  | { type: 'bill.post'; billId: string; overrideReason?: string }
  | { type: 'bill.pay'; billId: string; date: ISODate; cashAccountId: string; amount: Minor; reference?: string | null }
  | {
      type: 'claim.create'
      input: Omit<ExpenseClaim, 'id' | 'number' | 'status' | 'approvals' | 'entry_id' | 'lines' | 'claimant' | 'claimant_name'> & {
        id?: string
        lines: NewCoded
        submit?: boolean
      }
    }
  | { type: 'claim.submit'; claimId: string }
  | { type: 'claim.approve'; claimId: string; note?: string; overrideReason?: string }
  | { type: 'claim.reject'; claimId: string; note: string }
  | { type: 'claim.reimburse'; claimId: string; date: ISODate; cashAccountId: string; reference?: string | null }
  | { type: 'bank.add'; input: Omit<BankAccount, 'id'> }
  | {
      type: 'statement.import'
      bankAccountId: string
      rows: Array<Pick<StatementLine, 'date' | 'description' | 'reference' | 'amount'>>
    }
  | { type: 'statement.autoMatch'; bankAccountId: string }
  | { type: 'statement.match'; statementLineId: string; journalLineId: string }
  | { type: 'statement.unmatch'; statementLineId: string }
  | {
      type: 'statement.post'
      statementLineId: string
      counter: { account_id?: string; fund_id: string; grant_id?: string | null; memo?: string }
    }

export type CommandResult =
  | { type: 'id'; id: string }
  | { type: 'ids'; ids: Array<string> }
  | { type: 'autoMatch'; result: AutoMatchResult }
  | { type: 'none' }

/**
 * Applies a command. Throws LedgerError when a rule blocks it; the caller
 * should then discard the state change (the demo store applies commands to
 * a copy, SpacetimeDB rolls the reducer back).
 */
export function applyCommand(ledger: Ledger, actor: Actor, cmd: LedgerCommand): CommandResult {
  const id = (x: { id: string } | null) => (x ? { type: 'id' as const, id: x.id } : { type: 'none' as const })
  switch (cmd.type) {
    case 'entry.post':
      return id(ledger.post(actor, cmd.input))
    case 'entry.saveDraft':
      return id(ledger.saveDraft(actor, cmd.input))
    case 'entry.updateDraft':
      return id(ledger.updateDraft(actor, cmd.entryId, cmd.input))
    case 'entry.postDraft':
      return id(ledger.postDraft(actor, cmd.entryId, cmd.overrideReason))
    case 'entry.deleteDraft':
      ledger.deleteDraft(actor, cmd.entryId)
      return { type: 'none' }
    case 'entry.reverse':
      return id(ledger.reverse(actor, cmd.entryId, cmd.date, cmd.reason))
    case 'entry.import':
      return { type: 'ids', ids: cmd.inputs.map((i) => ledger.saveDraft(actor, i).id) }
    case 'account.add':
      return id(ledger.addAccount(actor, cmd.input))
    case 'account.update':
      return id(ledger.updateAccount(actor, cmd.accountId, cmd.patch))
    case 'grant.add':
      return id(ledger.addGrant(actor, cmd.input))
    case 'budgetLine.set':
      return id(ledger.setBudgetLine(actor, cmd.input))
    case 'period.close':
      return id(ledger.closePeriod(actor, cmd.periodId))
    case 'period.reopen':
      return id(ledger.reopenPeriod(actor, cmd.periodId, cmd.reason))
    case 'period.lock':
      return id(ledger.lockPeriod(actor, cmd.periodId))
    case 'year.close':
      return id(ledger.closeYear(actor, cmd.year))
    case 'vendor.add':
      return id(ledger.addVendor(actor, cmd.input))
    case 'bill.create':
      return id(ledger.createBill(actor, cmd.input))
    case 'bill.approve':
      return id(ledger.approveBill(actor, cmd.billId))
    case 'bill.post':
      return id(ledger.postBill(actor, cmd.billId, cmd.overrideReason))
    case 'bill.pay':
      return id(
        ledger.payBill(actor, cmd.billId, {
          date: cmd.date,
          cash_account_id: cmd.cashAccountId,
          amount: cmd.amount,
          reference: cmd.reference,
        }),
      )
    case 'claim.create':
      return id(ledger.createClaim(actor, cmd.input))
    case 'claim.submit':
      return id(ledger.submitClaim(actor, cmd.claimId))
    case 'claim.approve':
      return id(ledger.approveClaim(actor, cmd.claimId, cmd.note, cmd.overrideReason))
    case 'claim.reject':
      return id(ledger.rejectClaim(actor, cmd.claimId, cmd.note))
    case 'claim.reimburse':
      return id(
        ledger.reimburseClaim(actor, cmd.claimId, {
          date: cmd.date,
          cash_account_id: cmd.cashAccountId,
          reference: cmd.reference,
        }),
      )
    case 'bank.add':
      return id(ledger.addBankAccount(actor, cmd.input))
    case 'statement.import':
      return { type: 'ids', ids: ledger.importStatement(actor, cmd.bankAccountId, cmd.rows).map((l) => l.id) }
    case 'statement.autoMatch':
      return { type: 'autoMatch', result: autoMatch(ledger, actor, cmd.bankAccountId) }
    case 'statement.match':
      return id(ledger.matchStatementLine(actor, cmd.statementLineId, cmd.journalLineId))
    case 'statement.unmatch':
      return id(ledger.unmatchStatementLine(actor, cmd.statementLineId))
    case 'statement.post':
      return id(ledger.postFromStatementLine(actor, cmd.statementLineId, cmd.counter))
  }
}
