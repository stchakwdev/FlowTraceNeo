// Transparency fixtures for the module's MOD1 tables: the demo dataset for organisations,
// profiles, access groups and their members, verifications, campaigns, donations and donor
// updates (flowtrace/src/lib/demo/dataset.ts), and a scripted run of the demo API over it
// (flowtrace/src/lib/demo/api.ts), recorded call by call with its result and the store delta.
//
// The demo API reads the wall clock (`new Date()`, `Date.now()`) and draws ids from
// `crypto.randomUUID()`, with no env to inject. While this export runs, both are replaced: the
// clock is frozen at TRANSPARENCY_NOW and ids are UUID-shaped counters. The determinism guards
// (guards.ts) are put back afterwards.
import { canonical, fingerprint } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import type { FrozenEnv } from './fixture'

const API = 'flowtrace/src/lib/demo/api.ts'
const DATASET = 'flowtrace/src/lib/demo/dataset.ts'
const SUPABASE = 'flowtrace/src/lib/supabase.ts'

export const TRANSPARENCY_NOW = '2026-02-04T09:00:00.000Z'

/** The `n`th id the frozen `crypto.randomUUID` draws (1, 2, 3, ...). */
export const transparencyId = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`

export const TRANSPARENCY_ENV: FrozenEnv = {
  now: TRANSPARENCY_NOW,
  new_id:
    '00000000-0000-4000-8000- then n left-padded with 0 to 12 digits, n = 1, 2, 3, ... (a UUID v4 shape), one per crypto.randomUUID() call',
  defined_at:
    'rust/tools/src/transparency.ts#withFrozenClock: replaces Date and crypto.randomUUID while the demo API runs (it has no LedgerEnv)',
}

/** The store collections MOD1 covers, by demo store key, with the module table that holds them. */
export const IN_SCOPE: Record<string, string> = {
  organizations: 'organization_profile',
  profiles: 'profile',
  accessGroups: 'access_group',
  groupMembers: 'access_group_member',
  verifications: 'verification',
  campaigns: 'campaign',
  donations: 'donation',
  donorUpdates: 'donor_update',
}

type Rec = Record<string, Json>
type Store = Record<string, Array<Rec>>

async function withFrozenClock<T>(run: () => Promise<T>): Promise<T> {
  const Guarded = globalThis.Date
  const guardedUuid = globalThis.crypto.randomUUID
  const frozenMs = new Guarded(TRANSPARENCY_NOW).getTime()
  class FrozenDate extends Guarded {
    constructor(...args: Array<unknown>) {
      if (args.length === 0) super(TRANSPARENCY_NOW)
      else super(...(args as [string]))
    }
    static override now(): number {
      return frozenMs
    }
  }
  let n = 0
  globalThis.Date = FrozenDate as DateConstructor
  Object.defineProperty(globalThis.crypto, 'randomUUID', { value: () => transparencyId(++n), configurable: true })
  try {
    return await run()
  } finally {
    globalThis.Date = Guarded
    Object.defineProperty(globalThis.crypto, 'randomUUID', { value: guardedUuid, configurable: true })
  }
}

/** Per changed collection: added records, removed ids, changed (whole new) records. */
function delta(before: Store, after: Store): Record<string, Json> {
  const out: Record<string, Json> = {}
  for (const key of Object.keys(after).sort()) {
    const was = new Map((before[key] ?? []).map((r) => [String(r.id), fingerprint(r)]))
    const now = new Map(after[key].map((r) => [String(r.id), r]))
    const added = after[key].filter((r) => !was.has(String(r.id)))
    const removed = [...was.keys()].filter((id) => !now.has(id))
    const changed = after[key].filter((r) => was.has(String(r.id)) && was.get(String(r.id)) !== fingerprint(r))
    if (!added.length && !removed.length && !changed.length) continue
    const d: Record<string, Json> = {}
    if (added.length) d.added = added
    if (removed.length) d.removed = removed
    if (changed.length) d.changed = changed
    out[key] = d
  }
  return out
}

export async function exportTransparency(outDir: string) {
  const seed = await import('../../../flowtrace/src/lib/demo/dataset.ts')
  const supabase = await import('../../../flowtrace/src/lib/supabase.ts')

  writeFixture(outDir, 'transparency-dataset', {
    description:
      'The demo dataset for the transparency tables of the module (MOD1): organisations, the demo profile, access ' +
      'groups and their members, verifications, campaigns, donations and donor updates, as the demo store holds ' +
      'them before any call (amounts in major units, as the TS types). The public embed config is included for ' +
      'MOD2, which owns it. Also the trust points per verification type.',
    sources: [`${DATASET}`, `${SUPABASE}#VERIFICATION_TRUST_POINTS`],
    covers: ['transparency dataset'],
    env: null,
    demo_org_id: seed.DEMO_ORG_ID,
    demo_user_id: seed.DEMO_USER_ID,
    verification_trust_points: supabase.VERIFICATION_TRUST_POINTS,
    in_scope: IN_SCOPE,
    store: {
      organizations: seed.organizations,
      profiles: seed.profiles,
      accessGroups: seed.accessGroups,
      groupMembers: seed.groupMembers,
      verifications: seed.verifications,
      campaigns: seed.campaigns,
      donations: seed.donations,
      donorUpdates: seed.donorUpdates,
    },
    public_embed_configs: seed.publicEmbedConfigs,
  })

  const calls = await withFrozenClock(() => runCalls(seed))
  writeFixture(outDir, 'transparency-calls', {
    description:
      'A scripted run of the demo API over transparency-dataset.json#/store, one call at a time: the function, its ' +
      'arguments, the outcome (`ok` with the result, `error` with the message) and the store delta per collection ' +
      '(`added` records, `removed` ids, `changed` whole new records). Covers every MOD1 write the demo API has and ' +
      'the reads whose rules the module mirrors (campaign stats, user permissions and verification visibility). ' +
      'The `transactions` delta of createVerification belongs to MOD2.',
    sources: [API, `${SUPABASE}#getVerificationVisibility`],
    covers: [...new Set(calls.map((c) => String(c.fn)))],
    env: TRANSPARENCY_ENV,
    store: 'transparency-dataset.json#/store',
    calls,
  })
}

async function runCalls(seed: typeof import('../../../flowtrace/src/lib/demo/dataset.ts')) {
  const api = await import('../../../flowtrace/src/lib/demo/api.ts')
  const { getVerificationVisibility } = await import('../../../flowtrace/src/lib/supabase.ts')
  api.resetDemoStore()
  const ORG = seed.DEMO_ORG_ID
  const USER = seed.DEMO_USER_ID
  const G = (s: string) => `d0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const T = (s: string) => `b0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const E = (s: string) => `e0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const D = (s: string) => `f0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const N = (s: string) => `a0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`

  // The demo store, read through the API's own getters so the delta sees what the API changed.
  const snapshot = async (): Promise<Store> => {
    const orgs = await Promise.all(seed.organizations.map((o) => api.getOrganization(o.id).catch(() => null)))
    const groups = await api.getAccessGroups(ORG)
    const members = (await Promise.all(groups.map((g) => api.getGroupMembers(g.id)))).flat().map((m) => {
      const { profile: _profile, ...row } = m
      return row
    })
    const txns = await api.getTransactions(ORG)
    const verifications = (await Promise.all(txns.map((t) => api.getVerifications(t.id)))).flat()
    const campaigns = (await api.getCampaigns(ORG)).map(({ target_node: _t, ...c }) => c)
    const donations = (await Promise.all(campaigns.map((c) => api.getDonations(c.id))))
      .flat()
      .map(({ campaign: _c, earmarked_node: _e, ...d }) => d)
    const updates = (await Promise.all(campaigns.map((c) => api.getCampaignUpdates(c.id))))
      .flat()
      .map(({ donation: _d, campaign: _c, related_node: _r, ...u }) => u)
    const transactions = txns.map(({ from_node: _f, to_node: _t, ...t }) => t)
    const byId = (rows: Array<{ id: string }>) => [...rows].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    return canonical({
      organizations: byId(orgs.filter((o) => o !== null)),
      profiles: seed.profiles,
      accessGroups: byId(groups),
      groupMembers: byId(members),
      verifications: byId(verifications),
      campaigns: byId(campaigns),
      donations: byId(donations),
      donorUpdates: byId(updates),
      transactions: byId(transactions),
    }) as Store
  }

  const calls: Array<Record<string, Json>> = []
  let store = await snapshot()
  const call = async (fn: string, args: Array<unknown>, run: () => Promise<unknown>) => {
    const step: Record<string, Json> = { fn, args: canonical(args) }
    try {
      const result = await run()
      step.outcome = 'ok'
      step.result = result === undefined ? null : canonical(result)
    } catch (e) {
      step.outcome = 'error'
      step.error = e instanceof Error ? e.message : String(e)
    }
    const next = await snapshot()
    step.delta = delta(store, next)
    store = next
    calls.push(step)
    return step.result as Rec
  }

  // Organisation profile (updateOrganization, api.ts:162-180).
  await call('updateOrganization', [ORG, { name: 'Demo NGO Kenya', public_slug: 'demo-ngo-kenya', country: 'KE' }], () =>
    api.updateOrganization(ORG, { name: 'Demo NGO Kenya', public_slug: 'demo-ngo-kenya', country: 'KE' }),
  )
  await call('updateOrganization', [ORG, { public_slug: 'afya-bora' }], () =>
    api.updateOrganization(ORG, { public_slug: 'afya-bora' }),
  )
  await call('updateOrganization', ['org-missing', { name: 'Nobody' }], () =>
    api.updateOrganization('org-missing', { name: 'Nobody' }),
  )

  // Access groups and members (api.ts:585-683).
  const newGroup = {
    organization_id: ORG,
    name: 'Harambee grant team',
    slug: 'harambee-grant-team',
    description: 'People who follow the Harambee Health Fund money',
    icon: 'users',
    color: '#1C5FA8',
    permissions: {
      can_view_nodes: true,
      can_view_amounts: true,
      can_view_proofs: true,
      can_view_executor: false,
      can_view_full_proofs: false,
      can_edit_nodes: false,
      can_edit_transactions: false,
      can_manage_members: false,
      can_manage_groups: false,
    },
    node_scope: 'assigned' as const,
    is_default: false,
    is_public: false,
    priority: 50,
  }
  const group = await call('createAccessGroup', [newGroup], () => api.createAccessGroup(newGroup))
  const groupId = String(group.id)
  const groupPatch = {
    description: 'Grant officers who see the original proofs',
    permissions: { ...newGroup.permissions, can_view_full_proofs: true, can_view_executor: true },
  }
  await call('updateAccessGroup', [groupId, groupPatch], () => api.updateAccessGroup(groupId, groupPatch))
  await call('updateAccessGroup', ['group-missing', { name: 'Nobody' }], () =>
    api.updateAccessGroup('group-missing', { name: 'Nobody' }),
  )
  const member = (group_id: string, user_id: string, user_email: string) => ({
    group_id,
    user_id,
    user_email,
    granted_by: USER,
    valid_from: TRANSPARENCY_NOW,
    valid_until: null,
    notes: null,
  })
  const grants = member(groupId, 'user_grants', 'grants.officer@harambeehealth.example.org')
  await call('addGroupMember', [grants], () => api.addGroupMember(grants))
  const invite = ['donor.relations@northlight.example.org', '2026-06-30T23:59:59Z']
  const invited = await call('inviteMemberByEmail', [G('d03'), invite[0], USER, invite[1]], () =>
    api.inviteMemberByEmail(G('d03'), invite[0], USER, invite[1]),
  )
  const donor = member(G('d03'), 'user_donor', 'giving@kisima.example.org')
  await call('addGroupMember', [donor], () => api.addGroupMember(donor))
  const partner = member(G('d04'), 'user_partner', 'partners@majisafi.example.org')
  await call('addGroupMember', [partner], () => api.addGroupMember(partner))

  // Who sees a proof how (getUserPermissions api.ts:772-791, getVerificationVisibility supabase.ts:573-588).
  const proof = seed.verifications[0]
  for (const user of [USER, 'user_grants', 'user_donor', 'user_partner', 'user_nobody']) {
    const perms = await call('getUserPermissions', [user, ORG], () => api.getUserPermissions(user, ORG))
    await call('getVerificationVisibility', [proof.id, perms], async () =>
      getVerificationVisibility(proof, perms as unknown as Parameters<typeof getVerificationVisibility>[1]),
    )
  }
  await call('removeGroupMember', [String(invited.id)], () => api.removeGroupMember(String(invited.id)))
  await call('deleteAccessGroup', [groupId], () => api.deleteAccessGroup(groupId))
  await call('getUserPermissions', ['user_grants', ORG], () => api.getUserPermissions('user_grants', ORG))

  // Verifications (createVerification, api.ts:284-317).
  const proofBase = {
    transaction_id: T('b14'),
    file_url: null,
    ocr_data: null,
    metadata: {},
    verified_by: null,
    visibility_level: 'full' as const,
    thumbnail_url: null,
    executor_name: null,
    gps_latitude: null,
    gps_longitude: null,
    gps_accuracy: null,
    location_name: null,
    created_by: USER,
  }
  const receipt = { ...proofBase, type: 'receipt' as const, status: 'pending' as const, description: 'M-Pesa float top-up confirmation' }
  await call('createVerification', [receipt], () => api.createVerification(receipt))
  const photo = {
    ...proofBase,
    type: 'photo' as const,
    status: 'verified' as const,
    verified_by: USER,
    visibility_level: 'thumbnail' as const,
    executor_name: 'Achieng Otieno',
    gps_latitude: -1.2676,
    gps_longitude: 36.8108,
    gps_accuracy: 15,
    location_name: 'Westlands, Nairobi',
    description: 'Agent till receipt photographed at the counter',
  }
  await call('createVerification', [photo], () => api.createVerification(photo))

  // Campaigns (api.ts:1080-1126).
  const campaign = (name: string, is_public: boolean, extra: Record<string, unknown> = {}) => ({
    organization_id: ORG,
    name,
    description: `${name}: a fixture campaign.`,
    short_description: name,
    image_url: null,
    target_amount: 1500000,
    currency: 'KES',
    target_node_id: null,
    status: 'draft' as const,
    allow_overflow: false,
    overflow_campaign_id: null,
    overflow_consent_message: null,
    starts_at: '2026-03-01T00:00:00Z',
    ends_at: '2026-09-30T23:59:59Z',
    is_public,
    public_slug: null,
    category: 'education',
    tags: ['schools'],
    metadata: {},
    created_by: USER,
    ...extra,
  })
  const literacy = campaign('Digital Literacy', true)
  const created = await call('createCampaign', [literacy], () => api.createCampaign(literacy))
  const newCampaign = String(created.id)
  const boreholes = campaign('Borehole Maintenance Fund', false, { target_node_id: N('a35'), category: 'water', tags: null })
  const quiet = await call('createCampaign', [boreholes], () => api.createCampaign(boreholes))
  const named = campaign('School Feeding', true, { public_slug: 'school-feeding-kilifi' })
  await call('createCampaign', [named], () => api.createCampaign(named))
  await call('updateCampaign', [newCampaign, ORG, { status: 'active' }], () =>
    api.updateCampaign(newCampaign, ORG, { status: 'active' }),
  )
  const kilifi = { target_amount: 4000000, short_description: 'Equipping 8 health centres in Kilifi' }
  await call('updateCampaign', [E('e02'), ORG, kilifi], () => api.updateCampaign(E('e02'), ORG, kilifi))
  await call('updateCampaign', ['campaign-missing', ORG, { name: 'Nobody' }], () =>
    api.updateCampaign('campaign-missing', ORG, { name: 'Nobody' }),
  )
  await call('deleteCampaign', [E('e04'), ORG], () => api.deleteCampaign(E('e04'), ORG))
  await call('deleteCampaign', [E('e01'), ORG], () => api.deleteCampaign(E('e01'), ORG))

  // Donations (api.ts:1245-1289).
  const donation = (campaign_id: string, extra: Record<string, unknown>) => ({
    campaign_id,
    donor_email: null,
    donor_name: null,
    donor_phone: null,
    is_anonymous: false,
    amount: 40000,
    currency: 'KES',
    amount_usd: null,
    exchange_rate: null,
    earmarked_node_id: null,
    earmark_notes: null,
    allow_reallocation: true,
    flow_node_id: null,
    payment_method: 'mpesa',
    payment_reference: null,
    payment_status: 'pending' as const,
    transaction_id: null,
    source: 'website',
    utm_source: null,
    utm_medium: null,
    utm_campaign: null,
    metadata: {},
    ...extra,
  })
  const gift = donation(E('e01'), {
    donor_email: 'wanjiru.k@example.org',
    donor_name: 'Wanjiru Kamau',
    payment_reference: 'REF-NEW1',
    payment_status: 'confirmed',
  })
  const recorded = await call('createDonation', [gift], () => api.createDonation(gift))
  const pledge = donation(newCampaign, { is_anonymous: true, amount: 25000 })
  await call('createDonation', [pledge], () => api.createDonation(pledge))
  await call('updateDonationStatus', [D('f08'), 'confirmed', 'REF-F08'], () =>
    api.updateDonationStatus(D('f08'), 'confirmed', 'REF-F08'),
  )
  await call('updateDonationStatus', [D('f01'), 'refunded'], () => api.updateDonationStatus(D('f01'), 'refunded'))
  await call('updateDonationStatus', ['donation-missing', 'confirmed'], () =>
    api.updateDonationStatus('donation-missing', 'confirmed'),
  )

  // Campaign stats (getCampaignStats, api.ts:1130-1165) of every campaign left, and a missing one.
  for (const c of [E('e01'), E('e02'), E('e03'), newCampaign, String(quiet.id)]) {
    await call('getCampaignStats', [c], () => api.getCampaignStats(c))
  }
  await call('getCampaignStats', ['campaign-missing'], () => api.getCampaignStats('campaign-missing'))

  // Donor updates (api.ts:1329-1349).
  const update = {
    donation_id: String(recorded.id),
    campaign_id: E('e01'),
    update_type: 'thank_you' as const,
    title: 'Thank you for your gift',
    message: 'Your gift is part of the borehole maintenance budget for Turkana South.',
    image_url: null,
    related_node_id: N('a35'),
    related_transaction_id: null,
    delivery_method: 'email' as const,
    metadata: {},
  }
  const posted = await call('createDonorUpdate', [update], () => api.createDonorUpdate(update))
  await call('markUpdateSent', [String(posted.id)], () => api.markUpdateSent(String(posted.id)))
  await call('markUpdateSent', ['update-missing'], () => api.markUpdateSent('update-missing'))

  return calls
}
