import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useOrg } from './org-context'
import {
  createNode,
  createTransaction,
  createVerification,
  getNodes,
  getOrganizationStats,
  getTransaction,
  getTransactions,
  getTrustScoreHistory,
  getVerifications,
  updateTransaction,
  uploadVerificationFile,
} from './supabase'
import type { FlowNode, Transaction, Verification } from './supabase'

// Query key factory for consistent cache management
export const queryKeys = {
  transactions: (orgId: string) => ['transactions', orgId] as const,
  transaction: (id: string, orgId: string) =>
    ['transaction', id, orgId] as const,
  nodes: (orgId: string) => ['nodes', orgId] as const,
  orgStats: (orgId: string) => ['org-stats', orgId] as const,
  verifications: (txnId: string) => ['verifications', txnId] as const,
  trustHistory: (orgId: string) => ['trust-history', orgId] as const,
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

export function useCreateVerification() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({
      transactionId,
      type,
      fileUrl,
    }: {
      transactionId: string
      type: Verification['type']
      fileUrl?: string
    }) =>
      createVerification({
        transaction_id: transactionId,
        type,
        file_url: fileUrl || null,
        ocr_data: null,
        metadata: {},
        status: 'pending',
        verified_by: null,
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

export function useUploadVerification() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      transactionId,
      file,
      type,
    }: {
      transactionId: string
      file: File
      type: Verification['type']
    }) => {
      // Upload file to storage
      const fileUrl = await uploadVerificationFile(
        organizationId!,
        transactionId,
        file,
      )

      // Create verification record
      return createVerification({
        transaction_id: transactionId,
        type,
        file_url: fileUrl,
        ocr_data: null,
        metadata: { originalName: file.name, size: file.size },
        status: 'pending',
        verified_by: null,
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
