// The transparency records of the SpacetimeDB module as the web's types, and back. Pure, so the
// SDK glue (spacetime-transparency.ts) and the anonymous reads (public-sql.ts) share it and the
// tests need no connection.
//
// - Money: the module stores integer minor units (rust/crates/ledger-module/src/transparency/
//   types.rs, module docs); the web types hold major units. Every amount crosses here through
//   fromMajor/toMajor (money.ts).
// - Inputs: the reducers' JSON arguments are `deny_unknown_fields` (types.rs: NewCampaign,
//   CampaignPatch, NewDonation, NewDonorUpdate, OrganizationPatch), so each input keeps only the
//   fields its Rust type has; a null is dropped for a field Rust does not take as nullable.
// - Joins and figures the module does not keep (target node, donation details, counts,
//   underfunded campaigns, the members' campaign report) are built here the way the demo API
//   builds them (flowtrace/src/lib/demo/api.ts:1040-1443).
// - Flows and chains (RW2, ADR 0005): the transaction list with its node references, the flow map
//   data the `generate-flow-data` edge function served (the module has none, so it is computed
//   here as flows.rs `flow_data` and the demo's getFlowDataAPI do, api.ts:475-566), the proof
//   figures, chain counts and the chain trust score, and the inputs of the flow and chain
//   reducers (flows.rs:286-440).
// - Proofs, access groups, the trust score and the embed config (RW3, ADR 0004/0005): the proof
//   record with its R2 keys as the web's `proof:<key>` references (proofs.ts), the inputs of the
//   access group, proof and embed reducers (types.rs:654-800, flows.rs:443), and the trust row
//   with its four weighted components (trust.rs:149).
import { ledgerOrgId } from '../spacetime-session'
import { fromMajor, toMajor } from './money'
import type { SpacetimeMembership } from './spacetime-backend'
import type {
  AccessGroup,
  AccessGroupPermissions,
  Campaign,
  CampaignCounts,
  CampaignStats,
  CampaignWithDetails,
  ChainActivityLog,
  ChainCounts,
  ChainTrustScore,
  Donation,
  DonationWithDetails,
  DonorUpdate,
  DonorUpdateWithDetails,
  FlowDataEdge,
  FlowDataNode,
  FlowDataResult,
  FlowNode,
  GroupMember,
  NodeVisibility,
  OrgMember,
  OrgStats,
  Organization,
  OrganizationChainWithDetails,
  Profile,
  PublicEmbedConfig,
  Transaction,
  TransactionWithNodes,
  TrustComponent,
  TrustScoreHistory,
  TrustScoreSummary,
  Verification,
  VerificationAccessLog,
} from '../types'
import type { CampaignReport } from '../demo/api'

/** A doc row of a `my_*` view: the record as JSON, keyed by organisation and id. */
export interface DocRowLike {
  key: string
  orgId: string
  id: string
  seq: bigint
  data: string
}

type Json = Record<string, unknown>

const parse = (data: string): Json => JSON.parse(data) as Json

function majorOf(rec: Json, keys: Array<string>): Json {
  const out = { ...rec }
  for (const k of keys) {
    const v = out[k]
    if (typeof v === 'number') out[k] = toMajor(v)
  }
  return out
}

function minorOf(rec: Json, keys: Array<string>): Json {
  const out = { ...rec }
  for (const k of keys) {
    const v = out[k]
    if (typeof v === 'number') out[k] = fromMajor(v)
  }
  return out
}

/**
 * The fields of `rec` that a Rust input type has. `nullable` lists the ones it takes as null
 * (Option or Nullish); any other null is left out so the field's default applies.
 */
function pick(
  rec: object,
  fields: Array<string>,
  nullable: Array<string>,
): Json {
  const out: Json = {}
  for (const k of fields) {
    const v = (rec as Json)[k]
    if (v === undefined) continue
    if (v === null && !nullable.includes(k)) continue
    out[k] = v
  }
  return out
}

// ---------------------------------------------------------------------------
// Records out of the module
// ---------------------------------------------------------------------------

const CAMPAIGN_MONEY = ['target_amount', 'current_amount']
const STAT_MONEY = [
  'total_raised',
  'avg_donation',
  'largest_donation',
  'smallest_donation',
]
const DONATION_MONEY = ['amount', 'amount_usd']

export const decodeCampaign = (row: Pick<DocRowLike, 'data'>) =>
  majorOf(parse(row.data), CAMPAIGN_MONEY) as unknown as Campaign

/** A `campaign_stat` row (types.rs CampaignStat), without the two day counts. */
export type StoredStat = Omit<
  CampaignStats,
  'days_remaining' | 'days_active'
> & {
  campaign_id: string
}

export const decodeStat = (row: Pick<DocRowLike, 'data'>) =>
  majorOf(parse(row.data), STAT_MONEY) as unknown as StoredStat

export const decodeDonation = (row: Pick<DocRowLike, 'data'>) =>
  majorOf(parse(row.data), DONATION_MONEY) as unknown as Donation

export const decodeDonorUpdate = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as DonorUpdate

export const decodeFlowNode = (row: Pick<DocRowLike, 'data'>) =>
  majorOf(parse(row.data), ['balance']) as unknown as FlowNode

export const decodeTransaction = (row: Pick<DocRowLike, 'data'>) =>
  majorOf(parse(row.data), ['amount']) as unknown as Transaction

// ---------------------------------------------------------------------------
// Inputs to the reducers (types.rs:810-1040)
// ---------------------------------------------------------------------------

const NEW_CAMPAIGN = [
  'name',
  'description',
  'short_description',
  'image_url',
  'target_amount',
  'currency',
  'target_node_id',
  'status',
  'allow_overflow',
  'overflow_campaign_id',
  'overflow_consent_message',
  'starts_at',
  'ends_at',
  'is_public',
  'public_slug',
  'category',
  'tags',
  'metadata',
]
const NEW_CAMPAIGN_NULLABLE = [
  'description',
  'short_description',
  'image_url',
  'target_node_id',
  'overflow_campaign_id',
  'overflow_consent_message',
  'ends_at',
  'public_slug',
  'category',
  'tags',
]

/** `NewCampaign` (types.rs:810): no organisation, author or raised amount; minor units. */
export function campaignInput(c: Partial<Campaign>): Json {
  return minorOf(pick(c, NEW_CAMPAIGN, NEW_CAMPAIGN_NULLABLE), CAMPAIGN_MONEY)
}

/** `CampaignPatch` (types.rs:871): the same fields, `current_amount` never; minor units. */
export function campaignPatch(updates: Partial<Campaign>): Json {
  return minorOf(
    pick(updates, NEW_CAMPAIGN, NEW_CAMPAIGN_NULLABLE),
    CAMPAIGN_MONEY,
  )
}

const NEW_DONATION = [
  'campaign_id',
  'donor_email',
  'donor_name',
  'donor_phone',
  'is_anonymous',
  'amount',
  'currency',
  'amount_usd',
  'exchange_rate',
  'earmarked_node_id',
  'earmark_notes',
  'allow_reallocation',
  'flow_node_id',
  'payment_method',
  'payment_reference',
  'payment_status',
  'transaction_id',
  'source',
  'utm_source',
  'utm_medium',
  'utm_campaign',
  'metadata',
]
const NEW_DONATION_NOT_NULL = [
  'campaign_id',
  'is_anonymous',
  'amount',
  'currency',
  'allow_reallocation',
  'payment_status',
  'metadata',
]

/** `NewDonation` (types.rs:932); minor units. */
export function donationInput(d: Partial<Donation>): Json {
  return minorOf(
    pick(
      d,
      NEW_DONATION,
      NEW_DONATION.filter((k) => !NEW_DONATION_NOT_NULL.includes(k)),
    ),
    DONATION_MONEY,
  )
}

const NEW_DONOR_UPDATE = [
  'donation_id',
  'campaign_id',
  'update_type',
  'title',
  'message',
  'image_url',
  'related_node_id',
  'related_transaction_id',
  'delivery_method',
  'metadata',
]

/** `NewDonorUpdate` (types.rs:1009). */
export function donorUpdateInput(u: Partial<DonorUpdate>): Json {
  return pick(u, NEW_DONOR_UPDATE, [
    'campaign_id',
    'image_url',
    'related_node_id',
    'related_transaction_id',
  ])
}

/** `OrganizationPatch` (types.rs:620): what Settings edits, plus the module's profile fields. */
export function organizationPatch(updates: Partial<Organization> & Json): Json {
  return pick(
    updates,
    [
      'name',
      'type',
      'country',
      'currency',
      'public_slug',
      'description',
      'logo_key',
      'is_public',
    ],
    ['type', 'public_slug', 'description', 'logo_key'],
  )
}

/**
 * The row a reducer just created: reducers return nothing, so it is the row of this
 * organisation that was not there before and matches what was sent, the newest (highest `seq`)
 * first.
 */
export function resolveCreated<T>(
  rows: Iterable<DocRowLike>,
  before: Set<string>,
  orgId: string,
  matches: (record: T) => boolean,
  decode: (row: DocRowLike) => T = (r) => parse(r.data) as T,
): T | undefined {
  const fresh = [...rows]
    .filter((r) => r.orgId === orgId && !before.has(r.key))
    .sort((a, b) => (a.seq < b.seq ? 1 : a.seq > b.seq ? -1 : 0))
  for (const r of fresh) {
    const rec = decode(r)
    if (matches(rec)) return rec
  }
  return undefined
}

// ---------------------------------------------------------------------------
// Joins and figures
// ---------------------------------------------------------------------------

const DAY = 24 * 60 * 60 * 1000

const nodeRef = (nodes: Array<FlowNode>, id: string | null | undefined) => {
  const n = id ? nodes.find((x) => x.id === id) : undefined
  return n ? { id: n.id, name: n.name, type: n.type } : null
}

export const withTarget = (
  c: Campaign,
  nodes: Array<FlowNode>,
): CampaignWithDetails => ({
  ...c,
  target_node: nodeRef(nodes, c.target_node_id),
})

/**
 * The stat row plus `days_remaining` and `days_active`, which depend on the day they are read
 * (rust/crates/ledger-module/src/transparency/stats.rs:155-170, api.ts:1157-1163). A campaign
 * with no stat row yet has no confirmed gifts.
 */
export function campaignStats(
  c: Pick<Campaign, 'starts_at' | 'ends_at'>,
  stat: StoredStat | undefined,
  nowMs: number,
): CampaignStats {
  const end = c.ends_at ? new Date(c.ends_at).getTime() : NaN
  const start = new Date(c.starts_at).getTime()
  return {
    donation_count: stat?.donation_count ?? 0,
    unique_donors: stat?.unique_donors ?? 0,
    total_raised: stat?.total_raised ?? 0,
    funding_percentage: stat?.funding_percentage ?? 0,
    avg_donation: stat?.avg_donation ?? 0,
    largest_donation: stat?.largest_donation ?? 0,
    smallest_donation: stat?.smallest_donation ?? 0,
    days_remaining: Number.isFinite(end)
      ? Math.max(0, Math.ceil((end - nowMs) / DAY))
      : null,
    days_active: Number.isFinite(start)
      ? Math.max(0, Math.floor((nowMs - start) / DAY))
      : 0,
  }
}

/** `getCampaignCounts` (api.ts:1167-1182). */
export function campaignCounts(
  campaigns: Array<Campaign>,
  donations: Array<Donation>,
): CampaignCounts {
  const ids = new Set(campaigns.map((c) => c.id))
  return {
    total_campaigns: campaigns.length,
    active_campaigns: campaigns.filter((c) => c.status === 'active').length,
    completed_campaigns: campaigns.filter((c) => c.status === 'completed')
      .length,
    total_raised: campaigns.reduce((s, c) => s + c.current_amount, 0),
    total_donations: donations.filter(
      (d) => ids.has(d.campaign_id) && d.payment_status === 'confirmed',
    ).length,
  }
}

/** `getUnderfundedCampaigns` (api.ts:1184-1199). */
export function underfundedCampaigns(
  campaigns: Array<Campaign>,
  limit: number,
): Array<Campaign> {
  const pct = (c: Campaign) =>
    c.target_amount ? c.current_amount / c.target_amount : 1
  return campaigns
    .filter((c) => c.status === 'active' && pct(c) < 1)
    .sort((a, b) => pct(a) - pct(b))
    .slice(0, limit)
}

/** `withCampaign` (api.ts:1205-1220). */
export function donationWithDetails(
  d: Donation,
  campaigns: Array<Campaign>,
  nodes: Array<FlowNode>,
): DonationWithDetails {
  const c = campaigns.find((x) => x.id === d.campaign_id)
  return {
    ...d,
    campaign: c
      ? { id: c.id, name: c.name, public_slug: c.public_slug, status: c.status }
      : {
          id: d.campaign_id,
          name: 'Removed campaign',
          public_slug: null,
          status: 'cancelled',
        },
    earmarked_node: nodeRef(nodes, d.earmarked_node_id),
  }
}

/** `updateDetails` (api.ts:1291-1313). */
export function donorUpdateWithDetails(
  u: DonorUpdate,
  donations: Array<Donation>,
  campaigns: Array<Campaign>,
  nodes: Array<FlowNode>,
): DonorUpdateWithDetails {
  const d = donations.find((x) => x.id === u.donation_id)
  const c = u.campaign_id
    ? campaigns.find((x) => x.id === u.campaign_id)
    : undefined
  return {
    ...u,
    donation: d
      ? {
          id: d.id,
          donor_email: d.donor_email,
          donor_name: d.donor_name,
          amount: d.amount,
        }
      : { id: u.donation_id, donor_email: null, donor_name: null, amount: 0 },
    campaign: c ? { id: c.id, name: c.name } : null,
    related_node: nodeRef(nodes, u.related_node_id),
  }
}

const byDateDesc = (a: string, b: string) => (a < b ? 1 : a > b ? -1 : 0)

/** `getCampaignReport` (api.ts:1381-1443), for a member reading their own campaign. */
export function campaignReport(
  campaign: CampaignWithDetails,
  organization: CampaignReport['organization'],
  nodes: Array<FlowNode>,
  transactions: Array<Transaction>,
): CampaignReport {
  const txns = transactions.filter((t) => t.verification_status !== 'rejected')
  const sum = (rows: Array<Transaction>) =>
    rows.reduce((s, t) => s + t.amount, 0)
  const earmarks = nodes
    .filter((n) => n.type === 'project' || n.type === 'department')
    .map((n) => {
      const inflow = txns.filter((t) => t.to_node_id === n.id)
      return {
        id: n.id,
        name: n.name,
        type: n.type,
        received: sum(inflow),
        evidenced: sum(
          inflow.filter((t) => t.verification_status === 'verified'),
        ),
        is_target: n.id === campaign.target_node_id,
      }
    })
    .sort(
      (a, b) =>
        Number(b.is_target) - Number(a.is_target) || b.received - a.received,
    )
  const programme = new Set(earmarks.map((e) => e.id))
  const spending: Array<TransactionWithNodes> = txns
    .filter(
      (t) =>
        (t.to_node_id && programme.has(t.to_node_id)) ||
        (t.from_node_id && programme.has(t.from_node_id)),
    )
    .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
    .slice(0, 6)
    .map((t) => ({
      ...t,
      from_node: nodeRef(nodes, t.from_node_id),
      to_node: nodeRef(nodes, t.to_node_id),
    }))
  const target = earmarks.find((e) => e.is_target)
  return {
    campaign,
    organization,
    evidenced_amount: target
      ? Math.min(target.evidenced, campaign.current_amount)
      : null,
    earmarks,
    spending,
  }
}

// ---------------------------------------------------------------------------
// Organisation and person
// ---------------------------------------------------------------------------

/**
 * The organisation from its `my_organization_profile` JSON and `my_trust_score` JSON. Before the
 * first profile update there is no row; the module then defaults the name to the organisation id
 * and the currency to the books' base currency (plan.rs:132-165, update_organization).
 */
export function organizationFrom(
  orgId: string,
  profileJson: string | undefined,
  baseCurrency: string,
  trustJson: string | undefined,
): Organization {
  const p = profileJson ? parse(profileJson) : {}
  const trust = trustJson ? parse(trustJson) : {}
  const str = (k: string, fallback: string) =>
    typeof p[k] === 'string' ? p[k] : fallback
  return {
    id: orgId,
    name: str('name', orgId),
    type: (p.type as Organization['type'] | undefined) ?? null,
    country: str('country', ''),
    currency: str('currency', baseCurrency),
    trust_score: typeof trust.score === 'number' ? trust.score : 0,
    verified: p.verified === true,
    public_slug: (p.public_slug as string | null | undefined) ?? null,
    // The module keeps an R2 object key (P13), not a URL.
    logo_url: null,
    is_public: p.is_public === true,
    created_at: str('created_at', ''),
    updated_at: str('updated_at', ''),
  }
}

export interface ModuleOrgContext {
  orgId: string
  membership: SpacetimeMembership
  role: Profile['role']
}

/**
 * The organisation and role from the member rows: the organisation as ledgerOrgId picks it (the
 * same one the books screens read), the role from the member's rights. A member who can close
 * periods is the module's admin or manager (plan.rs:66-74, Ctx::manager), so `admin` here; one
 * who can post or approve is a member; anyone else a viewer. Null with no membership.
 */
export function orgContextFrom(input: {
  memberships: Array<SpacetimeMembership>
  configured: string | undefined
  contextOrgId: string | null
}): ModuleOrgContext | null {
  const orgId = ledgerOrgId({
    kind: 'spacetimedb',
    contextOrgId: input.contextOrgId,
    configured: input.configured,
    memberOrgs: input.memberships.map((m) => m.orgId),
  })
  const membership = input.memberships.find((m) => m.orgId === orgId)
  if (!orgId || !membership) return null
  const a = membership.actor
  const role: Profile['role'] = a.can_close
    ? 'admin'
    : a.can_post || a.can_approve
      ? 'member'
      : 'viewer'
  return { orgId, membership, role }
}

// ---------------------------------------------------------------------------
// Flow nodes and transactions (RW2; flows.rs:286-440, ADR 0005)
// ---------------------------------------------------------------------------

/** `NewFlowNode` (flows.rs:286): name, type and metadata; the module sets the balance to zero. */
export function flowNodeInput(n: Partial<FlowNode>): Json {
  return pick(n, ['name', 'type', 'metadata'], [])
}

const TRANSACTION_FIELDS = [
  'from_node_id',
  'to_node_id',
  'amount',
  'currency',
  'exchange_rate',
  'amount_usd',
  'description',
  'category',
  'reference',
  'notes',
  'verification_status',
  'transaction_date',
]
const TRANSACTION_NULLABLE = [
  'from_node_id',
  'to_node_id',
  'exchange_rate',
  'amount_usd',
  'category',
  'reference',
  'notes',
]
const TRANSACTION_MONEY = ['amount', 'amount_usd']

/**
 * `NewTransaction` (flows.rs:320): no organisation, `created_by` (the caller) or proof count;
 * minor units. Creating it does not post it to the books (ADR 0005, difference 1).
 */
export function transactionInput(t: Partial<Transaction>): Json {
  return minorOf(
    pick(t, TRANSACTION_FIELDS, TRANSACTION_NULLABLE),
    TRANSACTION_MONEY,
  )
}

/** `TransactionPatch` (flows.rs:358): the same fields; the proof count and author never. */
export function transactionPatch(u: Partial<Transaction>): Json {
  return minorOf(
    pick(u, TRANSACTION_FIELDS, TRANSACTION_NULLABLE),
    TRANSACTION_MONEY,
  )
}

/** `NodeVisibilitySettings` (flows.rs:401): `null` means the group's own setting. */
export function nodeVisibilitySettings(s: Partial<NodeVisibility>): Json {
  return pick(
    s,
    [
      'can_view',
      'can_view_amounts',
      'can_view_proofs',
      'can_view_executor',
      'display_name',
    ],
    [
      'can_view_amounts',
      'can_view_proofs',
      'can_view_executor',
      'display_name',
    ],
  )
}

export const decodeNodeVisibility = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as NodeVisibility

export type NodeVisibilityWithGroup = NodeVisibility & {
  group: Pick<AccessGroup, 'id' | 'name' | 'slug' | 'color'>
}

/** `getNodeVisibility` (api.ts:690-705): each row with its group, or a removed-group stand-in. */
export function nodeVisibilityWithGroups(
  rows: Array<NodeVisibility>,
  groups: Array<AccessGroup>,
): Array<NodeVisibilityWithGroup> {
  return rows.map((v) => {
    const g = groups.find((x) => x.id === v.group_id)
    return {
      ...v,
      group: g
        ? { id: g.id, name: g.name, slug: g.slug, color: g.color }
        : {
            id: v.group_id,
            name: 'Removed group',
            slug: 'removed',
            color: '#94a3b8',
          },
    }
  })
}

/** `withNodes` (api.ts:196-200, flows.rs:1029). */
export const withNodes = (
  t: Transaction,
  nodes: Array<FlowNode>,
): TransactionWithNodes => ({
  ...t,
  from_node: nodeRef(nodes, t.from_node_id),
  to_node: nodeRef(nodes, t.to_node_id),
})

/** `getTransactions` (api.ts:202-209): newest date first, equal dates in recorded order. */
export function transactionsWithNodes(
  transactions: Array<Transaction>,
  nodes: Array<FlowNode>,
): Array<TransactionWithNodes> {
  return [...transactions]
    .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
    .map((t) => withNodes(t, nodes))
}

/** What the flow figures read of a proof (the module's `Verification`, types.rs:282). */
export type ProofLike = Pick<Verification, 'status' | 'trust_points'> & {
  transaction_id: string | null
}

/**
 * `getFlowDataAPI` (api.ts:475-566; flows.rs:1110 `flow_data`): every transaction that is not
 * rejected, within the dates when given (compared as `YYYY-MM-DD`), summed onto its nodes and
 * onto one edge per ordered node pair, with each node's proof figures. `currency` is the
 * organisation's.
 */
export function flowData(input: {
  nodes: Array<FlowNode>
  transactions: Array<Transaction>
  proofs: Array<ProofLike>
  currency: string
  startDate?: string
  endDate?: string
}): FlowDataResult {
  const { startDate, endDate } = input
  const txns = input.transactions.filter(
    (t) =>
      t.verification_status !== 'rejected' &&
      (!startDate || t.transaction_date >= startDate.slice(0, 10)) &&
      (!endDate || t.transaction_date <= endDate.slice(0, 10)),
  )
  const nodes = new Map<string, FlowDataNode>()
  const touch = (id: string | null) => {
    if (!id || nodes.has(id)) return
    const n = input.nodes.find((x) => x.id === id)
    if (!n) return
    nodes.set(id, {
      id: n.id,
      name: n.name,
      type: n.type,
      inflow: 0,
      outflow: 0,
      balance: n.balance,
      verification_count: 0,
      verified_count: 0,
      pending_count: 0,
      rejected_count: 0,
      trust_points: 0,
    })
  }
  const edges = new Map<string, FlowDataEdge>()
  for (const t of txns) {
    touch(t.from_node_id)
    touch(t.to_node_id)
    const from = t.from_node_id ? nodes.get(t.from_node_id) : undefined
    const to = t.to_node_id ? nodes.get(t.to_node_id) : undefined
    if (from) from.outflow += t.amount
    if (to) to.inflow += t.amount
    const proofs = input.proofs.filter((p) => p.transaction_id === t.id)
    const count = (s: ProofLike['status']) =>
      proofs.filter((p) => p.status === s).length
    for (const n of [from, to]) {
      if (!n) continue
      n.verification_count = (n.verification_count ?? 0) + proofs.length
      n.verified_count = (n.verified_count ?? 0) + count('verified')
      n.pending_count = (n.pending_count ?? 0) + count('pending')
      n.rejected_count = (n.rejected_count ?? 0) + count('rejected')
      n.trust_points =
        (n.trust_points ?? 0) + proofs.reduce((s, p) => s + p.trust_points, 0)
    }
    if (t.from_node_id && t.to_node_id) {
      const key = `${t.from_node_id}->${t.to_node_id}`
      const edge = edges.get(key)
      if (edge) {
        edge.value += t.amount
        edge.count += 1
      } else {
        edges.set(key, {
          source: t.from_node_id,
          target: t.to_node_id,
          value: t.amount,
          count: 1,
          currency: t.currency,
        })
      }
    }
  }
  const edgeList = [...edges.values()]
  return {
    nodes: [...nodes.values()],
    edges: edgeList,
    summary: {
      total_nodes: nodes.size,
      total_edges: edgeList.length,
      total_flow: edgeList.reduce((s, e) => s + e.value, 0),
      transaction_count: txns.length,
      currency: input.currency,
    },
  }
}

/** `getOrganizationStats` (api.ts:120-148): transaction states and the proof figures. */
export function orgStats(
  transactions: Array<Transaction>,
  allProofs: Array<ProofLike>,
): OrgStats {
  const ids = new Set(transactions.map((t) => t.id))
  const proofs = allProofs.filter(
    (p) => p.transaction_id !== null && ids.has(p.transaction_id),
  )
  const total = transactions.length
  const withState = (s: Transaction['verification_status']) =>
    transactions.filter((t) => t.verification_status === s).length
  const verified = withState('verified')
  const withProofs = new Set(proofs.map((p) => p.transaction_id)).size
  const proofsIn = (s: ProofLike['status']) =>
    proofs.filter((p) => p.status === s).length
  return {
    total,
    verified,
    pending: withState('pending'),
    flagged: withState('flagged'),
    verificationRate: total > 0 ? Math.round((verified / total) * 100) : 0,
    proofStats: {
      totalProofs: proofs.length,
      verifiedProofs: proofsIn('verified'),
      pendingProofs: proofsIn('pending'),
      rejectedProofs: proofsIn('rejected'),
      totalTrustPoints: proofs.reduce((sum, p) => sum + p.trust_points, 0),
      transactionsWithProofs: withProofs,
      proofCoverageRate: total > 0 ? Math.round((withProofs / total) * 100) : 0,
    },
  }
}

// ---------------------------------------------------------------------------
// Organisation chains (chains.rs, ADR 0005)
// ---------------------------------------------------------------------------

interface ChainOrgRef {
  id: string
  name: string
  logo_key?: string | null
  trust_score: number
  type: Organization['type']
  country: string
}

const chainOrg = (o: ChainOrgRef) => ({
  id: o.id,
  name: o.name,
  // The module keeps an R2 object key (P13), not a URL.
  logo_url: null,
  trust_score: o.trust_score,
  type: o.type,
  country: o.country,
})

/**
 * A `my_organization_chain` row (`ChainDetails`, chains.rs:637): the chain with both
 * organisations' name, type, country and trust score, the target's linked node and granted group.
 */
export function decodeChain(
  row: Pick<DocRowLike, 'data'>,
): OrganizationChainWithDetails {
  const c = parse(row.data) as unknown as OrganizationChainWithDetails & {
    source_org: ChainOrgRef
    target_org: ChainOrgRef
  }
  return {
    ...c,
    source_org: chainOrg(c.source_org),
    target_org: chainOrg(c.target_org),
  }
}

export const decodeChainActivity = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as ChainActivityLog

/** `getOutgoingChains` / `getIncomingChains` (api.ts:842-858): newest request first. */
export function chainsOf(
  chains: Array<OrganizationChainWithDetails>,
  orgId: string,
  side: 'outgoing' | 'incoming',
): Array<OrganizationChainWithDetails> {
  return chains
    .filter((c) =>
      side === 'outgoing'
        ? c.source_org_id === orgId
        : c.target_org_id === orgId,
    )
    .sort((a, b) => byDateDesc(a.requested_at, b.requested_at))
}

/** `getChainCounts` (api.ts:1000-1016, chains.rs:505). */
export function chainCounts(
  chains: Array<
    Pick<
      OrganizationChainWithDetails,
      'source_org_id' | 'target_org_id' | 'status'
    >
  >,
  orgId: string,
): ChainCounts {
  const n = (side: 'source_org_id' | 'target_org_id', status: string) =>
    chains.filter((c) => c[side] === orgId && c.status === status).length
  return {
    outgoing_pending: n('source_org_id', 'pending'),
    outgoing_active: n('source_org_id', 'active'),
    incoming_pending: n('target_org_id', 'pending'),
    incoming_active: n('target_org_id', 'active'),
  }
}

/**
 * `getChainTrustScore` (api.ts:1018-1031): the rounded mean of the two organisations' scores,
 * which the module's chain row carries (chains.rs:563, ADR 0005 difference 6); null for an
 * unknown chain.
 */
export function chainTrustScore(
  chain:
    | Pick<OrganizationChainWithDetails, 'source_org' | 'target_org'>
    | undefined,
): ChainTrustScore | null {
  if (!chain) return null
  const source = chain.source_org.trust_score
  const target = chain.target_org.trust_score
  return {
    aggregated_score: Math.round((source + target) / 2),
    org_count: 2,
    scores: { source_org: source, target_org: target },
  }
}

export type OrganizationMatch = Pick<
  Organization,
  'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'
>

/**
 * `searchOrganizations` (api.ts:177-185) over the organisations the module publishes
 * (`public_organization`): the module has no directory of every organisation, so only public
 * ones can be found. Name contains the query, any case; at most ten; never the caller's own.
 */
export function matchOrganizations(
  published: Array<
    Pick<Organization, 'id' | 'name' | 'trust_score' | 'type' | 'country'>
  >,
  query: string,
  excludeOrgId: string,
): Array<OrganizationMatch> {
  const q = query.toLowerCase()
  return published
    .filter((o) => o.id !== excludeOrgId && o.name.toLowerCase().includes(q))
    .slice(0, 10)
    .map((o) => ({
      id: o.id,
      name: o.name,
      logo_url: null,
      trust_score: o.trust_score,
      type: o.type,
      country: o.country,
    }))
}

// ---------------------------------------------------------------------------
// Proofs (RW3; types.rs:282, 743; plan.rs:683-840)
// ---------------------------------------------------------------------------

/** The prefix of a proofs Worker reference (proofs.ts `proofRef`). */
const PROOF_REF = 'proof:'
const SHA256 = /^[0-9a-f]{64}$/

const keyOfRef = (ref: unknown) =>
  typeof ref === 'string' && ref.startsWith(PROOF_REF) && ref.length > PROOF_REF.length
    ? ref.slice(PROOF_REF.length)
    : null

/**
 * A `my_verification` row as the web's proof: the R2 keys become `proof:<key>` references, so
 * the viewer trades them for signed URLs (proofs.ts `useProofSrc`). The module has already cut
 * the row down to the reader's level (plan.rs:639 `redact`) and says which in `visible_as`.
 */
export function decodeVerification(row: Pick<DocRowLike, 'data'>): Verification {
  const v = parse(row.data)
  const ref = (k: string) =>
    typeof v[k] === 'string' && v[k] ? PROOF_REF + v[k] : null
  return {
    ...(v as unknown as Verification),
    transaction_id: (v.transaction_id as string | null | undefined) ?? '',
    file_url: ref('object_key'),
    thumbnail_url: ref('thumbnail_key'),
    published: v.published === true,
  }
}

/** `getVerifications` (api.ts:276-282): a transaction's proofs, newest first. */
export const proofsOf = (proofs: Array<Verification>, transactionId: string) =>
  proofs
    .filter((v) => v.transaction_id === transactionId)
    .sort((a, b) => byDateDesc(a.created_at, b.created_at))

const NEW_VERIFICATION = [
  'transaction_id',
  'type',
  'ocr_data',
  'metadata',
  'status',
  'visibility_level',
  'executor_name',
  'gps_latitude',
  'gps_longitude',
  'gps_accuracy',
  'location_name',
  'description',
]
const NEW_VERIFICATION_NOT_NULL = ['type', 'metadata', 'status', 'visibility_level']

/**
 * `NewVerification` (types.rs:743): the proof as the web records it, its file a `proof:<key>`
 * reference to the Worker's object `org/<org>/<sha256>` (P13). The module keeps no other kind
 * of file, so a session-local file (a blob URL, when VITE_PROOFS_URL is unset) is refused here
 * rather than dropped. `verified_by` and `created_by` are the caller's, so they are not sent.
 */
export function verificationInput(
  v: Omit<Verification, 'id' | 'created_at' | 'verified_at' | 'trust_points'>,
): Json {
  const out = pick(
    v,
    NEW_VERIFICATION,
    NEW_VERIFICATION.filter((k) => !NEW_VERIFICATION_NOT_NULL.includes(k)),
  )
  const meta = v.metadata as Json
  const key =
    keyOfRef(v.file_url) ??
    (typeof meta.object_key === 'string' ? meta.object_key : null)
  if (v.file_url && !key)
    throw new Error(
      'This file is only in your browser. Proof files are kept by the proof store; set VITE_PROOFS_URL to attach them.',
    )
  if (key) {
    const tail = key.split('/').at(-1) ?? ''
    const hash = typeof meta.sha256 === 'string' ? meta.sha256 : tail
    out.object_key = key
    if (SHA256.test(hash)) out.sha256 = hash
  }
  const thumb = keyOfRef(v.thumbnail_url)
  if (thumb) out.thumbnail_key = thumb
  return out
}

export const decodeAccess = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as VerificationAccessLog

// ---------------------------------------------------------------------------
// Access groups and members (RW3; types.rs:153-200, 654-740)
// ---------------------------------------------------------------------------

const ACCESS_GROUP_FIELDS = [
  'name',
  'slug',
  'description',
  'icon',
  'color',
  'permissions',
  'node_scope',
  'is_default',
  'is_public',
  'priority',
]
const PERMISSION_KEYS: Array<keyof AccessGroupPermissions> = [
  'can_view_nodes',
  'can_view_amounts',
  'can_view_proofs',
  'can_view_executor',
  'can_view_full_proofs',
  'can_edit_nodes',
  'can_edit_transactions',
  'can_manage_members',
  'can_manage_groups',
]

/** The nine flags only: `AccessGroupPermissions` (types.rs:100) takes no others. */
function permissionsOf(p: Partial<AccessGroupPermissions>): AccessGroupPermissions {
  const out = {} as AccessGroupPermissions
  for (const k of PERMISSION_KEYS) out[k] = p[k] === true
  return out
}

function withPermissions(out: Json): Json {
  if (out.permissions && typeof out.permissions === 'object')
    out.permissions = permissionsOf(out.permissions as Partial<AccessGroupPermissions>)
  return out
}

/** `NewAccessGroup` (types.rs:654): no organisation; the module sets ids and times. */
export function accessGroupInput(g: Partial<AccessGroup>): Json {
  return withPermissions(pick(g, ACCESS_GROUP_FIELDS, ['description']))
}

/** `AccessGroupPatch` (types.rs:683): the same fields; permissions go whole. */
export function accessGroupPatch(u: Partial<AccessGroup>): Json {
  return withPermissions(pick(u, ACCESS_GROUP_FIELDS, ['description']))
}

/** `NewGroupMember` (types.rs:719): `granted_by` is the caller, so it is not sent. */
export function groupMemberInput(m: Partial<GroupMember>): Json {
  return pick(
    m,
    ['group_id', 'user_id', 'user_email', 'valid_from', 'valid_until', 'notes'],
    ['user_id', 'user_email', 'valid_from', 'valid_until', 'notes'],
  )
}

export const decodeAccessGroup = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as AccessGroup
export const decodeGroupMember = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as GroupMember

/** `getAccessGroups` (api.ts:570-576): lowest priority first. */
export const accessGroupsInOrder = (groups: Array<AccessGroup>) =>
  [...groups].sort((a, b) => a.priority - b.priority)

/**
 * `getGroupMembers` (api.ts:625-632): a group's members, newest first, each with the person
 * behind it. The module keeps only a hash of an email (`set_profile`), so the profile carries the
 * member's name from the member row and no address.
 */
export type GroupMemberWithProfile = GroupMember & {
  profile: Pick<Profile, 'id' | 'email' | 'full_name'> | null
}

export function groupMembersWithProfiles(
  members: Array<GroupMember>,
  groupId: string,
  people: Array<OrgMember>,
): Array<GroupMemberWithProfile> {
  return members
    .filter((m) => m.group_id === groupId)
    .sort((a, b) => byDateDesc(a.created_at, b.created_at))
    .map((m) => {
      const who = m.user_id ? people.find((p) => p.user_id === m.user_id) : undefined
      return {
        ...m,
        profile: m.user_id
          ? { id: m.user_id, email: '', full_name: who?.name ?? null }
          : null,
      }
    })
}

// ---------------------------------------------------------------------------
// Trust score (RW3; trust.rs:149-200, P16)
// ---------------------------------------------------------------------------

const TRUST_KEYS: Array<TrustComponent['key']> = [
  'verification',
  'documentation',
  'consistency',
  'model',
]

/**
 * The `my_trust_score` row (`TrustScore`, trust.rs:149): the total, its grade and the four
 * components with their weights (32/24/24/20, P16). Null before the module has computed one.
 */
export function trustSummaryFrom(json: string | undefined): TrustScoreSummary | null {
  if (!json) return null
  const t = parse(json)
  const values = (t.components ?? {}) as Record<string, unknown>
  const weights = (t.weights ?? {}) as Record<string, unknown>
  const model = (t.model ?? {}) as Record<string, unknown>
  return {
    score: typeof t.score === 'number' ? t.score : 0,
    grade: typeof t.grade === 'string' ? t.grade : 'F',
    components: TRUST_KEYS.filter((k) => typeof weights[k] === 'number').map((k) => ({
      key: k,
      value: typeof values[k] === 'number' ? values[k] : 0,
      weight: weights[k] as number,
      ...(k === 'model' && { scored: model.source === 'rubric' }),
    })),
    computed_at: typeof t.computed_at === 'string' ? t.computed_at : null,
  }
}

export const decodeTrustHistory = (row: Pick<DocRowLike, 'data'>) =>
  parse(row.data) as unknown as TrustScoreHistory

/** `getTrustScoreHistory` (api.ts:397-404): the latest `limit` snapshots, oldest first. */
export const trustHistoryOf = (rows: Array<TrustScoreHistory>, limit = 12) =>
  [...rows]
    .sort((a, b) => byDateDesc(a.calculated_at, b.calculated_at))
    .slice(0, limit)
    .reverse()

// ---------------------------------------------------------------------------
// Public embed config (RW3; flows.rs:242, 443)
// ---------------------------------------------------------------------------

/** `EmbedConfigPatch` (flows.rs:443): a field left out is kept; the theme goes whole. */
export function embedConfigPatch(c: Partial<PublicEmbedConfig>): Json {
  return pick(
    c,
    [
      'display_mode',
      'show_amounts',
      'amount_format',
      'show_node_names',
      'show_trust_score',
      'show_transaction_count',
      'show_last_updated',
      'visible_node_types',
      'hidden_node_ids',
      'theme',
      'allow_embed',
      'allowed_domains',
    ],
    [],
  )
}
