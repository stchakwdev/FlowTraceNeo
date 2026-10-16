// In-memory implementation of the data API for dummy auth mode.
// Same names, arguments and return shapes as the live functions in
// ../supabase.ts; data.ts picks one or the other. Mutations change module
// state, so the demo reacts for the rest of the session (a reload resets it).
import { VERIFICATION_TRUST_POINTS, calculateTrustScore } from '../supabase'
import * as seed from './dataset'
import { postDemoTransaction } from './ledger'
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
  GroupMemberWithDetails,
  NodeVisibility,
  OrgStats,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
  PaymentStatus,
  ProcessReceiptResult,
  Profile,
  PublicEmbedConfig,
  Transaction,
  TransactionWithNodes,
  TrustScoreHistory,
  TrustScoreResult,
  Verification,
  VisibilityContext,
} from '../supabase'

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

function freshStore() {
  return structuredClone({
    organizations: seed.organizations,
    profiles: seed.profiles,
    flowNodes: seed.flowNodes,
    transactions: seed.transactions,
    verifications: seed.verifications,
    trustScoreHistory: seed.trustScoreHistory,
    accessGroups: seed.accessGroups,
    groupMembers: seed.groupMembers,
    nodeVisibility: seed.nodeVisibility,
    organizationChains: seed.organizationChains,
    chainActivityLog: seed.chainActivityLog,
    campaigns: seed.campaigns,
    donations: seed.donations,
    donorUpdates: seed.donorUpdates,
    publicEmbedConfigs: seed.publicEmbedConfigs,
  })
}

let db = freshStore()

/** Restore the seed state. Used by tests. */
export function resetDemoStore() {
  db = freshStore()
}

// Reads resolve on a short timer so loading states still render briefly.
const settle = <T>(value: T): Promise<T> =>
  new Promise((resolve) =>
    setTimeout(() => resolve(structuredClone(value)), 12),
  )

const now = () => new Date().toISOString()
const newId = () => crypto.randomUUID()

function notFound(what: string): never {
  throw new Error(`${what} not found in the demo data.`)
}

const nodeRef = (id: string | null) => {
  const n = id ? db.flowNodes.find((x) => x.id === id) : undefined
  return n ? { id: n.id, name: n.name, type: n.type } : null
}

const orgRef = (id: string) => {
  const o = db.organizations.find((x) => x.id === id)
  if (!o) return notFound('Organization')
  return {
    id: o.id,
    name: o.name,
    logo_url: o.logo_url,
    trust_score: o.trust_score,
    type: o.type,
    country: o.country,
  }
}

const groupRef = (id: string | null) => {
  const g = id ? db.accessGroups.find((x) => x.id === id) : undefined
  return g
    ? { id: g.id, name: g.name, slug: g.slug, permissions: g.permissions }
    : null
}

const byDateDesc = (a: string, b: string) => (a < b ? 1 : a > b ? -1 : 0)

// ---------------------------------------------------------------------------
// Organization
// ---------------------------------------------------------------------------

export function getOrganizationStats(
  organizationId: string,
): Promise<OrgStats> {
  const txns = db.transactions.filter(
    (t) => t.organization_id === organizationId,
  )
  const ids = new Set(txns.map((t) => t.id))
  const proofs = db.verifications.filter((v) => ids.has(v.transaction_id))
  const total = txns.length
  const verified = txns.filter(
    (t) => t.verification_status === 'verified',
  ).length
  const withProofs = new Set(proofs.map((p) => p.transaction_id)).size
  return settle({
    total,
    verified,
    pending: txns.filter((t) => t.verification_status === 'pending').length,
    flagged: txns.filter((t) => t.verification_status === 'flagged').length,
    verificationRate: total > 0 ? Math.round((verified / total) * 100) : 0,
    proofStats: {
      totalProofs: proofs.length,
      verifiedProofs: proofs.filter((p) => p.status === 'verified').length,
      pendingProofs: proofs.filter((p) => p.status === 'pending').length,
      rejectedProofs: proofs.filter((p) => p.status === 'rejected').length,
      totalTrustPoints: proofs.reduce((sum, p) => sum + p.trust_points, 0),
      transactionsWithProofs: withProofs,
      proofCoverageRate: total > 0 ? Math.round((withProofs / total) * 100) : 0,
    },
  })
}

export function getOrganization(id: string) {
  const o = db.organizations.find((x) => x.id === id)
  if (!o) return Promise.reject(new Error('Organization not found in the demo data.'))
  return settle(o)
}

/** Organisation profile fields an admin can edit in Settings. */
export type OrganizationProfileUpdate = Partial<
  Pick<Organization, 'name' | 'type' | 'country' | 'currency' | 'public_slug'>
>

export function updateOrganization(
  id: string,
  updates: OrganizationProfileUpdate,
) {
  const o = db.organizations.find((x) => x.id === id)
  if (!o) return Promise.reject(new Error('Organization not found in the demo data.'))
  if (
    updates.public_slug &&
    db.organizations.some(
      (x) => x.id !== id && x.public_slug === updates.public_slug,
    )
  ) {
    return Promise.reject(
      new Error('That public link is already taken. Choose another.'),
    )
  }
  Object.assign(o, updates, { updated_at: now() })
  return settle(o)
}

export function searchOrganizations(query: string, excludeOrgId: string) {
  const q = query.toLowerCase()
  return settle(
    db.organizations
      .filter((o) => o.id !== excludeOrgId && o.name.toLowerCase().includes(q))
      .slice(0, 10)
      .map((o) => orgRef(o.id)),
  )
}

// ---------------------------------------------------------------------------
// Transactions & nodes
// ---------------------------------------------------------------------------

const withNodes = (t: Transaction): TransactionWithNodes => ({
  ...t,
  from_node: nodeRef(t.from_node_id),
  to_node: nodeRef(t.to_node_id),
})

export function getTransactions(organizationId: string) {
  return settle(
    db.transactions
      .filter((t) => t.organization_id === organizationId)
      .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
      .map(withNodes),
  )
}

export function getTransaction(id: string, organizationId: string) {
  const t = db.transactions.find(
    (x) => x.id === id && x.organization_id === organizationId,
  )
  if (!t)
    return Promise.reject(new Error('Transaction not found in the demo data.'))
  return settle(withNodes(t))
}

export function createTransaction(
  transaction: Omit<
    Transaction,
    'id' | 'created_at' | 'updated_at' | 'verification_count'
  >,
) {
  const row: Transaction = {
    ...transaction,
    id: newId(),
    verification_count: 0,
    created_at: now(),
    updated_at: now(),
  }
  db.transactions.push(row)
  // Recording a transaction posts it to the books (the ledger bridge).
  // A rule that blocks posting leaves the transaction recorded, unposted.
  void postDemoTransaction(row)
  return settle(row)
}

export function updateTransaction(
  id: string,
  organizationId: string,
  updates: Partial<
    Omit<Transaction, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
) {
  const t = db.transactions.find(
    (x) => x.id === id && x.organization_id === organizationId,
  )
  if (!t)
    return Promise.reject(new Error('Transaction not found in the demo data.'))
  Object.assign(t, updates, { updated_at: now() })
  return settle(t)
}

export function getNodes(organizationId: string) {
  return settle(
    db.flowNodes
      .filter((n) => n.organization_id === organizationId)
      .sort((a, b) => a.name.localeCompare(b.name)),
  )
}

export function createNode(
  node: Omit<FlowNode, 'id' | 'created_at' | 'balance'>,
) {
  const row: FlowNode = { ...node, id: newId(), balance: 0, created_at: now() }
  db.flowNodes.push(row)
  return settle(row)
}

// ---------------------------------------------------------------------------
// Verifications
// ---------------------------------------------------------------------------

export function getVerifications(transactionId: string) {
  return settle(
    db.verifications
      .filter((v) => v.transaction_id === transactionId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at)),
  )
}

function insertVerification(
  verification: Omit<
    Verification,
    'id' | 'created_at' | 'verified_at' | 'trust_points'
  >,
): Verification {
  const row: Verification = {
    ...verification,
    id: newId(),
    trust_points: VERIFICATION_TRUST_POINTS[verification.type],
    verified_at: verification.status === 'verified' ? now() : null,
    created_at: now(),
  }
  db.verifications.push(row)
  // Mirrors the verification count and auto-verify triggers (migration 001).
  const t = db.transactions.find((x) => x.id === row.transaction_id)
  if (t) {
    t.verification_count += 1
    if (t.verification_count >= 2 && t.verification_status === 'pending') {
      t.verification_status = 'verified'
    }
  }
  return row
}

export function createVerification(
  verification: Omit<
    Verification,
    'id' | 'created_at' | 'verified_at' | 'trust_points'
  >,
) {
  return settle(insertVerification(verification))
}

// Files stay in the browser for the session; nothing is uploaded.
function localFileUrl(file: File) {
  if (typeof URL !== 'undefined' && 'createObjectURL' in URL) {
    try {
      return URL.createObjectURL(file)
    } catch {
      // fall through
    }
  }
  return `demo://uploads/${encodeURIComponent(file.name)}`
}

export function uploadVerificationFile(
  _organizationId: string,
  _transactionId: string,
  file: File,
): Promise<string> {
  return settle(localFileUrl(file))
}

// Releases a session-local file; demo files never left the browser.
export function deleteVerificationFile(filePath: string): Promise<void> {
  if (filePath.startsWith('blob:') && typeof URL !== 'undefined') {
    try {
      URL.revokeObjectURL(filePath)
    } catch {
      // nothing to release
    }
  }
  return settle(undefined)
}

export function processReceiptOCR(
  _organizationId: string,
  transactionId: string,
  file: File,
  verificationType: Verification['type'] = 'receipt',
): Promise<ProcessReceiptResult> {
  const fileUrl = localFileUrl(file)
  const row = insertVerification({
    transaction_id: transactionId,
    type: verificationType,
    file_url: fileUrl,
    ocr_data: null,
    metadata: { file_name: file.name, demo: true },
    status: 'pending',
    verified_by: null,
    visibility_level: 'full',
    thumbnail_url: null,
    executor_name: null,
    gps_latitude: null,
    gps_longitude: null,
    gps_accuracy: null,
    location_name: null,
    description: file.name,
    created_by: seed.DEMO_USER_ID,
  })
  const t = db.transactions.find((x) => x.id === transactionId)
  return settle({
    success: true,
    verification: {
      id: row.id,
      file_url: fileUrl,
      type: row.type,
      status: row.status,
    },
    ocr: {
      processed: false,
      message:
        "Receipt scanning isn't available in the demo, so the file was attached without reading its contents.",
    },
    high_value_alert: (t?.amount ?? 0) >= 1000000,
  })
}

// ---------------------------------------------------------------------------
// Trust score
// ---------------------------------------------------------------------------

export function getTrustScoreHistory(organizationId: string, limit = 12) {
  const rows = db.trustScoreHistory
    .filter((h) => h.organization_id === organizationId)
    .sort((a, b) => byDateDesc(a.calculated_at, b.calculated_at))
    .slice(0, limit)
    .reverse()
  return settle(rows)
}

export function calculateTrustScoreAPI(
  organizationId: string,
): Promise<TrustScoreResult> {
  const o = db.organizations.find((x) => x.id === organizationId)
  if (!o)
    return Promise.reject(new Error('Organization not found in the demo data.'))
  const txns = db.transactions.filter(
    (t) => t.organization_id === organizationId,
  )
  const total = txns.length
  const verifiedTxns = txns.filter(
    (t) => t.verification_status === 'verified',
  ).length
  const documented = new Set(
    db.verifications
      .filter((v) => txns.some((t) => t.id === v.transaction_id))
      .map((v) => v.transaction_id),
  ).size
  const latest = db.trustScoreHistory
    .filter((h) => h.organization_id === organizationId)
    .sort((a, b) => byDateDesc(a.calculated_at, b.calculated_at))[0] as
    | TrustScoreHistory
    | undefined
  const verification_rate = total ? Math.round((verifiedTxns / total) * 100) : 0
  const documentation_rate = total ? Math.round((documented / total) * 100) : 0
  const consistency_score = latest?.consistency_score ?? 80
  const trust_score = calculateTrustScore(
    verification_rate,
    documentation_rate,
    consistency_score,
  )
  const previous_score = o.trust_score
  o.trust_score = trust_score
  const entry: TrustScoreHistory = {
    id: newId(),
    organization_id: organizationId,
    score: trust_score,
    breakdown: {
      verification: verification_rate,
      documentation: documentation_rate,
      consistency: consistency_score,
    },
    verification_rate,
    documentation_rate,
    consistency_score,
    calculated_at: now(),
  }
  db.trustScoreHistory.push(entry)
  return settle({
    success: true,
    trust_score,
    previous_score,
    delta: trust_score - previous_score,
    details: {
      trust_score,
      verification_rate,
      documentation_rate,
      consistency_score,
      total_transactions: total,
      verified_transactions: verifiedTxns,
      documented_transactions: documented,
    },
  })
}

// ---------------------------------------------------------------------------
// Flow data (shape of the generate-flow-data edge function)
// ---------------------------------------------------------------------------

export function getFlowDataAPI(
  organizationId: string,
  startDate?: string,
  endDate?: string,
): Promise<FlowDataResult> {
  const txns = db.transactions.filter(
    (t) =>
      t.organization_id === organizationId &&
      t.verification_status !== 'rejected' &&
      (!startDate || t.transaction_date >= startDate.slice(0, 10)) &&
      (!endDate || t.transaction_date <= endDate.slice(0, 10)),
  )

  const nodes = new Map<string, FlowDataNode>()
  const touch = (id: string | null) => {
    if (!id || nodes.has(id)) return
    const n = db.flowNodes.find((x) => x.id === id)
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

    const proofs = db.verifications.filter((v) => v.transaction_id === t.id)
    for (const n of [from, to]) {
      if (!n) continue
      n.verification_count = (n.verification_count ?? 0) + proofs.length
      n.verified_count =
        (n.verified_count ?? 0) +
        proofs.filter((p) => p.status === 'verified').length
      n.pending_count =
        (n.pending_count ?? 0) +
        proofs.filter((p) => p.status === 'pending').length
      n.rejected_count =
        (n.rejected_count ?? 0) +
        proofs.filter((p) => p.status === 'rejected').length
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
  return settle({
    nodes: [...nodes.values()],
    edges: edgeList,
    summary: {
      total_nodes: nodes.size,
      total_edges: edgeList.length,
      total_flow: edgeList.reduce((s, e) => s + e.value, 0),
      transaction_count: txns.length,
      currency: seed.demoOrganization.currency,
    },
  })
}

// ---------------------------------------------------------------------------
// Access groups, members, visibility
// ---------------------------------------------------------------------------

export function getAccessGroups(organizationId: string) {
  return settle(
    db.accessGroups
      .filter((g) => g.organization_id === organizationId)
      .sort((a, b) => a.priority - b.priority),
  )
}

export function getAccessGroup(id: string) {
  const g = db.accessGroups.find((x) => x.id === id)
  if (!g)
    return Promise.reject(new Error('Access group not found in the demo data.'))
  return settle(g)
}

export function createAccessGroup(
  group: Omit<AccessGroup, 'id' | 'created_at' | 'updated_at'>,
) {
  const row: AccessGroup = {
    ...group,
    id: newId(),
    created_at: now(),
    updated_at: now(),
  }
  db.accessGroups.push(row)
  return settle(row)
}

export function updateAccessGroup(
  id: string,
  updates: Partial<
    Omit<AccessGroup, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
) {
  const g = db.accessGroups.find((x) => x.id === id)
  if (!g)
    return Promise.reject(new Error('Access group not found in the demo data.'))
  Object.assign(g, updates, { updated_at: now() })
  return settle(g)
}

export async function deleteAccessGroup(id: string) {
  db.accessGroups = db.accessGroups.filter((g) => g.id !== id)
  db.groupMembers = db.groupMembers.filter((m) => m.group_id !== id)
  db.nodeVisibility = db.nodeVisibility.filter((v) => v.group_id !== id)
  await settle(null)
}

const profileRef = (userId: string | null) => {
  const p: Profile | undefined = userId
    ? db.profiles.find((x) => x.id === userId)
    : undefined
  return p ? { id: p.id, email: p.email, full_name: p.full_name } : null
}

export function getGroupMembers(groupId: string) {
  return settle(
    db.groupMembers
      .filter((m) => m.group_id === groupId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map((m) => ({ ...m, profile: profileRef(m.user_id) })),
  )
}

export function getUserGroups(userId: string, organizationId: string) {
  const at = now()
  const rows = db.groupMembers
    .filter(
      (m) =>
        m.user_id === userId &&
        m.valid_from <= at &&
        (m.valid_until === null || m.valid_until > at),
    )
    .map((m) => {
      const g = db.accessGroups.find((x) => x.id === m.group_id)
      return g ? { ...m, group: g } : null
    })
    .filter(
      (m): m is GroupMemberWithDetails & { group: AccessGroup } =>
        m !== null && m.group.organization_id === organizationId,
    )
  return settle(rows)
}

export function addGroupMember(member: Omit<GroupMember, 'id' | 'created_at'>) {
  const row: GroupMember = { ...member, id: newId(), created_at: now() }
  db.groupMembers.push(row)
  return settle(row)
}

export async function removeGroupMember(id: string) {
  db.groupMembers = db.groupMembers.filter((m) => m.id !== id)
  await settle(null)
}

// No email goes out in the demo; the invite is recorded as a pending member.
export function inviteMemberByEmail(
  groupId: string,
  email: string,
  grantedBy: string,
  validUntil?: string,
) {
  const row: GroupMember = {
    id: newId(),
    group_id: groupId,
    user_id: null,
    user_email: email,
    granted_by: grantedBy,
    valid_from: now(),
    valid_until: validUntil || null,
    notes: 'Invited in the demo. No email was sent.',
    created_at: now(),
  }
  db.groupMembers.push(row)
  return settle(row)
}

export function getNodeVisibility(nodeId: string) {
  return settle(
    db.nodeVisibility
      .filter((v) => v.node_id === nodeId)
      .map((v) => {
        const g = db.accessGroups.find((x) => x.id === v.group_id)
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
      }),
  )
}

export function setNodeVisibility(
  nodeId: string,
  groupId: string,
  settings: Partial<
    Omit<NodeVisibility, 'id' | 'node_id' | 'group_id' | 'created_at'>
  >,
) {
  let row = db.nodeVisibility.find(
    (v) => v.node_id === nodeId && v.group_id === groupId,
  )
  if (row) {
    Object.assign(row, settings)
  } else {
    row = {
      id: newId(),
      node_id: nodeId,
      group_id: groupId,
      can_view: true,
      can_view_amounts: null,
      can_view_proofs: null,
      can_view_executor: null,
      display_name: null,
      created_at: now(),
      ...settings,
    }
    db.nodeVisibility.push(row)
  }
  return settle(row)
}

export async function removeNodeVisibility(nodeId: string, groupId: string) {
  db.nodeVisibility = db.nodeVisibility.filter(
    (v) => !(v.node_id === nodeId && v.group_id === groupId),
  )
  await settle(null)
}

export function getPublicEmbedConfig(organizationId: string) {
  return settle(
    db.publicEmbedConfigs.find((c) => c.organization_id === organizationId) ??
      null,
  )
}

export function updatePublicEmbedConfig(
  organizationId: string,
  config: Partial<
    Omit<
      PublicEmbedConfig,
      'id' | 'organization_id' | 'embed_token' | 'created_at' | 'updated_at'
    >
  >,
) {
  const c = db.publicEmbedConfigs.find(
    (x) => x.organization_id === organizationId,
  )
  if (!c)
    return Promise.reject(
      new Error('Embed settings not found in the demo data.'),
    )
  Object.assign(c, config, { updated_at: now() })
  return settle(c)
}

export function getUserPermissions(
  userId: string,
  organizationId: string,
): Promise<AccessGroupPermissions> {
  // Union of the user's groups, like get_user_permissions(). The demo user
  // sits on the Board, so this is full access.
  const groups = db.groupMembers
    .filter((m) => m.user_id === userId)
    .map((m) => db.accessGroups.find((g) => g.id === m.group_id))
    .filter(
      (g): g is AccessGroup => !!g && g.organization_id === organizationId,
    )
  const merged = {} as AccessGroupPermissions
  for (const key of Object.keys(seed.accessGroups[0].permissions) as Array<
    keyof AccessGroupPermissions
  >) {
    merged[key] = groups.some((g) => g.permissions[key])
  }
  return settle(merged)
}

export function getVisibilityContext(
  _userId: string,
  organizationId: string,
): Promise<VisibilityContext> {
  const nodes = db.flowNodes.filter(
    (n) => n.organization_id === organizationId,
  ).length
  const txns = db.transactions.filter(
    (t) => t.organization_id === organizationId,
  ).length
  return settle({
    total_nodes: nodes,
    visible_nodes: nodes,
    total_transactions: txns,
    visible_transactions: txns,
    visibility_percentage: 100,
  })
}

// ---------------------------------------------------------------------------
// Organization chains
// ---------------------------------------------------------------------------

const chainDetails = (c: OrganizationChain): OrganizationChainWithDetails => ({
  ...c,
  source_org: orgRef(c.source_org_id),
  target_org: orgRef(c.target_org_id),
  link_node: nodeRef(c.link_node_id),
  granted_group: groupRef(c.granted_group_id),
})

const logChain = (
  chain: OrganizationChain,
  action: ChainActivityLog['action'],
  actorOrgId: string,
  details: Record<string, unknown> = {},
) => {
  db.chainActivityLog.push({
    id: newId(),
    chain_id: chain.id,
    action,
    actor_id: seed.DEMO_USER_ID,
    actor_org_id: actorOrgId,
    details,
    created_at: now(),
  })
}

export function getOutgoingChains(organizationId: string) {
  return settle(
    db.organizationChains
      .filter((c) => c.source_org_id === organizationId)
      .sort((a, b) => byDateDesc(a.requested_at, b.requested_at))
      .map(chainDetails),
  )
}

export function getIncomingChains(organizationId: string) {
  return settle(
    db.organizationChains
      .filter((c) => c.target_org_id === organizationId)
      .sort((a, b) => byDateDesc(a.requested_at, b.requested_at))
      .map(chainDetails),
  )
}

export function getChain(chainId: string) {
  const c = db.organizationChains.find((x) => x.id === chainId)
  if (!c) return Promise.reject(new Error('Chain not found in the demo data.'))
  return settle(chainDetails(c))
}

export function requestChain(
  sourceOrgId: string,
  targetOrgId: string,
  requestedBy: string,
  message?: string,
) {
  const row: OrganizationChain = {
    id: newId(),
    source_org_id: sourceOrgId,
    target_org_id: targetOrgId,
    link_node_id: null,
    status: 'pending',
    granted_group_id: null,
    request_message: message || null,
    rejection_reason: null,
    requested_by: requestedBy,
    requested_at: now(),
    approved_by: null,
    approved_at: null,
    revoked_by: null,
    revoked_at: null,
    created_at: now(),
    updated_at: now(),
  }
  db.organizationChains.push(row)
  logChain(row, 'requested', sourceOrgId, { message: row.request_message })
  return settle(row)
}

function changeChain(
  chainId: string,
  requiredStatus: OrganizationChain['status'],
  updates: Partial<OrganizationChain>,
  action: ChainActivityLog['action'],
  actorSide: 'source' | 'target',
) {
  const c = db.organizationChains.find(
    (x) => x.id === chainId && x.status === requiredStatus,
  )
  if (!c) {
    return Promise.reject(
      new Error(
        `This chain is no longer ${requiredStatus}, so it can't be changed.`,
      ),
    )
  }
  Object.assign(c, updates, { updated_at: now() })
  logChain(
    c,
    action,
    actorSide === 'source' ? c.source_org_id : c.target_org_id,
    updates,
  )
  return settle(c)
}

export function approveChain(
  chainId: string,
  approvedBy: string,
  grantedGroupId: string,
  linkNodeId?: string,
) {
  return changeChain(
    chainId,
    'pending',
    {
      status: 'active',
      approved_by: approvedBy,
      approved_at: now(),
      granted_group_id: grantedGroupId,
      link_node_id: linkNodeId || null,
    },
    'approved',
    'target',
  )
}

export function rejectChain(
  chainId: string,
  rejectedBy: string,
  reason?: string,
) {
  return changeChain(
    chainId,
    'pending',
    {
      status: 'rejected',
      approved_by: rejectedBy,
      approved_at: now(),
      rejection_reason: reason || null,
    },
    'rejected',
    'target',
  )
}

export function revokeChain(chainId: string, revokedBy: string) {
  return changeChain(
    chainId,
    'active',
    { status: 'revoked', revoked_by: revokedBy, revoked_at: now() },
    'revoked',
    'target',
  )
}

export async function cancelChainRequest(chainId: string) {
  db.organizationChains = db.organizationChains.filter(
    (c) => !(c.id === chainId && c.status === 'pending'),
  )
  await settle(null)
}

export function updateChainPermissions(
  chainId: string,
  grantedGroupId: string,
  linkNodeId?: string,
) {
  return changeChain(
    chainId,
    'active',
    { granted_group_id: grantedGroupId, link_node_id: linkNodeId || null },
    'permissions_updated',
    'target',
  )
}

export function getChainActivityLog(chainId: string) {
  return settle(
    db.chainActivityLog
      .filter((l) => l.chain_id === chainId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at)),
  )
}

export function getChainCounts(organizationId: string): Promise<ChainCounts> {
  const c = db.organizationChains
  return settle({
    outgoing_pending: c.filter(
      (x) => x.source_org_id === organizationId && x.status === 'pending',
    ).length,
    outgoing_active: c.filter(
      (x) => x.source_org_id === organizationId && x.status === 'active',
    ).length,
    incoming_pending: c.filter(
      (x) => x.target_org_id === organizationId && x.status === 'pending',
    ).length,
    incoming_active: c.filter(
      (x) => x.target_org_id === organizationId && x.status === 'active',
    ).length,
  })
}

export function getChainTrustScore(
  chainId: string,
): Promise<ChainTrustScore | null> {
  const c = db.organizationChains.find((x) => x.id === chainId)
  if (!c) return settle(null)
  const source = orgRef(c.source_org_id).trust_score
  const target = orgRef(c.target_org_id).trust_score
  return settle({
    aggregated_score: Math.round((source + target) / 2),
    org_count: 2,
    scores: { source_org: source, target_org: target },
  })
}

// ---------------------------------------------------------------------------
// Campaigns
// ---------------------------------------------------------------------------

const withTarget = (c: Campaign): CampaignWithDetails => ({
  ...c,
  target_node: nodeRef(c.target_node_id),
})

export function getCampaigns(organizationId: string) {
  return settle(
    db.campaigns
      .filter((c) => c.organization_id === organizationId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map(withTarget),
  )
}

export function getCampaign(id: string, organizationId: string) {
  const c = db.campaigns.find(
    (x) => x.id === id && x.organization_id === organizationId,
  )
  if (!c)
    return Promise.reject(new Error('Campaign not found in the demo data.'))
  return settle(withTarget(c))
}

export function getCampaignBySlug(slug: string) {
  const c = db.campaigns.find(
    (x) => x.public_slug === slug && x.is_public && x.status === 'active',
  )
  if (!c)
    return Promise.reject(new Error('This campaign is not open for donations.'))
  return settle(withTarget(c))
}

function slugify(name: string) {
  const base = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
  let slug = base || 'campaign'
  let i = 2
  while (db.campaigns.some((c) => c.public_slug === slug))
    slug = `${base}-${i++}`
  return slug
}

export function createCampaign(
  campaign: Omit<
    Campaign,
    'id' | 'current_amount' | 'created_at' | 'updated_at'
  >,
) {
  const row: Campaign = {
    ...campaign,
    public_slug:
      campaign.public_slug ??
      (campaign.is_public ? slugify(campaign.name) : null),
    id: newId(),
    current_amount: 0,
    created_at: now(),
    updated_at: now(),
  }
  db.campaigns.push(row)
  return settle(row)
}

export function updateCampaign(
  id: string,
  organizationId: string,
  updates: Partial<
    Omit<Campaign, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
) {
  const c = db.campaigns.find(
    (x) => x.id === id && x.organization_id === organizationId,
  )
  if (!c)
    return Promise.reject(new Error('Campaign not found in the demo data.'))
  Object.assign(c, updates, { updated_at: now() })
  return settle(c)
}

export async function deleteCampaign(id: string, organizationId: string) {
  db.campaigns = db.campaigns.filter(
    (c) =>
      !(
        c.id === id &&
        c.organization_id === organizationId &&
        c.status === 'draft'
      ),
  )
  await settle(null)
}

const DAY = 24 * 60 * 60 * 1000

export function getCampaignStats(campaignId: string): Promise<CampaignStats> {
  const c = db.campaigns.find((x) => x.id === campaignId)
  if (!c)
    return Promise.reject(new Error('Campaign not found in the demo data.'))
  const confirmed = db.donations.filter(
    (d) => d.campaign_id === campaignId && d.payment_status === 'confirmed',
  )
  const amounts = confirmed.map((d) => d.amount)
  const donors = new Set(
    confirmed.map((d) =>
      d.is_anonymous ? d.id : (d.donor_email ?? d.donor_name ?? d.id),
    ),
  )
  // Every figure derives from the confirmed donation ledger.
  const raised = amounts.reduce((s, a) => s + a, 0)
  const t = Date.now()
  return settle({
    donation_count: confirmed.length,
    unique_donors: donors.size,
    total_raised: raised,
    funding_percentage:
      c.target_amount > 0
        ? Math.round((raised / c.target_amount) * 1000) / 10
        : 0,
    avg_donation: amounts.length ? Math.round(raised / amounts.length) : 0,
    largest_donation: amounts.length ? Math.max(...amounts) : 0,
    smallest_donation: amounts.length ? Math.min(...amounts) : 0,
    days_remaining: c.ends_at
      ? Math.max(0, Math.ceil((new Date(c.ends_at).getTime() - t) / DAY))
      : null,
    days_active: Math.max(
      0,
      Math.floor((t - new Date(c.starts_at).getTime()) / DAY),
    ),
  })
}

export function getCampaignCounts(
  organizationId: string,
): Promise<CampaignCounts> {
  const cs = db.campaigns.filter((c) => c.organization_id === organizationId)
  const ids = new Set(cs.map((c) => c.id))
  return settle({
    total_campaigns: cs.length,
    active_campaigns: cs.filter((c) => c.status === 'active').length,
    completed_campaigns: cs.filter((c) => c.status === 'completed').length,
    total_raised: cs.reduce((s, c) => s + c.current_amount, 0),
    total_donations: db.donations.filter(
      (d) => ids.has(d.campaign_id) && d.payment_status === 'confirmed',
    ).length,
  })
}

export function getUnderfundedCampaigns(organizationId: string, limit = 5) {
  const pct = (c: Campaign) =>
    c.target_amount ? c.current_amount / c.target_amount : 1
  return settle(
    db.campaigns
      .filter(
        (c) =>
          c.organization_id === organizationId &&
          c.status === 'active' &&
          pct(c) < 1,
      )
      .sort((a, b) => pct(a) - pct(b))
      .slice(0, limit),
  )
}

// ---------------------------------------------------------------------------
// Donations & donor updates
// ---------------------------------------------------------------------------

const withCampaign = (d: Donation): DonationWithDetails => {
  const c = db.campaigns.find((x) => x.id === d.campaign_id)
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
    earmarked_node: nodeRef(d.earmarked_node_id),
  }
}

export function getDonations(campaignId: string) {
  return settle(
    db.donations
      .filter((d) => d.campaign_id === campaignId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map(withCampaign),
  )
}

export function getDonation(id: string) {
  const d = db.donations.find((x) => x.id === id)
  if (!d)
    return Promise.reject(new Error('Donation not found in the demo data.'))
  return settle(withCampaign(d))
}

export function getDonorDonations(donorEmail: string) {
  return settle(
    db.donations
      .filter((d) => d.donor_email === donorEmail)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map(withCampaign),
  )
}

// Mirrors donations_update_campaign_amount (migration 011).
function applyToCampaign(
  d: Donation,
  from: PaymentStatus | null,
  to: PaymentStatus,
) {
  const c = db.campaigns.find((x) => x.id === d.campaign_id)
  if (!c) return
  if (from !== 'confirmed' && to === 'confirmed') c.current_amount += d.amount
  if (from === 'confirmed' && to !== 'confirmed') c.current_amount -= d.amount
}

export function createDonation(
  donation: Omit<
    Donation,
    'id' | 'impact_percentage' | 'created_at' | 'updated_at'
  >,
) {
  const row: Donation = {
    ...donation,
    id: newId(),
    impact_percentage: null,
    created_at: now(),
    updated_at: now(),
  }
  db.donations.push(row)
  applyToCampaign(row, null, row.payment_status)
  return settle(row)
}

export function updateDonationStatus(
  id: string,
  status: PaymentStatus,
  paymentReference?: string,
) {
  const d = db.donations.find((x) => x.id === id)
  if (!d)
    return Promise.reject(new Error('Donation not found in the demo data.'))
  applyToCampaign(d, d.payment_status, status)
  Object.assign(d, {
    payment_status: status,
    payment_reference: paymentReference || null,
    updated_at: now(),
  })
  return settle(d)
}

const updateDetails = (u: DonorUpdate): DonorUpdateWithDetails => {
  const d = db.donations.find((x) => x.id === u.donation_id)
  const c = u.campaign_id
    ? db.campaigns.find((x) => x.id === u.campaign_id)
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
    related_node: nodeRef(u.related_node_id),
  }
}

export function getDonorUpdates(donationId: string) {
  return settle(
    db.donorUpdates
      .filter((u) => u.donation_id === donationId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map(updateDetails),
  )
}

export function getCampaignUpdates(campaignId: string) {
  return settle(
    db.donorUpdates
      .filter((u) => u.campaign_id === campaignId)
      .sort((a, b) => byDateDesc(a.created_at, b.created_at))
      .map(updateDetails),
  )
}

export function createDonorUpdate(
  update: Omit<DonorUpdate, 'id' | 'sent_at' | 'created_at'>,
) {
  const row: DonorUpdate = {
    ...update,
    id: newId(),
    sent_at: null,
    created_at: now(),
  }
  db.donorUpdates.push(row)
  return settle(row)
}

// Nothing is delivered in the demo; the update is just marked as sent.
export function markUpdateSent(id: string) {
  const u = db.donorUpdates.find((x) => x.id === id)
  if (!u) return Promise.reject(new Error('Update not found in the demo data.'))
  u.sent_at = now()
  return settle(u)
}

// ---------------------------------------------------------------------------
// Campaign report (public donate page and campaign detail)
// ---------------------------------------------------------------------------

export interface CampaignReportEarmark {
  id: string
  name: string
  type: FlowNode['type']
  /** Money received by this node (non-rejected inflows). */
  received: number
  /** The part of `received` backed by verified proof. */
  evidenced: number
  /** The campaign's own target node. */
  is_target: boolean
}

export interface CampaignReport {
  campaign: CampaignWithDetails
  organization: {
    id: string
    name: string
    trust_score: number
    verified: boolean
  }
  /** Verified spending into the campaign's target node, capped at the raised amount. */
  evidenced_amount: number | null
  earmarks: Array<CampaignReportEarmark>
  /** Recent programme spending, newest first, with its proof state. */
  spending: Array<TransactionWithNodes>
}

export function getCampaignReport(campaignId: string): Promise<CampaignReport> {
  const c = db.campaigns.find((x) => x.id === campaignId)
  if (!c)
    return Promise.reject(new Error('Campaign not found in the demo data.'))
  const o = db.organizations.find((x) => x.id === c.organization_id)
  if (!o)
    return Promise.reject(new Error('Organization not found in the demo data.'))
  const txns = db.transactions.filter(
    (t) =>
      t.organization_id === c.organization_id &&
      t.verification_status !== 'rejected',
  )
  const sum = (rows: Array<Transaction>) =>
    rows.reduce((s, t) => s + t.amount, 0)
  const earmarks = db.flowNodes
    .filter(
      (n) =>
        n.organization_id === c.organization_id &&
        (n.type === 'project' || n.type === 'department'),
    )
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
        is_target: n.id === c.target_node_id,
      }
    })
    .sort(
      (a, b) =>
        Number(b.is_target) - Number(a.is_target) || b.received - a.received,
    )
  const programme = new Set(earmarks.map((e) => e.id))
  const spending = txns
    .filter(
      (t) =>
        (t.to_node_id && programme.has(t.to_node_id)) ||
        (t.from_node_id && programme.has(t.from_node_id)),
    )
    .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
    .slice(0, 6)
    .map(withNodes)
  const target = earmarks.find((e) => e.is_target)
  return settle({
    campaign: withTarget(c),
    organization: {
      id: o.id,
      name: o.name,
      trust_score: o.trust_score,
      verified: o.verified,
    },
    evidenced_amount: target
      ? Math.min(target.evidenced, c.current_amount)
      : null,
    earmarks,
    spending,
  })
}
