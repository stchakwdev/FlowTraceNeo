import { describe, expect, it } from 'vitest'
import {
  campaignCounts,
  campaignInput,
  campaignPatch,
  campaignReport,
  campaignStats,
  decodeCampaign,
  decodeDonation,
  decodeStat,
  donationInput,
  donationWithDetails,
  donorUpdateInput,
  orgContextFrom,
  organizationFrom,
  organizationPatch,
  resolveCreated,
  underfundedCampaigns,
  withTarget,
} from './transparency-model'
import type { Campaign, Donation, FlowNode, Transaction } from '../types'

// A campaign as the module stores it: amounts in minor units (rust/.../transparency/types.rs).
const storedCampaign = {
  id: 'c1',
  organization_id: 'rw1',
  name: 'Clean water',
  description: null,
  short_description: null,
  image_url: null,
  target_amount: 50_000_000,
  current_amount: 1_250_050,
  currency: 'KES',
  target_node_id: 'n1',
  status: 'active',
  allow_overflow: false,
  overflow_campaign_id: null,
  overflow_consent_message: null,
  starts_at: '2026-01-01T00:00:00.000Z',
  ends_at: '2026-03-01T00:00:00.000Z',
  is_public: true,
  public_slug: 'clean-water',
  category: null,
  tags: null,
  metadata: {},
  created_by: 'u1',
  created_at: '2026-01-01T00:00:00.000Z',
  updated_at: '2026-01-01T00:00:00.000Z',
}

const row = (data: object, seq = 1n, id = (data as { id: string }).id) => ({
  key: `rw1/${id}`,
  orgId: 'rw1',
  id,
  seq,
  data: JSON.stringify(data),
})

const node = (id: string, type: FlowNode['type'], name = id): FlowNode =>
  ({ id, organization_id: 'rw1', name, type }) as FlowNode

describe('money across the module boundary', () => {
  it('reads minor units as major units', () => {
    const c = decodeCampaign(row(storedCampaign))
    expect([c.target_amount, c.current_amount]).toEqual([500_000, 12_500.5])
  })

  it('sends major units as minor units', () => {
    const input = campaignInput({
      ...decodeCampaign(row(storedCampaign)),
      target_amount: 1234.56,
    })
    expect(input.target_amount).toBe(123_456)
  })

  it('converts the four money figures of a stat row', () => {
    const s = decodeStat(
      row(
        {
          campaign_id: 'c1',
          donation_count: 2,
          unique_donors: 2,
          total_raised: 300_000,
          funding_percentage: 0.6,
          avg_donation: 150_000,
          largest_donation: 200_000,
          smallest_donation: 100_000,
        },
        1n,
        'c1',
      ),
    )
    expect([
      s.total_raised,
      s.avg_donation,
      s.largest_donation,
      s.smallest_donation,
    ]).toEqual([3000, 1500, 2000, 1000])
  })

  it('converts a donation amount and its dollar amount, both ways', () => {
    const d = decodeDonation(
      row({
        id: 'd1',
        campaign_id: 'c1',
        amount: 250_000,
        amount_usd: 1_935,
        currency: 'KES',
      }),
    )
    expect([d.amount, d.amount_usd]).toEqual([2500, 19.35])
    expect(donationInput(d)).toMatchObject({
      amount: 250_000,
      amount_usd: 1935,
    })
  })
})

describe('inputs the module refuses unknown fields in', () => {
  it('drops organization_id, created_by and current_amount from a new campaign', () => {
    const input = campaignInput(decodeCampaign(row(storedCampaign)))
    for (const key of [
      'organization_id',
      'created_by',
      'current_amount',
      'id',
      'created_at',
      'updated_at',
    ])
      expect(input).not.toHaveProperty(key)
    expect(input).toMatchObject({
      name: 'Clean water',
      status: 'active',
      public_slug: 'clean-water',
    })
  })

  it('leaves out a null the module would not take for a field that is not nullable', () => {
    const input = campaignInput({
      ...decodeCampaign(row(storedCampaign)),
      metadata: null as unknown as Campaign['metadata'],
    })
    expect(input).not.toHaveProperty('metadata')
  })

  it('keeps only the patch fields of CampaignPatch, nulls included', () => {
    expect(
      campaignPatch({
        status: 'paused',
        ends_at: null,
        current_amount: 99,
        organization_id: 'x',
        target_amount: 10,
      } as Partial<Campaign>),
    ).toEqual({ status: 'paused', ends_at: null, target_amount: 1000 })
  })

  it('keeps only NewDonation fields', () => {
    const input = donationInput({
      campaign_id: 'c1',
      amount: 10,
      currency: 'KES',
      impact_percentage: 4,
      id: 'x',
      payment_status: 'pending',
    } as unknown as Donation)
    expect(input).toEqual({
      campaign_id: 'c1',
      amount: 1000,
      currency: 'KES',
      payment_status: 'pending',
    })
  })

  it('keeps only NewDonorUpdate fields', () => {
    expect(
      donorUpdateInput({
        donation_id: 'd1',
        campaign_id: null,
        update_type: 'thank_you',
        title: 'Thanks',
        message: 'It arrived',
        sent_at: null,
        image_url: null,
        related_node_id: null,
        related_transaction_id: null,
        delivery_method: 'email',
        metadata: {},
      } as never),
    ).toEqual({
      donation_id: 'd1',
      campaign_id: null,
      update_type: 'thank_you',
      title: 'Thanks',
      message: 'It arrived',
      image_url: null,
      related_node_id: null,
      related_transaction_id: null,
      delivery_method: 'email',
      metadata: {},
    })
  })

  it('keeps only the organisation fields the settings form edits', () => {
    expect(
      organizationPatch({
        name: 'RW1 NGO',
        currency: 'UGX',
        public_slug: 'rw1-ngo',
        trust_score: 80,
        id: 'rw1',
      } as never),
    ).toEqual({ name: 'RW1 NGO', currency: 'UGX', public_slug: 'rw1-ngo' })
  })
})

describe('resolving the row a reducer created', () => {
  it('takes the new row that matches what was sent, the highest seq first', () => {
    const before = new Set(['rw1/old'])
    const rows = [
      row({ ...storedCampaign, id: 'old' }, 1n, 'old'),
      row({ ...storedCampaign, id: 'other', name: 'Other' }, 9n, 'other'),
      row({ ...storedCampaign, id: 'a' }, 3n, 'a'),
      row({ ...storedCampaign, id: 'b' }, 5n, 'b'),
    ]
    const found = resolveCreated(
      rows,
      before,
      'rw1',
      (r: Campaign) => r.name === 'Clean water',
    )
    expect(found?.id).toBe('b')
  })

  it('finds nothing before the row arrives', () => {
    expect(resolveCreated([], new Set(), 'rw1', () => true)).toBeUndefined()
  })
})

describe('figures computed in the browser', () => {
  const now = Date.parse('2026-02-04T09:00:00.000Z')

  it('adds the two day counts to the stat row as stats.rs does', () => {
    const c = decodeCampaign(row(storedCampaign))
    const s = campaignStats(c, undefined, now)
    // ceil((Mar 1 - Feb 4 09:00) / day) = 25; floor((Feb 4 09:00 - Jan 1) / day) = 34
    expect(s).toMatchObject({
      days_remaining: 25,
      days_active: 34,
      donation_count: 0,
      total_raised: 0,
    })
  })

  it('has no days remaining for an open-ended campaign', () => {
    const c = { ...decodeCampaign(row(storedCampaign)), ends_at: null }
    expect(campaignStats(c, undefined, now).days_remaining).toBeNull()
  })

  it('counts campaigns and confirmed donations as the demo does', () => {
    const cs = [
      decodeCampaign(row(storedCampaign)),
      {
        ...decodeCampaign(row(storedCampaign)),
        id: 'c2',
        status: 'completed' as const,
        current_amount: 100,
      },
    ]
    const ds = [
      { campaign_id: 'c1', payment_status: 'confirmed' },
      { campaign_id: 'c2', payment_status: 'pending' },
      { campaign_id: 'zz', payment_status: 'confirmed' },
    ] as Array<Donation>
    expect(campaignCounts(cs, ds)).toEqual({
      total_campaigns: 2,
      active_campaigns: 1,
      completed_campaigns: 1,
      total_raised: 12_600.5,
      total_donations: 1,
    })
  })

  it('lists active campaigns under their target, least funded first', () => {
    const base = decodeCampaign(row(storedCampaign))
    const cs = [
      { ...base, id: 'half', target_amount: 100, current_amount: 50 },
      { ...base, id: 'tenth', target_amount: 100, current_amount: 10 },
      { ...base, id: 'full', target_amount: 100, current_amount: 100 },
      { ...base, id: 'draft', status: 'draft' as const, current_amount: 0 },
    ]
    expect(underfundedCampaigns(cs, 5).map((c) => c.id)).toEqual([
      'tenth',
      'half',
    ])
  })

  it('joins the target node and the donation details', () => {
    const c = withTarget(decodeCampaign(row(storedCampaign)), [
      node('n1', 'project', 'Wells'),
    ])
    expect(c.target_node).toEqual({ id: 'n1', name: 'Wells', type: 'project' })
    const d = donationWithDetails(
      { id: 'd1', campaign_id: 'c1', earmarked_node_id: 'n1' } as Donation,
      [c],
      [node('n1', 'project', 'Wells')],
    )
    expect(d.campaign).toEqual({
      id: 'c1',
      name: 'Clean water',
      public_slug: 'clean-water',
      status: 'active',
    })
    expect(d.earmarked_node?.name).toBe('Wells')
  })

  it('builds the campaign report as the demo API does', () => {
    const c = withTarget(decodeCampaign(row(storedCampaign)), [
      node('n1', 'project'),
    ])
    const tx = (
      id: string,
      to: string,
      amount: number,
      status: string,
      date: string,
    ) =>
      ({
        id,
        organization_id: 'rw1',
        from_node_id: 'donor',
        to_node_id: to,
        amount,
        currency: 'KES',
        verification_status: status,
        transaction_date: date,
      }) as Transaction
    const r = campaignReport(
      c,
      { id: 'rw1', name: 'RW1', trust_score: 40, verified: false },
      [node('n1', 'project'), node('n2', 'department'), node('donor', 'donor')],
      [
        tx('t1', 'n1', 20_000, 'verified', '2026-01-03'),
        tx('t2', 'n1', 5_000, 'pending', '2026-01-05'),
        tx('t3', 'n2', 1_000, 'rejected', '2026-01-04'),
      ],
    )
    expect(
      r.earmarks.map((e) => [e.id, e.received, e.evidenced, e.is_target]),
    ).toEqual([
      ['n1', 25_000, 20_000, true],
      ['n2', 0, 0, false],
    ])
    // Capped at the raised amount.
    expect(r.evidenced_amount).toBe(12_500.5)
    expect(r.spending.map((s) => s.id)).toEqual(['t2', 't1'])
  })
})

describe('the organisation from the module', () => {
  it('defaults the name to the id and the currency to the books until the first update', () => {
    const o = organizationFrom('rw1', undefined, 'KES', undefined)
    expect(o).toMatchObject({
      id: 'rw1',
      name: 'rw1',
      currency: 'KES',
      trust_score: 0,
      public_slug: null,
    })
  })

  it('reads the profile and the trust row', () => {
    const o = organizationFrom(
      'rw1',
      JSON.stringify({
        id: 'rw1',
        name: 'RW1 NGO',
        type: 'NGO',
        country: 'KE',
        currency: 'KES',
        verified: false,
        public_slug: 'rw1-ngo',
        logo_key: null,
        is_public: false,
        created_at: 'a',
        updated_at: 'b',
      }),
      'KES',
      JSON.stringify({ score: 62, grade: 'C' }),
    )
    expect(o).toMatchObject({
      name: 'RW1 NGO',
      type: 'NGO',
      public_slug: 'rw1-ngo',
      trust_score: 62,
      updated_at: 'b',
    })
  })
})

describe('the org context from the member row', () => {
  const member = (orgId: string, canClose: boolean, canPost = true) => ({
    orgId,
    actor: {
      id: `u-${orgId}`,
      name: 'Achieng',
      can_post: canPost,
      can_approve: canPost,
      can_close: canClose,
    },
    orgName: null,
  })

  it('takes the configured organisation, else the signed-in one, else the first', () => {
    const memberships = [member('b-org', false), member('a-org', true)]
    expect(
      orgContextFrom({ memberships, configured: undefined, contextOrgId: null })
        ?.orgId,
    ).toBe('a-org')
    expect(
      orgContextFrom({
        memberships,
        configured: undefined,
        contextOrgId: 'b-org',
      })?.orgId,
    ).toBe('b-org')
    expect(
      orgContextFrom({ memberships, configured: 'b-org', contextOrgId: null })
        ?.orgId,
    ).toBe('b-org')
  })

  it('makes a closing member an admin and the others members or viewers', () => {
    const role = (m: ReturnType<typeof member>) =>
      orgContextFrom({
        memberships: [m],
        configured: undefined,
        contextOrgId: null,
      })?.role
    expect([
      role(member('o', true)),
      role(member('o', false)),
      role(member('o', false, false)),
    ]).toEqual(['admin', 'member', 'viewer'])
  })

  it('is null with no membership, so the app shows the not-connected notice', () => {
    expect(
      orgContextFrom({
        memberships: [],
        configured: undefined,
        contextOrgId: 'x',
      }),
    ).toBeNull()
  })
})
