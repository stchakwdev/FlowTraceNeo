import { createClient } from '@supabase/supabase-js'
import type { SupabaseClient } from '@supabase/supabase-js'
import type {
  AccessGroup,
  AccessGroupPermissions,
  GroupMember,
  GroupMemberWithDetails,
  Organization,
  ProcessReceiptResult,
  Profile,
  PublicEmbedConfig,
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

// Transactions, flow nodes, node visibility, the flow data, the organisation stats and the
// organisation chains are served by the SpacetimeDB module (ledger/spacetime-transparency.ts)
// since RW2.

// ============================================================================
// CAMPAIGNS & DONATIONS TYPES
// ============================================================================

// Campaigns, donations, donor updates and the organisation profile are served by the
// SpacetimeDB module (ledger/spacetime-transparency.ts) since RW1.
