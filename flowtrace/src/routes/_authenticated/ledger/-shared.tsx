// Helpers shared by the Ledger screens. The leading dash keeps this file out
// of the route tree.
import type {
  EntryStatus,
  JournalLine,
  LedgerState,
  Minor,
  PeriodStatus,
  SourceType,
} from '@/lib/ledger'
import { DEMO_ACTOR } from '@/lib/demo/ledger'
import { SYSTEM_ACTOR } from '@/lib/ledger'
import { cn } from '@/lib/utils'

export const SOURCE_LABEL: Record<SourceType, string> = {
  manual: 'Manual entry',
  transaction: 'Transaction',
  bill: 'Bill',
  payment: 'Bill payment',
  expense_claim: 'Expense claim',
  reimbursement: 'Claim reimbursement',
  statement: 'Bank statement',
  opening: 'Opening balances',
  year_end_close: 'Year-end close',
  reversal: 'Reversal',
  import: 'Import',
}

export const ENTRY_STATUS_LABEL: Record<EntryStatus, string> = {
  draft: 'Draft',
  posted: 'Posted',
  reversed: 'Reversed',
}

export const PERIOD_STATUS_LABEL: Record<PeriodStatus, string> = {
  open: 'Open',
  closed: 'Closed',
  locked: 'Locked',
}

/**
 * A short line sample for a ledger state, so the state reads in greyscale:
 * a draft is dashed (not yet in the books), posted is solid, reversed is
 * dotted (cancelled by a later entry).
 */
function Mark({ dash, heavy, ends }: { dash?: string; heavy?: boolean; ends?: boolean }) {
  return (
    <svg width="22" height="10" viewBox="0 0 22 10" aria-hidden="true" className="shrink-0 overflow-visible">
      <line
        x1="0"
        y1="5"
        x2="22"
        y2="5"
        strokeWidth={heavy ? 4 : 3}
        strokeDasharray={dash}
        className="stroke-current"
      />
      {ends && (
        <path d="M1 0 V10 M21 0 V10" strokeWidth="2" className="stroke-current" />
      )}
    </svg>
  )
}

export function EntryStatusMark({ status, className }: { status: EntryStatus; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        status !== 'posted' && 'text-muted-foreground',
        className,
      )}
    >
      <Mark dash={status === 'draft' ? '6 5' : status === 'reversed' ? '2 5' : undefined} />
      {ENTRY_STATUS_LABEL[status]}
    </span>
  )
}

/**
 * Open periods are dashed (still taking entries), closed are solid, locked
 * are solid with end stops (sealed for good).
 */
export function PeriodStatusMark({ status, className }: { status: PeriodStatus; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        status === 'open' && 'text-muted-foreground',
        className,
      )}
    >
      <Mark dash={status === 'open' ? '6 5' : undefined} ends={status === 'locked'} heavy={status === 'locked'} />
      {PERIOD_STATUS_LABEL[status]}
    </span>
  )
}

/** Total debits of an entry's lines, in base currency. */
export function entryTotal(lines: Array<JournalLine>): Minor {
  let t = 0
  for (const l of lines) if (l.base_amount > 0) t += l.base_amount
  return t
}

export function linesByEntry(state: LedgerState) {
  const m = new Map<string, Array<JournalLine>>()
  for (const l of state.lines) {
    const list = m.get(l.entry_id)
    if (list) list.push(l)
    else m.set(l.entry_id, [l])
  }
  return m
}

/** A readable name for whoever acted in the audit trail. */
export function actorName(id: string) {
  if (id === SYSTEM_ACTOR.id) return 'FlowTrace (automatic)'
  if (id === DEMO_ACTOR.id) return DEMO_ACTOR.name
  if (id.startsWith('staff-')) return id.slice(6).replace(/^\w/, (c) => c.toUpperCase())
  return `Member ${id.slice(0, 8)}`
}

export const AUDIT_ACTION_LABEL: Record<string, string> = {
  'entry.draft': 'Saved as draft',
  'entry.update': 'Draft edited',
  'entry.post': 'Posted',
  'entry.delete': 'Draft deleted',
  'entry.reverse': 'Reversed',
  'account.create': 'Account added',
  'account.update': 'Account changed',
  'period.close': 'Period closed',
  'period.reopen': 'Period reopened',
  'period.lock': 'Period locked',
  'year.close': 'Year closed',
}

export const str = (v: unknown) => (typeof v === 'string' && v ? v : undefined)

export const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/
