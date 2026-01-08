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
  createNode,
  createTransaction,
  createVerification,
  deleteAccessGroup,
  getAccessGroup,
  getAccessGroups,
  getChain,
  getChainActivityLog,
  getChainCounts,
  getChainTrustScore,
  getFlowDataAPI,
  getGroupMembers,
  getIncomingChains,
  getNodes,
  getNodeVisibility,
  getOrganizationStats,
  getOutgoingChains,
  getPublicEmbedConfig,
  getTransaction,
  getTransactions,
  getTrustScoreHistory,
  getUserGroups,
  getUserPermissions,
  getVerifications,
  getVisibilityContext,
  inviteMemberByEmail,
  processReceiptOCR,
  rejectChain,
  removeGroupMember,
  removeNodeVisibility,
  requestChain,
  revokeChain,
  searchOrganizations,
  setNodeVisibility,
  updateAccessGroup,
  updateChainPermissions,
  updatePublicEmbedConfig,
  updateTransaction,
  uploadVerificationFile,
} from './supabase'
import type {
  AccessGroup,
  AccessGroupPermissions,
  ChainActivityLog,
  ChainCounts,
  ChainStatus,
  ChainTrustScore,
  FlowDataResult,
  FlowNode,
  GroupMember,
  NodeVisibility,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
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
  orgSearch: (query: string, excludeOrgId: string) =>
    ['org-search', query, excludeOrgId] as const,
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
    mutationFn: ({ id, groupId }: { id: string; groupId: string }) =>
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

// Export types for components
export type {
  AccessGroup,
  AccessGroupPermissions,
  ChainActivityLog,
  ChainCounts,
  ChainStatus,
  ChainTrustScore,
  FlowDataResult,
  GroupMember,
  NodeVisibility,
  Organization,
  OrganizationChain,
  OrganizationChainWithDetails,
  PublicEmbedConfig,
  TrustScoreResult,
  Verification,
  VerificationType,
  VerificationVisibility,
  VisibilityContext,
}
