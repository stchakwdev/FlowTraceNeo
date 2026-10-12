import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowRight } from '@phosphor-icons/react'
import { balances, ledgerSignals } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate } from '@/lib/format'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { BooksPage, BooksState, Money } from '@/components/ledger'
import { MatchLine } from '@/components/ledger/reports/match-mark'

export const Route = createFileRoute('/_authenticated/reconcile/')({
  component: ReconcilePage,
})

const KIND_LABEL = { bank: 'Bank', mobile_money: 'Mobile money', cash: 'Cash' } as const

function ReconcilePage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()

  const rows = useMemo(() => {
    if (!state) return []
    const bal = balances(state, { to: asOf })
    return state.bankAccounts.map((bank) => {
      const lines = state.statementLines.filter((s) => s.bank_account_id === bank.id)
      const matched = lines.filter((s) => s.matched_line_id).length
      const last = lines.reduce<string | null>((m, s) => (!m || s.date > m ? s.date : m), null)
      return {
        bank,
        gl: state.accounts.find((a) => a.id === bank.account_id),
        balance: bal.get(bank.account_id) ?? 0,
        lines: lines.length,
        matched,
        unmatched: lines.length - matched,
        last,
      }
    })
  }, [state, asOf])

  const signals = useMemo(() => (state ? ledgerSignals(state, asOf) : null), [state, asOf])
  const totals = rows.reduce(
    (t, r) => ({ lines: t.lines + r.lines, unmatched: t.unmatched + r.unmatched }),
    { lines: 0, unmatched: 0 },
  )
  const base = state?.settings.base_currency ?? 'KES'

  return (
    <BooksPage
      title="Reconcile"
      lede="Each bank and mobile-money account set against its statement. A line is reconciled when it is matched to a posted journal line; anything left over is money the books have not explained yet."
    >
      {!state || !signals ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="bank accounts" />
      ) : (
        <>
          <ReportSection
            title="Across all accounts"
            meta={
              <>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            <KeyFigures>
              <KeyFigure
                scale="lg"
                value={`${Math.round(signals.reconciledShare * 100)}%`}
                label="Cash movement reconciled"
                detail={<span className="text-muted-foreground">By value, opening balances left out</span>}
              />
              <KeyFigure value={rows.length} label="Accounts" />
              <KeyFigure value={totals.lines} label="Statement lines" />
              <KeyFigure
                value={totals.unmatched}
                label="Unmatched lines"
                detail={
                  totals.unmatched > 0 ? (
                    <span className="inline-flex items-center gap-2 font-semibold text-flag">
                      <MatchLine state="unmatched" />
                      To explain
                    </span>
                  ) : (
                    <span className="text-muted-foreground">Nothing left to explain</span>
                  )
                }
              />
            </KeyFigures>
          </ReportSection>

          <ReportSection title="Accounts" meta={<span>{rows.length} accounts</span>}>
            {rows.length === 0 ? (
              <ReportNotice title="No bank or mobile-money accounts yet">
                Link a bank or mobile-money account to a cash account in the chart, then import its statement here.
              </ReportNotice>
            ) : (
              <ul className="divide-y divide-border">
                {rows.map((r) => {
                  const share = r.lines > 0 ? r.matched / r.lines : 0
                  return (
                    <li key={r.bank.id}>
                      <Link
                        to="/reconcile/$bankId"
                        params={{ bankId: r.bank.id }}
                        className="group -mx-4 grid gap-4 px-4 py-5 transition-colors hover:bg-muted md:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]"
                      >
                        <div className="min-w-0">
                          <p className="flex items-center gap-2 font-semibold">
                            {r.bank.name}
                            <ArrowRight aria-hidden="true" className="size-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                          </p>
                          <p className="mt-1 text-sm text-muted-foreground">
                            {KIND_LABEL[r.bank.kind]} · {r.bank.provider} ·{' '}
                            <span className="font-mono text-xs">{r.bank.number_masked}</span>
                          </p>
                          <p className="mt-1 text-sm text-muted-foreground">
                            {r.gl ? `${r.gl.code} ${r.gl.name}` : 'Cash account'} ·{' '}
                            book balance <Money value={r.balance} currency={r.bank.currency} strong className="text-foreground" />
                          </p>
                        </div>
                        <div className="min-w-0">
                          {r.lines === 0 ? (
                            <p className="flex h-5 items-center gap-2 text-sm text-muted-foreground">
                              <MatchLine state="ambiguous" />
                              No statement imported yet
                            </p>
                          ) : (
                            <CoverageBar
                              total={r.lines}
                              filled={r.matched}
                              evidenced={r.matched}
                              label={`${Math.round(share * 100)}% of ${r.lines} statement lines matched`}
                            />
                          )}
                          <dl className="mt-2 grid grid-cols-3 gap-2 text-xs">
                            <div>
                              <dt className="text-muted-foreground">Matched</dt>
                              <dd className="font-semibold tabular-nums">
                                {r.matched} of {r.lines}
                              </dd>
                            </div>
                            <div>
                              <dt className="text-muted-foreground">Unmatched</dt>
                              <dd className="tabular-nums">
                                {r.unmatched > 0 ? (
                                  <span className="inline-flex items-center gap-1.5 font-semibold text-flag">
                                    <MatchLine state="unmatched" width={16} />
                                    {r.unmatched}
                                  </span>
                                ) : (
                                  '0'
                                )}
                              </dd>
                            </div>
                            <div className="text-right">
                              <dt className="text-muted-foreground">Last statement line</dt>
                              <dd className="tabular-nums">{r.last ? formatDate(r.last) : 'None'}</dd>
                            </div>
                          </dl>
                        </div>
                      </Link>
                    </li>
                  )
                })}
              </ul>
            )}
            <p className="flex flex-wrap gap-x-5 gap-y-1 border-t border-border py-3 text-xs text-muted-foreground">
              <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-primary" /> Matched statement lines</span>
              <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-gap" /> Not matched yet</span>
              <span>Book balance in {base}, {formatDate(asOf)}</span>
            </p>
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}
