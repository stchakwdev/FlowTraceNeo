# Agent primer: the whole setup in one read

This file is for an agent (or a person) picking up FlowTrace. It says where everything is, what
is fixed, how the multi-agent tooling is wired, and what the first pieces of work are. Read it
before touching anything. Last updated 2026-10-16.

## 1. What this project is

FlowTrace is a financial-transparency and bookkeeping product for NGOs in African markets. The web
app in `flowtrace/` (TanStack Start, React, on Vercel) stays as the public demo and is not changed
by this run. This run builds FlowTrace Books as a native desktop ERP in Rust.

Goal: build FlowTrace Books as a native Rust desktop ERP (GPUI with gpui-kit) on a Rust SpacetimeDB
module hosted on maincloud, with a shared Rust ledger core at parity with the TypeScript ledger
engine from PR #8 (merged 2026-10-16 as 54b4894).

Who it is for: finance officers at small NGOs who keep books, spend claims, grants and bank
reconciliation in one place over poor connectivity. The binding constraint is ledger correctness:
the books must balance and match the TS engine; UI scope bends before correctness does.

Layout to build, all under `rust/` (one Cargo workspace):

| Crate | What | Source of truth |
|---|---|---|
| `ledger-core` | money, types, engine, commands, reports, reconcile, importers, demo books; no I/O; ids and time through an `Env` trait | `flowtrace/src/lib/ledger/*.ts`, `flowtrace/src/lib/demo/ledger.ts` |
| `ledger-module` | SpacetimeDB 2.12 server module (wasm): private tables, `my_*` views, `run_command` | `spacetimedb/src/index.ts` |
| `ledger-client` | SpacetimeDB Rust client SDK: connect, subscribe, rebuild state, submit commands | `flowtrace/src/lib/ledger/spacetime-backend.ts` |
| `desktop` | GPUI app on gpui-kit 0.7.3 (`gpui-pre =0.3.9`); screens behind a `LedgerBackend` trait | `flowtrace/src/routes/_authenticated/{ledger,spend,grants,reconcile,reports}`, `DESIGN.md` |
| `tools/`, `fixtures/` | fixture exporter (bun or vitest) over the TS engine, and the JSON fixtures | |

Success means: cargo fmt --check, clippy -D warnings and cargo test green on the rust/ workspace; the Rust ledger core reproduces every TS parity fixture (demo books, command results, trial balance and statements) exactly and passes ported property tests; the Rust SpacetimeDB module runs the demo books end to end on a local server and on a maincloud dev database; the GPUI desktop app on Linux (Omarchy) and macOS covers chart of accounts, periods, journal and general ledger, spend claims, bills and payments, grants and budgets with budget-vs-actual, bank reconciliation with statement import, and financial statements with CSV export, all reading and writing through live SpacetimeDB; screenshots of every screen under rust/docs/screens/

Environment on Omarchy:

- Rust 1.99 (`/usr/bin/cargo`); bun at `~/.bun/bin/bun`; spacetime CLI 2.12 at
  `~/.local/bin/spacetime` (not on the non-interactive PATH; use the full path).
- Local SpacetimeDB server: systemd user service `spacetimedb-local`, `http://127.0.0.1:3100`, CLI
  nickname `local`. Threads publish only there (`spacetime publish -s local <name>`) with database
  names that carry the task id (for example `ft-s2-test`), and delete what they create. Never stop
  or restart the service; ask the coordinator. Port 3000 is the web app dev server.
- Maincloud: only the coordinator publishes, only at gates, only to the dev database (default
  `flowtrace-ledger-dev`). The production name and any reset are the user's.
- GPUI on Linux needs Wayland (the session is Hyprland) or X11, and Vulkan (AMD Radeon). A thread
  that needs a window runs it in the user's graphical session (`WAYLAND_DISPLAY` set) and saves a
  screenshot with `grim` under `rust/docs/screens/`.
- Commit identity: `git -c user.name=stchakwdev -c user.email=stchakwera@outlook.com commit ...`.
- Merging to `main` redeploys the web app on Vercel; Rust-only merges leave it unchanged.
- `.claude/skills/rust-best-practices/` (vendored from stolinski/s-stack at 3cc9e7c) is the Rust
  style authority: no `unwrap`/`expect` outside tests, `thiserror` in libraries, `anyhow` only in
  binaries, borrow over clone, `#[expect]` with a reason over `#[allow]`.

## 2. Machines and locations

| | omarchy (primary, always on) | mac (mirror) |
|---|---|---|
| Reach | `ssh omarchy` | local |
| Workspace | `/home/tim0/Projects/FlowTrace/V2` | not mirrored: `~/Playground/FlowTrace/V2` on the Mac is stale; the Mac check (Q3) uses a fresh clone |
| Read-only evidence | /home/tim0/Projects/FlowTrace/V2/flowtrace/src/lib/ledger (TS engine, read-only); /home/tim0/Projects/FlowTrace/V2/spacetimedb/src/index.ts (TS module, read-only); https://gpui-kit.com/docs/; https://spacetimedb.com/docs/quickstarts/rust/; https://github.com/stolinski/gpui-agent (reference only) | |

Do the real work on the machine named in the run (`omarchy`); the other is for reading and
reviewing. Do not edit docs on the mirror while threads are running.

## 3. Read in this order

1. `PRIMER.md`
2. `CLAUDE.md`
3. `.claude/skills/rust-best-practices/SKILL.md`
4. `Docs/flowtrace-ledger-plan.md`
5. `Docs/adr/0001-native-ledger.md`
6. `flowtrace/src/lib/ledger/engine.ts`
7. `spacetimedb/src/index.ts`
8. `DESIGN.md`

Narrow it per task: ledger-core threads read the matching TS file and its tests; module and client
threads read `spacetimedb/src/index.ts` and `flowtrace/src/lib/ledger/spacetime-backend.ts`; UI
threads read the matching route files, `DESIGN.md`, the gpui-kit docs and the ADR 0002 that F3
writes. `CLAUDE.md` describes the web app; for `rust/` this primer and the skill win.

## 4. Non-negotiables

- Ground truth order: TS ledger engine flowtrace/src/lib/ledger/ and its tests (on main after PR #8) > Docs/flowtrace-ledger-plan.md and Docs/adr/0001-native-ledger.md > Docs/flowtrace-prd.md > other docs.
- Never modify flowtrace/ (the web app; read-only for threads except a fixture-export script under rust/tools that imports it); Docs/adr/0001-native-ledger.md (new ADRs get new numbers); .infisical-envfiles and any .env file. Never write into the evidence paths.
- Tests that encode the acceptance criterion come before the implementation.
- Checks before a thread reports: cd rust && cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked.
- Every mirrored rule cites its source in the doc comment (flowtrace/src/lib/ledger/<file>.ts:line for behaviour claims; Docs/flowtrace-ledger-plan.md section for domain claims; a docs URL for gpui-kit or SpacetimeDB API claims).
- No emojis in code, UI or docs. Commit messages carry no AI co-authorship trailer.
- The default branch is `main`: threads branch from `origin/main` and open pull
  requests against it. The user decides: merging any PR that touches flowtrace/, Vercel settings or env, or anything outside rust/, Docs/ and the rust CI workflow; the maincloud production database name and any publish or data reset on it; WorkOS or other auth providers; spending beyond the plan; profile or safety changes. The coordinator merges clean PRs that touch only rust/, Docs/ and the rust CI workflow under .github/workflows/, starts planned tasks, publishes to the maincloud dev database at gates, and takes the recommended option on questions it cannot ask.

## 5. Model policy and agent profiles

Fixed by the user. The herdr-projects profiles encode this and the allow-lists refuse anything
else (`~/.config/herdr-projects/config.toml` on both machines).

| Profile | Model | Use for |
|---|---|---|
| `coordinator` | claude-fable-5-1 | The project coordinator. Plans, delegates, reads reports. Never does the work. |
| `worker` | claude-opus-5-5 | Bounded implementation, tests, docs. The default thread profile. |
| `deep` | claude-opus-5-5, effort xhigh | Hard correctness bars, architecture, long refactors. |
| `light` | claude-sonnet-5-5 | Renames, typo fixes, small lookups only. |

## 6. The multi-agent stack

Everything runs inside herdr (a terminal workspace manager for coding agents). Three plugins are
installed and configured on both machines.

### herdr-projects: coordinator and threads

- The project is `flowtrace`, home folder `/home/tim0/.herdr-projects/flowtrace`. `PROJECT.md` holds the goal,
  settings and standing instructions; `TASKS.md`, `MEMORY.md` and `memory/` belong to the
  coordinator; `threads/` holds thread records, briefs and reports; `uploads/common-rules.md`
  is the rules block for every brief.
- The coordinator's command prefix (also printed by `context`): `/home/tim0/.config/herdr/plugins/github/herdr-projects-b1278ffb803c/target/release/herdr-projects --root /home/tim0/.herdr-projects`
- Each thread is a separate agent in its own git worktree on branch `hp/flowtrace/<id>-<title>`,
  briefed with the goal, the standing instructions, the memory and its task.
- Settings: `max_parallel_threads = 3`, `start_threads = propose` (the
  coordinator proposes, the user says go), `trust_screens = coordinator`. Change them in the
  popup (`prefix+t`) or with `herdr-projects safety ...` at a terminal; agents may not.
- The coordinator's read-only subcommands are allow-listed in `/home/tim0/.herdr-projects/flowtrace/.claude/settings.local.json`.

### herdr-dagr: the live run graph

- The run file is `/home/tim0/.herdr-projects/flowtrace/.dagr/run.json`, seeded with the work plan in section 8. The
  coordinator owns it: write `run.json.tmp`, `dagr check run.json.tmp --strict --json` must print
  `[]`, then rename over `run.json`. Attempts and events are append-only. Every terminal outcome
  carries an evidence tier.
- The dagr pane reads `.dagr/run.json` under the cwd of the pane that has focus when you press
  `prefix+f`. Press it with the coordinator's pane focused (workspace `FlowTrace`), or in the
  repo's workspace (the repo has a gitignored link to the same file). From a thread's worktree
  it finds nothing. Anywhere: `scripts/dagr.sh runs/<run>` in the ops folder opens it with the
  path set. The coordinator sets `run.orchestrator.pane` to its own `$HERDR_PANE_ID` on its first write.

### herdr web ui: phone and browser control

| Machine | Address (tailnet only) |
|---|---|
| omarchy | https://omarchy.tail3b280b.ts.net |
| mac | https://samuels-macbook-pro-16-max-1.tail3b280b.ts.net |

Your own Tailscale devices get in with no code. On the phone: open the address, add it to the
home screen, then the menu, Alerts, for push alerts. Health: `curl -s http://127.0.0.1:7317/api/health`.

## 7. Starting and resuming

```
herdr-projects doctor                 # every line [ok]; the ticker must be running
herdr-projects ticker start           # if it is not
herdr-projects open flowtrace          # starts or jumps to the coordinator
herdr-projects context flowtrace       # the digest the coordinator reads every turn
```

The coordinator's first turn on a new project is fixed by its skill: restate the goal, list the
repos and machines, ask for the first piece of work. Say "start <ids>" from the plan below.

## 8. Work plan (the seeded DAG)

Approved by the user on 2026-10-16T01:11:12Z after a shaping walkthrough; the pre-decisions that bind
the coordinator are in the project's `PROJECT.md`. Do not re-plan the approved tasks.

| Id | Task | Depends on | Profile | Risk |
|---|---|---|---|---|
| Q1 | question: merge PR #8 before launch |  | user | low |
| O1 | ops: SpacetimeDB 2.12 CLI and a local server on Omarchy |  | operator | low |
| F1 | rust/ workspace skeleton, toolchain, lints and CI | Q1 | worker | low |
| F2 | parity fixtures exported from the TS engine | Q1 | deep | unknown |
| F3 | GPUI and SpacetimeDB spike on Omarchy | F1,O1 | deep | unknown |
| G1 | gate: foundations green | F1,F2,F3 | coordinator | low |
| C1 | ledger-core: money, ids, types, errors | G1 | worker | low |
| C2 | ledger-core: posting engine (accounts, funds, periods, entries, reversals) | C1 | deep | long |
| C3 | ledger-core: grants, budgets, AP and expense claims | C2 | deep | long |
| C4 | ledger-core: commands (sanitize and apply) | C3 | worker | low |
| C5 | ledger-core: reports | C4 | deep | unknown |
| C6 | ledger-core: reconcile, importers, CSV, CoA template, setup, bridge, demo books | C4 | worker | low |
| C7 | ledger-core: property tests | C4 | deep | low |
| G2 | gate: core parity | C5,C6,C7 | coordinator | low |
| S1 | ledger-module: Rust SpacetimeDB module scaffold (tables, views, auth reducers) | C1 | deep | low |
| S4 | ledger-module: run_command reducer on ledger-core | S1,C4 | deep | unknown |
| S2 | ledger-module: integration tests on the local server | S4,G2 | worker | low |
| S3 | ledger-client crate: connection, cache, commands | S4 | worker | unknown |
| Q2 | question: maincloud database names |  | user | external |
| G3 | gate: SpacetimeDB end to end | S2,S3,Q2 | coordinator | low |
| U1 | desktop: app shell, navigation, connection and theme | G2,F3 | worker | low |
| U2 | desktop: chart of accounts and periods | U1 | worker | low |
| U3 | desktop: journal and manual entry | U1 | worker | long |
| U4 | desktop: general ledger | U1 | worker | low |
| U5 | desktop: spend claims | U1 | worker | low |
| U6 | desktop: bills, payments and vendors | U1 | worker | low |
| U7 | desktop: grants and budgets | U1 | worker | low |
| U8 | desktop: bank reconciliation | U1 | worker | low |
| U9 | desktop: financial statements and export | U1 | worker | low |
| U10 | desktop: SpacetimeDB backend behind LedgerBackend | U1,G3 | worker | unknown |
| Q3 | question: macOS build and run check | U1 | operator | external |
| G4 | gate: Books end to end | U2,U3,U4,U5,U6,U7,U8,U9,Q3,U10 | coordinator | low |
| R1 | release: packaging, README and handoff | G4 | worker | low |

Tasks with a flagged risk and their fallback (the full pre-decisions, with triggers, are in `PROJECT.md`):

| Task | Risk | Fallback |
|---|---|---|
| F2 | unknown | if a TS module cannot run outside Vite (import.meta, path aliases), run the export through vitest as a test file under rust/tools with its own config; never edit flowtrace/ |
| F3 | unknown | if gpui-kit fails on Wayland, try the x11 feature under XWayland and record it; if gpui-kit itself blocks, stop and raise the fork to the user: plain gpui-pre 0.3.9 with hand-built tables, or wait for a fix |
| C2 | long | split at reversals and period close if it runs past 90 minutes |
| C3 | long | split AP (vendors, bills, payments) from grants and claims if over 90 minutes |
| C5 | unknown | where TS output depends on float or locale formatting, compare normalised minor units and record the difference in rust/fixtures/README.md |
| S4 | unknown | inject ids and time through an Env trait in ledger-core (as LedgerEnv in TS) and put wasm-unfriendly dependencies behind a feature; never fork or copy the engine into the module |
| S3 | unknown | if subscription rows do not map back to engine state one to one, keep a client-side mapping layer with its own tests and record the shape in ADR 0002 |
| Q2 | external | see pre-decisions |
| U3 | long | split the entry form from list and detail if over 90 minutes |
| U10 | unknown | if a screen needs data the client cache does not hold, add it to ledger-client with a test rather than special-casing the screen |
| Q3 | external | see pre-decisions |

## 9. What every thread task must contain

The coordinator writes the task for an agent that has not seen the conversation. Include:

- The scope (one module, crate or feature), the docs to read (section 3, narrowed), and the
  artefact or check that defines done.
- The exact acceptance command.
- The common rules block from `uploads/common-rules.md`.
- What to put under `## Remember` in the report: a durable lesson, not a status.

Threads report with `## Report`, `## Next` and `## Remember`; the coordinator folds `## Remember`
into project memory and never pastes it.

## 10. Troubleshooting

- `doctor` says the ticker is not running: `herdr-projects ticker start`.
- A new thread sits idle with no brief for over a minute: the coordinator runs `thread brief`.
- Web UI down: `herdr plugin action invoke start --plugin devswha.herdr-web-ui`, then the health check.
- dagr pane blank: it looks for `.dagr/run.json` under the workspace cwd; `dagr check <file> --strict` names the problem.
- Herdr itself: `herdr status`; never `herdr server stop` while agents run.
