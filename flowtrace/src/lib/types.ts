// The app's record types (the shapes the Supabase schema had, which the demo API and the
// SpacetimeDB module's transparency tables keep) and the pure helpers on them.

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
  /** Shown on public pages (the module's `public_organization` and `public_badge`). */
  is_public?: boolean
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
  /** Shown to anyone at the proofs Worker's public path (STO1); module only. */
  published?: boolean
  /** How much of the proof this reader was given; set by the module's `my_verification`. */
  visible_as?: VerificationVisibility
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

/** One part of the Trust Score: its value out of 100 and its weight in percent. */
export interface TrustComponent {
  key: 'verification' | 'documentation' | 'consistency' | 'model'
  value: number
  weight: number
}

/** The organisation's current Trust Score and how it is made. */
export interface TrustScoreSummary {
  score: number
  grade: string
  components: Array<TrustComponent>
  computed_at: string | null
}

/** A member of the organisation, as an access group member can be picked. */
export interface OrgMember {
  user_id: string
  name: string
}

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
  visible_node_types: Array<string>
  hidden_node_ids: Array<string>
  theme: {
    primary_color: string
    background: 'light' | 'dark'
    border_radius: 'none' | 'sm' | 'md' | 'lg' | 'full'
    show_logo: boolean
  }
  allow_embed: boolean
  allowed_domains: Array<string>
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

// Get verification visibility based on user's access group
export function getVerificationVisibility(
  _verification: Verification,
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
  nodes: Array<FlowDataNode>
  edges: Array<FlowDataEdge>
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
  source_org: Pick<
    Organization,
    'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'
  >
  target_org: Pick<
    Organization,
    'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'
  >
  link_node: Pick<FlowNode, 'id' | 'name' | 'type'> | null
  granted_group: Pick<
    AccessGroup,
    'id' | 'name' | 'slug' | 'permissions'
  > | null
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
    nodes: Array<FlowDataNode>
    edges: Array<FlowDataEdge>
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

export type CampaignStatus =
  | 'draft'
  | 'active'
  | 'paused'
  | 'completed'
  | 'cancelled'

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
  tags: Array<string> | null
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
