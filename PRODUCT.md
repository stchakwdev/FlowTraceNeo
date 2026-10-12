# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users
Primary: executive directors and finance leads at NGOs and nonprofits in African markets (Kenya first). They need to show donors, boards and partners where money went. Their reasons are to keep and win funding, and to stop rebuilding the same proof in spreadsheets for every report.

Secondary (inside the product, not the landing page's audience): donors and funders who receive scoped, read-only access to an org's flows. Field staff who capture receipts and proofs on phones.

## Product Purpose
FlowTrace records an organization's money movements and attaches proof to each one: receipts, photos, GPS, signatures, bank statements and audits. It shows the result as a flow from donor to beneficiary and condenses it into a 0–100 Trust Score. Success means an org can hand a funder a live, verifiable view of a grant instead of a PDF report, and funders trust it more because of that.

## Positioning
The proof is attached at the transaction, and access is scoped per audience: a board, staff, donors, partners and the public each see a different depth. Orgs can chain their views so a funder can follow a grant through an NGO to its local implementers. Accounting tools record numbers. FlowTrace records numbers together with their evidence, and lets an org choose exactly who sees what.

## Operating Context
- Mobile-first, low-bandwidth networks (2G/3G) and low-end Android devices. The target is under 3s page load on 3G.
- Mobile money is a primary payment rail: M-Pesa first, then MTN and Airtel.
- Multi-currency (KES first) and multi-tenant, with role-based access groups.
- Public surfaces are part of the product: donation campaign pages, embeddable campaign widgets and trust badges.

## Capabilities and Constraints
- Built today: flows visualization, transactions with verifications, Trust Score, access groups, cross-org chains, donation campaigns, public embeds and badges.
- Planned: a double-entry ledger with budgets, grants and reconciliation (`Docs/flowtrace-ledger-plan.md`).
- Auth currently runs in dummy mode (a demo user). WorkOS SSO is deferred until last.
- UI uses Phosphor icons. No emojis in the UI.

## Brand Commitments
- The name is FlowTrace.
- Mission (2025 brand guide): turn financial flows into visual trust so African organizations can unlock global investment through radical transparency.
- Personality: precise, trustworthy, progressive, human. Principles: restraint, clarity over decoration, "clean but never cold", no dark patterns.
- The 2025 guide's visual specifics (logo mark, palette, type) are NOT binding, and the visual design may be fully replaced.

## Evidence on Hand
- There are no real customer counts, volume figures, uptime figures, country counts, testimonials or logos. The landing page's previous stats were placeholders. Never fabricate proof.
- The real demo data lives in `flowtrace/supabase/migrations/013_seed_comprehensive.sql`: 5 orgs, 20+ nodes, 16 transactions, 4 campaigns. The demo app itself is the strongest evidence.

## Product Principles
1. Show the proof, not a claim about the proof.
2. Each audience sees exactly what it is entitled to, and no more.
3. Built for the field: the slow network and the cheap phone are the default, not the edge case.
4. Make transparency a by-product of normal work, not a separate reporting chore.

## Accessibility & Inclusion
WCAG AA contrast and touch targets. Must stay readable on small, low-quality screens in bright light. Use tabular numerals for all money.
