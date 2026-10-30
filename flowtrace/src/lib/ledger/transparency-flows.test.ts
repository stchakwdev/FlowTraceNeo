import { beforeEach, describe, expect, it } from 'vitest'
import dataset from '../../../../rust/fixtures/transparency-dataset.json'
import * as demo from '../demo/api'
import { summarise } from '../dashboard-summary'
import { fromMajor } from './money'
import {
  chainCounts,
  chainTrustScore,
  chainsOf,
  decodeChain,
  decodeFlowNode,
  decodeTransaction,
  flowData,
  flowNodeInput,
  matchOrganizations,
  nodeVisibilitySettings,
  nodeVisibilityWithGroups,
  orgStats,
  transactionInput,
  transactionPatch,
  transactionsWithNodes,
} from './transparency-model'
import type {
  AccessGroup,
  FlowNode,
  OrganizationChain,
  Transaction,
} from '../types'

// The demo dataset as the module holds it (rust/fixtures/transparency-dataset.json is what the
// demo seeds): doc rows whose JSON keeps amounts in minor units (flows.rs:46-120).
const store = dataset.store
const ORG = dataset.demo_org_id

type Rec = Record<string, unknown>
const asRow = (rec: Rec, money: Array<string>, seq: number) => {
  const data: Rec = { ...rec }
  for (const k of money)
    if (typeof data[k] === 'number') data[k] = fromMajor(data[k])
  return {
    key: `${String(rec.organization_id)}:${String(rec.id)}`,
    orgId: String(rec.organization_id),
    id: String(rec.id),
    seq: BigInt(seq),
    data: JSON.stringify(data),
  }
}

const moduleNodes = (store.flowNodes as Array<Rec>)
  .map((n, i) => asRow(n, ['balance'], i))
  .filter((r) => r.orgId === ORG)
  .map(decodeFlowNode)
const moduleTransactions = (store.transactions as Array<Rec>)
  .map((t, i) => asRow(t, ['amount', 'amount_usd'], i))
  .filter((r) => r.orgId === ORG)
  .map(decodeTransaction)
const proofs = store.verifications as Array<{
  transaction_id: string | null
  status: 'pending' | 'verified' | 'rejected'
  trust_points: number
}>

beforeEach(() => {
  demo.resetDemoStore()
})

describe('the module’s records give the demo’s figures', () => {
  it('lists transactions newest first with their nodes, as the demo does', async () => {
    expect(transactionsWithNodes(moduleTransactions, moduleNodes)).toEqual(
      await demo.getTransactions(ORG),
    )
  })

  it('computes the flow map data the demo API serves, with no dates', async () => {
    expect(
      flowData({
        nodes: moduleNodes,
        transactions: moduleTransactions,
        proofs,
        currency: 'KES',
      }),
    ).toEqual(await demo.getFlowDataAPI(ORG))
  })

  it('computes the flow map data the demo API serves, within dates', async () => {
    const start = '2026-01-10T00:00:00.000Z'
    const end = '2026-02-15'
    const got = flowData({
      nodes: moduleNodes,
      transactions: moduleTransactions,
      proofs,
      currency: 'KES',
      startDate: start,
      endDate: end,
    })
    expect(got).toEqual(await demo.getFlowDataAPI(ORG, start, end))
    expect(got.summary.transaction_count).toBeLessThan(
      moduleTransactions.length,
    )
  })

  it('leaves rejected transactions off the map', () => {
    const rejected: Transaction = {
      ...moduleTransactions[0],
      id: 'rej',
      verification_status: 'rejected',
    }
    const before = flowData({
      nodes: moduleNodes,
      transactions: moduleTransactions,
      proofs,
      currency: 'KES',
    })
    const after = flowData({
      nodes: moduleNodes,
      transactions: [...moduleTransactions, rejected],
      proofs,
      currency: 'KES',
    })
    expect(after).toEqual(before)
  })

  it('gives the demo’s organisation stats', async () => {
    expect(orgStats(moduleTransactions, proofs)).toEqual(
      await demo.getOrganizationStats(ORG),
    )
  })

  it('gives the dashboard the demo’s summary', async () => {
    expect(
      summarise(transactionsWithNodes(moduleTransactions, moduleNodes)),
    ).toEqual(summarise(await demo.getTransactions(ORG)))
  })
})

describe('reducer inputs keep only the fields the Rust types allow', () => {
  it('NewFlowNode: name, type and metadata', () => {
    expect(
      flowNodeInput({
        organization_id: ORG,
        name: 'Kisumu clinic',
        type: 'project',
        metadata: {},
        balance: 5,
      } as Partial<FlowNode>),
    ).toEqual({ name: 'Kisumu clinic', type: 'project', metadata: {} })
  })

  const newTxn = {
    organization_id: ORG,
    from_node_id: 'a',
    to_node_id: null,
    amount: 1234.56,
    currency: 'KES',
    exchange_rate: null,
    amount_usd: 9.5,
    description: 'Fuel',
    category: null,
    reference: 'R1',
    notes: null,
    verification_status: 'pending' as const,
    transaction_date: '2026-10-29',
    created_by: 'u1',
  }

  it('NewTransaction: minor units, no organisation, author or proof count', () => {
    const sent = transactionInput({ ...newTxn, verification_count: 3 })
    expect(sent).toEqual({
      from_node_id: 'a',
      to_node_id: null,
      amount: 123_456,
      currency: 'KES',
      exchange_rate: null,
      amount_usd: 950,
      description: 'Fuel',
      category: null,
      reference: 'R1',
      notes: null,
      verification_status: 'pending',
      transaction_date: '2026-10-29',
    })
  })

  it('TransactionPatch: only what changes, null clears an optional field', () => {
    expect(
      transactionPatch({
        verification_status: 'flagged',
        notes: null,
        verification_count: 2,
        created_by: 'x',
      }),
    ).toEqual({ verification_status: 'flagged', notes: null })
    expect(transactionPatch({ amount: 10 })).toEqual({ amount: 1000 })
  })

  it('NodeVisibilitySettings: can_view never null, the rest nullable', () => {
    expect(
      nodeVisibilitySettings({
        can_view: null as unknown as boolean,
        can_view_amounts: null,
        display_name: 'Partner A',
        node_id: 'n',
        group_id: 'g',
      }),
    ).toEqual({ can_view_amounts: null, display_name: 'Partner A' })
  })

  it('names a removed group on a visibility row', () => {
    const group = store.accessGroups[0] as unknown as AccessGroup
    const rows = nodeVisibilityWithGroups(
      [
        {
          id: 'v1',
          node_id: 'n',
          group_id: group.id,
          can_view: true,
          can_view_amounts: null,
          can_view_proofs: null,
          can_view_executor: null,
          display_name: null,
          created_at: 'x',
        },
        {
          id: 'v2',
          node_id: 'n',
          group_id: 'gone',
          can_view: false,
          can_view_amounts: null,
          can_view_proofs: null,
          can_view_executor: null,
          display_name: null,
          created_at: 'x',
        },
      ],
      [group],
    )
    expect(rows.map((r) => r.group.name)).toEqual([group.name, 'Removed group'])
  })
})

describe('chains', () => {
  const orgs = store.organizations as Array<{
    id: string
    name: string
    trust_score: number
    type: 'NGO'
    country: string
  }>
  const ref = (id: string) => {
    const o = orgs.find((x) => x.id === id)!
    return {
      id: o.id,
      name: o.name,
      logo_key: null,
      trust_score: o.trust_score,
      type: o.type,
      country: o.country,
    }
  }
  // `my_organization_chain` rows as the module writes them (`ChainDetails`, chains.rs:637).
  const rows = (
    store.organizationChains as Array<{
      id: string
      source_org_id: string
      target_org_id: string
    }>
  ).map((c, i) => ({
    id: c.id,
    sourceOrgId: c.source_org_id,
    targetOrgId: c.target_org_id,
    seq: BigInt(i),
    data: JSON.stringify({
      ...c,
      source_org: ref(c.source_org_id),
      target_org: ref(c.target_org_id),
      link_node: null,
      granted_group: null,
    }),
  }))
  const chains = rows.map(decodeChain)

  it('reads the other side’s name and score, with no logo URL', () => {
    const c = chains[0]
    expect(c.target_org).toEqual({
      id: c.target_org_id,
      name: ref(c.target_org_id).name,
      logo_url: null,
      trust_score: ref(c.target_org_id).trust_score,
      type: 'NGO',
      country: ref(c.target_org_id).country,
    })
  })

  it('splits outgoing and incoming as the demo does', async () => {
    const ids = (xs: Array<{ id: string }>) => xs.map((x) => x.id)
    expect(ids(chainsOf(chains, ORG, 'outgoing'))).toEqual(
      ids(await demo.getOutgoingChains(ORG)),
    )
    expect(ids(chainsOf(chains, ORG, 'incoming'))).toEqual(
      ids(await demo.getIncomingChains(ORG)),
    )
  })

  it('counts and scores chains as the demo does', async () => {
    for (const o of orgs)
      expect(chainCounts(chains, o.id)).toEqual(await demo.getChainCounts(o.id))
    for (const c of chains)
      expect(chainTrustScore(c)).toEqual(await demo.getChainTrustScore(c.id))
    expect(chainTrustScore(undefined)).toBeNull()
  })

  it('moves a chain through its states in the counts', () => {
    const [a, b] = [ORG, 'other']
    const base = {
      source_org_id: a,
      target_org_id: b,
    }
    const at = (status: OrganizationChain['status']) => [{ ...base, status }]
    expect(chainCounts(at('pending'), a).outgoing_pending).toBe(1)
    expect(chainCounts(at('pending'), b).incoming_pending).toBe(1)
    expect(chainCounts(at('active'), a).outgoing_active).toBe(1)
    expect(chainCounts(at('active'), b).incoming_active).toBe(1)
    for (const s of ['revoked', 'rejected'] as const)
      expect(chainCounts(at(s), a)).toEqual({
        outgoing_pending: 0,
        outgoing_active: 0,
        incoming_pending: 0,
        incoming_active: 0,
      })
  })

  it('finds published organisations by name, not the caller’s own', async () => {
    const got = matchOrganizations(orgs, 'a', ORG)
    expect(got).toEqual(await demo.searchOrganizations('a', ORG))
    expect(got.every((o) => o.id !== ORG)).toBe(true)
  })
})
