// Pure building blocks: money (money.ts), CSV and statement dates (csv.ts),
// and a new organisation's books (setup.ts, coa-template.ts, engine.ts).
import { canonical } from './canonical'
import type { Json } from './canonical'
import { frozenEnv, writeFixture } from './fixture'
import {
  SYSTEM_ACTOR,
  UNRESTRICTED_FUND_ID,
  accountId,
  actorFromPermissions,
  assertMinor,
  convert,
  createOrgLedger,
  defaultSettings,
  emptyLedger,
  fromMajor,
  ngoChartOfAccounts,
  normalBalanceOf,
  parseCsv,
  parseCsvRecords,
  parseMoney,
  parseStatementDate,
  sum,
  toCsv,
  toMajor,
} from './flowtrace'
import { src } from './sources'
import { errorOf } from './trace'

/** Calls fn on each argument list; records the result or the error. */
function cases(fn: (...a: Array<never>) => unknown, argsList: Array<Array<unknown>>): Array<Json> {
  return argsList.map((args) => {
    try {
      const r = fn(...(args as Array<never>))
      return { args: canonical(args), result: r === undefined ? null : canonical(r) }
    } catch (e) {
      return { args: canonical(args), error: errorOf(e) }
    }
  })
}

export function exportPrimitives(outDir: string) {
  writeFixture(outDir, 'money', {
    description:
      'Money helpers: parseMoney (decimal strings to minor units), convert (half-to-even with a 1e-9 tolerance), ' +
      'fromMajor, toMajor, sum and assertMinor. Inputs from engine.test.ts plus edges; errors are RangeErrors.',
    sources: [src('money.ts'), src('engine.test.ts')],
    covers: ['parseMoney', 'convert', 'fromMajor', 'toMajor', 'sum', 'assertMinor'],
    env: null,
    parseMoney: cases(parseMoney, [
      '1,250.5', '0.1', '(300)', '12.345', 'abc', '', '0', '-0.05', '-12', ' 1 000.00 ', '1e3', '00.10', '.5', '5.',
      '(1,200.25)', '90071992547409.91', '90071992547409.92', '99999999999999999',
    ].map((x) => [x])),
    convert: cases(convert, [
      [1, 0.5], [3, 0.5], [100, 129.5], [-1, 0.5], [-3, 0.5], [5, 0.1], [12_345, 129.37], [6_789, 129.37], [19_134, 129.37],
      [10_000, 128], [10_000, 130], [1, 1.5], [2, 1.5], [-10_000, 1], [0, 129.5], [100, 0], [100, -1], [100, Infinity], [100, NaN],
    ]),
    fromMajor: cases(fromMajor, [[34_000_000], [0.1], [12.345], [1.005], [-2.5], [0], [90_071_992_547_409.92]]),
    toMajor: cases(toMajor, [[125_050], [-1], [0], [1]]),
    sum: cases(sum, [[[]], [[1, 2, 3]], [[-5, 5]], [[100_000, -250]]]),
    assertMinor: cases(assertMinor, [[1], [-1], [0], [1.5], [Number.MAX_SAFE_INTEGER + 1], [NaN], [10.5, 'Line 1 amount']]),
  })

  writeFixture(outDir, 'csv', {
    description:
      'The CSV reader and writer and statement dates: parseCsv, parseCsvRecords, toCsv and parseStatementDate ' +
      '(day-first, falling back to month-first only when day-first is impossible). Inputs from reconcile.test.ts plus edges.',
    sources: [src('csv.ts'), src('reconcile.test.ts')],
    covers: ['parseCsv', 'parseCsvRecords', 'toCsv', 'parseStatementDate'],
    env: null,
    parseCsv: cases(parseCsv, [
      [toCsv([['a', 'b,c', 'say "hi"', 'line\nbreak']])],
      ['a,b\r\n1,2\r\n'],
      ['﻿x,y\n"q,1","say ""hi"""\n'],
      ['a,b\n\n , \n3,4'],
      ['a,"unterminated\nquote'],
      ['a,b,\n1,,3'],
      [''],
    ]),
    parseCsvRecords: cases(parseCsvRecords, [
      ['*Narration, Date ,AMOUNT\nx, 2026-01-01 ,5\ny'],
      [''],
    ]),
    toCsv: cases(toCsv, [
      [[['a', 'b,c', 'say "hi"', 'line\nbreak']]],
      [[[1, 2.5, -3, null, undefined, true], ['', 'x\ry']]],
      [[]],
    ]),
    parseStatementDate: cases(parseStatementDate, [
      '2026-01-15 14:03:22', '15/01/2026', '15 Jan 2026', '31/02/2026', '01/15/2026', '15-01-2026', '15.01.2026',
      '15-Jan-2026', '15 January 2026', '2026-1-5', '5/1/2026', '29/02/2028', '29/02/2026', '13/13/2026', '00/01/2026',
      ' 2026-02-03T10:12 ', '15 Foo 2026', '', 'garbage',
    ].map((x) => [x])),
  })

  const { env, doc } = frozenEnv(
    '2026-03-01T00:00:00.000Z',
    (n) => `id-${n}`,
    'id- then n in decimal, n = 1, 2, 3, ... (id-1); createOrgLedger itself draws no ids',
    'rust/tools/src/primitives.ts, mirroring testEnv() in flowtrace/src/lib/ledger/engine.test.ts:16-19',
  )
  const org = createOrgLedger('org', 'KES', 2026, env)
  const types = ['asset', 'liability', 'equity', 'income', 'expense'] as const
  writeFixture(outDir, 'setup', {
    description:
      'A new organisation’s books: createOrgLedger(\'org\', \'KES\', 2026) (the base every test scenario starts ' +
      'from), the NGO chart of accounts, default settings, emptyLedger, accountId, normalBalanceOf, ' +
      'actorFromPermissions, SYSTEM_ACTOR and UNRESTRICTED_FUND_ID.',
    sources: [src('setup.ts'), src('coa-template.ts'), src('engine.ts', 'emptyLedger'), src('engine.ts', 'normalBalanceOf')],
    covers: [
      'createOrgLedger', 'ngoChartOfAccounts', 'defaultSettings', 'emptyLedger', 'accountId', 'normalBalanceOf',
      'actorFromPermissions', 'SYSTEM_ACTOR', 'UNRESTRICTED_FUND_ID', 'Ledger.ensureFiscalYear',
    ],
    env: doc,
    createOrgLedger: { args: ['org', 'KES', 2026], state: org.state },
    ngoChartOfAccounts: ngoChartOfAccounts(),
    defaultSettings: cases(defaultSettings, [['org'], ['org', 'UGX']]),
    emptyLedger: cases(emptyLedger, [[defaultSettings('org')]]),
    accountId: cases(accountId, [['1110'], ['7000'], ['']]),
    normalBalanceOf: cases(normalBalanceOf, types.map((t) => [t])),
    actorFromPermissions: cases(actorFromPermissions, [
      [{ id: 'u1', name: 'Finance Lead' }, { can_edit_transactions: true, can_manage_groups: true }],
      [{ id: 'u2', name: 'Clerk' }, { can_edit_transactions: true, can_manage_groups: false, can_manage_members: true }],
      [{ id: 'u3', name: 'Viewer' }, { can_edit_transactions: false, can_manage_groups: false }],
    ]),
    SYSTEM_ACTOR,
    UNRESTRICTED_FUND_ID,
  })
}
