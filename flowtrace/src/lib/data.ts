// Data access for the app. In dummy auth mode (DEV_MODE) every call is
// served from the bundled demo dataset, so the app works with no backend;
// otherwise it goes to Supabase. Each export keeps the live signature.
//
// Campaigns, donations, donor updates and the organisation profile (RW1), and
// transactions, flow nodes, node visibility, the flow data, the organisation
// stats and chains (RW2) are served by the SpacetimeDB module when VITE_LEDGER_BACKEND=spacetimedb
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
export const calculateTrustScoreAPI: typeof live.calculateTrustScoreAPI =
  DEV_MODE ? demo.calculateTrustScoreAPI : live.calculateTrustScoreAPI
export const createAccessGroup: typeof live.createAccessGroup = DEV_MODE
  ? demo.createAccessGroup
  : live.createAccessGroup
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
export const getGroupMembers: typeof live.getGroupMembers = DEV_MODE
  ? demo.getGroupMembers
  : live.getGroupMembers
export const getPublicEmbedConfig: typeof live.getPublicEmbedConfig = DEV_MODE
  ? demo.getPublicEmbedConfig
  : live.getPublicEmbedConfig
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
export const removeGroupMember: typeof live.removeGroupMember = DEV_MODE
  ? demo.removeGroupMember
  : live.removeGroupMember
export const updateAccessGroup: typeof live.updateAccessGroup = DEV_MODE
  ? demo.updateAccessGroup
  : live.updateAccessGroup
export const updatePublicEmbedConfig: typeof live.updatePublicEmbedConfig =
  DEV_MODE ? demo.updatePublicEmbedConfig : live.updatePublicEmbedConfig
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

// ---------------------------------------------------------------------------
// Transactions, flow nodes, node visibility, flow data, chains (RW2)
// ---------------------------------------------------------------------------

export const getTransactions = served('getTransactions', demo.getTransactions)
export const getTransaction = served('getTransaction', demo.getTransaction)
export const createTransaction = served('createTransaction', demo.createTransaction)
export const updateTransaction = served('updateTransaction', demo.updateTransaction)
export const getNodes = served('getNodes', demo.getNodes)
export const createNode = served('createNode', demo.createNode)
export const getOrganizationStats = served('getOrganizationStats', demo.getOrganizationStats)
/** Computed from the flow nodes and transactions; the module has no flow-data function. */
export const getFlowDataAPI = served('getFlowDataAPI', demo.getFlowDataAPI)
export const getNodeVisibility = served('getNodeVisibility', demo.getNodeVisibility)
export const setNodeVisibility = served('setNodeVisibility', demo.setNodeVisibility)
export const removeNodeVisibility = served('removeNodeVisibility', demo.removeNodeVisibility)
export const getOutgoingChains = served('getOutgoingChains', demo.getOutgoingChains)
export const getIncomingChains = served('getIncomingChains', demo.getIncomingChains)
export const getChain = served('getChain', demo.getChain)
export const requestChain = served('requestChain', demo.requestChain)
export const approveChain = served('approveChain', demo.approveChain)
export const rejectChain = served('rejectChain', demo.rejectChain)
export const revokeChain = served('revokeChain', demo.revokeChain)
export const cancelChainRequest = served('cancelChainRequest', demo.cancelChainRequest)
export const updateChainPermissions = served('updateChainPermissions', demo.updateChainPermissions)
export const getChainActivityLog = served('getChainActivityLog', demo.getChainActivityLog)
export const getChainCounts = served('getChainCounts', demo.getChainCounts)
export const getChainTrustScore = served('getChainTrustScore', demo.getChainTrustScore)
/** The access groups a chain approval can grant, read from the module (RW3 moves their screens). */
export const getChainAccessGroups = served('getChainAccessGroups', demo.getAccessGroups)
/** Organisations a chain can be asked of: the module's published ones (public_organization). */
export const searchOrganizations = publicly(publicReads.searchPublicOrganizations, demo.searchOrganizations)
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
