# Handoff: FlowTrace Books, October 2026

What a maintainer who was not on the build needs: where things stand, what is known to differ
from the web app, the three decisions the product owner confirmed, and the next steps.
Start with `../README.md`; the plan the work followed is `PRIMER.md` section 8 at the repository
root.

## Where things stand

- **ledger-core** is at parity with the TypeScript ledger engine (`flowtrace/src/lib/ledger/`,
  PR #8) on every fixture the TS engine exports: 27 fixture files, 344 report cases, 37 accepted
  commands, 37 rejected cases, 108 demo backfill steps, 64 queries, 15 property tests.
  `parity.md` maps each of the 94 TS exports to its Rust counterpart and proving test.
- **ledger-module** is published to maincloud as `flowtrace-ledger-dev` (by the user,
  2026-10-16) and passed the end-to-end checks there (`module-release.md`). The production
  database `flowtrace-ledger` was published on 2026-10-16 at 20:25 UTC from main `d65f3ba`, by
  the coordinator on the user's explicit instruction (database identity
  `c200b4b1041fce8318a0027f02f44cf0517c52599a8b10b4b629476899750b4b`; sole operator the
  publisher, `c200a773ad0003c4f82fac12ada53ff076d83fec49a6d827149d813527431630`; no
  organisations at publish). Publishing, resetting or deleting production remains the user's
  call; a module update is the user-run command in `module-release.md`, section 4.
- **ledger-client** connects, keeps its own cache, rebuilds the books and submits commands; its
  live test passed against maincloud dev.
- **desktop** (FlowTrace Books) has the seven books screens (Accounts, Journal, General ledger,
  Spend, Grants, Reconcile, Reports), Dashboard and Campaigns (DT1), Flows and Chains (DT2), and
  Transactions with the shared Proofs section on transactions, claims and bills (DT3), and
  Trust Score, Settings, Access groups and Embed & badges (DT4)
  over either the in-process demo books or the live module,
  with a Connection screen, a saved token and an offline mode. It was checked on Linux
  (Omarchy, Hyprland on Wayland) and on macOS (release build and the full test suite on the
  user's MacBook, 2026-10-16).
- **Packaging**: `rust/tools/release.sh` builds a stripped release archive for the current OS
  (`../README.md`, "Release build"); a tag `v*` builds Linux and macOS archives in CI as
  workflow artifacts. Nothing is signed, and no GitHub Release is made. Each archive ships a
  `config.toml` naming `flowtrace-ledger` on maincloud; copied into the app's config dir it opens
  production, and without it the app opens the demo (INSTALL.txt in the archive).

## Decisions confirmed by the user (2026-10-16)

The coordinator took the recommended option on each while the user was away (standing rule S1)
and queued them; the user confirmed all three on 2026-10-16. Two of them shape stored data, so
changing either once production books exist means a migration.

1. **Timestamps in milliseconds in the Rust module.** Confirmed 2026-10-16. The TS module stamps
   `ctx.timestamp.toISOString()`, which has microseconds (`.123456Z`); ledger-core's `Timestamp`
   holds the engine's millisecond shape (`.123Z`), so the Rust module truncates to milliseconds
   (`crates/ledger-module/src/clock.rs`, `iso_millis`; ADR 0003, Decision 5). Every fixture and
   the engine use milliseconds, and the Rust module and the desktop share one database, so
   byte parity with the TS module's stored stamps is not needed. Matching TS would mean
   ledger-core's `Timestamp` accepting six fractional digits. Changing it later would rewrite or
   mix the stamps of every stored audit event and entry.
2. **`float_roundtrip` on the workspace `serde_json`.** Confirmed 2026-10-16. `rust/Cargo.toml` turns the feature on
   for every crate (PR #19), so a 16 or 17 digit float in command JSON (an `fx_rate`) parses to
   the nearest double, as JS `JSON.parse` does. Without it, three of the fifteen report fixture
   files fail by one ulp (`../fixtures/README.md`, "Rust reports"). The cost is a slightly
   slower float parser. It decides how stored rates read back.
3. **No `budget.add` command.** Confirmed 2026-10-16. Neither the TS command set (`flowtrace/src/lib/ledger/commands.ts`)
   nor ledger-core has a command that creates a budget; budgets exist only through the demo
   backfill (`Ledger::add_budget` is an engine method, not a command). The desktop's budget tab
   shows the web's "No budget set up" notice on books without one, as the web does. Parity won
   over inventing a command. If budgets should be created in the app, the command has to be
   added to the TS engine first (ground truth) and then mirrored.

## Known gaps and differences from the web app

### Scope

- Books covers the web app's finance area: `ledger/*`, `spend/*`, `grants/*`, `reconcile/*`,
  `reports`, and since DT1 the Dashboard and Campaigns (list, new, manage, record and confirm
  donations, preview card, link and embed code), and since DT2 Flows (the flow map, its details
  panel and ledger links) and Chains (list, request, approve, reject, revoke, cancel), and since
  DT3 Transactions (list, page, record) with proofs (gallery, attach panel, viewer), and since
  DT4 the Trust Score page, Settings (profile and "Show on public pages"), Access groups and the
  Trust Score badge. It has no counterpart for the public donate, badge and embed pages.
- DT1, deliberate difference (P20): the desktop manages and previews campaigns, and puts
  **Copy link** and **Copy embed code** on the campaign it manages; the web keeps the embed code
  under Settings > Embed and the copy-link button on the public donate page. The snippet and the
  link are the web's (`embed-code-generator.tsx`, `/donate/<slug>`) on `web_url` from
  `config.toml` (default `https://flowtrace.app`, the web's fallback origin).
- DT1: a pending gift has a **Confirm payment** button (`update_donation_status` to confirmed),
  which the web page lacks; without it a gift recorded by hand could never count toward the
  raised amount. Payment status stays a recorded field, nothing is charged (P19).
- DT1: as on the web, a campaign's name, target and dates cannot be edited after it is saved
  (`update_campaign` takes any field, but neither app has an edit form). The Dashboard's
  "Open the flow map" arrived with DT2 and "Record a transaction" with DT3.
- DT1 nav: Dashboard is key `0` at the top of the rail and Campaigns key `8` under
  "Transparency"; keys `1` to `7` keep their screens. The app still opens on Accounts.
- DT2 nav: Flows is key `9` under Dashboard and Chains sits under Campaigns with no key (the
  digits are spent); `--entry` takes an entry's name (`chains`) as well as its digit.
- DT2 Flows: the web's "Record a transaction" button (empty notice) and the movement rows'
  links to a transaction's page wait for the Transactions screen (DT3); the map is drawn
  natively (canvas lines, div nodes), so text sits on approximate baselines; the desktop adds
  keys (`w` `m` `q` `y` period, `-` `=` `f` zoom, Left and Right in money order, `l` ledger
  lines) to the web's Tab and Esc. Ledger links open the General ledger on the movement's
  linked journal entries first (`journal_entry_ids`), the web's source match only when it has
  none. Node visibility settings have no screen (the web has none either).
- DT2 Chains: the request dialog searches the published organisations
  (`public_organization`), since the module has no organisation search; the web's demo searches
  every organisation, so here an organisation must publish its profile to be found ("No
  organisation matches ... ask them to make their profile public on FlowTrace first."), and the
  module refuses a target without books. The text fields are single-line inputs where the web
  has text areas. In the in-process demo the two active chains' granted groups belong to the
  demo organisation (the dataset's quirk), so they show "None" for access: the module looks
  the group up in the target's records, where the web's demo looks it up anywhere.
- DT3 nav: Transactions sits second in the rail (the web's place) with no key; `--entry
  transactions`. The Flows empty notice's "Record a transaction" and the movement rows' links to
  a transaction's page are still not wired from Flows.
- DT3 Proofs, deliberate differences: a manager publishes a proof at the Worker's public link
  from the viewer (`mark_published`; the web has no control for it, and the public badge path
  needs it); the GPS reading is typed (no geolocation on the desktop); a file's type is read from
  its first bytes, so only the photos and PDFs the Worker stores pass even where the web's
  `PROOF_ACCEPT` lists Word, JSON, text or video (a Video proof cannot carry a file through the
  Worker today); PDFs and videos open in the browser, not inside the viewer; tiles show the
  type's short name where the web draws an icon; the screen limit is the web's 10 MB (the Worker
  allows 25 MB); opening and downloading a file log `view` and `download` through
  `log_verification_access` (the web logs nothing). Gaps outside DT3's scope: the Worker has no
  delete route, thumbnails exist only with its Images binding, and a proof's own
  `visibility_level` is enforced nowhere (module, TS and Worker alike: what a reader sees comes
  from their access groups). Claims' and bills' Proofs sections take the attach panel's kinds;
  their `a` key stays Approve, so the panel opens from its button there.
- DT4 nav: Trust Score, Settings, Access groups and Embed & badges sit under "Transparency"
  with no keys (`--entry trust-score`, `settings`, `access-groups`, `embed-badges`); the web
  reaches the last two from Settings only.
- DT4 Trust Score: four parts at the module's weights 32, 24, 24 and 20 (P16; RW3 made the same
  change on the web, which had 40, 30 and 30), Independent review reading "Not yet scored" until
  a rubric review is stored. The chart is a canvas, its labels positioned divs, so text sits on
  approximate baselines.
- DT4 badge: the web draws Flat, "Flat, square corners" and Plastic alike (two flat, square
  fields), and so does the desktop; only Large differs. The preview's letter spacing for Large
  is not drawn; the text is in capitals as on the web. The score and grade grids
  (`BadgePreviewGrid`) are drawn under the live preview; the web's page does not use them.
- DT4 Embed & badges: the campaign widget's code stays on each campaign (DT1, P20); this screen
  names the ready campaigns and opens Campaigns. The web's "Embed settings" section (added by
  RW3: allow embedding, amounts, Trust Score, node names, transaction count, a new token) is not
  drawn here, by the brief; `SettingsWrite::UpdateEmbedConfig` and `RegenerateEmbedToken` and
  the client wrappers exist for it.
- DT4 Access groups: "Create the standard groups" runs the module's
  `create_default_access_groups` (managers only; refused once any default exists), as RW3's web
  page does. Names come from the organisation's member rows; an invite or a member without one
  shows the address, else the user id (the module keeps no profile names). Invites send no
  email, and an expired membership still grants access until a cleanup reducer exists (module
  gaps). Dialogs are inline boxes; Access ends is typed day/month/year (no calendar picker);
  Description is a single-line input.
- DT4 live: an organisation with no transparency rows yet now reads empty records once its
  books arrive (`backend::spacetime`), so a new organisation's Settings and Access groups can
  write the first ones; before, such an organisation showed no records at all.
- Sign-in: with `workos_client_id` in `config.toml` the desktop signs in with WorkOS AuthKit
  through the system browser (AUTH2, `crates/desktop/README.md`); without it, it connects with a
  device identity (a token file). Auth providers are the user's decision.
- The desktop cannot open books for a new organisation or manage members on a live server:
  those are `create_org_ledger`, `add_member` and `remove_member` calls from the CLI
  (`module-release.md`, section 3). `ledger-client` has `create_org_ledger` and `import_books`
  but no member calls.
- The module stores no organisation name, so the live desktop lists organisations by id.
- The in-process demo books are not saved: they reset when the app closes.

### Screens (from `../crates/desktop/README.md`)

- Bills: a new supplier is added on the Vendors tab, not inside the bill form; vendors have no
  default account or contact field in the ledger.
- Proof files go to the proofs Worker (STO1, DT3): `proofs_url` in `config.toml`; without it,
  or on the in-process demo books, attaching says what to set up.
- Journal: the web's paste box on CSV import is left out (the path field and the file dialog
  read the same files); the list draws at most 500 rows and asks to filter past that.
- Reconcile: adding a bank account has a form here (`bank.add`) that the web does not have;
  posting from a statement line always posts (engine behaviour; no draft option).
- Claims: trail times show in UTC.
- Fonts: Archivo and IBM Plex Mono (`DESIGN.md`) are not bundled; text renders in the system
  font.
- Click flows are proven by unit tests and keyboard runs only: Hyprland has no synthetic click.
  Scroll smoothness was measured on release builds only (50,000 lines, p95 about 16.9 ms a
  frame on a quiet machine).

### Engine (from `parity.md`; no fixture reaches any of these)

- Inputs are typed, so a value of the wrong shape is refused where TS stores it: a malformed
  date or a fractional amount in a draft, a string `fx_rate`, a non-boolean `submit`, a
  malformed statement date. Extra input fields are dropped; the typed `AccountPatch` drops patch
  keys TS keeps.
- A command that does not fit its type is refused as `INVALID_INPUT` ("The <type> command is
  malformed: ...") where TS passes it on to the engine.
- A refused write changes nothing and draws no id; on two TS throw-after-draw paths TS has
  already drawn ids. `importStatement` checks every amount before drawing the batch id.
- A new budget line with `filter` left out stores `null` where TS leaves the key absent.
- Where TS throws (`RangeError`, `TypeError`, `NOT_FOUND`) Rust returns a `Result`; `dayBefore`,
  `grantSummary` and `parseStatementDate` have edge cases listed in the table.
- No wall clock or random ids in the core: the module injects UUIDs and the reducer time, the
  desktop a fixed demo env or the system clock.

### Module and client (from ADR 0002 and ADR 0003)

- Tables keep the TS module's row shape (typed journal rows plus JSON doc rows) because the
  SpacetimeDB 2.12 derive cannot handle ledger-core's tuple newtypes (ADR 0003).
- Rust-only reducers: `import_books` (loads whole books, operator only) and `remove_operator`.
- The client does not use the SDK's `run_threaded` (it panics on an unclean disconnect); its own
  loop handles a lost connection, but that path has no automated test.
- The wasm is published without `wasm-opt` (not installed on the build machine): 2.1 MiB.

### Fixtures (from `../fixtures/README.md`, "Not covered here")

- `spacetime-codec.ts`, `spacetime-backend.ts` and `backend.ts` are not fixture-exported; the
  module replay and integration runs cover them end to end instead.
- Engine error paths that no TS test asserts are not catalogued, except the sanitiser's.
- The bun fixture check (`rust/tools`, `bun run check`) is not in CI: a TS engine change can
  drift from the fixtures without a red build.

### Packaging

- Archives are unsigned. macOS Gatekeeper asks on first start; nothing is notarised.
- Linux archives need the system's Vulkan driver and GPUI's shared libraries (INSTALL.txt);
  there is no AppImage, Flatpak or .deb, and no desktop file or icon.
- Windows is not built or tested.

## Next steps for a maintainer

1. Production `flowtrace-ledger` is published (once, by the coordinator on the user's explicit
   instruction). Next there: open the first organisation and add its members, and decide which
   account should be the operator (`module-release.md`, section 4). Writes on production are
   the user's step.
2. Add the bun fixture check to `.github/workflows/rust.yml` so TS engine changes that move a
   fixture fail CI.
3. Give the desktop the organisation and member calls (`ledger-client` first, with a test, as
   pre-decision P7 asks), so a new organisation can start without the CLI.
4. Store proof files (receipts, bill attachments): needs a storage decision (the web used
   Supabase Storage, which is gone).
5. Bundle Archivo and IBM Plex Mono with the app.
6. Signing and notarisation for macOS, and a Linux package format, before handing archives to
   people outside the team.
7. Decide whether the transparency side of the web app (flows, trust score, campaigns) comes to
   the desktop or stays web-only.
8. A test for the client's unclean-disconnect path.
