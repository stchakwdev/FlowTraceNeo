// Transparency fixtures for the module's MOD2 tables: a scripted run of the demo API
// (flowtrace/src/lib/demo/api.ts) over flow nodes, transactions, proof counting, node
// visibility, the public embed config, organisation chains and the trust score, recorded call by
// call with its result and the store delta; and the trust formula's outputs
// (flowtrace/src/lib/types.ts calculateTrustScore and getTrustGrade) pinned on a grid of
// inputs and on the demo data.
//
// Same frozen clock and counted ids as transparency.ts (withFrozenClock); each run starts from a
// fresh demo store and its own id counter.
import { canonical } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import { TRANSPARENCY_ENV, delta, withFrozenClock } from './transparency'
import type { Rec, Store } from './transparency'

const API = 'flowtrace/src/lib/demo/api.ts'
const TYPES = 'flowtrace/src/lib/types.ts'

type Seed = typeof import('../../../flowtrace/src/lib/demo/dataset.ts')
type Api = typeof import('../../../flowtrace/src/lib/demo/api.ts')

/** The rates and consistency values the formula grid crosses (ties such as 2, 9, 0 included). */
const GRID = [0, 1, 2, 5, 9, 33, 50, 58, 67, 82, 85, 92, 100]

export async function exportTransparencyFlows(outDir: string) {
  const seed = await import('../../../flowtrace/src/lib/demo/dataset.ts')
  const api = await import('../../../flowtrace/src/lib/demo/api.ts')
  const { calculateTrustScore, getTrustGrade } = await import('../../../flowtrace/src/lib/types.ts')

  // The demo's own trust inputs and score before any call (calculateTrustScoreAPI, api.ts:406-472).
  api.resetDemoStore()
  const demo = await withFrozenClock(() => api.calculateTrustScoreAPI(seed.DEMO_ORG_ID))
  api.resetDemoStore()
  const fromHistory = seed.trustScoreHistory.map((h) => [
    h.verification_rate,
    h.documentation_rate,
    h.consistency_score,
    calculateTrustScore(h.verification_rate ?? 0, h.documentation_rate ?? 0, h.consistency_score ?? 0),
    h.score,
  ])
  const grid: Array<Array<number>> = []
  for (const a of GRID) for (const b of GRID) for (const c of GRID) grid.push([a, b, c, calculateTrustScore(a, b, c)])
  const grades: Array<[number, string]> = []
  for (let s = 0; s <= 100; s++) grades.push([s, getTrustGrade(s)])
  writeFixture(outDir, 'transparency-trust-score', {
    description:
      'The web trust formula pinned: calculateTrustScore (verification 40%, documentation 30%, consistency 30%, ' +
      'Math.round of the float sum) over a grid of inputs as [verification_rate, documentation_rate, ' +
      'consistency_score, score]; the seed history rows as [verification, documentation, consistency, formula ' +
      'score, stored score]; getTrustGrade for every score 0 to 100; and calculateTrustScoreAPI on the fresh demo ' +
      'store, whose inputs the module feeds its 32/24/24/20 formula with.',
    sources: [`${TYPES}#calculateTrustScore`, `${TYPES}#getTrustGrade`, `${API}#calculateTrustScoreAPI`],
    covers: ['calculateTrustScore', 'getTrustGrade', 'calculateTrustScoreAPI'],
    env: TRANSPARENCY_ENV,
    grid_values: GRID,
    formula_cases: grid,
    history_cases: fromHistory as Json,
    grade_cases: grades,
    demo: canonical(demo),
  })

  const calls = await withFrozenClock(() => runCalls(seed, api))
  writeFixture(outDir, 'transparency-flows', {
    description:
      'A scripted run of the demo API over transparency-dataset.json#/store for the MOD2 tables, one call at a ' +
      'time: the function, its arguments, the outcome (`ok` with the result, `error` with the message) and the ' +
      'store delta per collection (`added` records, `removed` ids, `changed` whole new records). Covers the flow ' +
      'node, transaction, proof counting, node visibility, embed config, chain and trust score writes the demo API ' +
      'has, and the reads whose rules the module mirrors (flow data, organisation stats, chain counts, chain trust ' +
      'score, chain activity log, trust history, campaign report). Chain activity of a cancelled request is not reachable through ' +
      'the API, so it shows as removed.',
    sources: [API],
    covers: [...new Set(calls.map((c) => String(c.fn)))],
    env: TRANSPARENCY_ENV,
    store: 'transparency-dataset.json#/store',
    calls,
  })
}

async function runCalls(seed: Seed, api: Api) {
  api.resetDemoStore()
  const ORG = seed.DEMO_ORG_ID
  const USER = seed.DEMO_USER_ID
  const [, MAJI, , TECHBRIDGE, PWANI] = seed.organizations.map((o) => o.id)
  const G = (s: string) => `d0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const N = (s: string) => `a0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const C = (s: string) => `g0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
  const byId = (rows: Array<{ id: string }>) => [...rows].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))

  // The demo store's MOD2 collections, read through the API's own getters.
  const snapshot = async (): Promise<Store> => {
    const orgs = await Promise.all(seed.organizations.map((o) => api.getOrganization(o.id)))
    const nodes = await api.getNodes(ORG)
    const txns = (await api.getTransactions(ORG)).map(({ from_node: _f, to_node: _t, ...t }) => t)
    const verifications = (await Promise.all(txns.map((t) => api.getVerifications(t.id)))).flat()
    const visibility = (await Promise.all(nodes.map((n) => api.getNodeVisibility(n.id))))
      .flat()
      .map(({ group: _g, ...v }) => v)
    const chains = (await Promise.all(seed.organizations.map((o) => api.getOutgoingChains(o.id))))
      .flat()
      .map(({ source_org: _s, target_org: _t, link_node: _l, granted_group: _g, ...c }) => c)
    const log = (await Promise.all(chains.map((c) => api.getChainActivityLog(c.id)))).flat()
    const history = await api.getTrustScoreHistory(ORG, 1000)
    const embed = await api.getPublicEmbedConfig(ORG)
    return canonical({
      organizations: byId(orgs),
      flowNodes: byId(nodes),
      transactions: byId(txns),
      verifications: byId(verifications),
      nodeVisibility: byId(visibility),
      organizationChains: byId(chains),
      chainActivityLog: byId(log),
      trustScoreHistory: byId(history),
      publicEmbedConfigs: embed ? [embed] : [],
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

  // Flow nodes (createNode, api.ts:264-274).
  const chv = { organization_id: ORG, name: 'Community Health Volunteers', type: 'beneficiary' as const, metadata: { county: 'Kilifi' } }
  const volunteers = String((await call('createNode', [chv], () => api.createNode(chv))).id)
  const mss = { organization_id: ORG, name: 'Medisafe Supplies', type: 'vendor' as const, metadata: {} }
  const medisafe = String((await call('createNode', [mss], () => api.createNode(mss))).id)

  // Transactions (createTransaction and updateTransaction, api.ts:220-254).
  const txn = (extra: Record<string, unknown>) => ({
    organization_id: ORG,
    from_node_id: N('a36'),
    to_node_id: medisafe,
    amount: 185000,
    currency: 'KES',
    exchange_rate: null,
    amount_usd: null,
    description: 'Delivery kits for Kilifi health centres',
    category: 'Supplies',
    reference: 'MS-2026-014',
    notes: null,
    verification_status: 'pending' as const,
    transaction_date: '2026-02-03',
    created_by: USER,
    ...extra,
  })
  const kits = txn({})
  const t1 = String((await call('createTransaction', [kits], () => api.createTransaction(kits))).id)
  const stipends = txn({
    from_node_id: N('a16'),
    to_node_id: volunteers,
    amount: 96000,
    description: 'Volunteer transport stipends, January',
    category: 'Programs',
    reference: null,
    transaction_date: '2026-01-30',
  })
  const t2 = String((await call('createTransaction', [stipends], () => api.createTransaction(stipends))).id)
  const usd = txn({
    from_node_id: N('a20'),
    to_node_id: N('a11'),
    amount: 2590000,
    currency: 'USD',
    exchange_rate: 129.5,
    amount_usd: 20000,
    description: 'Northlight Foundation supplementary grant',
    category: 'Funding',
    reference: 'NLF-2026-002',
    verification_status: 'verified' as const,
    transaction_date: '2026-02-02',
  })
  await call('createTransaction', [usd], () => api.createTransaction(usd))
  const note = { notes: 'Two cartons short; supplier credit note to follow', category: 'Equipment' }
  await call('updateTransaction', [t1, ORG, note], () => api.updateTransaction(t1, ORG, note))
  await call('updateTransaction', [t2, ORG, { verification_status: 'flagged' }], () =>
    api.updateTransaction(t2, ORG, { verification_status: 'flagged' }),
  )
  await call('updateTransaction', ['txn-missing', ORG, { notes: 'Nobody' }], () =>
    api.updateTransaction('txn-missing', ORG, { notes: 'Nobody' }),
  )

  // Proof counting and auto-verify at two (insertVerification, api.ts:284-307).
  const proof = (transaction_id: string, type: string, status: string, description: string) => ({
    transaction_id,
    type,
    file_url: null,
    ocr_data: null,
    metadata: {},
    status,
    verified_by: status === 'verified' ? USER : null,
    visibility_level: 'full',
    thumbnail_url: null,
    executor_name: null,
    gps_latitude: null,
    gps_longitude: null,
    gps_accuracy: null,
    location_name: null,
    description,
    created_by: USER,
  })
  for (const p of [
    proof(t1, 'invoice', 'pending', 'Medisafe invoice MS-2026-014'),
    proof(t1, 'photo', 'verified', 'Delivered kits at Mariakani health centre'),
    proof(t2, 'receipt', 'pending', 'M-Pesa stipend batch confirmation'),
    proof(t2, 'signature', 'pending', 'Signed volunteer register'),
  ]) {
    await call('createVerification', [p], () => api.createVerification(p as Parameters<Api['createVerification']>[0]))
  }

  // Flow data and organisation stats (api.ts:475-566, 120-149).
  await call('getFlowDataAPI', [ORG], () => api.getFlowDataAPI(ORG))
  await call('getFlowDataAPI', [ORG, '2026-01-01', '2026-01-31'], () => api.getFlowDataAPI(ORG, '2026-01-01', '2026-01-31'))
  await call('getOrganizationStats', [ORG], () => api.getOrganizationStats(ORG))

  // Node visibility (api.ts:708-743).
  const masked = { can_view_amounts: false, display_name: 'Maternal health programme' }
  await call('setNodeVisibility', [N('a36'), G('d03'), masked], () => api.setNodeVisibility(N('a36'), G('d03'), masked))
  await call('setNodeVisibility', [N('a35'), G('d04'), { can_view_proofs: true }], () =>
    api.setNodeVisibility(N('a35'), G('d04'), { can_view_proofs: true }),
  )
  await call('removeNodeVisibility', [N('a36'), G('d03')], () => api.removeNodeVisibility(N('a36'), G('d03')))

  // Public embed config (api.ts:752-770).
  const embed = {
    display_mode: 'detailed' as const,
    amount_format: 'exact' as const,
    hidden_node_ids: [N('a19')],
    theme: { primary_color: '#0f766e', background: 'dark' as const, border_radius: 'md' as const, show_logo: false },
  }
  await call('updatePublicEmbedConfig', [ORG, embed], () => api.updatePublicEmbedConfig(ORG, embed))
  await call('updatePublicEmbedConfig', ['org-missing', { show_amounts: false }], () =>
    api.updatePublicEmbedConfig('org-missing', { show_amounts: false }),
  )

  // Organisation chains (api.ts:865-1033).
  const toPwani = 'Linking the Kilifi maternal health money end to end.'
  const asked = await call('requestChain', [ORG, PWANI, USER, toPwani], () => api.requestChain(ORG, PWANI, USER, toPwani))
  const toTech = await call('requestChain', [ORG, TECHBRIDGE, USER], () => api.requestChain(ORG, TECHBRIDGE, USER))
  await call('approveChain', [C('g03'), USER, G('d04'), N('a16')], () => api.approveChain(C('g03'), USER, G('d04'), N('a16')))
  const no = 'We share programme data through the county dashboard instead.'
  await call('rejectChain', [C('g04'), USER, no], () => api.rejectChain(C('g04'), USER, no))
  await call('approveChain', [C('g04'), USER, G('d04')], () => api.approveChain(C('g04'), USER, G('d04')))
  await call('updateChainPermissions', [C('g03'), G('d03')], () => api.updateChainPermissions(C('g03'), G('d03')))
  await call('revokeChain', [C('g02'), USER], () => api.revokeChain(C('g02'), USER))
  await call('revokeChain', [C('g02'), USER], () => api.revokeChain(C('g02'), USER))
  await call('cancelChainRequest', [String(toTech.id)], () => api.cancelChainRequest(String(toTech.id)))
  await call('cancelChainRequest', [C('g01')], () => api.cancelChainRequest(C('g01')))
  await call('getChainCounts', [ORG], () => api.getChainCounts(ORG))
  await call('getChainCounts', [MAJI], () => api.getChainCounts(MAJI))
  await call('getChainActivityLog', [C('g03')], () => api.getChainActivityLog(C('g03')))
  await call('getChainActivityLog', [String(asked.id)], () => api.getChainActivityLog(String(asked.id)))

  // Trust score (api.ts:397-472) and the chain score that reads it (api.ts:1018-1031).
  await call('calculateTrustScoreAPI', [ORG], () => api.calculateTrustScoreAPI(ORG))
  await call('calculateTrustScoreAPI', ['org-missing'], () => api.calculateTrustScoreAPI('org-missing'))
  await call('getTrustScoreHistory', [ORG], () => api.getTrustScoreHistory(ORG))
  await call('getTrustScoreHistory', [ORG, 3], () => api.getTrustScoreHistory(ORG, 3))
  await call('getChainTrustScore', [C('g01')], () => api.getChainTrustScore(C('g01')))
  await call('getChainTrustScore', [C('g02')], () => api.getChainTrustScore(C('g02')))
  await call('getChainTrustScore', ['chain-missing'], () => api.getChainTrustScore('chain-missing'))

  // The campaign report the public donate page and embed show (api.ts:1381-1443).
  for (const c of await api.getCampaigns(ORG)) {
    await call('getCampaignReport', [c.id], () => api.getCampaignReport(c.id))
  }

  return calls
}
