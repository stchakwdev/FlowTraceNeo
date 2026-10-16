// Reconciliation and imports: autoMatch and candidatesFor over the demo bank
// statements (reconcile.ts), statement CSV parsing for every format, and the
// QuickBooks and Xero journal importers (importers.ts).
import { canonical } from './canonical'
import type { Json } from './canonical'
import { frozenEnv, writeFixture } from './fixture'
import {
  Ledger,
  SYSTEM_ACTOR,
  UNRESTRICTED_FUND_ID,
  accountId,
  autoMatch,
  candidatesFor,
  createOrgLedger,
  detectFormat,
  detectJournalSource,
  parseJournalCsv,
  parseStatement,
} from './flowtrace'
import type { Actor, LedgerState } from './flowtrace'
import { DEMO_ACTOR } from './demo'
import { src } from './sources'
import { Scenario } from './trace'

const A = accountId
const U = UNRESTRICTED_FUND_ID

const testEnv = () =>
  frozenEnv(
    '2026-03-01T00:00:00.000Z',
    (n) => `id-${n}`,
    'id- then n in decimal, n = 1, 2, 3, ... (id-1)',
    'mirrors testEnv() in flowtrace/src/lib/ledger/engine.test.ts:16-19; reconcile.test.ts itself uses defaultEnv (createOrgLedger without an env)',
  )

/** Runs autoMatch on a copy of `state`, recording candidates before and the statement lines after. */
function autoMatchCase(label: string, stateRef: Json, state: LedgerState, bankAccountId: string, actor: Actor, windowDays?: number) {
  const ledger = new Ledger(structuredClone(state), testEnv().env)
  const open = ledger.state.statementLines.filter((s) => s.bank_account_id === bankAccountId && !s.matched_line_id)
  const candidates: Record<string, unknown> = {}
  for (const s of open) candidates[s.id] = candidatesFor(ledger, s, windowDays)
  const result = windowDays === undefined ? autoMatch(ledger, actor, bankAccountId) : autoMatch(ledger, actor, bankAccountId, windowDays)
  return {
    label,
    state: stateRef,
    args: canonical([bankAccountId, windowDays]),
    actor: canonical(actor),
    candidates_before: canonical(candidates),
    result: canonical(result),
    statement_lines_after: canonical(
      ledger.state.statementLines
        .filter((s) => s.bank_account_id === bankAccountId)
        .map((s) => ({ id: s.id, matched_line_id: s.matched_line_id, match_kind: s.match_kind })),
    ),
  }
}

export function exportReconcile(outDir: string, demoState: LedgerState, preAutoMatch: LedgerState) {
  // The auto-match test in reconcile.test.ts:51-78, as a scenario.
  const test = new Scenario(createOrgLedger('org', 'KES', 2026, testEnv().env))
  const admin: Actor = { id: 'u1', name: 'Admin', can_post: true, can_approve: true, can_close: true }
  test.ok('addBankAccount', admin, { id: 'b', name: 'Main', kind: 'bank', provider: 'X', account_id: A('1110'), currency: 'KES', number_masked: '1' })
  const pay = (amount: number, date: string, memo: string) =>
    test.ok('post', admin, { date, memo, lines: [{ account_id: A('7100'), debit: amount, fund_id: U }, { account_id: A('1110'), credit: amount, fund_id: U }] })
  pay(10_000, '2026-01-10', 'Stationery · INV-100')
  pay(10_000, '2026-01-11', 'Stationery · INV-101')
  pay(20_000, '2026-01-12', 'Fuel')
  pay(30_000, '2026-01-12', 'Rent A')
  pay(30_000, '2026-01-12', 'Rent B')
  test.ok('importStatement', admin, 'b', [
    { date: '2026-01-11', description: 'INV-101', reference: 'INV-101', amount: -10_000 },
    { date: '2026-01-12', description: 'X', reference: null, amount: -10_000 },
    { date: '2026-01-14', description: 'FUEL', reference: null, amount: -20_000 },
    { date: '2026-01-13', description: 'RENT', reference: null, amount: -30_000 },
    { date: '2026-01-30', description: 'LATE', reference: null, amount: -20_000 },
  ])
  const testState = structuredClone(test.ledger.state)

  const cases = [
    autoMatchCase('demo bank-main, as the demo runs it (after the January import)', 'inline: state_before_demo_auto_match', preAutoMatch, 'bank-main', SYSTEM_ACTOR),
    autoMatchCase('demo bank-main, window 0 days', 'inline: state_before_demo_auto_match', preAutoMatch, 'bank-main', SYSTEM_ACTOR, 0),
    autoMatchCase('demo M-Pesa till statement (left unmatched by the demo)', 'demo-books.json#/state', demoState, 'mpesa-till', DEMO_ACTOR as Actor),
    autoMatchCase('demo M-Pesa till, window 10 days', 'demo-books.json#/state', demoState, 'mpesa-till', DEMO_ACTOR as Actor, 10),
    autoMatchCase('demo bank-main after the demo (what is left)', 'demo-books.json#/state', demoState, 'bank-main', DEMO_ACTOR as Actor),
    autoMatchCase('reconcile.test.ts auto-match scenario', 'inline: test_scenario', testState, 'b', admin),
  ]

  writeFixture(outDir, 'reconcile-auto-match', {
    description:
      'autoMatch and candidatesFor: the demo’s own run on bank-main (state just before it, inline), the M-Pesa ' +
      'till statement the demo leaves unmatched, what is left on bank-main, and the reconcile.test.ts scenario. ' +
      'Each case: candidatesFor every open line (default window unless args gives one), the AutoMatchResult and ' +
      'every statement line of the account afterwards.',
    sources: [src('reconcile.ts', 'autoMatch'), src('reconcile.ts', 'candidatesFor'), src('reconcile.test.ts'), 'flowtrace/src/lib/demo/ledger.ts#buildDemoLedger'],
    covers: ['autoMatch', 'candidatesFor', 'Ledger.matchStatementLine', 'Ledger.importStatement', 'Ledger.addBankAccount'],
    env: testEnv().doc,
    state_before_demo_auto_match: preAutoMatch,
    test_scenario: {
      setup: 'createOrgLedger(\'org\', \'KES\', 2026, env), then the steps',
      steps: test.steps,
    },
    cases,
  })

  exportStatementParsing(outDir)
  exportJournalImports(outDir)
}

const STATEMENTS: Array<{ name: string; input_source: string; text: string }> = [
  {
    name: 'M-Pesa statement with a failed row',
    input_source: 'flowtrace/src/lib/ledger/reconcile.test.ts:28-33',
    text: [
      'Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn,Balance',
      'RB17Q0XT3M,2026-02-01 10:22:01,Customer Transfer Fuliza charge,Completed,,410.00,59590.00',
      'RB34NNA81C,2026-02-03 08:00:00,Funds received from Equity Bank,Completed,"15,000.00",,74590.00',
      'RB99FAILED,2026-02-03 09:00:00,Pay Bill to KPLC,Failed,,500.00,74590.00',
    ].join('\n'),
  },
  {
    name: 'Bank CSV with debit and credit columns',
    input_source: 'flowtrace/src/lib/ledger/reconcile.test.ts:44',
    text: 'Date,Narration,Reference,Debit,Credit\n15/01/2026,SUPPLIES,INV-9,"1,200.50",\n16/01/2026,GRANT,HHF-1,,5000',
  },
  {
    name: 'M-Pesa placeholder from the statement import screen',
    input_source: 'flowtrace/src/routes/_authenticated/reconcile/$bankId.tsx:624',
    text: 'Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn\nRB34NNA81C,2026-02-03 10:12,Funds received,Completed,15000,',
  },
  {
    name: 'MTN MoMo statement, signed amount with a direction column',
    input_source: 'synthetic: written for this fixture (no MTN sample in flowtrace/)',
    text: [
      'Financial Transaction Id,External Transaction Id,Date,Description,Status,Amount,Direction',
      'FT-1001,EXT-77,15/01/2026,Payment to supplier,Successful,"25,000",Dr',
      'FT-1002,,16-Jan-2026,Deposit,Successful,40000.50,Cr',
      'FT-1003,,17/01/2026,Reversal,Reversed,100,Dr',
    ].join('\n'),
  },
  {
    name: 'Airtel Money statement, money in and money out columns',
    input_source: 'synthetic: written for this fixture (no Airtel sample in flowtrace/)',
    text: [
      'Transaction ID,Transaction Date,Transaction Type,Service Type,Money In,Money Out,Transaction Status',
      'AT001,2026-01-20 08:15:00,Cash In,Agent,"5,000",,Success',
      'AT002,20 Jan 2026,Merchant Payment,Pay Bill,,(1200.25),Success',
      'AT003,2026-01-21,Bundle,Airtime,,,Success',
    ].join('\n'),
  },
  {
    name: 'Bank CSV with BOM, CRLF, a signed amount, a type column and bad rows',
    input_source: 'synthetic: written for this fixture',
    text: '﻿Value Date,Particulars,Ref,Amount,Type\r\n01/02/2026,CHEQUE 1001,CHQ1001,500.00,DR\r\n02/02/2026,DEPOSIT,,(75.5),\r\nnot a date,BROKEN,,10,\r\n03/02/2026,ZERO,,0.00,\r\n04/02/2026,,,12.345,\r\n',
  },
]

function exportStatementParsing(outDir: string) {
  const headerSets = [
    ['receipt no.', 'completion time', 'details'],
    ['receipt no'],
    ['completion time', 'paid in', 'withdrawn'],
    ['completion time', 'withdrawn'],
    ['external transaction id', 'amount'],
    ['financial transaction id'],
    ['transaction id', 'transaction type'],
    ['transaction id', 'service type'],
    ['transaction id'],
    ['date', 'narration', 'debit', 'credit'],
    [],
  ]
  writeFixture(outDir, 'reconcile-parse-statement', {
    description:
      'parseStatement over a statement CSV of every format (M-Pesa, MTN, Airtel, bank), from the tests and the ' +
      'import screen where a sample exists and synthetic otherwise (input_source says which); detectFormat over ' +
      'header sets for each branch.',
    sources: [src('reconcile.ts', 'parseStatement'), src('reconcile.ts', 'detectFormat')],
    covers: ['parseStatement', 'detectFormat'],
    env: null,
    cases: STATEMENTS.map((s) => ({ ...s, result: parseStatement(s.text) })),
    detect_format: headerSets.map((h) => ({ args: [h], result: detectFormat(h) })),
  })
}

function exportJournalImports(outDir: string) {
  const { env } = testEnv()
  const ledger = createOrgLedger('org', 'KES', 2026, env)
  const samples = [
    {
      name: 'QuickBooks journal export',
      input_source: 'flowtrace/src/lib/ledger/reconcile.test.ts:83-88',
      text: [
        'Date,Transaction Type,Num,Name,Memo/Description,Account,Debit,Credit',
        '01/15/2026,Journal Entry,12,,Office rent January,7500 Rent,"45,000.00",',
        ',,,,,Bank: Main operating account,,"45,000.00"',
        '20/01/2026,Journal Entry,13,,Unknown,9999 Mystery,100,',
      ].join('\n'),
    },
    {
      name: 'Xero manual journal import',
      input_source: 'flowtrace/src/lib/ledger/reconcile.test.ts:98',
      text: '*Narration,*Date,Description,*AccountCode,*TaxRate,*Amount\nAudit fee,31/01/2026,Audit,7600,Tax Exempt,25000\nAudit fee,31/01/2026,Audit,2200,Tax Exempt,-25000',
    },
    {
      name: 'QuickBooks export with an unreadable amount and a journal with no date',
      input_source: 'synthetic: written for this fixture',
      text: [
        'Date,Num,Memo,Account,Debit,Credit',
        ',7,Accrual,1110,12.345,',
        ',,,Rent,,100',
        '02/02/2026,8,Fuel,7200,"1,000.00",',
        ',,,1110 Bank: Main operating account,,"1,000.00"',
      ].join('\n'),
    },
    {
      name: 'Not a journal file',
      input_source: 'synthetic: written for this fixture',
      text: 'Receipt No.,Completion Time,Details\nX,2026-01-01,Y',
    },
  ]
  const cases = samples.map((s) => {
    const result = parseJournalCsv(s.text, ledger.state, U)
    return { ...s, args: { fundId: U }, result, validate: result.entries.map((e) => ledger.validateEntry(e)) }
  })
  writeFixture(outDir, 'importers-journal', {
    description:
      'parseJournalCsv over QuickBooks and Xero samples (from reconcile.test.ts) and synthetic edge cases, into ' +
      'the unrestricted fund, against the default chart (createOrgLedger(\'org\', \'KES\', 2026); only ' +
      'state.accounts is read), with validateEntry on each parsed entry; detectJournalSource over header sets.',
    sources: [src('importers.ts', 'parseJournalCsv'), src('importers.ts', 'detectJournalSource'), src('reconcile.test.ts')],
    covers: ['parseJournalCsv', 'detectJournalSource', 'Ledger.validateEntry'],
    env: null,
    state: "createOrgLedger('org', 'KES', 2026, env).state, as in setup.json",
    cases,
    detect_journal_source: [
      ['accountcode', 'amount', 'narration'],
      ['account', 'debit'],
      ['account', 'credit'],
      ['account'],
      ['narration', 'amount'],
    ].map((h) => ({ args: [h], result: detectJournalSource(h) })),
  })
}
