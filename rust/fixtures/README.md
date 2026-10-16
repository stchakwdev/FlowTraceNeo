# Parity fixtures from the TS ledger engine

These JSON files are the mechanical ground truth for the Rust `ledger-core`. Each one was written
by running the TypeScript ledger engine in `flowtrace/src/lib/ledger/` (and the demo books in
`flowtrace/src/lib/demo/ledger.ts`) unchanged, under a frozen environment, through the exporter in
`rust/tools/`. Nothing here is written by hand. If a Rust result differs from a fixture, the TS
engine on `main` is right (ground truth order: TS engine and its tests first).

## Regenerate and check

From the repository root:

```
cd rust/tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run export
```

`bun run check` (same folder) runs the export twice in separate processes into `rust/out/`,
requires the two runs and this folder to be byte-identical, then runs the coverage assertions
(`coverage.ts`): every command type has an accepted case, every export of `reports.ts` has a
`report-*.json`, every export of the ledger index and every public `Ledger` method is listed in
some fixture's `covers`, every rejection asserted in the TS tests is present case for case,
replaying the recorded deltas rebuilds `demo-books.json#/state` and the commands' `final_state`,
and this table has one row per file. Stale fixtures fail the check.

The exporter runs in a child process with `TZ=UTC`, `LC_ALL=C.UTF-8`, no `VITE_*` variables and
`NODE_PATH` set to `rust/tools/node_modules`: the demo module's import chain
(`demo/dataset.ts` -> `supabase.ts`) needs `@supabase/supabase-js`, pinned there at 2.89.0, the
version in `flowtrace/bun.lock`. Auto-install is disabled. The wall clock, `Math.random` and
`crypto.randomUUID` throw while it runs (`rust/tools/src/guards.ts`).

## How ids and time are injected

The engine takes ids and time only from a `LedgerEnv` (`flowtrace/src/lib/ledger/engine.ts:41-44`):
`{ now: () => string, newId: () => string }`, passed to `new Ledger(state, env)` or
`createOrgLedger(org, currency, year, env)`. `defaultEnv` (`engine.ts:46-49`) uses the wall clock
and `crypto.randomUUID`; no fixture uses it. Three frozen envs appear:

| Env | `now` | `newId` | Defined at | Used by |
|---|---|---|---|---|
| demo | `2026-02-04T09:00:00.000Z` | `demo-` + n in base 36, padded to 4 (`demo-0001`, ..., `demo-000a`) | `flowtrace/src/lib/demo/ledger.ts:118-124` (`seedEnv`) | demo-books, demo-backfill |
| fixture | `2026-02-04T09:00:00.000Z` | `fx-` + n in decimal, padded to 4 (`fx-0001`) | `rust/tools/src/commands.ts#commandsEnv` | commands-accepted |
| test | `2026-03-01T00:00:00.000Z` | `id-` + n in decimal (`id-1`) | `engine.test.ts:16-19`, `review-regressions.test.ts:16-19` | rejected-cases, reconcile-auto-match, setup |

Every counter starts at 1 for each new ledger. Each audit event, journal line, statement line,
import batch, coded line and record created without an id draws one id, in call order. No
timestamp other than these two values appears in any fixture (the exporter checks every `at`,
`created_at`, `posted_at` and `now`).

## Files

Naming: `<area>-<subject>.json`, kebab-case; one file per export of `reports.ts`
(`report-<function>.json`). Every file has `fixture` (its name), `description`, `sources` (TS file
and function), `covers` (index exports and `Ledger.<method>` names it exercises) and `env`.

| File | What it holds | TS source and function | Env |
|---|---|---|---|
| `demo-books.json` | The demo books after `buildDemoLedger()`: full `LedgerState` (chart, funds, grants, dimensions, periods, 2025 opening and close, backfilled entries, budget, bills, claims, bank accounts, statements, audit, counters) | `demo/ledger.ts#buildDemoLedger` | demo |
| `demo-backfill.json` | All 108 top-level engine calls of `buildDemoLedger()` that changed the books, in order: method, args, return value, state delta, and `via` (`direct`, `applyCommand` 13, `autoMatch` 6, `createOrgLedger` 1); `initial_state` before the first | `demo/ledger.ts#buildDemoLedger`, `setup.ts#createOrgLedger`, `commands.ts#applyCommand`, `reconcile.ts#autoMatch` | demo |
| `commands-accepted.json` | All 32 `LedgerCommand` types accepted at least once (37 steps) over the demo books: raw command, sanitised command, `CommandResult`, state delta; `final_state` | `commands.ts#sanitizeCommand`, `commands.ts#applyCommand` | fixture |
| `rejected-cases.json` | 19 scenarios, 37 rejections: every rejection asserted in `engine.test.ts` and `review-regressions.test.ts` (32, one scenario per test, the test's calls in order) plus 5 sanitiser rejections; exact `code`, `message`, `issues`, and `state_unchanged` | `engine.test.ts`, `review-regressions.test.ts`, `errors.ts#LedgerError`, `commands.ts#sanitizeCommand` | test |
| `engine-queries.json` | 64 calls of the read methods over the demo books: lookups, `periodFor`, `inputOf`, `grantSpend`, `outstanding`, `claim`, and `validateEntry` over 22 inputs (every issue, with `line` and `overridable`) | `engine.ts#Ledger` | none (reads draw no ids) |
| `report-activities.json` | `activities(state, from, to, basis)`: every active period, active fiscal year and year to the demo as-of date, both bases, both books | `reports.ts#activities` | none |
| `report-balances.json` | `balances(state, filter)`: by date, range, basis, fund, grant, account group, dimension, closing excluded | `reports.ts#balances` | none |
| `report-budget-vs-actual.json` | `budgetVsActual(state, opts)`: all, cash, by budget, grant, dimension and as-of date | `reports.ts#budgetVsActual` | none |
| `report-cash-flow.json` | `cashFlow(state, from, to)` over the same ranges as activities | `reports.ts#cashFlow` | none |
| `report-day-before.json` | `dayBefore(date)` for every period start and leap-year edges | `reports.ts#dayBefore` | none |
| `report-descendants.json` | `descendants(state, accountId)` for every header account, a leaf and an unknown id | `reports.ts#descendants` | none |
| `report-financial-position.json` | `financialPosition(state, asOf, basis)` at every as-of date, both bases | `reports.ts#financialPosition` | none |
| `report-functional-expenses.json` | `functionalExpenses(state, from, to, basis)` over the activities ranges | `reports.ts#functionalExpenses` | none |
| `report-general-ledger.json` | `generalLedger(state, filter)`: all, one month (with opening balances), one month on the cash group, cash basis year to date | `reports.ts#generalLedger` | none |
| `report-grant-summary.json` | `grantSummary(state, grantId, asOf)` for every grant at every as-of date | `reports.ts#grantSummary` | none |
| `report-ledger-signals.json` | `ledgerSignals(state, asOf?)` with and without an as-of date | `reports.ts#ledgerSignals` | none |
| `report-natural.json` | `natural(account, signed)` for both normal balances and signs | `reports.ts#natural` | none |
| `report-posted-lines.json` | `postedLines(state, filter)`: all, cash basis (re-coded settlements), range, group on cash basis, grant, dimension | `reports.ts#postedLines` | none |
| `report-trial-balance.json` | `trialBalance(state, asOf?, basis)` at every as-of date and none, both bases | `reports.ts#trialBalance` | none |
| `report-warning-level.json` | `warningLevel(used)` across the 0.8 and 1 thresholds | `reports.ts#warningLevel` | none |
| `reconcile-auto-match.json` | `autoMatch` and `candidatesFor`: the demo's own bank-main run (state before it inline), window 0, the M-Pesa till statement, what is left on bank-main, and the `reconcile.test.ts` scenario | `reconcile.ts#autoMatch`, `reconcile.ts#candidatesFor` | test |
| `reconcile-parse-statement.json` | `parseStatement` for M-Pesa, MTN, Airtel and bank CSVs (MTN, Airtel and the edge-case bank file are synthetic, marked `input_source`); `detectFormat` per branch | `reconcile.ts#parseStatement`, `reconcile.ts#detectFormat` | none |
| `importers-journal.json` | `parseJournalCsv` for the QuickBooks and Xero samples and synthetic edge cases, with `validateEntry` on each parsed entry; `detectJournalSource` | `importers.ts#parseJournalCsv`, `importers.ts#detectJournalSource` | none |
| `bridge.json` | `bridgeTransaction` for all 16 demo transactions in backfill order, with the demo bridge context as data; `expenseAccountForCategory` | `bridge.ts#bridgeTransaction`, `bridge.ts#expenseAccountForCategory`, `demo/ledger.ts#demoBridgeContext` | none |
| `money.json` | `parseMoney`, `convert`, `fromMajor`, `toMajor`, `sum`, `assertMinor`, including errors | `money.ts` | none |
| `csv.json` | `parseCsv`, `parseCsvRecords`, `toCsv`, `parseStatementDate` | `csv.ts` | none |
| `setup.json` | `createOrgLedger('org', 'KES', 2026)` state (the base of every test scenario), `ngoChartOfAccounts`, `defaultSettings`, `emptyLedger`, `accountId`, `normalBalanceOf`, `actorFromPermissions`, `SYSTEM_ACTOR`, `UNRESTRICTED_FUND_ID` | `setup.ts`, `coa-template.ts`, `engine.ts#emptyLedger` | test |

## Shapes

- A pure case is `{ "args": [...], "result": ... }` or `{ "args": [...], "error": {...} }`.
  `args` are positional and leave out the books (`state`) where the function takes them first.
- A report case adds `"state"`: `demo-books` (`demo-books.json#/state`) or `commands-final`
  (`commands-accepted.json#/final_state`, which adds drafts, a reversal, a two-fund USD bill, a
  rejected claim, a locked period and a year-end close).
- An error is `{ "name": "LedgerError", "code", "message", "issues": [{ "code", "message", "line"?, "overridable"? }] }`
  (`message` is the issue messages joined by one space, `errors.ts:43-54`) or
  `{ "name": "RangeError", "message" }`.
- A scenario step is `{ "op": "method", "method", "args", "outcome", ... }` or
  `{ "op": "command", "actor", "command", "sanitized", "outcome", ... }`. `outcome: "ok"` carries
  `result` and `delta`; `outcome: "rejected"` carries `error` and `state_unchanged` (true for all
  37 here). A step's `args` include the actor first, as the method takes it.
- A delta lists, per changed collection, `added` (records in state order), `removed` (ids),
  `changed` (whole new records) and `order` (all ids) only when the order is not survivors then
  additions (for example `ensureFiscalYear` sorts periods); `settings` when changed; `counters`
  with the changed keys.
- `demo-books.json#/state` is reproduced by replaying `demo-backfill.json` steps from
  `initial_state` (each step's `args` through the named method, or by applying each delta).

## Formatting notes

Facts for pre-decision P4 (task C5). Values are as the TS engine produced them; nothing is
reformatted. Where JSON forces a representation, it is listed here.

- **Money is integer minor units everywhere** (`types.ts:5-6`): every amount, balance and total in
  the fixtures is an integer. Compare these exactly.
- **Floats in results**, IEEE-754 doubles written with ECMAScript shortest round-trip formatting
  (parse as `f64` and compare exactly): `budgetVsActual` `used` (`reports.ts:512`);
  `functionalExpenses` `programRatio` (`reports.ts:393`); `grantSummary` `used`, `monthsLeft`,
  `monthsToEnd` (`reports.ts:568-582`); `ledgerSignals` shares (`reports.ts:616-620`); `fx_rate` on
  lines, bills, claims and payments; `toMajor` results. Months are milliseconds divided by
  `1000 * 60 * 60 * 24 * 30.4375` (`reports.ts:552`); day gaps in `candidatesFor` are milliseconds
  divided by `86_400_000` (`reconcile.ts:108`), both from UTC dates.
- **Non-finite numbers** are written as the strings `"Infinity"`, `"-Infinity"`, `"NaN"` (JSON has
  no such numbers; `JSON.stringify` would write `null`). They occur in `budgetVsActual` `used` when
  a budget line is 0 and actual spend is positive (`reports.ts:512`; 6 cases, commands-final books,
  line `fx-0043`), and as inputs in `money.json` and `report-warning-level.json`.
- **Float-to-integer rounding** in the engine: `convert` (`money.ts:36-51`) floors `amount * rate`
  and rounds half to even within a `1e-9` tolerance; `Math.round` (ties toward +Infinity, so
  `Math.round(-2.5) = -2`, unlike Rust `f64::round`) in `fromMajor` (`money.ts:27`, so
  `fromMajor(1.005) = 100` and `fromMajor(12.345) = 1235` from double products), payment splits
  across funds (`engine.ts:1101`), cash-basis re-coding (`reports.ts:123`) and `burnPerMonth`
  (`reports.ts:570`).
- **Money in messages** uses `(minor / 100).toFixed(2)` with the currency code and no thousands
  separator: `Debits and credits differ by KES 0.01.` (`engine.ts:408`),
  `This takes Water grant over its award by KES 0.01.` and `... by KES 3500000.00.`
  (`engine.ts:438`); the payment message `Only <owed> is outstanding.` (`engine.ts:1080`) uses the same
  form but no fixture triggers it.
- **No locale formatting** anywhere in the ledger modules (no `toLocaleString`, no `Intl`).
- **Sorting with `localeCompare`** (locale-dependent collation): `byCode` (`reports.ts:207`), the
  general ledger order by date then entry number (`reports.ts:184-185`) and the demo's transaction
  order (`demo/ledger.ts:199`). The exporter asserts that for every set of books here (4-digit
  account codes, ISO dates, `JE-0001` numbers) it gives exactly the byte order, so byte comparison
  reproduces the fixtures.
- **Dates** are `YYYY-MM-DD` strings compared as strings; period, day-before and day-gap
  arithmetic is UTC (`engine.ts:91-93`, `reports.ts:201-205`, `reconcile.ts:107-108`). Timestamps
  are only the frozen `now` values above.
- **Audit detail of `account.update`** is `JSON.stringify` of the cleaned patch, keys in the order
  `name`, `is_active`, `expense_function` (`engine.ts:214-226`), for example
  `{"name":"Internet, data and airtime","expense_function":"program"}`. This is a string inside the
  fixture; its key order is not sorted.
- **Strings are UTF-8 and not escaped**: memos use `·` (U+00B7), messages `’` (U+2019), masked
  numbers `•` (U+2022). Statement descriptions are upper-cased with `toUpperCase`
  (`demo/ledger.ts:340`), not a locale-aware mapping.
- **JSON representation**: object keys are sorted by byte order; JS `Map` values (the
  `balances` result, `generalLedger` `opening` and `closing`) are objects keyed by account id;
  the `descendants` `Set` is an array in insertion order; `undefined` is omitted from objects and
  written as `null` in arrays (so an omitted optional argument appears as `null`); `-0` is written
  as `0`.
- **Rust reprint** (task C1): `crates/ledger-core/tests/state_roundtrip.rs` loads the six ledger
  states (`demo-books.json#/state`, `commands-accepted.json#/final_state`,
  `demo-backfill.json#/initial_state`, `reconcile-auto-match.json#/state_before_demo_auto_match`,
  `setup.json#/createOrgLedger/state` and `#/emptyLedger/0/result`) into `LedgerState`, and
  reprints each whole file byte-equal (keys sorted, two-space indent, trailing newline). This
  holds because `fx_rate` is written through `ledger_core::js_number` (a whole number without
  `.0`, non-finite values as the strings above) and every other state number is an integer.
  It would break on a float that ryu and ECMAScript print differently, such as `1e21` against
  `1e+21`; no state fixture holds one.

## Not covered here

- Property tests in `engine.test.ts` (fast-check, random inputs): ported as Rust property tests
  (task C7). Their one rejection assertion is excluded from the case-for-case check.
- `spacetime-codec.ts`, `spacetime-backend.ts` and `backend.ts` are not exported from the ledger
  index and belong to the SpacetimeDB client work.
- `defaultEnv` (wall clock, random ids) by design.
- Engine error paths that no TS test asserts are not catalogued, except the sanitiser's.
