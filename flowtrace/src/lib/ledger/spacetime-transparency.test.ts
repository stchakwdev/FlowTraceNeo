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
    createTransaction: vi.fn(
      (_args: { orgId: string; transactionJson: string }) => Promise.resolve(),
    ),
    createFlowNode: vi.fn((_args: { orgId: string; nodeJson: string }) =>
      Promise.resolve(),
    ),
    setNodeVisibility: vi.fn(
      (_args: {
        orgId: string
        nodeId: string
        groupId: string
        settingsJson: string
      }) => Promise.resolve(),
    ),
    requestChain: vi.fn(
      (_args: { sourceOrgId: string; targetOrgId: string; message?: string }) =>
        Promise.resolve(),
    ),
    approveChain: vi.fn(
      (_args: {
        chainId: string
        grantedGroupId: string
        linkNodeId?: string
      }) => Promise.resolve(),
    ),
    revokeChain: vi.fn((_args: { chainId: string }) => Promise.resolve()),
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
        'SELECT * FROM my_transaction',
        'SELECT * FROM my_node_visibility',
        'SELECT * FROM my_verification',
        'SELECT * FROM my_access_group',
        'SELECT * FROM my_organization_chain',
        'SELECT * FROM my_chain_activity_log',
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

describe('transactions and flow nodes', () => {
  const docRow = (view: string, id: string, seq: bigint, data: object) =>
    fake.viewOf(view).insert({
      key: `rw1:${id}`,
      orgId: 'rw1',
      id,
      seq,
      data: JSON.stringify({ id, organization_id: 'rw1', ...data }),
    })

  it('records a transaction in minor units and resolves it from my_transaction', async () => {
    fake.reducers.createTransaction.mockImplementation((args) => {
      const sent = JSON.parse(args.transactionJson) as Record<string, unknown>
      docRow('transaction', 't1', 3n, {
        ...sent,
        verification_count: 0,
        created_by: 'u1',
        created_at: 'n',
        updated_at: 'n',
      })
      return Promise.resolve()
    })
    const txn = await t.createTransaction({
      organization_id: 'rw1',
      from_node_id: 'n1',
      to_node_id: 'n2',
      amount: 1500.25,
      currency: 'KES',
      exchange_rate: null,
      amount_usd: null,
      description: 'Borehole pump',
      category: null,
      reference: null,
      notes: null,
      verification_status: 'pending',
      transaction_date: '2026-10-29',
      created_by: 'u1',
    })
    const args = fake.reducers.createTransaction.mock.calls[0][0]
    const sent = JSON.parse(args.transactionJson) as Record<string, unknown>
    expect(args.orgId).toBe('rw1')
    expect(sent.amount).toBe(150_025)
    expect(sent).not.toHaveProperty('organization_id')
    expect(sent).not.toHaveProperty('created_by')
    expect(txn).toMatchObject({ id: 't1', amount: 1500.25 })
  })

  it('lists transactions with their node names', async () => {
    docRow('flowNode', 'n1', 1n, { name: 'Donor A', type: 'donor', balance: 0 })
    docRow('flowNode', 'n2', 2n, {
      name: 'Clinic',
      type: 'project',
      balance: 0,
    })
    docRow('transaction', 't1', 3n, {
      from_node_id: 'n1',
      to_node_id: 'n2',
      amount: 10_000,
      transaction_date: '2026-10-29',
    })
    const [row] = await t.getTransactions('rw1')
    expect(row).toMatchObject({
      amount: 100,
      from_node: { id: 'n1', name: 'Donor A', type: 'donor' },
      to_node: { id: 'n2', name: 'Clinic', type: 'project' },
    })
  })

  it('creates a node with only the NewFlowNode fields', async () => {
    fake.reducers.createFlowNode.mockImplementation(() => {
      docRow('flowNode', 'n9', 9n, {
        name: 'Kisumu clinic',
        type: 'project',
        balance: 0,
        metadata: {},
        created_at: 'n',
      })
      return Promise.resolve()
    })
    const n = await t.createNode({
      organization_id: 'rw1',
      name: 'Kisumu clinic',
      type: 'project',
      metadata: {},
    })
    expect(
      JSON.parse(fake.reducers.createFlowNode.mock.calls[0][0].nodeJson),
    ).toEqual({ name: 'Kisumu clinic', type: 'project', metadata: {} })
    expect(n.id).toBe('n9')
  })

  it('sets a node visibility on the node’s organisation and resolves the row', async () => {
    docRow('flowNode', 'n1', 1n, { name: 'Donor A', type: 'donor', balance: 0 })
    fake.reducers.setNodeVisibility.mockImplementation((args) => {
      docRow('nodeVisibility', 'v1', 2n, {
        node_id: args.nodeId,
        group_id: args.groupId,
        can_view: true,
        can_view_amounts: false,
        can_view_proofs: null,
        can_view_executor: null,
        display_name: null,
        created_at: 'n',
      })
      return Promise.resolve()
    })
    const v = await t.setNodeVisibility('n1', 'g1', { can_view_amounts: false })
    expect(fake.reducers.setNodeVisibility.mock.calls[0][0]).toMatchObject({
      orgId: 'rw1',
      nodeId: 'n1',
      groupId: 'g1',
    })
    expect(v).toMatchObject({ group_id: 'g1', can_view_amounts: false })
  })

  it('shows a manager-only refusal with the module’s text', async () => {
    docRow('flowNode', 'n1', 1n, { name: 'Donor A', type: 'donor', balance: 0 })
    fake.reducers.setNodeVisibility.mockRejectedValue(
      new Error(
        'Only members who can close periods can change what access groups see.',
      ),
    )
    await expect(t.setNodeVisibility('n1', 'g1', {})).rejects.toThrow(
      'Only members who can close periods can change what access groups see.',
    )
  })
})

describe('a chain from request to revocation', () => {
  const org = (id: string, name: string, score: number) => ({
    id,
    name,
    logo_key: null,
    trust_score: score,
    type: 'NGO',
    country: 'KE',
  })
  const put = (status: string, updated: string, seq: bigint) => {
    const v = fake.viewOf('organizationChain')
    const rec = {
      id: 'ch1',
      sourceOrgId: 'rw1',
      targetOrgId: 'rw2',
      seq,
      data: JSON.stringify({
        id: 'ch1',
        source_org_id: 'rw1',
        target_org_id: 'rw2',
        status,
        requested_at: '2026-10-29T10:00:00.000Z',
        updated_at: updated,
        source_org: org('rw1', 'RW1 Water Trust', 24),
        target_org: org('rw2', 'RW2 Partner', 40),
        link_node: null,
        granted_group: null,
      }),
    }
    const at = v.rows.findIndex((r) => r.id === 'ch1')
    if (at >= 0) {
      const old = v.rows[at]
      v.rows[at] = rec
      for (const cb of v.listeners.update) cb(null, old, rec)
    } else v.insert(rec)
  }

  it('resolves each step from my_organization_chain and scores the pair', async () => {
    fake.reducers.requestChain.mockImplementation(() => {
      put('pending', 'a', 1n)
      return Promise.resolve()
    })
    const asked = await t.requestChain(
      'rw1',
      'rw2',
      'ignored',
      'Link our wells',
    )
    expect(fake.reducers.requestChain.mock.calls[0][0]).toEqual({
      sourceOrgId: 'rw1',
      targetOrgId: 'rw2',
      message: 'Link our wells',
    })
    expect(asked.status).toBe('pending')
    expect(await t.getChainCounts('rw2')).toMatchObject({ incoming_pending: 1 })

    fake.reducers.approveChain.mockImplementation(() => {
      setTimeout(() => put('active', 'b', 2n), 5)
      return Promise.resolve()
    })
    const approved = await t.approveChain('ch1', 'ignored', 'g1')
    expect(fake.reducers.approveChain.mock.calls[0][0]).toEqual({
      chainId: 'ch1',
      grantedGroupId: 'g1',
      linkNodeId: undefined,
    })
    expect(approved.status).toBe('active')
    const [incoming] = await t.getIncomingChains('rw2')
    expect(incoming.source_org).toMatchObject({
      name: 'RW1 Water Trust',
      trust_score: 24,
      logo_url: null,
    })
    expect(await t.getChainTrustScore('ch1')).toEqual({
      aggregated_score: 32,
      org_count: 2,
      scores: { source_org: 24, target_org: 40 },
    })

    fake.reducers.revokeChain.mockImplementation(() => {
      put('revoked', 'c', 3n)
      return Promise.resolve()
    })
    expect((await t.revokeChain('ch1', 'ignored')).status).toBe('revoked')
    expect(await t.getChainCounts('rw1')).toEqual({
      outgoing_pending: 0,
      outgoing_active: 0,
      incoming_pending: 0,
      incoming_active: 0,
    })
  })

  it('shows the module’s refusal when a non-manager approves', async () => {
    put('pending', 'a', 1n)
    fake.reducers.approveChain.mockRejectedValue(
      new Error(
        "Only members who can close periods can answer for their organisation's chains.",
      ),
    )
    await expect(t.approveChain('ch1', 'x', 'g1')).rejects.toThrow(
      "Only members who can close periods can answer for their organisation's chains.",
    )
  })
})
