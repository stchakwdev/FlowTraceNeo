# FlowTrace proofs Worker

A Cloudflare Worker that keeps proof files (receipts, invoices, photos) in an R2 bucket and
hands them out only to people allowed to see them. It reads who may see what from the ledger
module (SpacetimeDB) and never writes to it: after an upload, the app records the returned
`key` and `sha256` through the module's `create_verification` reducer.

It runs on the Workers free plan and the R2 free tier.

## Routes

| Route | Sign-in | What it does |
| --- | --- | --- |
| `POST /upload` | Bearer token, any member of `org` | Multipart form with `org`, `link_kind` (`claim`, `bill` or `transaction`), `link_id` and `file`. Images (JPEG, PNG, WebP, GIF, HEIC/HEIF) and PDF, up to 25 MB, type decided from the file's bytes. Answers `201 { key, sha256, size, content_type, thumbnail_key }`. |
| `GET /file/<key>` | Bearer token, member whose access groups allow it | Answers `{ url, expires_at }` (`expires_at` in unix seconds): a signed link valid for 10 minutes. With `?redirect=1` it answers `302` to that link instead. Full viewers get originals; thumbnail viewers get thumbnails only. The uploader may preview a file nobody has recorded yet. |
| `GET /signed/<key>?exp=&sig=` | The signature in the link | Streams the file (also `HEAD`). Expired or altered links get `403`. |
| `GET /public/<sha256>` | None | Streams a proof only if a verification with that hash is `published`. Cached publicly for an hour. |
| `OPTIONS *` | None | CORS preflight for the origins in `ALLOWED_ORIGINS`. |

Errors are JSON: `{ "error": "plain-English message" }`. Unknown files and files you may not
see both answer `404`, so the Worker does not reveal what exists. A non-member gets `403`.

Keys look like `org/<org>/<sha256>` and `org/<org>/thumb/<sha256>.webp`.

Sign-in: send `Authorization: Bearer <token>`. Today that is the SpacetimeDB token the app
already holds; the Worker checks it with the SpacetimeDB host (`/v1/identity/<id>/verify`) and
remembers a good answer for a minute. WorkOS tokens are accepted once configured (below).

## Settings

Vars (in `wrangler.toml` under `[vars]`):

| Name | Default | Meaning |
| --- | --- | --- |
| `SPACETIMEDB_URL` | `https://maincloud.spacetimedb.com` | SpacetimeDB host |
| `SPACETIMEDB_DATABASE` | `flowtrace-ledger-dev` | The ledger module's database |
| `ALLOWED_ORIGINS` | `http://localhost:3000` | Comma list of web origins allowed by CORS |
| `AUTH_MODES` | `spacetimedb,workos` | Which sign-in paths are on |
| `WORKOS_ISSUER` | empty | Empty turns the WorkOS path off |
| `WORKOS_JWKS_URL` | empty | WorkOS signing keys |
| `WORKOS_AUDIENCE` | empty | Expected `aud`; empty skips that check |

Secrets (never in the repo):

| Name | Meaning |
| --- | --- |
| `MODULE_TOKEN` | A SpacetimeDB token whose identity can read the module's private tables over SQL. On dev this is the owner (operator) CLI identity. |
| `URL_SIGNING_KEY` | HMAC key for signed links |

## Local development

```bash
cd workers/proofs
bun install
cp .dev.vars.example .dev.vars   # then fill in MODULE_TOKEN and URL_SIGNING_KEY
bun run dev                      # wrangler dev, local R2 bucket
bun test                         # unit tests and Miniflare tests
bun run typecheck
```

`bun run dev` uses a local R2 store under `.wrangler/`, so nothing touches your Cloudflare
account. Point `SPACETIMEDB_URL` at a local SpacetimeDB server (for example `http://127.0.0.1:3100`)
in `.dev.vars` if you want to test against a local module.

## Deploy

1. Sign in: `bunx wrangler login`.
2. Create the bucket: `bunx wrangler r2 bucket create flowtrace-proofs`.
3. Set the secrets:
   ```bash
   bunx wrangler secret put MODULE_TOKEN
   openssl rand -hex 32 | bunx wrangler secret put URL_SIGNING_KEY
   ```
4. Set the vars in `wrangler.toml`: `SPACETIMEDB_DATABASE` to the database you publish the
   module to, and `ALLOWED_ORIGINS` to the web app's origin(s).
5. Optional thumbnails: uncomment the `[images]` block in `wrangler.toml`. Thumbnails use
   Cloudflare Images transformations; the free plan includes a limited number of
   transformations each month (see the Images pricing page for the current allowance). Without
   the binding, or if a transform fails, only the original is stored and uploads return
   `thumbnail_key: null`.
6. Deploy: `bunx wrangler deploy`.
7. Custom domain (optional): add `proofs.<your-domain>` under the Worker's
   Settings > Domains & Routes in the dashboard, or uncomment the `routes` line in
   `wrangler.toml` with your domain.
8. Tell the apps where it lives: set `VITE_PROOFS_URL` in the web app's environment and
   `proofs_url` in the desktop config to the Worker's URL.

## When WorkOS tokens arrive (AUTH1)

Set these vars and redeploy:

- `WORKOS_ISSUER`: the `iss` of your AuthKit access tokens.
- `WORKOS_JWKS_URL`: `https://api.workos.com/sso/jwks/<client_id>`.
- `WORKOS_AUDIENCE`: the expected `aud`, or leave empty to skip that check.

A token whose `iss` equals `WORKOS_ISSUER` is verified against those keys. Its `sub` must equal
a `member.user_id` in the organisation, so each person's member row needs their WorkOS user id.
To stop accepting SpacetimeDB tokens later, set `AUTH_MODES = "workos"`.

## Moving to a service identity (P17)

`MODULE_TOKEN` is today the owner identity's token, which can read every private table. MOD2
(PR #43) added the service role, so the switch now works like this: an operator registers a
new identity as a service with `register_service(identity, label, scopes)` (for example
`spacetime call -s maincloud <database> register_service '"0x<identity>"' '"proofs-worker"'
'["..."]'`), and that identity's token replaces the secret (`bunx wrangler secret put
MODULE_TOKEN`). The single place that picks the token is `moduleToken()` in `src/module.ts`.
One gap first: the Worker reads `member`, `access_group`, `access_group_member` and
`verification` over SQL, and SpacetimeDB lets only the database owner read private tables
that way. MOD2's scopes (`flags`, `rubric`, `trust`) and views give a service no read access
to those rows, so a service token alone would fail every lookup. The switch needs a read path
for the Worker in the module first (for example a `proofs` scope with views returning those
rows to services that hold it). That and the switch itself are left to JV1 or a follow-up;
this PR keeps the owner token.

## Known limits

- `/public/<sha256>` reads the whole `verification` table (cached for 60 seconds per Worker
  instance) to find published proofs. Fine for a few thousand proofs; MOD2's public projection
  should replace it.
- There is no delete route. Removing a file means deleting it from the bucket by hand.
- Files are content-addressed per organisation: the same bytes uploaded twice in one
  organisation are stored once, and the first upload's metadata (including `uploaded_by`) wins.
- Access rules follow the ledger module exactly: the union of a member's access groups decides
  full, thumbnail, summary or hidden; group membership dates and a proof's own
  `visibility_level` are not checked. A member in no access group sees no files (except their
  own unrecorded uploads).
