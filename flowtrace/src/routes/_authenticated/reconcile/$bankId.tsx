import { Link, createFileRoute } from '@tanstack/react-router'
import { useId, useMemo, useState } from 'react'
import { ArrowLeft, ArrowsLeftRight, FileArrowUp, LinkBreak, MagicWand } from '@phosphor-icons/react'
import type { MatchState } from '@/components/ledger/reports/match-mark'
import type {
  AutoMatchResult,
  BankAccount,
  Ledger,
  LedgerState,
  ParseResult,
  StatementFormat,
  StatementLine,
} from '@/lib/ledger'
import {
  UNRESTRICTED_FUND_ID,
  balances,
  candidatesFor,
  parseStatement,
} from '@/lib/ledger'
import { useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { ReportDialog } from '@/components/report-dialog'
import { Field } from '@/components/form'
import {
  AsOf,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import {
  AccountSelect,
  BooksPage,
  BooksState,
  FundSelect,
  GrantSelect,
  IssueSummary,
  Money,
  formatMinor,
  formatMinorCompact,
} from '@/components/ledger'
import {
  MATCH_SHORT,
  MatchLegend,
  MatchLine,
  MatchMark,
} from '@/components/ledger/reports/match-mark'

type BankSearch = { state?: string }

const str = (v: unknown) => (typeof v === 'string' && v ? v : undefined)

export const Route = createFileRoute('/_authenticated/reconcile/$bankId')({
  validateSearch: (search: Record<string, unknown>): BankSearch => ({
    state: str(search.state),
  }),
  component: BankPage,
})

const STATES: Array<MatchState> = ['auto', 'manual', 'ambiguous', 'unmatched']
const isState = (v?: string): v is MatchState => STATES.includes(v as MatchState)

const FORMAT_LABEL: Record<StatementFormat, string> = {
  mpesa: 'Safaricom M-Pesa statement',
  mtn: 'MTN MoMo statement',
  airtel: 'Airtel Money statement',
  bank: 'Bank CSV',
}

// autoMatch uses a three-day window; manual matching looks a little wider.
const AUTO_WINDOW = 3
const MANUAL_WINDOW = 7

function matchState(ledger: Ledger, s: StatementLine): MatchState {
  if (s.matched_line_id) return s.match_kind === 'auto' ? 'auto' : 'manual'
  return candidatesFor(ledger, s, AUTO_WINDOW).length > 1 ? 'ambiguous' : 'unmatched'
}


function BankPage() {
  const { bankId } = Route.useParams()
  const search = Route.useSearch()
  const filter = isState(search.state) ? search.state : undefined
  const { state, ledger, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()

  const bank = state?.bankAccounts.find((b) => b.id === bankId)

  const lines = useMemo(() => {
    if (!state || !ledger || !bank) return []
    return state.statementLines
      .filter((s) => s.bank_account_id === bank.id)
      .sort((a, b) => b.date.localeCompare(a.date))
      .map((s) => {
        const jl = s.matched_line_id ? state.lines.find((l) => l.id === s.matched_line_id) : undefined
        return { s, state: matchState(ledger, s), entry: jl ? ledger.entry(jl.entry_id) : null }
      })
  }, [state, ledger, bank])

  const counts = useMemo(() => {
    const c: Record<MatchState, number> = { auto: 0, manual: 0, ambiguous: 0, unmatched: 0 }
    for (const l of lines) c[l.state]++
    return c
  }, [lines])

  const shown = filter ? lines.filter((l) => l.state === filter) : lines

  const [importOpen, setImportOpen] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const [autoResult, setAutoResult] = useState<AutoMatchResult | null>(null)
  const [actionError, setActionError] = useState<unknown>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [matching, setMatching] = useState<StatementLine | null>(null)
  const [posting, setPosting] = useState<StatementLine | null>(null)

  const runAutoMatch = async () => {
    setActionError(null)
    setNotice(null)
    try {
      const r = await command.mutateAsync({ type: 'statement.autoMatch', bankAccountId: bankId })
      setAutoResult(r.type === 'autoMatch' ? r.result : null)
    } catch (e) {
      setAutoResult(null)
      setActionError(e)
    }
  }

  const unmatch = async (s: StatementLine) => {
    setActionError(null)
    setAutoResult(null)
    setBusyId(s.id)
    try {
      await command.mutateAsync({ type: 'statement.unmatch', statementLineId: s.id })
      setNotice(`Unmatched “${s.description}”. It is back in the unmatched lines.`)
    } catch (e) {
      setActionError(e)
    } finally {
      setBusyId(null)
    }
  }

  if (!state || !ledger) {
    return (
      <BooksPage title="Reconcile" nav={<BackLink />}>
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="statement" />
      </BooksPage>
    )
  }

  if (!bank) {
    return (
      <BooksPage title="Reconcile" nav={<BackLink />}>
        <ReportNotice title="This account is not in the books">
          It may have been removed, or the link is out of date. Go back to the list of accounts.
        </ReportNotice>
      </BooksPage>
    )
  }

  const busy = command.isPending

  return (
    <BooksPage
      title={bank.name}
      lede={
        <>
          {bank.provider} · <span className="font-mono text-xs">{bank.number_masked}</span> · {bank.currency}
        </>
      }
      nav={<BackLink />}
      actions={
        <>
          <Button variant="outline" onClick={() => setImportOpen(true)}>
            <FileArrowUp aria-hidden="true" />
            Import statement
          </Button>
          <Button onClick={runAutoMatch} disabled={busy || counts.unmatched + counts.ambiguous === 0}>
            <MagicWand aria-hidden="true" />
            {busy && !busyId ? 'Matching…' : 'Auto-match'}
          </Button>
        </>
      }
    >
      <div aria-live="polite" className="empty:hidden">
        {autoResult && <AutoMatchReport result={autoResult} />}
        {notice && <p className="border-y border-border py-3 text-sm">{notice}</p>}
      </div>
      <IssueSummary error={actionError} title="That did not go through" />

      <Summary state={state} bank={bank} asOf={asOf} lines={lines.map((l) => l.s)} counts={counts} />

      <ReportSection title="Statement lines" meta={<MatchLegend className="hidden lg:flex" />}>
        <div className="border-b border-border py-3">
          <nav aria-label="Filter by match state">
            <ul className="flex flex-wrap gap-px border border-border bg-border">
              {[undefined, ...STATES].map((s) => {
                const active = filter === s
                return (
                  <li key={s ?? 'all'} className="grow bg-background sm:grow-0">
                    <Link
                      to="/reconcile/$bankId"
                      params={{ bankId }}
                      search={{ state: s }}
                      replace
                      aria-current={active ? 'page' : undefined}
                      className={cn(
                        'flex h-10 items-center justify-center gap-2 px-3 text-sm whitespace-nowrap transition-colors',
                        active ? 'bg-primary font-semibold text-primary-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                      )}
                    >
                      {s && (
                        <MatchLine
                          state={s}
                          width={18}
                          className={cn(active && '[&_line]:stroke-primary-foreground [&_path]:stroke-primary-foreground')}
                        />
                      )}
                      {s ? MATCH_SHORT[s] : 'All'}
                      <span className="tabular-nums opacity-80">{s ? counts[s] : lines.length}</span>
                    </Link>
                  </li>
                )
              })}
            </ul>
          </nav>
        </div>

        {lines.length === 0 ? (
          <ReportNotice
            title="No statement imported yet"
            action={
              <Button variant="outline" onClick={() => setImportOpen(true)}>
                <FileArrowUp aria-hidden="true" />
                Import statement
              </Button>
            }
          >
            Import a CSV from the bank, or an M-Pesa, MTN MoMo or Airtel Money statement, to start matching it against the books.
          </ReportNotice>
        ) : shown.length === 0 ? (
          <ReportNotice
            title="No lines in this state"
            action={
              <Link to="/reconcile/$bankId" params={{ bankId }} search={{}} className="text-sm font-medium text-primary hover:underline">
                Show all lines
              </Link>
            }
          >
            Every line on this statement is in another state.
          </ReportNotice>
        ) : (
          <ul className="divide-y divide-border">
            {shown.map(({ s, state: ms, entry }) => (
              <li key={s.id} className="grid gap-x-6 gap-y-2 py-4 md:grid-cols-[6.5rem_minmax(0,1fr)_9rem_minmax(0,15rem)] md:items-start">
                <p className="text-sm text-muted-foreground tabular-nums">
                  <time dateTime={s.date}>{formatDate(s.date)}</time>
                </p>
                <div className="min-w-0">
                  <p className="font-semibold break-words">{s.description}</p>
                  <p className="mt-0.5 font-mono text-xs text-muted-foreground">{s.reference ?? 'No reference'}</p>
                  <MatchMark state={ms} className="mt-2" />
                </div>
                <p className="text-left md:text-right">
                  <Money value={s.amount} signed strong />
                  <span className="block text-xs text-muted-foreground">{s.amount > 0 ? 'Money in' : 'Money out'}</span>
                </p>
                <div className="flex flex-wrap items-center gap-2 md:justify-end">
                  {s.matched_line_id ? (
                    <>
                      {entry && (
                        <Link to="/ledger/entries/$id" params={{ id: entry.id }} className="inline-flex min-h-10 items-center text-sm font-medium text-primary hover:underline">
                          {entry.number}
                        </Link>
                      )}
                      <Button variant="ghost" onClick={() => unmatch(s)} disabled={busy}>
                        <LinkBreak aria-hidden="true" />
                        {busyId === s.id ? 'Unmatching…' : 'Unmatch'}
                        <span className="sr-only"> {s.description}</span>
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button variant="outline" onClick={() => { setActionError(null); setMatching(s) }} disabled={busy}>
                        <ArrowsLeftRight aria-hidden="true" />
                        Match
                        <span className="sr-only"> {s.description}</span>
                      </Button>
                      <Button variant="ghost" onClick={() => { setActionError(null); setPosting(s) }} disabled={busy}>
                        Post as new entry
                        <span className="sr-only"> for {s.description}</span>
                      </Button>
                    </>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
        <MatchLegend className="border-t border-border py-3 lg:hidden" />
      </ReportSection>

      <ImportDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        bank={bank}
        onImported={(added, total) => {
          setAutoResult(null)
          setNotice(
            added === total
              ? `${added} statement ${added === 1 ? 'line' : 'lines'} imported. Run Auto-match to pair them with the books.`
              : `${added} of ${total} lines imported; ${total - added} were already on this statement and were left out.`,
          )
        }}
      />
      {matching && (
        <MatchDialog
          line={matching}
          ledger={ledger}
          onClose={() => setMatching(null)}
          onMatched={(n) => {
            setAutoResult(null)
            setNotice(`Matched “${matching.description}” to ${n}.`)
          }}
        />
      )}
      {posting && (
        <PostDialog
          line={posting}
          state={state}
          onClose={() => setPosting(null)}
          onPosted={() => {
            setAutoResult(null)
            setNotice(`Posted “${posting.description}” as a new entry and matched it.`)
          }}
        />
      )}
    </BooksPage>
  )
}

function BackLink() {
  return (
    <Link to="/reconcile" className="inline-flex min-h-10 items-center gap-1.5 text-sm font-medium text-muted-foreground hover:text-foreground">
      <ArrowLeft aria-hidden="true" className="size-4" />
      All accounts
    </Link>
  )
}

function AutoMatchReport({ result }: { result: AutoMatchResult }) {
  const n = (x: number, one: string, many: string) => `${x} ${x === 1 ? one : many}`
  return (
    <div className="rule-heavy py-3 text-sm">
      <p className="font-semibold">Auto-match finished</p>
      <ul className="mt-2 flex flex-wrap gap-x-6 gap-y-1">
        <li className="inline-flex items-center gap-2">
          <MatchLine state="auto" />
          {n(result.matched.length, 'line', 'lines')} matched
        </li>
        <li className="inline-flex items-center gap-2">
          <MatchLine state="ambiguous" />
          {n(result.ambiguous.length, 'line needs', 'lines need')} a decision
        </li>
        <li className="inline-flex items-center gap-2">
          <MatchLine state="unmatched" />
          {n(result.unmatched.length, 'line', 'lines')} unmatched
        </li>
      </ul>
    </div>
  )
}

/**
 * The account's position: book balance, and the statement against the
 * books over the statement's dates. Matched lines agree on both sides, so
 * the difference is what is only on the statement less what is only in
 * the books.
 */
function Summary({
  state,
  bank,
  asOf,
  lines,
  counts,
}: {
  state: LedgerState
  bank: BankAccount
  asOf: string
  lines: Array<StatementLine>
  counts: Record<MatchState, number>
}) {
  const s = useMemo(() => {
    const bookBalance = balances(state, { to: asOf }).get(bank.account_id) ?? 0
    if (lines.length === 0) return { bookBalance, from: null, to: null, matched: 0, onlyStatement: 0, onlyBooks: 0, onlyBooksCount: 0 }
    const dates = lines.map((l) => l.date).sort()
    const from = dates[0]
    const to = dates[dates.length - 1]
    const matchedIds = new Set(state.statementLines.map((x) => x.matched_line_id).filter(Boolean))
    const entries = new Map(state.entries.map((e) => [e.id, e]))
    const matched = lines.filter((l) => l.matched_line_id).reduce((t, l) => t + l.amount, 0)
    const onlyStatement = lines.filter((l) => !l.matched_line_id).reduce((t, l) => t + l.amount, 0)
    const bookOnly = state.lines.filter((l) => {
      if (l.account_id !== bank.account_id || matchedIds.has(l.id)) return false
      const e = entries.get(l.entry_id)
      return !!e && e.status !== 'draft' && e.source_type !== 'opening' && e.date >= from && e.date <= to
    })
    const onlyBooks = bookOnly.reduce((t, l) => t + l.base_amount, 0)
    return { bookBalance, from, to, matched, onlyStatement, onlyBooks, onlyBooksCount: bookOnly.length }
  }, [state, bank, asOf, lines])

  const matchedCount = counts.auto + counts.manual
  const share = lines.length ? matchedCount / lines.length : 0
  const statementNet = s.matched + s.onlyStatement
  const booksNet = s.matched + s.onlyBooks
  const difference = statementNet - booksNet
  const meta = (
    <>
      <AsOf date={asOf} />
      {DEV_MODE && <DemoTag />}
    </>
  )

  return (
    <>
      <ReportSection title="Position" meta={meta}>
        <KeyFigures>
          <KeyFigure scale="lg" value={formatMinorCompact(s.bookBalance)} label={`Book balance, ${bank.currency}`} />
          <KeyFigure value={`${Math.round(share * 100)}%`} label="Statement lines matched" detail={<span className="text-muted-foreground">{matchedCount} of {lines.length}</span>} />
          <KeyFigure value={counts.ambiguous} label="Need a decision" />
          <KeyFigure
            value={counts.unmatched}
            label="Unmatched"
            detail={counts.unmatched > 0 ? <span className="font-semibold text-flag">To explain</span> : undefined}
          />
        </KeyFigures>
      </ReportSection>

      {s.from && s.to && (
        <ReportSection title="Statement against the books" meta={<span>{formatDate(s.from)} to {formatDate(s.to)}</span>}>
          <div className="overflow-x-auto">
            <table className="w-full min-w-[30rem] text-sm">
              <caption className="sr-only">
                Statement movement against book movement on {bank.name}, {formatDate(s.from)} to {formatDate(s.to)}
              </caption>
              <thead>
                <tr className="border-b border-foreground text-xs text-muted-foreground">
                  <th scope="col" className="py-2.5 pr-4 text-left font-semibold">Net movement</th>
                  <th scope="col" className="py-2.5 pl-4 text-right font-semibold">Statement</th>
                  <th scope="col" className="py-2.5 pl-4 text-right font-semibold">Books</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-border">
                  <th scope="row" className="py-2 pr-4 text-left font-normal">Matched lines</th>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(s.matched)}</td>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(s.matched)}</td>
                </tr>
                <tr className="border-b border-border">
                  <th scope="row" className="py-2 pr-4 text-left font-normal">
                    Only on the statement
                    <span className="block text-xs text-muted-foreground">{counts.ambiguous + counts.unmatched} lines not yet in the books</span>
                  </th>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(s.onlyStatement)}</td>
                  <td className="py-2 pl-4 text-right tabular-nums">–</td>
                </tr>
                <tr className="border-b border-border">
                  <th scope="row" className="py-2 pr-4 text-left font-normal">
                    Only in the books
                    <span className="block text-xs text-muted-foreground">{s.onlyBooksCount} journal lines in these dates not on the statement</span>
                  </th>
                  <td className="py-2 pl-4 text-right tabular-nums">–</td>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(s.onlyBooks)}</td>
                </tr>
                <tr className="border-t border-b-[3px] border-double border-foreground font-bold">
                  <th scope="row" className="py-2 pr-4 text-left">Net movement</th>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(statementNet)}</td>
                  <td className="py-2 pl-4 text-right tabular-nums">{formatMinor(booksNet)}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p className="flex flex-col gap-1 border-b border-border py-3 text-sm sm:flex-row sm:justify-between">
            <span className="text-muted-foreground">
              {difference === 0
                ? 'The statement and the books agree over these dates.'
                : 'The difference is what is only on the statement less what is only in the books. Match or post the open lines to close it.'}
            </span>
            <span className="shrink-0 tabular-nums">
              <span className="text-muted-foreground">Difference </span>
              <span className="font-semibold">{formatMinor(difference, { zero: '0.00' })}</span>
            </span>
          </p>
        </ReportSection>
      )}
    </>
  )
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

function ImportDialog({
  open,
  onOpenChange,
  bank,
  onImported,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  bank: BankAccount
  onImported: (added: number, total: number) => void
}) {
  const command = useLedgerCommand()
  const fileId = useId()
  const pasteId = useId()
  const [text, setText] = useState('')
  const [fileName, setFileName] = useState<string | null>(null)
  const [readError, setReadError] = useState<string | null>(null)

  const parsed = useMemo<ParseResult | null>(() => {
    if (!text.trim()) return null
    try {
      return parseStatement(text)
    } catch {
      return null
    }
  }, [text])

  const close = (next: boolean) => {
    if (!next) {
      setText('')
      setFileName(null)
      setReadError(null)
      command.reset()
    }
    onOpenChange(next)
  }

  const readFile = async (file: File | undefined) => {
    setReadError(null)
    if (!file) return
    try {
      setText(await file.text())
      setFileName(file.name)
    } catch {
      setReadError('The file could not be read. Save it as CSV and try again.')
    }
  }

  const submit = async () => {
    if (!parsed?.rows.length) return
    try {
      const r = await command.mutateAsync({ type: 'statement.import', bankAccountId: bank.id, rows: parsed.rows })
      onImported(r.type === 'ids' ? r.ids.length : 0, parsed.rows.length)
      close(false)
    } catch {
      // Shown through command.error.
    }
  }

  const inflow = parsed ? parsed.rows.filter((r) => r.amount > 0).reduce((t, r) => t + r.amount, 0) : 0
  const outflow = parsed ? parsed.rows.filter((r) => r.amount < 0).reduce((t, r) => t + r.amount, 0) : 0

  return (
    <ReportDialog
      open={open}
      onOpenChange={close}
      title="Import statement"
      description={`Into ${bank.name}. Lines already on this statement are left out.`}
      className="max-w-2xl"
      footer={
        <>
          <Button variant="outline" onClick={() => close(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!parsed?.rows.length || command.isPending}>
            {command.isPending
              ? 'Importing…'
              : parsed?.rows.length
                ? `Import ${parsed.rows.length} ${parsed.rows.length === 1 ? 'line' : 'lines'}`
                : 'Import'}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label htmlFor={fileId} className="mb-1.5 block text-sm font-medium">
            Statement file (CSV)
          </label>
          <Input
            id={fileId}
            type="file"
            accept=".csv,text/csv,text/plain"
            onChange={(e) => readFile(e.target.files?.[0])}
            className="h-10 py-1.5"
          />
          {readError && (
            <p role="alert" className="mt-1.5 text-sm text-destructive">
              {readError}
            </p>
          )}
        </div>
        <div>
          <label htmlFor={pasteId} className="mb-1.5 block text-sm font-medium">
            Or paste the statement
          </label>
          <Textarea
            id={pasteId}
            value={text}
            onChange={(e) => {
              setText(e.target.value)
              setFileName(null)
            }}
            rows={5}
            placeholder={'Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn\nRB34NNA81C,2026-02-03 10:12,Funds received,Completed,15000,'}
            className="max-h-48 font-mono text-xs"
          />
          <p className="mt-1 text-xs text-muted-foreground">
            Bank CSVs and M-Pesa, MTN MoMo and Airtel Money statements are recognised from their column names.
          </p>
        </div>

        {text.trim() && !parsed && (
          <p role="alert" className="text-sm text-destructive">
            This does not read as a CSV statement. Check that the first row holds the column names.
          </p>
        )}

        {parsed && (
          <section aria-label="Import preview" className="rule-heavy">
            <dl className="grid grid-cols-2 border-b border-border text-sm sm:grid-cols-4">
              <div className="py-2 pr-3">
                <dt className="text-xs text-muted-foreground">Detected</dt>
                <dd className="font-semibold">{FORMAT_LABEL[parsed.format]}</dd>
              </div>
              <div className="border-l border-border py-2 pl-3">
                <dt className="text-xs text-muted-foreground">Rows to import</dt>
                <dd className="font-semibold tabular-nums">{parsed.rows.length}</dd>
              </div>
              <div className="border-border py-2 pr-3 sm:border-l sm:pl-3">
                <dt className="text-xs text-muted-foreground">Money in</dt>
                <dd className="tabular-nums">{formatMinor(inflow)}</dd>
              </div>
              <div className="border-l border-border py-2 pl-3">
                <dt className="text-xs text-muted-foreground">Money out</dt>
                <dd className="tabular-nums">{formatMinor(outflow)}</dd>
              </div>
            </dl>
            {fileName && <p className="pt-2 text-xs text-muted-foreground">From {fileName}</p>}
            {parsed.rows.length > 0 && (
              <div className="mt-2 overflow-x-auto">
                <table className="w-full min-w-[28rem] text-sm">
                  <caption className="sr-only">First rows of the statement as they will be imported</caption>
                  <thead>
                    <tr className="border-b border-border text-left text-xs text-muted-foreground">
                      <th scope="col" className="py-2 pr-3 font-semibold">Date</th>
                      <th scope="col" className="py-2 pr-3 font-semibold">Description</th>
                      <th scope="col" className="py-2 text-right font-semibold">Amount</th>
                    </tr>
                  </thead>
                  <tbody>
                    {parsed.rows.slice(0, 6).map((r, i) => (
                      <tr key={i} className="border-b border-border align-top">
                        <td className="py-2 pr-3 whitespace-nowrap tabular-nums">{formatDate(r.date)}</td>
                        <td className="py-2 pr-3">
                          {r.description}
                          {r.reference && <span className="block font-mono text-xs text-muted-foreground">{r.reference}</span>}
                        </td>
                        <td className="py-2 text-right whitespace-nowrap tabular-nums">{formatMinor(r.amount, { signed: true })}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {parsed.rows.length > 6 && (
                  <p className="py-2 text-xs text-muted-foreground">and {parsed.rows.length - 6} more</p>
                )}
              </div>
            )}
            {parsed.skipped.length > 0 && (
              <div className="mt-3">
                <p className="text-sm font-semibold">
                  {parsed.skipped.length} {parsed.skipped.length === 1 ? 'row' : 'rows'} skipped
                </p>
                <ul className="mt-1 max-h-32 space-y-0.5 overflow-y-auto text-xs text-muted-foreground">
                  {parsed.skipped.map((k) => (
                    <li key={k.line}>
                      Line {k.line}: {k.reason}
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {parsed.rows.length === 0 && (
              <p role="alert" className="py-3 text-sm text-destructive">
                No rows could be read. Each row needs a date and an amount.
              </p>
            )}
          </section>
        )}
        <IssueSummary error={command.error} title="Not imported" />
      </div>
    </ReportDialog>
  )
}

// ---------------------------------------------------------------------------
// Manual match
// ---------------------------------------------------------------------------

function MatchDialog({
  line,
  ledger,
  onClose,
  onMatched,
}: {
  line: StatementLine
  ledger: Ledger
  onClose: () => void
  onMatched: (entryNumber: string) => void
}) {
  const command = useLedgerCommand()
  const candidates = useMemo(() => candidatesFor(ledger, line, MANUAL_WINDOW), [ledger, line])
  const [pending, setPending] = useState<string | null>(null)

  const match = async (journalLineId: string, entryNumber: string) => {
    setPending(journalLineId)
    try {
      await command.mutateAsync({ type: 'statement.match', statementLineId: line.id, journalLineId })
      onMatched(entryNumber)
      onClose()
    } catch {
      setPending(null)
    }
  }

  return (
    <ReportDialog
      open
      onOpenChange={(o) => !o && onClose()}
      title="Match to the books"
      description={
        <>
          {line.description} · {formatDate(line.date)} · <Money value={line.amount} signed strong className="text-foreground" />
        </>
      }
      className="max-w-xl"
      bodyClassName=""
      footer={
        <Button variant="outline" onClick={onClose}>
          Close
        </Button>
      }
    >
      {candidates.length === 0 ? (
        <ReportNotice title="No journal line fits">
          Nothing posted to this account for exactly this amount within {MANUAL_WINDOW} days is still unmatched. Post the line as a new entry instead.
        </ReportNotice>
      ) : (
        <>
          <p className="border-b border-border px-5 py-3 text-sm text-muted-foreground">
            Posted lines on this account for the same amount within {MANUAL_WINDOW} days, closest first.
          </p>
          <ul className="divide-y divide-border">
            {candidates.map((c) => (
              <li key={c.line.id} className="flex flex-col gap-3 px-5 py-3 sm:flex-row sm:items-center sm:justify-between">
                <div className="min-w-0">
                  <p className="text-sm font-semibold">
                    <Link to="/ledger/entries/$id" params={{ id: c.entry.id }} className="hover:underline">
                      {c.entry.number}
                    </Link>{' '}
                    <span className="font-normal text-muted-foreground">· {formatDate(c.entry.date)}</span>
                  </p>
                  <p className="text-sm break-words">{c.line.memo ?? c.entry.memo}</p>
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {c.dayGap === 0 ? 'Same day' : `${c.dayGap} ${c.dayGap === 1 ? 'day' : 'days'} apart`}
                    {c.referenceMatch && <span className="font-semibold text-foreground"> · Reference matches</span>}
                  </p>
                </div>
                <Button onClick={() => match(c.line.id, c.entry.number)} disabled={command.isPending} className="shrink-0">
                  {pending === c.line.id ? 'Matching…' : 'Match'}
                  <span className="sr-only"> to {c.entry.number}</span>
                </Button>
              </li>
            ))}
          </ul>
        </>
      )}
      <IssueSummary error={command.error} title="Not matched" className="mx-5 mb-4" />
    </ReportDialog>
  )
}

// ---------------------------------------------------------------------------
// Post as a new entry
// ---------------------------------------------------------------------------

function PostDialog({
  line,
  state,
  onClose,
  onPosted,
}: {
  line: StatementLine
  state: LedgerState
  onClose: () => void
  onPosted: () => void
}) {
  const command = useLedgerCommand()
  const [account, setAccount] = useState('')
  const [fund, setFund] = useState(state.funds.some((f) => f.id === UNRESTRICTED_FUND_ID) ? UNRESTRICTED_FUND_ID : (state.funds[0]?.id ?? ''))
  const [grant, setGrant] = useState('')
  const [memo, setMemo] = useState(line.description)
  const [tried, setTried] = useState(false)
  const suspense = state.accounts.find((a) => a.id === state.settings.system_accounts.suspense)

  const submit = async () => {
    setTried(true)
    if (!fund) return
    try {
      await command.mutateAsync({
        type: 'statement.post',
        statementLineId: line.id,
        counter: { account_id: account || undefined, fund_id: fund, grant_id: grant || null, memo: memo.trim() || undefined },
      })
      onPosted()
      onClose()
    } catch {
      // Shown through command.error.
    }
  }

  return (
    <ReportDialog
      open
      onOpenChange={(o) => !o && onClose()}
      title="Post as a new entry"
      description={
        <>
          {line.description} · {formatDate(line.date)} · <Money value={line.amount} signed strong className="text-foreground" />
        </>
      }
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={command.isPending}>
            {command.isPending ? 'Posting…' : 'Post and match'}
          </Button>
        </>
      }
    >
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
      >
        <p className="text-sm text-muted-foreground">
          {line.amount < 0 ? 'Money left' : 'Money came into'} {state.bankAccounts.find((b) => b.id === line.bank_account_id)?.name ?? 'the account'}. Choose what it was for; the entry is posted on {formatDate(line.date)} and matched to this line.
        </p>
        <Field
          id="post-account"
          label={line.amount < 0 ? 'Charge it to' : 'Record it as'}
          hint={`Leave empty to hold it in ${suspense ? `${suspense.code} ${suspense.name}` : 'suspense'} until it is explained.`}
        >
          {(p) => (
            <AccountSelect
              {...p}
              state={state}
              value={account}
              onValueChange={setAccount}
              types={line.amount < 0 ? ['expense', 'asset', 'liability'] : ['income', 'asset', 'liability']}
            />
          )}
        </Field>
        <Field id="post-fund" label="Fund" required error={tried && !fund ? 'Choose the fund this money belongs to.' : undefined}>
          {(p) => (
            <FundSelect
              {...p}
              state={state}
              value={fund}
              onValueChange={(v) => {
                setFund(v)
                setGrant('')
              }}
            />
          )}
        </Field>
        <Field id="post-grant" label="Grant">
          {(p) => <GrantSelect {...p} state={state} fundId={fund} value={grant} onValueChange={setGrant} />}
        </Field>
        <Field id="post-memo" label="Memo" hint="Shown on the journal entry.">
          {(p) => <Input {...p} value={memo} onChange={(e) => setMemo(e.target.value)} className="h-10" />}
        </Field>
        <IssueSummary error={command.error} title="Not posted" />
      </form>
    </ReportDialog>
  )
}
