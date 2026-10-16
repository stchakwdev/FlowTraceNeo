# ADR 0004: transparency tables in the ledger module (MOD1)

Status: accepted · 2026-10-16

## Context

The web app's transparency side (organisation profile, people, access groups, proofs, campaigns,
donations, donor updates) lost its backend when Supabase went; it runs on the in-memory demo API
(`flowtrace/src/lib/demo/api.ts`) over the demo dataset (`flowtrace/src/lib/demo/dataset.ts`),
with its types in `flowtrace/src/lib/supabase.ts`. Pre-decision P12 puts this data in the same
SpacetimeDB module as the ledger; P13 keeps proof files in R2 with the object key and SHA-256 in
the row; P19 makes donations records only; P21 keeps access groups their own table. Production
`flowtrace-ledger` is live, so the schema change must be additive.

Ground truth for this task, as the brief sets it: the TS types and the demo API for shapes and
behaviour; where the demo API and the Supabase functions disagree, the demo API. The demo API is
single-user and checks no permissions at all, so who may do what comes from the web's own
backend: the row-level security policies in `flowtrace/supabase/migrations/` (`002`, `006`,
`008`, `012`), with the caller's organisation and role read from their member row.

## Decision

### Tables (additive: no ledger table changes)

All private, read through `my_*` views. The eight organisation tables are doc rows in the
ledger's shape (ADR 0003): `key` (`{org_id}:{id}`, primary), `org_id` (btree), `id`, `seq`
(insertion order within the organisation and table, next = highest + 1) and the record as JSON in
`data`, so a record can gain fields without a schema change. No view needs another typed column:
the membership views join on `org_id`, and the three procedural views read the JSON.

| Table | Key | Record (`data`) | TS type |
|---|---|---|---|
| `organization_profile` | `org_id` | `OrganizationProfile`: id, name, type, country, currency, verified, public_slug, description, logo_key, is_public, created_at, updated_at | `Organization` (`supabase.ts:28-40`) less `trust_score` (MOD2), `logo_url` as `logo_key` |
| `profile` | `identity` | `Profile`: identity, user_id, display_name, email_hash (SHA-256 of the trimmed lowercased email), created_at, updated_at | `Profile` (`supabase.ts:42-50`); email, phone, role and organisation not kept |
| `access_group` | `org:id` | `AccessGroup` as TS | `supabase.ts:191-206` |
| `access_group_member` | `org:id` | `GroupMember` as TS; `org_id` is the group's | `supabase.ts:208-218` |
| `verification` | `org:id` | `Verification` with `object_key`, `sha256`, `thumbnail_key` for the URLs, one link of `transaction_id`, `claim_id`, `bill_id` | `supabase.ts:128-151` |
| `verification_access_log` | `org:id` | `VerificationAccess` as TS | `VerificationAccessLog`, `supabase.ts:153-162` |
| `campaign` | `org:id` | `Campaign`, `target_amount` and `current_amount` in minor units | `supabase.ts:1390-1415` |
| `campaign_stat` | `org:campaign_id` | `CampaignStat`: donation_count, unique_donors, total_raised, funding_percentage, avg_donation, largest_donation, smallest_donation | `CampaignStats`, `supabase.ts:1417-1427`, less the two day counts |
| `donation` | `org:id` | `Donation`, `amount` and `amount_usd` in minor units, plus `donor_profile` | `supabase.ts:1442-1469` |
| `donor_update` | `org:id` | `DonorUpdate` as TS | `supabase.ts:1476-1490` |

Money is integer minor units, as everywhere in the ledger; the TS transparency types hold major
units and `ledger_module::transparency::ts` converts with `fromMajor`
(`flowtrace/src/lib/ledger/money.ts:26-31`). The day counts of `CampaignStats` depend on the
reading day; readers compute them (`transparency::stats::{days_remaining, days_active}`).

### Views

| View | Kind | Rows |
|---|---|---|
| `my_organization_profile`, `my_access_group`, `my_access_group_member`, `my_campaign`, `my_campaign_stat`, `my_donation`, `my_donor_update` | query (member semijoin, as the ledger's) | every row of the caller's organisations |
| `my_verification` | procedural | every proof of the caller's organisations, cut to `getVerificationVisibility` (`supabase.ts:573-588`) of the union of the caller's access groups (`getUserPermissions`, `api.ts:772-791`): `full` as stored, `thumbnail` without `object_key`, `summary` without either key, `hidden` left out; `visible_as` says which |
| `my_verification_access_log` | procedural | the caller's own entries (policy "Users can view own access logs", `008_trust_points.sql:121-123`) |
| `my_profile` | procedural | the caller's profile and their co-members' (`002_rls_policies.sql:50-56`) |

A caller's access groups are found through their member row's `user_id`
(`access_group_member.user_id`), so permission never comes from the call.

### Reducers and who may call them

The caller's role is their member row: any member is the policies' `organization_id =
get_current_org_id()`; a profile `role` of `admin` or `manager` is a member who can close
periods, the mapping the TS ledger already makes between access-group rights and ledger rights
(`actorFromPermissions`, `flowtrace/src/lib/ledger/setup.ts:28-43`: `can_close` is
`can_manage_groups`). Who acted (`created_by`, `granted_by`, `verified_by`, `accessed_by`) is the
member's `user_id`, ids are `ctx.new_uuid_v4()` and times the reducer clock in milliseconds, as
`run_command` does. Rules: `crates/ledger-module/src/transparency/plan.rs`.

| Reducer | Behaviour (demo API) | Who (policy) | Module additions |
|---|---|---|---|
| `set_profile(display_name, email)` | `createOrUpdateProfile` (`supabase.ts:311-330`) | the caller's own (`002:58-61`) | email kept as its hash; user id is the JWT subject or identity |
| `update_organization_profile(org, patch)` | `updateOrganization` (`api.ts:162-180`): slug taken refusal, field-by-field update | admin (`002:44-47`, "Admins can update own organization") | first update makes the profile; `verified` not settable |
| `create_default_access_groups(org)` | the five groups of `dataset.ts:698-789` (trigger `005:198-334`) | admin | once per organisation; nobody is put in a group |
| `create_access_group(org, group)` | `createAccessGroup` (`api.ts:585-596`) | admin or manager (`006:26-36`) | |
| `update_access_group(org, id, patch)` | `updateAccessGroup` (`api.ts:598-609`) | admin or manager (`006:39-51`) | default groups editable, as the demo and web form |
| `delete_access_group(org, id)` | `deleteAccessGroup` (`api.ts:611-616`): group and its members go | admin (`006:54-65`) | default groups refused (web UI and policy) |
| `add_access_group_member(org, member)` | `addGroupMember` (`api.ts:654-658`) | admin or manager (`006:82-95`) | group must be the organisation's |
| `invite_access_group_member(org, group, email, until)` | `inviteMemberByEmail` (`api.ts:666-683`) | admin or manager | note "Invited by email. No email was sent." |
| `remove_access_group_member(org, id)` | `removeGroupMember` (`api.ts:660-663`) | admin or manager (`006:114-127`) | |
| `create_verification(org, proof)` | `insertVerification` (`api.ts:284-317`): trust points, `verified_at` | any member (`002:108-114`) | exactly one link; claim and bill checked against the books; key and SHA-256 together |
| `log_verification_access(org, proof, view/download)` | `logVerificationAccess` (`supabase.ts:554-571`) | any member who can see the proof | level derived, never passed |
| `create_campaign(org, campaign)` | `createCampaign` (`api.ts:1080-1098`), `slugify` (`api.ts:1068-1078`) | any member (`012:29-31`) | target above zero, link unique (`011:20,42`) |
| `update_campaign(org, id, patch)` | `updateCampaign` (`api.ts:1100-1114`) | any member (`012:34-37`) | `current_amount` not settable; target and link checks |
| `delete_campaign(org, id)` | `deleteCampaign` (`api.ts:1116-1126`): drafts only, silent otherwise | any member (`012:40-45`) | |
| `record_donation(org, donation)` | `createDonation` + `applyToCampaign` (`api.ts:1245-1272`) | any member (`012:82-88`); anyone on an open public campaign (`012:69-79`) | a non-member's gift is pending only and keeps their identity as `donor_profile` unless anonymous |
| `update_donation_status(org, id, status, reference)` | `updateDonationStatus` (`api.ts:1274-1289`) | any member (`012:91-102`) | |
| `post_donor_update(org, update)` | `createDonorUpdate` (`api.ts:1329-1341`) | any member (`012:135-147`) | donation must be the organisation's |
| `mark_donor_update_sent(org, id)` | `markUpdateSent` (`api.ts:1343-1349`) | any member (`012:150-156`) | |
| `import_transparency(org, store)` | none (like `import_books`) | operator | replaces the organisation's records with a demo-store-shaped dataset |

Error texts are the demo API's without "in the demo data", or the module's own.

### Where the demo API and the Supabase backend disagree (the demo API is followed)

1. Permissions merge: `getUserPermissions` takes the union of every group and ignores
   `valid_from` and `valid_until` (`api.ts:772-791`); the Supabase function lets the highest
   priority group win per flag and skips memberships out of their dates
   (`005_access_groups.sql:341-367`). The module takes the union and ignores the dates; views
   have no clock, so expiry needs a scheduled reducer (open).
2. Default groups: the demo deletes any group; the policy refuses defaults
   (`006:54-65`). The web offers no delete for a default (`access-groups.tsx:402-413`), so the
   module refuses it: the one place the web UI wins over the demo API.
3. Default groups: the policy refuses any update to a default group (`006:39-51`); the demo and the
   web form allow it ("You can change its depth"). The module allows it.
4. Raised amount: the Supabase trigger floors `current_amount` at zero and handles deletes
   (`011:191-221`); the demo neither. The module follows the demo (no floor; donations are never
   deleted).
5. Completion: the Supabase trigger marks a campaign `completed` when it reaches its target
   without overflow (`011:224-238`); the demo does not. The module does not.
6. Campaign figures: `get_campaign_stats` caps the percentage at 100 and averages without the
   confirmed filter's rounding (`011:264-305`); the demo computes from confirmed donations,
   uncapped, one decimal (`api.ts:1130-1165`). The module follows the demo.
7. Campaign links: the demo makes one with `slugify` for a public campaign without one; Supabase
   inserts what it is given and has `public_slug UNIQUE` (`011:42`). The module slugifies as the
   demo and refuses a given link that is taken.
8. Invites: the demo writes the note "Invited in the demo. No email was sent."; Supabase writes
   none. The module writes "Invited by email. No email was sent."
9. Proof side effects: the demo raises the transaction's `verification_count` and auto-verifies
   at two proofs (migration 001 triggers); transactions are MOD2's, so not here.
10. Public donations: the policy lets anyone insert any donation to an open public campaign; the
    demo checks nothing. The module lets a non-member record only a pending gift to an open
    campaign (no processor confirms it, P19; a member confirms).
11. Access log: Supabase takes `accessed_by` and `access_level` from the caller; the module
    derives both.

Not a disagreement but worth knowing: `getVerificationVisibility` ignores the proof's own
`visibility_level` (its `_verification` parameter is unused), so a proof marked `hidden` is shown
in full to a Board member. The module does exactly the same, as the brief requires; using the
lower of the two would be a product decision.

### Hooks left for MOD2

`trust_score` (organisation profile), flow nodes (`target_node_id`, `earmarked_node_id`,
`flow_node_id`, `related_node_id`), transactions (`transaction_id`, `related_transaction_id`,
verification counts and auto-verify), node visibility, the public projection (donate pages,
badges, embeds), `PublicEmbedConfig` (exported in `transparency-dataset.json` for MOD2), chains
and the service role. These ids are kept as opaque text.

## Consequences

- The fixture test (`crates/ledger-module/tests/transparency_fixtures.rs`) replays all 47
  recorded demo API calls through the planning functions with the same ids and time; the
  permission and integrity rules the demo lacks have unit tests (`transparency/plan_tests.rs`).
- ledger-client reads the ten views into `OrgTransparency` and `Profile` through the module's own
  codec and announces `ClientEvent::TransparencyChanged` once per applied message (P7).
- The release wasm grows from 2,176,355 to 3,086,475 bytes.
- Publishing this module over main's on a database with books needs no clear (the local proof
  does it every run with `--base-wasm`).
