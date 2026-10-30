// What a visitor with no token reads: the module's public tables (pre-decision P20;
// rust/crates/ledger-module/src/transparency/public.rs) through the SpacetimeDB HTTP SQL
// endpoint, `POST {uri}/v1/database/{name}/sql` with the query as the body and no
// Authorization header (https://spacetimedb.com/docs/http/database). Public tables answer; a
// private one is refused with 400 and its reason as text. No WebSocket and no identity, so the
// donate page, the campaign embed and the trust badge stay anonymous.
import { campaignStats } from './transparency-model'
import { toMajor } from './money'
import type { StoredStat } from './transparency-model'
import type { CampaignReport } from '../demo/api'
import type {
  CampaignStats,
  CampaignWithDetails,
  FlowNode,
  Organization,
  TransactionWithNodes,
} from '../types'

type Row = Record<string, unknown>

interface StatementResult {
  schema: { elements: Array<{ name: { some?: string } | null }> }
  rows: Array<Array<unknown>>
}

/** The database's HTTP origin from the WebSocket URI the app connects with. */
export function httpOrigin(uri: string): string {
  return uri.replace(/^ws(s?):\/\//, 'http$1://').replace(/\/+$/, '')
}

function config() {
  const uri = import.meta.env.VITE_SPACETIMEDB_URI as string | undefined
  const name = import.meta.env.VITE_SPACETIMEDB_MODULE as string | undefined
  if (!uri || !name)
    throw new Error(
      'Set VITE_SPACETIMEDB_URI and VITE_SPACETIMEDB_MODULE to read public pages from SpacetimeDB.',
    )
  return { origin: httpOrigin(uri), name }
}

/** A SQL string literal. */
export const sqlText = (s: string) => `'${s.replace(/'/g, "''")}'`

/** Runs one query anonymously and returns its rows by column name. */
export async function publicSql(query: string): Promise<Array<Row>> {
  const { origin, name } = config()
  const res = await fetch(
    `${origin}/v1/database/${encodeURIComponent(name)}/sql`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: query,
    },
  )
  if (!res.ok) {
    const reason = (await res.text()).trim()
    throw new Error(reason || `SpacetimeDB answered ${res.status}.`)
  }
  const results = (await res.json()) as Array<StatementResult>
  const first = results.at(0)
  if (!first) return []
  const names = first.schema.elements.map((e, i) => e.name?.some ?? String(i))
  return first.rows.map((values) =>
    Object.fromEntries(names.map((n, i) => [n, values[i]])),
  )
}

// ---------------------------------------------------------------------------
// public_campaign (public.rs:147, PublicCampaign), amounts in minor units
// ---------------------------------------------------------------------------

interface NodeRef {
  id: string
  name: string
  type: FlowNode['type']
}

interface PublicCampaignData {
  campaign: Row & {
    id: string
    organization_id: string
    target_amount: number
    current_amount: number
  }
  stats: (StoredStat & Row) | null
  organization: CampaignReport['organization']
  evidenced_amount: number | null
  earmarks: Array<CampaignReport['earmarks'][number]>
  spending: Array<
    Row & {
      id: string
      amount: number
      from_node: NodeRef | null
      to_node: NodeRef | null
    }
  >
}

async function publicCampaign(
  where: string,
): Promise<PublicCampaignData | null> {
  const rows = await publicSql(`SELECT * FROM public_campaign WHERE ${where}`)
  const data = rows.at(0)?.data
  return typeof data === 'string'
    ? (JSON.parse(data) as PublicCampaignData)
    : null
}

/** A published campaign as the app's campaign record; fields the projection leaves out are empty. */
function toCampaign(p: PublicCampaignData): CampaignWithDetails {
  const c = p.campaign
  const defaults = {
    overflow_campaign_id: null,
    metadata: {},
    created_by: null,
    created_at: c.starts_at,
    updated_at: c.starts_at,
    is_public: true,
  }
  return {
    ...defaults,
    ...(c as unknown as CampaignWithDetails),
    target_amount: toMajor(c.target_amount),
    current_amount: toMajor(c.current_amount),
    target_node_id: (c.target_node as NodeRef | null)?.id ?? null,
  }
}

function toStats(p: PublicCampaignData, nowMs: number): CampaignStats {
  const s = p.stats
  const stat: StoredStat | undefined = s
    ? {
        ...s,
        total_raised: toMajor(s.total_raised),
        avg_donation: toMajor(s.avg_donation),
        largest_donation: toMajor(s.largest_donation),
        smallest_donation: toMajor(s.smallest_donation),
      }
    : undefined
  return campaignStats(toCampaign(p), stat, nowMs)
}

function toReport(p: PublicCampaignData): CampaignReport {
  return {
    campaign: toCampaign(p),
    organization: p.organization,
    evidenced_amount:
      p.evidenced_amount === null ? null : toMajor(p.evidenced_amount),
    earmarks: p.earmarks.map((e) => ({
      ...e,
      received: toMajor(e.received),
      evidenced: toMajor(e.evidenced),
    })),
    spending: p.spending.map(
      (s) =>
        ({
          ...s,
          organization_id: p.campaign.organization_id,
          from_node_id: s.from_node?.id ?? null,
          to_node_id: s.to_node?.id ?? null,
          amount: toMajor(s.amount),
        }) as unknown as TransactionWithNodes,
    ),
  }
}

const NOT_OPEN = 'This campaign is not open for donations.'

/** `getCampaignBySlug`: an active public campaign by its link, with its figures. */
export async function getPublicCampaignBySlug(
  slug: string,
): Promise<CampaignWithDetails> {
  const p = await publicCampaign(`slug = ${sqlText(slug)}`)
  if (!p) throw new Error(NOT_OPEN)
  return { ...toCampaign(p), stats: toStats(p, Date.now()) }
}

/** `getCampaignStats` for a published campaign. */
export async function getPublicCampaignStats(
  campaignId: string,
): Promise<CampaignStats> {
  const p = await publicCampaign(`campaign_id = ${sqlText(campaignId)}`)
  if (!p) throw new Error(NOT_OPEN)
  return toStats(p, Date.now())
}

/** `getCampaignReport` for a published campaign: the donate page and the campaign embed. */
export async function getPublicCampaignReport(
  campaignId: string,
): Promise<CampaignReport> {
  const p = await publicCampaign(`campaign_id = ${sqlText(campaignId)}`)
  if (!p) throw new Error(NOT_OPEN)
  return toReport(p)
}

// ---------------------------------------------------------------------------
// public_organization and public_badge (public.rs:34, 64)
// ---------------------------------------------------------------------------

async function dataOf(table: string, orgId: string): Promise<Row | null> {
  const rows = await publicSql(
    `SELECT * FROM ${table} WHERE org_id = ${sqlText(orgId)}`,
  )
  const data = rows.at(0)?.data
  return typeof data === 'string' ? (JSON.parse(data) as Row) : null
}

/** A published organisation; it publishes once its profile is public (public.rs:5-8). */
export async function getPublicOrganization(
  orgId: string,
): Promise<Organization> {
  const o = await dataOf('public_organization', orgId)
  if (!o) throw new Error('This organisation does not publish its profile.')
  return {
    id: orgId,
    name: o.name as string,
    type: (o.type as Organization['type']) ?? null,
    country: o.country as string,
    currency: o.currency as string,
    trust_score: (o.trust_score as number | null) ?? 0,
    verified: o.verified === true,
    public_slug: (o.public_slug as string | null) ?? null,
    logo_url: null,
    created_at: o.updated_at as string,
    updated_at: o.updated_at as string,
  }
}

/** What the trust badge draws, in `useTrustScore`'s shape; zero when nothing is published. */
export async function getPublicBadge(orgId: string) {
  const b = await dataOf('public_badge', orgId)
  return {
    overall_score: (b?.score as number | undefined) ?? 0,
    breakdown: (b?.breakdown as Record<string, number> | undefined) ?? {},
    verification_rate: null,
    calculated_at: (b?.calculated_at as string | undefined) ?? null,
  }
}
