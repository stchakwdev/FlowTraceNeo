import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { UploadSimple } from '@phosphor-icons/react'
import type { Ledger, LedgerState } from '@/lib/ledger'
import { UNRESTRICTED_FUND_ID, parseJournalCsv } from '@/lib/ledger'
import { useLedger, useLedgerCommand } from '@/lib/ledger-client'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Field } from '@/components/form'
import { ReportNotice, ReportSection } from '@/components/report'
import {
  BooksPage,
  BooksState,
  FundSelect,
  IssueSummary,
  Money,
  SubNav,
} from '@/components/ledger'
import { LEDGER_NAV } from '@/components/ledger/ledger-nav'

export const Route = createFileRoute('/_authenticated/ledger/import')({
  component: ImportPage,
})

const SOURCE_NAME = { quickbooks: 'QuickBooks journal export', xero: 'Xero manual journal' }

function ImportPage() {
  const { state, ledger, isLoading, error, refetch } = useLedger()
  return (
    <BooksPage
      title="Import journals"
      lede="Bring entries across from QuickBooks or Xero. Each journal in the file becomes a draft here; nothing reaches the books until someone reviews and posts it."
      nav={<SubNav label="Ledger" items={LEDGER_NAV} />}
    >
      {!state || !ledger ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="ledger" />
      ) : (
        <ImportForm state={state} ledger={ledger} />
      )}
    </BooksPage>
  )
}

function ImportForm({ state, ledger }: { state: LedgerState; ledger: Ledger }) {
  const command = useLedgerCommand()
  const [text, setText] = useState('')
  const [fileName, setFileName] = useState<string | null>(null)
  const [fileError, setFileError] = useState<string>()
  const [fund, setFund] = useState(
    state.funds.some((f) => f.id === UNRESTRICTED_FUND_ID) ? UNRESTRICTED_FUND_ID : (state.funds[0]?.id ?? ''),
  )
  const [failure, setFailure] = useState<unknown>(null)
  const [created, setCreated] = useState<number | null>(null)

  const parsed = useMemo(() => {
    if (!text.trim() || !fund) return null
    const result = parseJournalCsv(text, state, fund)
    const entries = result.entries
      .filter((e) => e.lines.length > 0)
      .map((input) => ({
        input,
        issues: ledger.validateEntry(input),
        total: input.lines.reduce((t, l) => t + (l.debit ?? 0), 0),
      }))
    return { ...result, entries }
  }, [text, fund, state, ledger])

  const readFile = async (file: File | undefined) => {
    setFileError(undefined)
    setCreated(null)
    if (!file) return
    if (file.size > 5 * 1024 * 1024) {
      setFileError('This file is larger than 5 MB. Export a shorter date range.')
      return
    }
    try {
      setText(await file.text())
      setFileName(file.name)
    } catch {
      setFileError('The file could not be read. Try exporting it again as CSV.')
    }
  }

  const create = async () => {
    if (!parsed?.entries.length) return
    setFailure(null)
    try {
      await command.mutateAsync({ type: 'entry.import', inputs: parsed.entries.map((e) => e.input) })
      setCreated(parsed.entries.length)
      setText('')
      setFileName(null)
    } catch (e) {
      setFailure(e)
    }
  }

  const clean = parsed?.entries.filter((e) => e.issues.length === 0).length ?? 0

  return (
    <>
      {created !== null && (
        <div role="status" className="border border-foreground p-4 text-sm">
          <p className="font-semibold">
            {created} draft {created === 1 ? 'entry was' : 'entries were'} created
          </p>
          <p className="mt-1 text-muted-foreground">
            Review each draft, fix anything the ledger flags, then post it.
          </p>
          <Link
            to="/ledger"
            search={{ status: 'draft', source: 'import' }}
            className={cn(buttonVariants({ variant: 'outline' }), 'mt-3')}
          >
            Review imported drafts
          </Link>
        </div>
      )}

      <ReportSection title="The file">
        <div className="grid gap-5 pt-5 md:grid-cols-[minmax(0,1fr)_18rem]">
          <div className="space-y-4">
            <div>
              <p className="mb-1.5 text-sm font-medium" id="import-file-label">
                Upload a CSV
              </p>
              <label
                className={cn(
                  'flex min-h-20 cursor-pointer flex-col items-center justify-center gap-1 border border-dashed border-input px-4 py-4 text-center text-sm transition-colors hover:bg-muted focus-within:ring-2 focus-within:ring-ring',
                  fileError && 'border-destructive',
                )}
              >
                <UploadSimple aria-hidden="true" className="size-5 text-muted-foreground" />
                <span className="font-semibold">{fileName ?? 'Choose a CSV file'}</span>
                <span className="text-xs text-muted-foreground">QuickBooks Journal report or Xero manual journal, up to 5 MB</span>
                <input
                  type="file"
                  accept=".csv,text/csv"
                  aria-labelledby="import-file-label"
                  aria-describedby={fileError ? 'import-file-error' : undefined}
                  aria-invalid={fileError ? true : undefined}
                  className="sr-only"
                  onChange={(e) => {
                    void readFile(e.target.files?.[0])
                    e.target.value = ''
                  }}
                />
              </label>
              {fileError && (
                <p id="import-file-error" className="mt-1.5 text-sm text-destructive">
                  {fileError}
                </p>
              )}
            </div>
            <Field
              id="import-text"
              label="Or paste the CSV"
              hint="QuickBooks: Date, Num, Account, Memo/Description, Debit, Credit. Xero: Date, Narration, AccountCode, Amount (debits positive, credits negative)."
            >
              {(props) => (
                <Textarea
                  {...props}
                  rows={8}
                  value={text}
                  onChange={(e) => {
                    setText(e.target.value)
                    setFileName(null)
                    setCreated(null)
                  }}
                  spellCheck={false}
                  className="font-mono text-xs"
                />
              )}
            </Field>
          </div>
          <div className="space-y-3">
            <Field id="import-fund" label="Fund for every line" required hint="Imported lines all go to this fund. Change the fund on individual drafts before posting if needed.">
              {(props) => <FundSelect {...props} state={state} value={fund} onValueChange={setFund} />}
            </Field>
          </div>
        </div>
      </ReportSection>

      {parsed && (
        <ReportSection
          title="Preview"
          meta={
            <span>
              {SOURCE_NAME[parsed.source]} · {parsed.entries.length} entries · {clean} ready to post
            </span>
          }
        >
          {parsed.problems.length > 0 && (
            <div role="alert" className="mt-4 border border-destructive p-4 text-sm">
              <p className="font-semibold text-destructive">Rows that could not be read</p>
              <ul className="mt-2 space-y-1">
                {parsed.problems.map((p, i) => (
                  <li key={i}>{p}</li>
                ))}
              </ul>
            </div>
          )}

          {parsed.unmappedAccounts.length > 0 && (
            <div className="mt-4 border border-border p-4 text-sm">
              <p className="font-semibold">
                {parsed.unmappedAccounts.length} {parsed.unmappedAccounts.length === 1 ? 'account is' : 'accounts are'} not in the chart
              </p>
              <p className="mt-1 text-muted-foreground">
                Lines on these accounts are left out, so the drafts they belong to will not balance. Add the accounts to the{' '}
                <Link to="/ledger/accounts" className="font-semibold text-foreground underline underline-offset-4">
                  chart of accounts
                </Link>{' '}
                with the same code or name, then load the file again.
              </p>
              <ul className="mt-2 flex flex-wrap gap-2">
                {parsed.unmappedAccounts.map((a) => (
                  <li key={a} className="border border-border px-2 py-0.5 font-mono text-xs">
                    {a}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {parsed.entries.length === 0 ? (
            <ReportNotice title="No entries found">
              Check that the file is a QuickBooks Journal report or a Xero manual journal export with its header row.
            </ReportNotice>
          ) : (
            <ol className="mt-4 divide-y divide-border border-y border-border">
              {parsed.entries.map(({ input, issues, total }, i) => (
                <li key={i} className="py-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-semibold">{input.memo}</p>
                      <p className="text-xs text-muted-foreground tabular-nums">
                        {/^\d{4}-\d{2}-\d{2}$/.test(input.date) ? formatDate(input.date) : input.date} · {input.lines.length}{' '}
                        {input.lines.length === 1 ? 'line' : 'lines'}
                      </p>
                    </div>
                    <Money value={total} strong className="shrink-0" />
                  </div>
                  {issues.length === 0 ? (
                    <p className="mt-1 text-xs font-semibold">Ready to post once reviewed</p>
                  ) : (
                    <ul className="mt-1 space-y-0.5 text-xs text-destructive">
                      {issues.map((x, n) => (
                        <li key={n}>
                          {x.message}
                          {x.overridable && <span className="text-muted-foreground"> (can be overridden with a reason)</span>}
                        </li>
                      ))}
                    </ul>
                  )}
                </li>
              ))}
            </ol>
          )}

          <div className="space-y-3 border-t-[3px] border-foreground pt-5 mt-6">
            <IssueSummary error={failure} title="No drafts created" />
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <p className="max-w-prose text-sm text-muted-foreground">
                Drafts are saved even when they have issues, so they can be fixed here instead of in the file.
              </p>
              <Button size="lg" onClick={create} disabled={command.isPending || parsed.entries.length === 0}>
                {command.isPending
                  ? 'Creating drafts…'
                  : `Create ${parsed.entries.length} ${parsed.entries.length === 1 ? 'draft' : 'drafts'}`}
              </Button>
            </div>
          </div>
        </ReportSection>
      )}
    </>
  )
}
