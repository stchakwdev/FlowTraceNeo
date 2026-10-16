// The MOD1 transparency tables end to end on the local SpacetimeDB server.
//
// 1. Publishes the module as ft-mod1-<stamp>. With `--base-wasm <file>` (a module built from
//    main) it publishes that first, opens books on it, and then publishes this module over it
//    without clearing: the additive-schema check.
// 2. Opens the demo organisation's books and loads the transparency fixtures
//    (rust/fixtures/transparency-dataset.json) with `import_transparency`.
// 3. Runs every transparency reducer once through `spacetime call`, as the CLI identity (the
//    publisher, so an operator, and the books' owner).
// 4. Reads back through SQL: row counts per table as the owner, the `my_*` views over HTTP as a
//    member on the Board, and as a stranger, who must see nothing.
// 5. Reads back through the Rust client: `ledger-client`'s `live_transparency` test joins as a
//    Donors-group member and must get the same records, with proofs cut to thumbnails.
// 6. Deletes the database (unless `--keep`).
//
// Usage: bun transparency-proof.ts [--db ft-mod1-...] [--base-wasm <file>] [--keep] [--no-build]
// Writes rust/out/mod1-proof/{expected.json,summary.json}. Local server only.
import fs from 'node:fs'
import path from 'node:path'
import { OUT_DIR, RUST_DIR, FIXTURES_DIR } from './src/paths'
import { SPACETIME, WASM, args, localServer } from './src/module-server'
import type { Json, Row, Who } from './src/module-server'

const { flag, option } = args()
const stamp = new Date().toISOString().replace(/[-:]/g, '').replace('T', '-').slice(0, 15)
const DB = option('db', `ft-mod1-${stamp}`)
const BASE = option('base-wasm', '')
const OUT = path.join(OUT_DIR, 'mod1-proof')
fs.mkdirSync(OUT, { recursive: true })

const dataset = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'transparency-dataset.json'), 'utf8'))
const ORG: string = dataset.demo_org_id
const BOARD = 'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d01'
const DONORS = 'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d03'
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

  // 2. Defaults, then the fixtures over them.
  call('create_default_access_groups', ORG)
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
  say(`ran all ${expectedReducers.length} transparency reducers through spacetime call`)

  // 4. SQL: counts as the owner, values through the views as the reader and a stranger.
  const counts: Record<string, number> = {}
  for (const t of ['access_group', 'access_group_member', 'verification', 'verification_access_log', 'campaign', 'campaign_stat', 'donation', 'donor_update', 'organization_profile']) {
    counts[t] = count(t)
  }
  const want = { access_group: 5, access_group_member: 10, verification: 22, verification_access_log: 1, campaign: 5, campaign_stat: 5, donation: 26, donor_update: 4, organization_profile: 1 }
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

  // 5. The Rust client as a Donors-group member: same records, proofs as thumbnails.
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
  say(`client read-back passed (${((performance.now() - t) / 1000).toFixed(1)} s): ${(/client: .*/.exec(testOut) ?? ['?'])[0]}`)

  if (!flag('keep')) {
    server.remove()
    say(`deleted ${DB}`)
  } else say(`kept ${DB}`)
  const secs = ((performance.now() - started) / 1000).toFixed(1)
  say(`done in ${secs} s`)
  fs.writeFileSync(path.join(OUT, 'summary.json'), `${JSON.stringify({ db: DB, counts, log, seconds: Number(secs) }, null, 2)}\n`)
}

main().catch((e) => {
  console.error(`proof: FAILED: ${e instanceof Error ? e.message : String(e)}`)
  if (!flag('keep')) server.cli(['delete', '-s', 'local', '-y', DB], true)
  process.exit(1)
})
