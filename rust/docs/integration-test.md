# Integration test v1 (IT1)

`rust/tools` -> `bun run it1` runs two people on the web app and on the desktop against one
fresh organisation, and requires the five financial reports to come out the same three ways. It
is phase 2 task IT1 (pre-decision P24); IT2 schedules it nightly. Code: `rust/tools/it1.ts`, its
helpers `rust/tools/src/it1-*.ts`, and the desktop-side probe
`rust/crates/desktop/examples/it1_probe.rs`.

## What it proves

One run, in order, every step timed:

| Step | Who and where | What |
|---|---|---|
| a | operator | Opens `it1-<stamp>` (never `web1-20261016`), loads the demo books (`rust/fixtures/demo-books.json#/state`, `import_books`) and adds Alice (post, approve, close), Bob (post, approve) and Carol (none) |
| b1-b5 | Alice, web | Adds account 7460, posts a manual entry (7460 Dr, 1110 Cr, 1,500.00), submits a claim (5700, 3,450.00), enters a bill (7100, 18,200.00), imports the statement fixture into the M-Pesa till |
| c1 | Bob, desktop | The desktop binary opens the books on Bob's token file (`--timings on`) |
| c2-c4 | Bob, desktop | Approves Alice's claim, approves and posts the bill, runs Auto-match on the till |
| d1 | Bob, web | The claim page says "Approved and posted" |
| d2 | Alice, web | Pays the bill from 1110; the page says "Paid in full" |
| e1, e2 | Carol, web and desktop | Posts an entry; both refused with the module's reason ("Carol does not have permission to post to the ledger."), books unchanged |
| e3 | Alice, HTTP | One request id sent twice (below) |
| f1, f2 | Alice offline, then Bob on the desktop | A claim saved with no network and sent on reconnecting (`captured_offline`), then approved |
| g1-g4 | everyone | The five reports read three ways and compared byte for byte |
| h | | The summary table |

**Separation of duties across clients.** Alice's claim and bill are approved by a second person
on another client; the engine refuses self-approval, so only a two-person run can show this.

**The three report readings.** The financial position, activities, functional expenses, cash
flow and trial balance, year to date as at the run's date, accrual basis:

1. **web**: the Reports screen's tables and footing checks, read from the DOM (Playwright);
2. **desktop**: `desktop::statements::statement` (what the Reports screen draws) on the books
   `ledger-client` rebuilds from the module rows as Bob (`it1_probe reports`);
3. **oracle**: every command the run sent, from either client, replayed through `ledger-core`
   from `rust/fixtures/demo-books.json` (the parity suite's starting books) with the module's
   own `parse_command` and `apply` (`it1_probe oracle`). It also checks that each command's
   outcome (accepted with the same result shape, or refused with the same issues) is the
   module's. Ids differ (the module draws UUIDs, the oracle `it1-oracle-0001`, ...), so the
   oracle learns the module's id for each record from the results and translates later
   commands.

Each reading is normalised (`src/it1-report.ts`): figures as integer minor units (`(1,234.50)`
is `-123450`, an en dash is `0`), object keys sorted, gap rows and empty tables dropped, one row
per line. Any difference fails step g4 and prints a line diff; the three texts and the diff stay
in the run folder.

**Where the web's commands come from.** The browser's `run_command` calls and their
`my_command_result` rows are read off the WebSocket frames (`src/it1-frames.ts`: BSATN strings,
gzip from the server), so the oracle replays exactly what the web sent, not what the script
meant to send.

**A repeated request id is not a no-op.** The TS module, the ground truth, stores each result
under the caller's request id and dedupes nothing (`spacetimedb/src/index.ts:658-687`); the Rust
module does the same. Step e3 checks that behaviour: two calls with one id leave one result row,
overwritten, and the command runs twice (two vendors). The IT1 brief expected an idempotent
no-op; the higher source wins.

## Run it by hand

Once per worktree: the release desktop and probe, the tools and the web app's packages.

```sh
cd rust && cargo build --release --locked -p desktop --bin desktop --example it1_probe
cd tools && ~/.bun/bin/bun install --frozen-lockfile
cd ../../flowtrace && ~/.bun/bin/bun install --frozen-lockfile
```

On maincloud dev, which must carry main's module (step 0 checks for the `trusted_issuer` table and
stops otherwise). The CLI login must be an operator of the database (its publisher is):

```sh
cd rust/tools
~/.bun/bin/bun run it1 --url https://maincloud.spacetimedb.com --db flowtrace-ledger-dev --offline-emulated
```

On the local server instead (needs the wasm, `cargo build --release --locked --target
wasm32-unknown-unknown -p ledger-module`); the run publishes `ft-it1-<stamp>` and deletes it at
the end unless `--keep`:

```sh
~/.bun/bin/bun run it1 --local --offline-emulated
```

| Flag | Meaning |
|---|---|
| `--url`, `--db` | The server and database; required unless `--local`. The database name must end in `-dev`; `flowtrace-ledger` (production) and anything else is refused |
| `--local` | The local server (`http://127.0.0.1:3100`) and a database of the run's own |
| `--keep` | With `--local`: keep the database |
| `--auth dummy\|workos` | Default `dummy` (device identities the run mints); `workos` (Alice and Bob sign in with WorkOS) is below |
| `--offline-emulated` | Step f in a Playwright context set offline, then online |
| `--phone` | Step f on the user's Android phone (below); excludes `--offline-emulated` |
| `--web-url` | Default `http://localhost:3000`. If nothing answers there, the run starts `vite dev` itself on the run's database and organisation, and stops it at the end. An app already running there must use `VITE_LEDGER_BACKEND=spacetimedb` on the same database |
| `--desktop` | The desktop binary; default `rust/target/release/desktop` |
| `--headed` | Show the browser |

The browser is the newest Chromium Playwright cached (`~/.cache/ms-playwright/chromium-*`) or
`IT1_CHROMIUM`. The desktop needs a Wayland session (`WAYLAND_DISPLAY`, default `wayland-1`); it
runs only until it shows the books, and its `--timings on` lines are kept. Exit status 0 is a
pass, 1 a failed step, 2 a refused command line.

Everything goes to `rust/out/it1/<stamp>/` (gitignored): `transcript.log`, `summary.md`,
`shots/` (the web after each step and each report), `desktop-bob.log`, `command-log.json`,
`reports-{web-dom,desktop,oracle}.json`, `normalised-{web,desktop,oracle}.txt`, and
`report-diff.txt` on a difference. `tokens/` holds the run's three device tokens (mode 0600);
never copy it anywhere. The transcript never holds a token.

## The phone step

`--phone` replaces the emulated offline claim with the user's Android phone, once by hand. The
web app must be reachable from the phone, so this works against maincloud dev (the phone cannot
reach a local SpacetimeDB on 127.0.0.1) with the run's own dev server, which it then starts with
`--host`. The run prints these steps and waits:

1. On the phone, on the same network, open `http://<this machine's address>:3000/spend/claims/new`
   in Chrome. The Books screens say "The books are not connected yet" and show the phone's
   identity.
2. Type that identity at the run's prompt. The run adds it as a member "Alice (phone)" (user id
   `it1-alice-phone`, post only: one user id cannot hold two identities, so the phone cannot be
   Alice's own member row in dummy mode; with WorkOS it would sign in as Alice).
3. Turn on aeroplane mode, submit a claim (kind of cost 5700), see "Saved on this phone".
4. Turn aeroplane mode off and keep the page open.

The run waits up to 15 minutes for a claim from `it1-alice-phone` marked `captured_offline`,
rebuilds its `claim.create` from the claim row for the oracle (the phone is no Playwright page),
and Bob approves it on the desktop.

## WorkOS mode

`--auth workos` runs the same steps with Alice and Bob as WorkOS users; the code is
`../tools/src/it1-workos.ts`. It needs these values, untracked and never committed (both files
are gitignored):

- `rust/tools/.env.it1`: `WORKOS_CLIENT_ID` (the sandbox environment) and, for Alice and Bob,
  `IT1_WORKOS_<ALICE|BOB>_ID` (the WorkOS user id), `_EMAIL` and `_PASSWORD`;
- `flowtrace/.env.local` per AUTH1 (`flowtrace/README.md`, "Signing in with WorkOS against dev"):
  the same `WORKOS_CLIENT_ID`, `WORKOS_API_KEY`, `WORKOS_COOKIE_PASSWORD` and
  `WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback`.

If anything is missing, the run names the missing keys (never a value) and writes nothing. In the
WorkOS dashboard the environment needs the redirects `http://localhost:3000/api/auth/callback` (web)
and `http://127.0.0.1:*/callback` (desktop), and email and password sign-in. **The test users'
email domain must not be claimed by an organisation with an SSO connection.** The sandbox's
"Test Organization" claims `example.com`, so users on that domain are sent to the test identity
provider instead of a password page (a password sign-in answers `sso_required`). The run says so
in plain words.

**Trust first (an operator, once per database).** The run looks for a `trusted_issuer` row whose
audience is the client id. Without one, preflight signs Alice in on the web, and the Books
screens' refusal names the issuer and audience to register. The run then stops with:

```sh
~/.local/bin/spacetime call -s maincloud flowtrace-ledger-dev add_trusted_issuer '"<issuer>"' '"<audience>"' '"WorkOS staging"'
```

**What changes in the steps.**

| Step | Workos mode |
|---|---|
| 0 | the web app runs under `bun --bun` (below), with a second app in dummy mode on port 3001 for Carol |
| a | Alice and Bob join by subject (`add_member_by_subject` at the trusted issuer, their WorkOS user ids as user ids); Carol is a device identity, as she has no WorkOS user |
| w1 | web, Alice signs in on the hosted AuthKit page: email, "Continue with email", password, Continue; back on the app, the books load as her |
| c1 | desktop, Bob: "Sign in with WorkOS" (`--sign-in on`). An `xdg-open` wrapper first on PATH hands the AuthKit URL to the run, which signs Bob in on it in its own browser context. AuthKit sends that browser to the desktop's loopback callback, and the desktop connects as Bob. The desktop is then started again without the flag: it trades the refresh token it saved and opens the books as Bob |
| w2 | web, Bob signs in on AuthKit before step d1 |
| e1 | Carol is refused on the dummy-mode app (port 3001) |
| other steps | as in dummy mode |

The run's own reads and the probe's ledger-client calls as Alice or Bob use an access token from
WorkOS's password authentication (`grant_type=password` with the API key). That is the same
environment, so the issuer, subject and identity are the same as their browser sign-ins. Each
token is checked against the issuer and subject the person joined with, and renewed when it has
under 90 s left. The tokens stay in the run's `tokens/` folder (mode 0600) and never reach the
transcript.

**Why `bun --bun`.** Under Node 26, Nitro's dev worker (nitro 3.0.1-alpha.1, srvx 0.10.1) delivers
every response without its headers. Dummy mode does not notice, but the web's sign-in redirect
arrives as a bare 307 with no Location, and the browser shows a blank page. Under Bun's runtime,
the same request carries its Location. The production build is not affected by this dev-only
problem.

## Nightly (IT2)

The command IT2 should schedule, on a runner with the SpacetimeDB CLI logged in as an operator
of dev (a CI secret), a Wayland session for the desktop or `--desktop` pointed at a headless
build, and Playwright's Chromium:

```sh
cd rust/tools && bun run it1 --url https://maincloud.spacetimedb.com --db flowtrace-ledger-dev --offline-emulated
```

On a runner without the dev login, `bun run it1 --local --offline-emulated` against a SpacetimeDB
server started in the job proves the same, except that dev carries what is published there. The
offline unit tests (`src/it1-*.test.ts`: flags and the database guard, the normaliser and the
three-way comparison including a one-minor-unit difference, the frame decoder on real frames)
already run in the `fixtures` job's `bun test`.

## What a run leaves on dev

One organisation `it1-<stamp>` per run, with the demo books (about 450 rows), the run's postings
and four members (the run's operator, withdrawn from the operator table at the end, stays a
member of its organisation). In workos mode Alice's and Bob's member rows are by subject, and
the desktop's WorkOS refresh token stays in the run's gitignored `tokens/` folder. Nothing else
changes, and nothing is ever written to production. The
run never publishes, deletes or renames on maincloud: off the local server it goes through
`remoteServer` (`src/module-server.ts`), which allows only `call` and `sql`. While fiscal year
2026 is the demo books' only year, a run in a later year opens its own year first (`year.open`,
logged for the oracle).
