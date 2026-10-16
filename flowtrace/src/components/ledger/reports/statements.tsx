// The five statements, each with its CSV. Figures come straight from the
// engine's report functions; this file only sets them on the page.
import {
  AmountRow,
  FootingCheck,
  GapRow,
  GroupHeading,
  StatementHead,
  StatementTable,
} from './statement'
import type { ReactNode } from 'react'
import type { Basis, ISODate, LedgerState } from '@/lib/ledger'
import {
  activities,
  cashFlow,
  dayBefore,
  financialPosition,
  functionalExpenses,
  postedLines,
  trialBalance,
} from '@/lib/ledger'
import { formatDate } from '@/lib/format'
import { KeyFigure, KeyFigures } from '@/components/report'
import { csvAmount } from '@/components/ledger/export'
import { formatMinorCompact } from '@/components/ledger/money'

export type ReportId = 'trial-balance' | 'position' | 'activities' | 'functional' | 'cash-flow'

export const REPORTS: Array<{
  id: ReportId
  title: string
  short: string
  /** 'point' reports are as at a date; 'range' reports cover a period. */
  span: 'point' | 'range'
  hasBasis: boolean
}> = [
  { id: 'trial-balance', title: 'Trial balance', short: 'Trial balance', span: 'point', hasBasis: true },
  { id: 'position', title: 'Statement of financial position', short: 'Financial position', span: 'point', hasBasis: true },
  { id: 'activities', title: 'Statement of activities', short: 'Activities', span: 'range', hasBasis: true },
  { id: 'functional', title: 'Statement of functional expenses', short: 'Functional expenses', span: 'range', hasBasis: true },
  { id: 'cash-flow', title: 'Statement of cash flows', short: 'Cash flow', span: 'range', hasBasis: false },
]

export const reportMeta = (id: ReportId) => REPORTS.find((r) => r.id === id)!

export interface ReportParams {
  from: ISODate
  to: ISODate
  basis: Basis
}

const BASIS_LABEL: Record<Basis, string> = { accrual: 'Accrual basis', cash: 'Cash basis' }

function periodText(id: ReportId, p: ReportParams) {
  return reportMeta(id).span === 'point'
    ? `As at ${formatDate(p.to)}`
    : `For the period ${formatDate(p.from)} to ${formatDate(p.to)}`
}

/** The statement for `id`, with its title block and footing check. */
export function ReportBody({
  id,
  state,
  params,
  org,
}: {
  id: ReportId
  state: LedgerState
  params: ReportParams
  org?: string | null
}) {
  const meta = reportMeta(id)
  const currency = state.settings.base_currency
  const head = (
    <StatementHead
      org={org}
      title={meta.title}
      period={periodText(id, params)}
      currency={currency}
      basis={meta.hasBasis ? BASIS_LABEL[params.basis] : 'Direct method'}
    />
  )
  let body: ReactNode
  switch (id) {
    case 'trial-balance':
      body = <TrialBalance state={state} params={params} />
      break
    case 'position':
      body = <Position state={state} params={params} />
      break
    case 'activities':
      body = <Activities state={state} params={params} />
      break
    case 'functional':
      body = <Functional state={state} params={params} currency={currency} />
      break
    case 'cash-flow':
      body = <CashFlowStatement state={state} params={params} />
      break
  }
  return (
    <div>
      {head}
      {body}
    </div>
  )
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="border-b border-border py-6 text-sm text-muted-foreground">{children}</p>
}

// ---------------------------------------------------------------------------

function TrialBalance({ state, params }: { state: LedgerState; params: ReportParams }) {
  const tb = trialBalance(state, params.to, params.basis)
  return (
    <>
      {tb.rows.length === 0 ? (
        <Empty>No posted balances on or before {formatDate(params.to)}.</Empty>
      ) : (
        <StatementTable caption={`Trial balance as at ${formatDate(params.to)}`} firstColumn="Account" columns={['Debit', 'Credit']}>
          <tbody>
            {tb.rows.map((r) => (
              <AmountRow key={r.account.id} code={r.account.code} label={r.account.name} values={[r.debit, r.credit]} />
            ))}
            <AmountRow kind="total" label="Totals" values={[tb.debit, tb.credit]} />
          </tbody>
        </StatementTable>
      )}
      <FootingCheck statement="Total debits equal total credits." difference={tb.difference} />
    </>
  )
}

function Position({ state, params }: { state: LedgerState; params: ReportParams }) {
  const fp = financialPosition(state, params.to, params.basis)
  const span = 2
  return (
    <>
      <StatementTable caption={`Statement of financial position as at ${formatDate(params.to)}`} firstColumn="" columns={[formatDate(params.to)]}>
        <tbody>
          <GroupHeading label="Assets" span={span} />
          {fp.assets.map((r) => (
            <AmountRow key={r.account.id} code={r.account.code} label={r.account.name} values={[r.amount]} />
          ))}
          <AmountRow kind="subtotal" label="Total assets" values={[fp.totalAssets]} />
          <GapRow span={span} />
          <GroupHeading label="Liabilities" span={span} />
          {fp.liabilities.length === 0 && <AmountRow label="No liabilities" values={[0]} />}
          {fp.liabilities.map((r) => (
            <AmountRow key={r.account.id} code={r.account.code} label={r.account.name} values={[r.amount]} />
          ))}
          <AmountRow kind="subtotal" label="Total liabilities" values={[fp.totalLiabilities]} />
          <GapRow span={span} />
          <GroupHeading label="Net assets" span={span} />
          <AmountRow label="Without donor restrictions" values={[fp.netAssets.unrestricted]} />
          <AmountRow label="With donor restrictions" values={[fp.netAssets.restricted]} />
          <AmountRow kind="subtotal" label="Total net assets" values={[fp.totalNetAssets]} />
          <GapRow span={span} />
          <AmountRow kind="total" label="Total liabilities and net assets" values={[fp.totalLiabilities + fp.totalNetAssets]} />
        </tbody>
      </StatementTable>
      <FootingCheck
        statement="Assets equal liabilities plus net assets."
        difference={fp.difference}
        detail="Net assets include this year's surplus or deficit to date, split by fund restriction."
      />
    </>
  )
}

function Activities({ state, params }: { state: LedgerState; params: ReportParams }) {
  const a = activities(state, params.from, params.to, params.basis)
  // Cross-check against the balance sheet: the change in net assets over the
  // period equals the movement in net assets between its two dates.
  const before = financialPosition(state, dayBefore(params.from), params.basis)
  const after = financialPosition(state, params.to, params.basis)
  const movement = after.totalNetAssets - before.totalNetAssets
  // Entries straight to net assets (opening balances) move the balance
  // sheet without passing through income or expenses.
  const accounts = new Map(state.accounts.map((x) => [x.id, x]))
  const direct = -postedLines(state, { from: params.from, to: params.to, basis: params.basis, excludeClosing: true })
    .filter((l) => accounts.get(l.account_id)?.type === 'equity')
    .reduce((t, l) => t + l.base_amount, 0)
  const cols = ['Without restrictions', 'With restrictions', 'Total']
  const span = 4
  const v = (x: { unrestricted: number; restricted: number; total: number }) => [x.unrestricted, x.restricted, x.total]
  return (
    <>
      {a.income.length === 0 && a.expenses.length === 0 ? (
        <Empty>No income or expenses posted in this period.</Empty>
      ) : (
        <StatementTable caption={`Statement of activities, ${formatDate(params.from)} to ${formatDate(params.to)}`} firstColumn="" columns={cols}>
          <tbody>
            <GroupHeading label="Income" span={span} />
            {a.income.map((r) => (
              <AmountRow key={r.account.id} code={r.account.code} label={r.account.name} values={v(r)} />
            ))}
            <AmountRow kind="subtotal" label="Total income" values={v(a.totalIncome)} />
            <GapRow span={span} />
            <GroupHeading label="Expenses" span={span} />
            {a.expenses.map((r) => (
              <AmountRow key={r.account.id} code={r.account.code} label={r.account.name} values={v(r)} />
            ))}
            <AmountRow kind="subtotal" label="Total expenses" values={v(a.totalExpenses)} />
            <GapRow span={span} />
            <AmountRow kind="total" label="Change in net assets" values={v(a.changeInNetAssets)} />
          </tbody>
        </StatementTable>
      )}
      <FootingCheck
        statement="Change in net assets equals the movement in net assets on the statement of financial position."
        difference={a.changeInNetAssets.total + direct - movement}
        detail={`Net assets ${formatMinorCompact(before.totalNetAssets)} on ${formatDate(dayBefore(params.from))}, ${formatMinorCompact(after.totalNetAssets)} on ${formatDate(params.to)}.${direct !== 0 ? ` Includes ${formatMinorCompact(direct)} posted straight to net assets, such as opening balances.` : ''} Year-end closing entries are left out.`}
      />
      <div className="mt-8">
        <h3 className="section-label border-b border-border pb-2">Schedule by fund</h3>
        {a.byFund.length === 0 ? (
          <Empty>No fund had income or expenses in this period.</Empty>
        ) : (
          <StatementTable caption="Income, expenses and change in net assets by fund" firstColumn="Fund" columns={['Income', 'Expenses', 'Change']}>
            <tbody>
              {a.byFund.map((f) => (
                <AmountRow
                  key={f.fund.id}
                  label={f.fund.name}
                  detail={f.fund.restriction === 'unrestricted' ? 'Without donor restrictions' : 'With donor restrictions'}
                  values={[f.income, f.expenses, f.change]}
                />
              ))}
              <AmountRow
                kind="total"
                label="All funds"
                values={[a.totalIncome.total, a.totalExpenses.total, a.changeInNetAssets.total]}
              />
            </tbody>
          </StatementTable>
        )}
      </div>
    </>
  )
}

function Functional({ state, params, currency }: { state: LedgerState; params: ReportParams; currency: string }) {
  const fe = functionalExpenses(state, params.from, params.to, params.basis)
  const act = activities(state, params.from, params.to, params.basis)
  const pct = (n: number) => (fe.totals.total > 0 ? `${Math.round((n / fe.totals.total) * 100)}%` : '–')
  return (
    <>
      <KeyFigures>
        <KeyFigure
          scale="lg"
          value={fe.totals.total > 0 ? `${Math.round(fe.programRatio * 100)}%` : '–'}
          label="Programme ratio"
          detail={<span className="text-muted-foreground">Share of spend on programmes</span>}
        />
        <KeyFigure value={formatMinorCompact(fe.totals.program)} label={`Programmes, ${currency}`} />
        <KeyFigure value={formatMinorCompact(fe.totals.management)} label="Management and general" />
        <KeyFigure value={formatMinorCompact(fe.totals.fundraising)} label="Fundraising" />
      </KeyFigures>
      {fe.rows.length === 0 ? (
        <Empty>No expenses posted in this period.</Empty>
      ) : (
        <StatementTable
          caption={`Statement of functional expenses, ${formatDate(params.from)} to ${formatDate(params.to)}`}
          firstColumn="Expense"
          columns={['Programmes', 'Management', 'Fundraising', 'Total']}
        >
          <tbody>
            {fe.rows.map((r) => (
              <AmountRow
                key={r.account.id}
                code={r.account.code}
                label={r.account.name}
                values={[r.program, r.management, r.fundraising, r.total]}
              />
            ))}
            <AmountRow
              kind="total"
              label="Total expenses"
              values={[fe.totals.program, fe.totals.management, fe.totals.fundraising, fe.totals.total]}
            />
            <AmountRow label="Share of total" values={[pct(fe.totals.program), pct(fe.totals.management), pct(fe.totals.fundraising), pct(fe.totals.total)]} />
          </tbody>
        </StatementTable>
      )}
      <FootingCheck
        statement="Total expenses equal total expenses on the statement of activities."
        difference={fe.totals.total - act.totalExpenses.total}
        detail="Each expense account carries one function; accounts without one count as programmes."
      />
    </>
  )
}

function CashFlowStatement({ state, params }: { state: LedgerState; params: ReportParams }) {
  const cf = cashFlow(state, params.from, params.to)
  return (
    <>
      <StatementTable caption={`Statement of cash flows, ${formatDate(params.from)} to ${formatDate(params.to)}`} firstColumn="" columns={['Amount']}>
        <tbody>
          <GroupHeading label="Operating activities" span={2} />
          <AmountRow label="Cash received from donors and other income" values={[cf.operating.receipts]} />
          <AmountRow label="Cash paid to suppliers, staff and programmes" values={[cf.operating.payments]} />
          <AmountRow kind="subtotal" label="Net cash from operating activities" values={[cf.operating.net]} />
          <GapRow span={2} />
          <GroupHeading label="Investing activities" span={2} />
          <AmountRow kind="subtotal" label="Net cash from investing activities" values={[cf.investing]} />
          <GapRow span={2} />
          <GroupHeading label="Financing activities" span={2} />
          <AmountRow kind="subtotal" label="Net cash from financing activities" values={[cf.financing]} />
          <GapRow span={2} />
          <AmountRow kind="subtotal" label="Net change in cash" values={[cf.net]} />
          <AmountRow label={`Cash at ${formatDate(dayBefore(params.from))}`} values={[cf.opening]} />
          <AmountRow kind="total" label={`Cash at ${formatDate(params.to)}`} values={[cf.closing]} />
        </tbody>
      </StatementTable>
      <FootingCheck
        statement="Opening cash plus the net change equals closing cash."
        difference={cf.difference}
        detail="Transfers between the organisation's own bank and mobile-money accounts are not cash flows and are left out."
      />
    </>
  )
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

/** The statement as rows for a spreadsheet, amounts in major units. */
export function reportCsv(id: ReportId, state: LedgerState, p: ReportParams): Array<Array<unknown>> {
  const meta = reportMeta(id)
  const head: Array<Array<unknown>> = [
    [meta.title],
    [periodText(id, p)],
    [`Amounts in ${state.settings.base_currency}`, meta.hasBasis ? BASIS_LABEL[p.basis] : 'Direct method'],
    [],
  ]
  const c = csvAmount
  switch (id) {
    case 'trial-balance': {
      const tb = trialBalance(state, p.to, p.basis)
      return [
        ...head,
        ['Code', 'Account', 'Debit', 'Credit'],
        ...tb.rows.map((r) => [r.account.code, r.account.name, c(r.debit), c(r.credit)]),
        ['', 'Totals', c(tb.debit), c(tb.credit)],
        ['', 'Difference', c(tb.difference)],
      ]
    }
    case 'position': {
      const fp = financialPosition(state, p.to, p.basis)
      return [
        ...head,
        ['Section', 'Code', 'Line', 'Amount'],
        ...fp.assets.map((r) => ['Assets', r.account.code, r.account.name, c(r.amount)]),
        ['Assets', '', 'Total assets', c(fp.totalAssets)],
        ...fp.liabilities.map((r) => ['Liabilities', r.account.code, r.account.name, c(r.amount)]),
        ['Liabilities', '', 'Total liabilities', c(fp.totalLiabilities)],
        ['Net assets', '', 'Without donor restrictions', c(fp.netAssets.unrestricted)],
        ['Net assets', '', 'With donor restrictions', c(fp.netAssets.restricted)],
        ['Net assets', '', 'Total net assets', c(fp.totalNetAssets)],
        ['', '', 'Total liabilities and net assets', c(fp.totalLiabilities + fp.totalNetAssets)],
        ['', '', 'Difference', c(fp.difference)],
      ]
    }
    case 'activities': {
      const a = activities(state, p.from, p.to, p.basis)
      const row = (s: string, code: string, l: string, x: { unrestricted: number; restricted: number; total: number }) => [
        s, code, l, c(x.unrestricted), c(x.restricted), c(x.total),
      ]
      return [
        ...head,
        ['Section', 'Code', 'Line', 'Without restrictions', 'With restrictions', 'Total'],
        ...a.income.map((r) => row('Income', r.account.code, r.account.name, r)),
        row('Income', '', 'Total income', a.totalIncome),
        ...a.expenses.map((r) => row('Expenses', r.account.code, r.account.name, r)),
        row('Expenses', '', 'Total expenses', a.totalExpenses),
        row('', '', 'Change in net assets', a.changeInNetAssets),
        [],
        ['Schedule by fund'],
        ['Fund', 'Restriction', 'Income', 'Expenses', 'Change'],
        ...a.byFund.map((f) => [f.fund.name, f.fund.restriction, c(f.income), c(f.expenses), c(f.change)]),
      ]
    }
    case 'functional': {
      const fe = functionalExpenses(state, p.from, p.to, p.basis)
      return [
        ...head,
        ['Code', 'Expense', 'Programmes', 'Management', 'Fundraising', 'Total'],
        ...fe.rows.map((r) => [r.account.code, r.account.name, c(r.program), c(r.management), c(r.fundraising), c(r.total)]),
        ['', 'Total expenses', c(fe.totals.program), c(fe.totals.management), c(fe.totals.fundraising), c(fe.totals.total)],
        ['', 'Programme ratio', fe.programRatio.toFixed(4)],
      ]
    }
    case 'cash-flow': {
      const cf = cashFlow(state, p.from, p.to)
      return [
        ...head,
        ['Section', 'Line', 'Amount'],
        ['Operating', 'Cash received', c(cf.operating.receipts)],
        ['Operating', 'Cash paid', c(cf.operating.payments)],
        ['Operating', 'Net cash from operating activities', c(cf.operating.net)],
        ['Investing', 'Net cash from investing activities', c(cf.investing)],
        ['Financing', 'Net cash from financing activities', c(cf.financing)],
        ['', 'Net change in cash', c(cf.net)],
        ['', 'Opening cash', c(cf.opening)],
        ['', 'Closing cash', c(cf.closing)],
        ['', 'Difference', c(cf.difference)],
      ]
    }
  }
}
