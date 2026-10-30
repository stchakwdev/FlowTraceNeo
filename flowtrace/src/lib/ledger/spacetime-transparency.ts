// Campaigns, donations, donor updates and the organisation profile (RW1), transactions, flow
// nodes, node visibility, the flow data and organisation chains (RW2), and proofs, access groups,
// the trust score and the public embed config (RW3) on the SpacetimeDB module
// (MOD1, rust/docs/adr/0004-transparency-tables.md; MOD2, ADR 0005). Each export keeps the
// signature of the demo function it stands beside (demo/api.ts), so data.ts can select it.
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
  accessGroupInput,
  accessGroupPatch,
  accessGroupsInOrder,
  campaignCounts,
  campaignInput,
  campaignPatch,
  campaignReport,
  campaignStats,
  chainCounts,
  chainTrustScore,
  chainsOf,
  decodeAccess,
  decodeAccessGroup,
  decodeCampaign,
  decodeChain,
  decodeChainActivity,
  decodeDonation,
  decodeDonorUpdate,
  decodeFlowNode,
  decodeGroupMember,
  decodeNodeVisibility,
  decodeStat,
  decodeTransaction,
  decodeTrustHistory,
  decodeVerification,
  donationInput,
  donationWithDetails,
  donorUpdateInput,
  donorUpdateWithDetails,
  embedConfigPatch,
  flowData,
  flowNodeInput,
  groupMemberInput,
  groupMembersWithProfiles,
  nodeVisibilitySettings,
  nodeVisibilityWithGroups,
  orgStats,
  organizationFrom,
  organizationPatch,
  proofsOf,
  resolveCreated,
  transactionInput,
  transactionPatch,
  transactionsWithNodes,
  trustHistoryOf,
  trustSummaryFrom,
  underfundedCampaigns,
  verificationInput,
  withNodes,
  withTarget,
} from './transparency-model'
import type { DbConnection } from './spacetime-bindings'
import type {
  DocRowLike,
  GroupMemberWithProfile,
  NodeVisibilityWithGroup,
  ProofLike,
} from './transparency-model'
import type { CampaignReport, OrganizationProfileUpdate } from '../demo/api'
import type {
  AccessGroup,
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
  FlowDataResult,
  FlowNode,
  GroupMember,
  NodeVisibility,
  OrgMember,
  OrgStats,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
  PaymentStatus,
  PublicEmbedConfig,
  Transaction,
  TransactionWithNodes,
  TrustScoreHistory,
  TrustScoreSummary,
  Verification,
  VerificationAccessLog,
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
  'nodeVisibility',
  'verification',
  'verificationAccessLog',
  'accessGroup',
  'accessGroupMember',
  'trustScoreHistory',
] as const
const ORG_VIEWS = [
  'organizationProfile',
  'trustScore',
  'publicEmbedConfig',
  'ledgerOrg',
  'profile',
] as const
/** The two chain views: rows keyed by chain, not by organisation (ADR 0005, Views). */
const CHAIN_VIEWS = ['organizationChain', 'chainActivityLog'] as const
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
          [...DOC_VIEWS, ...ORG_VIEWS, ...CHAIN_VIEWS].map(
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

// ---------------------------------------------------------------------------
// Transactions and flow nodes (RW2)
// ---------------------------------------------------------------------------

const transactions = (conn: DbConnection, orgId: string) =>
  ofOrg(docs(conn, 'transaction'), orgId)
    .sort((a, b) => (a.seq < b.seq ? -1 : a.seq > b.seq ? 1 : 0))
    .map(decodeTransaction)
const proofs = (conn: DbConnection) =>
  docs(conn, 'verification').map((r) => JSON.parse(r.data) as ProofLike)
const accessGroups = (conn: DbConnection, orgId: string) =>
  ofOrg(docs(conn, 'accessGroup'), orgId).map(decodeAccessGroup)

export async function getTransactions(
  organizationId: string,
): Promise<Array<TransactionWithNodes>> {
  const conn = await session()
  return transactionsWithNodes(
    transactions(conn, organizationId),
    nodes(conn, organizationId),
  )
}

export async function getTransaction(
  id: string,
  organizationId: string,
): Promise<TransactionWithNodes> {
  const conn = await session()
  const t = transactions(conn, organizationId).find((x) => x.id === id)
  if (!t) throw new Error('Transaction not found.')
  return withNodes(t, nodes(conn, organizationId))
}

/**
 * Records a transaction (create_transaction). Unlike the demo, recording does not post it to the
 * books: posting stays a ledger command and `link_transaction_entries` records the link
 * (ADR 0005, difference 1).
 */
export async function createTransaction(
  transaction: Omit<
    Transaction,
    'id' | 'created_at' | 'updated_at' | 'verification_count'
  >,
): Promise<Transaction> {
  const conn = await session()
  const orgId = transaction.organization_id
  const before = new Set(docs(conn, 'transaction').map((r) => r.key))
  return write(
    conn,
    'transaction',
    () =>
      conn.reducers.createTransaction({
        orgId,
        transactionJson: JSON.stringify(transactionInput(transaction)),
      }),
    () =>
      resolveCreated(
        docs(conn, 'transaction'),
        before,
        orgId,
        (t: Transaction) =>
          t.description === transaction.description &&
          t.amount === transaction.amount,
        decodeTransaction,
      ),
  )
}

export async function updateTransaction(
  id: string,
  organizationId: string,
  updates: Partial<
    Omit<Transaction, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
): Promise<Transaction> {
  const conn = await session()
  const find = () => transactions(conn, organizationId).find((t) => t.id === id)
  const was = updatedAt(find())
  return write(
    conn,
    'transaction',
    () =>
      conn.reducers.updateTransaction({
        orgId: organizationId,
        transactionId: id,
        patchJson: JSON.stringify(transactionPatch(updates)),
      }),
    () => {
      const t = find()
      return t && t.updated_at !== was ? t : undefined
    },
  )
}

/** `getNodes` (api.ts:256-262): by name. */
export async function getNodes(
  organizationId: string,
): Promise<Array<FlowNode>> {
  const conn = await session()
  return nodes(conn, organizationId).sort((a, b) =>
    a.name.localeCompare(b.name),
  )
}

export async function createNode(
  node: Omit<FlowNode, 'id' | 'created_at' | 'balance'>,
): Promise<FlowNode> {
  const conn = await session()
  const orgId = node.organization_id
  const before = new Set(docs(conn, 'flowNode').map((r) => r.key))
  return write(
    conn,
    'flowNode',
    () =>
      conn.reducers.createFlowNode({
        orgId,
        nodeJson: JSON.stringify(flowNodeInput(node)),
      }),
    () =>
      resolveCreated(
        docs(conn, 'flowNode'),
        before,
        orgId,
        (n: FlowNode) => n.name === node.name,
        decodeFlowNode,
      ),
  )
}

export async function getOrganizationStats(
  organizationId: string,
): Promise<OrgStats> {
  const conn = await session()
  return orgStats(transactions(conn, organizationId), proofs(conn))
}

/**
 * The flow map data. The web read it from the `generate-flow-data` edge function, which the
 * module has no counterpart for, so it is computed from `my_flow_node`, `my_transaction` and
 * `my_verification` as the demo computes it (api.ts:475-566, flows.rs `flow_data`).
 */
export async function getFlowDataAPI(
  organizationId: string,
  startDate?: string,
  endDate?: string,
): Promise<FlowDataResult> {
  const conn = await session()
  return flowData({
    nodes: nodes(conn, organizationId),
    transactions: transactions(conn, organizationId),
    proofs: proofs(conn),
    currency: organizationOf(conn, organizationId).currency,
    startDate,
    endDate,
  })
}

// ---------------------------------------------------------------------------
// Node visibility (RW2; admins and managers write, ADR 0005)
// ---------------------------------------------------------------------------

function nodeOrg(conn: DbConnection, nodeId: string): string {
  const row = docs(conn, 'flowNode').find((r) => r.id === nodeId)
  if (!row)
    throw new Error(
      'Flow node not found, or you are not a member of its organisation.',
    )
  return row.orgId
}

const visibilityOf = (conn: DbConnection, nodeId: string) =>
  docs(conn, 'nodeVisibility')
    .map(decodeNodeVisibility)
    .filter((v) => v.node_id === nodeId)

export async function getNodeVisibility(
  nodeId: string,
): Promise<Array<NodeVisibilityWithGroup>> {
  const conn = await session()
  const orgId = docs(conn, 'flowNode').find((r) => r.id === nodeId)?.orgId
  return nodeVisibilityWithGroups(
    visibilityOf(conn, nodeId),
    orgId ? accessGroups(conn, orgId) : [],
  )
}

export async function setNodeVisibility(
  nodeId: string,
  groupId: string,
  settings: Partial<
    Omit<NodeVisibility, 'id' | 'node_id' | 'group_id' | 'created_at'>
  >,
): Promise<NodeVisibility> {
  const conn = await session()
  const orgId = nodeOrg(conn, nodeId)
  const find = () =>
    visibilityOf(conn, nodeId).find((v) => v.group_id === groupId)
  const was = JSON.stringify(find() ?? null)
  const sent = nodeVisibilitySettings(settings)
  return write(
    conn,
    'nodeVisibility',
    () =>
      conn.reducers.setNodeVisibility({
        orgId,
        nodeId,
        groupId,
        settingsJson: JSON.stringify(sent),
      }),
    () => {
      const v = find()
      if (!v) return undefined
      // A row that already held these settings does not change; it is the result as it is.
      const same = Object.entries(sent).every(
        ([k, x]) => (v as unknown as Record<string, unknown>)[k] === x,
      )
      return JSON.stringify(v) !== was || same ? v : undefined
    },
  )
}

export async function removeNodeVisibility(
  nodeId: string,
  groupId: string,
): Promise<void> {
  const conn = await session()
  await conn.reducers.removeNodeVisibility({
    orgId: nodeOrg(conn, nodeId),
    nodeId,
    groupId,
  })
}

/** The organisation's access groups by name, for the chain approval's group choice. */
export async function getChainAccessGroups(
  organizationId: string,
): Promise<Array<AccessGroup>> {
  const conn = await session()
  return accessGroups(conn, organizationId).sort((a, b) =>
    a.name.localeCompare(b.name),
  )
}

// ---------------------------------------------------------------------------
// Organisation chains (RW2; chains.rs, ADR 0005)
// ---------------------------------------------------------------------------

interface ChainRow {
  id: string
  sourceOrgId: string
  targetOrgId: string
  seq: bigint
  data: string
}
interface ChainActivityRow {
  id: string
  chainId: string
  seq: bigint
  data: string
}

const chains = (conn: DbConnection) =>
  [...watch<ChainRow>(conn, 'organizationChain').iter()].map(decodeChain)
const chainById = (conn: DbConnection, id: string) =>
  chains(conn).find((c) => c.id === id)

function chainOrFail(conn: DbConnection, id: string) {
  const c = chainById(conn, id)
  if (!c)
    throw new Error(
      'Chain not found, or neither organisation is one you are a member of.',
    )
  return c
}

/** Calls a chain reducer and resolves the chain once it shows `done`. */
function changeChain(
  conn: DbConnection,
  chainId: string,
  call: () => Promise<void>,
  done: (c: OrganizationChainWithDetails, was: string) => boolean,
): Promise<OrganizationChainWithDetails> {
  const was = chainOrFail(conn, chainId).updated_at
  return write(conn, 'organizationChain', call, () => {
    const c = chainById(conn, chainId)
    return c && done(c, was) ? c : undefined
  })
}

export async function getOutgoingChains(
  organizationId: string,
): Promise<Array<OrganizationChainWithDetails>> {
  return chainsOf(chains(await session()), organizationId, 'outgoing')
}

export async function getIncomingChains(
  organizationId: string,
): Promise<Array<OrganizationChainWithDetails>> {
  return chainsOf(chains(await session()), organizationId, 'incoming')
}

export async function getChain(
  chainId: string,
): Promise<OrganizationChainWithDetails> {
  return chainOrFail(await session(), chainId)
}

/**
 * Asks `targetOrgId` for a chain (request_chain). The requester is the caller, so `requestedBy`
 * is not sent; the module refuses a second pending or active chain between the two (ADR 0005,
 * difference 4).
 */
export async function requestChain(
  sourceOrgId: string,
  targetOrgId: string,
  _requestedBy: string,
  message?: string,
): Promise<OrganizationChain> {
  const conn = await session()
  const before = new Set(chains(conn).map((c) => c.id))
  return write(
    conn,
    'organizationChain',
    () =>
      conn.reducers.requestChain({
        sourceOrgId,
        targetOrgId,
        message: message || undefined,
      }),
    () =>
      chains(conn).find(
        (c) =>
          !before.has(c.id) &&
          c.source_org_id === sourceOrgId &&
          c.target_org_id === targetOrgId,
      ),
  )
}

/** Approves a pending chain; the module asks a manager of the target (ADR 0005, Reducers). */
export async function approveChain(
  chainId: string,
  _approvedBy: string,
  grantedGroupId: string,
  linkNodeId?: string,
): Promise<OrganizationChain> {
  const conn = await session()
  return changeChain(
    conn,
    chainId,
    () =>
      conn.reducers.approveChain({
        chainId,
        grantedGroupId,
        linkNodeId: linkNodeId || undefined,
      }),
    (c) => c.status === 'active',
  )
}

export async function rejectChain(
  chainId: string,
  _rejectedBy: string,
  reason?: string,
): Promise<OrganizationChain> {
  const conn = await session()
  return changeChain(
    conn,
    chainId,
    () => conn.reducers.rejectChain({ chainId, reason: reason || undefined }),
    (c) => c.status === 'rejected',
  )
}

export async function revokeChain(
  chainId: string,
  _revokedBy: string,
): Promise<OrganizationChain> {
  const conn = await session()
  return changeChain(
    conn,
    chainId,
    () => conn.reducers.revokeChain({ chainId }),
    (c) => c.status === 'revoked',
  )
}

/** Withdraws a pending request; the module drops it with its log (ADR 0005, difference 4). */
export async function cancelChainRequest(chainId: string): Promise<void> {
  const conn = await session()
  await conn.reducers.cancelChainRequest({ chainId })
}

export async function updateChainPermissions(
  chainId: string,
  grantedGroupId: string,
  linkNodeId?: string,
): Promise<OrganizationChain> {
  const conn = await session()
  return changeChain(
    conn,
    chainId,
    () =>
      conn.reducers.updateChainPermissions({
        chainId,
        grantedGroupId,
        linkNodeId: linkNodeId || undefined,
      }),
    (c, was) => c.updated_at !== was,
  )
}

/** `getChainActivityLog` (api.ts:992-998): newest first. */
export async function getChainActivityLog(
  chainId: string,
): Promise<Array<ChainActivityLog>> {
  const conn = await session()
  return [...watch<ChainActivityRow>(conn, 'chainActivityLog').iter()]
    .filter((r) => r.chainId === chainId)
    .map(decodeChainActivity)
    .sort((a, b) =>
      a.created_at < b.created_at ? 1 : a.created_at > b.created_at ? -1 : 0,
    )
}

export async function getChainCounts(
  organizationId: string,
): Promise<ChainCounts> {
  return chainCounts(chains(await session()), organizationId)
}

export async function getChainTrustScore(
  chainId: string,
): Promise<ChainTrustScore | null> {
  return chainTrustScore(chainById(await session(), chainId))
}

// ---------------------------------------------------------------------------
// Proofs (RW3; plan.rs:683-840, ADR 0004)
// ---------------------------------------------------------------------------

const verifications = (conn: DbConnection) =>
  docs(conn, 'verification').map(decodeVerification)

function proofOrg(conn: DbConnection, verificationId: string): string {
  const row = docs(conn, 'verification').find((r) => r.id === verificationId)
  if (!row)
    throw new Error('Proof not found, or your access groups do not show it to you.')
  return row.orgId
}

/** A transaction's proofs as far as the caller's access groups show them (`my_verification`). */
export async function getVerifications(
  transactionId: string,
): Promise<Array<Verification>> {
  return proofsOf(verifications(await session()), transactionId)
}

/**
 * Records a proof (create_verification) after its file went to the proofs Worker
 * (proofs.ts `storeProofFile`). The proof is the caller's newest on that transaction once its
 * row arrives; a caller in no access group never sees it, so then this resolves with what was
 * sent (ADR 0004 rule table: no group, nothing shown).
 */
export async function createVerification(
  verification: Omit<
    Verification,
    'id' | 'created_at' | 'verified_at' | 'trust_points'
  >,
): Promise<Verification> {
  const conn = await session()
  const orgId = docs(conn, 'transaction').find(
    (r) => r.id === verification.transaction_id,
  )?.orgId
  if (!orgId)
    throw new Error(
      'Transaction not found, or you are not a member of its organisation.',
    )
  const input = verificationInput(verification)
  const before = new Set(docs(conn, 'verification').map((r) => r.key))
  const txBefore = docs(conn, 'transaction').find(
    (r) => r.id === verification.transaction_id,
  )?.data
  return write(
    conn,
    'transaction',
    () =>
      conn.reducers.createVerification({
        orgId,
        verificationJson: JSON.stringify(input),
      }),
    () => {
      const made = resolveCreated(
        docs(conn, 'verification'),
        before,
        orgId,
        (v: Verification) =>
          v.transaction_id === verification.transaction_id &&
          v.type === verification.type,
        decodeVerification,
      )
      if (made) return made
      // The proof counts towards its transaction (flows.rs count_proof), visible or not.
      const tx = docs(conn, 'transaction').find(
        (r) => r.id === verification.transaction_id,
      )?.data
      return tx !== txBefore
        ? ({
            ...verification,
            id: '',
            created_at: new Date().toISOString(),
            verified_at: null,
            trust_points: 0,
          } as Verification)
        : undefined
    },
  )
}

/** Shows a proof's file at the proofs Worker's public path, or takes it back (mark_published). */
export async function publishVerification(
  verificationId: string,
  published: boolean,
): Promise<Verification> {
  const conn = await session()
  const orgId = proofOrg(conn, verificationId)
  return write(
    conn,
    'verification',
    () => conn.reducers.markPublished({ orgId, verificationId, published }),
    () => {
      const v = verifications(conn).find((x) => x.id === verificationId)
      return v && v.published === published ? v : undefined
    },
  )
}

/**
 * Records that the caller opened or downloaded a proof (log_verification_access); the module
 * sets the level from the caller's groups, never from the call.
 */
export async function logVerificationAccess(
  verificationId: string,
  accessType: 'view' | 'download',
): Promise<void> {
  const conn = await session()
  await conn.reducers.logVerificationAccess({
    orgId: proofOrg(conn, verificationId),
    verificationId,
    accessType,
  })
}

/** The caller's own looks at a proof, newest first (`my_verification_access_log`). */
export async function getVerificationAccessLog(
  verificationId: string,
): Promise<Array<VerificationAccessLog>> {
  const conn = await session()
  return docs(conn, 'verificationAccessLog')
    .map(decodeAccess)
    .filter((a) => a.verification_id === verificationId)
    .sort((a, b) =>
      a.accessed_at < b.accessed_at ? 1 : a.accessed_at > b.accessed_at ? -1 : 0,
    )
}

// ---------------------------------------------------------------------------
// Access groups and their members (RW3; plan.rs:340-600)
// ---------------------------------------------------------------------------

interface MemberRowLike {
  orgId: string
  userId: string
  name: string
}

const groupRow = (conn: DbConnection, id: string) =>
  docs(conn, 'accessGroup').find((r) => r.id === id)

function groupOrFail(conn: DbConnection, id: string) {
  const row = groupRow(conn, id)
  if (!row)
    throw new Error(
      'Access group not found, or you are not a member of its organisation.',
    )
  return { orgId: row.orgId, group: decodeAccessGroup(row) }
}

const members = (conn: DbConnection) =>
  docs(conn, 'accessGroupMember').map(decodeGroupMember)

/** The organisation's members (`my_member`), by name. */
function people(conn: DbConnection, orgId: string): Array<OrgMember> {
  return [...watch<MemberRowLike>(conn, 'member').iter()]
    .filter((m) => m.orgId === orgId)
    .map((m) => ({ user_id: m.userId, name: m.name }))
    .sort((a, b) => a.name.localeCompare(b.name))
}

export async function getAccessGroups(
  organizationId: string,
): Promise<Array<AccessGroup>> {
  return accessGroupsInOrder(accessGroups(await session(), organizationId))
}

export async function getAccessGroup(id: string): Promise<AccessGroup> {
  return groupOrFail(await session(), id).group
}

/** The five default groups (create_default_access_groups); refused once they exist. */
export async function createDefaultAccessGroups(
  organizationId: string,
): Promise<Array<AccessGroup>> {
  const conn = await session()
  return write(
    conn,
    'accessGroup',
    () => conn.reducers.createDefaultAccessGroups({ orgId: organizationId }),
    () => {
      const groups = accessGroups(conn, organizationId)
      return groups.some((g) => g.is_default)
        ? accessGroupsInOrder(groups)
        : undefined
    },
  )
}

export async function createAccessGroup(
  group: Omit<AccessGroup, 'id' | 'created_at' | 'updated_at'>,
): Promise<AccessGroup> {
  const conn = await session()
  const orgId = group.organization_id
  const before = new Set(docs(conn, 'accessGroup').map((r) => r.key))
  return write(
    conn,
    'accessGroup',
    () =>
      conn.reducers.createAccessGroup({
        orgId,
        groupJson: JSON.stringify(accessGroupInput(group)),
      }),
    () =>
      resolveCreated(
        docs(conn, 'accessGroup'),
        before,
        orgId,
        (g: AccessGroup) => g.slug === group.slug,
        decodeAccessGroup,
      ),
  )
}

export async function updateAccessGroup(
  id: string,
  updates: Partial<
    Omit<AccessGroup, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
): Promise<AccessGroup> {
  const conn = await session()
  const { orgId, group } = groupOrFail(conn, id)
  return write(
    conn,
    'accessGroup',
    () =>
      conn.reducers.updateAccessGroup({
        orgId,
        groupId: id,
        patchJson: JSON.stringify(accessGroupPatch(updates)),
      }),
    () => {
      const row = groupRow(conn, id)
      const g = row && decodeAccessGroup(row)
      return g && g.updated_at !== group.updated_at ? g : undefined
    },
  )
}

/** Deletes a group with its members and node settings; the module refuses a default group. */
export async function deleteAccessGroup(id: string): Promise<void> {
  const conn = await session()
  const { orgId } = groupOrFail(conn, id)
  await conn.reducers.deleteAccessGroup({ orgId, groupId: id })
}

export async function getGroupMembers(
  groupId: string,
): Promise<Array<GroupMemberWithProfile>> {
  const conn = await session()
  const { orgId } = groupOrFail(conn, groupId)
  return groupMembersWithProfiles(members(conn), groupId, people(conn, orgId))
}

/** Members of the organisation, for adding one to a group by name. */
export async function getOrgMembers(
  organizationId: string,
): Promise<Array<OrgMember>> {
  return people(await session(), organizationId)
}

function newMember(
  conn: DbConnection,
  orgId: string,
  groupId: string,
  call: () => Promise<void>,
  matches: (m: GroupMember) => boolean,
): Promise<GroupMember> {
  const before = new Set(docs(conn, 'accessGroupMember').map((r) => r.key))
  return write(conn, 'accessGroupMember', call, () =>
    resolveCreated(
      docs(conn, 'accessGroupMember'),
      before,
      orgId,
      (m: GroupMember) => m.group_id === groupId && matches(m),
      decodeGroupMember,
    ),
  )
}

/** Adds a member of the organisation to a group; `granted_by` is the caller. */
export async function addGroupMember(
  member: Omit<GroupMember, 'id' | 'created_at'>,
): Promise<GroupMember> {
  const conn = await session()
  const { orgId } = groupOrFail(conn, member.group_id)
  return newMember(
    conn,
    orgId,
    member.group_id,
    () =>
      conn.reducers.addAccessGroupMember({
        orgId,
        memberJson: JSON.stringify(groupMemberInput(member)),
      }),
    (m) => m.user_id === member.user_id && m.user_email === member.user_email,
  )
}

/**
 * Invites an address to a group (invite_access_group_member). No email goes out and the invite
 * does not attach to an account yet (the module's note says so); `grantedBy` is the caller.
 */
export async function inviteMemberByEmail(
  groupId: string,
  email: string,
  _grantedBy: string,
  validUntil?: string,
): Promise<GroupMember> {
  const conn = await session()
  const { orgId } = groupOrFail(conn, groupId)
  return newMember(
    conn,
    orgId,
    groupId,
    () =>
      conn.reducers.inviteAccessGroupMember({
        orgId,
        groupId,
        email,
        validUntil: validUntil || undefined,
      }),
    (m) => m.user_email === email,
  )
}

export async function removeGroupMember(id: string): Promise<void> {
  const conn = await session()
  const row = docs(conn, 'accessGroupMember').find((r) => r.id === id)
  if (!row) return
  await conn.reducers.removeAccessGroupMember({ orgId: row.orgId, memberId: id })
}

// ---------------------------------------------------------------------------
// Trust score (RW3; trust.rs, P16, P17)
// ---------------------------------------------------------------------------

/** The organisation's trust row, which the module refreshes on every transparency write. */
export async function getTrustScore(
  organizationId: string,
): Promise<TrustScoreSummary | null> {
  const conn = await session()
  const row = [...watch<OrgDocRow>(conn, 'trustScore').iter()].find(
    (r) => r.orgId === organizationId,
  )
  return trustSummaryFrom(row?.data)
}

/**
 * "Recalculate score" (recompute_trust, trust_reducers.rs:192): any member may ask. The row is
 * already current after every write, so this mostly records a history snapshot; it resolves
 * once that snapshot has arrived.
 */
export async function recomputeTrustScore(
  organizationId: string,
): Promise<TrustScoreSummary | null> {
  const conn = await session()
  const before = ofOrg(docs(conn, 'trustScoreHistory'), organizationId).length
  await write(
    conn,
    'trustScoreHistory',
    () => conn.reducers.recomputeTrust({ orgId: organizationId }),
    () =>
      ofOrg(docs(conn, 'trustScoreHistory'), organizationId).length > before
        ? true
        : undefined,
  )
  return getTrustScore(organizationId)
}

export async function getTrustScoreHistory(
  organizationId: string,
  limit = 12,
): Promise<Array<TrustScoreHistory>> {
  const conn = await session()
  return trustHistoryOf(
    ofOrg(docs(conn, 'trustScoreHistory'), organizationId).map(
      decodeTrustHistory,
    ),
    limit,
  )
}

// ---------------------------------------------------------------------------
// Public embed config (RW3; flows.rs:900-990)
// ---------------------------------------------------------------------------

function embedConfigOf(
  conn: DbConnection,
  orgId: string,
): PublicEmbedConfig | null {
  const row = [...watch<OrgDocRow>(conn, 'publicEmbedConfig').iter()].find(
    (r) => r.orgId === orgId,
  )
  return row ? (JSON.parse(row.data) as PublicEmbedConfig) : null
}

/** The embed settings; null until an admin first saves them (the module makes them then). */
export async function getPublicEmbedConfig(
  organizationId: string,
): Promise<PublicEmbedConfig | null> {
  return embedConfigOf(await session(), organizationId)
}

export async function updatePublicEmbedConfig(
  organizationId: string,
  config: Partial<
    Omit<
      PublicEmbedConfig,
      'id' | 'organization_id' | 'embed_token' | 'created_at' | 'updated_at'
    >
  >,
): Promise<PublicEmbedConfig> {
  const conn = await session()
  const was = embedConfigOf(conn, organizationId)?.updated_at
  return write(
    conn,
    'publicEmbedConfig',
    () =>
      conn.reducers.updatePublicEmbedConfig({
        orgId: organizationId,
        patchJson: JSON.stringify(embedConfigPatch(config)),
      }),
    () => {
      const c = embedConfigOf(conn, organizationId)
      return c && c.updated_at !== was ? c : undefined
    },
  )
}

/** A new embed token, so embeds with the old one stop working (regenerate_embed_token). */
export async function regenerateEmbedToken(
  organizationId: string,
): Promise<PublicEmbedConfig> {
  const conn = await session()
  const was = embedConfigOf(conn, organizationId)?.embed_token
  return write(
    conn,
    'publicEmbedConfig',
    () => conn.reducers.regenerateEmbedToken({ orgId: organizationId }),
    () => {
      const c = embedConfigOf(conn, organizationId)
      return c && c.embed_token !== was ? c : undefined
    },
  )
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
      const views = [...DOC_VIEWS, ...ORG_VIEWS, ...CHAIN_VIEWS, 'member'].map(
        (v) => watch<unknown>(conn, v),
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
