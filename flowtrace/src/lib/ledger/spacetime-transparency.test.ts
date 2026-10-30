import { beforeEach, describe, expect, it, vi } from 'vitest'

// A connection whose views are plain arrays with the SDK's callbacks, and whose reducers are
// spies. A reducer's effect is applied by the test, as the host's transaction would be.
const fake = vi.hoisted(() => {
  type Cb = (...args: Array<unknown>) => void
  const makeView = () => {
    const rows: Array<Record<string, unknown>> = []
    const listeners = {
      insert: new Set<Cb>(),
      update: new Set<Cb>(),
      delete: new Set<Cb>(),
    }
    return {
      rows,
      listeners,
      iter: () => rows,
      onInsert: (cb: Cb) => listeners.insert.add(cb),
      removeOnInsert: (cb: Cb) => listeners.insert.delete(cb),
      onUpdate: (cb: Cb) => listeners.update.add(cb),
      removeOnUpdate: (cb: Cb) => listeners.update.delete(cb),
      onDelete: (cb: Cb) => listeners.delete.add(cb),
      removeOnDelete: (cb: Cb) => listeners.delete.delete(cb),
      insert(row: Record<string, unknown>) {
        rows.push(row)
        for (const cb of listeners.insert) cb(null, row)
      },
    }
  }
  const views = new Map<string, ReturnType<typeof makeView>>()
  const viewOf = (name: string) => {
    let v = views.get(name)
    if (!v) {
      v = makeView()
      views.set(name, v)
    }
    return v
  }
  const reducers = {
    createCampaign: vi.fn((_args: { orgId: string; campaignJson: string }) =>
      Promise.resolve(),
    ),
    updateOrganizationProfile: vi.fn(
      (_args: { orgId: string; patchJson: string }) => Promise.resolve(),
    ),
    recordDonation: vi.fn((_args: { orgId: string; donationJson: string }) =>
      Promise.resolve(),
    ),
  }
  const subscribed: Array<Array<string>> = []
  const conn = {
    reducers,
    subscriptionBuilder: () => {
      let applied = () => {}
      const b = {
        onApplied: (cb: () => void) => {
          applied = cb
          return b
        },
        onError: () => b,
        subscribe: (qs: Array<string>) => {
          subscribed.push(qs)
          queueMicrotask(applied)
        },
      }
      return b
    },
  }
  return { views, viewOf, reducers, conn, subscribed }
})

vi.mock('./spacetime-backend', () => ({
  connectSpacetime: () => Promise.resolve(fake.conn),
  view: (_conn: unknown, name: string) => fake.viewOf(name),
  spacetimeMemberships: () =>
    Promise.resolve({
      identity: 'c200me',
      memberships: [
        {
          orgId: 'rw1',
          actor: {
            id: 'u1',
            name: 'Achieng',
            can_post: true,
            can_approve: true,
            can_close: true,
          },
          orgName: null,
        },
      ],
    }),
}))

const t = await import('./spacetime-transparency')

const campaignRow = (
  id: string,
  seq: bigint,
  over: Record<string, unknown> = {},
) => ({
  key: `rw1:${id}`,
  orgId: 'rw1',
  id,
  seq,
  data: JSON.stringify({
    id,
    organization_id: 'rw1',
    name: 'Clean water',
    target_amount: 50_000_000,
    current_amount: 0,
    currency: 'KES',
    status: 'draft',
    starts_at: '2026-01-01',
    created_at: 'x',
    updated_at: 'x',
    ...over,
  }),
})

const newCampaign = {
  organization_id: 'rw1',
  name: 'Clean water',
  description: null,
  short_description: null,
  image_url: null,
  target_amount: 500_000,
  currency: 'KES',
  target_node_id: null,
  status: 'draft' as const,
  allow_overflow: false,
  overflow_campaign_id: null,
  overflow_consent_message: null,
  starts_at: '2026-01-01',
  ends_at: null,
  is_public: false,
  public_slug: null,
  category: null,
  tags: null,
  metadata: {},
  created_by: 'u1',
}

beforeEach(() => {
  fake.views.clear()
  for (const r of Object.values(fake.reducers))
    r.mockReset().mockResolvedValue(undefined)
})

describe('one subscription to the caller’s views', () => {
  it('reads every transparency view the module limits to the caller', async () => {
    await t.getCampaigns('rw1')
    expect(fake.subscribed.at(0)).toEqual(
      expect.arrayContaining([
        'SELECT * FROM my_campaign',
        'SELECT * FROM my_campaign_stat',
        'SELECT * FROM my_donation',
        'SELECT * FROM my_flow_node',
        'SELECT * FROM my_organization_profile',
        'SELECT * FROM my_trust_score',
        'SELECT * FROM my_profile',
      ]),
    )
  })
})

describe('createCampaign', () => {
  it('sends the NewCampaign JSON in minor units, without the fields the module refuses', async () => {
    fake.reducers.createCampaign.mockImplementation(() => {
      fake.viewOf('campaign').insert(campaignRow('c-new', 7n))
      return Promise.resolve()
    })
    await t.createCampaign(newCampaign)
    const args = fake.reducers.createCampaign.mock.calls[0][0]
    const sent = JSON.parse(args.campaignJson) as Record<string, unknown>
    expect(args.orgId).toBe('rw1')
    expect(sent.target_amount).toBe(50_000_000)
    expect(sent).not.toHaveProperty('organization_id')
    expect(sent).not.toHaveProperty('created_by')
  })

  it('resolves the row the reducer created, in major units, even when it arrives after the reply', async () => {
    fake.viewOf('campaign').insert(campaignRow('c-old', 1n))
    fake.reducers.createCampaign.mockImplementation(() => {
      setTimeout(
        () => fake.viewOf('campaign').insert(campaignRow('c-new', 8n)),
        5,
      )
      return Promise.resolve()
    })
    const created = await t.createCampaign(newCampaign)
    expect(created).toMatchObject({ id: 'c-new', target_amount: 500_000 })
  })

  it('rejects with the module’s own refusal text', async () => {
    fake.reducers.createCampaign.mockRejectedValue(
      new Error('You are not a member of organisation rw1.'),
    )
    await expect(t.createCampaign(newCampaign)).rejects.toThrow(
      'You are not a member of organisation rw1.',
    )
  })
})

describe('the organisation profile', () => {
  const books = {
    orgId: 'rw1',
    settingsJson: JSON.stringify({ base_currency: 'KES' }),
  }

  it('defaults the name to the id and the currency to the books before the first update', async () => {
    fake.viewOf('ledgerOrg').insert(books)
    const snap = await t.orgSnapshot()
    expect(snap.organizations.get('rw1')).toMatchObject({
      name: 'rw1',
      currency: 'KES',
    })
    expect(snap.person).toBeNull()
  })

  it('sends only the profile fields and resolves the updated profile', async () => {
    fake.viewOf('ledgerOrg').insert(books)
    fake.reducers.updateOrganizationProfile.mockImplementation(() => {
      fake.viewOf('organizationProfile').insert({
        orgId: 'rw1',
        data: JSON.stringify({
          id: 'rw1',
          name: 'RW1 NGO',
          currency: 'UGX',
          country: 'UG',
          public_slug: 'rw1',
          updated_at: 'now',
        }),
      })
      return Promise.resolve()
    })
    const org = await t.updateOrganization('rw1', {
      name: 'RW1 NGO',
      currency: 'UGX',
      public_slug: 'rw1',
      trust_score: 9,
    } as never)
    expect(
      JSON.parse(
        fake.reducers.updateOrganizationProfile.mock.calls[0][0].patchJson,
      ),
    ).toEqual({
      name: 'RW1 NGO',
      currency: 'UGX',
      public_slug: 'rw1',
    })
    expect(org).toMatchObject({
      name: 'RW1 NGO',
      currency: 'UGX',
      public_slug: 'rw1',
    })
  })

  it('shows a taken link with the module’s text', async () => {
    fake.reducers.updateOrganizationProfile.mockRejectedValue(
      new Error('That public link is already taken. Choose another.'),
    )
    await expect(
      t.updateOrganization('rw1', { public_slug: 'taken' }),
    ).rejects.toThrow('That public link is already taken. Choose another.')
  })
})

describe('createDonation', () => {
  it('records a member’s gift in minor units and resolves it from my_donation', async () => {
    fake.viewOf('campaign').insert(campaignRow('c1', 1n))
    fake.reducers.recordDonation.mockImplementation((args) => {
      const sent = JSON.parse(args.donationJson) as Record<string, unknown>
      fake.viewOf('donation').insert({
        key: 'rw1:d1',
        orgId: 'rw1',
        id: 'd1',
        seq: 2n,
        data: JSON.stringify({
          ...sent,
          id: 'd1',
          created_at: 'n',
          updated_at: 'n',
        }),
      })
      return Promise.resolve()
    })
    const d = await t.createDonation({
      campaign_id: 'c1',
      amount: 2500,
      currency: 'KES',
      payment_status: 'pending',
      is_anonymous: false,
    } as never)
    expect(
      JSON.parse(fake.reducers.recordDonation.mock.calls[0][0].donationJson),
    ).toMatchObject({ amount: 250_000 })
    expect(d).toMatchObject({ id: 'd1', amount: 2500 })
  })
})
