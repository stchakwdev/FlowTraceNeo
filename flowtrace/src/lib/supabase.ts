import { createClient } from '@supabase/supabase-js'
import type { SupabaseClient } from '@supabase/supabase-js'
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
  VerificationVisibility,
  VisibilityContext,
} from './types'

export * from './types'

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
    return (_supabase as unknown as Record<string, unknown>)[prop as string]
  },
})

// Development mode constants
export { DEV_MODE } from './auth'
export const DEV_ORG_ID = '550e8400-e29b-41d4-a716-446655440000'

// ============================================================================
// ACCESS GROUPS & VISIBILITY TYPES
// ============================================================================

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
  return data as Array<OrganizationChainWithDetails>
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
  return data as Array<OrganizationChainWithDetails>
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
  return data as Array<ChainActivityLog>
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
  return data as Array<Pick<Organization, 'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'>>
}

// ============================================================================
// CAMPAIGNS & DONATIONS TYPES
// ============================================================================

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
  return data as Array<CampaignWithDetails>
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
  return data as Array<Campaign>
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
  return data as Array<DonationWithDetails>
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
  return data as Array<DonationWithDetails>
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
  return data as Array<DonorUpdateWithDetails>
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
  return data as Array<DonorUpdateWithDetails>
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
