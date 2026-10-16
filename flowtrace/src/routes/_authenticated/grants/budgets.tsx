import { createFileRoute } from '@tanstack/react-router'
import { Fragment, useMemo, useState } from 'react'
import { DownloadSimple, PencilSimple, Plus, Warning } from '@phosphor-icons/react'
import { GRANTS_NAV } from './index'
import type { Basis, BudgetLine, BudgetVsActualRow, LedgerState } from '@/lib/ledger'
import { budgetVsActual, parseMoney } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerActor, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Field, FormSelect } from '@/components/form'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import {
  BooksPage,
  BooksState,
  DimensionSelect,
  GrantSelect,
  IssueSummary,
  Money,
  SubNav,
  csvAmount,
  downloadCsv,
  formatMinorCompact,
} from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/grants/budgets')({
  component: BudgetsPage,
})

type Group = 'grant' | 'running' | 'department'

const GROUP_TITLE: Record<Group, string> = {
  grant: 'Grant budgets',
  running: 'Unrestricted running costs',
  department: 'Department allocations',
}

const STATUS_TEXT: Record<BudgetVsActualRow['status'], string> = {
  within: 'Within budget',
  warning: '80% or more used',
  over: 'Over budget',
}

interface Row extends BudgetVsActualRow {
  line: BudgetLine
  group: Group
  sub: string | null
}

function groupOf(l: BudgetLine): Group {
  if (l.grant_id) return 'grant'
  if (l.dimension_id && !l.account_id) return 'department'
  return 'running'
}

function BudgetsPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()

  return (
    <BooksPage
      title="Budget vs actual"
      lede="Every budget line for the year against what has been spent on it. Lines are flagged at 80% used and again when they go over."
      nav={<SubNav label="Grants" items={GRANTS_NAV} />}
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="budget" />
      ) : state.budgets.length === 0 ? (
        <ReportNotice title="No budget set up">
          A budget for the financial year has to be created before lines can be compared with spend.
        </ReportNotice>
      ) : (
        <Budget state={state} asOf={asOf} />
      )}
    </BooksPage>
  )
}

function Budget({ state, asOf }: { state: LedgerState; asOf: string }) {
  const actor = useLedgerActor()
  const command = useLedgerCommand()
  const year = Number(asOf.slice(0, 4))
  const budgets = useMemo(
    () => [...state.budgets].sort((a, b) => b.fiscal_year - a.fiscal_year),
    [state.budgets],
  )
  const [budgetId, setBudgetId] = useState(
    () => (budgets.find((b) => b.fiscal_year === year) ?? budgets[0]).id,
  )
  const [grantId, setGrantId] = useState('')
  const [dimensionId, setDimensionId] = useState('')
  const [basis, setBasis] = useState<Basis>('accrual')
  const [editing, setEditing] = useState<{ id: string; value: string; error?: string } | null>(null)
  const [editError, setEditError] = useState<unknown>(null)
  const [saved, setSaved] = useState<string | null>(null)

  const budget = state.budgets.find((b) => b.id === budgetId) ?? budgets[0]
  const canEdit = !!actor?.can_post

  const rows = useMemo<Array<Row>>(() => {
    const byId = new Map(state.budgetLines.map((l) => [l.id, l]))
    return budgetVsActual(state, {
      budgetId: budget.id,
      grantId: grantId || undefined,
      dimensionId: dimensionId || undefined,
      to: asOf,
      basis,
    }).map((r) => {
      const line = byId.get(r.lineId)!
      const group = groupOf(line)
      const sub =
        group === 'grant' ? (state.grants.find((g) => g.id === line.grant_id)?.name ?? 'Grant') : null
      return { ...r, line, group, sub }
    })
  }, [state, budget.id, grantId, dimensionId, asOf, basis])

  const grouped = useMemo(() => {
    const out: Array<{ group: Group; subs: Array<{ sub: string | null; rows: Array<Row> }> }> = []
    for (const g of ['grant', 'running', 'department'] as const) {
      const rs = rows.filter((r) => r.group === g)
      if (rs.length === 0) continue
      const subs = new Map<string | null, Array<Row>>()
      for (const r of rs) subs.set(r.sub, [...(subs.get(r.sub) ?? []), r])
      out.push({ group: g, subs: [...subs].map(([sub, list]) => ({ sub, rows: list })) })
    }
    return out
  }, [rows])

  const totals = useMemo(
    () => ({
      budget: rows.reduce((t, r) => t + r.budget, 0),
      actual: rows.reduce((t, r) => t + r.actual, 0),
      warning: rows.filter((r) => r.status === 'warning').length,
      over: rows.filter((r) => r.status === 'over').length,
    }),
    [rows],
  )
  const base = state.settings.base_currency

  const saveEdit = async (row: Row) => {
    if (!editing) return
    const amount = parseMoney(editing.value)
    if (amount === null || amount < 0) {
      setEditing({ ...editing, error: 'Enter an amount of zero or more, like 250000.' })
      return
    }
    setEditError(null)
    try {
      await command.mutateAsync({ type: 'budgetLine.set', input: { ...row.line, amount } })
      setEditing(null)
      setSaved(`Budget for ${row.label} set to ${base} ${formatMinorCompact(amount)}.`)
    } catch (e) {
      setEditError(e)
    }
  }

  const exportCsv = () =>
    downloadCsv(`budget-vs-actual-${budget.fiscal_year}-${basis}-${asOf}`, [
      ['Group', 'Grant', 'Line', `Budget (${base})`, `Actual (${base})`, `Variance (${base})`, 'Used %', 'Status'],
      ...rows.map((r) => [
        GROUP_TITLE[r.group],
        r.sub ?? '',
        r.label,
        csvAmount(r.budget),
        csvAmount(r.actual),
        csvAmount(r.variance),
        Number.isFinite(r.used) ? Math.round(r.used * 100) : '',
        STATUS_TEXT[r.status],
      ]),
    ])

  const dimensionGroups = useMemo(() => {
    const label = { project: 'Projects', department: 'Departments', location: 'Places', donor: 'Donors' } as const
    return (['department', 'project', 'location', 'donor'] as const)
      .map((k) => ({
        label: label[k],
        options: state.dimensions.filter((d) => d.kind === k).map((d) => ({ value: d.id, label: d.name })),
      }))
      .filter((g) => g.options.length)
  }, [state.dimensions])

  return (
    <>
      <ReportSection
        title={budget.name}
        meta={
          <>
            <span>{basis === 'cash' ? 'Cash basis' : 'Accrual basis'}</span>
            <AsOf date={asOf} />
            {DEMO_BOOKS && <DemoTag />}
          </>
        }
      >
        <KeyFigures>
          <KeyFigure scale="lg" value={formatMinorCompact(totals.budget)} label={`Budgeted, ${base}`} />
          <KeyFigure
            value={formatMinorCompact(totals.actual)}
            label="Spent"
            detail={<span className="text-muted-foreground">{totals.budget > 0 ? Math.round((totals.actual / totals.budget) * 100) : 0}% of budget</span>}
          />
          <KeyFigure
            value={String(totals.warning)}
            label="Lines 80% used"
            detail={totals.warning > 0 ? <FlagNote>Near their limit</FlagNote> : undefined}
          />
          <KeyFigure
            value={String(totals.over)}
            label="Lines over budget"
            detail={totals.over > 0 ? <FlagNote>Over their limit</FlagNote> : undefined}
          />
        </KeyFigures>

        <div className="grid gap-4 border-b border-border py-4 sm:grid-cols-2 lg:grid-cols-4">
          {budgets.length > 1 && (
            <Field id="bva-budget" label="Budget" fixed>
              {(p) => (
                <FormSelect
                  {...p}
                  value={budget.id}
                  onValueChange={(v) => v && setBudgetId(v)}
                  options={budgets.map((b) => ({ value: b.id, label: b.name }))}
                />
              )}
            </Field>
          )}
          <Field id="bva-grant" label="Grant" fixed>
            {(p) => (
              <FormSelect
                {...p}
                value={grantId}
                onValueChange={setGrantId}
                options={[{ value: '', label: 'All lines' }, ...state.grants.map((g) => ({ value: g.id, label: g.name }))]}
              />
            )}
          </Field>
          <Field id="bva-dimension" label="Department, project or place" fixed>
            {(p) => (
              <FormSelect
                {...p}
                value={dimensionId}
                onValueChange={setDimensionId}
                options={[{ value: '', label: 'All lines' }]}
                groups={dimensionGroups}
              />
            )}
          </Field>
          <div className="min-w-0">
            <p id="bva-basis-label" className="mb-1.5 text-sm font-medium">Basis</p>
            <div role="group" aria-labelledby="bva-basis-label" className="flex gap-px border border-border bg-border">
              {(['accrual', 'cash'] as const).map((b) => (
                <button
                  key={b}
                  type="button"
                  aria-pressed={basis === b}
                  onClick={() => setBasis(b)}
                  className={cn(
                    'h-10 flex-1 px-3 text-sm whitespace-nowrap transition-colors',
                    basis === b
                      ? 'bg-primary font-semibold text-primary-foreground'
                      : 'bg-background text-muted-foreground hover:bg-muted hover:text-foreground',
                  )}
                >
                  {b === 'accrual' ? 'Accrual' : 'Cash'}
                </button>
              ))}
            </div>
          </div>
        </div>
        <p className="py-3 text-xs text-muted-foreground">
          {basis === 'accrual'
            ? 'Accrual counts spend when a bill is posted or a claim approved.'
            : 'Cash counts spend only when the money has left the bank or M-Pesa.'}
        </p>
      </ReportSection>

      {saved && (
        <p role="status" className="border-y border-border py-3 text-sm font-semibold">
          {saved}
        </p>
      )}
      {!!editError && <IssueSummary error={editError} title="Budget not changed" />}

      {rows.length === 0 ? (
        <ReportNotice title="No budget lines match">Clear the filters to see the whole budget.</ReportNotice>
      ) : (
        grouped.map(({ group, subs }) => (
          <ReportSection
            key={group}
            title={GROUP_TITLE[group]}
            meta={<span>{subs.reduce((n, s) => n + s.rows.length, 0)} lines</span>}
            actions={
              group === grouped[0].group ? (
                <Button variant="outline" size="sm" onClick={exportCsv}>
                  <DownloadSimple />
                  CSV
                </Button>
              ) : undefined
            }
          >
            {subs.map(({ sub, rows: list }) => (
              <Fragment key={sub ?? 'none'}>
                {sub && (
                  <h3 className="border-b border-border pt-4 pb-2 font-sans text-sm font-semibold [font-variation-settings:'wdth'_100]">
                    {sub}
                  </h3>
                )}
                <ul className="divide-y divide-border">
                  {list.map((r) => {
                    const isEditing = editing?.id === r.lineId
                    const inputId = `edit-${r.lineId}`
                    return (
                      <li key={r.lineId} className="grid gap-3 py-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto] md:items-start">
                        <div className="min-w-0">
                          <p className="font-semibold">{r.label}</p>
                          <p
                            className={cn(
                              'mt-1 inline-flex items-center gap-1.5 text-xs font-semibold',
                              r.status === 'over' ? 'text-destructive' : r.status === 'warning' ? 'text-flag' : 'text-muted-foreground',
                            )}
                          >
                            {r.status !== 'within' && <Warning weight="bold" aria-hidden="true" className="size-3.5" />}
                            {STATUS_TEXT[r.status]} ·{' '}
                            {Number.isFinite(r.used) ? `${Math.round(r.used * 100)}% used` : 'spent with no budget'}
                          </p>
                        </div>
                        <div className="min-w-0">
                          <CoverageBar
                            total={r.budget}
                            filled={Math.min(r.actual, r.budget)}
                            evidenced={Math.min(r.actual, r.budget)}
                            label={`${Number.isFinite(r.used) ? Math.round(r.used * 100) : 100}% of budget spent`}
                          />
                          <dl className="mt-2 grid grid-cols-3 gap-2 text-xs">
                            <div>
                              <dt className="text-muted-foreground">Budget</dt>
                              <dd><Money value={r.budget} decimals={0} zero="0" /></dd>
                            </div>
                            <div>
                              <dt className="text-muted-foreground">Actual</dt>
                              <dd><Money value={r.actual} decimals={0} strong /></dd>
                            </div>
                            <div className="text-right">
                              <dt className="text-muted-foreground">{r.variance < 0 ? 'Over by' : 'Left'}</dt>
                              <dd><Money value={Math.abs(r.variance)} decimals={0} strong zero="0" /></dd>
                            </div>
                          </dl>
                        </div>
                        <div className="md:w-56">
                          {isEditing ? (
                            <form
                              noValidate
                              onSubmit={(e) => {
                                e.preventDefault()
                                saveEdit(r)
                              }}
                              className="space-y-2"
                            >
                              <label htmlFor={inputId} className="block text-xs font-semibold">
                                New budget, {base}
                              </label>
                              <Input
                                id={inputId}
                                autoFocus
                                inputMode="decimal"
                                value={editing.value}
                                aria-invalid={editing.error ? true : undefined}
                                aria-describedby={editing.error ? `${inputId}-error` : undefined}
                                onChange={(e) => setEditing({ ...editing, value: e.target.value, error: undefined })}
                                onKeyDown={(e) => e.key === 'Escape' && setEditing(null)}
                                className="h-10 text-right tabular-nums"
                              />
                              {editing.error && (
                                <p id={`${inputId}-error`} className="text-xs text-destructive">{editing.error}</p>
                              )}
                              <div className="flex gap-2">
                                <Button type="submit" className="flex-1" disabled={command.isPending}>
                                  {command.isPending ? 'Saving…' : 'Save'}
                                </Button>
                                <Button type="button" variant="ghost" onClick={() => setEditing(null)}>
                                  Cancel
                                </Button>
                              </div>
                            </form>
                          ) : (
                            canEdit && (
                              <Button
                                variant="ghost"
                                onClick={() => {
                                  setEditError(null)
                                  setSaved(null)
                                  setEditing({ id: r.lineId, value: (r.budget / 100).toFixed(2) })
                                }}
                                aria-label={`Change the budget for ${r.label}`}
                              >
                                <PencilSimple />
                                Change
                              </Button>
                            )
                          )}
                        </div>
                      </li>
                    )
                  })}
                </ul>
              </Fragment>
            ))}
          </ReportSection>
        ))
      )}

      {canEdit && <AddLine state={state} budgetId={budget.id} onSaved={setSaved} />}
    </>
  )
}

function FlagNote({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center gap-1 font-semibold text-flag">
      <Warning weight="bold" aria-hidden="true" className="size-3.5" />
      {children}
    </span>
  )
}

function AddLine({
  state,
  budgetId,
  onSaved,
}: {
  state: LedgerState
  budgetId: string
  onSaved: (msg: string) => void
}) {
  const command = useLedgerCommand()
  const [account, setAccount] = useState('')
  const [fund, setFund] = useState('')
  const [grant, setGrant] = useState('')
  const [dimension, setDimension] = useState('')
  const [amount, setAmount] = useState('')
  const [amountError, setAmountError] = useState<string | undefined>()
  const [saveError, setSaveError] = useState<unknown>(null)

  const expenseOptions = useMemo(
    () =>
      state.accounts
        .filter((a) => a.type === 'expense' && !a.is_group && a.is_active)
        .sort((a, b) => a.code.localeCompare(b.code))
        .map((a) => ({ value: a.id, label: `${a.code} · ${a.name}` })),
    [state.accounts],
  )

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaveError(null)
    const m = parseMoney(amount)
    if (!amount.trim()) return setAmountError('Enter the budgeted amount.')
    if (m === null || m < 0) return setAmountError('Enter an amount of zero or more, like 250000.')
    setAmountError(undefined)
    try {
      await command.mutateAsync({
        type: 'budgetLine.set',
        input: {
          budget_id: budgetId,
          account_id: account || null,
          fund_id: fund || null,
          grant_id: grant || null,
          dimension_id: dimension || null,
          period_id: null,
          amount: m,
        },
      })
      setAmount('')
      onSaved('Budget line added.')
    } catch (err) {
      setSaveError(err)
    }
  }

  return (
    <ReportSection title="Add a budget line">
      <form noValidate onSubmit={submit} className="space-y-4 py-4">
        {!!saveError && <IssueSummary error={saveError} title="Line not added" />}
        <fieldset disabled={command.isPending} className="grid min-w-0 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <Field id="add-account" label="Expense account" hint="Leave as all for a department allocation">
            {(p) => (
              <FormSelect
                {...p}
                value={account}
                onValueChange={setAccount}
                options={[{ value: '', label: 'All expense accounts' }, ...expenseOptions]}
              />
            )}
          </Field>
          <Field id="add-fund" label="Fund">
            {(p) => (
              <FormSelect
                {...p}
                value={fund}
                onValueChange={(v) => {
                  setFund(v)
                  if (grant && v && state.grants.find((g) => g.id === grant)?.fund_id !== v) setGrant('')
                }}
                options={[{ value: '', label: 'Any fund' }, ...state.funds.map((f) => ({ value: f.id, label: f.name }))]}
              />
            )}
          </Field>
          <Field id="add-grant" label="Grant">
            {(p) => (
              <GrantSelect {...p} state={state} fundId={fund || undefined} value={grant} onValueChange={setGrant} />
            )}
          </Field>
          <Field id="add-dimension" label="Department, project or place">
            {(p) => <DimensionSelect {...p} state={state} value={dimension} onValueChange={setDimension} />}
          </Field>
          <Field id="add-amount" label={`Amount for the year, ${state.settings.base_currency}`} required error={amountError}>
            {(p) => (
              <Input
                {...p}
                inputMode="decimal"
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value)
                  if (amountError) setAmountError(undefined)
                }}
                autoComplete="off"
                className="h-10 text-right tabular-nums"
              />
            )}
          </Field>
          <div className="flex items-end">
            <Button type="submit" size="lg" className="w-full sm:w-auto">
              <Plus />
              {command.isPending ? 'Adding…' : 'Add line'}
            </Button>
          </div>
        </fieldset>
      </form>
    </ReportSection>
  )
}
