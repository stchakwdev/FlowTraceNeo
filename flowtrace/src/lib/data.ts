// Data access for the app. In dummy auth mode (DEV_MODE) every call is
// served from the bundled demo dataset, so the app works with no backend;
// otherwise it goes to Supabase. Each export keeps the live signature.
import { DEV_MODE } from './auth'
import * as demo from './demo/api'
import * as live from './supabase'
import type { OrganizationProfileUpdate } from './demo/api'

export const addGroupMember: typeof live.addGroupMember = DEV_MODE
  ? demo.addGroupMember
  : live.addGroupMember
export const approveChain: typeof live.approveChain = DEV_MODE
  ? demo.approveChain
  : live.approveChain
export const calculateTrustScoreAPI: typeof live.calculateTrustScoreAPI =
  DEV_MODE ? demo.calculateTrustScoreAPI : live.calculateTrustScoreAPI
export const cancelChainRequest: typeof live.cancelChainRequest = DEV_MODE
  ? demo.cancelChainRequest
  : live.cancelChainRequest
export const createAccessGroup: typeof live.createAccessGroup = DEV_MODE
  ? demo.createAccessGroup
  : live.createAccessGroup
export const createCampaign: typeof live.createCampaign = DEV_MODE
  ? demo.createCampaign
  : live.createCampaign
export const createDonation: typeof live.createDonation = DEV_MODE
  ? demo.createDonation
  : live.createDonation
export const createDonorUpdate: typeof live.createDonorUpdate = DEV_MODE
  ? demo.createDonorUpdate
  : live.createDonorUpdate
export const createNode: typeof live.createNode = DEV_MODE
  ? demo.createNode
  : live.createNode
export const createTransaction: typeof live.createTransaction = DEV_MODE
  ? demo.createTransaction
  : live.createTransaction
export const createVerification: typeof live.createVerification = DEV_MODE
  ? demo.createVerification
  : live.createVerification
export const deleteAccessGroup: typeof live.deleteAccessGroup = DEV_MODE
  ? demo.deleteAccessGroup
  : live.deleteAccessGroup
export const deleteCampaign: typeof live.deleteCampaign = DEV_MODE
  ? demo.deleteCampaign
  : live.deleteCampaign
export const deleteVerificationFile: typeof live.deleteVerificationFile =
  DEV_MODE ? demo.deleteVerificationFile : live.deleteVerificationFile
export const getAccessGroup: typeof live.getAccessGroup = DEV_MODE
  ? demo.getAccessGroup
  : live.getAccessGroup
export const getAccessGroups: typeof live.getAccessGroups = DEV_MODE
  ? demo.getAccessGroups
  : live.getAccessGroups
export const getCampaign: typeof live.getCampaign = DEV_MODE
  ? demo.getCampaign
  : live.getCampaign
export const getCampaignBySlug: typeof live.getCampaignBySlug = DEV_MODE
  ? demo.getCampaignBySlug
  : live.getCampaignBySlug
export const getCampaignCounts: typeof live.getCampaignCounts = DEV_MODE
  ? demo.getCampaignCounts
  : live.getCampaignCounts
export const getCampaigns: typeof live.getCampaigns = DEV_MODE
  ? demo.getCampaigns
  : live.getCampaigns
export const getCampaignStats: typeof live.getCampaignStats = DEV_MODE
  ? demo.getCampaignStats
  : live.getCampaignStats
export const getCampaignUpdates: typeof live.getCampaignUpdates = DEV_MODE
  ? demo.getCampaignUpdates
  : live.getCampaignUpdates
export const getChain: typeof live.getChain = DEV_MODE
  ? demo.getChain
  : live.getChain
export const getChainActivityLog: typeof live.getChainActivityLog = DEV_MODE
  ? demo.getChainActivityLog
  : live.getChainActivityLog
export const getChainCounts: typeof live.getChainCounts = DEV_MODE
  ? demo.getChainCounts
  : live.getChainCounts
export const getChainTrustScore: typeof live.getChainTrustScore = DEV_MODE
  ? demo.getChainTrustScore
  : live.getChainTrustScore
export const getDonation: typeof live.getDonation = DEV_MODE
  ? demo.getDonation
  : live.getDonation
export const getDonations: typeof live.getDonations = DEV_MODE
  ? demo.getDonations
  : live.getDonations
export const getDonorDonations: typeof live.getDonorDonations = DEV_MODE
  ? demo.getDonorDonations
  : live.getDonorDonations
export const getDonorUpdates: typeof live.getDonorUpdates = DEV_MODE
  ? demo.getDonorUpdates
  : live.getDonorUpdates
export const getFlowDataAPI: typeof live.getFlowDataAPI = DEV_MODE
  ? demo.getFlowDataAPI
  : live.getFlowDataAPI
export const getGroupMembers: typeof live.getGroupMembers = DEV_MODE
  ? demo.getGroupMembers
  : live.getGroupMembers
export const getIncomingChains: typeof live.getIncomingChains = DEV_MODE
  ? demo.getIncomingChains
  : live.getIncomingChains
export const getNodes: typeof live.getNodes = DEV_MODE
  ? demo.getNodes
  : live.getNodes
export const getNodeVisibility: typeof live.getNodeVisibility = DEV_MODE
  ? demo.getNodeVisibility
  : live.getNodeVisibility
export const getOrganizationStats: typeof live.getOrganizationStats = DEV_MODE
  ? demo.getOrganizationStats
  : live.getOrganizationStats
export const getOutgoingChains: typeof live.getOutgoingChains = DEV_MODE
  ? demo.getOutgoingChains
  : live.getOutgoingChains
export const getPublicEmbedConfig: typeof live.getPublicEmbedConfig = DEV_MODE
  ? demo.getPublicEmbedConfig
  : live.getPublicEmbedConfig
export const getTransaction: typeof live.getTransaction = DEV_MODE
  ? demo.getTransaction
  : live.getTransaction
export const getTransactions: typeof live.getTransactions = DEV_MODE
  ? demo.getTransactions
  : live.getTransactions
export const getTrustScoreHistory: typeof live.getTrustScoreHistory = DEV_MODE
  ? demo.getTrustScoreHistory
  : live.getTrustScoreHistory
export const getUnderfundedCampaigns: typeof live.getUnderfundedCampaigns =
  DEV_MODE ? demo.getUnderfundedCampaigns : live.getUnderfundedCampaigns
export const getUserGroups: typeof live.getUserGroups = DEV_MODE
  ? demo.getUserGroups
  : live.getUserGroups
export const getUserPermissions: typeof live.getUserPermissions = DEV_MODE
  ? demo.getUserPermissions
  : live.getUserPermissions
export const getVerifications: typeof live.getVerifications = DEV_MODE
  ? demo.getVerifications
  : live.getVerifications
export const getVisibilityContext: typeof live.getVisibilityContext = DEV_MODE
  ? demo.getVisibilityContext
  : live.getVisibilityContext
export const inviteMemberByEmail: typeof live.inviteMemberByEmail = DEV_MODE
  ? demo.inviteMemberByEmail
  : live.inviteMemberByEmail
export const markUpdateSent: typeof live.markUpdateSent = DEV_MODE
  ? demo.markUpdateSent
  : live.markUpdateSent
export const processReceiptOCR: typeof live.processReceiptOCR = DEV_MODE
  ? demo.processReceiptOCR
  : live.processReceiptOCR
export const rejectChain: typeof live.rejectChain = DEV_MODE
  ? demo.rejectChain
  : live.rejectChain
export const removeGroupMember: typeof live.removeGroupMember = DEV_MODE
  ? demo.removeGroupMember
  : live.removeGroupMember
export const removeNodeVisibility: typeof live.removeNodeVisibility = DEV_MODE
  ? demo.removeNodeVisibility
  : live.removeNodeVisibility
export const requestChain: typeof live.requestChain = DEV_MODE
  ? demo.requestChain
  : live.requestChain
export const revokeChain: typeof live.revokeChain = DEV_MODE
  ? demo.revokeChain
  : live.revokeChain
export const searchOrganizations: typeof live.searchOrganizations = DEV_MODE
  ? demo.searchOrganizations
  : live.searchOrganizations
export const setNodeVisibility: typeof live.setNodeVisibility = DEV_MODE
  ? demo.setNodeVisibility
  : live.setNodeVisibility
export const updateAccessGroup: typeof live.updateAccessGroup = DEV_MODE
  ? demo.updateAccessGroup
  : live.updateAccessGroup
export const updateCampaign: typeof live.updateCampaign = DEV_MODE
  ? demo.updateCampaign
  : live.updateCampaign
export const updateChainPermissions: typeof live.updateChainPermissions =
  DEV_MODE ? demo.updateChainPermissions : live.updateChainPermissions
export const updateDonationStatus: typeof live.updateDonationStatus = DEV_MODE
  ? demo.updateDonationStatus
  : live.updateDonationStatus
export const updatePublicEmbedConfig: typeof live.updatePublicEmbedConfig =
  DEV_MODE ? demo.updatePublicEmbedConfig : live.updatePublicEmbedConfig
export const updateTransaction: typeof live.updateTransaction = DEV_MODE
  ? demo.updateTransaction
  : live.updateTransaction
export const uploadVerificationFile: typeof live.uploadVerificationFile =
  DEV_MODE ? demo.uploadVerificationFile : live.uploadVerificationFile
export const getOrganization: typeof live.getOrganization = DEV_MODE
  ? demo.getOrganization
  : live.getOrganization
export const updateOrganization: (
  id: string,
  updates: OrganizationProfileUpdate,
) => Promise<live.Organization> = DEV_MODE
  ? demo.updateOrganization
  : async (id, updates) => {
      const { data, error } = await live.supabase
        .from('organizations')
        .update(updates)
        .eq('id', id)
        .select('*')
        .single()
      if (error) throw error
      return data as live.Organization
    }

// Campaign report: the org behind a campaign, where money can be earmarked,
// and recent programme spending with its proof state. Read by the public
// donate page, so it must not depend on the signed-in org.
export type {
  CampaignReport,
  CampaignReportEarmark,
} from './demo/api'

async function liveCampaignReport(
  campaignId: string,
): Promise<demo.CampaignReport> {
  const { data: c, error } = await live.supabase
    .from('campaigns')
    .select('*, target_node:flow_nodes(id, name, type)')
    .eq('id', campaignId)
    .single()
  if (error) throw error
  const campaign = c as live.CampaignWithDetails
  const [orgRes, nodesRes, txnRes] = await Promise.all([
    live.supabase
      .from('organizations')
      .select('id, name, trust_score, verified')
      .eq('id', c.organization_id)
      .single(),
    live.supabase
      .from('flow_nodes')
      .select('id, name, type')
      .eq('organization_id', c.organization_id)
      .in('type', ['project', 'department']),
    live.supabase
      .from('transactions')
      .select(
        '*, from_node:flow_nodes!from_node_id(id, name, type), to_node:flow_nodes!to_node_id(id, name, type)',
      )
      .eq('organization_id', c.organization_id)
      .neq('verification_status', 'rejected')
      .order('transaction_date', { ascending: false }),
  ])
  if (orgRes.error) throw orgRes.error
  if (nodesRes.error) throw nodesRes.error
  if (txnRes.error) throw txnRes.error
  const txns = txnRes.data as Array<live.TransactionWithNodes>
  const earmarks = (
    nodesRes.data as Array<Pick<live.FlowNode, 'id' | 'name' | 'type'>>
  )
    .map((n) => {
      const inflow = txns.filter((t) => t.to_node_id === n.id)
      return {
        ...n,
        received: inflow.reduce((s, t) => s + t.amount, 0),
        evidenced: inflow
          .filter((t) => t.verification_status === 'verified')
          .reduce((s, t) => s + t.amount, 0),
        is_target: n.id === c.target_node_id,
      }
    })
    .sort(
      (a, b) =>
        Number(b.is_target) - Number(a.is_target) || b.received - a.received,
    )
  const programme = new Set(earmarks.map((e) => e.id))
  const target = earmarks.find((e) => e.is_target)
  return {
    campaign,
    organization: orgRes.data as demo.CampaignReport['organization'],
    evidenced_amount: target
      ? Math.min(target.evidenced, campaign.current_amount)
      : null,
    earmarks,
    spending: txns
      .filter(
        (t) =>
          (t.to_node_id && programme.has(t.to_node_id)) ||
          (t.from_node_id && programme.has(t.from_node_id)),
      )
      .slice(0, 6),
  }
}

export const getCampaignReport: (
  campaignId: string,
) => Promise<demo.CampaignReport> = DEV_MODE
  ? demo.getCampaignReport
  : liveCampaignReport
