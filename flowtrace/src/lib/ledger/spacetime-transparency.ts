// Campaigns, donations, donor updates and the organisation profile on the SpacetimeDB module
// (MOD1, rust/docs/adr/0004-transparency-tables.md; MOD2, ADR 0005). Each export keeps the
// signature of the Supabase function it replaces (supabase.ts), so data.ts can select it.
// Loaded lazily with the SDK, like spacetime-backend.ts.
//
// Reads come from one subscription to the caller's `my_*` views, which the module already
// limits to the organisations the identity is a member of. Writes call the reducers; a reducer
// returns nothing, so a write resolves the record from the view once its row has arrived, and a
// refusal rejects with the module's own text (the SDK's SenderError message).
import {
  connectSpacetime,
  spacetimeMemberships,
  view,
} from './spacetime-backend'
import { getPublicCampaignBySlug } from './public-sql'
import {
  campaignCounts,
  campaignInput,
  campaignPatch,
  campaignReport,
  campaignStats,
  decodeCampaign,
  decodeDonation,
  decodeDonorUpdate,
  decodeFlowNode,
  decodeStat,
  decodeTransaction,
  donationInput,
  donationWithDetails,
  donorUpdateInput,
  donorUpdateWithDetails,
  organizationFrom,
  organizationPatch,
  resolveCreated,
  underfundedCampaigns,
  withTarget,
} from './transparency-model'
import type { DbConnection } from './spacetime-bindings'
import type { DocRowLike } from './transparency-model'
import type { CampaignReport, OrganizationProfileUpdate } from '../demo/api'
import type {
  Campaign,
  CampaignCounts,
  CampaignStats,
  CampaignWithDetails,
  Donation,
  DonationWithDetails,
  DonorUpdate,
  DonorUpdateWithDetails,
  Organization,
  PaymentStatus,
} from '../types'

const WRITE_TIMEOUT_MS = 15_000

/** Accessor names of the generated bindings (`my_campaign` is `myCampaign`). */
const DOC_VIEWS = [
  'campaign',
  'campaignStat',
  'donation',
  'donorUpdate',
  'flowNode',
  'transaction',
] as const
const ORG_VIEWS = [
  'organizationProfile',
  'trustScore',
  'ledgerOrg',
  'profile',
] as const
type DocView = (typeof DOC_VIEWS)[number]

const sqlName = (accessor: string) =>
  `my_${accessor.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)}`

interface Watch<T> {
  iter: () => Iterable<T>
  onInsert: (cb: (ctx: unknown, row: T) => void) => void
  removeOnInsert: (cb: (ctx: unknown, row: T) => void) => void
  onUpdate: (cb: (ctx: unknown, oldRow: T, newRow: T) => void) => void
  removeOnUpdate: (cb: (ctx: unknown, oldRow: T, newRow: T) => void) => void
  onDelete: (cb: (ctx: unknown, row: T) => void) => void
  removeOnDelete: (cb: (ctx: unknown, row: T) => void) => void
}

const subscribed = new WeakMap<DbConnection, Promise<void>>()

function ensureSubscribed(conn: DbConnection): Promise<void> {
  let sub = subscribed.get(conn)
  if (!sub) {
    sub = new Promise<void>((resolve, reject) => {
      conn
        .subscriptionBuilder()
        .onApplied(() => resolve())
        .onError((ctx) => {
          subscribed.delete(conn)
          reject(ctx.event ?? new Error('SpacetimeDB subscription failed.'))
        })
        .subscribe(
          [...DOC_VIEWS, ...ORG_VIEWS].map(
            (v) => `SELECT * FROM ${sqlName(v)}`,
          ),
        )
    })
    subscribed.set(conn, sub)
  }
  return sub
}

async function session(): Promise<DbConnection> {
  const conn = await connectSpacetime()
  await ensureSubscribed(conn)
  return conn
}

const watch = <T>(conn: DbConnection, accessor: string) =>
  view<T>(conn, accessor) as unknown as Watch<T>
const docs = (conn: DbConnection, v: DocView) => [
  ...watch<DocRowLike>(conn, v).iter(),
]
const ofOrg = (rows: Array<DocRowLike>, orgId: string) =>
  rows.filter((r) => r.orgId === orgId)

function campaigns(conn: DbConnection, orgId?: string): Array<Campaign> {
  const rows = docs(conn, 'campaign')
  return (orgId ? ofOrg(rows, orgId) : rows).map(decodeCampaign)
}
const nodes = (conn: DbConnection, orgId: string) =>
  ofOrg(docs(conn, 'flowNode'), orgId).map(decodeFlowNode)
const donations = (conn: DbConnection) =>
  docs(conn, 'donation').map(decodeDonation)
const donorUpdates = (conn: DbConnection) =>
  docs(conn, 'donorUpdate').map(decodeDonorUpdate)
const byCreatedDesc = <T extends { created_at: string }>(a: T, b: T) =>
  a.created_at < b.created_at ? 1 : a.created_at > b.created_at ? -1 : 0

function campaignOrg(conn: DbConnection, campaignId: string): string {
  const row = docs(conn, 'campaign').find((r) => r.id === campaignId)
  if (!row)
    throw new Error(
      'Campaign not found, or you are not a member of its organisation.',
    )
  return row.orgId
}

/**
 * Calls a reducer, then waits for `found` to see its effect in the client cache. The SDK applies
 * the reducer's own rows with its result, so `found` usually holds at once; otherwise this waits
 * for the next change of `accessor`.
 */
async function write<T>(
  conn: DbConnection,
  accessor: string,
  call: () => Promise<void>,
  found: () => T | undefined,
): Promise<T> {
  await call()
  const now = found()
  if (now !== undefined) return now
  const v = watch<unknown>(conn, accessor)
  return new Promise<T>((resolve, reject) => {
    const check = () => {
      const got = found()
      if (got === undefined) return
      stop()
      resolve(got)
    }
    const onRow = () => check()
    const onUpdate = () => check()
    const timer = setTimeout(() => {
      stop()
      reject(new Error('The module did not confirm the change in time.'))
    }, WRITE_TIMEOUT_MS)
    const stop = () => {
      clearTimeout(timer)
      v.removeOnInsert(onRow)
      v.removeOnUpdate(onUpdate)
    }
    v.onInsert(onRow)
    v.onUpdate(onUpdate)
  })
}

const updatedAt = (rec: { updated_at?: string } | undefined) => rec?.updated_at

// ---------------------------------------------------------------------------
// Campaigns
// ---------------------------------------------------------------------------

export async function getCampaigns(
  organizationId: string,
): Promise<Array<CampaignWithDetails>> {
  const conn = await session()
  const ns = nodes(conn, organizationId)
  return campaigns(conn, organizationId)
    .sort(byCreatedDesc)
    .map((c) => withTarget(c, ns))
}

export async function getCampaign(
  id: string,
  organizationId: string,
): Promise<CampaignWithDetails> {
  const conn = await session()
  const c = campaigns(conn, organizationId).find((x) => x.id === id)
  if (!c) throw new Error('Campaign not found.')
  return withTarget(c, nodes(conn, organizationId))
}

export const getCampaignBySlug = getPublicCampaignBySlug

export async function createCampaign(
  campaign: Omit<
    Campaign,
    'id' | 'current_amount' | 'created_at' | 'updated_at'
  >,
): Promise<Campaign> {
  const conn = await session()
  const orgId = campaign.organization_id
  const before = new Set(docs(conn, 'campaign').map((r) => r.key))
  const input = campaignInput(campaign)
  return write(
    conn,
    'campaign',
    () =>
      conn.reducers.createCampaign({
        orgId,
        campaignJson: JSON.stringify(input),
      }),
    () =>
      resolveCreated(
        docs(conn, 'campaign'),
        before,
        orgId,
        (c: Campaign) => c.name === campaign.name,
        decodeCampaign,
      ),
  )
}

export async function updateCampaign(
  id: string,
  organizationId: string,
  updates: Partial<
    Omit<Campaign, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
): Promise<Campaign> {
  const conn = await session()
  const find = () => campaigns(conn, organizationId).find((c) => c.id === id)
  const was = updatedAt(find())
  return write(
    conn,
    'campaign',
    () =>
      conn.reducers.updateCampaign({
        orgId: organizationId,
        campaignId: id,
        patchJson: JSON.stringify(campaignPatch(updates)),
      }),
    () => {
      const c = find()
      return c && c.updated_at !== was ? c : undefined
    },
  )
}

/** Drafts only; any other campaign is left as it is, without an error (api.ts:1116-1126). */
export async function deleteCampaign(
  id: string,
  organizationId: string,
): Promise<void> {
  const conn = await session()
  await conn.reducers.deleteCampaign({ orgId: organizationId, campaignId: id })
}

export async function getCampaignStats(
  campaignId: string,
): Promise<CampaignStats> {
  const conn = await session()
  const c = campaigns(conn).find((x) => x.id === campaignId)
  if (!c) throw new Error('Campaign not found.')
  const stat = docs(conn, 'campaignStat')
    .map(decodeStat)
    .find((s) => s.campaign_id === campaignId)
  return campaignStats(c, stat, Date.now())
}

export async function getCampaignCounts(
  organizationId: string,
): Promise<CampaignCounts> {
  const conn = await session()
  return campaignCounts(campaigns(conn, organizationId), donations(conn))
}

export async function getUnderfundedCampaigns(
  organizationId: string,
  limit = 5,
): Promise<Array<Campaign>> {
  const conn = await session()
  return underfundedCampaigns(campaigns(conn, organizationId), limit)
}

/** The campaign report for a member (campaigns/$id.tsx); visitors read public-sql.ts instead. */
export async function getCampaignReport(
  campaignId: string,
): Promise<CampaignReport> {
  const conn = await session()
  const orgId = campaignOrg(conn, campaignId)
  const ns = nodes(conn, orgId)
  const c = campaigns(conn, orgId).find((x) => x.id === campaignId)
  if (!c) throw new Error('Campaign not found.')
  const org = organizationOf(conn, orgId)
  return campaignReport(
    withTarget(c, ns),
    {
      id: orgId,
      name: org.name,
      trust_score: org.trust_score,
      verified: org.verified,
    },
    ns,
    ofOrg(docs(conn, 'transaction'), orgId).map(decodeTransaction),
  )
}

// ---------------------------------------------------------------------------
// Donations and donor updates
// ---------------------------------------------------------------------------

function donationDetails(conn: DbConnection, d: Donation): DonationWithDetails {
  const orgId =
    docs(conn, 'campaign').find((r) => r.id === d.campaign_id)?.orgId ?? ''
  return donationWithDetails(d, campaigns(conn), nodes(conn, orgId))
}

export async function getDonations(
  campaignId: string,
): Promise<Array<DonationWithDetails>> {
  const conn = await session()
  return donations(conn)
    .filter((d) => d.campaign_id === campaignId)
    .sort(byCreatedDesc)
    .map((d) => donationDetails(conn, d))
}

export async function getDonation(id: string): Promise<DonationWithDetails> {
  const conn = await session()
  const d = donations(conn).find((x) => x.id === id)
  if (!d) throw new Error('Donation not found.')
  return donationDetails(conn, d)
}

export async function getDonorDonations(
  donorEmail: string,
): Promise<Array<DonationWithDetails>> {
  const conn = await session()
  return donations(conn)
    .filter((d) => d.donor_email === donorEmail)
    .sort(byCreatedDesc)
    .map((d) => donationDetails(conn, d))
}

/**
 * Records a gift. A member's gift is resolved from `my_donation`. A visitor who is not a member
 * may give to an open public campaign (record_donation, ADR 0004); the module keeps it pending
 * and the visitor cannot read it back, so the record returned is the one sent, with no id.
 */
export async function createDonation(
  donation: Omit<
    Donation,
    'id' | 'impact_percentage' | 'created_at' | 'updated_at'
  >,
): Promise<Donation> {
  const conn = await session()
  const member = docs(conn, 'campaign').find(
    (r) => r.id === donation.campaign_id,
  )
  const orgId = member?.orgId ?? (await publicCampaignOrg(donation.campaign_id))
  const input = donationInput(donation)
  const call = () =>
    conn.reducers.recordDonation({ orgId, donationJson: JSON.stringify(input) })
  if (!member) {
    await call()
    const at = new Date().toISOString()
    return {
      ...donation,
      id: '',
      impact_percentage: null,
      created_at: at,
      updated_at: at,
    }
  }
  const before = new Set(docs(conn, 'donation').map((r) => r.key))
  return write(conn, 'donation', call, () =>
    resolveCreated(
      docs(conn, 'donation'),
      before,
      orgId,
      (d: Donation) =>
        d.campaign_id === donation.campaign_id && d.amount === donation.amount,
      decodeDonation,
    ),
  )
}

async function publicCampaignOrg(campaignId: string): Promise<string> {
  const { publicSql, sqlText } = await import('./public-sql')
  const rows = await publicSql(
    `SELECT * FROM public_campaign WHERE campaign_id = ${sqlText(campaignId)}`,
  )
  const orgId = rows.at(0)?.org_id
  if (typeof orgId !== 'string')
    throw new Error('This campaign is not open for donations.')
  return orgId
}

export async function updateDonationStatus(
  id: string,
  status: PaymentStatus,
  paymentReference?: string,
): Promise<Donation> {
  const conn = await session()
  const row = docs(conn, 'donation').find((r) => r.id === id)
  if (!row) throw new Error('Donation not found.')
  const find = () => donations(conn).find((d) => d.id === id)
  const was = updatedAt(find())
  return write(
    conn,
    'donation',
    () =>
      conn.reducers.updateDonationStatus({
        orgId: row.orgId,
        donationId: id,
        status,
        paymentReference: paymentReference || undefined,
      }),
    () => {
      const d = find()
      return d && d.updated_at !== was ? d : undefined
    },
  )
}

function updateDetails(
  conn: DbConnection,
  u: DonorUpdate,
): DonorUpdateWithDetails {
  const orgId =
    docs(conn, 'donorUpdate').find((r) => r.id === u.id)?.orgId ?? ''
  return donorUpdateWithDetails(
    u,
    donations(conn),
    campaigns(conn),
    nodes(conn, orgId),
  )
}

export async function getDonorUpdates(
  donationId: string,
): Promise<Array<DonorUpdateWithDetails>> {
  const conn = await session()
  return donorUpdates(conn)
    .filter((u) => u.donation_id === donationId)
    .sort(byCreatedDesc)
    .map((u) => updateDetails(conn, u))
}

export async function getCampaignUpdates(
  campaignId: string,
): Promise<Array<DonorUpdateWithDetails>> {
  const conn = await session()
  return donorUpdates(conn)
    .filter((u) => u.campaign_id === campaignId)
    .sort(byCreatedDesc)
    .map((u) => updateDetails(conn, u))
}

export async function createDonorUpdate(
  update: Omit<DonorUpdate, 'id' | 'sent_at' | 'created_at'>,
): Promise<DonorUpdate> {
  const conn = await session()
  const row = docs(conn, 'donation').find((r) => r.id === update.donation_id)
  if (!row) throw new Error('Donation not found.')
  const before = new Set(docs(conn, 'donorUpdate').map((r) => r.key))
  return write(
    conn,
    'donorUpdate',
    () =>
      conn.reducers.postDonorUpdate({
        orgId: row.orgId,
        updateJson: JSON.stringify(donorUpdateInput(update)),
      }),
    () =>
      resolveCreated(
        docs(conn, 'donorUpdate'),
        before,
        row.orgId,
        (u: DonorUpdate) =>
          u.donation_id === update.donation_id && u.title === update.title,
        decodeDonorUpdate,
      ),
  )
}

export async function markUpdateSent(id: string): Promise<DonorUpdate> {
  const conn = await session()
  const row = docs(conn, 'donorUpdate').find((r) => r.id === id)
  if (!row) throw new Error('Update not found.')
  const find = () => donorUpdates(conn).find((u) => u.id === id)
  return write(
    conn,
    'donorUpdate',
    () => conn.reducers.markDonorUpdateSent({ orgId: row.orgId, updateId: id }),
    () => {
      const u = find()
      return u?.sent_at ? u : undefined
    },
  )
}

// ---------------------------------------------------------------------------
// Organisation and person
// ---------------------------------------------------------------------------

interface OrgDocRow {
  orgId: string
  data: string
}
interface LedgerOrgRow {
  orgId: string
  settingsJson: string
}
interface ProfileRow {
  identity: { toHexString: () => string }
  data: string
}

function organizationOf(conn: DbConnection, orgId: string): Organization {
  const one = (accessor: string) =>
    [...watch<OrgDocRow>(conn, accessor).iter()].find((r) => r.orgId === orgId)
  const books = [...watch<LedgerOrgRow>(conn, 'ledgerOrg').iter()].find(
    (r) => r.orgId === orgId,
  )
  let base = ''
  try {
    if (books)
      base =
        (JSON.parse(books.settingsJson) as { base_currency?: string })
          .base_currency ?? ''
  } catch {
    // Unreadable settings: the currency stays empty until the profile names one.
  }
  return organizationFrom(
    orgId,
    one('organizationProfile')?.data,
    base,
    one('trustScore')?.data,
  )
}

export async function getOrganization(id: string): Promise<Organization> {
  return organizationOf(await session(), id)
}

/** Settings' profile form (update_organization_profile); the module's refusal text on failure. */
export async function updateOrganization(
  id: string,
  updates: OrganizationProfileUpdate,
): Promise<Organization> {
  const conn = await session()
  const was = organizationOf(conn, id).updated_at
  return write(
    conn,
    'organizationProfile',
    () =>
      conn.reducers.updateOrganizationProfile({
        orgId: id,
        patchJson: JSON.stringify(organizationPatch(updates)),
      }),
    () => {
      const o = organizationOf(conn, id)
      return o.updated_at && o.updated_at !== was ? o : undefined
    },
  )
}

/** This identity's own profile (`my_profile`), when it has set one. */
export interface ModulePerson {
  identity: string
  user_id: string
  display_name: string | null
}

/**
 * What org-context needs in one read: this identity, its memberships, each of their
 * organisations, and its own profile.
 */
export async function orgSnapshot() {
  const conn = await session()
  const { identity, memberships } = await spacetimeMemberships()
  const organizations = new Map(
    memberships.map((m) => [m.orgId, organizationOf(conn, m.orgId)]),
  )
  const mine = [...watch<ProfileRow>(conn, 'profile').iter()].find(
    (r) => r.identity.toHexString() === identity,
  )
  let person: ModulePerson | null = null
  if (mine) {
    try {
      person = JSON.parse(mine.data) as ModulePerson
    } catch {
      person = null
    }
  }
  return { identity, memberships, organizations, person }
}

/** `set_profile`: the caller's display name and email (the module keeps only its hash). */
export async function setProfile(
  displayName: string,
  email: string,
): Promise<void> {
  const conn = await session()
  await conn.reducers.setProfile({ displayName, email })
}

/** Calls back once per transaction that changes any record this file reads. */
export function onTransparencyChange(onChange: () => void): () => void {
  let stopped = false
  let pending = false
  let detach = () => {}
  const emit = () => {
    if (pending || stopped) return
    pending = true
    setTimeout(() => {
      pending = false
      if (!stopped) onChange()
    }, 0)
  }
  const onRow = () => emit()
  const onUpdate = () => emit()
  void session()
    .then((conn) => {
      if (stopped) return
      // `member` is spacetime-backend.ts's subscription; a new membership changes the org context.
      const views = [...DOC_VIEWS, ...ORG_VIEWS, 'member'].map((v) =>
        watch<unknown>(conn, v),
      )
      for (const v of views) {
        v.onInsert(onRow)
        v.onUpdate(onUpdate)
        v.onDelete(onRow)
      }
      detach = () => {
        for (const v of views) {
          v.removeOnInsert(onRow)
          v.removeOnUpdate(onUpdate)
          v.removeOnDelete(onRow)
        }
      }
    })
    .catch(() => {
      // Connection errors surface through the reads.
    })
  return () => {
    stopped = true
    detach()
  }
}
