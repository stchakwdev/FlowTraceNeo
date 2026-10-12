// Selects for the ledger's reference data, built on the shared FormSelect.
import { useMemo } from 'react'
import type { ControlProps } from '@/components/form'
import type { AccountType, LedgerState } from '@/lib/ledger'
import { FormSelect } from '@/components/form'

const TYPE_LABEL: Record<AccountType, string> = {
  asset: 'Assets',
  liability: 'Liabilities',
  equity: 'Net assets',
  income: 'Income',
  expense: 'Expenses',
}

type PickerProps = ControlProps & {
  state: LedgerState
  value: string
  onValueChange: (v: string) => void
  placeholder?: string
  disabled?: boolean
  className?: string
}

/** Postable, active accounts grouped by type; optionally limited to types or cash. */
export function AccountSelect({
  state,
  types,
  cashOnly,
  ...props
}: PickerProps & { types?: Array<AccountType>; cashOnly?: boolean }) {
  const groups = useMemo(() => {
    const out: Array<{ label: string; options: Array<{ value: string; label: string }> }> = []
    for (const type of Object.keys(TYPE_LABEL) as Array<AccountType>) {
      if (types && !types.includes(type)) continue
      const options = state.accounts
        .filter((a) => a.type === type && !a.is_group && a.is_active && (!cashOnly || a.is_cash))
        .sort((a, b) => a.code.localeCompare(b.code))
        .map((a) => ({ value: a.id, label: `${a.code} · ${a.name}` }))
      if (options.length) out.push({ label: TYPE_LABEL[type], options })
    }
    return out
  }, [state.accounts, types, cashOnly])
  return <FormSelect {...props} placeholder={props.placeholder ?? 'Choose an account'} groups={groups} />
}

export function FundSelect({ state, ...props }: PickerProps) {
  return (
    <FormSelect
      {...props}
      placeholder={props.placeholder ?? 'Choose a fund'}
      options={state.funds.map((f) => ({ value: f.id, label: f.name }))}
    />
  )
}

/** Grants, optionally only those on `fundId`; '' means no grant. */
export function GrantSelect({ state, fundId, ...props }: PickerProps & { fundId?: string }) {
  return (
    <FormSelect
      {...props}
      options={[
        { value: '', label: 'No grant' },
        ...state.grants
          .filter((g) => !fundId || g.fund_id === fundId)
          .map((g) => ({ value: g.id, label: g.name })),
      ]}
    />
  )
}

/** Dimension values grouped by kind; '' means none. */
export function DimensionSelect({ state, ...props }: PickerProps) {
  const groups = useMemo(() => {
    const kinds = ['project', 'department', 'location', 'donor'] as const
    const label = { project: 'Projects', department: 'Departments', location: 'Locations', donor: 'Donors' }
    return kinds
      .map((k) => ({
        label: label[k],
        options: state.dimensions.filter((d) => d.kind === k).map((d) => ({ value: d.id, label: d.name })),
      }))
      .filter((g) => g.options.length)
  }, [state.dimensions])
  return <FormSelect {...props} options={[{ value: '', label: 'None' }]} groups={groups} />
}
