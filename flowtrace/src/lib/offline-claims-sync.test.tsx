import 'fake-indexeddb/auto'
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  RETRY_MS,
  closeOfflineDb,
  enqueueClaim,
  listQueued,
  removeQueued,
  useOfflineClaimSync,
} from './offline-claims'
import type { ClaimInput } from './offline-claims'

// The hook's ledger: a command whose sends the test scripts, and a signed-in member.
const ledger = vi.hoisted(() => ({ mutateAsync: vi.fn() }))
vi.mock('./ledger-client', () => ({
  useLedgerCommand: () => ({ mutateAsync: ledger.mutateAsync }),
  useLedgerActor: () => ({
    id: 'alice',
    name: 'Alice',
    can_post: true,
    can_approve: false,
    can_close: false,
  }),
}))

const claim = (purpose: string): ClaimInput => ({
  claim_date: '2026-02-03',
  currency: 'KES',
  fx_rate: 1,
  purpose,
  receipt_names: [],
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

let online = true
let visibility: DocumentVisibilityState = 'visible'

// testing-library's waitFor polls on setInterval, which these tests fake; this polls on
// the real setTimeout.
async function eventually(check: () => Promise<void>, timeoutMs = 2000) {
  const end = Date.now() + timeoutMs
  for (;;) {
    try {
      await check()
      return
    } catch (e) {
      if (Date.now() > end) throw e
      await new Promise((r) => setTimeout(r, 10))
    }
  }
}

async function clear() {
  for (const q of await listQueued()) await removeQueued(q.key)
}

// The first send fails as WebKit reports a dropped network; later sends go through.
async function queueOneThatFailsOnce() {
  await enqueueClaim(claim('Phone claim'))
  ledger.mutateAsync
    .mockRejectedValueOnce(new TypeError('The network connection was lost.'))
    .mockResolvedValue({ type: 'none' })
  const hook = renderHook(() => useOfflineClaimSync())
  await eventually(async () =>
    expect((await listQueued())[0]?.last_error).toBe(
      'The network connection was lost.',
    ),
  )
  return hook
}

beforeEach(async () => {
  // Only the retry interval is faked: fake-indexeddb schedules its own work on real timers.
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  online = true
  visibility = 'visible'
  vi.spyOn(navigator, 'onLine', 'get').mockImplementation(() => online)
  vi.spyOn(document, 'visibilityState', 'get').mockImplementation(
    () => visibility,
  )
  ledger.mutateAsync.mockReset()
  await clear()
})

afterEach(async () => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  await clear()
  await closeOfflineDb()
})

describe('useOfflineClaimSync after a failed send', () => {
  it('sends again on the retry interval while online, with no online event', async () => {
    const { unmount } = await queueOneThatFailsOnce()
    expect(ledger.mutateAsync).toHaveBeenCalledTimes(1)

    await act(() => vi.advanceTimersByTimeAsync(RETRY_MS))
    await eventually(async () => expect(await listQueued()).toHaveLength(0))
    expect(ledger.mutateAsync).toHaveBeenCalledTimes(2)
    // The same claim both times, so the ledger answers DUPLICATE if the first one did land.
    const ids = ledger.mutateAsync.mock.calls.map(
      ([cmd]) => (cmd as { input: { id: string } }).input.id,
    )
    expect(new Set(ids).size).toBe(1)
    unmount()
  })

  it('sends again when the page becomes visible', async () => {
    const { unmount } = await queueOneThatFailsOnce()
    visibility = 'hidden'
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'))
    })
    expect(ledger.mutateAsync).toHaveBeenCalledTimes(1)

    visibility = 'visible'
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'))
    })
    await eventually(async () => expect(await listQueued()).toHaveLength(0))
    expect(ledger.mutateAsync).toHaveBeenCalledTimes(2)
    unmount()
  })

  it('does not retry while the phone is offline', async () => {
    const { unmount } = await queueOneThatFailsOnce()
    online = false
    await act(() => vi.advanceTimersByTimeAsync(RETRY_MS * 3))
    expect(ledger.mutateAsync).toHaveBeenCalledTimes(1)
    expect(await listQueued()).toHaveLength(1)
    unmount()
  })
})
