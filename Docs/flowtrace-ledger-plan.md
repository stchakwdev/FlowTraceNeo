# FlowTrace Ledger: Plan for NetSuite-class Spend Tracking

Status: accepted with the defaults below · 2026-10-12. Decisions and the switch from Supabase to SpacetimeDB are recorded in `Docs/adr/0001-native-ledger.md`.

## 1. The problem

FlowTrace can show money moving and prove that it moved: flows, verifications, the trust score. But an org can't run its books in it. Today a `transaction` is one `from_node → to_node` amount (`001_initial_schema.sql`). There is no chart of accounts, no double entry, no budgets, no grant or fund restrictions, no reconciliation and no financial statements. So FlowTrace is a second system the finance team has to feed by hand, and that is why nobody adopts it.

The goal is for FlowTrace to be the place where an NGO records spend and tracks it against budgets and grants. Transparency then becomes a by-product of normal bookkeeping, not extra work.

## 2. Why not reverse-engineer NetSuite

- **There is nothing to reverse-engineer.** NetSuite is Oracle-hosted SaaS. Its logic runs on Oracle's servers. REA can only watch the browser UI and network traffic of a licensed account, and that shows screens, not the accounting engine.
- **Legal risk.** NetSuite's subscription terms prohibit reverse engineering and building competing products. A finance product can't afford that taint.
- **It isn't needed.** What makes NetSuite valuable is double-entry accounting, fund accounting, dimensions, approvals and period close. Those are public, standardised concepts, and mature open-source ERPs implement them in readable code.

**Where REA does fit:** REA's own docs say to skip it for ordinary source repos. Open-source ERPs are source repos, so we read their code and schemas directly. REA stays useful later for checking how a *shipped* client app behaves, such as a mobile-money provider's Android app, if an integration has no documentation.

**Where NetSuite does fit:** as an **integration target**. Orgs that already run NetSuite can sync with it through its official SuiteTalk REST API (Phase 6).

## 3. Open-source options

| Project | License | What to take from it | Use as |
|---|---|---|---|
| **ERPNext / Frappe** | GPL-3.0 | GL Entry model, Accounting Dimensions, Budget vs Actual, Period Closing Voucher, the **ERPNext Non-Profit** module (grants, donors) | Data-model reference. Don't copy code: GPL would infect FlowTrace. |
| **Odoo Community** | LGPL-3.0 | `account.move` / `account.move.line`, analytic accounts, bank reconciliation model | Data-model reference |
| **Apache OFBiz** | Apache-2.0 | Accounting and GL schema; code reuse is permissive | Reference; code reuse is allowed |
| **Formance Ledger** | MIT (verify) | Programmable double-entry ledger service | Candidate engine (option B) |
| **Blnk** | Apache-2.0 (verify) | Ledger plus reconciliation engine | Candidate engine (option B) |
| **TigerBeetle** | Apache-2.0 | High-throughput double-entry database | Overkill at our scale |
| Akaunting, Dolibarr, Beancount | mixed (verify) | UX ideas for small orgs | Reference only |

Licenses marked "verify" get checked in Phase 0 before anything is adopted.

## 4. Architecture options (recommendation: A)

**A. Native ledger in Supabase Postgres (recommended).** Build double-entry tables next to the existing schema, modelled on the ERPNext and Odoo designs.
- It fits the current stack: RLS multi-tenancy, triggers, Realtime and Edge Functions. There's no new service to host.
- Flows, the trust score and access groups can read the ledger directly.
- Costs: we own correctness, so the ledger needs DB-enforced invariants and heavy tests.

**B. Embed a ledger engine (Formance or Blnk) as a service.** Gives proven posting semantics, but adds a Go service to deploy and sync. Supabase RLS no longer covers ledger data, so access groups would need reimplementing. Revisit only if A hits correctness or scale limits.

**C. Run ERPNext as the back office and make FlowTrace a transparency layer on top.** The fastest way to full ERP features, but the heaviest ops: Python, MariaDB and Redis. It's GPL, the UX is foreign, and it's poorly suited to 3G and mobile use. Rejected as the core, but its data model is the main reference.

Integrations (QuickBooks, Xero, NetSuite, CSV and M-Pesa imports) are needed under any option, because many orgs won't replace their books on day one.

## 5. Target domain model (option A)

New migrations start at `014_`. All tables carry `organization_id` and RLS, following the `002` and `006` patterns.

```
accounts            chart of accounts (tree): code, name, type
                    asset|liability|equity|income|expense, normal_balance, parent_id
                    seeded from an NGO template (IFRS for SMEs / IPSAS-friendly)
funds               unrestricted | temporarily_restricted | permanently_restricted
grants              donor (flow_node), fund_id, award amount, currency, start/end, terms
dimensions          project, department, location, donor (generic key/value per org)
fiscal_periods      open | closed | locked
journal_entries     date, period_id, memo, source_type/source_id, status draft|posted|reversed,
                    reversal_of, posted_by, posted_at
journal_lines       entry_id, account_id, debit, credit (one side > 0), currency,
                    fx_rate, base_amount, fund_id, grant_id, dimension refs, flow_node_id
budgets / budget_lines   by account × fund/grant × dimension × period
vendors, bills, bill_lines, payments           (AP: what the org owes and pays)
expense_claims, expense_lines, approvals       (staff spend, approval chain)
bank_accounts, statement_lines, reconciliations (bank + M-Pesa / MTN / Airtel)
```

### Invariants enforced in the database, not the UI

- A posted entry's lines balance: Σ debit = Σ credit, checked in base currency by a deferred constraint trigger.
- Posted entries are immutable. A correction is a reversal plus a new entry, which matches the audit-trail ethos of the trust score.
- No posting into a closed or locked period.
- Spend against restricted funds requires a `grant_id` or `fund_id`. A grant line is blocked once the grant is past its end date or over budget, unless an override with a reason is given.
- All money is stored as `numeric`, never float. Base-currency amounts are frozen at posting.

### How the existing model maps in

- `flow_nodes` become a view over accounts plus dimensions. A donor maps to a grant or donor dimension, a department to a dimension, a vendor to the `vendors` table, and an account to a bank or cash account. Visualization nodes remain, now derived from the ledger.
- An existing `transaction` becomes a **business document**. Posting it creates a `journal_entry`, for example Dr Expense:Programs [grant X, project Y] / Cr Bank:M-Pesa. `from_node`/`to_node` keep working for the flow diagram. A backfill migration converts the seeded transactions.
- `verifications` attach to source documents (bills, claims, payments) as they do now. The trust score gains new signals: the reconciled share, the share of spend inside budget, and the share of spend coded to a grant.

## 6. Phased plan

Each phase ships a usable vertical slice. Acceptance criteria are the gates.

### Phase 0: Decisions and research (about 1 week)
- Lock the open questions in §8.
- Study the ERPNext GL Entry, Budget and Non-Profit doctypes and Odoo's `account.move.line` and reconciliation. Write `Docs/adr/0001-native-ledger.md`.
- Verify the licenses in the §3 table.
- Interview 2–3 target NGOs about how they track spend today (Excel, QuickBooks, Tally, Sage?), what donor reports they file, and what their M-Pesa statement exports look like.
- **Done when:** an ADR is accepted, the NGO chart-of-accounts template is drafted, and the list of required reports is fixed.

### Phase 1: Ledger core (about 2–3 weeks)
- Migrations `014_ledger_core` and `015_ledger_rls`: accounts, funds, dimensions, fiscal periods, journal entries and lines, the invariant triggers, and a `post_journal_entry()` RPC.
- Seed a CoA template on org creation, alongside the existing default-groups trigger.
- UI: chart-of-accounts editor, manual journal entry, and a general ledger view with filters on account, fund, grant and dimension.
- Bridge: posting a `transaction` creates a journal entry, plus a backfill for existing data.
- **Done when:** pgTAP or Vitest tests prove that unbalanced, mutated or closed-period postings are rejected, that a trial balance always nets to zero, and that RLS isolates orgs. The flow diagram still renders from the bridged data.

### Phase 2: Spend tracking, the adoption driver (about 3 weeks)
- Expense claims: mobile capture of photo receipt, amount and grant/project, then an approval chain driven by access-group permissions, then posting on approval.
- Vendors, bills and payments (AP). Proofs attach through the existing `verifications`.
- Budgets per grant or fund × account × period, with **budget-vs-actual** on the dashboard and on each grant page.
- Grant pages: award, spent, committed, remaining and burn rate, with a warning at 80% and 100%.
- Offline-first capture: queue claims in IndexedDB and sync when online (this is the PWA requirement in `CLAUDE.md`).
- **Done when:** a field officer on a low-end Android phone can submit a claim offline, a manager approves it, it posts to the right grant line, and budget-vs-actual updates.

### Phase 3: Reconciliation (about 2 weeks)
- Import bank CSV, OFX and M-Pesa / MTN / Airtel statement exports into `statement_lines`.
- Auto-match by amount, date window and reference, and confirm by hand when matching is ambiguous. Unmatched lines become draft entries.
- Feed the reconciled share into the trust score.
- **Done when:** a month of real M-Pesa statement data reconciles with fewer than 5% of lines left for manual review.

### Phase 4: Period close and financial reports (about 2–3 weeks)
- Period close and lock, plus year-end closing entries to net assets by fund.
- Reports: trial balance, Statement of Financial Position, Statement of Activities by fund (restricted vs unrestricted), Statement of Functional Expenses (program / admin / fundraising), cash flow, and grant / donor reports.
- Export to CSV, XLSX and PDF.
- Access groups govern which reports Donors and Public can see. This is the transparency hook: a donor sees *their* grant's report live.
- **Done when:** the reports from the seeded data foot and cross-foot, and an accountant reviews them.

### Phase 5: Flows and trust on the ledger (about 1–2 weeks)
- `/flows` reads from journal lines, so edges represent real postings. Node drill-down opens the ledger.
- Chain links (`009`) can share grant-level ledger views across orgs.
- **Done when:** every flow edge traces to its journal lines, and through them to its proofs.

### Phase 6: Integrations (ongoing)
- Import QuickBooks Online and Xero first (the most common for small NGOs), then two-way sync.
- **NetSuite via SuiteTalk REST and SuiteQL**, for orgs already on NetSuite. FlowTrace acts as the transparency and field-capture layer and posts approved spend back.
- M-Pesa Daraja API for live transaction pulls, which replaces statement imports.

## 7. Cross-cutting
- **Testing:** the ledger gets the strictest bar in the codebase, using property-based tests (fast-check) for posting and balance invariants plus DB-level tests for triggers and RLS.
- **Security:** posting and closing are RPCs that check `get_user_permissions()`. The audit log is append-only. No client-side writes to `journal_lines`.
- **Performance (3G):** paginated ledger queries with materialized balances per account × period × fund, refreshed on post. Reports are server-rendered.
- **Accounting review:** before Phase 4 ships, an accountant with NGO experience reviews the CoA template, the fund logic and the statement formats.

## 8. Open questions (Phase 0)
1. **Who keeps the books?** Is FlowTrace the system of record, or a layer on top of the org's existing books? This decides whether Phase 6 comes before Phase 2.
2. **Which reporting standard?** IFRS for SMEs, IPSAS, or local rules (for example Kenya's PBO Act), and which donor formats (USAID, FCDO, foundations)?
3. **Accounting basis:** cash or accrual? Many small NGOs run on cash basis. We could support both, with accrual as the core and a cash-basis report view.
4. **Multi-currency:** is base currency per org enough, or do grants in USD funding KES spend need FX gain/loss tracking from day one?
5. **First pilot org:** a real partner would shape the CoA template and the M-Pesa import format.
