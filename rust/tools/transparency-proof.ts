// The transparency tables (MOD1 and MOD2) end to end on the local SpacetimeDB server.
//
// 1. Publishes the module as ft-mod2-<stamp>. With `--base-wasm <file>` (a module built from
//    main) it publishes that first, opens books on it, and then publishes this module over it
//    without clearing: the additive-schema check.
// 2. Opens the demo organisation's books, loads the demo books (`import_books`) and the
//    transparency fixtures (rust/fixtures/transparency-dataset.json, MOD2's collections and chains
//    included) with `import_transparency`.
// 3. Runs every MOD1 transparency reducer once through `spacetime call`, as the CLI identity (the
//    publisher, so an operator, and the books' owner).
// 4. Runs every MOD2 reducer once: flow nodes, transactions and their journal entry links,
//    visibility, the embed config, chains with a second organisation, a service registered by the
//    operator that records flags and rubric scores and recomputes the trust score over HTTP as
//    itself, a flag resolved, a refusal for a non-service, and the service removed.
// 5. Reads back through SQL: row counts per table as the owner, the `my_*` views over HTTP as a
//    member on the Board, and as a stranger, who must see nothing; the public tables over HTTP
//    with no `Authorization` header at all.
// 6. Reads back through the Rust client: `live_transparency` joins as a Donors-group member and
//    must get the same records (proofs cut to thumbnails, the MOD2 views equal); `live_public`
//    connects with no token and must read the public tables and nothing else.
// 7. Deletes the database (unless `--keep`).
//
// Usage: bun transparency-proof.ts [--db ft-mod2-...] [--base-wasm <file>] [--keep] [--no-build]
// Writes rust/out/transparency-proof/{expected.json,public.json,summary.json}. Local server only.
import fs from 'node:fs'
import path from 'node:path'
import { OUT_DIR, RUST_DIR, FIXTURES_DIR } from './src/paths'
import { SPACETIME, WASM, args, localServer } from './src/module-server'
import type { Json, Row, Who } from './src/module-server'

const { flag, option } = args()
const stamp = new Date().toISOString().replace(/[-:]/g, '').replace('T', '-').slice(0, 15)
const DB = option('db', `ft-mod2-${stamp}`)
const BASE = option('base-wasm', '')
const OUT = path.join(OUT_DIR, 'transparency-proof')
fs.mkdirSync(OUT, { recursive: true })

const dataset = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'transparency-dataset.json'), 'utf8'))
const ORG: string = dataset.demo_org_id
const BOARD = 'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d01'
const DONORS = 'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d03'
const PARTNER_ORG = 'ft-mod2-partner-org'
const N = (s: string) => `a0eebc99-9c0b-4ef8-bb6d-6bb9bd380${s}`
const server = localServer(DB)
const started = performance.now()
const log: Array<string> = []
const say = (s: string) => {
  log.push(s)
  console.log(`proof: ${s}`)
}
const fail = (s: string): never => {
  throw new Error(s)
}

function build() {
  const t = performance.now()
  const r = Bun.spawnSync(['cargo', 'build', '--release', '--locked', '--target', 'wasm32-unknown-unknown', '-p', 'ledger-module'], {
    cwd: RUST_DIR,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  if (r.exitCode !== 0) fail(`wasm build failed:\n${r.stderr.toString()}`)
  say(`built ${path.relative(RUST_DIR, WASM)}: ${Bun.file(WASM).size} bytes in ${((performance.now() - t) / 1000).toFixed(1)} s`)
}

/** `spacetime call` with each argument as JSON, as the CLI identity. */
const reducersCalled = new Set<string>()
function call(reducer: string, ...argv: Array<Json>) {
  server.cli(['call', '-s', 'local', DB, reducer, ...argv.map((a) => JSON.stringify(a))])
  reducersCalled.add(reducer)
}
const some = (v: Json) => ({ some: v })

/** One number from `SELECT COUNT(*) AS n ...` through the CLI, which reads private tables as owner. */
function count(table: string): number {
  const out = server.cli(['sql', '-s', 'local', DB, `SELECT COUNT(*) AS n FROM ${table} WHERE org_id = '${ORG}'`])
  const m = /\n\s*(\d+)\s*$/.exec(out)
  return m ? Number(m[1]) : fail(`no count in: ${out}`)
}

/** SQL over HTTP with no `Authorization` header: what anyone on the network can read. */
async function anonymousSql(query: string): Promise<{ ok: boolean; rows: Array<Record<string, Json>>; text: string }> {
  const res = await fetch(`http://127.0.0.1:3100/v1/database/${DB}/sql`, { method: 'POST', body: query })
  const text = await res.text()
  if (!res.ok) return { ok: false, rows: [], text }
  const [result] = JSON.parse(text) as Array<{ schema: { elements: Array<{ name: { some: string } }> }; rows: Array<Array<Json>> }>
  const names = result.schema.elements.map((e) => e.name.some)
  return { ok: true, text, rows: result.rows.map((r) => Object.fromEntries(names.map((n, i) => [n, r[i]]))) }
}

/** `SELECT COUNT(*)` of any query tail through the CLI, as owner. */
function countWhere(tail: string): number {
  const out = server.cli(['sql', '-s', 'local', DB, `SELECT COUNT(*) AS n FROM ${tail}`])
  const m = /\n\s*(\d+)\s*$/.exec(out)
  return m ? Number(m[1]) : fail(`no count in: ${out}`)
}

async function docs(who: Who, view: string): Promise<Array<Record<string, Json>>> {
  const rows: Array<Row> = (await server.sql(who, `SELECT * FROM ${view}`)).filter((r) => r.org_id === ORG)
  return rows
    .sort((a, b) => Number(a.seq) - Number(b.seq))
    .map((r) => JSON.parse(r.data as string) as Record<string, Json>)
}

async function main() {
  if (!flag('no-build')) build()

  // 1. Publish, over main's module when given.
  if (BASE) {
    server.publish(BASE)
    call('create_org_ledger', ORG, 'KES', 2026, 'Demo User')
    const out = server.cli(['publish', '-s', 'local', '-b', WASM, '-y', DB])
    if (/clear|breaking/i.test(out) && !/Updated database/i.test(out)) fail(`republish was not additive:\n${out}`)
    say(`published ${path.basename(BASE)} (main) then this module over it without clearing: ${out.split('\n').filter((l) => /Updated|Created|Publishing/.test(l)).join(' / ')}`)
  } else {
    server.publish()
    call('create_org_ledger', ORG, 'KES', 2026, 'Demo User')
    say(`published ${DB}`)
  }

  // 2. Defaults, the demo books (for journal entry links), then the fixtures over them.
  call('create_default_access_groups', ORG)
  const operator = await server.newIdentity('operator')
  call('add_operator', `0x${operator.identity}`)
  const books = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'demo-books.json'), 'utf8')).state
  await server.mustCall(operator, 'import_books', [ORG, JSON.stringify(books)])
  call('remove_operator', `0x${operator.identity}`)
  call('import_transparency', ORG, JSON.stringify(dataset.store))
  call('set_profile', 'Demo User', 'demo@flowtrace.dev')

  // A reader on the Board, over HTTP, to read the views as JSON.
  const reader = await server.newIdentity('reader')
  call('add_member', ORG, `0x${reader.identity}`, 'user_reader', 'Board Reader', false, false, false)
  const stranger = await server.newIdentity('stranger')
  const owner = (await server.sql(reader, 'SELECT * FROM my_member')).find((m) => m.name === 'Demo User') ?? fail('no owner row')
  const ownerUser = owner.user_id as string

  // 3. Every transparency reducer once.
  call('update_organization_profile', ORG, JSON.stringify({ description: 'Water, health and education programmes in Kenya', public_slug: 'demo-ngo-kenya' }))
  call('add_access_group_member', ORG, JSON.stringify({ group_id: BOARD, user_id: ownerUser }))
  call('add_access_group_member', ORG, JSON.stringify({ group_id: BOARD, user_id: 'user_reader' }))
  const group = {
    name: 'Proof auditors',
    slug: 'proof-auditors',
    icon: 'users',
    color: '#1C5FA8',
    permissions: {
      can_view_nodes: true, can_view_amounts: true, can_view_proofs: true, can_view_executor: false,
      can_view_full_proofs: false, can_edit_nodes: false, can_edit_transactions: false,
      can_manage_members: false, can_manage_groups: false,
    },
    node_scope: 'assigned',
    priority: 50,
  }
  call('create_access_group', ORG, JSON.stringify(group))
  const groups = await docs(reader, 'my_access_group')
  const gid = String(groups.find((g) => g.slug === 'proof-auditors')?.id ?? fail('no new group'))
  call('update_access_group', ORG, gid, JSON.stringify({ description: 'Auditors who check proof thumbnails' }))
  call('invite_access_group_member', ORG, gid, 'auditor@ledgerline-audit.example.co.ke', some('2026-06-30T23:59:59Z'))
  const invite = (await docs(reader, 'my_access_group_member')).find((m) => m.group_id === gid) ?? fail('no invite')
  call('remove_access_group_member', ORG, String(invite.id))
  call('delete_access_group', ORG, gid)

  const sha256 = new Bun.CryptoHasher('sha256').update('fixture receipt').digest('hex')
  call('create_verification', ORG, JSON.stringify({
    transaction_id: 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b14',
    type: 'receipt',
    object_key: `${ORG}/proofs/mpesa-float-receipt.pdf`,
    sha256,
    thumbnail_key: `${ORG}/proofs/mpesa-float-receipt-small.jpg`,
    status: 'verified',
    description: 'M-Pesa float top-up receipt',
  }))
  const proof = (await docs(reader, 'my_verification')).find((v) => v.sha256 === sha256) ?? fail('no new proof')
  call('log_verification_access', ORG, String(proof.id), 'download')

  call('create_campaign', ORG, JSON.stringify({
    name: 'School Desks Turkana', target_amount: 300000000, currency: 'KES', starts_at: '2026-02-01T00:00:00Z',
    ends_at: '2026-12-31T23:59:59Z', is_public: true, category: 'education',
  }))
  call('create_campaign', ORG, JSON.stringify({ name: 'Draft to delete', target_amount: 100000, currency: 'KES', starts_at: '2026-03-01T00:00:00Z' }))
  let campaigns = await docs(reader, 'my_campaign')
  const cid = String(campaigns.find((c) => c.name === 'School Desks Turkana')?.id ?? fail('no new campaign'))
  const draft = String(campaigns.find((c) => c.name === 'Draft to delete')?.id ?? fail('no draft'))
  call('update_campaign', ORG, cid, JSON.stringify({ status: 'active' }))
  call('delete_campaign', ORG, draft)
  call('record_donation', ORG, JSON.stringify({ campaign_id: cid, donor_name: 'Lokichar Parents Group', amount: 2500000, currency: 'KES', payment_method: 'mpesa' }))
  const gift = (await docs(reader, 'my_donation')).find((d) => d.campaign_id === cid) ?? fail('no new donation')
  call('update_donation_status', ORG, String(gift.id), 'confirmed', some('REF-PROOF-1'))
  call('post_donor_update', ORG, JSON.stringify({ donation_id: gift.id, campaign_id: cid, update_type: 'thank_you', title: 'Thank you', message: 'Your gift buys desks for Lokichar.' }))
  const update = (await docs(reader, 'my_donor_update')).find((u) => u.donation_id === gift.id) ?? fail('no update')
  call('mark_donor_update_sent', ORG, String(update.id))

  const expectedReducers = [
    'set_profile', 'update_organization_profile', 'create_default_access_groups', 'create_access_group',
    'update_access_group', 'delete_access_group', 'add_access_group_member', 'invite_access_group_member',
    'remove_access_group_member', 'create_verification', 'log_verification_access', 'create_campaign',
    'update_campaign', 'delete_campaign', 'record_donation', 'update_donation_status', 'post_donor_update',
    'mark_donor_update_sent', 'import_transparency',
  ]
  const missing = expectedReducers.filter((r) => !reducersCalled.has(r))
  if (missing.length) fail(`reducers not called: ${missing.join(', ')}`)
  say(`ran all ${expectedReducers.length} MOD1 transparency reducers through spacetime call`)

  const mod2 = await runMod2(reader, stranger)

  // 4. SQL: counts as the owner, values through the views as the reader and a stranger.
  const counts: Record<string, number> = {}
  for (const t of ['access_group', 'access_group_member', 'verification', 'verification_access_log', 'campaign', 'campaign_stat', 'donation', 'donor_update', 'organization_profile',
    'flow_node', 'transaction', 'node_visibility', 'transaction_visibility', 'trust_score', 'trust_score_history', 'flag', 'public_embed_config', 'public_organization', 'public_badge', 'public_campaign']) {
    counts[t] = count(t)
  }
  counts.organization_chain = countWhere(`organization_chain WHERE source_org_id = '${ORG}'`) + countWhere(`organization_chain WHERE target_org_id = '${ORG}'`)
  counts.chain_activity_log = countWhere('chain_activity_log')
  counts.service = countWhere('service')
  const want = { access_group: 5, access_group_member: 10, verification: 22, verification_access_log: 1, campaign: 5, campaign_stat: 5, donation: 26, donor_update: 4, organization_profile: 1,
    flow_node: 23, transaction: 17, node_visibility: 1, transaction_visibility: 0, trust_score: 1, trust_score_history: 10, flag: 1, public_embed_config: 1,
    public_organization: 1, public_badge: 1, public_campaign: 3, organization_chain: 6, chain_activity_log: 12, service: 0 }
  for (const [t, n] of Object.entries(want)) if (counts[t] !== n) fail(`${t}: ${counts[t]} rows, expected ${n}`)
  say(`row counts as owner: ${Object.entries(counts).map(([t, n]) => `${t} ${n}`).join(', ')}`)

  campaigns = await docs(reader, 'my_campaign')
  const desks = campaigns.find((c) => c.id === cid) ?? fail('campaign gone')
  const stat = (await docs(reader, 'my_campaign_stat')).find((s) => s.campaign_id === cid) ?? fail('no stat')
  if (desks.status !== 'active' || desks.current_amount !== 2500000 || stat.total_raised !== 2500000 || stat.donation_count !== 1)
    fail(`campaign after the calls: ${JSON.stringify({ desks, stat })}`)
  if (campaigns.some((c) => c.id === draft)) fail('the deleted draft is still there')
  const seen = (await docs(reader, 'my_verification')).find((v) => v.id === proof.id) ?? fail('proof gone')
  if (seen.visible_as !== 'full' || seen.object_key !== `${ORG}/proofs/mpesa-float-receipt.pdf`) fail(`board reader proof: ${JSON.stringify(seen)}`)
  const [profile] = await docs(reader, 'my_organization_profile')
  if (profile?.public_slug !== 'demo-ngo-kenya') fail(`profile: ${JSON.stringify(profile)}`)
  const people = await server.sql(reader, 'SELECT * FROM my_profile')
  if (people.length !== 1) fail(`profiles the reader sees: ${people.length}`)
  const ownLog = await server.sql(reader, 'SELECT * FROM my_verification_access_log')
  if (ownLog.length !== 0) fail('the reader sees another member’s access log')
  say(`views as a Board member: campaign raised 2500000 of 300000000 minor units, proof visible_as full with its object key, profile slug demo-ngo-kenya, 1 profile, 0 of the owner's access-log rows`)
  let strangerRows = 0
  for (const v of ['my_organization_profile', 'my_access_group', 'my_access_group_member', 'my_verification', 'my_verification_access_log', 'my_campaign', 'my_campaign_stat', 'my_donation', 'my_donor_update', 'my_profile'])
    strangerRows += (await server.sql(stranger, `SELECT * FROM ${v}`)).length
  if (strangerRows !== 0) fail(`a stranger sees ${strangerRows} rows`)
  say('views as a stranger: 0 rows in all 10 transparency views')

  // The public tables with no identity at all, and a private table refused the same way.
  const pub = {
    public_organization: (await anonymousSql('SELECT * FROM public_organization')).rows,
    public_badge: (await anonymousSql('SELECT * FROM public_badge')).rows,
    public_campaign: (await anonymousSql('SELECT * FROM public_campaign')).rows,
  }
  const hidden = await anonymousSql('SELECT * FROM transaction')
  if (hidden.ok) fail('an anonymous read of the private transaction table succeeded')
  const pubOrg = pub.public_organization.map((r) => JSON.parse(r.data as string))
  const pubBadge = pub.public_badge.map((r) => JSON.parse(r.data as string))
  const pubCampaigns = pub.public_campaign
    .map((r) => JSON.parse(r.data as string))
    .sort((a, b) => (a.campaign.id < b.campaign.id ? -1 : 1))
  if (pubOrg.length !== 1 || pubOrg[0].trust_score !== mod2.score || pubBadge[0]?.score !== mod2.score)
    fail(`public organisation and badge: ${JSON.stringify({ pubOrg, pubBadge, score: mod2.score })}`)
  const desksPublic = pubCampaigns.find((c) => c.campaign.id === cid)
  if (pubCampaigns.length !== 3 || desksPublic?.stats.total_raised !== 2500000)
    fail(`public campaigns: ${JSON.stringify(pubCampaigns.map((c) => c.campaign.id))}`)
  if (/donor_email|donor_name|"created_by"|"metadata"/.test(JSON.stringify(pubCampaigns))) fail('a public campaign row carries private fields')
  say(`public tables over HTTP with no Authorization header: 1 organisation (trust ${pubOrg[0].trust_score} ${pubOrg[0].grade}), 1 badge, ${pubCampaigns.length} public, active campaigns (${pubCampaigns.map((c) => c.campaign.public_slug).join(', ')}; ${desksPublic.campaign.public_slug} raised ${desksPublic.stats.total_raised}); the private transaction table refused: ${hidden.text.slice(0, 60)}`)
  const publicPath = path.join(OUT, 'public.json')
  fs.writeFileSync(publicPath, `${JSON.stringify({ org_id: ORG, public_organization: pubOrg, public_badge: pubBadge, public_campaign: pubCampaigns }, null, 2)}\n`)

  // 6. The Rust client as a Donors-group member: same records, proofs as thumbnails.
  const expected: Record<string, Json> = {
    org_id: ORG,
    donors_group: DONORS,
    organization_profile: profile,
    access_groups: await docs(reader, 'my_access_group'),
    group_members: await docs(reader, 'my_access_group_member'),
    verifications: await docs(reader, 'my_verification'),
    campaigns,
    campaign_stats: await docs(reader, 'my_campaign_stat'),
    donations: await docs(reader, 'my_donation'),
    donor_updates: await docs(reader, 'my_donor_update'),
    flow_nodes: await docs(reader, 'my_flow_node'),
    transactions: await docs(reader, 'my_transaction'),
    trust_history: await docs(reader, 'my_trust_score_history'),
    flags: await docs(reader, 'my_flag'),
    trust_score: JSON.parse((await server.sql(reader, 'SELECT * FROM my_trust_score')).find((r) => r.org_id === ORG)?.data as string),
    chains: (await server.sql(reader, 'SELECT * FROM my_organization_chain'))
      .sort((a, b) => Number(a.seq) - Number(b.seq))
      .map((r) => JSON.parse(r.data as string)),
  }
  const expectedPath = path.join(OUT, 'expected.json')
  fs.writeFileSync(expectedPath, `${JSON.stringify(expected, null, 2)}\n`)
  const t = performance.now()
  const test = Bun.spawnSync(['cargo', 'test', '--locked', '-p', 'ledger-client', '--test', 'live_transparency', '--', '--ignored', '--nocapture'], {
    cwd: RUST_DIR,
    env: { ...process.env, FT_MOD1_DB: DB, FT_MOD1_EXPECTED: expectedPath, SPACETIME },
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const testOut = `${test.stdout.toString()}${test.stderr.toString()}`
  fs.writeFileSync(path.join(OUT, 'client-test.log'), testOut)
  if (test.exitCode !== 0) fail(`client read-back failed:\n${testOut.slice(-3000)}`)
  say(`client read-back passed (${((performance.now() - t) / 1000).toFixed(1)} s): ${[...testOut.matchAll(/client: .*/g)].map((m) => m[0]).join(' / ')}`)
  const t2 = performance.now()
  const anon = Bun.spawnSync(['cargo', 'test', '--locked', '-p', 'ledger-client', '--test', 'live_public', '--', '--ignored', '--nocapture'], {
    cwd: RUST_DIR,
    env: { ...process.env, FT_MOD2_DB: DB, FT_MOD2_PUBLIC: publicPath, SPACETIME },
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const anonOut = `${anon.stdout.toString()}${anon.stderr.toString()}`
  fs.writeFileSync(path.join(OUT, 'client-public.log'), anonOut)
  if (anon.exitCode !== 0) fail(`anonymous client read failed:\n${anonOut.slice(-3000)}`)
  say(`anonymous client read passed (${((performance.now() - t2) / 1000).toFixed(1)} s): ${(/client: .*/.exec(anonOut) ?? ['?'])[0]}`)

  if (!flag('keep')) {
    server.remove()
    say(`deleted ${DB}`)
  } else say(`kept ${DB}`)
  const secs = ((performance.now() - started) / 1000).toFixed(1)
  say(`done in ${secs} s`)
  fs.writeFileSync(path.join(OUT, 'summary.json'), `${JSON.stringify({ db: DB, counts, mod2, log, seconds: Number(secs) }, null, 2)}\n`)
}

/** Step 4: every MOD2 reducer once. Returns what the later steps check. */
async function runMod2(reader: Who, stranger: Who) {
  const one = async (view: string, pick: (r: Record<string, Json>) => boolean) =>
    (await docs(reader, view)).find(pick) ?? fail(`nothing in ${view}`)

  // Flow nodes and transactions.
  call('create_flow_node', ORG, JSON.stringify({ name: 'Lokichar Clinic', type: 'beneficiary', metadata: { county: 'Turkana' } }))
  const clinic = String((await one('my_flow_node', (n) => n.name === 'Lokichar Clinic')).id)
  call('update_flow_node', ORG, clinic, JSON.stringify({ name: 'Lokichar Health Centre' }))
  call('create_transaction', ORG, JSON.stringify({
    from_node_id: N('a14'), to_node_id: clinic, amount: 4500000, currency: 'KES', description: 'Cold chain fridge for Lokichar',
    category: 'Equipment', verification_status: 'pending', transaction_date: '2026-02-03',
  }))
  const txn = String((await one('my_transaction', (x) => x.description === 'Cold chain fridge for Lokichar')).id)
  call('update_transaction', ORG, txn, JSON.stringify({ reference: 'LOK-2026-07' }))
  const entry = (await server.sql(reader, 'SELECT * FROM my_journal_entry')).find((e) => e.org_id === ORG && e.status === 'posted') ?? fail('no posted entry')
  call('link_transaction_entries', ORG, txn, [String(entry.id)])
  const linked = await one('my_transaction', (x) => x.id === txn)
  if (JSON.stringify(linked.journal_entry_ids) !== JSON.stringify([entry.id]) || linked.reference !== 'LOK-2026-07')
    fail(`transaction after the calls: ${JSON.stringify(linked)}`)

  // Visibility and the embed config.
  call('set_node_visibility', ORG, clinic, DONORS, JSON.stringify({ can_view_amounts: false, display_name: 'A Turkana clinic' }))
  call('remove_node_visibility', ORG, clinic, DONORS)
  call('set_transaction_visibility', ORG, txn, DONORS, JSON.stringify({ can_view_amount: false }))
  call('remove_transaction_visibility', ORG, txn, DONORS)
  call('update_public_embed_config', ORG, JSON.stringify({ show_amounts: false, amount_format: 'hidden' }))
  const tokenBefore = (await one('my_public_embed_config', () => true)).embed_token
  call('regenerate_embed_token', ORG)
  const embed = await one('my_public_embed_config', () => true)
  if (embed.embed_token === tokenBefore || embed.show_amounts !== false) fail(`embed config: ${JSON.stringify(embed)}`)

  // Chains with a second organisation the CLI identity also owns.
  call('create_org_ledger', PARTNER_ORG, 'KES', 2026, 'Partner Owner')
  const chain = async (pick: (c: Record<string, Json>) => boolean) => {
    const rows = await server.sql(reader, 'SELECT * FROM my_organization_chain')
    const row = rows.map((r) => JSON.parse(r.data as string) as Record<string, Json>).find(pick)
    return row ?? fail('no such chain')
  }
  call('request_chain', PARTNER_ORG, ORG, some('We fund the Turkana water programme and want to follow it.'))
  const a = String((await chain((c) => c.source_org_id === PARTNER_ORG && c.status === 'pending')).id)
  call('approve_chain', a, DONORS, some(clinic))
  call('update_chain_permissions', a, BOARD, { none: [] })
  call('request_chain', ORG, PARTNER_ORG, { none: [] })
  const b = String((await chain((c) => c.source_org_id === ORG && c.status === 'pending')).id)
  call('reject_chain', b, some('We report through the county instead.'))
  call('revoke_chain', a)
  call('request_chain', PARTNER_ORG, ORG, some('Asking again after the review.'))
  const c = String((await chain((x) => x.source_org_id === PARTNER_ORG && x.status === 'pending')).id)
  call('cancel_chain_request', c)
  const revoked = await chain((x) => x.id === a)
  if (revoked.status !== 'revoked' || (revoked.source_org as Record<string, Json>).name !== 'ft-mod2-partner-org' || (revoked.granted_group as Record<string, Json>)?.slug !== 'board')
    fail(`chain details: ${JSON.stringify(revoked)}`)

  // A service: registered by the operator, writing as itself over HTTP.
  const jev = await server.newIdentity('jev-worker')
  call('register_service', `0x${jev.identity}`, 'jev-worker', ['flags', 'rubric', 'trust'])
  const mine = await server.sql(jev, 'SELECT * FROM my_service')
  if (mine.length !== 1 || mine[0].label !== 'jev-worker') fail(`my_service as the service: ${JSON.stringify(mine)}`)
  const batch = { model_version: 'jev-proof', flags: [{ subject_type: 'transaction', subject_id: txn, question_key: 'round_amount_near_threshold', answer: 0.82, probability: 0.82, confidence: 0.7 }] }
  const refused = await server.call(reader, 'record_flags', [ORG, JSON.stringify(batch)])
  if (refused.ok || !/registered service with the flags scope/.test(refused.text)) fail(`record_flags as a member: ${refused.text}`)
  await server.mustCall(jev, 'record_flags', [ORG, JSON.stringify(batch)])
  reducersCalled.add('record_flags')
  await server.mustCall(jev, 'record_rubric', [ORG, JSON.stringify({ model_version: 'jev-proof', scores: { receipts_match: 90, timing: 70 } })])
  reducersCalled.add('record_rubric')
  await server.mustCall(jev, 'recompute_trust', [ORG])
  reducersCalled.add('recompute_trust')
  const flag = await one('my_flag', () => true)
  call('resolve_flag', ORG, String(flag.id), 'confirmed', some('Split to stay under the approval limit.'))
  call('recompute_trust', ORG)
  const trustRow = JSON.parse((await server.sql(reader, 'SELECT * FROM my_trust_score')).find((r) => r.org_id === ORG)?.data as string)
  if (trustRow.model.source !== 'rubric' || trustRow.model.confirmed_flags !== 1 || trustRow.components.model !== 75)
    fail(`trust row: ${JSON.stringify(trustRow)}`)
  const audit = (await server.sql(reader, 'SELECT * FROM my_audit_event')).filter((e) => e.org_id === ORG).map((e) => JSON.parse(e.data as string))
  if (!audit.some((e) => e.action === 'flag.confirm' && e.subject === flag.id)) fail('no flag.confirm audit event')
  call('remove_service', `0x${jev.identity}`)
  const gone = await server.call(jev, 'record_flags', [ORG, JSON.stringify(batch)])
  if (gone.ok) fail('a removed service still records flags')

  const strangerRows = (await Promise.all(['my_flow_node', 'my_transaction', 'my_trust_score', 'my_flag', 'my_organization_chain', 'my_chain_activity_log', 'my_public_embed_config', 'my_service']
    .map((v) => server.sql(stranger, `SELECT * FROM ${v}`)))).flat().length
  if (strangerRows !== 0) fail(`a stranger sees ${strangerRows} MOD2 rows`)

  const mod2Reducers = [
    'create_flow_node', 'update_flow_node', 'create_transaction', 'update_transaction', 'link_transaction_entries',
    'set_node_visibility', 'remove_node_visibility', 'set_transaction_visibility', 'remove_transaction_visibility',
    'update_public_embed_config', 'regenerate_embed_token', 'request_chain', 'approve_chain', 'reject_chain', 'revoke_chain',
    'cancel_chain_request', 'update_chain_permissions', 'register_service', 'remove_service', 'record_flags', 'resolve_flag',
    'record_rubric', 'recompute_trust',
  ]
  const missing = mod2Reducers.filter((r) => !reducersCalled.has(r))
  if (missing.length) fail(`MOD2 reducers not called: ${missing.join(', ')}`)
  say(`ran all ${mod2Reducers.length} MOD2 reducers: transaction ${txn} linked to entry ${entry.id}; chain ${a} approved, regranted, revoked, ${b} rejected, ${c} cancelled; service jev-worker recorded 1 flag and a rubric (mean 80) and recomputed; flag confirmed (audit flag.confirm); trust ${trustRow.score} ${trustRow.grade} (model ${trustRow.components.model}); a member's record_flags refused; the removed service refused; a stranger sees 0 MOD2 rows`)
  return { score: trustRow.score as number, grade: trustRow.grade as string, transaction: txn, chain: a }
}

main().catch((e) => {
  console.error(`proof: FAILED: ${e instanceof Error ? e.message : String(e)}`)
  if (!flag('keep')) server.cli(['delete', '-s', 'local', '-y', DB], true)
  process.exit(1)
})
