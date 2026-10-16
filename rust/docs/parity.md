# Parity table: TS ledger engine and ledger-core

Every export of the TS ledger modules (`flowtrace/src/lib/ledger/<file>.ts`, the engine of PR #8)
and every public method of its `Ledger` class, with the Rust counterpart, what proves it and the
known differences. 94 rows. Type-only exports (interfaces and type aliases) map to
`ledger_core::types` and the input structs of `ledger_core::engine` and are not listed.

The TS column is the output of grepping the files for `export function|const|class` and the
`Ledger` class body for methods that are not `private`; `crates/ledger-core/tests/parity_table.rs`
runs the same grep and fails when a row is missing or names nothing.

Reading the columns:

- Proved by: a fixture under `rust/fixtures/` and, in parentheses, the test under
  `rust/crates/ledger-core/tests/` that replays it and compares exactly; then other tests in that
  folder. `properties.rs` is the property suite (task C7), `review_regressions.rs` the named
  ports of `review-regressions.test.ts`, `engine_rules.rs` and `document_rules.rs` the rules no
  fixture rejects.
- Known differences: what the C threads recorded, all on inputs no fixture holds. The common one:
  inputs are typed, so a value of the wrong shape fails to deserialise where TS would store it.
  An empty cell means none recorded. Formatting: none (pre-decision P4 did not fire; see
  `rust/fixtures/README.md`, "Formatting notes").
- `buildDemoLedger` (`flowtrace/src/lib/demo/ledger.ts`) is outside these files; its counterpart
  is `ledger_core::demo::build_demo_ledger`, proved by `demo-books.json` (demo.rs) and
  `demo-backfill.json` (replay.rs).

| TS file | TS export | Rust counterpart | Proved by | Known differences |
|---|---|---|---|---|
| engine.ts:46 | `defaultEnv` | none in ledger-core; `ledger_core::LedgerEnv` with `env::fixed::{demo, fixture, test}`; the module injects `ModuleEnv` (`crates/ledger-module/src/command.rs`) | not ported by design (`fixtures/README.md`, "Not covered here") | no wall clock or random ids in the core (pre-decision P5) |
| engine.ts:51 | `emptyLedger` | `ledger_core::engine::empty_ledger` | `setup.json` (setup.rs) |  |
| engine.ts:95 | `normalBalanceOf` | `ledger_core::engine::normal_balance_of` | `setup.json` (setup.rs) |  |
| engine.ts:103 | `Ledger` | `ledger_core::Ledger::new(&mut state, &mut dyn LedgerEnv)`, `Ledger::state()` | `demo-backfill.json` (replay.rs); `commands-accepted.json` (replay.rs, commands.rs); properties.rs | borrows the books; reads are methods on `LedgerState`; no default env; a rejected write changes nothing and draws no id, where TS mutates and its callers discard the books (`engine/mod.rs`) |
| engine.ts:113 | `Ledger.account` | `ledger_core::LedgerState::account` | `engine-queries.json` (queries.rs) |  |
| engine.ts:116 | `Ledger.accountByCode` | `ledger_core::LedgerState::account_by_code` | `engine-queries.json` (queries.rs) |  |
| engine.ts:119 | `Ledger.fund` | `ledger_core::LedgerState::fund` | `engine-queries.json` (queries.rs) |  |
| engine.ts:122 | `Ledger.grant` | `ledger_core::LedgerState::grant` | `engine-queries.json` (queries.rs) |  |
| engine.ts:125 | `Ledger.entry` | `ledger_core::LedgerState::entry` | `engine-queries.json` (queries.rs) |  |
| engine.ts:128 | `Ledger.linesOf` | `ledger_core::LedgerState::lines_of` | `engine-queries.json` (queries.rs); properties.rs |  |
| engine.ts:131 | `Ledger.periodFor` | `ledger_core::LedgerState::period_for` | `engine-queries.json` (queries.rs); review_regressions.rs |  |
| engine.ts:169 | `Ledger.addAccount` | `ledger_core::Ledger::add_account` | `commands-accepted.json` (replay.rs, commands.rs); engine_rules.rs; properties.rs |  |
| engine.ts:203 | `Ledger.updateAccount` | `ledger_core::Ledger::update_account` (`AccountPatch`) | `commands-accepted.json` (replay.rs, commands.rs); engine_rules.rs; review_regressions.rs | the typed `AccountPatch` drops patch keys TS keeps (C4) |
| engine.ts:230 | `Ledger.addFund` | `ledger_core::Ledger::add_fund` | `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); properties.rs |  |
| engine.ts:239 | `Ledger.addGrant` | `ledger_core::Ledger::add_grant` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs | typed input: a malformed date fails to deserialise and extra fields are dropped (C3) |
| engine.ts:253 | `Ledger.addDimension` | `ledger_core::Ledger::add_dimension` | `demo-backfill.json` (replay.rs) |  |
| engine.ts:264 | `Ledger.openFiscalYear` | `ledger_core::Ledger::open_fiscal_year` | `commands-accepted.json` (replay.rs, commands.rs); engine_rules.rs; review_regressions.rs |  |
| engine.ts:272 | `Ledger.ensureFiscalYear` | `ledger_core::Ledger::ensure_fiscal_year` | `demo-backfill.json` (replay.rs); `setup.json` (setup.rs) | returns a `Result` (C2) |
| engine.ts:300 | `Ledger.validateEntry` | `ledger_core::LedgerState::validate_entry` | `engine-queries.json` (queries.rs); `importers-journal.json` (imports.rs); properties.rs |  |
| engine.ts:448 | `Ledger.grantSpend` | `ledger_core::LedgerState::grant_spend` | `engine-queries.json` (queries.rs); properties.rs |  |
| engine.ts:500 | `Ledger.saveDraft` | `ledger_core::Ledger::save_draft` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs | a malformed date or a fractional amount is refused; TS stores the draft (C2); a string `fx_rate` is refused, TS stores it (C4) |
| engine.ts:527 | `Ledger.post` | `ledger_core::Ledger::post` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); properties.rs; review_regressions.rs |  |
| engine.ts:540 | `Ledger.inputOf` | `ledger_core::LedgerState::input_of` | `engine-queries.json` (queries.rs) |  |
| engine.ts:564 | `Ledger.updateDraft` | `ledger_core::Ledger::update_draft` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs | as `saveDraft` (C2) |
| engine.ts:579 | `Ledger.deleteDraft` | `ledger_core::Ledger::delete_draft` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs |  |
| engine.ts:589 | `Ledger.postDraft` | `ledger_core::Ledger::post_draft` | `commands-accepted.json` (replay.rs, commands.rs); properties.rs |  |
| engine.ts:610 | `Ledger.reverse` | `ledger_core::Ledger::reverse` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs; review_regressions.rs |  |
| engine.ts:672 | `Ledger.closePeriod` | `ledger_core::Ledger::close_period` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); properties.rs |  |
| engine.ts:689 | `Ledger.reopenPeriod` | `ledger_core::Ledger::reopen_period` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs; review_regressions.rs |  |
| engine.ts:705 | `Ledger.lockPeriod` | `ledger_core::Ledger::lock_period` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); properties.rs |  |
| engine.ts:720 | `Ledger.closeYear` | `ledger_core::Ledger::close_year` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); engine_rules.rs; review_regressions.rs |  |
| engine.ts:780 | `Ledger.addBudget` | `ledger_core::Ledger::add_budget` | `demo-backfill.json` (replay.rs); document_rules.rs | typed input: a malformed date fails to deserialise and extra fields are dropped (C3) |
| engine.ts:788 | `Ledger.setBudgetLine` | `ledger_core::Ledger::set_budget_line` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); document_rules.rs | a new line with `filter` left out stores `null` where TS leaves the key absent (C3) |
| engine.ts:810 | `Ledger.addVendor` | `ledger_core::Ledger::add_vendor` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); review_regressions.rs |  |
| engine.ts:822 | `Ledger.createBill` | `ledger_core::Ledger::create_bill` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs; review_regressions.rs | typed input: a malformed date fails to deserialise and extra fields are dropped (C3) |
| engine.ts:847 | `Ledger.approveBill` | `ledger_core::Ledger::approve_bill` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); review_regressions.rs |  |
| engine.ts:859 | `Ledger.postBill` | `ledger_core::Ledger::post_bill` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); review_regressions.rs |  |
| engine.ts:877 | `Ledger.outstanding` | `ledger_core::LedgerState::outstanding` (`AppliesTo`) | `engine-queries.json` (queries.rs); review_regressions.rs | takes `AppliesTo::{Bill, ExpenseClaim}` in place of a kind string and an id |
| engine.ts:888 | `Ledger.payBill` | `ledger_core::Ledger::pay_bill` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs; review_regressions.rs |  |
| engine.ts:905 | `Ledger.claim` | `ledger_core::LedgerState::claim` | `engine-queries.json` (queries.rs) | returns `Result`; TS throws `NOT_FOUND` |
| engine.ts:909 | `Ledger.createClaim` | `ledger_core::Ledger::create_claim` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs | a malformed date or a non-boolean `submit` fails to deserialise; extra fields are dropped (C3) |
| engine.ts:939 | `Ledger.submitClaim` | `ledger_core::Ledger::submit_claim` | `commands-accepted.json` (replay.rs, commands.rs); document_rules.rs |  |
| engine.ts:954 | `Ledger.approveClaim` | `ledger_core::Ledger::approve_claim` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs |  |
| engine.ts:975 | `Ledger.rejectClaim` | `ledger_core::Ledger::reject_claim` | `commands-accepted.json` (replay.rs, commands.rs); document_rules.rs |  |
| engine.ts:986 | `Ledger.reimburseClaim` | `ledger_core::Ledger::reimburse_claim` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `rejected-cases.json` (rejected.rs); document_rules.rs |  |
| engine.ts:1144 | `Ledger.addBankAccount` | `ledger_core::Ledger::add_bank_account` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `reconcile-auto-match.json` (reconcile.rs); `rejected-cases.json` (rejected.rs) |  |
| engine.ts:1158 | `Ledger.importStatement` | `ledger_core::Ledger::import_statement` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `reconcile-auto-match.json` (reconcile.rs); properties.rs; review_regressions.rs | a malformed statement date fails to deserialise (TS stores it); every amount is checked before any id is drawn, where TS draws the batch id first (C6) |
| engine.ts:1200 | `Ledger.matchStatementLine` | `ledger_core::Ledger::match_statement_line` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs); `reconcile-auto-match.json` (reconcile.rs); properties.rs |  |
| engine.ts:1217 | `Ledger.unmatchStatementLine` | `ledger_core::Ledger::unmatch_statement_line` | `commands-accepted.json` (replay.rs, commands.rs) |  |
| engine.ts:1230 | `Ledger.postFromStatementLine` | `ledger_core::Ledger::post_from_statement_line` | `commands-accepted.json` (replay.rs, commands.rs); `demo-backfill.json` (replay.rs) |  |
| commands.ts:107 | `sanitizeCommand` | `ledger_core::sanitize_command` (to a typed `LedgerCommand`) | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs; review_regressions.rs | a non-number `fx_rate` on a line is dropped; a command that does not fit its type is refused as `INVALID_INPUT` ("The <type> command is malformed: ..."), where TS passes it to the engine (C4) |
| commands.ts:139 | `applyCommand` | `ledger_core::apply_command` | `commands-accepted.json` (replay.rs, commands.rs); `rejected-cases.json` (rejected.rs); properties.rs; review_regressions.rs | `entry.import` is atomic through `Ledger::atomically`; ids drawn stay drawn, as in TS |
| reports.ts:40 | `descendants` | `ledger_core::reports::descendants` | `report-descendants.json` (reports.rs) |  |
| reports.ts:59 | `postedLines` | `ledger_core::reports::posted_lines` | `report-posted-lines.json` (reports.rs) |  |
| reports.ts:142 | `balances` | `ledger_core::reports::balances` | `report-balances.json` (reports.rs) |  |
| reports.ts:149 | `natural` | `ledger_core::reports::natural` | `report-natural.json` (reports.rs) | takes the account's `NormalBalance` rather than the account |
| reports.ts:162 | `trialBalance` | `ledger_core::reports::trial_balance` | `report-trial-balance.json` (reports.rs); report_footing.rs; properties.rs; review_regressions.rs |  |
| reports.ts:181 | `generalLedger` | `ledger_core::reports::general_ledger` | `report-general-ledger.json` (reports.rs) | returns `Result` (`ReportError`) |
| reports.ts:201 | `dayBefore` | `ledger_core::reports::day_before` | `report-day-before.json` (reports.rs) | `ReportError::InvalidDate` for a date off the calendar or `0000-01-01`, where TS throws a `RangeError` (C5) |
| reports.ts:238 | `financialPosition` | `ledger_core::reports::financial_position` | `report-financial-position.json` (reports.rs); report_footing.rs; properties.rs |  |
| reports.ts:299 | `activities` | `ledger_core::reports::activities` | `report-activities.json` (reports.rs); report_footing.rs; review_regressions.rs |  |
| reports.ts:369 | `functionalExpenses` | `ledger_core::reports::functional_expenses` | `report-functional-expenses.json` (reports.rs) |  |
| reports.ts:412 | `cashFlow` | `ledger_core::reports::cash_flow` | `report-cash-flow.json` (reports.rs); report_footing.rs |  |
| reports.ts:469 | `warningLevel` | `ledger_core::reports::warning_level` | `report-warning-level.json` (reports.rs) |  |
| reports.ts:475 | `budgetVsActual` | `ledger_core::reports::budget_vs_actual` | `report-budget-vs-actual.json` (reports.rs) |  |
| reports.ts:555 | `grantSummary` | `ledger_core::reports::grant_summary` | `report-grant-summary.json` (reports.rs); properties.rs; review_regressions.rs | returns `Result`: `UnknownGrant` where TS throws a `TypeError`, `InvalidDate` where TS returns `NaN` months (C5) |
| reports.ts:599 | `ledgerSignals` | `ledger_core::reports::ledger_signals` | `report-ledger-signals.json` (reports.rs) |  |
| reconcile.ts:34 | `detectFormat` | `ledger_core::reconcile::detect_format` | `reconcile-parse-statement.json` (imports.rs) |  |
| reconcile.ts:47 | `parseStatement` | `ledger_core::reconcile::parse_statement` | `reconcile-parse-statement.json` (imports.rs) |  |
| reconcile.ts:113 | `candidatesFor` | `ledger_core::reconcile::candidates_for` | `reconcile-auto-match.json` (reconcile.rs); properties.rs | `taken` is a slice of line ids in place of a `Set` |
| reconcile.ts:141 | `autoMatch` | `ledger_core::reconcile::auto_match` | `reconcile-auto-match.json` (reconcile.rs); properties.rs | the window is an argument (`DEFAULT_WINDOW_DAYS` = 3) in place of a default |
| importers.ts:18 | `detectJournalSource` | `ledger_core::importers::detect_journal_source` | `importers-journal.json` (imports.rs) |  |
| importers.ts:29 | `parseJournalCsv` | `ledger_core::importers::parse_journal_csv` | `importers-journal.json` (imports.rs) |  |
| csv.ts:4 | `parseCsv` | `ledger_core::csv::parse_csv` | `csv.json` (imports.rs); properties.rs |  |
| csv.ts:39 | `parseCsvRecords` | `ledger_core::csv::parse_csv_records` | `csv.json` (imports.rs) |  |
| csv.ts:56 | `toCsv` | `ledger_core::csv::to_csv` | `csv.json` (imports.rs); properties.rs | cells are `CsvCell` (empty, bool, number, text) in place of `unknown` |
| csv.ts:69 | `parseStatementDate` | `ledger_core::csv::parse_statement_date` | `csv.json` (imports.rs) | `None` for a year below 1000, where TS returns a string that is not `YYYY-MM-DD` (C6) |
| coa-template.ts:90 | `accountId` | `ledger_core::coa_template::account_id` | `setup.json` (setup.rs) |  |
| coa-template.ts:103 | `ngoChartOfAccounts` | `ledger_core::coa_template::ngo_chart_of_accounts` | `setup.json` (setup.rs) |  |
| coa-template.ts:123 | `defaultSettings` | `ledger_core::coa_template::default_settings` | `setup.json` (setup.rs) |  |
| coa-template.ts:144 | `UNRESTRICTED_FUND_ID` | `ledger_core::coa_template::UNRESTRICTED_FUND_ID` | `setup.json` (setup.rs) |  |
| setup.ts:11 | `createOrgLedger` | `ledger_core::setup::create_org_ledger` | `setup.json` (setup.rs); properties.rs; review_regressions.rs | returns the books (`LedgerState`), not a `Ledger` |
| setup.ts:28 | `actorFromPermissions` | `ledger_core::setup::actor_from_permissions` | `setup.json` (setup.rs) |  |
| setup.ts:45 | `SYSTEM_ACTOR` | `ledger_core::setup::system_actor()` | `setup.json` (setup.rs) | a function returning the actor |
| bridge.ts:62 | `expenseAccountForCategory` | `ledger_core::bridge::expense_account_for_category` | `bridge.json` (setup.rs) |  |
| bridge.ts:69 | `bridgeTransaction` | `ledger_core::bridge::bridge_transaction` | `bridge.json` (setup.rs) | the context is the `BridgeContext` trait (demo: `demo::DemoBridgeContext`) in place of an object of maps and callbacks |
| money.ts:6 | `assertMinor` | `ledger_core::money::assert_minor` | `money.json` (money.rs) | returns `Result<Minor, MoneyError>`; TS throws a `RangeError` |
| money.ts:13 | `parseMoney` | `ledger_core::money::parse_money` | `money.json` (money.rs) |  |
| money.ts:23 | `toMajor` | `ledger_core::money::to_major` | `money.json` (money.rs) |  |
| money.ts:26 | `fromMajor` | `ledger_core::money::from_major` | `money.json` (money.rs) | returns `Result`; TS throws a `RangeError` |
| money.ts:36 | `convert` | `ledger_core::money::convert` | `money.json` (money.rs); properties.rs | returns `Result`; TS throws a `RangeError` |
| money.ts:53 | `sum` | `ledger_core::money::sum` | `money.json` (money.rs) |  |
| errors.ts:43 | `LedgerError` | `ledger_core::LedgerError` (`new`, `fail`, `code()`, `issues()`, `Display`) | `rejected-cases.json` (rejected.rs); errors.rs | `LedgerError::new` returns `None` for no issues, so an error always has one; RangeErrors are `MoneyError` |
| errors.ts:56 | `fail` | `ledger_core::LedgerError::fail` | `rejected-cases.json` (rejected.rs); errors.rs | returns the error for `Err(..)`; TS throws |
