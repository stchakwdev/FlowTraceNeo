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

export interface Verification {
  id: string
  transaction_id: string
  type:
    | 'receipt'
    | 'photo'
    | 'gps'
    | 'signature'
    | 'biometric'
    | 'bank_statement'
    | 'document'
  file_url: string | null
  ocr_data: Record<string, unknown> | null
  metadata: Record<string, unknown>
  status: 'pending' | 'verified' | 'rejected'
  verified_by: string | null
  verified_at: string | null
  created_at: string
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

export interface OrgStats {
  total: number
  verified: number
  pending: number
  flagged: number
  verificationRate: number
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
  const { data: transactions, error } = await supabase
    .from('transactions')
    .select('verification_status')
    .eq('organization_id', organizationId)

  if (error) throw error

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

  return { total, verified, pending, flagged, verificationRate }
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
  verification: Omit<Verification, 'id' | 'created_at' | 'verified_at'>,
) {
  const { data, error } = await supabase
    .from('verifications')
    .insert(verification)
    .select()
    .single()

  if (error) throw error
  return data as Verification
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
