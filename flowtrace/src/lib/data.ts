// Data access for the app. In dummy auth mode (DEV_MODE) every call is
// served from the bundled demo dataset, so the app works with no backend;
// otherwise it goes to Supabase. Each export keeps the live signature.
//
// Campaigns, donations, donor updates and the organisation profile (RW1) are
// served by the SpacetimeDB module when VITE_LEDGER_BACKEND=spacetimedb
// (ledger/spacetime-transparency.ts, loaded lazily with the SDK), by the demo
// dataset when the books are the demo's, and by nothing otherwise: Supabase
// is gone (Docs/adr/0001-native-ledger.md). Public pages read the module's
// public tables anonymously (ledger/public-sql.ts).
import { DEV_MODE } from './auth'
import * as demo from './demo/api'
import * as live from './supabase'
import * as publicReads from './ledger/public-sql'
import { BACKEND, BooksNotConnectedError } from './spacetime-session'
import type { LedgerBackendKind } from './spacetime-session'
import type * as TransparencyModule from './ledger/spacetime-transparency'

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
export const deleteVerificationFile: typeof live.deleteVerificationFile =
  DEV_MODE ? demo.deleteVerificationFile : live.deleteVerificationFile
export const getAccessGroup: typeof live.getAccessGroup = DEV_MODE
  ? demo.getAccessGroup
  : live.getAccessGroup
export const getAccessGroups: typeof live.getAccessGroups = DEV_MODE
  ? demo.getAccessGroups
  : live.getAccessGroups
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
export const updateChainPermissions: typeof live.updateChainPermissions =
  DEV_MODE ? demo.updateChainPermissions : live.updateChainPermissions
export const updatePublicEmbedConfig: typeof live.updatePublicEmbedConfig =
  DEV_MODE ? demo.updatePublicEmbedConfig : live.updatePublicEmbedConfig
export const updateTransaction: typeof live.updateTransaction = DEV_MODE
  ? demo.updateTransaction
  : live.updateTransaction
export const uploadVerificationFile: typeof live.uploadVerificationFile =
  DEV_MODE ? demo.uploadVerificationFile : live.uploadVerificationFile
// ---------------------------------------------------------------------------
// Campaigns, donations, donor updates, organisation profile (RW1)
// ---------------------------------------------------------------------------

export type TransparencySource = 'module' | 'demo' | 'none'

/** Who serves campaigns, donations and the organisation profile for a backend kind. */
export function transparencySource(kind: LedgerBackendKind): TransparencySource {
  if (kind === 'spacetimedb') return 'module'
  if (kind === 'demo') return 'demo'
  return 'none'
}

const SOURCE = transparencySource(BACKEND)

type Module = typeof TransparencyModule
type Fn = (...args: Array<never>) => Promise<unknown>

// Lazy, so the demo bundle never loads the SpacetimeDB SDK.
const transparencyLoaders = import.meta.glob<Module>(
  './ledger/spacetime-transparency.ts',
)

export function loadTransparency(): Promise<Module> {
  const load = Object.values(transparencyLoaders).at(0)
  if (!load) throw new Error('The SpacetimeDB module is not built into this app.')
  return load()
}

const notConnected = () => Promise.reject(new BooksNotConnectedError())

/** The module's function when the module serves the app, else the demo's, else a refusal. */
function served<TName extends keyof Module>(
  name: TName,
  demoFn: Module[TName],
): Module[TName] {
  if (SOURCE === 'demo') return demoFn
  if (SOURCE === 'none') return notConnected as unknown as Module[TName]
  const viaModule = (...args: Array<unknown>) =>
    loadTransparency().then((m) => (m[name] as unknown as (...a: Array<unknown>) => Promise<unknown>)(...args))
  return viaModule as unknown as Module[TName]
}

/** For public pages: the anonymous read when the module serves the app. */
function publicly<T extends Fn>(moduleFn: T, demoFn: T): T {
  if (SOURCE === 'demo') return demoFn
  if (SOURCE === 'none') return notConnected as unknown as T
  return moduleFn
}

export const getCampaigns = served('getCampaigns', demo.getCampaigns)
export const getCampaign = served('getCampaign', demo.getCampaign)
export const createCampaign = served('createCampaign', demo.createCampaign)
export const updateCampaign = served('updateCampaign', demo.updateCampaign)
export const deleteCampaign = served('deleteCampaign', demo.deleteCampaign)
export const getCampaignStats = served('getCampaignStats', demo.getCampaignStats)
export const getCampaignCounts = served('getCampaignCounts', demo.getCampaignCounts)
export const getUnderfundedCampaigns = served('getUnderfundedCampaigns', demo.getUnderfundedCampaigns)
export const getDonations = served('getDonations', demo.getDonations)
export const getDonation = served('getDonation', demo.getDonation)
export const getDonorDonations = served('getDonorDonations', demo.getDonorDonations)
export const createDonation = served('createDonation', demo.createDonation)
export const updateDonationStatus = served('updateDonationStatus', demo.updateDonationStatus)
export const getDonorUpdates = served('getDonorUpdates', demo.getDonorUpdates)
export const getCampaignUpdates = served('getCampaignUpdates', demo.getCampaignUpdates)
export const createDonorUpdate = served('createDonorUpdate', demo.createDonorUpdate)
export const markUpdateSent = served('markUpdateSent', demo.markUpdateSent)
export const getOrganization = served('getOrganization', demo.getOrganization)
export const updateOrganization = served('updateOrganization', demo.updateOrganization)

// Campaign report: the org behind a campaign, where money can be earmarked,
// and recent programme spending with its proof state.
export type {
  CampaignReport,
  CampaignReportEarmark,
} from './demo/api'

/** For a member on the campaign page. */
export const getCampaignReport = served('getCampaignReport', demo.getCampaignReport)

/** Public pages: the donate page, the campaign embed and the trust badge, readable with no token. */
export const getCampaignBySlug = publicly(publicReads.getPublicCampaignBySlug, demo.getCampaignBySlug)
export const getPublicCampaignStats = publicly(publicReads.getPublicCampaignStats, demo.getCampaignStats)
export const getPublicCampaignReport = publicly(publicReads.getPublicCampaignReport, demo.getCampaignReport)
export const getPublicOrganization = publicly(publicReads.getPublicOrganization, demo.getOrganization)

/** The badge's score: the module's published badge, or the demo's latest history snapshot. */
export const getPublicTrustScore = publicly(publicReads.getPublicBadge, async (organizationId: string) => {
  const history = await demo.getTrustScoreHistory(organizationId, 1)
  const latest = history.at(-1)
  return {
    overall_score: latest?.score ?? 0,
    breakdown: (latest?.breakdown ?? {}),
    verification_rate: latest?.verification_rate ?? null,
    calculated_at: latest?.calculated_at ?? null,
  }
})
