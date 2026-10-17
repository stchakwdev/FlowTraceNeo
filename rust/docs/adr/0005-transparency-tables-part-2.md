# ADR 0005: transparency tables, part 2 (MOD2)

Status: accepted · 2026-10-17

## Context

MOD1 (ADR 0004) moved the first half of the web's transparency side into the ledger module. This
task moves the rest: flow nodes, transactions, their visibility settings, the public embed config,
organisation chains, the trust score, and three things the web never had: flags raised by the
model service (pre-decisions P14, P15), a `service` role for it (P17), and a published projection
that public pages read without an identity (P20). P16 fixes the trust score: computed in the
module, one row per organisation, four components weighted 32/24/24/20, the fourth from the model.
Production `flowtrace-ledger` is live, so the schema change must be additive.

Ground truth as in MOD1: the TS types (`flowtrace/src/lib/supabase.ts`) and the demo API
(`flowtrace/src/lib/demo/api.ts`) for shapes and behaviour, the Supabase row-level security
policies (`flowtrace/supabase/migrations/`) for who may write, the demo API where they disagree.

## Decision

### Tables (additive: no ledger or MOD1 table changes)

| Table | Key | Record (`data`) | Source |
|---|---|---|---|
| `flow_node` | `org:id` (doc row) | `FlowNode`, `balance` in minor units | `supabase.ts:52-66` |
| `transaction` | `org:id` | `Transaction`, amounts in minor units, plus `journal_entry_ids` | `supabase.ts:68-87` |
| `node_visibility` | `org:id` | `NodeVisibility` | `supabase.ts:225-235` |
| `transaction_visibility` | `org:id` | `TransactionVisibility` | `supabase.ts:237-246` |
| `trust_score_history` | `org:id` | `TrustScoreHistory` plus `model_score` and `grade` | `supabase.ts:164-173` |
| `flag` | `org:id` | `Flag` (below) | P14, P15 |
| `trust_score` | `org_id` | `TrustScore` (below) | P16 |
| `public_embed_config` | `org_id` | `PublicEmbedConfig` | `supabase.ts:248-271` |
| `organization_chain` | `id`; `source_org_id`, `target_org_id` typed and indexed | `OrganizationChain` | `supabase.ts:1088-1105` |
| `chain_activity_log` | `id`; `chain_id` indexed | `ChainActivityLog` | `supabase.ts:1122-1130` |
| `service` | `identity` | typed: `label`, `scopes`, `registered_by`, `registered_at` | P17 |
| `public_organization` (public) | `org_id`; `slug` indexed | `PublicOrganization` | P20 |
| `public_campaign` (public) | `campaign_id`; `org_id`, `slug` indexed | `PublicCampaign` | P20 |
| `public_badge` (public) | `org_id` | `PublicBadge` | P20 |

The doc rows have MOD1's shape (`key`, `org_id`, `id`, `seq`, `data`) so a record can gain fields
without a schema change. A chain belongs to two organisations, so it is keyed by its own id with
both organisations as typed columns, each side's view finding it through its own. `transaction`
works as a table name in the module, SQL and the generated bindings. `trusted_issuer` is AUTH1's
(PR #41) and is not added here.

### Views

Eight membership semijoins as MOD1's (`my_flow_node`, `my_transaction`, `my_node_visibility`,
`my_transaction_visibility`, `my_trust_score`, `my_trust_score_history`, `my_flag`,
`my_public_embed_config`; policies `002:68-97,133-135`, `006:134-141,170-177,211-213`),
`my_service` (the caller's own row), and two procedural views over the chains of the caller's
organisations on either side (`010_chains_rls.sql:10-15,46-54`): `my_organization_chain` gives
each chain as `ChainDetails` (the TS `OrganizationChainWithDetails`: both organisations' name,
logo key, type, country and trust score, the target's linked node and granted group), since
neither side can read the other's records; `my_chain_activity_log` their entries. The public
tables need no view: SpacetimeDB lets any connection read a public table, anonymous ones included
(`public` on the table attribute, <https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.table.html>).

### Reducers and who may call them

| Reducer | Behaviour | Who |
|---|---|---|
| `create_flow_node(org, node)` | `createNode` (`api.ts:264-274`) | any member (`002:72-74`) |
| `update_flow_node(org, id, patch)` | module addition: name, type, balance, metadata | any member (`002:76-79`) |
| `create_transaction(org, txn)` | `createTransaction` (`api.ts:220-238`) | any member (`002:90-92`) |
| `update_transaction(org, id, patch)` | `updateTransaction` (`api.ts:240-254`) | any member (`002:94-97`) |
| `link_transaction_entries(org, id, entry_ids)` | module addition (`bridge.ts:83-86` ties entries by `source_id`) | any member |
| `set_node_visibility`, `remove_node_visibility` | `api.ts:708-743` | admin or manager (`006:143-157`) |
| `set_transaction_visibility`, `remove_transaction_visibility` | module addition modelled on the node calls | admin or manager (`006:179-193`) |
| `update_public_embed_config(org, patch)` | `updatePublicEmbedConfig` (`api.ts:752-770`) | admin or manager (`006:216-226`) |
| `regenerate_embed_token(org)` | `regenerateEmbedToken` (`supabase.ts:874-885`) | admin or manager |
| `request_chain(source, target, message)` | `requestChain` (`api.ts:865-892`) | any member of the source (`010:17-21`) |
| `approve_chain`, `reject_chain`, `revoke_chain`, `update_chain_permissions` | `api.ts:921-990` | admin or manager of the target |
| `cancel_chain_request(chain)` | `cancelChainRequest` (`api.ts:971-976`) | any member of the source (`010:35-39`) |
| `register_service(identity, label, scopes)`, `remove_service(identity)` | P17 | an operator |
| `record_flags(org, batch)` | P14 | a service with the `flags` scope |
| `record_rubric(org, rubric)` | P16 | a service with the `rubric` scope |
| `resolve_flag(org, flag, resolution, reason)` | P15 | a member who can approve spend |
| `recompute_trust(org)` | `calculateTrustScoreAPI` (`api.ts:406-472`) | any member, or a service with the `trust` scope |

As in MOD1, an admin or manager is a member who can close periods. Chains: the policies let any
member of the target change a chain; approving, rejecting, revoking and regranting hand another
organisation one of the target's access groups, and managing access is for admins and managers
(`006:26-65`), so the module asks a member of the target who can close periods. Rules:
`crates/ledger-module/src/transparency/{flows,chains,trust,services,public}.rs`.

Every transparency write (MOD1's included) now ends the same way (`reducers::change`): the trust
row is computed again when one of its inputs moved (`trust::refresh`; no history row), and the
organisation's public rows are rewritten from its records (`reducers::publish`). A proof of a
transaction counts towards it and verifies a pending one at two (`flows::count_proof`, the demo's
mirror of the migration 001 triggers, `api.ts:300-306`); the proof's transaction must now exist.
Deleting an access group drops its node and transaction visibility rows (`api.ts:611-616`).
Chain writes touch neither the trust inputs nor the public rows.

### The trust score

The web's score is `calculateTrustScore` (`supabase.ts:603-612`): `Math.round(0.4 v + 0.3 d +
0.3 c)`, with `v` the verified transactions over all (percent, rounded), `d` the transactions with
a proof over all, `c` the latest history row's consistency, else 80 (`api.ts:413-428`). The module
reproduces it float for float (`trust::calculate_trust_score`), pinned on 2,197 grid cases, the
seed history and the demo (`fixtures/transparency-trust-score.json`).

The module's score (P16): `score = round(0.32 v + 0.24 d + 0.24 c + 0.20 m)`, grade as
`getTrustGrade` (`supabase.ts:614-621`), with

- `m = max(0, base - 5 k)` for `k` confirmed flags (`PENALTY_PER_FLAG` = 5, the brief's default);
- `base` the mean of the rubric scores the model service last stored (`record_rubric`), and,
  before the model has scored, the web's own three-component score `0.4 v + 0.3 d + 0.3 c`.

The brief asked for a fixture proving that with `m = 100` and no flags the new formula equals the
old score for the demo. It cannot: `0.32 v + 0.24 d + 0.24 c + 20 = 0.8 × old + 20`, which equals
the old score only at 100; the demo (75, 75, 82) scores 77 on the old formula and 82 with
`m = 100`. The module therefore makes an unscored model neutral: `0.32 v + 0.24 d + 0.24 c +
0.2 (0.4 v + 0.3 d + 0.3 c)` is the old score exactly, and with no rubric and no confirmed flag the
module takes the total from `calculate_trust_score`, so the two agree for every input
(`tests/transparency_trust.rs` checks all 101³). Until JV1 writes rubric scores every organisation
keeps its web score; a confirmed flag then costs one point of total (0.2 × 5). The `m = 100` value
is pinned too. Changing the neutral base or the penalty is one constant each.

The row (`TrustScore`): `score`, `grade`, the four `components` (`model` unrounded), the four
`weights`, `breakdown` (the four rounded, by name: the TS history's three plus `model`), `model`
(`source` rubric or neutral, `rubric`, `rubric_mean`, `model_version`, `scored_at`,
`confirmed_flags`, `open_flags`, `penalty_per_flag`), `inputs` (the three transaction counts),
`computed_at`, `computed_by` (user id, `service:<label>` or `module`). The automatic recompute
writes the row only; `recompute_trust` also appends a history snapshot (id then time, as the TS
draws them) and returns the TS `TrustScoreResult`, whose `previous_score` is the latest snapshot's
(the TS reads the organisation's stored score, which only that call writes, so they agree).

### Flags

`Flag`: id, organisation, `subject_type` (`transaction`, `claim`, `bill`) and `subject_id`,
`question_key`, `answer` (the model's JSON), `probability` and `confidence` (0 to 1),
`model_version`, `state` (`open`, `dismissed`, `confirmed`), `reason`, `raised_by`
(`service:<label>`), `resolved_by`, `resolved_at`, `created_at`, `updated_at`. `record_flags`
upserts by subject and question: a new finding opens a flag, an open one takes the new answer and
model, a resolved one never changes. Subjects must exist (transactions in the organisation, claims
and bills in the books). `resolve_flag` takes an open flag to dismissed (a reason is required) or
confirmed, and appends `flag.dismiss` or `flag.confirm` to the books' audit log with the flag id
and the reason.

### Services

`register_service(identity, label, scopes)` (operators; scopes `flags`, `rubric`, `trust`;
re-registering replaces label and scopes) and `remove_service(identity)`. A service is not a member
of any organisation; the gated reducers check the caller's `service` row and scope. With AUTH1 the
service authenticates with a WorkOS client-credentials token, and its identity is the one the host
derives from that token's issuer and subject.

### The public projection

Written for each organisation from its records by every transparency write:

- `public_organization` and `public_badge` for an organisation whose profile is public
  (`is_public`, which starts as "verified and has a public slug", `002:40-42`): the profile fields
  the public pages show, the trust score and grade; the badge's score, grade, breakdown and time.
- `public_campaign` for each campaign open to the public (public, active, with a link:
  `getCampaignBySlug`, `api.ts:1059-1066`): the campaign's public fields, its figures, and its
  report as the donate page and the embed show it (`getCampaignReport`, `api.ts:1381-1443`):
  organisation name, score and verified flag, evidenced amount, earmarks, and the six latest
  programme transactions with what the page draws of them (`public-campaign-header.tsx:113-136`).
  No donor, no metadata, no `created_by`, no transaction notes or references.

### Where the module differs from the web (the demo API is followed otherwise)

1. Posting: the demo posts a new transaction to its books through the bridge with its own
   node-to-account mapping (`postDemoTransaction`, `demo/ledger.ts:432-456`). The module has no
   such mapping: posting stays a `run_command` (an entry with `source_type` `transaction`) and
   `link_transaction_entries` records the link; an entry records at most one transaction.
2. Checks the demo lacks: a transaction amount above zero, a `YYYY-MM-DD` date, nodes of the
   organisation; a flow node needs a name; `verification_count` and `created_by` are not in an
   update (the TS type allows both); a proof's transaction must exist.
3. Embed config: the demo refuses an organisation without one; the module makes one on the first
   update from the column defaults (`005_access_groups.sql:126-160`), token included.
4. Chains: the table's unique pair refuses any second chain between the same two organisations;
   the module refuses only a pending or active one, so a rejected or revoked pair may ask again.
   A target must be an organisation of the module. A cancelled request takes its log with it (the
   table's cascade; the demo leaves the entries behind, unreachable). Who acts in a chain's log is
   the caller, not the demo's fixed demo user.
5. Badge: the web draws the latest history snapshot; the module's badge is the current trust row,
   which the module keeps current (P16 puts the breakdown on the badge).
6. Chain trust score: `getChainTrustScore` reads both organisations' stored scores; the module's
   `chains::chain_trust_score` takes the two trust rows' scores (0 for an organisation without one),
   which `ChainDetails` carries.

## Consequences

- `tests/transparency_flows.rs` replays all 45 recorded MOD2 demo API calls
  (`fixtures/transparency-flows.json`) through the module's rules with the same ids and time;
  `tests/transparency_fixtures.rs` (MOD1) now also checks the proof counting on transactions;
  `tests/transparency_trust.rs` pins the formulas; `src/transparency/mod2_tests.rs` covers the
  permission and integrity rules the demo lacks.
- ledger-client reads every new view and the public tables (`chains`, `chain_activity`,
  `service`, `public_organizations`, `public_campaigns`, `public_badges`; the organisation views
  are part of `OrgTransparency`), and `ClientEvent::TransparencyChanged` gains `chains`, `public`
  and `service`. An anonymous connection reads the public tables (`tests/live_public.rs`).
- The release wasm grows from 3,086,475 to 4,117,903 bytes.
- Publishing this module over main's on a database with books needs no clear (the local proof
  does it every run with `--base-wasm`: 14 tables and 11 views created).
- What the next tasks call: JV1 registers its identity once (`register_service(identity,
  "jev-worker", ["flags", "rubric", "trust"])`, an operator call), then per run `record_flags(org,
  {"model_version", "flags": [{subject_type, subject_id, question_key, answer, probability,
  confidence}]})`, `record_rubric(org, {"model_version", "scores": {key: 0-100}})` and
  `recompute_trust(org)`. STO1 (PR #42) attaches files through MOD1's `create_verification`
  (object key and SHA-256, linked to a transaction, claim or bill), which now counts the proof
  towards its transaction and moves the trust row, and its own `mark_published`.
