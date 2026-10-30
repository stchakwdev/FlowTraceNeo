import { beforeEach, describe, expect, it, vi } from 'vitest'

// The RW3 reads and writes over a fake connection, as spacetime-transparency.test.ts does: views
// are arrays with the SDK's callbacks, reducers are spies, and a test applies a reducer's effect
// the way the host's transaction would.
const fake = vi.hoisted(() => {
  type Cb = (...args: Array<unknown>) => void
  const makeView = () => {
    const rows: Array<Record<string, unknown>> = []
    const listeners = { insert: new Set<Cb>(), update: new Set<Cb>() }
    return {
      rows,
      iter: () => rows,
      onInsert: (cb: Cb) => listeners.insert.add(cb),
      removeOnInsert: (cb: Cb) => listeners.insert.delete(cb),
      onUpdate: (cb: Cb) => listeners.update.add(cb),
      removeOnUpdate: (cb: Cb) => listeners.update.delete(cb),
      onDelete: () => {},
      removeOnDelete: () => {},
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
    createVerification: vi.fn(
      (_args: { orgId: string; verificationJson: string }) => Promise.resolve(),
    ),
    inviteAccessGroupMember: vi.fn(
      (_args: {
        orgId: string
        groupId: string
        email: string
        validUntil?: string
      }) => Promise.resolve(),
    ),
    updatePublicEmbedConfig: vi.fn(
      (_args: { orgId: string; patchJson: string }) => Promise.resolve(),
    ),
  }
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
        subscribe: () => queueMicrotask(applied),
      }
      return b
    },
  }
  return { views, viewOf, reducers, conn }
})

vi.mock('./spacetime-backend', () => ({
  connectSpacetime: () => Promise.resolve(fake.conn),
  view: (_conn: unknown, name: string) => fake.viewOf(name),
  spacetimeMemberships: () =>
    Promise.resolve({ identity: 'c200me', memberships: [] }),
}))

const t = await import('./spacetime-transparency')

const SHA = 'b'.repeat(64)
const KEY = `org/rw1/${SHA}`

const doc = (id: string, seq: bigint, data: Record<string, unknown>) => ({
  key: `rw1:${id}`,
  orgId: 'rw1',
  id,
  seq,
  data: JSON.stringify({ id, ...data }),
})

beforeEach(() => {
  fake.views.clear()
  for (const r of Object.values(fake.reducers))
    r.mockReset().mockResolvedValue(undefined)
})

describe('proofs from my_verification', () => {
  it('lists a transaction’s proofs as far as the reader may see them, newest first', async () => {
    const v = fake.viewOf('verification')
    v.rows.push(
      doc('v1', 1n, {
        transaction_id: 'tx-1',
        type: 'receipt',
        object_key: KEY,
        created_at: '2026-10-01',
        visible_as: 'full',
      }),
      doc('v2', 2n, {
        transaction_id: 'tx-1',
        type: 'photo',
        object_key: null,
        thumbnail_key: null,
        created_at: '2026-10-02',
        visible_as: 'summary',
      }),
      doc('v3', 3n, { transaction_id: 'tx-2', type: 'receipt', created_at: '2026-10-03' }),
    )
    const out = await t.getVerifications('tx-1')
    expect(out.map((p) => [p.id, p.file_url, p.visible_as])).toEqual([
      ['v2', null, 'summary'],
      ['v1', `proof:${KEY}`, 'full'],
    ])
  })

  it('records a proof after the Worker upload and resolves it from the view', async () => {
    fake.viewOf('transaction').rows.push(doc('tx-1', 1n, { amount: 100 }))
    fake.reducers.createVerification.mockImplementation(() => {
      fake.viewOf('verification').insert(
        doc('v9', 9n, {
          transaction_id: 'tx-1',
          type: 'receipt',
          object_key: KEY,
          created_at: '2026-10-30',
        }),
      )
      return Promise.resolve()
    })
    const made = await t.createVerification({
      transaction_id: 'tx-1',
      type: 'receipt',
      file_url: `proof:${KEY}`,
      ocr_data: null,
      metadata: { file_name: 'r.jpg' },
      status: 'pending',
      verified_by: null,
      visibility_level: 'full',
      thumbnail_url: null,
      executor_name: null,
      gps_latitude: null,
      gps_longitude: null,
      gps_accuracy: null,
      location_name: null,
      description: 'Receipt',
      created_by: null,
    })
    const sent = fake.reducers.createVerification.mock.calls[0][0]
    expect(sent.orgId).toBe('rw1')
    expect(JSON.parse(sent.verificationJson)).toMatchObject({
      transaction_id: 'tx-1',
      object_key: KEY,
      sha256: SHA,
    })
    expect(made.id).toBe('v9')
  })
})

describe('the trust page reads the module row and its history', () => {
  it('reads my_trust_score and my_trust_score_history', async () => {
    fake.viewOf('trustScore').rows.push({
      orgId: 'rw1',
      data: JSON.stringify({
        score: 64,
        grade: 'C',
        components: { verification: 40, documentation: 75, consistency: 80, model: 60 },
        weights: { verification: 32, documentation: 24, consistency: 24, model: 20 },
        computed_at: '2026-10-30T10:00:00Z',
      }),
    })
    const h = fake.viewOf('trustScoreHistory')
    h.rows.push(
      doc('h2', 2n, { score: 64, calculated_at: '2026-10-30T10:00:00Z' }),
      doc('h1', 1n, { score: 58, calculated_at: '2026-10-29T10:00:00Z' }),
    )
    const score = await t.getTrustScore('rw1')
    expect(score?.score).toBe(64)
    expect(score?.components.find((c) => c.key === 'model')).toEqual({
      key: 'model',
      value: 60,
      weight: 20,
    })
    const history = await t.getTrustScoreHistory('rw1')
    expect(history.map((x) => x.score)).toEqual([58, 64])
    expect(await t.getTrustScore('other-org')).toBeNull()
  })
})

describe('access group members', () => {
  it('invites by email and names members from my_member', async () => {
    fake.viewOf('accessGroup').rows.push(
      doc('g1', 1n, { name: 'Auditors', slug: 'auditors', priority: 40 }),
    )
    fake.viewOf('member').rows.push({ orgId: 'rw1', userId: 'u-b', name: 'Baraka' })
    fake.viewOf('accessGroupMember').rows.push(
      doc('m1', 1n, { group_id: 'g1', user_id: 'u-b', user_email: null, created_at: '2026-10-01' }),
    )
    fake.reducers.inviteAccessGroupMember.mockImplementation(() => {
      fake.viewOf('accessGroupMember').insert(
        doc('m2', 2n, {
          group_id: 'g1',
          user_id: null,
          user_email: 'auditor@example.org',
          notes: 'Invited by email. No email was sent.',
          created_at: '2026-10-30',
        }),
      )
      return Promise.resolve()
    })
    const invited = await t.inviteMemberByEmail('g1', 'auditor@example.org', 'ignored')
    expect(fake.reducers.inviteAccessGroupMember).toHaveBeenCalledWith({
      orgId: 'rw1',
      groupId: 'g1',
      email: 'auditor@example.org',
      validUntil: undefined,
    })
    expect(invited.notes).toBe('Invited by email. No email was sent.')
    const members = await t.getGroupMembers('g1')
    expect(members.map((m) => m.profile?.full_name ?? m.user_email)).toEqual([
      'auditor@example.org',
      'Baraka',
    ])
  })
})

describe('embed settings', () => {
  it('sends only the changed fields and resolves the saved settings', async () => {
    const view = fake.viewOf('publicEmbedConfig')
    view.rows.push({
      orgId: 'rw1',
      data: JSON.stringify({ show_amounts: true, embed_token: 't1', updated_at: 'a' }),
    })
    fake.reducers.updatePublicEmbedConfig.mockImplementation(() => {
      view.rows[0] = {
        orgId: 'rw1',
        data: JSON.stringify({ show_amounts: false, embed_token: 't1', updated_at: 'b' }),
      }
      return Promise.resolve()
    })
    const saved = await t.updatePublicEmbedConfig('rw1', { show_amounts: false })
    expect(
      JSON.parse(fake.reducers.updatePublicEmbedConfig.mock.calls[0][0].patchJson),
    ).toEqual({ show_amounts: false })
    expect(saved.show_amounts).toBe(false)
  })
})
