import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useOrg } from './org-context'
import {
  addGroupMember,
  approveChain,
  calculateTrustScoreAPI,
  cancelChainRequest,
  createAccessGroup,
  createCampaign,
  createDonation,
  createDonorUpdate,
  createNode,
  createTransaction,
  createVerification,
  deleteAccessGroup,
  deleteCampaign,
  getAccessGroup,
  getAccessGroups,
  getCampaign,
  getCampaignBySlug,
  getCampaignCounts,
  getCampaigns,
  getCampaignStats,
  getCampaignUpdates,
  getChain,
  getChainActivityLog,
  getChainAccessGroups,
  getChainCounts,
  getChainTrustScore,
  getDonation,
  getDonations,
  getDonorDonations,
  getDonorUpdates,
  getFlowDataAPI,
  getGroupMembers,
  getIncomingChains,
  getNodes,
  getNodeVisibility,
  getOrganizationStats,
  getOutgoingChains,
  getPublicCampaignStats,
  getPublicEmbedConfig,
  getPublicTrustScore,
  getTransaction,
  getTransactions,
  getTrustScoreHistory,
  getUnderfundedCampaigns,
  getUserGroups,
  getUserPermissions,
  getVerifications,
  getVisibilityContext,
  inviteMemberByEmail,
  markUpdateSent,
  processReceiptOCR,
  rejectChain,
  removeGroupMember,
  removeNodeVisibility,
  requestChain,
  revokeChain,
  searchOrganizations,
  setNodeVisibility,
  updateAccessGroup,
  updateCampaign,
  updateChainPermissions,
  updateDonationStatus,
  updatePublicEmbedConfig,
  updateTransaction,
  uploadVerificationFile,
} from './data'
import type {
  AccessGroup,
  AccessGroupPermissions,
  Campaign,
  CampaignCounts,
  CampaignStats,
  CampaignStatus,
  CampaignWithDetails,
  ChainActivityLog,
  ChainCounts,
  ChainStatus,
  ChainTrustScore,
  DeliveryMethod,
  Donation,
  DonationWithDetails,
  DonorUpdate,
  DonorUpdateType,
  DonorUpdateWithDetails,
  FlowDataResult,
  FlowNode,
  GroupMember,
  NodeVisibility,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
  PaymentStatus,
  PublicEmbedConfig,
  Transaction,
  TrustScoreResult,
  Verification,
  VerificationType,
  VerificationVisibility,
  VisibilityContext,
} from './supabase'

// Query key factory for consistent cache management
export const queryKeys = {
  transactions: (orgId: string) => ['transactions', orgId] as const,
  transaction: (id: string, orgId: string) =>
    ['transaction', id, orgId] as const,
  nodes: (orgId: string) => ['nodes', orgId] as const,
  orgStats: (orgId: string) => ['org-stats', orgId] as const,
  verifications: (txnId: string) => ['verifications', txnId] as const,
  trustHistory: (orgId: string) => ['trust-history', orgId] as const,
  flowData: (orgId: string, startDate?: string, endDate?: string) =>
    ['flow-data', orgId, startDate, endDate] as const,
  // Access Groups & Visibility
  accessGroups: (orgId: string) => ['access-groups', orgId] as const,
  accessGroup: (id: string) => ['access-group', id] as const,
  groupMembers: (groupId: string) => ['group-members', groupId] as const,
  userGroups: (userId: string, orgId: string) =>
    ['user-groups', userId, orgId] as const,
  userPermissions: (userId: string, orgId: string) =>
    ['user-permissions', userId, orgId] as const,
  nodeVisibility: (nodeId: string) => ['node-visibility', nodeId] as const,
  visibilityContext: (userId: string, orgId: string) =>
    ['visibility-context', userId, orgId] as const,
  publicEmbedConfig: (orgId: string) => ['embed-config', orgId] as const,
  // Organization Chains
  outgoingChains: (orgId: string) => ['chains', 'outgoing', orgId] as const,
  incomingChains: (orgId: string) => ['chains', 'incoming', orgId] as const,
  chain: (chainId: string) => ['chain', chainId] as const,
  chainCounts: (orgId: string) => ['chain-counts', orgId] as const,
  chainActivityLog: (chainId: string) => ['chain-activity', chainId] as const,
  chainTrustScore: (chainId: string) => ['chain-trust-score', chainId] as const,
  chainAccessGroups: (orgId: string) => ['chain-access-groups', orgId] as const,
  orgSearch: (query: string, excludeOrgId: string) =>
    ['org-search', query, excludeOrgId] as const,
  // Campaigns & Donations
  campaigns: (orgId: string) => ['campaigns', orgId] as const,
  campaign: (id: string, orgId: string) => ['campaign', id, orgId] as const,
  campaignBySlug: (slug: string) => ['campaign-slug', slug] as const,
  campaignStats: (campaignId: string) => ['campaign-stats', campaignId] as const,
  campaignCounts: (orgId: string) => ['campaign-counts', orgId] as const,
  underfundedCampaigns: (orgId: string) => ['underfunded-campaigns', orgId] as const,
  donations: (campaignId: string) => ['donations', campaignId] as const,
  donation: (id: string) => ['donation', id] as const,
  donorDonations: (email: string) => ['donor-donations', email] as const,
  donorUpdates: (donationId: string) => ['donor-updates', donationId] as const,
  campaignUpdates: (campaignId: string) => ['campaign-updates', campaignId] as const,
}

// Query options (for use with suspense or prefetching)
export function transactionsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.transactions(orgId),
    queryFn: () => getTransactions(orgId),
    staleTime: 30 * 1000, // 30 seconds
    enabled: !!orgId,
  })
}

export function transactionQueryOptions(id: string, orgId: string) {
  return queryOptions({
    queryKey: queryKeys.transaction(id, orgId),
    queryFn: () => getTransaction(id, orgId),
    staleTime: 30 * 1000,
    enabled: !!id && !!orgId,
  })
}

export function nodesQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.nodes(orgId),
    queryFn: () => getNodes(orgId),
    staleTime: 5 * 60 * 1000, // 5 minutes (nodes change less frequently)
    enabled: !!orgId,
  })
}

export function orgStatsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.orgStats(orgId),
    queryFn: () => getOrganizationStats(orgId),
    staleTime: 60 * 1000, // 1 minute
    enabled: !!orgId,
  })
}

export function trustHistoryQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.trustHistory(orgId),
    queryFn: () => getTrustScoreHistory(orgId),
    staleTime: 5 * 60 * 1000, // 5 minutes
    enabled: !!orgId,
  })
}

// Hooks using organization context

export function useTransactions() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.transactions(organizationId || ''),
    queryFn: () => getTransactions(organizationId!),
    enabled: !!organizationId,
    staleTime: 30 * 1000,
  })
}

export function useTransaction(id: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.transaction(id, organizationId || ''),
    queryFn: () => getTransaction(id, organizationId!),
    enabled: !!organizationId && !!id,
    staleTime: 30 * 1000,
  })
}

export function useNodes() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.nodes(organizationId || ''),
    queryFn: () => getNodes(organizationId!),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

export function useOrgStats() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.orgStats(organizationId || ''),
    queryFn: () => getOrganizationStats(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useVerifications(transactionId: string) {
  return useQuery({
    queryKey: queryKeys.verifications(transactionId),
    queryFn: () => getVerifications(transactionId),
    enabled: !!transactionId,
    staleTime: 30 * 1000,
  })
}

export function useTrustHistory() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.trustHistory(organizationId || ''),
    queryFn: () => getTrustScoreHistory(organizationId!),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

// Trust score for organization (public or by org ID)
export function useTrustScore(organizationId: string) {
  return useQuery({
    queryKey: ['trust-score', organizationId],
    queryFn: async () => {
      const history = await getTrustScoreHistory(organizationId, 1)
      if (history && history.length > 0) {
        return {
          overall_score: history[0].score,
          breakdown: history[0].breakdown,
          verification_rate: history[0].verification_rate,
          calculated_at: history[0].calculated_at,
        }
      }
      // Return default if no history
      return {
        overall_score: 0,
        breakdown: {},
        verification_rate: null,
        calculated_at: null,
      }
    },
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

// Mutations

export function useCreateTransaction() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<
        Transaction,
        | 'id'
        | 'created_at'
        | 'updated_at'
        | 'verification_count'
        | 'organization_id'
      >,
    ) => createTransaction({ ...data, organization_id: organizationId! }),
    onSuccess: () => {
      // Invalidate related queries
      queryClient.invalidateQueries({
        queryKey: queryKeys.transactions(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.orgStats(organizationId!),
      })
    },
  })
}

export function useUpdateTransaction() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      id,
      updates,
    }: {
      id: string
      updates: Partial<
        Omit<
          Transaction,
          'id' | 'organization_id' | 'created_at' | 'updated_at'
        >
      >
    }) => updateTransaction(id, organizationId!, updates),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.transaction(variables.id, organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.transactions(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.orgStats(organizationId!),
      })
    },
  })
}

export function useCreateNode() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<FlowNode, 'id' | 'created_at' | 'balance' | 'organization_id'>,
    ) => createNode({ ...data, organization_id: organizationId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.nodes(organizationId!),
      })
    },
  })
}

// Enhanced verification input for Phase 3
export interface CreateVerificationInput {
  transactionId: string
  type: VerificationType
  fileUrl?: string
  // New Phase 3 fields
  visibilityLevel?: VerificationVisibility
  executorName?: string
  description?: string
  gpsLatitude?: number
  gpsLongitude?: number
  gpsAccuracy?: number
  locationName?: string
  createdBy?: string
}

export function useCreateVerification() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      transactionId,
      type,
      fileUrl,
      visibilityLevel,
      executorName,
      description,
      gpsLatitude,
      gpsLongitude,
      gpsAccuracy,
      locationName,
      createdBy,
    }: CreateVerificationInput) =>
      createVerification({
        transaction_id: transactionId,
        type,
        file_url: fileUrl || null,
        ocr_data: null,
        metadata: {},
        status: 'pending',
        verified_by: null,
        // New Phase 3 fields
        visibility_level: visibilityLevel || 'full',
        thumbnail_url: null,
        executor_name: executorName || null,
        description: description || null,
        gps_latitude: gpsLatitude || null,
        gps_longitude: gpsLongitude || null,
        gps_accuracy: gpsAccuracy || null,
        location_name: locationName || null,
        created_by: createdBy || null,
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.verifications(variables.transactionId),
      })
      // Also invalidate transaction to update verification count
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === 'transaction' &&
          query.queryKey[1] === variables.transactionId,
      })
    },
  })
}

// Enhanced upload verification input for Phase 3
export interface UploadVerificationInput {
  transactionId: string
  file: File
  type: VerificationType
  // New Phase 3 fields
  visibilityLevel?: VerificationVisibility
  executorName?: string
  description?: string
  gpsLatitude?: number
  gpsLongitude?: number
  gpsAccuracy?: number
  locationName?: string
  createdBy?: string
}

export function useUploadVerification() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      transactionId,
      file,
      type,
      visibilityLevel,
      executorName,
      description,
      gpsLatitude,
      gpsLongitude,
      gpsAccuracy,
      locationName,
      createdBy,
    }: UploadVerificationInput) => {
      // Upload file to storage
      const fileUrl = await uploadVerificationFile(
        organizationId!,
        transactionId,
        file,
      )

      // Create verification record with Phase 3 fields
      return createVerification({
        transaction_id: transactionId,
        type,
        file_url: fileUrl,
        ocr_data: null,
        metadata: { originalName: file.name, size: file.size },
        status: 'pending',
        verified_by: null,
        // New Phase 3 fields
        visibility_level: visibilityLevel || 'full',
        thumbnail_url: null,
        executor_name: executorName || null,
        description: description || null,
        gps_latitude: gpsLatitude || null,
        gps_longitude: gpsLongitude || null,
        gps_accuracy: gpsAccuracy || null,
        location_name: locationName || null,
        created_by: createdBy || null,
      })
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.verifications(variables.transactionId),
      })
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === 'transaction' &&
          query.queryKey[1] === variables.transactionId,
      })
    },
  })
}

// ======================
// Edge Function Hooks
// ======================

// Flow Data from Edge Function
export function useFlowData(startDate?: string, endDate?: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.flowData(organizationId || '', startDate, endDate),
    queryFn: () => getFlowDataAPI(organizationId!, startDate, endDate),
    enabled: !!organizationId,
    staleTime: 60 * 1000, // 1 minute
  })
}

// Recalculate Trust Score
export function useRecalculateTrustScore() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: () => calculateTrustScoreAPI(organizationId!),
    onSuccess: () => {
      // Invalidate related queries to refresh data
      queryClient.invalidateQueries({
        queryKey: queryKeys.trustHistory(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.orgStats(organizationId!),
      })
      // Also refetch org context
      queryClient.invalidateQueries({
        predicate: (query) => query.queryKey[0] === 'organization',
      })
    },
  })
}

// Process Receipt with OCR
export function useProcessReceiptOCR() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      transactionId,
      file,
      type,
    }: {
      transactionId: string
      file: File
      type: Verification['type']
    }) => processReceiptOCR(organizationId!, transactionId, file, type),
    onSuccess: (_, variables) => {
      // Invalidate verification queries
      queryClient.invalidateQueries({
        queryKey: queryKeys.verifications(variables.transactionId),
      })
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === 'transaction' &&
          query.queryKey[1] === variables.transactionId,
      })
    },
  })
}

// ======================
// Access Groups & Visibility Hooks
// ======================

// Access Groups Query Options
export function accessGroupsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.accessGroups(orgId),
    queryFn: () => getAccessGroups(orgId),
    staleTime: 5 * 60 * 1000, // 5 minutes
    enabled: !!orgId,
  })
}

export function accessGroupQueryOptions(id: string) {
  return queryOptions({
    queryKey: queryKeys.accessGroup(id),
    queryFn: () => getAccessGroup(id),
    staleTime: 5 * 60 * 1000,
    enabled: !!id,
  })
}

// Access Groups Hooks
export function useAccessGroups() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.accessGroups(organizationId || ''),
    queryFn: () => getAccessGroups(organizationId!),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

export function useAccessGroup(id: string) {
  return useQuery({
    queryKey: queryKeys.accessGroup(id),
    queryFn: () => getAccessGroup(id),
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  })
}

export function useCreateAccessGroup() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<AccessGroup, 'id' | 'created_at' | 'updated_at' | 'organization_id'>,
    ) => createAccessGroup({ ...data, organization_id: organizationId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.accessGroups(organizationId!),
      })
    },
  })
}

export function useUpdateAccessGroup() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      id,
      updates,
    }: {
      id: string
      updates: Partial<Omit<AccessGroup, 'id' | 'organization_id' | 'created_at' | 'updated_at'>>
    }) => updateAccessGroup(id, updates),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.accessGroup(variables.id),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.accessGroups(organizationId!),
      })
    },
  })
}

export function useDeleteAccessGroup() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => deleteAccessGroup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.accessGroups(organizationId!),
      })
    },
  })
}

// Group Members Hooks
export function useGroupMembers(groupId: string) {
  return useQuery({
    queryKey: queryKeys.groupMembers(groupId),
    queryFn: () => getGroupMembers(groupId),
    enabled: !!groupId,
    staleTime: 60 * 1000,
  })
}

export function useUserGroups(userId: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.userGroups(userId, organizationId || ''),
    queryFn: () => getUserGroups(userId, organizationId!),
    enabled: !!userId && !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useAddGroupMember() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (member: Omit<GroupMember, 'id' | 'created_at'>) =>
      addGroupMember(member),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.groupMembers(variables.group_id),
      })
    },
  })
}

export function useInviteMemberByEmail() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      groupId,
      email,
      grantedBy,
      validUntil,
    }: {
      groupId: string
      email: string
      grantedBy: string
      validUntil?: string
    }) => inviteMemberByEmail(groupId, email, grantedBy, validUntil),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.groupMembers(variables.groupId),
      })
    },
  })
}

export function useRemoveGroupMember() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id }: { id: string; groupId: string }) =>
      removeGroupMember(id),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.groupMembers(variables.groupId),
      })
    },
  })
}

// Visibility Hooks
export function useNodeVisibility(nodeId: string) {
  return useQuery({
    queryKey: queryKeys.nodeVisibility(nodeId),
    queryFn: () => getNodeVisibility(nodeId),
    enabled: !!nodeId,
    staleTime: 60 * 1000,
  })
}

export function useSetNodeVisibility() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      nodeId,
      groupId,
      settings,
    }: {
      nodeId: string
      groupId: string
      settings: Partial<Omit<NodeVisibility, 'id' | 'node_id' | 'group_id' | 'created_at'>>
    }) => setNodeVisibility(nodeId, groupId, settings),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.nodeVisibility(variables.nodeId),
      })
    },
  })
}

export function useRemoveNodeVisibility() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ nodeId, groupId }: { nodeId: string; groupId: string }) =>
      removeNodeVisibility(nodeId, groupId),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.nodeVisibility(variables.nodeId),
      })
    },
  })
}

// User Permissions & Visibility Context
export function useUserPermissions(userId: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.userPermissions(userId, organizationId || ''),
    queryFn: () => getUserPermissions(userId, organizationId!),
    enabled: !!userId && !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

export function useVisibilityContext(userId: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.visibilityContext(userId, organizationId || ''),
    queryFn: () => getVisibilityContext(userId, organizationId!),
    enabled: !!userId && !!organizationId,
    staleTime: 60 * 1000,
  })
}

// Public Embed Config Hooks
export function usePublicEmbedConfig() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.publicEmbedConfig(organizationId || ''),
    queryFn: () => getPublicEmbedConfig(organizationId!),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

export function useUpdatePublicEmbedConfig() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      config: Partial<Omit<PublicEmbedConfig, 'id' | 'organization_id' | 'embed_token' | 'created_at' | 'updated_at'>>,
    ) => updatePublicEmbedConfig(organizationId!, config),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.publicEmbedConfig(organizationId!),
      })
    },
  })
}

// ======================
// Organization Chains Hooks
// ======================

// Query Options
export function outgoingChainsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.outgoingChains(orgId),
    queryFn: () => getOutgoingChains(orgId),
    staleTime: 60 * 1000, // 1 minute
    enabled: !!orgId,
  })
}

export function incomingChainsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.incomingChains(orgId),
    queryFn: () => getIncomingChains(orgId),
    staleTime: 60 * 1000,
    enabled: !!orgId,
  })
}

export function chainQueryOptions(chainId: string) {
  return queryOptions({
    queryKey: queryKeys.chain(chainId),
    queryFn: () => getChain(chainId),
    staleTime: 60 * 1000,
    enabled: !!chainId,
  })
}

// Hooks
export function useOutgoingChains() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.outgoingChains(organizationId || ''),
    queryFn: () => getOutgoingChains(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useIncomingChains() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.incomingChains(organizationId || ''),
    queryFn: () => getIncomingChains(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useChain(chainId: string) {
  return useQuery({
    queryKey: queryKeys.chain(chainId),
    queryFn: () => getChain(chainId),
    enabled: !!chainId,
    staleTime: 60 * 1000,
  })
}

export function useChainCounts() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.chainCounts(organizationId || ''),
    queryFn: () => getChainCounts(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useChainActivityLog(chainId: string) {
  return useQuery({
    queryKey: queryKeys.chainActivityLog(chainId),
    queryFn: () => getChainActivityLog(chainId),
    enabled: !!chainId,
    staleTime: 30 * 1000,
  })
}

export function useChainTrustScore(chainId: string) {
  return useQuery({
    queryKey: queryKeys.chainTrustScore(chainId),
    queryFn: () => getChainTrustScore(chainId),
    enabled: !!chainId,
    staleTime: 5 * 60 * 1000,
  })
}

/** The groups a chain approval can grant: the organisation's access groups, read only. */
export function useChainAccessGroups() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.chainAccessGroups(organizationId || ''),
    queryFn: () => getChainAccessGroups(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useSearchOrganizations(query: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.orgSearch(query, organizationId || ''),
    queryFn: () => searchOrganizations(query, organizationId!),
    enabled: !!organizationId && query.length >= 2,
    staleTime: 30 * 1000,
  })
}

// Mutations
export function useRequestChain() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      targetOrgId,
      requestedBy,
      message,
    }: {
      targetOrgId: string
      requestedBy: string
      message?: string
    }) => requestChain(organizationId!, targetOrgId, requestedBy, message),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.outgoingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.chainCounts(organizationId!),
      })
    },
  })
}

export function useApproveChain() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      chainId,
      approvedBy,
      grantedGroupId,
      linkNodeId,
    }: {
      chainId: string
      approvedBy: string
      grantedGroupId: string
      linkNodeId?: string
    }) => approveChain(chainId, approvedBy, grantedGroupId, linkNodeId),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.chain(variables.chainId),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.incomingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.chainCounts(organizationId!),
      })
    },
  })
}

export function useRejectChain() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      chainId,
      rejectedBy,
      reason,
    }: {
      chainId: string
      rejectedBy: string
      reason?: string
    }) => rejectChain(chainId, rejectedBy, reason),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.chain(variables.chainId),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.incomingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.chainCounts(organizationId!),
      })
    },
  })
}

export function useRevokeChain() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      chainId,
      revokedBy,
    }: {
      chainId: string
      revokedBy: string
    }) => revokeChain(chainId, revokedBy),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.chain(variables.chainId),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.incomingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.outgoingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.chainCounts(organizationId!),
      })
    },
  })
}

export function useCancelChainRequest() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (chainId: string) => cancelChainRequest(chainId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.outgoingChains(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.chainCounts(organizationId!),
      })
    },
  })
}

export function useUpdateChainPermissions() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      chainId,
      grantedGroupId,
      linkNodeId,
    }: {
      chainId: string
      grantedGroupId: string
      linkNodeId?: string
    }) => updateChainPermissions(chainId, grantedGroupId, linkNodeId),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.chain(variables.chainId),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.incomingChains(organizationId!),
      })
    },
  })
}

// ======================
// Campaigns & Donations Hooks
// ======================

// Query Options
export function campaignsQueryOptions(orgId: string) {
  return queryOptions({
    queryKey: queryKeys.campaigns(orgId),
    queryFn: () => getCampaigns(orgId),
    staleTime: 60 * 1000, // 1 minute
    enabled: !!orgId,
  })
}

export function campaignQueryOptions(id: string, orgId: string) {
  return queryOptions({
    queryKey: queryKeys.campaign(id, orgId),
    queryFn: () => getCampaign(id, orgId),
    staleTime: 60 * 1000,
    enabled: !!id && !!orgId,
  })
}

export function campaignBySlugQueryOptions(slug: string) {
  return queryOptions({
    queryKey: queryKeys.campaignBySlug(slug),
    queryFn: () => getCampaignBySlug(slug),
    staleTime: 60 * 1000,
    enabled: !!slug,
  })
}

// Campaign Hooks
export function useCampaigns() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.campaigns(organizationId || ''),
    queryFn: () => getCampaigns(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useCampaign(id: string) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.campaign(id, organizationId || ''),
    queryFn: () => getCampaign(id, organizationId!),
    enabled: !!organizationId && !!id,
    staleTime: 60 * 1000,
  })
}

export function useCampaignBySlug(slug: string) {
  return useQuery({
    queryKey: queryKeys.campaignBySlug(slug),
    queryFn: () => getCampaignBySlug(slug),
    enabled: !!slug,
    staleTime: 60 * 1000,
  })
}

// Alias for public campaign access
export const usePublicCampaign = useCampaignBySlug

/** A published campaign's figures, readable with no sign-in (the donate page). */
export function usePublicCampaignStats(campaignId: string) {
  return useQuery({
    queryKey: [...queryKeys.campaignStats(campaignId), 'public'],
    queryFn: () => getPublicCampaignStats(campaignId),
    enabled: !!campaignId,
    staleTime: 30 * 1000,
  })
}

/** The score a public trust badge draws, readable with no sign-in. */
export function usePublicTrustScore(organizationId: string) {
  return useQuery({
    queryKey: ['trust-score', organizationId, 'public'],
    queryFn: () => getPublicTrustScore(organizationId),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

export function useCampaignStats(campaignId: string) {
  return useQuery({
    queryKey: queryKeys.campaignStats(campaignId),
    queryFn: () => getCampaignStats(campaignId),
    enabled: !!campaignId,
    staleTime: 30 * 1000,
  })
}

export function useCampaignCounts() {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.campaignCounts(organizationId || ''),
    queryFn: () => getCampaignCounts(organizationId!),
    enabled: !!organizationId,
    staleTime: 60 * 1000,
  })
}

export function useUnderfundedCampaigns(limit = 5) {
  const { organizationId } = useOrg()

  return useQuery({
    queryKey: queryKeys.underfundedCampaigns(organizationId || ''),
    queryFn: () => getUnderfundedCampaigns(organizationId!, limit),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000,
  })
}

// Campaign Mutations
export function useCreateCampaign() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<Campaign, 'id' | 'current_amount' | 'created_at' | 'updated_at' | 'organization_id'>,
    ) => createCampaign({ ...data, organization_id: organizationId! }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaigns(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaignCounts(organizationId!),
      })
    },
  })
}

export function useUpdateCampaign() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      id,
      updates,
    }: {
      id: string
      updates: Partial<Omit<Campaign, 'id' | 'organization_id' | 'created_at' | 'updated_at'>>
    }) => updateCampaign(id, organizationId!, updates),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaign(variables.id, organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaigns(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaignCounts(organizationId!),
      })
    },
  })
}

export function useDeleteCampaign() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => deleteCampaign(id, organizationId!),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaigns(organizationId!),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaignCounts(organizationId!),
      })
    },
  })
}

// Donations Hooks
export function useDonations(campaignId: string) {
  return useQuery({
    queryKey: queryKeys.donations(campaignId),
    queryFn: () => getDonations(campaignId),
    enabled: !!campaignId,
    staleTime: 30 * 1000,
  })
}

export function useDonation(id: string) {
  return useQuery({
    queryKey: queryKeys.donation(id),
    queryFn: () => getDonation(id),
    enabled: !!id,
    staleTime: 30 * 1000,
  })
}

export function useDonorDonations(email: string) {
  return useQuery({
    queryKey: queryKeys.donorDonations(email),
    queryFn: () => getDonorDonations(email),
    enabled: !!email,
    staleTime: 60 * 1000,
  })
}

// Donation Mutations
export function useCreateDonation() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<Donation, 'id' | 'impact_percentage' | 'created_at' | 'updated_at'>,
    ) => createDonation(data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.donations(variables.campaign_id),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaignStats(variables.campaign_id),
      })
      // Invalidate campaign to refresh current_amount
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === 'campaign' &&
          query.queryKey[1] === variables.campaign_id,
      })
    },
  })
}

export function useUpdateDonationStatus() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      id,
      status,
      paymentReference,
    }: {
      id: string
      status: PaymentStatus
      paymentReference?: string
    }) => updateDonationStatus(id, status, paymentReference),
    onSuccess: (data) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.donation(data.id),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.donations(data.campaign_id),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.campaignStats(data.campaign_id),
      })
      // Invalidate campaign to refresh current_amount
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === 'campaign' &&
          query.queryKey[1] === data.campaign_id,
      })
    },
  })
}

// Donor Updates Hooks
export function useDonorUpdates(donationId: string) {
  return useQuery({
    queryKey: queryKeys.donorUpdates(donationId),
    queryFn: () => getDonorUpdates(donationId),
    enabled: !!donationId,
    staleTime: 60 * 1000,
  })
}

export function useCampaignUpdates(campaignId: string) {
  return useQuery({
    queryKey: queryKeys.campaignUpdates(campaignId),
    queryFn: () => getCampaignUpdates(campaignId),
    enabled: !!campaignId,
    staleTime: 60 * 1000,
  })
}

export function useCreateDonorUpdate() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (
      data: Omit<DonorUpdate, 'id' | 'sent_at' | 'created_at'>,
    ) => createDonorUpdate(data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.donorUpdates(variables.donation_id),
      })
      if (variables.campaign_id) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.campaignUpdates(variables.campaign_id),
        })
      }
    },
  })
}

export function useMarkUpdateSent() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => markUpdateSent(id),
    onSuccess: (data) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.donorUpdates(data.donation_id),
      })
      if (data.campaign_id) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.campaignUpdates(data.campaign_id),
        })
      }
    },
  })
}

// Export types for components
export type {
  AccessGroup,
  AccessGroupPermissions,
  Campaign,
  CampaignCounts,
  CampaignStats,
  CampaignStatus,
  CampaignWithDetails,
  ChainActivityLog,
  ChainCounts,
  ChainStatus,
  ChainTrustScore,
  DeliveryMethod,
  Donation,
  DonationWithDetails,
  DonorUpdate,
  DonorUpdateType,
  DonorUpdateWithDetails,
  FlowDataResult,
  GroupMember,
  NodeVisibility,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
  PaymentStatus,
  PublicEmbedConfig,
  TrustScoreResult,
  Verification,
  VerificationType,
  VerificationVisibility,
  VisibilityContext,
}
