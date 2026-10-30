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
import { ledgerOrgId } from '../spacetime-session'
import { fromMajor, toMajor } from './money'
import type { SpacetimeMembership } from './spacetime-backend'
import type {
  Campaign,
  CampaignCounts,
  CampaignStats,
  CampaignWithDetails,
  Donation,
  DonationWithDetails,
  DonorUpdate,
  DonorUpdateWithDetails,
  FlowNode,
  Organization,
  Profile,
  Transaction,
  TransactionWithNodes,
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
