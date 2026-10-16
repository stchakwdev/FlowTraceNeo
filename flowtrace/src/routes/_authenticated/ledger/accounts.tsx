import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { Plus } from '@phosphor-icons/react'
import type { Account, AccountType, ExpenseFunction, LedgerState, Minor } from '@/lib/ledger'
import { balances, natural } from '@/lib/ledger'
import { useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Field, FormSelect } from '@/components/form'
import { CheckboxField, ReportDialog } from '@/components/report-dialog'
import { AsOf, DemoTag, ReportNotice, ReportSection } from '@/components/report'
import {
  BooksPage,
  BooksState,
  IssueSummary,
  Money,
  SubNav,
} from '@/components/ledger'
import { LEDGER_NAV } from '@/components/ledger/ledger-nav'

export const Route = createFileRoute('/_authenticated/ledger/accounts')({
  component: AccountsPage,
})

const TYPE_LABEL: Record<AccountType, string> = {
  asset: 'Asset',
  liability: 'Liability',
  equity: 'Net assets',
  income: 'Income',
  expense: 'Expense',
}

const FUNCTION_LABEL: Record<ExpenseFunction, string> = {
  program: 'Programme',
  management: 'Management and general',
  fundraising: 'Fundraising',
}

type TreeRow = { account: Account; depth: number; balance: Minor }

function buildTree(state: LedgerState, asOf: string): Array<TreeRow> {
  const bal = balances(state, { to: asOf })
  const children = new Map<string | null, Array<Account>>()
  for (const a of state.accounts) {
    const key = a.parent_id && state.accounts.some((p) => p.id === a.parent_id) ? a.parent_id : null
    const list = children.get(key) ?? []
    list.push(a)
    children.set(key, list)
  }
  const out: Array<TreeRow> = []
  const walk = (parent: string | null, depth: number): Minor => {
    let total = 0
    for (const a of (children.get(parent) ?? []).sort((x, y) => x.code.localeCompare(y.code))) {
      const row: TreeRow = { account: a, depth, balance: 0 }
      out.push(row)
      const own = bal.get(a.id) ?? 0
      const below = walk(a.id, depth + 1)
      row.balance = own + below
      total += row.balance
    }
    return total
  }
  walk(null, 0)
  return out
}

function AccountsPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const [addOpen, setAddOpen] = useState(false)
  const [failure, setFailure] = useState<unknown>(null)
  const [busyId, setBusyId] = useState<string | null>(null)

  const rows = useMemo(() => (state ? buildTree(state, asOf) : []), [state, asOf])
  const system = useMemo(
    () => new Set(state ? Object.values(state.settings.system_accounts) : []),
    [state],
  )
  const inactive = rows.filter((r) => !r.account.is_active).length

  const toggle = async (a: Account) => {
    setFailure(null)
    setBusyId(a.id)
    try {
      await command.mutateAsync({ type: 'account.update', accountId: a.id, patch: { is_active: !a.is_active } })
    } catch (e) {
      setFailure(e)
    } finally {
      setBusyId(null)
    }
  }

  return (
    <BooksPage
      title="Chart of accounts"
      lede="The accounts every entry posts to, grouped under header accounts. Balances are the natural way round: credit accounts such as income and liabilities show as positive."
      actions={
        <Button size="lg" onClick={() => setAddOpen(true)} disabled={!state}>
          <Plus />
          Add an account
        </Button>
      }
      nav={<SubNav label="Ledger" items={LEDGER_NAV} />}
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="chart of accounts" />
      ) : (
        <>
          {!!failure && <IssueSummary error={failure} title="Not changed" />}
          <ReportSection
            title="Accounts"
            meta={
              <>
                <span>
                  {rows.length} accounts{inactive ? ` · ${inactive} inactive` : ''}
                </span>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            {rows.length === 0 ? (
              <ReportNotice title="No accounts yet">Add the first account to start the books.</ReportNotice>
            ) : (
              <>
                <div className="hidden overflow-x-auto md:block">
                  <table className="w-full min-w-[52rem] text-sm">
                    <caption className="sr-only">
                      Chart of accounts as a tree, with type, expense function, balance and status
                    </caption>
                    <thead>
                      <tr className="border-b border-border text-left text-xs text-muted-foreground">
                        <th scope="col" className="py-2.5 pr-4 font-semibold">Code</th>
                        <th scope="col" className="py-2.5 pr-4 font-semibold">Account</th>
                        <th scope="col" className="py-2.5 pr-4 font-semibold">Type</th>
                        <th scope="col" className="py-2.5 pr-4 font-semibold">Function</th>
                        <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Balance</th>
                        <th scope="col" className="py-2.5 text-right font-semibold">
                          <span className="sr-only">Status and actions</span>
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {rows.map(({ account: a, depth, balance }) => (
                        <tr
                          key={a.id}
                          className={cn(
                            'border-b border-border align-middle',
                            a.is_group && depth === 0 && 'border-t-2 border-t-foreground',
                            !a.is_active && 'text-muted-foreground',
                          )}
                        >
                          <td className="py-2 pr-4 font-mono text-xs">{a.code}</td>
                          <td className="py-2 pr-4">
                            <span style={{ paddingLeft: `${depth * 1.25}rem` }} className="inline-flex flex-wrap items-center gap-x-2">
                              <span className={cn(a.is_group && 'font-semibold')}>{a.name}</span>
                              <AccountTags account={a} system={system.has(a.id)} />
                            </span>
                          </td>
                          <td className="py-2 pr-4">{TYPE_LABEL[a.type]}</td>
                          <td className="py-2 pr-4 text-muted-foreground">
                            {a.expense_function ? FUNCTION_LABEL[a.expense_function] : '–'}
                          </td>
                          <td className="py-2 pr-4 text-right">
                            {a.is_group ? (
                              <Money value={natural(a, balance)} strong />
                            ) : (
                              <Link
                                to="/ledger/general"
                                search={{ account: a.id }}
                                className="underline-offset-4 hover:underline focus-visible:underline"
                                aria-label={`${a.code} ${a.name} in the general ledger`}
                              >
                                <Money value={natural(a, balance)} />
                              </Link>
                            )}
                          </td>
                          <td className="py-1 text-right">
                            <ActiveToggle
                              account={a}
                              system={system.has(a.id)}
                              busy={busyId === a.id}
                              onToggle={() => toggle(a)}
                            />
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <ul className="divide-y divide-border md:hidden">
                  {rows.map(({ account: a, depth, balance }) => (
                    <li
                      key={a.id}
                      className={cn('py-2.5', !a.is_active && 'text-muted-foreground')}
                      style={{ paddingLeft: `${Math.min(depth, 3) * 0.875}rem` }}
                    >
                      <div className="flex items-start justify-between gap-3">
                        <p className="min-w-0 text-sm">
                          <span className="font-mono text-xs text-muted-foreground">{a.code}</span>{' '}
                          <span className={cn(a.is_group && 'font-semibold')}>{a.name}</span>
                        </p>
                        <Money value={natural(a, balance)} strong={a.is_group} className="shrink-0 text-sm" />
                      </div>
                      <div className="mt-1 flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
                        <p className="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                          {TYPE_LABEL[a.type]}
                          {a.expense_function && <span>· {FUNCTION_LABEL[a.expense_function]}</span>}
                          <AccountTags account={a} system={system.has(a.id)} />
                        </p>
                        <ActiveToggle account={a} system={system.has(a.id)} busy={busyId === a.id} onToggle={() => toggle(a)} />
                      </div>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </ReportSection>
          <AddAccountDialog open={addOpen} onOpenChange={setAddOpen} state={state} />
        </>
      )}
    </BooksPage>
  )
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex h-5 items-center border border-border px-1.5 text-[0.6875rem] font-semibold tracking-[0.08em] text-muted-foreground uppercase">
      {children}
    </span>
  )
}

function AccountTags({ account: a, system }: { account: Account; system: boolean }) {
  return (
    <>
      {a.is_group && <Tag>Header</Tag>}
      {a.is_cash && <Tag>Cash</Tag>}
      {system && <Tag>System</Tag>}
      {!a.is_active && <Tag>Inactive</Tag>}
    </>
  )
}

function ActiveToggle({
  account: a,
  system,
  busy,
  onToggle,
}: {
  account: Account
  system: boolean
  busy: boolean
  onToggle: () => void
}) {
  if (system) return null
  return (
    <Button
      variant="ghost"
      size="sm"
      className="h-10"
      onClick={onToggle}
      disabled={busy}
      aria-label={`${a.is_active ? 'Deactivate' : 'Reactivate'} ${a.code} ${a.name}`}
    >
      {busy ? 'Saving…' : a.is_active ? 'Deactivate' : 'Reactivate'}
    </Button>
  )
}

type NewAccount = {
  code: string
  name: string
  type: AccountType
  parent: string
  isGroup: boolean
  isCash: boolean
  fn: ExpenseFunction | ''
}

const EMPTY: NewAccount = { code: '', name: '', type: 'expense', parent: '', isGroup: false, isCash: false, fn: 'program' }

function AddAccountDialog({
  open,
  onOpenChange,
  state,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  state: LedgerState
}) {
  const command = useLedgerCommand()
  const [f, setF] = useState<NewAccount>(EMPTY)
  const [errors, setErrors] = useState<Partial<Record<'code' | 'name', string>>>({})
  const [failure, setFailure] = useState<unknown>(null)

  const parents = state.accounts
    .filter((a) => a.is_group && a.type === f.type)
    .sort((a, b) => a.code.localeCompare(b.code))

  const set = <TKey extends keyof NewAccount>(k: TKey, v: NewAccount[TKey]) =>
    setF((prev) => {
      const next = { ...prev, [k]: v }
      if (k === 'type') next.parent = ''
      return next
    })

  const close = (o: boolean) => {
    onOpenChange(o)
    if (!o) {
      setF(EMPTY)
      setErrors({})
      setFailure(null)
    }
  }

  const submit = async () => {
    const e: typeof errors = {}
    if (!f.code.trim()) e.code = 'Enter a code, such as 5150.'
    else if (state.accounts.some((a) => a.code === f.code.trim())) e.code = `Code ${f.code.trim()} is already in use.`
    if (!f.name.trim()) e.name = 'Enter the account name.'
    setErrors(e)
    if (Object.keys(e).length) return
    setFailure(null)
    try {
      await command.mutateAsync({
        type: 'account.add',
        input: {
          code: f.code.trim(),
          name: f.name.trim(),
          type: f.type,
          parent_id: f.parent || null,
          is_group: f.isGroup,
          is_cash: f.type === 'asset' && !f.isGroup && f.isCash,
          expense_function: f.type === 'expense' && !f.isGroup && f.fn ? f.fn : null,
          cash_flow_class: null,
        },
      })
      close(false)
    } catch (err) {
      setFailure(err)
    }
  }

  return (
    <ReportDialog
      open={open}
      onOpenChange={close}
      title="Add an account"
      description="New accounts are active straight away. An account cannot be deleted once added, only deactivated."
      footer={
        <>
          <Button variant="ghost" onClick={() => close(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={command.isPending}>
            {command.isPending ? 'Adding…' : 'Add account'}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <IssueSummary error={failure} title="Not added" />
        <div className="grid gap-4 sm:grid-cols-[8rem_minmax(0,1fr)]">
          <Field id="acc-code" label="Code" required error={errors.code}>
            {(props) => (
              <Input {...props} value={f.code} onChange={(e) => set('code', e.target.value)} autoComplete="off" className="h-10 font-mono" />
            )}
          </Field>
          <Field id="acc-name" label="Name" required error={errors.name}>
            {(props) => (
              <Input {...props} value={f.name} onChange={(e) => set('name', e.target.value)} autoComplete="off" className="h-10" />
            )}
          </Field>
        </div>
        <Field id="acc-type" label="Type" fixed>
          {(props) => (
            <FormSelect
              {...props}
              value={f.type}
              onValueChange={(v) => set('type', v as AccountType)}
              options={(Object.keys(TYPE_LABEL) as Array<AccountType>).map((t) => ({ value: t, label: TYPE_LABEL[t] }))}
            />
          )}
        </Field>
        <Field id="acc-parent" label="Parent" hint="Header accounts of the same type">
          {(props) => (
            <FormSelect
              {...props}
              value={f.parent}
              onValueChange={(v) => set('parent', v)}
              options={[
                { value: '', label: 'None, top level' },
                ...parents.map((a) => ({ value: a.id, label: `${a.code} · ${a.name}` })),
              ]}
            />
          )}
        </Field>
        <CheckboxField
          checked={f.isGroup}
          onCheckedChange={(v) => set('isGroup', v)}
          label="Header account"
          hint="Groups other accounts. Entries cannot post to it directly."
        />
        {f.type === 'asset' && !f.isGroup && (
          <CheckboxField
            checked={f.isCash}
            onCheckedChange={(v) => set('isCash', v)}
            label="Cash account"
            hint="A bank, mobile money or petty cash account. Counts as cash in the cash flow statement."
          />
        )}
        {f.type === 'expense' && !f.isGroup && (
          <Field id="acc-function" label="Expense function" fixed hint="Where this spend appears in the Statement of Functional Expenses">
            {(props) => (
              <FormSelect
                {...props}
                value={f.fn}
                onValueChange={(v) => set('fn', v as ExpenseFunction)}
                options={(Object.keys(FUNCTION_LABEL) as Array<ExpenseFunction>).map((x) => ({ value: x, label: FUNCTION_LABEL[x] }))}
              />
            )}
          </Field>
        )}
      </div>
    </ReportDialog>
  )
}
