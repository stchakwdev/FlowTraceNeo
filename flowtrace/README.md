# TanStack Start + shadcn/ui

This is a template for a new TanStack Start project with React, TypeScript, and shadcn/ui.

## Running the Books screens on SpacetimeDB

By default the Books screens (Spend, Grants, Ledger, Reconcile, Reports) run on the bundled
demo books, which reset on reload. To run them on the Rust ledger module instead, put this in
an untracked `.env.local` (never commit it):

```sh
VITE_LEDGER_BACKEND=spacetimedb
VITE_SPACETIMEDB_URI=wss://maincloud.spacetimedb.com
VITE_SPACETIMEDB_MODULE=flowtrace-ledger-dev
# Optional: which organisation's books to open. Unset, the app opens the first
# organisation this browser's identity is a member of.
# VITE_SPACETIMEDB_ORG=org-example
```

Then `bun run dev`. This works in dummy auth mode as well as with WorkOS (below). In dummy
mode the browser keeps its SpacetimeDB identity in local storage
(`flowtrace.spacetimedb.token`), so each browser profile is one identity. A new identity belongs to no organisation: the Books screens say
"The books are not connected yet" and show the identity (`0x` and 64 hex digits). An
operator opens books and adds the identity as a member, for example:

```sh
S=~/.local/bin/spacetime
$S call -s maincloud flowtrace-ledger-dev create_org_ledger '"org-example"' '"KES"' 2026 '"Ama Owner"'
$S call -s maincloud flowtrace-ledger-dev add_member '"org-example"' '"0x<identity>"' '"ama"' '"Ama Owner"' true true true
```

The screens pick the new membership up without a reload. The member row is the actor: the
user id, name and the post, approve and close permissions given here are what the module
checks on every command, so approving a claim or a bill needs a second identity. The full
access rules are in `rust/docs/module-release.md` section 3.

### Signing in with WorkOS against dev

In workos auth mode the browser connects to SpacetimeDB as the signed-in person, not as a
device: it presents the WorkOS AuthKit access token, so one WorkOS user is one SpacetimeDB
identity on every browser (and on the desktop once it signs in the same way). The token is never
written to local storage and is fetched fresh for each connection, so AuthKit's refresh keeps a
long session working. Dummy mode keeps the device identity above. Put this in `.env.local`
(untracked; never commit it) next to the SpacetimeDB lines:

```sh
VITE_AUTH_MODE=workos
WORKOS_CLIENT_ID=client_...
WORKOS_API_KEY=sk_...
WORKOS_COOKIE_PASSWORD=<at least 32 characters>
WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback
```

The database must trust the WorkOS issuer first. Until an operator registers it, the Books
screens say "SpacetimeDB refused this sign-in: an operator must trust issuer ... for audience ...
on this database" with the two values to register; the commands are in
`rust/docs/module-release.md` section 5 (`add_trusted_issuer`, then `add_member_by_subject` with
the WorkOS user id, or `add_member` with the identity the Books screens show once connected).

With WorkOS, the books are `VITE_SPACETIMEDB_ORG` when set, else the WorkOS organisation when the
person is a member of a ledger organisation with that id, else the first organisation they are a
member of. The masthead names the WorkOS user and the books' organisation: its profile name in
the module (`my_organization_profile`, from MOD1), or its id when it has no profile.

### Campaigns, donations and the organisation profile

With `VITE_LEDGER_BACKEND=spacetimedb` the module also serves campaigns (list, detail, create,
status changes, delete of drafts, figures, counts, the campaign report), donations and donor
updates, the organisation profile in Settings, and the signed-in person
(`src/lib/ledger/spacetime-transparency.ts`, selected in `src/lib/data.ts`). The organisation and
role come from this identity's member row: a member who can close periods is an admin. With no
membership the app shows only the not-connected notice with the identity; nothing is created for
it. Without the setting the demo dataset serves these screens; with no backend at all (WorkOS and
no `VITE_LEDGER_BACKEND`) they say the books are not connected, as Supabase is gone.

The module keeps money in minor units; the web converts at the boundary. Every gift starts
pending: on the campaign page a member presses "Confirm received" once the money has arrived,
and only confirmed gifts count towards the raised amount and the figures (as in the demo).

The public pages (`/donate/<link>`, `/embed/campaign/<id>`, `/badge/trust/<org>`) read the
module's public tables `public_campaign`, `public_organization` and `public_badge` over HTTP SQL
(`POST https://maincloud.spacetimedb.com/v1/database/<name>/sql`, no Authorization header;
`src/lib/ledger/public-sql.ts`). A visitor opens no WebSocket and gets no identity until they
press Give, which records a pending gift as a device identity (dummy mode). A campaign is public
once it is active, marked public and has a link. The organisation and its badge are public once
its profile has `is_public` set; Settings has no switch for it yet, so an operator or closing
member sets it:

```sh
$S call -s maincloud flowtrace-ledger-dev update_organization_profile '"org-example"' '"{\"is_public\":true}"'
```

The client bindings in `src/lib/ledger/spacetime-bindings/` are generated from the Rust
module. After a module change:

```sh
cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
cd ../flowtrace && rm -rf src/lib/ledger/spacetime-bindings
~/.local/bin/spacetime generate --lang typescript \
  -b ../rust/target/wasm32-unknown-unknown/release/ledger_module.wasm \
  -o src/lib/ledger/spacetime-bindings -y
bunx prettier --write src/lib/ledger/spacetime-bindings
```

## Proof files

Receipts, photos and PDFs attached to transactions are stored by the Cloudflare Worker in
`workers/proofs`, which keeps them in R2. Point the app at it in `.env.local`:

```sh
VITE_PROOFS_URL=http://127.0.0.1:8787   # `wrangler dev` in workers/proofs; no trailing slash
```

The Worker takes images (JPEG, PNG, WebP, GIF, HEIC) and PDFs up to 25 MB. Each request
carries `Authorization: Bearer <token>`: in dummy auth mode that is this browser's
SpacetimeDB token (`flowtrace.spacetimedb.token`, so connect to the books first), with WorkOS
it is the AuthKit access token. A proof record stores `proof:<key>` in `file_url` (and the
thumbnail's key in `thumbnail_url`); the viewer trades it for a signed URL that lasts 10
minutes. The client code is `src/lib/proofs.ts`.

With `VITE_PROOFS_URL` unset, chosen files stay in the browser session only (blob URLs), so
the demo runs with no backend. Proof files are no longer sent to Supabase Storage.
