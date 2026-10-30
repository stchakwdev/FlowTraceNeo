# Screen parity table: web app and desktop

Every full path of the web app's router (`FileRoutesByFullPath` in
`flowtrace/src/routeTree.gen.ts`) with the desktop nav entry that holds the same screen, what
proves it and the known differences. 38 rows. `crates/desktop/tests/screen_parity.rs` reads the
route tree and the rail (`flowtrace/src/lib/navigation.ts`) as text and `NavEntry::ALL` from the
crate, and fails when a route has no row, a row names a route or desktop entry that does not
exist, a cell is empty, or a desktop entry has no route.

Reading the columns:

- Web route: the full path; `$name` is a path parameter.
- Web nav: the rail item the web lights on that path (`isNavItemActive`, `navigation.ts:46-50`),
  or `none` for pages outside the signed-in rail.
- Desktop entry: the `--entry` name (`NavEntry::name`, `crates/desktop/src/model.rs`) of the nav
  entry that holds the screen, or `n/a: <reason>` where the desktop has no counterpart by design,
  or `gap: <task>` naming the task that owns a missing counterpart. Sub-pages of one web section
  (a detail page, a new form) are a tab, a selection or a dialog inside the same entry.
- Proved by: the unit tests (module path under `crates/desktop/src/`, or `tests/<file>`) and the
  IT1 steps (`rust/docs/integration-test.md`) that drive the screen. `tests/live_*.rs` are
  ignored live checks against a real database, run by hand.
- Known differences: deliberate or known differences from the web page; empty when none is
  recorded. The longer list is in `rust/docs/handoff.md`, "Known gaps and differences from the
  web app".

Public pages stay web-rendered (pre-decision P20): the desktop manages and previews what they
show, and links to them, but does not render donate, badge or embed pages itself.

| Web route | Web nav | Desktop entry | Proved by | Known differences |
|---|---|---|---|---|
| `/` | none | n/a: the public landing page; the desktop opens on the connect screen or the books |  |  |
| `/sign-out` | none | n/a: ends the web session cookie; the desktop forgets its token file from the connect screen (README, Signing in with WorkOS) |  |  |
| `/api/auth/callback` | none | n/a: the web's AuthKit callback; the desktop takes its own loopback callback (`src/signin`) |  |  |
| `/donate/$slug` | none | n/a: public donate page, web-rendered (P20); Campaigns copies its link |  |  |
| `/badge/trust/$orgId` | none | n/a: public trust badge image, web-rendered (P20) |  |  |
| `/embed/campaign/$id` | none | n/a: public embed card, web-rendered (P20); Campaigns previews it and copies the embed code |  |  |
| `/dashboard` | Dashboard | `dashboard` | `dashboard` tests; `tests/live_campaigns.rs` |  |
| `/transactions` | Transactions | `transactions` | `transactions` tests | |
| `/transactions/new` | Transactions | `transactions` | `transactions` tests (record form) | a form inside the entry, not a page |
| `/transactions/$id` | Transactions | `transactions` | `transactions`, `proofs` and `verification` tests; `tests/live_proofs.rs` | the transaction page with its proofs section, in the entry |
| `/flows` | Flows | `flows` | `flows` tests; `tests/live_flows_chains.rs` |  |
| `/spend` | Spend | `spend` | `claims` and `bills` tests | |
| `/spend/claims` | Spend | `spend` | `claims` tests; IT1 b3, c2, f1, f2 |  |
| `/spend/claims/new` | Spend | `spend` | `claims` tests | a form inside the entry |
| `/spend/claims/$id` | Spend | `spend` | `claims` tests; IT1 c2, d1 | a selected claim, not a page |
| `/spend/bills` | Spend | `spend` | `bills` tests; `tests/bill_postings.rs`; IT1 b4, c3 |  |
| `/spend/bills/new` | Spend | `spend` | `bills` tests | a form inside the entry |
| `/spend/bills/$id` | Spend | `spend` | `bills` tests; `tests/bill_postings.rs`; IT1 c3, d2 | a selected bill, not a page |
| `/grants` | Grants | `grants` | `tests/grants.rs` |  |
| `/grants/$id` | Grants | `grants` | `tests/grants.rs` | a selected grant, not a page |
| `/grants/budgets` | Grants | `grants` | `tests/grants.rs` (budget versus actual) | no `budget.add` command on either side; both show "No budget set up" (handoff.md) |
| `/ledger` | Ledger | `journal` | `journal` tests; IT1 e1, e2 | the web's Ledger section is split over three desktop entries |
| `/ledger/new` | Ledger | `journal` | `entry_form` tests; IT1 b2 | manual entry is a form inside Journal |
| `/ledger/import` | Ledger | `journal` | `entry_form` tests (QuickBooks and Xero import preview) | the import is a form inside Journal |
| `/ledger/entries/$id` | Ledger | `journal` | `journal` tests | a selected entry, not a page |
| `/ledger/accounts` | Ledger | `accounts` | `accounts` tests; IT1 b1 | the chart is a tab of Accounts |
| `/ledger/periods` | Ledger | `accounts` | `accounts` tests | the periods are a tab of Accounts |
| `/ledger/general` | Ledger | `general-ledger` | `general_ledger` tests | its own entry, not a Ledger sub-page |
| `/reconcile` | Reconcile | `reconcile` | `reconcile` and `statements` tests; `tests/reconcile_parity.rs`; IT1 b5, c4 |  |
| `/reconcile/$bankId` | Reconcile | `reconcile` | `reconcile` tests; `tests/reconcile_parity.rs` | a selected account, not a page |
| `/reports` | Reports | `reports` | `tests/report_csv.rs`; IT1 g1-g4 |  |
| `/campaigns` | Campaigns | `campaigns` | `campaigns` tests; `tests/live_campaigns.rs` |  |
| `/campaigns/$id` | Campaigns | `campaigns` | `campaigns` tests; `tests/live_campaigns.rs` | Copy link and Copy embed code sit on the managed campaign (P20) |
| `/chains` | Chains | `chains` | `chains` tests; `tests/live_flows_chains.rs` |  |
| `/trust-score` | Trust Score | `trust-score` | `trust` tests | weights 32, 24, 24, 20 on both sides (P16); the chart's labels sit on approximate baselines |
| `/settings` | Settings | `settings` | `settings` tests; `tests/live_settings.rs` | the web reaches Access groups and Embed from Settings only; the desktop gives each its own entry |
| `/settings/access-groups` | Settings | `access-groups` | `access_groups` tests; `tests/live_settings.rs` | its own entry, not a Settings sub-page |
| `/settings/embed` | Settings | `embed-badges` | `badge` tests | its own entry; campaign embed code also sits on the managed campaign (P20); the Large badge's letter spacing is not drawn |
