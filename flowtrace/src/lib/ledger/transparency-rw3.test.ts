import { describe, expect, it } from 'vitest'
import {
  accessGroupInput,
  accessGroupPatch,
  decodeVerification,
  embedConfigPatch,
  groupMemberInput,
  groupMembersWithProfiles,
  organizationFrom,
  organizationPatch,
  trustHistoryOf,
  trustSummaryFrom,
  verificationInput,
} from './transparency-model'
import type {
  AccessGroup,
  GroupMember,
  TrustScoreHistory,
  Verification,
} from '../types'

const SHA = 'a'.repeat(64)
const KEY = `org/rw1/${SHA}`

// The fields each Rust input type has (types.rs:654-800, flows.rs:443); every one is
// `deny_unknown_fields`, so nothing else may be sent.
const NEW_VERIFICATION_FIELDS = [
  'transaction_id',
  'claim_id',
  'bill_id',
  'type',
  'object_key',
  'sha256',
  'thumbnail_key',
  'ocr_data',
  'metadata',
  'status',
  'visibility_level',
  'executor_name',
  'gps_latitude',
  'gps_longitude',
  'gps_accuracy',
  'location_name',
  'description',
]
const ACCESS_GROUP_FIELDS = [
  'name',
  'slug',
  'description',
  'icon',
  'color',
  'permissions',
  'node_scope',
  'is_default',
  'is_public',
  'priority',
]
const PERMISSIONS = [
  'can_view_nodes',
  'can_view_amounts',
  'can_view_proofs',
  'can_view_executor',
  'can_view_full_proofs',
  'can_edit_nodes',
  'can_edit_transactions',
  'can_manage_members',
  'can_manage_groups',
]
const EMBED_PATCH_FIELDS = [
  'display_mode',
  'show_amounts',
  'amount_format',
  'show_node_names',
  'show_trust_score',
  'show_transaction_count',
  'show_last_updated',
  'visible_node_types',
  'hidden_node_ids',
  'theme',
  'allow_embed',
  'allowed_domains',
]

const onlyFrom = (obj: object, allowed: Array<string>) =>
  Object.keys(obj).filter((k) => !allowed.includes(k))

function webProof(
  over: Partial<Verification> = {},
): Omit<Verification, 'id' | 'created_at' | 'verified_at' | 'trust_points'> {
  return {
    transaction_id: 'tx-1',
    type: 'receipt',
    file_url: `proof:${KEY}`,
    ocr_data: null,
    metadata: { file_name: 'r.jpg', object_key: KEY, sha256: SHA },
    status: 'pending',
    verified_by: 'someone-else',
    visibility_level: 'full',
    thumbnail_url: `proof:${KEY}.thumb`,
    executor_name: null,
    gps_latitude: null,
    gps_longitude: null,
    gps_accuracy: null,
    location_name: null,
    description: 'Cement receipt',
    created_by: 'someone-else',
    ...over,
  }
}

describe('proof inputs (NewVerification, types.rs:743)', () => {
  it('sends the Worker object key and its hash, and nothing the module sets itself', () => {
    const input = verificationInput(webProof())
    expect(onlyFrom(input, NEW_VERIFICATION_FIELDS)).toEqual([])
    expect(input.object_key).toBe(KEY)
    expect(input.sha256).toBe(SHA)
    expect(input.thumbnail_key).toBe(`${KEY}.thumb`)
    expect(input).not.toHaveProperty('file_url')
    expect(input).not.toHaveProperty('created_by')
    expect(input).not.toHaveProperty('verified_by')
  })

  it('takes the hash from the key when the metadata has none', () => {
    const input = verificationInput(
      webProof({ metadata: {}, thumbnail_url: null }),
    )
    expect(input.sha256).toBe(SHA)
    expect(input).not.toHaveProperty('thumbnail_key')
  })

  it('records a proof with no file, such as a GPS reading, without keys', () => {
    const input = verificationInput(
      webProof({ file_url: null, thumbnail_url: null, metadata: {}, gps_latitude: -1.28 }),
    )
    expect(input).not.toHaveProperty('object_key')
    expect(input).not.toHaveProperty('sha256')
    expect(input.gps_latitude).toBe(-1.28)
  })

  it('refuses a file that only lives in this browser instead of dropping it', () => {
    expect(() =>
      verificationInput(webProof({ file_url: 'blob:http://x/1', metadata: {} })),
    ).toThrow(/VITE_PROOFS_URL/)
  })

  it('reads a view row back with proof references and the reader level', () => {
    const v = decodeVerification({
      data: JSON.stringify({
        id: 'v1',
        transaction_id: 'tx-1',
        type: 'receipt',
        object_key: null,
        thumbnail_key: `${KEY}.thumb`,
        visibility_level: 'full',
        visible_as: 'thumbnail',
        published: true,
      }),
    })
    expect(v.file_url).toBeNull()
    expect(v.thumbnail_url).toBe(`proof:${KEY}.thumb`)
    expect(v.visible_as).toBe('thumbnail')
    expect(v.published).toBe(true)
  })
})

describe('access group and member inputs (types.rs:654-740)', () => {
  const group: Partial<AccessGroup> & Record<string, unknown> = {
    id: 'g1',
    organization_id: 'rw1',
    name: 'Auditors',
    slug: 'auditors',
    description: null,
    icon: 'eye',
    color: '#336699',
    permissions: {
      can_view_nodes: true,
      can_view_amounts: true,
      can_view_proofs: true,
      can_view_executor: false,
      can_view_full_proofs: true,
      can_edit_nodes: false,
      can_edit_transactions: false,
      can_manage_members: false,
      can_manage_groups: false,
      // A stray flag from an older form must not reach the module.
      ...({ can_export: true } as object),
    },
    node_scope: 'all',
    is_default: false,
    is_public: false,
    priority: 40,
    created_at: 'x',
    updated_at: 'x',
  }

  it('keeps only the group fields and the nine permission flags', () => {
    const input = accessGroupInput(group)
    expect(onlyFrom(input, ACCESS_GROUP_FIELDS)).toEqual([])
    expect(Object.keys(input.permissions as object).sort()).toEqual(
      [...PERMISSIONS].sort(),
    )
    expect(input.description).toBeNull()
  })

  it('sends a patch with only what changed', () => {
    expect(accessGroupPatch({ name: 'External auditors' })).toEqual({
      name: 'External auditors',
    })
  })

  it('leaves granted_by and ids to the module for a new member', () => {
    const member: Omit<GroupMember, 'id' | 'created_at'> = {
      group_id: 'g1',
      user_id: 'u-b',
      user_email: null,
      granted_by: 'u-a',
      valid_from: '2026-10-30T00:00:00Z',
      valid_until: null,
      notes: null,
    }
    expect(groupMemberInput(member)).toEqual({
      group_id: 'g1',
      user_id: 'u-b',
      user_email: null,
      valid_from: '2026-10-30T00:00:00Z',
      valid_until: null,
      notes: null,
    })
  })

  it('names members from the member rows and keeps invites without a person', () => {
    const rows: Array<GroupMember> = [
      {
        id: 'm1',
        group_id: 'g1',
        user_id: 'u-b',
        user_email: null,
        granted_by: 'u-a',
        valid_from: '2026-10-01',
        valid_until: null,
        notes: null,
        created_at: '2026-10-01',
      },
      {
        id: 'm2',
        group_id: 'g1',
        user_id: null,
        user_email: 'auditor@example.org',
        granted_by: 'u-a',
        valid_from: '2026-10-02',
        valid_until: null,
        notes: 'Invited by email. No email was sent.',
        created_at: '2026-10-02',
      },
      { ...rowOtherGroup() },
    ]
    const out = groupMembersWithProfiles(rows, 'g1', [
      { user_id: 'u-b', name: 'Baraka' },
    ])
    expect(out.map((m) => m.id)).toEqual(['m2', 'm1'])
    expect(out[1].profile?.full_name).toBe('Baraka')
    expect(out[0].profile).toBeNull()
  })
})

function rowOtherGroup(): GroupMember {
  return {
    id: 'm3',
    group_id: 'g2',
    user_id: 'u-c',
    user_email: null,
    granted_by: null,
    valid_from: '2026-10-03',
    valid_until: null,
    notes: null,
    created_at: '2026-10-03',
  }
}

describe('embed settings patch (EmbedConfigPatch, flows.rs:443)', () => {
  it('drops the token, ids and times, which the module owns', () => {
    const patch = embedConfigPatch({
      id: 'e1',
      organization_id: 'rw1',
      embed_token: 'old',
      created_at: 'x',
      updated_at: 'x',
      show_amounts: false,
      allow_embed: true,
    })
    expect(onlyFrom(patch, EMBED_PATCH_FIELDS)).toEqual([])
    expect(patch).toEqual({ show_amounts: false, allow_embed: true })
  })
})

describe('the Show on public pages switch (is_public)', () => {
  it('is sent alone in the profile patch', () => {
    expect(organizationPatch({ is_public: true })).toEqual({ is_public: true })
  })

  it('is read from the module profile', () => {
    const on = organizationFrom('rw1', JSON.stringify({ is_public: true }), 'KES', undefined)
    const off = organizationFrom('rw1', JSON.stringify({ name: 'RW1' }), 'KES', undefined)
    expect(on.is_public).toBe(true)
    expect(off.is_public).toBe(false)
  })
})

describe('the trust row (trust.rs:149, P16)', () => {
  const row = JSON.stringify({
    organization_id: 'rw1',
    score: 71,
    grade: 'B',
    components: { verification: 50, documentation: 100, consistency: 80, model: 68.4 },
    weights: { verification: 32, documentation: 24, consistency: 24, model: 20 },
    breakdown: { verification: 50, documentation: 100, consistency: 80, model: 68 },
    computed_at: '2026-10-30T09:00:00Z',
    computed_by: 'u-a',
  })

  it('gives the four components with their weights, in order', () => {
    const t = trustSummaryFrom(row)
    expect(t?.score).toBe(71)
    expect(t?.grade).toBe('B')
    expect(t?.components.map((c) => [c.key, c.weight])).toEqual([
      ['verification', 32],
      ['documentation', 24],
      ['consistency', 24],
      ['model', 20],
    ])
    expect(t?.computed_at).toBe('2026-10-30T09:00:00Z')
  })

  it('marks the review part not yet scored while its source is neutral', () => {
    const neutral = JSON.parse(row) as Record<string, unknown>
    const of = (source: string) =>
      trustSummaryFrom(JSON.stringify({ ...neutral, model: { source } }))?.components.find(
        (c) => c.key === 'model',
      )?.scored
    expect(of('neutral')).toBe(false)
    expect(of('rubric')).toBe(true)
  })

  it('is null before the module has computed one', () => {
    expect(trustSummaryFrom(undefined)).toBeNull()
  })

  it('keeps the latest snapshots, oldest first', () => {
    const h = (id: string, at: string): TrustScoreHistory => ({
      id,
      organization_id: 'rw1',
      score: 1,
      breakdown: {},
      verification_rate: null,
      documentation_rate: null,
      consistency_score: null,
      calculated_at: at,
    })
    const out = trustHistoryOf(
      [h('b', '2026-10-02'), h('c', '2026-10-03'), h('a', '2026-10-01')],
      2,
    )
    expect(out.map((x) => x.id)).toEqual(['b', 'c'])
  })
})
