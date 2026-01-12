import { createClient, SupabaseClient } from '@supabase/supabase-js'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL || ''
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY || ''

// Lazy initialization to avoid SSR issues
let _supabase: SupabaseClient | null = null

export const supabase = new Proxy({} as SupabaseClient, {
  get(_target, prop) {
    if (!_supabase) {
      if (!supabaseUrl || !supabaseAnonKey) {
        console.warn('Supabase not configured - using mock client')
        // Return a mock that won't crash
        return () => Promise.resolve({ data: null, error: null })
      }
      _supabase = createClient(supabaseUrl, supabaseAnonKey)
    }
    return (_supabase as Record<string, unknown>)[prop as string]
  },
})

// Development mode constants
export const DEV_MODE = !import.meta.env.VITE_WORKOS_CLIENT_ID
export const DEV_ORG_ID = '550e8400-e29b-41d4-a716-446655440000'

// Database types (matching migration schema)
export interface Organization {
  id: string
  name: string
  type: 'NGO' | 'SME' | 'SOCIAL_ENTERPRISE' | 'GOVERNMENT' | null
  country: string
  currency: string
  trust_score: number
  verified: boolean
  public_slug: string | null
  logo_url: string | null
  created_at: string
  updated_at: string
}

export interface Profile {
  id: string // WorkOS user ID
  organization_id: string | null
  email: string
  full_name: string | null
  phone: string | null
  role: 'admin' | 'manager' | 'member' | 'viewer'
  created_at: string
}

export interface FlowNode {
  id: string
  organization_id: string
  name: string
  type:
    | 'donor'
    | 'department'
    | 'vendor'
    | 'beneficiary'
    | 'account'
    | 'project'
  balance: number
  metadata: Record<string, unknown>
  created_at: string
}

export interface Transaction {
  id: string
  organization_id: string
  from_node_id: string | null
  to_node_id: string | null
  amount: number
  currency: string
  exchange_rate: number | null
  amount_usd: number | null
  description: string
  category: string | null
  reference: string | null
  notes: string | null
  verification_status: 'pending' | 'verified' | 'flagged' | 'rejected'
  verification_count: number
  transaction_date: string
  created_by: string | null
  created_at: string
  updated_at: string
}

export interface TransactionWithNodes extends Transaction {
  from_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
  to_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
}

// Extended verification types for proof system
export type VerificationType =
  | 'receipt'
  | 'photo'
  | 'gps'
  | 'signature'
  | 'biometric'
  | 'bank_statement'
  | 'document'
  | 'invoice'
  | 'video'
  | 'audit'
  | 'contract'
  | 'government_doc'

// Trust points per verification type
export const VERIFICATION_TRUST_POINTS: Record<VerificationType, number> = {
  receipt: 5,
  bank_statement: 10,
  invoice: 4,
  photo: 8,
  gps: 3,
  video: 10,
  biometric: 7,
  signature: 7,
  audit: 15,
  contract: 6,
  government_doc: 8,
  document: 4,
}

// Visibility levels for proofs
export type VerificationVisibility = 'full' | 'thumbnail' | 'summary' | 'hidden'

export interface Verification {
  id: string
  transaction_id: string
  type: VerificationType
  file_url: string | null
  ocr_data: Record<string, unknown> | null
  metadata: Record<string, unknown>
  status: 'pending' | 'verified' | 'rejected'
  verified_by: string | null
  verified_at: string | null
  created_at: string
  // New Phase 3 fields
  trust_points: number
  visibility_level: VerificationVisibility
  thumbnail_url: string | null
  executor_name: string | null
  gps_latitude: number | null
  gps_longitude: number | null
  gps_accuracy: number | null
  location_name: string | null
  description: string | null
  created_by: string | null
}

// Verification access log for audit trail
export interface VerificationAccessLog {
  id: string
  verification_id: string
  accessed_by: string
  access_type: 'view' | 'download'
  access_level: VerificationVisibility
  ip_address: string | null
  user_agent: string | null
  accessed_at: string
}

export interface TrustScoreHistory {
  id: string
  organization_id: string
  score: number
  breakdown: Record<string, number>
  verification_rate: number | null
  documentation_rate: number | null
  consistency_score: number | null
  calculated_at: string
}

// ============================================================================
// ACCESS GROUPS & VISIBILITY TYPES
// ============================================================================

export interface AccessGroupPermissions {
  can_view_nodes: boolean
  can_view_amounts: boolean
  can_view_proofs: boolean
  can_view_executor: boolean
  can_view_full_proofs: boolean
  can_edit_nodes: boolean
  can_edit_transactions: boolean
  can_manage_members: boolean
  can_manage_groups: boolean
}

export interface AccessGroup {
  id: string
  organization_id: string
  name: string
  slug: string
  description: string | null
  icon: string
  color: string
  permissions: AccessGroupPermissions
  node_scope: 'all' | 'assigned' | 'downstream'
  is_default: boolean
  is_public: boolean
  priority: number
  created_at: string
  updated_at: string
}

export interface GroupMember {
  id: string
  group_id: string
  user_id: string | null
  user_email: string | null
  granted_by: string | null
  valid_from: string
  valid_until: string | null
  notes: string | null
  created_at: string
}

export interface GroupMemberWithDetails extends GroupMember {
  group: Pick<AccessGroup, 'id' | 'name' | 'slug' | 'color' | 'icon'>
  profile?: Pick<Profile, 'id' | 'email' | 'full_name'> | null
}

export interface NodeVisibility {
  id: string
  node_id: string
  group_id: string
  can_view: boolean
  can_view_amounts: boolean | null
  can_view_proofs: boolean | null
  can_view_executor: boolean | null
  display_name: string | null
  created_at: string
}

export interface TransactionVisibility {
  id: string
  transaction_id: string
  group_id: string
  can_view: boolean
  can_view_amount: boolean | null
  can_view_proofs: boolean | null
  can_view_executor: boolean | null
  created_at: string
}

export interface PublicEmbedConfig {
  id: string
  organization_id: string
  display_mode: 'simplified' | 'detailed' | 'custom'
  show_amounts: boolean
  amount_format: 'exact' | 'ranges' | 'percentages' | 'hidden'
  show_node_names: boolean
  show_trust_score: boolean
  show_transaction_count: boolean
  show_last_updated: boolean
  visible_node_types: string[]
  hidden_node_ids: string[]
  theme: {
    primary_color: string
    background: 'light' | 'dark'
    border_radius: 'none' | 'sm' | 'md' | 'lg' | 'full'
    show_logo: boolean
  }
  allow_embed: boolean
  allowed_domains: string[]
  embed_token: string
  created_at: string
  updated_at: string
}

export interface VisibilityContext {
  total_nodes: number
  visible_nodes: number
  total_transactions: number
  visible_transactions: number
  visibility_percentage: number
}

export interface OrgStats {
  total: number
  verified: number
  pending: number
  flagged: number
  verificationRate: number
  // Proof verification stats
  proofStats: {
    totalProofs: number
    verifiedProofs: number
    pendingProofs: number
    rejectedProofs: number
    totalTrustPoints: number
    transactionsWithProofs: number
    proofCoverageRate: number // % of transactions with at least one proof
  }
}

// Profile helpers
export async function getProfile(workosUserId: string) {
  const { data, error } = await supabase
    .from('profiles')
    .select('*, organization:organizations(*)')
    .eq('id', workosUserId)
    .single()

  if (error && error.code !== 'PGRST116') throw error // PGRST116 = not found
  return data as (Profile & { organization: Organization | null }) | null
}

export async function createOrUpdateProfile(
  workosUserId: string,
  email: string,
  fullName?: string,
  organizationId?: string,
) {
  const { data, error } = await supabase
    .from('profiles')
    .upsert({
      id: workosUserId,
      email,
      full_name: fullName || null,
      organization_id: organizationId || null,
    })
    .select()
    .single()

  if (error) throw error
  return data as Profile
}

// Organization helpers
export async function getOrganization(id: string) {
  const { data, error } = await supabase
    .from('organizations')
    .select('*')
    .eq('id', id)
    .single()

  if (error) throw error
  return data as Organization
}

export async function getOrganizationStats(
  organizationId: string,
): Promise<OrgStats> {
  // Fetch transactions with verification counts
  const { data: transactions, error: txnError } = await supabase
    .from('transactions')
    .select('id, verification_status, verification_count')
    .eq('organization_id', organizationId)

  if (txnError) throw txnError

  // Fetch verification proof stats
  const { data: verifications, error: verError } = await supabase
    .from('verifications')
    .select('id, status, trust_points, transaction_id')
    .in('transaction_id', transactions.map((t) => t.id))

  if (verError) throw verError

  const txnList = transactions
  const total = txnList.length
  const verified = txnList.filter(
    (t) => t.verification_status === 'verified',
  ).length
  const pending = txnList.filter(
    (t) => t.verification_status === 'pending',
  ).length
  const flagged = txnList.filter(
    (t) => t.verification_status === 'flagged',
  ).length
  const verificationRate = total > 0 ? Math.round((verified / total) * 100) : 0

  // Calculate proof stats
  const proofList = verifications ?? []
  const totalProofs = proofList.length
  const verifiedProofs = proofList.filter((p) => p.status === 'verified').length
  const pendingProofs = proofList.filter((p) => p.status === 'pending').length
  const rejectedProofs = proofList.filter((p) => p.status === 'rejected').length
  const totalTrustPoints = proofList.reduce(
    (sum, p) => sum + (p.trust_points ?? 0),
    0,
  )

  // Count unique transactions with at least one proof
  const transactionIdsWithProofs = new Set(proofList.map((p) => p.transaction_id))
  const transactionsWithProofs = transactionIdsWithProofs.size
  const proofCoverageRate =
    total > 0 ? Math.round((transactionsWithProofs / total) * 100) : 0

  return {
    total,
    verified,
    pending,
    flagged,
    verificationRate,
    proofStats: {
      totalProofs,
      verifiedProofs,
      pendingProofs,
      rejectedProofs,
      totalTrustPoints,
      transactionsWithProofs,
      proofCoverageRate,
    },
  }
}

// Transaction helpers
export async function getTransactions(organizationId: string) {
  const { data, error } = await supabase
    .from('transactions')
    .select(
      `
      *,
      from_node:flow_nodes!from_node_id(id, name, type),
      to_node:flow_nodes!to_node_id(id, name, type)
    `,
    )
    .eq('organization_id', organizationId)
    .order('transaction_date', { ascending: false })

  if (error) throw error
  return data as Array<TransactionWithNodes>
}

export async function getTransaction(id: string, organizationId: string) {
  const { data, error } = await supabase
    .from('transactions')
    .select(
      `
      *,
      from_node:flow_nodes!from_node_id(id, name, type),
      to_node:flow_nodes!to_node_id(id, name, type)
    `,
    )
    .eq('id', id)
    .eq('organization_id', organizationId)
    .single()

  if (error) throw error
  return data as TransactionWithNodes
}

export async function createTransaction(
  transaction: Omit<
    Transaction,
    'id' | 'created_at' | 'updated_at' | 'verification_count'
  >,
) {
  const { data, error } = await supabase
    .from('transactions')
    .insert(transaction)
    .select()
    .single()

  if (error) throw error
  return data as Transaction
}

export async function updateTransaction(
  id: string,
  organizationId: string,
  updates: Partial<
    Omit<Transaction, 'id' | 'organization_id' | 'created_at' | 'updated_at'>
  >,
) {
  const { data, error } = await supabase
    .from('transactions')
    .update(updates)
    .eq('id', id)
    .eq('organization_id', organizationId)
    .select()
    .single()

  if (error) throw error
  return data as Transaction
}

// Flow node helpers
export async function getNodes(organizationId: string) {
  const { data, error } = await supabase
    .from('flow_nodes')
    .select('*')
    .eq('organization_id', organizationId)
    .order('name')

  if (error) throw error
  return data as Array<FlowNode>
}

export async function createNode(
  node: Omit<FlowNode, 'id' | 'created_at' | 'balance'>,
) {
  const { data, error } = await supabase
    .from('flow_nodes')
    .insert(node)
    .select()
    .single()

  if (error) throw error
  return data as FlowNode
}

// Verification helpers
export async function getVerifications(transactionId: string) {
  const { data, error } = await supabase
    .from('verifications')
    .select('*')
    .eq('transaction_id', transactionId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as Array<Verification>
}

export async function createVerification(
  verification: Omit<Verification, 'id' | 'created_at' | 'verified_at' | 'trust_points'>,
) {
  const { data, error } = await supabase
    .from('verifications')
    .insert(verification)
    .select()
    .single()

  if (error) throw error
  return data as Verification
}

// Get verifications with trust points for a transaction
export async function getVerificationsWithTrustPoints(transactionId: string) {
  const { data, error } = await supabase
    .from('verifications')
    .select('*')
    .eq('transaction_id', transactionId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as Array<Verification>
}

// Calculate total trust points for a transaction
export async function getTransactionTrustPoints(transactionId: string): Promise<number> {
  const { data, error } = await supabase
    .rpc('get_transaction_trust_points', { txn_id: transactionId })

  if (error) throw error
  return data as number
}

// Log verification access for audit
export async function logVerificationAccess(
  verificationId: string,
  accessedBy: string,
  accessType: 'view' | 'download',
  accessLevel: VerificationVisibility,
) {
  const { error } = await supabase
    .from('verification_access_log')
    .insert({
      verification_id: verificationId,
      accessed_by: accessedBy,
      access_type: accessType,
      access_level: accessLevel,
    })

  if (error) throw error
}

// Get verification visibility based on user's access group
export function getVerificationVisibility(
  verification: Verification,
  userPermissions: AccessGroupPermissions,
): VerificationVisibility {
  if (userPermissions.can_view_full_proofs) {
    return 'full'
  }
  if (userPermissions.can_view_proofs) {
    return 'thumbnail'
  }
  if (userPermissions.can_view_nodes) {
    return 'summary'
  }
  return 'hidden'
}

// Trust score history helpers
export async function getTrustScoreHistory(organizationId: string, limit = 12) {
  const { data, error } = await supabase
    .from('trust_score_history')
    .select('*')
    .eq('organization_id', organizationId)
    .order('calculated_at', { ascending: false })
    .limit(limit)

  if (error) throw error
  return (data as Array<TrustScoreHistory>).reverse() // Return in chronological order
}

// Trust score calculation
export function calculateTrustScore(
  verificationRate: number,
  documentationRate: number,
  consistencyScore: number,
): number {
  // Weights: verification 40%, documentation 30%, consistency 30%
  const score =
    verificationRate * 0.4 + documentationRate * 0.3 + consistencyScore * 0.3
  return Math.round(score)
}

export function getTrustGrade(score: number): string {
  if (score >= 90) return 'A+'
  if (score >= 80) return 'A'
  if (score >= 70) return 'B'
  if (score >= 60) return 'C'
  if (score >= 50) return 'D'
  return 'F'
}

// File upload helper
export async function uploadVerificationFile(
  organizationId: string,
  transactionId: string,
  file: File,
): Promise<string> {
  const fileExt = file.name.split('.').pop()
  const fileName = `${organizationId}/${transactionId}/${Date.now()}.${fileExt}`

  const { data, error } = await supabase.storage
    .from('verifications')
    .upload(fileName, file, {
      cacheControl: '3600',
      upsert: false,
    })

  if (error) throw error

  const {
    data: { publicUrl },
  } = supabase.storage.from('verifications').getPublicUrl(data.path)

  return publicUrl
}

export async function deleteVerificationFile(filePath: string): Promise<void> {
  const { error } = await supabase.storage
    .from('verifications')
    .remove([filePath])

  if (error) throw error
}

// ============================================================================
// ACCESS GROUP HELPERS
// ============================================================================

export async function getAccessGroups(organizationId: string) {
  const { data, error } = await supabase
    .from('access_groups')
    .select('*')
    .eq('organization_id', organizationId)
    .order('priority')

  if (error) throw error
  return data as Array<AccessGroup>
}

export async function getAccessGroup(id: string) {
  const { data, error } = await supabase
    .from('access_groups')
    .select('*')
    .eq('id', id)
    .single()

  if (error) throw error
  return data as AccessGroup
}

export async function createAccessGroup(
  group: Omit<AccessGroup, 'id' | 'created_at' | 'updated_at'>,
) {
  const { data, error } = await supabase
    .from('access_groups')
    .insert(group)
    .select()
    .single()

  if (error) throw error
  return data as AccessGroup
}

export async function updateAccessGroup(
  id: string,
  updates: Partial<Omit<AccessGroup, 'id' | 'organization_id' | 'created_at' | 'updated_at'>>,
) {
  const { data, error } = await supabase
    .from('access_groups')
    .update(updates)
    .eq('id', id)
    .select()
    .single()

  if (error) throw error
  return data as AccessGroup
}

export async function deleteAccessGroup(id: string) {
  const { error } = await supabase
    .from('access_groups')
    .delete()
    .eq('id', id)

  if (error) throw error
}

// ============================================================================
// GROUP MEMBERS HELPERS
// ============================================================================

export async function getGroupMembers(groupId: string) {
  const { data, error } = await supabase
    .from('group_members')
    .select(`
      *,
      profile:profiles(id, email, full_name)
    `)
    .eq('group_id', groupId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as Array<GroupMember & { profile: Pick<Profile, 'id' | 'email' | 'full_name'> | null }>
}

export async function getUserGroups(userId: string, organizationId: string) {
  const { data, error } = await supabase
    .from('group_members')
    .select(`
      *,
      group:access_groups(id, name, slug, color, icon, permissions, node_scope)
    `)
    .eq('user_id', userId)
    .lte('valid_from', new Date().toISOString())
    .or(`valid_until.is.null,valid_until.gt.${new Date().toISOString()}`)

  if (error) throw error

  // Filter by organization
  return (data as Array<GroupMemberWithDetails & { group: AccessGroup }>)
    .filter((m) => m.group.organization_id === organizationId)
}

export async function addGroupMember(
  member: Omit<GroupMember, 'id' | 'created_at'>,
) {
  const { data, error } = await supabase
    .from('group_members')
    .insert(member)
    .select()
    .single()

  if (error) throw error
  return data as GroupMember
}

export async function removeGroupMember(id: string) {
  const { error } = await supabase
    .from('group_members')
    .delete()
    .eq('id', id)

  if (error) throw error
}

export async function inviteMemberByEmail(
  groupId: string,
  email: string,
  grantedBy: string,
  validUntil?: string,
) {
  const { data, error } = await supabase
    .from('group_members')
    .insert({
      group_id: groupId,
      user_email: email,
      granted_by: grantedBy,
      valid_until: validUntil || null,
    })
    .select()
    .single()

  if (error) throw error
  return data as GroupMember
}

// ============================================================================
// NODE VISIBILITY HELPERS
// ============================================================================

export async function getNodeVisibility(nodeId: string) {
  const { data, error } = await supabase
    .from('node_visibility')
    .select(`
      *,
      group:access_groups(id, name, slug, color)
    `)
    .eq('node_id', nodeId)

  if (error) throw error
  return data as Array<NodeVisibility & { group: Pick<AccessGroup, 'id' | 'name' | 'slug' | 'color'> }>
}

export async function setNodeVisibility(
  nodeId: string,
  groupId: string,
  settings: Partial<Omit<NodeVisibility, 'id' | 'node_id' | 'group_id' | 'created_at'>>,
) {
  const { data, error } = await supabase
    .from('node_visibility')
    .upsert({
      node_id: nodeId,
      group_id: groupId,
      ...settings,
    }, { onConflict: 'node_id,group_id' })
    .select()
    .single()

  if (error) throw error
  return data as NodeVisibility
}

export async function removeNodeVisibility(nodeId: string, groupId: string) {
  const { error } = await supabase
    .from('node_visibility')
    .delete()
    .eq('node_id', nodeId)
    .eq('group_id', groupId)

  if (error) throw error
}

// ============================================================================
// PUBLIC EMBED CONFIG HELPERS
// ============================================================================

export async function getPublicEmbedConfig(organizationId: string) {
  const { data, error } = await supabase
    .from('public_embed_config')
    .select('*')
    .eq('organization_id', organizationId)
    .single()

  if (error && error.code !== 'PGRST116') throw error
  return data as PublicEmbedConfig | null
}

export async function updatePublicEmbedConfig(
  organizationId: string,
  config: Partial<Omit<PublicEmbedConfig, 'id' | 'organization_id' | 'embed_token' | 'created_at' | 'updated_at'>>,
) {
  const { data, error } = await supabase
    .from('public_embed_config')
    .update(config)
    .eq('organization_id', organizationId)
    .select()
    .single()

  if (error) throw error
  return data as PublicEmbedConfig
}

export async function regenerateEmbedToken(organizationId: string) {
  // This requires a server-side call to regenerate the token
  const { data, error } = await supabase
    .rpc('regenerate_embed_token', { org_id: organizationId })

  if (error) throw error
  return data as string
}

// ============================================================================
// VISIBILITY CONTEXT HELPERS
// ============================================================================

export async function getVisibilityContext(
  userId: string,
  organizationId: string,
): Promise<VisibilityContext> {
  const { data, error } = await supabase
    .rpc('get_visibility_context', {
      p_user_id: userId,
      p_org_id: organizationId,
    })

  if (error) throw error
  return data as VisibilityContext
}

export async function getUserPermissions(
  userId: string,
  organizationId: string,
): Promise<AccessGroupPermissions> {
  const { data, error } = await supabase
    .rpc('get_user_permissions', {
      p_user_id: userId,
      p_org_id: organizationId,
    })

  if (error) throw error
  return data as AccessGroupPermissions
}

// ======================
// Edge Function Types
// ======================

export interface TrustScoreResult {
  success: boolean
  trust_score: number
  previous_score: number
  delta: number
  details: {
    trust_score: number
    verification_rate: number
    documentation_rate: number
    consistency_score: number
    total_transactions: number
    verified_transactions: number
    documented_transactions: number
  }
}

export interface FlowDataNode {
  id: string
  name: string
  type: string
  inflow: number
  outflow: number
  balance: number
  // Verification stats for transactions through this node
  verification_count?: number
  verified_count?: number
  pending_count?: number
  rejected_count?: number
  trust_points?: number
}

export interface FlowDataEdge {
  source: string
  target: string
  value: number
  count: number
  currency: string
}

export interface FlowDataResult {
  nodes: FlowDataNode[]
  edges: FlowDataEdge[]
  summary: {
    total_nodes: number
    total_edges: number
    total_flow: number
    transaction_count: number
    currency: string
  }
}

export interface ProcessReceiptResult {
  success: boolean
  verification: {
    id: string
    file_url: string
    type: string
    status: string
  }
  ocr: {
    processed: boolean
    message: string
  }
  high_value_alert: boolean
}

// ======================
// Edge Function API Helpers
// ======================

const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL || ''

async function invokeEdgeFunction<T>(
  functionName: string,
  options: {
    method?: 'GET' | 'POST'
    body?: Record<string, unknown>
    params?: Record<string, string>
  } = {},
): Promise<T> {
  const { method = 'POST', body, params } = options

  let url = `${SUPABASE_URL}/functions/v1/${functionName}`
  if (params) {
    const searchParams = new URLSearchParams(params)
    url += `?${searchParams.toString()}`
  }

  const response = await fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${import.meta.env.VITE_SUPABASE_ANON_KEY}`,
    },
    body: body ? JSON.stringify(body) : undefined,
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Edge function failed')
  }

  return response.json()
}

// Calculate Trust Score via Edge Function
export async function calculateTrustScoreAPI(
  organizationId: string,
): Promise<TrustScoreResult> {
  return invokeEdgeFunction<TrustScoreResult>('calculate-trust-score', {
    method: 'POST',
    body: { organization_id: organizationId },
  })
}

// Get Flow Data via Edge Function
export async function getFlowDataAPI(
  organizationId: string,
  startDate?: string,
  endDate?: string,
): Promise<FlowDataResult> {
  const params: Record<string, string> = { organization_id: organizationId }
  if (startDate) params.start_date = startDate
  if (endDate) params.end_date = endDate

  return invokeEdgeFunction<FlowDataResult>('generate-flow-data', {
    method: 'GET',
    params,
  })
}

// Process Receipt OCR via Edge Function
export async function processReceiptOCR(
  organizationId: string,
  transactionId: string,
  file: File,
  verificationType: Verification['type'] = 'receipt',
): Promise<ProcessReceiptResult> {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('organization_id', organizationId)
  formData.append('transaction_id', transactionId)
  formData.append('type', verificationType)

  const response = await fetch(
    `${SUPABASE_URL}/functions/v1/process-receipt-ocr`,
    {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${import.meta.env.VITE_SUPABASE_ANON_KEY}`,
      },
      body: formData,
    },
  )

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'OCR processing failed')
  }

  return response.json()
}

// ============================================================================
// ORGANIZATION CHAINS TYPES
// ============================================================================

export type ChainStatus = 'pending' | 'active' | 'rejected' | 'revoked'

export interface OrganizationChain {
  id: string
  source_org_id: string
  target_org_id: string
  link_node_id: string | null
  status: ChainStatus
  granted_group_id: string | null
  request_message: string | null
  rejection_reason: string | null
  requested_by: string
  requested_at: string
  approved_by: string | null
  approved_at: string | null
  revoked_by: string | null
  revoked_at: string | null
  created_at: string
  updated_at: string
}

export interface OrganizationChainWithDetails extends OrganizationChain {
  source_org: Pick<Organization, 'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'>
  target_org: Pick<Organization, 'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'>
  link_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
  granted_group: Pick<AccessGroup, 'id' | 'name' | 'slug' | 'permissions'> | null
}

export type ChainActivityAction =
  | 'requested'
  | 'approved'
  | 'rejected'
  | 'revoked'
  | 'permissions_updated'
  | 'node_linked'

export interface ChainActivityLog {
  id: string
  chain_id: string
  action: ChainActivityAction
  actor_id: string
  actor_org_id: string
  details: Record<string, unknown>
  created_at: string
}

export interface ChainCounts {
  outgoing_pending: number
  outgoing_active: number
  incoming_pending: number
  incoming_active: number
}

export interface ChainTrustScore {
  aggregated_score: number
  org_count: number
  scores: {
    source_org: number
    target_org: number
  }
}

// Cross-org flow data for chain visualization
export interface ChainedFlowData {
  organizations: Array<{
    org: Pick<Organization, 'id' | 'name' | 'logo_url' | 'trust_score'>
    nodes: FlowDataNode[]
    edges: FlowDataEdge[]
  }>
  chain_links: Array<{
    chain_id: string
    source_org_id: string
    target_org_id: string
    link_node_id: string | null
  }>
  aggregated_trust_score: number
  total_flow: number
}

// ============================================================================
// ORGANIZATION CHAINS HELPERS
// ============================================================================

export async function getOutgoingChains(organizationId: string) {
  const { data, error } = await supabase
    .from('organization_chains')
    .select(`
      *,
      target_org:organizations!target_org_id(id, name, logo_url, trust_score, type, country),
      link_node:flow_nodes(id, name, type),
      granted_group:access_groups(id, name, slug, permissions)
    `)
    .eq('source_org_id', organizationId)
    .order('requested_at', { ascending: false })

  if (error) throw error
  return data as OrganizationChainWithDetails[]
}

export async function getIncomingChains(organizationId: string) {
  const { data, error } = await supabase
    .from('organization_chains')
    .select(`
      *,
      source_org:organizations!source_org_id(id, name, logo_url, trust_score, type, country),
      link_node:flow_nodes(id, name, type),
      granted_group:access_groups(id, name, slug, permissions)
    `)
    .eq('target_org_id', organizationId)
    .order('requested_at', { ascending: false })

  if (error) throw error
  return data as OrganizationChainWithDetails[]
}

export async function getChain(chainId: string) {
  const { data, error } = await supabase
    .from('organization_chains')
    .select(`
      *,
      source_org:organizations!source_org_id(id, name, logo_url, trust_score, type, country),
      target_org:organizations!target_org_id(id, name, logo_url, trust_score, type, country),
      link_node:flow_nodes(id, name, type),
      granted_group:access_groups(id, name, slug, permissions)
    `)
    .eq('id', chainId)
    .single()

  if (error) throw error
  return data as OrganizationChainWithDetails
}

export async function requestChain(
  sourceOrgId: string,
  targetOrgId: string,
  requestedBy: string,
  message?: string,
) {
  const { data, error } = await supabase
    .from('organization_chains')
    .insert({
      source_org_id: sourceOrgId,
      target_org_id: targetOrgId,
      requested_by: requestedBy,
      request_message: message || null,
    })
    .select()
    .single()

  if (error) throw error
  return data as OrganizationChain
}

export async function approveChain(
  chainId: string,
  approvedBy: string,
  grantedGroupId: string,
  linkNodeId?: string,
) {
  const { data, error } = await supabase
    .from('organization_chains')
    .update({
      status: 'active',
      approved_by: approvedBy,
      approved_at: new Date().toISOString(),
      granted_group_id: grantedGroupId,
      link_node_id: linkNodeId || null,
    })
    .eq('id', chainId)
    .eq('status', 'pending')
    .select()
    .single()

  if (error) throw error
  return data as OrganizationChain
}

export async function rejectChain(
  chainId: string,
  rejectedBy: string,
  reason?: string,
) {
  const { data, error } = await supabase
    .from('organization_chains')
    .update({
      status: 'rejected',
      approved_by: rejectedBy,
      approved_at: new Date().toISOString(),
      rejection_reason: reason || null,
    })
    .eq('id', chainId)
    .eq('status', 'pending')
    .select()
    .single()

  if (error) throw error
  return data as OrganizationChain
}

export async function revokeChain(chainId: string, revokedBy: string) {
  const { data, error } = await supabase
    .from('organization_chains')
    .update({
      status: 'revoked',
      revoked_by: revokedBy,
      revoked_at: new Date().toISOString(),
    })
    .eq('id', chainId)
    .eq('status', 'active')
    .select()
    .single()

  if (error) throw error
  return data as OrganizationChain
}

export async function cancelChainRequest(chainId: string) {
  const { error } = await supabase
    .from('organization_chains')
    .delete()
    .eq('id', chainId)
    .eq('status', 'pending')

  if (error) throw error
}

export async function updateChainPermissions(
  chainId: string,
  grantedGroupId: string,
  linkNodeId?: string,
) {
  const { data, error } = await supabase
    .from('organization_chains')
    .update({
      granted_group_id: grantedGroupId,
      link_node_id: linkNodeId || null,
    })
    .eq('id', chainId)
    .eq('status', 'active')
    .select()
    .single()

  if (error) throw error
  return data as OrganizationChain
}

export async function getChainActivityLog(chainId: string) {
  const { data, error } = await supabase
    .from('chain_activity_log')
    .select('*')
    .eq('chain_id', chainId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as ChainActivityLog[]
}

export async function getChainCounts(organizationId: string): Promise<ChainCounts> {
  const { data, error } = await supabase.rpc('get_org_chain_counts', {
    p_org_id: organizationId,
  })

  if (error) throw error
  return data as ChainCounts
}

export async function getChainTrustScore(chainId: string): Promise<ChainTrustScore | null> {
  const { data, error } = await supabase.rpc('get_chain_trust_score', {
    p_chain_id: chainId,
  })

  if (error) throw error
  return data?.[0] as ChainTrustScore | null
}

export async function searchOrganizations(query: string, excludeOrgId: string) {
  const { data, error } = await supabase
    .from('organizations')
    .select('id, name, logo_url, trust_score, type, country')
    .neq('id', excludeOrgId)
    .ilike('name', `%${query}%`)
    .limit(10)

  if (error) throw error
  return data as Pick<Organization, 'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'>[]
}

// ============================================================================
// CAMPAIGNS & DONATIONS TYPES
// ============================================================================

export type CampaignStatus = 'draft' | 'active' | 'paused' | 'completed' | 'cancelled'
export type PaymentStatus = 'pending' | 'confirmed' | 'failed' | 'refunded'
export type DonorUpdateType =
  | 'donation_received'
  | 'funds_allocated'
  | 'milestone_reached'
  | 'project_update'
  | 'impact_report'
  | 'campaign_complete'
  | 'thank_you'
  | 'custom'
export type DeliveryMethod = 'email' | 'sms' | 'push' | 'in_app'

export interface Campaign {
  id: string
  organization_id: string
  name: string
  description: string | null
  short_description: string | null
  image_url: string | null
  target_amount: number
  current_amount: number
  currency: string
  target_node_id: string | null
  status: CampaignStatus
  allow_overflow: boolean
  overflow_campaign_id: string | null
  overflow_consent_message: string | null
  starts_at: string
  ends_at: string | null
  is_public: boolean
  public_slug: string | null
  category: string | null
  tags: string[] | null
  metadata: Record<string, unknown>
  created_by: string | null
  created_at: string
  updated_at: string
}

export interface CampaignStats {
  donation_count: number
  unique_donors: number
  total_raised: number
  funding_percentage: number
  avg_donation: number
  largest_donation: number
  smallest_donation: number
  days_remaining: number | null
  days_active: number
}

export interface CampaignWithDetails extends Campaign {
  target_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
  stats?: CampaignStats
}

export interface CampaignCounts {
  total_campaigns: number
  active_campaigns: number
  completed_campaigns: number
  total_raised: number
  total_donations: number
}

export interface Donation {
  id: string
  campaign_id: string
  donor_email: string | null
  donor_name: string | null
  donor_phone: string | null
  is_anonymous: boolean
  amount: number
  currency: string
  amount_usd: number | null
  exchange_rate: number | null
  earmarked_node_id: string | null
  earmark_notes: string | null
  allow_reallocation: boolean
  flow_node_id: string | null
  impact_percentage: number | null
  payment_method: string | null
  payment_reference: string | null
  payment_status: PaymentStatus
  transaction_id: string | null
  source: string | null
  utm_source: string | null
  utm_medium: string | null
  utm_campaign: string | null
  metadata: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface DonationWithDetails extends Donation {
  campaign: Pick<Campaign, 'id' | 'name' | 'public_slug' | 'status'>
  earmarked_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
}

export interface DonorUpdate {
  id: string
  donation_id: string
  campaign_id: string | null
  update_type: DonorUpdateType
  title: string
  message: string
  image_url: string | null
  related_node_id: string | null
  related_transaction_id: string | null
  sent_at: string | null
  delivery_method: DeliveryMethod
  metadata: Record<string, unknown>
  created_at: string
}

export interface DonorUpdateWithDetails extends DonorUpdate {
  donation: Pick<Donation, 'id' | 'donor_email' | 'donor_name' | 'amount'>
  campaign: Pick<Campaign, 'id' | 'name'> | null
  related_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
}

// ============================================================================
// CAMPAIGNS HELPERS
// ============================================================================

export async function getCampaigns(organizationId: string) {
  const { data, error } = await supabase
    .from('campaigns')
    .select(`
      *,
      target_node:flow_nodes(id, name, type)
    `)
    .eq('organization_id', organizationId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as CampaignWithDetails[]
}

export async function getCampaign(id: string, organizationId: string) {
  const { data, error } = await supabase
    .from('campaigns')
    .select(`
      *,
      target_node:flow_nodes(id, name, type)
    `)
    .eq('id', id)
    .eq('organization_id', organizationId)
    .single()

  if (error) throw error
  return data as CampaignWithDetails
}

export async function getCampaignBySlug(slug: string) {
  const { data, error } = await supabase
    .from('campaigns')
    .select(`
      *,
      target_node:flow_nodes(id, name, type)
    `)
    .eq('public_slug', slug)
    .eq('is_public', true)
    .eq('status', 'active')
    .single()

  if (error) throw error
  return data as CampaignWithDetails
}

export async function createCampaign(
  campaign: Omit<Campaign, 'id' | 'current_amount' | 'created_at' | 'updated_at'>,
) {
  const { data, error } = await supabase
    .from('campaigns')
    .insert(campaign)
    .select()
    .single()

  if (error) throw error
  return data as Campaign
}

export async function updateCampaign(
  id: string,
  organizationId: string,
  updates: Partial<Omit<Campaign, 'id' | 'organization_id' | 'created_at' | 'updated_at'>>,
) {
  const { data, error } = await supabase
    .from('campaigns')
    .update(updates)
    .eq('id', id)
    .eq('organization_id', organizationId)
    .select()
    .single()

  if (error) throw error
  return data as Campaign
}

export async function deleteCampaign(id: string, organizationId: string) {
  const { error } = await supabase
    .from('campaigns')
    .delete()
    .eq('id', id)
    .eq('organization_id', organizationId)
    .eq('status', 'draft')

  if (error) throw error
}

export async function getCampaignStats(campaignId: string): Promise<CampaignStats> {
  const { data, error } = await supabase.rpc('get_campaign_stats', {
    p_campaign_id: campaignId,
  })

  if (error) throw error
  return data?.[0] as CampaignStats
}

export async function getCampaignCounts(organizationId: string): Promise<CampaignCounts> {
  const { data, error } = await supabase.rpc('get_org_campaign_counts', {
    p_org_id: organizationId,
  })

  if (error) throw error
  return data?.[0] as CampaignCounts
}

export async function getUnderfundedCampaigns(organizationId: string, limit = 5) {
  const { data, error } = await supabase.rpc('get_underfunded_campaigns', {
    p_org_id: organizationId,
    p_limit: limit,
  })

  if (error) throw error
  return data as Campaign[]
}

export async function generateCampaignSlug(name: string, organizationId: string): Promise<string> {
  const { data, error } = await supabase.rpc('generate_campaign_slug', {
    p_name: name,
    p_org_id: organizationId,
  })

  if (error) throw error
  return data as string
}

// ============================================================================
// DONATIONS HELPERS
// ============================================================================

export async function getDonations(campaignId: string) {
  const { data, error } = await supabase
    .from('donations')
    .select(`
      *,
      campaign:campaigns(id, name, public_slug, status),
      earmarked_node:flow_nodes(id, name, type)
    `)
    .eq('campaign_id', campaignId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as DonationWithDetails[]
}

export async function getDonation(id: string) {
  const { data, error } = await supabase
    .from('donations')
    .select(`
      *,
      campaign:campaigns(id, name, public_slug, status),
      earmarked_node:flow_nodes(id, name, type)
    `)
    .eq('id', id)
    .single()

  if (error) throw error
  return data as DonationWithDetails
}

export async function createDonation(
  donation: Omit<Donation, 'id' | 'impact_percentage' | 'created_at' | 'updated_at'>,
) {
  const { data, error } = await supabase
    .from('donations')
    .insert(donation)
    .select()
    .single()

  if (error) throw error
  return data as Donation
}

export async function updateDonationStatus(
  id: string,
  status: PaymentStatus,
  paymentReference?: string,
) {
  const { data, error } = await supabase
    .from('donations')
    .update({
      payment_status: status,
      payment_reference: paymentReference || null,
    })
    .eq('id', id)
    .select()
    .single()

  if (error) throw error
  return data as Donation
}

export async function getDonorDonations(donorEmail: string) {
  const { data, error } = await supabase
    .from('donations')
    .select(`
      *,
      campaign:campaigns(id, name, public_slug, status)
    `)
    .eq('donor_email', donorEmail)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as DonationWithDetails[]
}

export async function calculateDonationImpact(donationId: string): Promise<number> {
  const { data, error } = await supabase.rpc('calculate_donation_impact', {
    p_donation_id: donationId,
  })

  if (error) throw error
  return data as number
}

// ============================================================================
// DONOR UPDATES HELPERS
// ============================================================================

export async function getDonorUpdates(donationId: string) {
  const { data, error } = await supabase
    .from('donor_updates')
    .select(`
      *,
      donation:donations(id, donor_email, donor_name, amount),
      campaign:campaigns(id, name),
      related_node:flow_nodes(id, name, type)
    `)
    .eq('donation_id', donationId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as DonorUpdateWithDetails[]
}

export async function getCampaignUpdates(campaignId: string) {
  const { data, error } = await supabase
    .from('donor_updates')
    .select(`
      *,
      donation:donations(id, donor_email, donor_name, amount),
      campaign:campaigns(id, name),
      related_node:flow_nodes(id, name, type)
    `)
    .eq('campaign_id', campaignId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as DonorUpdateWithDetails[]
}

export async function createDonorUpdate(
  update: Omit<DonorUpdate, 'id' | 'sent_at' | 'created_at'>,
) {
  const { data, error } = await supabase
    .from('donor_updates')
    .insert(update)
    .select()
    .single()

  if (error) throw error
  return data as DonorUpdate
}

export async function markUpdateSent(id: string) {
  const { data, error } = await supabase
    .from('donor_updates')
    .update({ sent_at: new Date().toISOString() })
    .eq('id', id)
    .select()
    .single()

  if (error) throw error
  return data as DonorUpdate
}

// Create donor flow node helper
export async function createDonorFlowNode(
  organizationId: string,
  donorName: string | null,
  donorEmail: string | null,
): Promise<string> {
  const { data, error } = await supabase.rpc('create_donor_node', {
    p_org_id: organizationId,
    p_donor_name: donorName,
    p_donor_email: donorEmail,
  })

  if (error) throw error
  return data as string
}
