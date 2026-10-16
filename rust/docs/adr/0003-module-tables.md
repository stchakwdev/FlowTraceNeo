# ADR 0003: ledger-module tables keep the TS module's row shape

Status: accepted · 2026-10-16

## Context

Task S1 builds the Rust SpacetimeDB module (`crates/ledger-module`) that mirrors
`spacetimedb/src/index.ts`. Its brief preferred tables built from the ledger-core entity
types: a `spacetime` feature on ledger-core adding `#[derive(SpacetimeType)]` to the entity
and enum types (pre-decision P5, one engine for app, module and client). It allowed
module-local rows with tested conversions for any type the derive cannot express.

The TS module itself does not store typed entities. Through `spacetime-codec.ts:1-10` it
gives journal entries and lines typed columns (amounts as i64, so SQL can query them) and
stores every other collection as a doc row, `{ key, orgId, id, seq, data }`, with the record
as JSON in `data`. Keys are `${orgId}:${id}` because ids are only unique per organisation,
and `seq` keeps the state's array order.

Tried first: the feature, with the derive on 39 types in `types.rs`, `Timestamp` in `env.rs`
and the id macro in `ids.rs`, deriving through `spacetimedb-lib` with
`sats(crate = spacetimedb_lib)` so ledger-core would not depend on the module bindings.
The derive panics on every tuple struct:

```
error: proc-macro derive panicked
  --> crates/ledger-core/src/env.rs:22:42
   = help: message: called `Option::unwrap()` on a `None` value
```

`spacetimedb-bindings-macro` 2.12.0 (`src/sats.rs`) builds a product from named fields only.
ledger-core's ids (`AccountId(String)` and twenty others), `IsoDate` and `Timestamp` are all
tuple newtypes, and every entity has an id, so no entity can derive. Only the seventeen
plain enums could, and `AppliesTo` could not (its payloads are ids). `BTreeMap` (the
counters) has no SATS type either.

## Decision

1. **ledger-core is unchanged.** No `spacetime` feature: nothing in it would derive except
   the enums, and those are not needed (below). The derive attempt was reverted.
2. **The module's tables are the TS module's tables, column for column** (`src/tables.rs`),
   with snake case column names (`org_id`), which is also what the TS client queries
   (`spacetime-backend.ts:128`):

   | Tables | Rows | ledger-core type, and the conversion |
   |---|---|---|
   | `journal_entry`, `journal_line` | typed columns: ids, dates, timestamps and the TS enum literals as `String`, amounts `i64`, `fx_rate` `f64` | `JournalEntry`, `JournalLine`, through `codec::{entry_to_row, row_to_entry, line_to_row, row_to_line}` (`spacetime-codec.ts:121-201`); the way back validates dates, timestamps and enum literals and refuses amounts outside the JS safe range |
   | 14 doc tables, `account` to `audit_event` | `key`, `org_id`, `id`, `seq`, `data` (JSON) | the 14 record types, through their serde shape (the TS JSON, proven by the C1 round-trip fixtures), `codec::{doc_to_row, row_to_doc}` |
   | `ledger_org` | `settings_json`, `counters_json`, `next_seq`, `created_by`, `created_at` | `OrgLedgerSettings`, `LedgerState.counters`, through serde |
   | `operator`, `member`, `command_result` | as TS | none |

   `codec::state_to_rows` and `codec::assemble_state` take a whole `LedgerState` to rows and
   back; the demo books (306 records) round-trip equal in a host test, so the rows map back
   to engine state one to one (pre-decision P6 does not fire).
3. **No mirror of the entity types.** Typed module-local rows for every entity would have
   been 21 structs copying ledger-core field by field: a second set of types that drifts,
   which P5 exists to prevent. The doc rows reuse ledger-core's own serde shape instead, and
   the two typed rows destructure the ledger-core structs in full, so a field added to
   `JournalEntry` or `JournalLine` breaks the module's build rather than being dropped.
4. **Views** are the TS views: the 2.12 Rust query builder has the same
   `member.where(identity = sender).right_semijoin(table, org_id)` form, so 17 views are
   query views. `my_member` is a procedural view over the two member indexes: the builder
   names both sides of a self-join by the bare table name, which would give the ambiguous
   `FROM "member" JOIN "member"`.
5. **Timestamps.** The TS module writes `ctx.timestamp.toISOString()`, which in the TS SDK
   has microseconds (`.123456Z`, SDK 2.11 `src/lib/timestamp.ts:125-148`). ledger-core's
   `Timestamp` holds the engine's millisecond shape (`.123Z`), so the module writes
   milliseconds (`clock::iso_millis`, truncating as the TS SDK does).

## Consequences

- S4 loads and writes back an organisation with `codec` and the `DocRow` trait; it parses
  JSON per doc row, as the TS module does.
- S3's client decodes doc rows with serde straight into ledger-core types; only entry and
  line rows need the `row_to_*` conversions, which live in this crate for now.
- Entry and line columns stay SQL-queryable; doc rows are opaque to SQL beyond key, org and
  seq, as in TS.
- Audit and engine timestamps from this module have millisecond precision where the TS
  module's have microseconds. Matching TS exactly would need ledger-core's `Timestamp` to
  accept six fractional digits; that is a ledger-core decision, not taken here.
- `create_org_ledger` stores the default settings only: ledger-core has no chart-of-accounts
  template or `ensureFiscalYear` yet (C2, C6). S4 seeds the full state (`src/seed.rs`).
- `remove_operator` has no TS counterpart; it refuses non-operators, unknown identities and
  removing the last operator.
