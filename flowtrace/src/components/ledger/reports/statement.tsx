// Building blocks for statements set like an accountant's report: hairline
// rows, a rule above subtotals, a double rule under grand totals, tabular
// figures with negatives in parentheses, and a visible footing check.
import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { StatusLine } from '@/components/report'
import { formatMinor } from '@/components/ledger/money'

/** A statement table that scrolls sideways inside its section, never the page. */
export function StatementTable({
  caption,
  columns,
  firstColumn = 'Line',
  children,
  className,
}: {
  caption: string
  columns: Array<string>
  firstColumn?: string
  children: ReactNode
  className?: string
}) {
  return (
    <div className="overflow-x-auto">
      <table className={cn('w-full text-sm', columns.length > 2 && 'min-w-[36rem]', className)}>
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr className="border-b border-foreground text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 text-left font-semibold">
              {firstColumn}
            </th>
            {columns.map((c) => (
              <th key={c} scope="col" className="py-2.5 pl-4 text-right font-semibold whitespace-nowrap">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        {children}
      </table>
    </div>
  )
}

/** A heading row that opens a group of lines (Assets, Income…). */
export function GroupHeading({ label, span }: { label: ReactNode; span: number }) {
  return (
    <tr>
      <th scope="colgroup" colSpan={span} className="section-label pt-6 pb-1.5 text-left">
        {label}
      </th>
    </tr>
  )
}

export type RowKind = 'line' | 'subtotal' | 'total'

/** One line of figures. Subtotals sit on a single rule, totals on a double. */
export function AmountRow({
  label,
  code,
  values,
  kind = 'line',
  indent = false,
  detail,
}: {
  label: ReactNode
  code?: string
  values: Array<number | ReactNode>
  kind?: RowKind
  indent?: boolean
  detail?: ReactNode
}) {
  return (
    <tr
      className={cn(
        'align-baseline',
        kind === 'line' && 'border-b border-border',
        kind === 'subtotal' && 'border-t border-foreground font-semibold',
        kind === 'total' && 'border-t border-b-[3px] border-double border-foreground font-bold',
      )}
    >
      <th scope="row" className={cn('py-2 pr-4 text-left', kind === 'line' && 'font-normal', indent && 'pl-4')}>
        {code && <span className="mr-2 font-mono text-xs font-normal text-muted-foreground">{code}</span>}
        {label}
        {detail && <span className="block text-xs font-normal text-muted-foreground">{detail}</span>}
      </th>
      {values.map((v, i) => (
        <td key={i} className="py-2 pl-4 text-right whitespace-nowrap tabular-nums">
          {typeof v === 'number' ? formatMinor(v) : v}
        </td>
      ))}
    </tr>
  )
}

/** A spacer row so groups breathe without boxes. */
export function GapRow({ span }: { span: number }) {
  return (
    <tr aria-hidden="true">
      <td colSpan={span} className="h-3" />
    </tr>
  )
}

/**
 * The statement's footing check, shown on the page: what must agree, and
 * the difference (0 when it foots). A difference is flagged and announced.
 */
export function FootingCheck({
  statement,
  difference,
  detail,
}: {
  statement: ReactNode
  difference: number
  detail?: ReactNode
}) {
  const ok = difference === 0
  return (
    <div
      role={ok ? undefined : 'alert'}
      className="flex flex-col gap-2 border-b border-border py-3 text-sm sm:flex-row sm:items-center sm:justify-between"
    >
      <p className="flex items-start gap-3">
        <StatusLine status={ok ? 'verified' : 'flagged'} width={22} className="mt-1.5" />
        <span>
          <span className={cn('font-semibold', !ok && 'text-flag')}>
            {ok ? 'Foots.' : 'Does not foot.'}
          </span>{' '}
          {statement}
          {detail && <span className="block text-xs text-muted-foreground">{detail}</span>}
        </span>
      </p>
      <p className="shrink-0 pl-[34px] tabular-nums sm:pl-0">
        <span className="text-muted-foreground">Difference </span>
        <span className="font-semibold">{formatMinor(difference, { zero: '0.00' })}</span>
      </p>
    </div>
  )
}

/** The statement's title block: whose, which statement, which dates, which money. */
export function StatementHead({
  org,
  title,
  period,
  currency,
  basis,
}: {
  org?: string | null
  title: string
  period: ReactNode
  currency: string
  basis?: string
}) {
  return (
    <div className="border-b border-border py-4">
      {org && <p className="text-sm font-semibold">{org}</p>}
      <p className="text-2xl">{title}</p>
      <p className="mt-1 text-sm text-muted-foreground">
        {period} · Amounts in {currency}
        {basis && ` · ${basis}`}
      </p>
    </div>
  )
}
