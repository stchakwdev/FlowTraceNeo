import { beforeEach, describe, expect, it } from 'vitest'
import * as data from '../data'
import { DEV_MODE } from '../auth'
import * as demo from './api'
import * as seed from './dataset'
import { DEMO_ORG_ID } from './dataset'

beforeEach(() => {
  demo.resetDemoStore()
})

describe('demo dataset matches the seed', () => {
  it('has the 16 Demo NGO Kenya transactions with their statuses', async () => {
    const txns = await demo.getTransactions(DEMO_ORG_ID)
    expect(txns).toHaveLength(16)
    expect(
      txns.filter((t) => t.verification_status === 'verified'),
    ).toHaveLength(12)
    expect(
      txns.filter((t) => t.verification_status === 'pending'),
    ).toHaveLength(3)
    expect(
      txns.filter((t) => t.verification_status === 'flagged'),
    ).toHaveLength(1)
    // Joined node names, newest first
    expect(txns[0].description).toBe(
      'Pan-African Education Fund - Education Grant',
    )
    expect(txns[0].from_node?.name).toBe('Pan-African Education Fund')
  })

  it('backs every verification_count with proof rows', async () => {
    const txns = await demo.getTransactions(DEMO_ORG_ID)
    for (const t of txns) {
      const proofs = await demo.getVerifications(t.id)
      expect(proofs).toHaveLength(t.verification_count)
    }
  })

  it('resolves a public campaign by slug with its stats', async () => {
    const campaign = await demo.getCampaignBySlug('clean-water-turkana')
    expect(campaign.name).toBe('Clean Water for Turkana')
    expect(campaign.target_node?.name).toBe('Clean Water Turkana')
    const stats = await demo.getCampaignStats(campaign.id)
    expect(stats.donation_count).toBe(11)
    expect(stats.total_raised).toBe(3200000)
  })

  it('reconciles every campaign raised amount with its confirmed donations', async () => {
    const campaigns = await demo.getCampaigns(DEMO_ORG_ID)
    expect(campaigns.length).toBeGreaterThan(0)
    for (const c of campaigns) {
      const ledger = seed.donations
        .filter(
          (d) => d.campaign_id === c.id && d.payment_status === 'confirmed',
        )
        .reduce((s, d) => s + d.amount, 0)
      expect(ledger, c.name).toBe(c.current_amount)
      const stats = await demo.getCampaignStats(c.id)
      expect(stats.total_raised, c.name).toBe(c.current_amount)
    }
  })

  it('rejects slugs for draft or unknown campaigns', async () => {
    await expect(demo.getCampaignBySlug('does-not-exist')).rejects.toThrow()
  })

  it('builds flow data with edges between seeded nodes', async () => {
    const flow = await demo.getFlowDataAPI(DEMO_ORG_ID)
    expect(flow.summary.transaction_count).toBe(16)
    // The flagged withdrawal has no destination, so it adds no edge.
    expect(flow.summary.total_edges).toBe(15)
    const harambee = flow.nodes.find((n) => n.name === 'Harambee Health Fund')
    expect(harambee?.outflow).toBe(8500000)
  })
})

describe('demo mutations', () => {
  it('shows a created transaction on the next read', async () => {
    const before = await demo.getTransactions(DEMO_ORG_ID)
    const created = await demo.createTransaction({
      organization_id: DEMO_ORG_ID,
      from_node_id: before[0].from_node_id,
      to_node_id: before[0].to_node_id,
      amount: 12345,
      currency: 'KES',
      exchange_rate: null,
      amount_usd: null,
      description: 'Demo test payment',
      category: 'Operations',
      reference: null,
      notes: null,
      verification_status: 'pending',
      transaction_date: '2026-02-10',
      created_by: null,
    })
    const after = await demo.getTransactions(DEMO_ORG_ID)
    expect(after).toHaveLength(before.length + 1)
    expect(after.find((t) => t.id === created.id)?.description).toBe(
      'Demo test payment',
    )
    const stats = await demo.getOrganizationStats(DEMO_ORG_ID)
    expect(stats.total).toBe(17)
  })

  it('auto-verifies a pending transaction at two proofs', async () => {
    const id = 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b14'
    const base = {
      transaction_id: id,
      file_url: null,
      ocr_data: null,
      metadata: {},
      status: 'verified' as const,
      verified_by: null,
      visibility_level: 'full' as const,
      thumbnail_url: null,
      executor_name: null,
      gps_latitude: null,
      gps_longitude: null,
      gps_accuracy: null,
      location_name: null,
      description: null,
      created_by: null,
    }
    await demo.createVerification({ ...base, type: 'receipt' })
    await demo.createVerification({ ...base, type: 'photo' })
    const txn = await demo.getTransaction(id, DEMO_ORG_ID)
    expect(txn.verification_count).toBe(2)
    expect(txn.verification_status).toBe('verified')
  })
})

describe('data.ts', () => {
  it('serves the demo implementation in dummy auth mode', () => {
    expect(DEV_MODE).toBe(true)
    expect(data.getTransactions).toBe(demo.getTransactions)
    expect(data.getFlowDataAPI).toBe(demo.getFlowDataAPI)
    expect(data.getCampaignBySlug).toBe(demo.getCampaignBySlug)
  })
})
