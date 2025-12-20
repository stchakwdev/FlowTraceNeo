import { createClient } from '@supabase/supabase-js'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL || ''
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY || ''

export const supabase = createClient(supabaseUrl, supabaseAnonKey)

// Database types (extend as schema grows)
export interface Transaction {
  id: string
  organization_id: string
  amount: number
  currency: string
  description: string
  category: string
  from_node_id: string
  to_node_id: string
  date: string
  status: 'pending' | 'verified' | 'flagged'
  verification_count: number
  created_at: string
  updated_at: string
}

export interface Node {
  id: string
  organization_id: string
  name: string
  type: 'donor' | 'department' | 'vendor' | 'beneficiary' | 'account'
  metadata?: Record<string, unknown>
  created_at: string
}

export interface Verification {
  id: string
  transaction_id: string
  type: 'receipt' | 'photo' | 'gps' | 'signature' | 'document'
  file_url?: string
  metadata?: Record<string, unknown>
  verified_by?: string
  created_at: string
}

export interface Organization {
  id: string
  name: string
  slug: string
  trust_score: number
  created_at: string
}

// Query helpers
export async function getTransactions(organizationId: string) {
  const { data, error } = await supabase
    .from('transactions')
    .select('*')
    .eq('organization_id', organizationId)
    .order('date', { ascending: false })

  if (error) throw error
  return data as Transaction[]
}

export async function getTransaction(id: string) {
  const { data, error } = await supabase
    .from('transactions')
    .select('*')
    .eq('id', id)
    .single()

  if (error) throw error
  return data as Transaction
}

export async function createTransaction(
  transaction: Omit<Transaction, 'id' | 'created_at' | 'updated_at'>
) {
  const { data, error } = await supabase
    .from('transactions')
    .insert(transaction)
    .select()
    .single()

  if (error) throw error
  return data as Transaction
}

export async function getNodes(organizationId: string) {
  const { data, error } = await supabase
    .from('nodes')
    .select('*')
    .eq('organization_id', organizationId)
    .order('name')

  if (error) throw error
  return data as Node[]
}

export async function getVerifications(transactionId: string) {
  const { data, error } = await supabase
    .from('verifications')
    .select('*')
    .eq('transaction_id', transactionId)
    .order('created_at', { ascending: false })

  if (error) throw error
  return data as Verification[]
}

export async function getOrganization(id: string) {
  const { data, error } = await supabase
    .from('organizations')
    .select('*')
    .eq('id', id)
    .single()

  if (error) throw error
  return data as Organization
}

// Trust score calculation
export function calculateTrustScore(
  verificationRate: number,
  documentationRate: number,
  consistencyScore: number
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
