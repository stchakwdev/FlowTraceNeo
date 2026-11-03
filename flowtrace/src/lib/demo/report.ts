// Pure reads over demo data, shared by the demo API (on its live store) and the landing page (on
// the seed). No timers, no ledger engine, so the landing page can import it without pulling the
// demo books into its bundle.
import type {
  Campaign,
  CampaignWithDetails,
  FlowNode,
  Organization,
  Transaction,
  TransactionWithNodes,
} from '../types'

export interface DemoData {
  organizations: Array<Organization>
  flowNodes: Array<FlowNode>
  transactions: Array<Transaction>
  campaigns: Array<Campaign>
}

export interface CampaignReportEarmark {
  id: string
  name: string
  type: FlowNode['type']
  /** Money received by this node (non-rejected inflows). */
  received: number
  /** The part of `received` backed by verified proof. */
  evidenced: number
  /** The campaign's own target node. */
  is_target: boolean
}

export interface CampaignReport {
  campaign: CampaignWithDetails
  organization: {
    id: string
    name: string
    trust_score: number
    verified: boolean
  }
  /** Verified spending into the campaign's target node, capped at the raised amount. */
  evidenced_amount: number | null
  earmarks: Array<CampaignReportEarmark>
  /** Recent programme spending, newest first, with its proof state. */
  spending: Array<TransactionWithNodes>
}

const byDateDesc = (a: string, b: string) => (a < b ? 1 : a > b ? -1 : 0)

export function nodeRef(data: Pick<DemoData, 'flowNodes'>, id: string | null) {
  const n = id ? data.flowNodes.find((x) => x.id === id) : undefined
  return n ? { id: n.id, name: n.name, type: n.type } : null
}

export const withNodes = (
  data: Pick<DemoData, 'flowNodes'>,
  t: Transaction,
): TransactionWithNodes => ({
  ...t,
  from_node: nodeRef(data, t.from_node_id),
  to_node: nodeRef(data, t.to_node_id),
})

/** An organisation's transactions, newest first, with their node names. */
export function transactionsOf(data: DemoData, organizationId: string) {
  return data.transactions
    .filter((t) => t.organization_id === organizationId)
    .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
    .map((t) => withNodes(data, t))
}

/** The public donate page's report for a campaign; null when the campaign or org is missing. */
export function campaignReport(
  data: DemoData,
  campaignId: string,
): CampaignReport | null {
  const c = data.campaigns.find((x) => x.id === campaignId)
  if (!c) return null
  const o = data.organizations.find((x) => x.id === c.organization_id)
  if (!o) return null
  const txns = data.transactions.filter(
    (t) =>
      t.organization_id === c.organization_id &&
      t.verification_status !== 'rejected',
  )
  const sum = (rows: Array<Transaction>) =>
    rows.reduce((s, t) => s + t.amount, 0)
  const earmarks = data.flowNodes
    .filter(
      (n) =>
        n.organization_id === c.organization_id &&
        (n.type === 'project' || n.type === 'department'),
    )
    .map((n) => {
      const inflow = txns.filter((t) => t.to_node_id === n.id)
      return {
        id: n.id,
        name: n.name,
        type: n.type,
        received: sum(inflow),
        evidenced: sum(
          inflow.filter((t) => t.verification_status === 'verified'),
        ),
        is_target: n.id === c.target_node_id,
      }
    })
    .sort(
      (a, b) =>
        Number(b.is_target) - Number(a.is_target) || b.received - a.received,
    )
  const programme = new Set(earmarks.map((e) => e.id))
  const spending = txns
    .filter(
      (t) =>
        (t.to_node_id && programme.has(t.to_node_id)) ||
        (t.from_node_id && programme.has(t.from_node_id)),
    )
    .sort((a, b) => byDateDesc(a.transaction_date, b.transaction_date))
    .slice(0, 6)
    .map((t) => withNodes(data, t))
  const target = earmarks.find((e) => e.is_target)
  return {
    campaign: { ...c, target_node: nodeRef(data, c.target_node_id) },
    organization: {
      id: o.id,
      name: o.name,
      trust_score: o.trust_score,
      verified: o.verified,
    },
    evidenced_amount: target
      ? Math.min(target.evidenced, c.current_amount)
      : null,
    earmarks,
    spending,
  }
}
