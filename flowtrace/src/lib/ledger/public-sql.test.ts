import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// What maincloud answers to `POST /v1/database/<name>/sql` (shape checked against
// flowtrace-ledger-dev): one result per statement, columns in `schema.elements`, rows as arrays.
const answer = (columns: Array<string>, rows: Array<Array<unknown>>) => [
  {
    schema: {
      elements: columns.map((c) => ({
        name: { some: c },
        algebraic_type: { String: [] },
      })),
    },
    rows,
    total_duration_micros: 100,
    stats: { rows_inserted: 0, rows_deleted: 0, rows_updated: 0 },
  },
]

const publishedCampaign = {
  campaign: {
    id: 'c1',
    organization_id: 'rw1',
    name: 'Clean water',
    description: null,
    short_description: 'Wells for Kibera',
    image_url: null,
    target_amount: 50_000_000,
    current_amount: 250_000,
    currency: 'KES',
    target_node: { id: 'n1', name: 'Wells', type: 'project' },
    status: 'active',
    allow_overflow: false,
    overflow_consent_message: null,
    starts_at: '2026-01-01T00:00:00.000Z',
    ends_at: null,
    public_slug: 'clean-water',
    category: null,
    tags: null,
  },
  stats: {
    campaign_id: 'c1',
    donation_count: 1,
    unique_donors: 1,
    total_raised: 250_000,
    funding_percentage: 0.5,
    avg_donation: 250_000,
    largest_donation: 250_000,
    smallest_donation: 250_000,
  },
  organization: {
    id: 'rw1',
    name: 'RW1 NGO',
    trust_score: 41,
    verified: false,
  },
  evidenced_amount: 100_000,
  earmarks: [
    {
      id: 'n1',
      name: 'Wells',
      type: 'project',
      received: 300_000,
      evidenced: 100_000,
      is_target: true,
    },
  ],
  spending: [
    {
      id: 't1',
      description: 'Drilling',
      amount: 300_000,
      currency: 'KES',
      transaction_date: '2026-01-10',
      verification_status: 'pending',
      verification_count: 0,
      from_node: null,
      to_node: { id: 'n1', name: 'Wells', type: 'project' },
    },
  ],
}

const fetchMock = vi.fn()

async function load() {
  vi.stubEnv('VITE_SPACETIMEDB_URI', 'wss://maincloud.spacetimedb.com')
  vi.stubEnv('VITE_SPACETIMEDB_MODULE', 'flowtrace-ledger-dev')
  vi.stubGlobal('fetch', fetchMock)
  vi.resetModules()
  return import('./public-sql')
}

const ok = (body: unknown) => ({
  ok: true,
  status: 200,
  json: () => Promise.resolve(body),
  text: () => Promise.resolve(''),
})

beforeEach(() => fetchMock.mockReset())
afterEach(() => {
  vi.unstubAllEnvs()
  vi.unstubAllGlobals()
})

describe('anonymous reads over HTTP SQL', () => {
  it('posts the query to the database over https with no Authorization header', async () => {
    const sql = await load()
    fetchMock.mockResolvedValue(
      ok(answer(['campaign_id', 'org_id', 'slug', 'data'], [])),
    )
    await expect(sql.getPublicCampaignBySlug('clean-water')).rejects.toThrow(
      'This campaign is not open for donations.',
    )
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toBe(
      'https://maincloud.spacetimedb.com/v1/database/flowtrace-ledger-dev/sql',
    )
    expect(init.method).toBe('POST')
    expect(init.body).toBe(
      "SELECT * FROM public_campaign WHERE slug = 'clean-water'",
    )
    expect(
      Object.keys(init.headers ?? {}).map((k) => k.toLowerCase()),
    ).not.toContain('authorization')
  })

  it('quotes what it is given, so a link cannot change the query', async () => {
    const sql = await load()
    fetchMock.mockResolvedValue(ok(answer(['data'], [])))
    await sql.getPublicCampaignStats("x' OR '1'='1").catch(() => {})
    expect((fetchMock.mock.calls[0] as [string, RequestInit])[1].body).toBe(
      "SELECT * FROM public_campaign WHERE campaign_id = 'x'' OR ''1''=''1'",
    )
  })

  it('reads a published campaign in major units, with its figures and the day counts', async () => {
    const sql = await load()
    fetchMock.mockResolvedValue(
      ok(
        answer(
          ['campaign_id', 'org_id', 'slug', 'data'],
          [['c1', 'rw1', 'clean-water', JSON.stringify(publishedCampaign)]],
        ),
      ),
    )
    const c = await sql.getPublicCampaignBySlug('clean-water')
    expect(c).toMatchObject({
      id: 'c1',
      organization_id: 'rw1',
      target_amount: 500_000,
      current_amount: 2500,
      target_node_id: 'n1',
      target_node: { id: 'n1', name: 'Wells', type: 'project' },
      is_public: true,
    })
    expect(c.stats).toMatchObject({
      donation_count: 1,
      total_raised: 2500,
      avg_donation: 2500,
      days_remaining: null,
    })
  })

  it('reads the campaign report in major units', async () => {
    const sql = await load()
    fetchMock.mockResolvedValue(
      ok(answer(['data'], [[JSON.stringify(publishedCampaign)]])),
    )
    const r = await sql.getPublicCampaignReport('c1')
    expect(r.evidenced_amount).toBe(1000)
    expect(r.earmarks[0]).toMatchObject({
      received: 3000,
      evidenced: 1000,
      is_target: true,
    })
    expect(r.spending[0]).toMatchObject({
      id: 't1',
      amount: 3000,
      to_node_id: 'n1',
      organization_id: 'rw1',
    })
    expect(r.organization.name).toBe('RW1 NGO')
  })

  it('reads the published organisation and its badge', async () => {
    const sql = await load()
    fetchMock
      .mockResolvedValueOnce(
        ok(
          answer(
            ['org_id', 'slug', 'data'],
            [
              [
                'rw1',
                'rw1-ngo',
                JSON.stringify({
                  id: 'rw1',
                  name: 'RW1 NGO',
                  type: 'NGO',
                  country: 'KE',
                  currency: 'KES',
                  verified: false,
                  public_slug: 'rw1-ngo',
                  trust_score: 41,
                  grade: 'F',
                  updated_at: 'u',
                }),
              ],
            ],
          ),
        ),
      )
      .mockResolvedValueOnce(
        ok(
          answer(
            ['org_id', 'data'],
            [
              [
                'rw1',
                JSON.stringify({
                  score: 41,
                  grade: 'F',
                  breakdown: { verification: 40 },
                  calculated_at: 'c',
                }),
              ],
            ],
          ),
        ),
      )
    expect(await sql.getPublicOrganization('rw1')).toMatchObject({
      name: 'RW1 NGO',
      trust_score: 41,
      public_slug: 'rw1-ngo',
    })
    expect(await sql.getPublicBadge('rw1')).toEqual({
      overall_score: 41,
      breakdown: { verification: 40 },
      verification_rate: null,
      calculated_at: 'c',
    })
  })

  it('passes on the refusal of a private table', async () => {
    const sql = await load()
    fetchMock.mockResolvedValue({
      ok: false,
      status: 400,
      text: () =>
        Promise.resolve(
          'no such table: `campaign`. If the table exists, it may be marked private.\n',
        ),
    })
    await expect(sql.publicSql('SELECT * FROM campaign')).rejects.toThrow(
      'no such table: `campaign`. If the table exists, it may be marked private.',
    )
  })
})
