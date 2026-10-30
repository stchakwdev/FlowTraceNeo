# FlowTrace: Financial Transparency Platform

## Overview

FlowTrace is a financial transparency platform for organizations operating in African markets. It addresses a core problem: opaque fund flows through NGOs, government programs, and development organizations erode donor trust and reduce funding effectiveness.

The platform provides real-time transaction verification, AI-powered trust scoring (0-100), and interactive flow visualizations that trace money movement from donors through intermediaries to beneficiaries. It is designed mobile-first for low-bandwidth networks (target: <3s page load on 3G) and integrates with mobile money systems (M-Pesa, MTN Money, Airtel Money).

---

## Architecture

**TanStack Start (React 19) + Nitro SSR** deployed on Vercel, with file-based routing and server-side auth checks.

Key architectural decisions:

- **Protected route layouts** -- the `_authenticated` route layout runs a server-side `loader` that calls WorkOS AuthKit's `getAuth()`. Unauthenticated requests redirect to the WorkOS-hosted sign-in page. All child routes (`/dashboard`, `/transactions`, `/flows`, etc.) inherit this protection without per-route auth boilerplate.
- **Progressive enhancement** -- when WorkOS credentials are absent (dev mode), auth checks are bypassed entirely with a demo organization fallback. When Supabase is unreachable, the org context gracefully degrades to cached/demo data rather than crashing.
- **Multi-tenant isolation** -- Supabase Row-Level Security (RLS) policies enforce organization boundaries at the database level. A React context (`OrgProvider`) syncs the WorkOS user with a Supabase profile and provides `useOrg()`, `useHasRole()`, and `useCanManage()` hooks throughout the app.
- **Query architecture** -- TanStack Query with a hierarchical key factory (`['resource', 'id', 'orgId']`), resource-specific stale times (30s for transactions, 5min for nodes), and predicate-based cache invalidation on mutations.

---

## Tech Stack

| Category | Technology | Rationale |
|----------|-----------|-----------|
| Framework | TanStack Start + React 19 | SSR with streaming, file-based routing, server functions |
| Routing | TanStack Router | Type-safe routes, layout nesting, loader-based data fetching |
| Server | Nitro (Vite 7) | Edge-compatible SSR, API routes, middleware |
| Auth | WorkOS AuthKit | Enterprise SSO (SAML/OIDC), MFA, user management for nonprofits |
| Database | Supabase PostgreSQL | RLS for multi-tenancy, Edge Functions, Realtime subscriptions |
| Extensions | PostGIS, pgvector, pgcrypto | Geospatial verification, embedding search, field-level encryption |
| Styling | Tailwind CSS v4 (CSS-first) | OKLch color system, no config file, CSS variable theming |
| Components | shadcn/ui + Base UI React | Unstyled primitives with CVA variants, accessible by default |
| Icons | Phosphor Icons | Consistent icon system, multiple weights |
| State | TanStack Query | Server state with intelligent caching and invalidation |
| Fonts | IBM Plex Sans/Mono, DM Sans | Tabular numerics for financial figures, variable font for display |

---

## Database Design

> Superseded: the Supabase database below is deleted; the books, campaigns, donations, organisation profile and org context now run on the SpacetimeDB module (flowtrace/README.md, Docs/adr/0001-native-ledger.md).

The schema is built through **13 progressive migrations**, each adding a self-contained feature layer with its own tables, indexes, RLS policies, triggers, and functions.

### Migration Progression

| # | Migration | Purpose |
|---|-----------|---------|
| 001 | `initial_schema` | Core tables: organizations, profiles, flow_nodes, transactions, verifications, trust_score_history |
| 002 | `rls_policies` | Row-Level Security with `get_current_org_id()` and `get_current_user_id()` helpers |
| 003 | `storage` | Supabase Storage buckets (verifications, documents, logos) with RLS |
| 004 | `seed_dev` | Demo organization with sample data |
| 005 | `access_groups` | RBAC system: groups, members, node/transaction visibility, public embeds |
| 006 | `access_groups_rls` | RLS for the access control layer |
| 007 | `verifications_enhance` | Trust points, GPS coordinates, visibility levels, verification access log |
| 008 | `trust_points` | Trust score calculation functions, org verification stats, materialized view |
| 009 | `organization_chains` | Cross-org linking with activity log and chain trust aggregation |
| 010 | `chains_rls` | Bi-directional RLS for organization chains |
| 011 | `campaigns` | Donation campaigns, donations, donor updates with overflow handling |
| 012 | `campaigns_rls` | Public donation access, donor self-service policies |
| 013 | `seed_comprehensive` | 5 organizations, 20+ nodes, 16 transactions, 4 campaigns, 13 donations |

### Triggers

| Trigger | Table | Behavior |
|---------|-------|----------|
| `verifications_count_trigger` | verifications | Increment/decrement `transaction.verification_count` on insert/delete |
| `transactions_auto_verify` | transactions | Set `verification_status='verified'` when `verification_count >= 2` |
| `organizations_create_default_groups` | organizations | Create 5 default access groups (Board, Staff, Donors, Partners, Public) on org creation |
| `verification_trust_points_trigger` | verifications | Auto-assign trust points based on verification type |
| `donations_update_campaign_amount` | donations | Update `campaign.current_amount` when donation status changes |
| `campaigns_check_completion` | campaigns | Auto-complete campaign when `current_amount >= target_amount` (if overflow disabled) |

### Stored Functions

**Access Control:**
- `get_user_permissions(user_id, org_id)` -- aggregates permissions across all groups a user belongs to, returns JSONB
- `can_view_node(user_id, node_id)` -- evaluates node visibility based on group scope (`all`, `assigned`, `downstream`)
- `get_visibility_context(user_id, org_id)` -- returns visible/total counts for UI permission indicators

**Trust Score:**
- `get_trust_points(verification_type)` -- maps type to integer trust points
- `get_transaction_trust_points(txn_id)` -- sums verified trust points for a transaction
- `get_org_verification_stats(org_id)` -- returns total/verified transactions, verification rate, average trust per transaction

**Campaigns:**
- `generate_campaign_slug(name, org_id)` -- creates unique URL-safe public slug
- `get_campaign_stats(campaign_id)` -- donation count, unique donors, funding percentage, average donation
- `calculate_donation_impact(donation_id)` -- returns donor's percentage contribution to campaign
- `create_donor_node(org_id, name, email)` -- creates/finds a flow node for the donor (for visualization)

**Cross-Org Chains:**
- `get_chain_trust_score(chain_id)` -- aggregates trust scores across linked organizations (average)
- `get_org_chain_counts(org_id)` -- outgoing/incoming pending/active chain counts

### Indexing Strategy

- **Foreign keys indexed** for join performance across all tables
- **Date columns use DESC** for recent-first queries (`transaction_date DESC`, `created_at DESC`, `calculated_at DESC`)
- **Partial indexes** on nullable columns (`WHERE public_slug IS NOT NULL`, `WHERE is_public = TRUE`, `WHERE category IS NOT NULL`)
- **Composite indexes** for common query patterns (`organization_id, slug`)
- **Materialized view** (`org_trust_metrics`) for expensive trust score aggregations with unique index on `organization_id`

---

## Trust Score System

The trust score is a weighted multi-factor algorithm producing a 0-100 score that evaluates organizational financial transparency. The design parallels AI model evaluation: multiple independent signals are weighted and combined into a single interpretable metric.

### Components

| Factor | Weight | Measures |
|--------|--------|----------|
| Verification rate | 40% | Percentage of transactions with at least one verification |
| Documentation completeness | 30% | Breadth and quality of supporting documents attached |
| Consistency | 30% | Regularity of reporting, absence of anomalous patterns |

### Verification Types and Trust Points

Each verification type carries a weighted trust point value reflecting its evidentiary strength:

| Type | Points | Rationale |
|------|--------|-----------|
| `audit` | 15 | Third-party audit reports -- highest independent assurance |
| `bank_statement` | 10 | Bank-issued confirmation of transfers |
| `video` | 10 | Video evidence of activities/deliveries |
| `photo` | 8 | Photographic proof |
| `government_doc` | 8 | Official government documents |
| `biometric` | 7 | Fingerprint/facial recognition verification |
| `signature` | 7 | Signed attestations |
| `contract` | 6 | Executed agreements |
| `receipt` | 5 | Standard transaction receipts |
| `invoice` | 4 | Vendor invoices |
| `document` | 4 | General supporting documents |
| `gps` | 3 | GPS coordinates (supplementary, not standalone) |

### Mechanics

- **Auto-verification trigger**: transactions are automatically marked `verified` when they accumulate 2+ verifications
- **Trust points auto-assigned**: the `set_verification_trust_points()` trigger assigns points based on type at insert time
- **Organization-level aggregation**: `get_org_verification_stats()` computes total/verified transactions, verification rate, and average trust per transaction
- **Historical tracking**: `trust_score_history` records scores over time with component breakdown for trend analysis
- **Materialized view**: `org_trust_metrics` pre-computes expensive aggregations, refreshed via `refresh_org_trust_metrics()`

---

## Access Control and Security

### Authentication

WorkOS AuthKit provides enterprise-grade authentication:
- SSO via SAML and OIDC for nonprofit enterprise accounts
- Multi-factor authentication
- Server-side auth checks in route loaders (not client-side guards)
- Callback handler at `/api/auth/callback`

### Role-Based Access Control

Five default access groups are auto-created for each organization via database trigger:

| Group | Priority | Scope | Key Permissions |
|-------|----------|-------|----------------|
| Board | 10 | `all` | Full access: manage groups, members, edit nodes/transactions, view full proofs |
| Staff | 20 | `all` | Operational: edit nodes/transactions, view proofs (not full proofs) |
| Donors | 30 | `downstream` | Read downstream: view nodes, amounts, proofs (no editing) |
| Partners | 40 | `assigned` | Assigned nodes only: view nodes and amounts |
| Public | 100 | `all` | Minimal: view node names only |

### Permission Model

Nine granular permission flags stored as JSONB:
`can_view_nodes`, `can_view_amounts`, `can_view_proofs`, `can_view_executor`, `can_view_full_proofs`, `can_edit_nodes`, `can_edit_transactions`, `can_manage_members`, `can_manage_groups`

**Node scope** controls visibility breadth:
- `all` -- see every node in the organization
- `assigned` -- see only specifically assigned nodes (via `node_visibility` table)
- `downstream` -- see assigned nodes and all downstream nodes in the flow graph

**Time-bounded memberships**: `valid_from` / `valid_until` fields enable temporary access for auditors, consultants, or time-limited grants.

**Verification visibility levels**: `full` (original file), `thumbnail` (blurred preview), `summary` (metadata only), `hidden` (completely hidden) -- controlled per access group.

### Database-Level Security

- RLS policies on every table enforce organization isolation
- `get_current_org_id()` reads from PostgreSQL session settings (set by Supabase client)
- Verification access logging tracks who viewed what proof and when
- Field-level encryption via pgcrypto for sensitive data

---

## Performance Engineering

### Network Optimization (Target: <3s on 3G)

- **SSR via Nitro**: server-rendered first paint eliminates client-side data fetching waterfall
- **Route-level code splitting**: TanStack Router's file-based routes enable automatic lazy loading
- **TanStack Query caching**: resource-specific stale times (30s transactions, 5min nodes) minimize refetches
- **`refetchOnWindowFocus: false`**: prevents unnecessary network requests on tab switches

### Visual Performance

- **OKLch color system**: perceptually uniform color space for consistent contrast ratios across light/dark themes
- **Skeleton shimmer loading states**: gradient animation placeholders prevent layout shift
- **Theme flash prevention**: inline script in `<head>` detects and applies theme before React hydration
- **`font-variant-numeric: tabular-nums`**: prevents layout jitter in financial figures

### Mobile-First Design

- Desktop sidebar hidden on mobile; replaced with right-sliding drawer navigation
- `md:` breakpoint (768px) as primary desktop/mobile boundary
- Horizontal scroll for action ribbons on narrow viewports
- Touch-friendly target sizes (min h-8)

---

## Cross-Organization Features

### Organization Chains

Bi-directional linking between organizations with an approval workflow:

1. Source org sends chain request with optional message
2. Target org approves/rejects (setting `granted_group_id` to control access level)
3. Active chains allow source org to view target org's data at the granted permission level
4. Either party can revoke access
5. Full activity log tracks all state changes

**Chain trust score**: aggregates trust scores across linked organizations (average), enabling donors to evaluate entire funding chains rather than individual organizations.

### Donation Campaigns

- Public campaign pages at `/donate/{slug}` with customizable trust badges
- **Overflow handling**: `allow_overflow` flag, optional `overflow_campaign_id` for automatic redirection, custom consent messaging
- **Earmarking**: donors can designate funds for specific nodes within a campaign
- **Donor flow tracking**: `create_donor_node()` creates a node in the flow diagram for each donor, `calculate_donation_impact()` computes their percentage contribution
- **Donor updates**: 8 update types (donation_received through impact_report) delivered via email, SMS, push, or in-app
- **Campaign statistics**: real-time via `get_campaign_stats()` -- donation count, unique donors, funding percentage, average donation amount

### Public Embeds

- Embeddable campaign widgets at `/embed/campaign/{id}`
- Trust score badges at `/badge/trust/{orgId}`
- Configurable via `public_embed_config` table with `allow_embed` flag
- Public RLS policies allow unauthenticated access to active public campaigns

---

## Flow Visualization

Interactive SVG diagram rendering money movement through organization nodes:

- **Hierarchical layout**: nodes positioned by type (donor -> account -> department/project -> vendor/beneficiary)
- **Glassmorphic design**: backdrop-blur effects on nodes with animated bezier curve edges
- **Flowing particles**: animated dots along edges indicating transaction direction and volume
- **Selection interaction**: clicking a node highlights inflows/outflows, dimming unrelated paths
- **Color-coded node types**: Donor (primary), Account (success), Department (chart-4), Vendor (accent), Beneficiary (chart-3), Project (chart-5)

---

## Repository Structure

```
V2/
├── flowtrace/
│   ├── src/
│   │   ├── components/
│   │   │   ├── ui/              # shadcn/ui base components (button, card, badge, etc.)
│   │   │   ├── auth/            # Auth components
│   │   │   └── layout/          # AppShell, MobileNav, ThemeToggle
│   │   ├── lib/
│   │   │   ├── auth.ts          # WorkOS AuthKit integration
│   │   │   ├── org-context.tsx  # Multi-tenant organization context
│   │   │   ├── navigation.ts   # Route definitions with icons
│   │   │   └── utils.ts        # cn() helper
│   │   ├── routes/
│   │   │   ├── __root.tsx       # Root layout, QueryClient, AuthKitProvider
│   │   │   ├── _authenticated.tsx       # Protected route layout
│   │   │   ├── _authenticated/          # All protected routes
│   │   │   ├── api/             # API routes (auth callback)
│   │   │   └── index.tsx        # Public landing page
│   │   ├── styles.css           # OKLch theme, Tailwind v4, custom utilities
│   │   └── routeTree.gen.ts     # Auto-generated route tree
│   ├── supabase/
│   │   └── migrations/          # 13 progressive SQL migrations
│   └── vite.config.ts           # Vite 7 + TanStack Start + Nitro
└── Docs/                        # Architecture, backend, security documentation
```
