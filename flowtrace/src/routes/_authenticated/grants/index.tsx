import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowRight } from '@phosphor-icons/react'
import type { GrantSummary } from '@/lib/ledger'
import { grantSummary } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
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
  Money,
  SubNav,
  formatMinorCompact,
} from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/grants/')({
  component: GrantsPage,
})

export const GRANTS_NAV = [
  { to: '/grants', label: 'Grants', exact: true },
  { to: '/grants/budgets', label: 'Budget vs actual' },
]

const STATUS_TEXT: Record<GrantSummary['status'], string> = {
  within: 'Within award',
  warning: '80% used',
  over: 'Over award',
}

function GrantsPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()

  const rows = useMemo(() => {
    if (!state) return []
    return state.grants
      .map((g) => ({
        grant: g,
        fund: state.funds.find((f) => f.id === g.fund_id),
        s: grantSummary(state, g.id, asOf),
      }))
      .sort((a, b) => b.s.used - a.s.used)
  }, [state, asOf])

  const totals = useMemo(
    () =>
      rows.reduce(
        (t, r) => ({
          award: t.award + r.s.award,
          spent: t.spent + r.s.spent,
          committed: t.committed + r.s.committed,
          remaining: t.remaining + r.s.remaining,
        }),
        { award: 0, spent: 0, committed: 0, remaining: 0 },
      ),
    [rows],
  )
  const base = state?.settings.base_currency ?? 'KES'

  return (
    <BooksPage
      title="Grants"
      lede="Each award with what has been spent against it, what is committed and what is left. Spend posts here when a claim is approved or a bill is posted."
      nav={<SubNav label="Grants" items={GRANTS_NAV} />}
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="grants" />
      ) : (
        <>
          <ReportSection
            title="Across all grants"
            meta={
              <>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            <KeyFigures>
              <KeyFigure scale="lg" value={formatMinorCompact(totals.award)} label={`Awarded, ${base}`} />
              <KeyFigure value={formatMinorCompact(totals.spent)} label="Spent" />
              <KeyFigure value={formatMinorCompact(totals.committed)} label="Committed" detail={<span className="text-muted-foreground">Approved bills and submitted claims</span>} />
              <KeyFigure value={formatMinorCompact(totals.remaining)} label="Remaining" />
            </KeyFigures>
          </ReportSection>

          <ReportSection title="Awards" meta={<span>{rows.length} grants</span>}>
            {rows.length === 0 ? (
              <ReportNotice title="No grants recorded">
                Add a grant to start charging spend to it.
              </ReportNotice>
            ) : (
              <ul className="divide-y divide-border">
                {rows.map(({ grant, fund, s }) => (
                  <li key={grant.id}>
                    <Link
                      to="/grants/$id"
                      params={{ id: grant.id }}
                      className="group -mx-4 grid gap-4 px-4 py-5 transition-colors hover:bg-muted md:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]"
                    >
                      <div className="min-w-0">
                        <p className="flex items-center gap-2 font-semibold">
                          {grant.name}
                          <ArrowRight aria-hidden="true" className="size-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                        </p>
                        <p className="mt-1 text-sm text-muted-foreground">
                          {grant.donor_name} · {fund?.name ?? 'Fund'} ·{' '}
                          {formatDate(grant.start_date)} to {formatDate(grant.end_date)}
                        </p>
                        <p
                          className={cn(
                            'mt-2 text-xs font-semibold',
                            s.status === 'over' ? 'text-destructive' : s.status === 'warning' ? 'text-flag' : 'text-muted-foreground',
                          )}
                        >
                          {STATUS_TEXT[s.status]} · {Math.round(s.used * 100)}% spent or committed
                        </p>
                      </div>
                      <div className="min-w-0">
                        <CoverageBar
                          total={s.award}
                          filled={s.spent + s.committed}
                          evidenced={s.spent}
                          label={`${Math.round((s.spent / Math.max(1, s.award)) * 100)}% spent, ${Math.round((s.committed / Math.max(1, s.award)) * 100)}% committed`}
                        />
                        <dl className="mt-2 grid grid-cols-3 gap-2 text-xs">
                          <div>
                            <dt className="text-muted-foreground">Spent</dt>
                            <dd><Money value={s.spent} decimals={0} strong /></dd>
                          </div>
                          <div>
                            <dt className="text-muted-foreground">Committed</dt>
                            <dd><Money value={s.committed} decimals={0} /></dd>
                          </div>
                          <div className="text-right">
                            <dt className="text-muted-foreground">Remaining of <Money value={s.award} decimals={0} /></dt>
                            <dd><Money value={s.remaining} decimals={0} strong /></dd>
                          </div>
                        </dl>
                      </div>
                    </Link>
                  </li>
                ))}
              </ul>
            )}
            <p className="flex flex-wrap gap-x-5 gap-y-1 border-t border-border py-3 text-xs text-muted-foreground">
              <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-primary" /> Spent</span>
              <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-report" /> Committed</span>
              <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-gap" /> Remaining</span>
            </p>
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}
