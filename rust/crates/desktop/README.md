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
| `panels` | placeholder panels and shared pieces: `page_title`, `section_header`, `demo_tag`; `panels::accounts::AccountsPanel` (U2) | yes |
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
view (`uniform_list`), and emits `OpenEntry(EntryId)` on a click for the Journal screen (U3).

Flags: `--large-books 50000` adds synthetic books (`large_books::build`, two-line entries posted
through the engine with the fixed test env) listed and selected first; `--gl-account 1110`
filters on an account code; `--gl-row 25000` scrolls to a row and opens its entry;
`--gl-scroll-bench on` scrolls the whole table 25 rows a frame and logs frame times to stderr.
Shots: `docs/screens/u4-general-ledger.png` (demo books, account 1110, an entry open) and
`docs/screens/u4-general-ledger-50k.png` (50,000 lines, mid-scroll).
