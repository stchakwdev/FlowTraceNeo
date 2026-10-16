# ADR 0001: A native double-entry ledger, stored in SpacetimeDB

Status: accepted · 2026-10-12

## Context

FlowTrace showed money moving but could not keep an organisation's books
(`Docs/flowtrace-ledger-plan.md` §1). The plan recommended a native ledger in
Supabase Postgres (option A). The Supabase project has since been deleted, and
the owner chose SpacetimeDB as the backend.

## Decision

1. **Native ledger, engine in TypeScript.** The rules live in one pure module,
   `flowtrace/src/lib/ledger/`, with no framework imports. The bundled demo
   and the SpacetimeDB module both run it, so a rule is written once.
2. **Storage in SpacetimeDB.** `spacetimedb/` is a TypeScript server module.
   Every write is the `run_command` reducer: it loads the organisation's rows,
   applies one `LedgerCommand` through the engine and writes back the
   difference. A broken rule throws and the reducer rolls back. Tables are
   private; members read through per-identity views. The actor always comes
   from the `member` table, never from the client.
3. **Invariants enforced in the engine, and again at write-back:**
   - Entries balance in base currency and within each fund.
   - Posted entries are immutable; a correction is a reversal plus a new entry.
   - Nothing posts into a closed or locked period, and periods close in order.
   - Restricted spend must name its grant. Spend outside a grant's dates or
     over its award needs an override reason, which is stored.
   - Money is integer minor units; base amounts are frozen at posting.
   - Claimants cannot approve their own claims.
4. **Answers to the plan's open questions (§8), taking its defaults:**
   - *System of record:* FlowTrace keeps the books. Spend tracking (Phase 2)
     came before integrations (Phase 6); QuickBooks and Xero imports exist
     for orgs migrating in.
   - *Reporting standard:* IFRS for SMEs presentation. Statement of Financial
     Position, Statement of Activities by restriction, Statement of
     Functional Expenses and cash flow (direct). IPSAS is a settings value
     for later.
   - *Basis:* accrual books, with a cash-basis view of every report.
   - *Currency:* one base currency per organisation. Lines may be in other
     currencies with a rate; realised FX gains and losses post on payment.
     Unrealised revaluation is not done yet.
   - *Pilot org:* the demo organisation (fictional) until a partner signs on;
     the chart of accounts template is the NGO default in `coa-template.ts`.
5. **No borrowed code.** ERPNext (GPL-3.0) and Odoo (LGPL-3.0) were used as
   data-model references only. Formance and Blnk were not adopted, so their
   licences do not bind FlowTrace.

## Consequences

- Correctness is ours: the engine carries property-based tests (fast-check)
  for balance, reversal and immutability, plus rule tests and a footing test
  on the demo books.
- Reports are computed from the full state on the client. That is fine at
  NGO scale; very large orgs will need server-side aggregates.
- Every command loads the whole organisation in the reducer. Same caveat.
- NetSuite (SuiteTalk) and M-Pesa Daraja live sync need credentials and a
  partner organisation; they are not built. Statement CSV import covers
  M-Pesa, MTN MoMo, Airtel Money and bank exports today.
- An accountant with NGO experience should review the chart of accounts,
  fund logic and statement formats before a real organisation relies on them
  (plan §7).
