import 'fake-indexeddb/auto'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LedgerError } from './ledger'
import {
  closeOfflineDb,
  enqueueClaim,
  flushQueued,
  listQueued,
  removeQueued,
} from './offline-claims'
import type { ClaimInput } from './offline-claims'

const claim = (purpose: string): ClaimInput => ({
  claim_date: '2026-02-03',
  currency: 'KES',
  fx_rate: 1,
  purpose,
  receipt_names: ['fuel.jpg'],
  captured_offline: false,
  submit: true,
  lines: [
    {
      description: 'Fuel',
      account_id: 'acc-5700',
      amount: 120000,
      fund_id: 'fund-unrestricted',
      grant_id: null,
      dimension_ids: [],
    },
  ],
})

async function clear() {
  for (const q of await listQueued()) await removeQueued(q.key)
}

beforeEach(clear)
afterEach(async () => {
  await clear()
  await closeOfflineDb()
})

describe('offline claim queue', () => {
  it('enqueues a claim with an id and the offline marker', async () => {
    const q = await enqueueClaim(claim('Site visit'))
    expect(q.key).toBeTruthy()
    expect(q.input.id).toBe(q.key)
    expect(q.input.captured_offline).toBe(true)
    expect(q.input.purpose).toBe('Site visit')
    expect(q.input.submit).toBe(true)

    const listed = await listQueued()
    expect(listed).toHaveLength(1)
    expect(listed[0].input.lines[0].amount).toBe(120000)
  })

  it('keeps a given id so a resend cannot create a second claim', async () => {
    const q = await enqueueClaim({ ...claim('Keeps id'), id: 'claim-fixed' })
    expect(q.key).toBe('claim-fixed')
  })

  it('lists oldest first', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-02-03T08:00:00Z'))
    await enqueueClaim(claim('First'))
    vi.setSystemTime(new Date('2026-02-03T09:00:00Z'))
    await enqueueClaim(claim('Second'))
    vi.useRealTimers()
    expect((await listQueued()).map((q) => q.input.purpose)).toEqual(['First', 'Second'])
  })

  it('removes one claim', async () => {
    const a = await enqueueClaim(claim('A'))
    await enqueueClaim(claim('B'))
    await removeQueued(a.key)
    expect((await listQueued()).map((q) => q.input.purpose)).toEqual(['B'])
  })

  it('flushes every claim through the runner and empties the queue', async () => {
    await enqueueClaim(claim('A'))
    await enqueueClaim(claim('B'))
    const run = vi.fn().mockResolvedValue({ type: 'id', id: 'x' })
    const r = await flushQueued(run)
    expect(run).toHaveBeenCalledTimes(2)
    expect(run.mock.calls[0][0].captured_offline).toBe(true)
    expect(r.sent).toHaveLength(2)
    expect(r.failed).toHaveLength(0)
    expect(await listQueued()).toHaveLength(0)
  })

  it('keeps a claim in the queue when sending it fails', async () => {
    const ok = await enqueueClaim(claim('Goes through'))
    const bad = await enqueueClaim(claim('Fails'))
    const run = vi.fn((input: { id: string }) =>
      input.id === bad.key
        ? Promise.reject(new TypeError('Failed to fetch'))
        : Promise.resolve({ type: 'id', id: input.id }),
    )
    const r = await flushQueued(run)
    expect(r.sent).toEqual([ok.key])
    expect(r.failed).toEqual([{ key: bad.key, error: 'Failed to fetch' }])
    const left = await listQueued()
    expect(left.map((q) => q.key)).toEqual([bad.key])
    expect(left[0].last_error).toBe('Failed to fetch')
  })

  it('drops a claim the ledger already has', async () => {
    await enqueueClaim(claim('Arrived before'))
    const run = vi.fn().mockRejectedValue(
      new LedgerError({ code: 'DUPLICATE', message: 'A claim with this id already exists.' }),
    )
    const r = await flushQueued(run)
    expect(r.sent).toHaveLength(1)
    expect(await listQueued()).toHaveLength(0)
  })

  it('shares one pass between concurrent flushes', async () => {
    await enqueueClaim(claim('Once'))
    const run = vi.fn().mockResolvedValue({ type: 'none' })
    await Promise.all([flushQueued(run), flushQueued(run)])
    expect(run).toHaveBeenCalledTimes(1)
  })
})
