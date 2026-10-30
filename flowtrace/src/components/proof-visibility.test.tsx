import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { SPACETIME_TOKEN_KEY } from '@/lib/proofs'
import { decodeVerification } from '@/lib/ledger/transparency-model'
import { VerificationGallery } from '@/components/verification-gallery'
import { VerificationViewer } from '@/components/verification-viewer'

// One proof as the module stores it, and as `my_verification` hands it to three readers
// (plan.rs:628-660, ADR 0004 rule table): a member of a group with full proofs, a member of a
// group with summaries only, and a stranger (no group in the organisation), who gets no row.
const SHA = 'd'.repeat(64)
const KEY = `org/rw1/${SHA}`
const STORED = {
  id: 'v1',
  transaction_id: 'tx-1',
  type: 'receipt',
  object_key: KEY,
  sha256: SHA,
  thumbnail_key: null,
  ocr_data: null,
  metadata: { file_name: 'r.jpg', mime_type: 'image/jpeg' },
  status: 'pending',
  verified_by: null,
  verified_at: null,
  created_at: '2026-10-30T10:00:00Z',
  trust_points: 5,
  visibility_level: 'full',
  executor_name: null,
  gps_latitude: null,
  gps_longitude: null,
  gps_accuracy: null,
  location_name: null,
  description: 'Cement receipt',
  created_by: 'u-a',
}

/** The row `redact` gives at a level (plan.rs:639). */
const seenAt = (level: 'full' | 'summary') =>
  decodeVerification({
    data: JSON.stringify(
      level === 'full'
        ? { ...STORED, visible_as: 'full' }
        : { ...STORED, object_key: null, thumbnail_key: null, visible_as: 'summary' },
    ),
  })

const fetchMock = vi.fn<typeof fetch>()

function withQuery(ui: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

beforeEach(() => {
  fetchMock.mockReset()
  fetchMock.mockResolvedValue(
    new Response(
      JSON.stringify({ url: 'https://signed.example.test/r.jpg', expires_at: 2e9 }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    ),
  )
  vi.stubGlobal('fetch', fetchMock)
  const items = new Map<string, string>([[SPACETIME_TOKEN_KEY, 'stdb-token']])
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => items.get(k) ?? null,
    setItem: (k: string, v: string) => void items.set(k, v),
    removeItem: (k: string) => void items.delete(k),
  })
  vi.stubEnv('VITE_PROOFS_URL', 'https://proofs.example.test')
})

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
  vi.unstubAllEnvs()
})

describe('a proof at each reader’s level', () => {
  it('a member of a group with full proofs gets the original file', async () => {
    withQuery(<VerificationViewer verification={seenAt('full')} onClose={() => {}} />)
    expect(await screen.findByRole('link', { name: /download original/i })).toBeTruthy()
    expect(screen.queryByText(/sees that this proof is attached/i)).toBeNull()
  })

  it('a member of a group with summaries sees that proof exists, not the file', async () => {
    withQuery(<VerificationViewer verification={seenAt('summary')} onClose={() => {}} />)
    expect(
      screen.getAllByText(/your access group sees that this proof is attached, not the file/i)
        .length,
    ).toBeGreaterThan(0)
    expect(screen.queryByRole('link', { name: /download/i })).toBeNull()
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([u]) => String(u).includes('/file/')),
      ).toBe(false),
    )
  })

  it('a stranger gets no row, so the transaction shows no proof', () => {
    withQuery(<VerificationGallery verifications={[]} showAddButton={false} />)
    expect(screen.getByText(/no proof attached yet/i)).toBeTruthy()
    expect(screen.queryByText(/cement receipt/i)).toBeNull()
  })

  it('offers publishing only when the page passes the action, and not without a file', () => {
    const onPublish = vi.fn()
    const { unmount } = withQuery(
      <VerificationViewer
        verification={seenAt('full')}
        onClose={() => {}}
        onPublish={onPublish}
      />,
    )
    screen.getByRole('button', { name: /publish this proof/i }).click()
    expect(onPublish).toHaveBeenCalledWith(expect.objectContaining({ id: 'v1' }), true)
    unmount()
    withQuery(
      <VerificationViewer
        verification={seenAt('summary')}
        onClose={() => {}}
        onPublish={onPublish}
      />,
    )
    expect(screen.queryByRole('button', { name: /publish this proof/i })).toBeNull()
  })
})
