# Releasing the ledger module

How `crates/ledger-module` (the SpacetimeDB 2.12 server module) is built, how the maincloud dev
and production databases were published, how people get access to them, and what a later
production publish needs.
Day-to-day commands against the local server are in `../crates/ledger-module/README.md`; this
page is about maincloud.

Rule for agents and scripts: **nothing here is run against maincloud by an agent.** Publishing,
deleting or renaming a maincloud database is the user's step, at a terminal logged in to
SpacetimeDB through the web. Agents publish only to the local server, under a name carrying
their task id, and delete it afterwards. The one production publish so far was done by the
coordinator on the user's explicit instruction (section 4); that does not change this rule.

## 1. Build the wasm

From `rust/`, either command (both write
`target/wasm32-unknown-unknown/release/ledger_module.wasm`):

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
~/.local/bin/spacetime build -p crates/ledger-module
```

The `wasm32-unknown-unknown` target comes with the pinned toolchain (`rust-toolchain.toml`,
1.99.0); if it is missing, `rustup target add wasm32-unknown-unknown`. `spacetime build` looks
for `wasm-opt` afterwards and publishes unoptimised without it. Sizes and times are in the
module README (2.1 MiB unoptimised, about 11 s cold).

Before publishing anywhere, the module must pass the workspace checks and the local end-to-end
run (gate G3 in `PRIMER.md`):

```sh
cd rust
cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked
cd tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run integration
```

After any change to tables, views or reducers, regenerate the client bindings
(`../crates/ledger-client/README.md`, "Generated bindings") in the same change, or the desktop
reads the old shape.

## 2. How maincloud dev was published

The dev database is `flowtrace-ledger-dev` on maincloud
(`https://maincloud.spacetimedb.com`, dashboard <https://spacetimedb.com/flowtrace-ledger-dev>).
The user published it on 2026-10-16 at 16:24 UTC, from `rust/` on main, after
`spacetime login` (a browser login to their SpacetimeDB account):

```sh
spacetime publish -s maincloud flowtrace-ledger-dev -p rust/crates/ledger-module -y
```

(written from the repository root; `-p` builds the module first). The name was chosen by
pre-decision P8: dev is disposable. The module's `init` reducer made the publishing identity the
first operator (`src/reducers.rs`, `init`), so the CLI login that published it is the operator.

Thread t-0024 then checked the published database without publishing anything, through the
tools' remote mode (`../tools/README.md`, "Remote mode"): `integration` 20 of 20 checks,
`module-replay` 37 of 37 commands, and the client live test against it. Each of those runs
leaves a demo organisation in the database; three were there after that check.

Updating dev later is the same command after a merge to main. The CLI asks before a change that
breaks existing clients or would delete data; `-y` with no value answers yes to every prompt,
including the delete-data one, so for anything but a throwaway database leave `-y` out and read
the prompts. Never add `-c` / `--delete-data`.

## 3. Operators and members

Two levels of access, both kept in the module's private tables (`src/rules.rs`):

| Who | Can | Granted by |
|---|---|---|
| Operator | open books for a new organisation (`create_org_ledger`), load books (`import_books`), add and remove operators | `init` (the publisher), then `add_operator` by an existing operator |
| Member of an organisation | read that organisation's rows through the `my_*` views, `run_command` as that member; with `can_post`, `can_approve`, `can_close` as given | `create_org_ledger` makes the caller the owner (all three); then `add_member` (by identity) or `add_member_by_subject` (by a WorkOS user id, section 5) by a member who can close |

Before either, the connection itself must be let in: operators always are, device identities
while the database allows them, and signed-in people when their issuer is trusted (section 5).

A new identity (a desktop that just connected with a fresh token file) is a member of nothing
and sees no organisations. The Connection screen shows the first eight hex digits of its
identity; the full 64 digits are on stderr when the app runs with `--timings on`
(`desktop: status Connected { identity: "..." }`, `crates/desktop/src/store.rs`). The identity is
public; the token file next to it is not, and is never sent to anyone. Then, logged in as an
operator or a closing member, from any directory:

```sh
S=~/.local/bin/spacetime
DB=flowtrace-ledger-dev

# Open books for an organisation (operator): org id, base currency, fiscal year, your name.
$S call -s maincloud $DB create_org_ledger '"org-example"' '"KES"' 2026 '"Ama Owner"'

# Add a member (a member who can close): org, identity, user id, name, can_post, can_approve, can_close.
$S call -s maincloud $DB add_member '"org-example"' '"0x<64 hex digits>"' '"kofi"' '"Kofi Clerk"' true false false

# Remove a member; nobody can remove themselves or drop their own close permission.
$S call -s maincloud $DB remove_member '"org-example"' '"0x<64 hex digits>"'

# Operators (operator only); the last operator cannot be removed.
$S call -s maincloud $DB add_operator '"0x<64 hex digits>"'
$S call -s maincloud $DB remove_operator '"0x<64 hex digits>"'
```

Arguments are JSON: a string carries its own quotes and an identity is a `"0x..."` string. A
refusal exits 1 with the rule's message (`Error: Response text: ...`). `add_member` on an
existing member updates their permissions; a member's user id cannot change once given. The
module stores no organisation name: the desktop lists organisations by id.

These calls write to a shared database, so in the project they are the user's step on
maincloud, as publishing is. The same calls with `-s local` are how threads test.

## 4. Production: `flowtrace-ledger`

1. **Published.** The user named the production database `flowtrace-ledger` on 2026-10-16. It
   was published to maincloud on 2026-10-16 at 20:25 UTC from main `d65f3ba`, by the
   coordinator on the user's explicit instruction, done once (dashboard
   <https://spacetimedb.com/flowtrace-ledger>, database identity
   `c200b4b1041fce8318a0027f02f44cf0517c52599a8b10b4b629476899750b4b`). The module is the one
   published as dev; only `crates/ledger-module/README.md` changed between the two. `init` ran:
   the operator table holds one row, the publishing identity
   `c200a773ad0003c4f82fac12ada53ff076d83fec49a6d827149d813527431630` (the CLI login on the
   machine that published), and `ledger_org` was empty. Publishing, resetting or deleting
   production remains the user's call. The dev database stays `flowtrace-ledger-dev`.
   The desktop's built-in default is still `flowtrace-ledger-dev` (`DEFAULT_DATABASE` in
   `crates/desktop/src/model.rs`); release archives from `tools/release.sh` ship a `config.toml`
   naming `https://maincloud.spacetimedb.com` and `flowtrace-ledger` (no `token_file`), byte for
   byte `crates/desktop/release/config.toml`, which `crates/desktop/src/config.rs` tests opens
   that database. INSTALL.txt in the archive says where to copy it.
2. **The three decisions** in `handoff.md`, confirmed by the user on 2026-10-16. Two of them
   shape stored data: timestamps in milliseconds (every audit and engine timestamp written), and
   `float_roundtrip` (how stored rates are parsed). Changing either after real books exist means
   a migration.
3. **The operator.** The publisher became the only operator through `init`. To hand production
   to an account meant to own it (an organisation account, if the team should share it), that
   operator adds the new identity with `add_operator` (section 3); that is a write on
   production, so the user's step.
4. **No test data.** Never point `rust/tools` `integration` or `module-replay --existing`, or the
   client live test, at production: each run writes a demo organisation and leaves it there.
5. **A module update** is run by the user, not an agent: from a clean checkout of the commit to
   ship, after the checks in section 1, without `-y` so every prompt is read:

   ```sh
   spacetime publish -s maincloud flowtrace-ledger -p rust/crates/ledger-module
   ```

6. **Later publishes** update the schema of a database that holds real books. The CLI prompts
   before a change that breaks existing clients and before anything that would destroy data
   (`spacetime publish --help`, options `--break-clients`, `--delete-data`, `--yes`). Never use
   `--delete-data` on production; a breaking change needs a migration plan first.
7. **Backups and retention** of the maincloud database are not covered by anything in this
   repository yet.

## 5. Who may connect: trusted issuers and device identities

Since AUTH1 the module's `client_connected` reducer decides who connects
(`../crates/ledger-module/src/auth.rs`, `check_connection`). It runs for every WebSocket and every
HTTP reducer call or SQL query; a refusal closes the WebSocket, and over HTTP (the CLI) the caller
gets 403 with the reason.

| Connection | Let in when |
|---|---|
| An operator | always, whatever the token; an operator cannot lock themselves out |
| A registered service (MOD2, `register_service`) | always, whatever the token: the operator registered that identity, so a service keeps working whatever its issuer and whether device identities are on; `remove_service` ends that |
| A device identity: a token the host signed itself (issuer `localhost`), as the desktop's token file and the web's dummy mode use, or no token | device identities are allowed: yes until an operator runs `set_device_identities false` |
| Any other OpenID Connect token, such as a WorkOS sign-in | a `trusted_issuer` row has its issuer, byte for byte, and its audience: the token's `aud` claim, or its `client_id` claim when it has no `aud` (WorkOS AuthKit tokens) |

**How maincloud learns to trust an issuer: it does not need to.** There is no per-database or
per-server setting. The SpacetimeDB 2.12 host verifies any token by fetching
`{iss}/.well-known/openid-configuration` and the keys at its `jwks_uri`, for any http(s) issuer,
and checks neither issuer nor audience
(`crates/core/src/auth/token_validation.rs` at v2.12.0: `validate_aud = false`, "We should
require a specific audience at some point"). The docs name no configuration step either
(<https://spacetimedb.com/docs/core-concepts/authentication>, `/authentication/usage`, which asks
the module to check `iss` and `aud` itself). The module's `trusted_issuer` table is therefore the
whole of the trust decision. Proven on the local server by `../tools/auth-proof.ts`
(`cd rust/tools && ~/.bun/bin/bun run auth-proof`): the host fetched the discovery document and
keys of a test issuer on 127.0.0.1 by itself, a validly signed token from an issuer nobody
registered was refused by the module, not the host, and the same token connected once its issuer
was registered. Maincloud runs the same host code; the device tokens it issued on 2026-10-16 had
the same issuer (`localhost`) and audience (`spacetimedb`) as the local server's.

**The identity** is `Identity::from_claims(iss, sub)`: a blake3 hash of `"{iss}|{sub}"`, shown
as 64 hex digits starting `c200`. One person at one issuer is one identity on every client and
every database; another issuer (or the same issuer with a trailing slash) is another identity.

### Registering WorkOS for a database

1. The WorkOS client id of the environment (`WORKOS_CLIENT_ID`, `client_...`). AuthKit access
   tokens carry it as `client_id` and have no `aud` unless a JWT template adds one
   (<https://workos.com/blog/verify-workos-access-tokens-in-your-own-api>).
2. The issuer, exactly as the tokens' `iss`. Without a custom auth domain it is
   `https://api.workos.com/user_management/<client id>`, whose discovery document WorkOS serves
   per client; with a custom auth domain it is that domain. The safest source is a real token:
   sign in on the web app in workos mode against the database, and while the issuer is not
   trusted the Books screens say "SpacetimeDB refused this sign-in: an operator must trust issuer
   ... for audience ... on this database", naming both values.
3. As an operator (the user on maincloud; a thread only with `-s local`):

```sh
S=~/.local/bin/spacetime
DB=flowtrace-ledger-dev
ISS=https://api.workos.com/user_management/client_XXXXXXXX   # exactly the tokens' iss
AUD=client_XXXXXXXX                                          # the client id, or the aud a JWT template sets

# Trust it (again with a new label to relabel; one row per issuer and audience).
$S call -s maincloud $DB add_trusted_issuer "\"$ISS\"" "\"$AUD\"" '"WorkOS staging"'

# Add a person by WorkOS user id before they ever connect (a member who can close):
# org, issuer, subject (the WorkOS user id), name, can_post, can_approve, can_close.
$S call -s maincloud $DB add_member_by_subject '"org-example"' "\"$ISS\"" '"user_01XXXXXXXX"' '"Ama Owner"' true true false

# What is trusted (private table: the database owner reads it).
$S sql -s maincloud $DB "SELECT * FROM trusted_issuer"

# Stop trusting it; open connections stay open, the next one is refused.
$S call -s maincloud $DB remove_trusted_issuer "\"$ISS\"" "\"$AUD\""

# Device identities off (on again with true). Operators still connect.
$S call -s maincloud $DB set_device_identities false
```

`add_member_by_subject` refuses an issuer that is not trusted, so a mistyped issuer is caught.
The member's user id is the subject, as `create_org_ledger` makes a signed-in owner's user id
their subject. A person who already connected can also be added by identity with `add_member`;
the Books screens show it.

**Production.** Device identities are on in every database until an operator turns them off,
including `flowtrace-ledger`: the desktop (and its release archives) connect with a device
identity until AUTH2 adds the browser sign-in, so turning them off on production before that
locks the desktop out. It also refuses anonymous readers: a client with no token gets a device
identity, so the MOD2 public tables (`public_organization`, `public_campaign`, `public_badge`)
can then only be read by signed-in people, services and operators, which the public donate,
badge and embed pages would need to account for. Registering WorkOS on production and turning device identities off are
writes on production, so the user's step.

## 6. Proof files (STO1)

Receipts, invoices and photos are not in the module: they live in a Cloudflare R2 bucket behind
the proofs Worker (`../../workers/proofs/`, deploy steps in its `README.md`; pre-decision P13).
The module keeps only each file's R2 object key and SHA-256 on its `verification` row.

- **Who writes what.** The Worker never writes to the module. A client uploads to the Worker
  (`POST /upload`), gets `{ key, sha256, size, content_type, thumbnail_key }` back, and records
  that with `create_verification(org_id, verification_json)` (the desktop does this on save;
  `crates/desktop/README.md`, "Proof files").
- **How the Worker reads the module.** Over the HTTP SQL endpoint
  (`POST /v1/database/<name>/sql`, <https://spacetimedb.com/docs/http/database>) with its
  `MODULE_TOKEN` secret, it reads `member`, `access_group`, `access_group_member` and
  `verification` to check membership and the caller's proof visibility (the same rules as the
  `my_verification` view). Those tables are private, so on dev `MODULE_TOKEN` is the token of the
  identity that owns the database (the operator CLI login; never print `cli.toml`, copy the token
  into the Worker secret by hand). MOD2 adds a `service` role (pre-decision P17); then the
  Worker gets that role's token instead and nothing else changes.
- **Callers.** The Worker accepts a SpacetimeDB token (checked at
  `GET /v1/identity/:identity/verify`, members matched by identity) and, once AUTH1 sets
  `WORKOS_ISSUER`, `WORKOS_JWKS_URL` and `WORKOS_AUDIENCE`, a WorkOS access token (members matched
  by `user_id` = the token's `sub`).
- **Publishing.** `mark_published(org_id, verification_id, published)` (admins and managers,
  module addition in STO1) sets `published` on the proof's record; the Worker then serves the
  file to anyone at `GET /public/<sha256>`. The flag lives in the row's JSON (`data`), so no
  table changed and rows written before it read as unpublished.
- **Database.** The Worker's `SPACETIMEDB_DATABASE` is `flowtrace-ledger-dev` until the user
  points it at production; a module update that adds the reducer is a publish, so on
  `flowtrace-ledger` it is the user's step (section 4).

