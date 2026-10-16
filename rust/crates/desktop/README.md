# desktop: FlowTrace Books

The GPUI desktop app on gpui-kit 0.7.3. U1 built the shell: a masthead, a nav rail with the
organisation picker and seven entries, one placeholder panel per entry, a connect screen and a
status bar, over an in-process backend that runs `ledger-core` in the app. Each screen task
(U2 to U9) replaces one placeholder; U10 adds the SpacetimeDB backend behind the same trait.

## Run

```sh
cd rust
cargo run -p desktop                               # follows the system light or dark appearance
cargo run -p desktop -- --theme dark --entry 3     # dark, General ledger open
cargo run -p desktop -- --screen connect           # opens on the connect screen
cargo run -p desktop -- --report activities        # Reports, on the statement of activities
```

Keys `1` to `7` open the nav entries (Accounts, Journal, General ledger, Spend, Grants,
Reconcile, Reports) while the books show and no field has focus (the shell only handles keys
while its own focus handle is focused, so typing in a field never switches screens). Other
keys go to the screen showing, except `Page Up`, `Page Down`, `Home` and `End`, which scroll
the content. On Accounts, `a` opens the chart, `p` the periods and `n` the
add form with the code field focused (Enter moves to the name, Enter there submits). The
status bar names the organisation, its currency and the connection, and toggles the theme.

The demo organisation (Demo NGO Kenya) holds the demo books from
`ledger_core::demo::build_demo_ledger()`; its commands draw ids from the fixed demo env,
continued past the ids the demo already holds, at the demo's frozen time. The second
organisation is empty books from `ledger_core::setup::create_org_ledger` for this year in KES;
it draws UUIDs and wall-clock time from `backend::SystemEnv`. Nothing is saved: books reset
when the app closes.

## Layout

| Module | Holds | GPUI? |
|---|---|---|
| `backend` | `LedgerBackend` trait, `InProcessBackend`, `SystemEnv`, `BackendEvent`, `ConnectionStatus` | no |
| `model` | `NavEntry`, `Navigation` (entry, screen, keys), `LedgerView` (orgs, selection, status, snapshot, revision), `ConnectForm` | no |
| `accounts` | Accounts screen view models: `account_rows` (tree and balances), `NewAccountForm`, `EditAccountForm`, `fiscal_years`, `PeriodRow`, `Pending` (reopen, lock, year-end close), `Failure` (engine messages), `format_minor`, `format_date` | no |
| `theme` | `DESIGN.md` tokens: `LIGHT`/`DARK` palettes in OKLCH, `text`, `space`, `radius`; `apply(mode, cx)` | `apply` only |
| `store` | `LedgerStore`, the one store entity: owns the backend and the `LedgerView`, drains the change channel in batches | yes |
| `shell` | `Shell`, the root view: masthead, rail, content, connect screen, status bar, keys | yes |
| `bills` | Bills, payments and vendors view models: `bill_rows`, `filtered`, `filter_count`, `vendor_rows`, `NewVendorForm`, `NewBillForm` (lines, `total`, `command`), `next_step` (status to actions), `bill_detail` (coded lines, accrual and payment entries, fund split, status trail), `PayForm` | no |
| `journal` | Journal view models: `entry_rows` (latest first, totals, funds), `StatusCounts`, `JournalFilter` (status, source, period, text), `source_choices`, `period_choices`, `list_notice`, `list_footer`, `EntryView` (lines, facts, audit trail, reversal links, `actions_for` by status, post and delete commands), `ReverseForm`, `DeleteDialog`, `override_reason`, `format_signed` | no |
| `entry_form` | Manual entry form and CSV import view models: `EntryForm` (lines, `set_debit`/`set_credit`, `set_fund`, `balance`, `field_errors`, `issues`, `summary`, `save_command`, `post_commands`, `from_draft`), `Balance`, `account_choices`, `tag_choices`, `ImportPreview`, `read_import_file` | no |
| `claims` | Spend claims view models: `claim_rows`, `filter_counts`, `list_notice`, `ClaimForm` (lines, `set_fund`, `total`, `command`), `parse_day_month_year`, pickers, `ClaimDetail` (trail, next step, actions, `Posting` with the grants hit), `ReimburseForm`, `has_overridable` | no |
| `panels` | placeholder panels and shared pieces: `page_title`, `section_header`, `demo_tag`, and the form pieces `field`, `choice`, `tag`, `hint`, `failure_box`, `form_buttons`; `panels::accounts::AccountsPanel` (U2), `panels::spend::SpendPanel` (U5) hosting `panels::bills::BillsPanel` (U6); `Pick`/`PickState` id-valued select items | yes |
| `reconcile` | Reconcile view models: `bank_rows`, `overview`, `NewBankForm`, `ImportPreview` (parse, skipped rows, money in and out, `repeats` as the engine dedupes), `LineStatus` and `status_of`, `line_rows`, `candidates`, `position`, `auto_match_summary`, the `bank.add` and `statement.*` command builders, `PostForm`, `post_accounts` | no |
| `statements` | the financial statements as rows: `ReportKind`, `ReportForm` (parameters and validation), `presets`, `statement` (tables, key figures, footing checks), `csv_rows`/`export_csv`/`write_export`, `format_minor`, `format_compact`, `format_date`, `to_fixed` | no |
| `app` | start-up: gpui-kit init, theme, store, window (`app_id` `flowtrace-books`) | yes |

Everything under "no" is unit-tested (`cargo test -p desktop`); rendering code stays thin and
reads those structs.

### The backend trait

```rust
pub trait LedgerBackend {
    fn organisations(&self) -> Vec<Organisation>;
    fn state(&self, org: &OrganizationId) -> Option<Arc<LedgerState>>;
    fn subscribe(&mut self) -> Receiver<BackendEvent>;          // async_channel, one per call
    fn submit(&mut self, org: &OrganizationId, command: LedgerCommand)
        -> Result<CommandResult, BackendError>;
    fn status(&self) -> ConnectionStatus;  // Disconnected | Connecting | Connected { identity } | Error { message }
    fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError>;
}

pub enum BackendEvent {
    Ledger { org: OrganizationId, state: Arc<LedgerState> },   // whole new snapshot
    Organisations(Vec<Organisation>),
    Status(ConnectionStatus),
}
```

A successful `submit` sends one `BackendEvent::Ledger` with the new books; a refused one sends
nothing and changes nothing. Snapshots are immutable `Arc`s: the backend applies a command to a
copy, so a screen holding the old snapshot never sees it change. For U10, `ledger-client`'s
`SpacetimeClient` maps on directly: its blocking `submit` returns the same
`Result<CommandResult, _>`, and its `ClientEvent::Changed { orgs }` becomes one
`BackendEvent::Ledger` per org after re-reading `state(org)`.

## How a screen replaces its placeholder

The pattern is ADR 0002's ("Pattern for U1 to U10"): screens read the store, never the backend
directly, and never patch rows after a write.

1. Add `src/panels/<screen>.rs` (turn `panels.rs` into `panels/mod.rs` the first time) with a
   view entity, for example `AccountsPanel`. It takes `Entity<LedgerStore>` and keeps a
   `cx.observe(&store, ...)` subscription in a field; in the callback it re-derives its rows from
   `store.read(cx).view().snapshot()` (skip the work when `view().revision()` is unchanged) and
   calls `cx.notify()`. Tables hold their rows in their own `TableState` delegate.
2. Put the derivation in a plain function or struct under the screen's module and unit-test it
   against `ledger_core::demo::build_demo_ledger()`, as `model` does for the shell.
3. In `shell.rs`, add the entity as a field of `Shell`, build it in `Shell::new` with
   `cx.new(|cx| AccountsPanel::new(store.clone(), window, cx))`, and replace that entry's arm in
   `Shell::panel` with `NavEntry::Accounts => self.accounts.clone().into_any_element()`.
4. Writes: build a `ledger_core::LedgerCommand` and call
   `self.store.update(cx, |store, _| store.submit(command))`. Show the `BackendError` on failure;
   on success do nothing, because the new books arrive through the store's change channel and
   the observer re-derives the rows.
5. Use the `theme` tokens and the `panels` pieces (`page_title`, `section_header`, `demo_tag`)
   rather than literal colours or sizes; tag every block of demo figures with `demo_tag`.
6. Save a screenshot of the screen to `rust/docs/screens/<task>-<screen>.png`.

A screen that needs data the snapshot does not hold adds it to the backend trait with a test
(pre-decision P7), not a special case in the screen.

## Theme

`theme::apply(mode, cx)` calls `Theme::change(mode)` and then edits the colours in a second
`Theme::update`: switching mode loads that mode's registered theme over any colours set in the
same closure (gpui-component 0.7.3 `theme/mod.rs`, `Theme::update`;
<https://gpui-kit.com/component/theme>). Token names follow `DESIGN.md`: `report_blue`,
`report_blue_ink`, `coverage_blue`, `signal_orange`, `rejection_red`, `report_paper`, `sheet`,
`ink_navy`, `ink_muted`, `gap_grey`, `hairline`, `field_stroke`, `wash`, `rail`, `rail_hover`,
`masthead_ink`, `masthead_paper`; type sizes in `theme::text` (`HEADLINE`, `TITLE`, `FIGURE`,
`LEDE`, `BODY`, `SECTION_LABEL`, `LABEL`, `MONO`); sizes in `theme::space` (`CELL`, `GUTTER`,
`SECTION`, `RULE_HEAVY`, `HAIRLINE`, `MASTHEAD`, `RAIL`, `NAV_ITEM`, `SECTION_HEADER`,
`INPUT`, `BUTTON`, `STATUS_BAR`); radii in `theme::radius` (`NONE`, `HAIRLINE`). Read the
palette in render with `theme::palette(cx.theme().mode)`.

Archivo and IBM Plex Mono are not installed on the Omarchy machine, so text renders in the
system font; the families are named in `theme::text` for when they are bundled.

## Screenshots

On Omarchy (Hyprland), from a shell that is not inside the graphical session:

```sh
export XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1
# HYPRLAND_INSTANCE_SIGNATURE: read it from the environ of a Hyprland child process
export HYPRLAND_INSTANCE_SIGNATURE=$(tr '\0' '\n' < /proc/$(pgrep -P $(pgrep -n Hyprland) | head -1)/environ \
  | sed -n 's/^HYPRLAND_INSTANCE_SIGNATURE=//p')
./target/debug/desktop --theme light --entry 1 &
sleep 4
geom=$(hyprctl clients -j | python3 -c "import json,sys
for c in json.load(sys.stdin):
    if c['class'] == 'flowtrace-books': print('%d,%d %dx%d' % (*c['at'], *c['size']))")
grim -g "$geom" docs/screens/u1-shell.png
kill %1
```

`wtype <key>` sends a key to the focused window (U1 checked that `4` opens Spend this way);
Hyprland 0.56 has no synthetic click. Omarchy gives every window 0.985 opacity, so the wallpaper
shows faintly through screenshots (ADR 0002). Current shots: `docs/screens/u1-shell.png` (demo
org, light), `docs/screens/u1-shell-dark.png` (dark, General ledger),
`docs/screens/u1-connect.png`, and from U2 `docs/screens/u2-accounts.png`,
`docs/screens/u2-periods.png` (key `p`) and `docs/screens/u2-accounts-error.png` (keys `n`,
`1110`, Return, a name, Return: the engine refuses the code in use). Keys reach the window
after `hyprctl dispatch focuswindow address:<addr>`; `wtype -k Return` sends Enter. Other
threads may have their own `flowtrace-books` windows open, so pick yours from
`hyprctl clients -j` by `pid` (the `$!` of the launch), not by class.
`wtype -k End` (or `Page_Down`) scrolls the content before the shot.

## Reports (U9)

`desktop -- --report <id>` opens Reports on one report: `trial-balance`, `position`,
`activities`, `functional`, `cash-flow` or `grant`. The picker, the period presets, the dates,
the basis and (for the grant report) the grant are the web screen's
(`flowtrace/src/routes/_authenticated/reports/index.tsx`); the demo books open as at the demo
date, other books as at today. Each statement is laid out by `statements::statement` as
`ReportBody` lays it out: tables of rows (`RowKind::Heading`, `Line`, `Subtotal`, `Total`,
`Gap`), key figures and footing checks; `panels::reports` only draws them. Export writes
`statements::export_csv` (the web's `reportCsv` columns, amounts as `toFixed(2)`, through
`ledger_core::csv::to_csv`) to the path in the field, which follows the report as
`~/Downloads/<id>_<date or from_to_to>[_<basis>].csv` (the web's download name) until the user
types or chooses one; `Choose...` opens the platform save dialog. `tests/report_csv.rs` parses the
CSV of every demo-books case of the `report-*.json` fixtures back and checks its totals. Shots:
`docs/screens/u9-trial-balance.png`, `u9-trial-balance-totals.png`,
`u9-financial-position.png`, `u9-activities.png`, `u9-grant-dark.png`; a sample export is
`docs/screens/u9-trial-balance.csv` (checked against a fresh export by the tests).

## General ledger (U4)

`general_ledger` holds the screen's view models, free of GPUI: `GlFilter` (account with its
sub-accounts, fund, grant, tag, period, from, to) maps to a `ledger_core::reports::LineFilter`
(a period gives its first and last day, overlapped with from and to); `FilterChoices` lists each
select's options in the web screen's order; `GlTable::derive` turns
`reports::general_ledger` into rows (date, entry, account, memo, fund, grant, debit, credit, and
the running balance natural side positive) plus a totals row; `GlCache` derives again only when
the store revision or the filter changes; `EntryDetail` is the clicked line's entry.
`panels::general_ledger` draws it with a gpui-kit `DataTable`, which renders only the rows in
view (`uniform_list`), and emits `OpenEntry(EntryId)` on a click; the shell opens that entry on the Journal screen (U3).

Flags: `--large-books 50000` adds synthetic books (`large_books::build`, two-line entries posted
through the engine with the fixed test env) listed and selected first; `--gl-account 1110`
filters on an account code; `--gl-row 25000` scrolls to a row and opens its entry;
`--gl-scroll-bench on` scrolls the whole table 25 rows a frame and logs frame times to stderr.
Shots: `docs/screens/u4-general-ledger.png` (demo books, account 1110, an entry open) and
`docs/screens/u4-general-ledger-50k.png` (50,000 lines, mid-scroll).

## Spend: expense claims (U5)

The Spend entry has two tabs: Expense claims (`c`) and Bills (`b`, U6, below). The claims tab shows the list, a new
claim, or one claim (`ClaimsView`). Keys while no field has focus: `n` new claim, `f` next status
filter, `j`/`k` or the arrows move the highlighted row, Enter opens it; on a claim `s` submits,
`a` approves, `r` rejects (the reason field takes focus), `p` records the reimbursement, Enter
confirms the open box and Escape steps back. The backend trait gained `actor(org)`, the person
the app acts as (pre-decision P7), so the screen knows who may submit or approve: the demo user
cannot approve their own claims, as the engine rules.

After an approval the detail shows the posted entry and each grant its expense lines charged,
with the grant's spend so far (`claims::Posting`). Shots: `docs/screens/u5-claims.png`,
`u5-claim-new.png` (keys `n`, typing, Enter between fields) and `u5-claim-detail.png` (Enter on
EC-0002, `a`, Enter).

## Spend: bills, payments and vendors (U6)

The Bills tab of Spend is `panels::bills::BillsPanel`, an entity `SpendPanel` holds and
observes: `SpendPanel::render` takes the title, lede and action from `BillsPanel::header` and
the body from `BillsPanel::body`, and forwards keys while the Bills tab shows. Under the Spend
tabs the panel switches between All bills (`b`, which also returns to the list) and Vendors
(`v`). Keys while no field has focus: `n` new bill, `f` next status filter (All, Draft,
Approved, To pay, Paid, Void, as the web), `j`/`k` or the arrows move the highlighted row, Enter
opens the bill, Escape goes back to the list.

The detail shows "What happens next" with only the actions the engine would take from the
acting user (`bills::next_step`): a draft is approved by someone with approval rights who did
not enter it, so a bill the demo user enters here waits for another approver, as in the engine.
Approve and Post run at once; Pay opens an inline form (amount defaulting to what is owed, date,
cash account, reference; part payments allowed). Each payment lists the engine's fund split
(the payable debits of its entry, never recomputed), and "In the books" shows the accrual and
payment entries with their lines (U4's `general_ledger::EntryDetail`). The status trail joins
the bill's audit events with its posting and payments. `tests/bill_postings.rs` posts and pays
the approved demo bill through the in-process backend and checks the postings.

Differences from the web: a new supplier is added on the Vendors tab rather than inside the
bill form; proof files are not attached (the desktop has no file store yet); vendors have no
default account or contact field in the ledger, so the list shows payment details and the
flow-node id. Shots: `docs/screens/u6-bills.png`, `u6-vendors.png`, `u6-bill-new.png` (keys `b`,
`n`, typing) and `u6-bill-detail.png`, `u6-bill-detail-books.png` (the paid AutoXpress bill,
keys `b`, `j`, `j`, Enter, then End).

## Grants (U7)

`grants` holds the screen's view models, free of GPUI: `grant_rows` (every grant with its
`reports::grant_summary`, most used first, as the web's `grants/index.tsx`), `grant_totals`,
`grant_detail` (key figures, `runway`, the grant's budget lines and its posted lines through
`GlFilter` with closing entries left out, newest first), `budget_view` (the chosen budget, else
the one for the as-at year, else the latest; lines from `reports::budget_vs_actual` grouped
grant, running, department as `budgets.tsx` groups them; `table()` for the Reports statement
table, `figures()`, `bars()` for the chart), and the forms `NewGrantForm` (`grant.add`),
`BudgetLineForm` and `LineEdit` (both `budgetLine.set`). Warnings are the engine's
`warning_level`: 80% used from exactly 0.8, over above 1; a zero budget with spend has `used`
Infinity and shows "spent with no budget" and an en dash in the Used column, as the web does.
There is no command that adds a budget (TS `commands.ts` has none), so the budget tab shows the
web's "No budget set up" notice on books without one.

`panels::grants` draws it: keys `g` (grants), `b` (budget vs actual) and `n` (add a grant);
clicking a grant opens its page. The budget chart is gpui-kit's `BarChart`
(<https://gpui-kit.com/component/chart>), single-series, so each line gets a budget bar (gap
grey) and an actual bar (report blue, signal orange at 80%, rejection red over) with unique band
labels. `--grants list|budgets|<grant id>` opens the entry on a view for screenshots
(`--entry 5 --grants grant-kisima-wash`). Tests: `tests/grants.rs`. Shots:
`docs/screens/u7-grants.png`, `u7-grant-detail.png`, `u7-budgets.png` (figures and chart) and
`u7-budgets-table.png` (Page Down: the over-budget line, the axis and the table).

## Journal (U3)

Key `2` opens the Journal: key figures, the status choices with counts (`f` steps through them),
a search over number, memo and override reason, and the source and period pickers, then the
entries latest first (number, date, memo with its line count, source, funds, status, total) and
the debits shown. `j`/`k` or the arrows move the highlighted row, Enter opens it; a click on a
line in the General ledger opens its entry here too (`Shell::on_open_entry`). The list draws
the latest 500 rows and asks to filter past that (only the synthetic large books reach it).

An entry shows its key figures, created and posted by whom, the override reason, links to the
entry it reverses or that reversed it, the lines (account and memo, fund and grant, tags, debit
and credit in the line's currency, the signed base amount, the rate of a foreign line) with
totals, and the audit trail. A draft offers Edit (`e`), Delete (`d`, with a confirmation) and
Post (`p`); a refusal every issue of which an override lets through opens the Post with an
override box. A posted entry offers Reverse (`r`: date and reason; Enter confirms) and then
opens the reversing entry. Engine messages show as the engine words them.

`n` (or New journal entry) opens the form: date, memo, and two or more lines (account, debit,
credit, fund, grant, tag, line memo) as the web's `new.tsx`. Typing a debit clears that line's
credit and the other way round, and the debits, credits and difference under the lines follow
every keystroke (`EntryForm::balance`, at each kept line's rate when a loaded draft carries a
foreign currency). Save draft sends `entry.saveDraft` (`entry.updateDraft` on a draft); Post
checks the entry with the engine's `validate_entry` first and lists what stops it, then sends
`entry.post` (or `entry.updateDraft` and `entry.postDraft`). `i` (or Import journals) opens the
CSV import: a path (Enter or Load reads it; `Choose...` opens the platform file dialog) and the
fund for every line, then a preview of each draft with its issues, the unreadable rows and the
accounts not in the chart, and Create N drafts sends one `entry.import`. The web's paste box is
left out for now (a gpui-kit textarea could take it); the path field and the dialog read the
same files.

Shots: `docs/screens/u3-journal.png`, `u3-entry-detail.png` (Enter on JE-0016, `r`, a reason,
Enter: the reversing entry), `u3-entry-new.png` (`n`, a memo, Enter, 2,500, Tab to line 2's
credit, 2,000: the difference shows 500.00 in red) and `u3-import.png` (`i`, the path of
`docs/screens/u3-import-sample.csv`, Enter).

## Reconcile (U8)

`panels::reconcile::ReconcilePanel` replaces the Reconcile placeholder, over the view models in
`reconcile` (web: `reconcile/index.tsx` and `reconcile/$bankId.tsx`). The accounts list shows
each bank and mobile-money account with its book balance (`reports::balances` to the as-of date),
the net of its statement lines, matched and unmatched counts and the last statement date, under
the web's key figures. `n` opens the add-account form (`bank.add`; the web has no such form, the
fields are the engine's `NewBankAccount`). `j`/`k` and Enter open an account.

On an account: `i` opens the import box. Type a path and press Enter (or use `Choose...`, the
platform open dialog through `App::prompt_for_paths`); the preview is `reconcile::parse_statement`
with the detected format, the rows, the skipped rows with their reasons, money in and out, and
how many rows are already on the statement (`reconcile::repeats`, the engine's dedupe rule).
Enter again imports (`statement.import`) and reports the engine's count. `a` runs Auto-match
(`statement.autoMatch`) and shows its three counts. `j`/`k` move through the statement lines;
the highlighted open line lists its candidates from `ledger_core::reconcile::candidates_for` at
the web's 7-day manual window, in the engine's order; `c` moves to the next candidate and `m`
matches (`statement.match`); `u` unmatches; `p` opens Post as a new entry (`statement.post`,
account, fund, grant and memo as the web's dialog; an empty account posts to suspense). `f`
cycles the status filter; Escape steps back.

Statuses are the web's four plus Posted: a line matched to the entry posted from it (source
type statement) shows "Posted from the statement" rather than "Matched by hand". The engine
always posts a statement entry (no draft option), so the screen does too.
`tests/reconcile_parity.rs` runs every demo-books case of `rust/fixtures/reconcile-auto-match.json`
through the in-process backend: the candidates the view model lists equal the fixture's, and
Auto-match gives the fixture's result and statement lines. Shots: `docs/screens/u8-bank-accounts.png`,
`u8-reconcile.png` (keys `j j j` Enter, End), `u8-import.png` (then `i`, a path, Enter;
the sample statement is `docs/screens/u8-mpesa-statement.csv`, synthetic) and
`u8-reconcile-matched.png` (`a`, Page Down). Typing the absolute path of the sample (under `~/.herdr/...`) with `wtype` was cut at
`/home/tim0/`, so the shot typed a relative, dot-free symlink to it (`out/u8-statement`).
