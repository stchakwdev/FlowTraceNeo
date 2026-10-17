import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { Verification } from '@/lib/supabase'
import { SPACETIME_TOKEN_KEY } from '@/lib/proofs'
import { FileUpload } from '@/components/file-upload'
import { VerificationViewer } from '@/components/verification-viewer'

const BASE = 'https://proofs.example.test'
const SHA = 'c'.repeat(64)
const KEY = `org/org-a/${SHA}`
const SIGNED = 'https://signed.example.test/file?sig=xyz'

const fetchMock = vi.fn<typeof fetch>()

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function memoryStorage(): Storage {
  const items = new Map<string, string>()
  return {
    get length() {
      return items.size
    },
    clear: () => items.clear(),
    getItem: (k) => items.get(k) ?? null,
    key: (i) => [...items.keys()][i] ?? null,
    removeItem: (k) => void items.delete(k),
    setItem: (k, v) => void items.set(k, String(v)),
  }
}

function withQuery(ui: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

function proof(over: Partial<Verification> = {}): Verification {
  return {
    id: 'v1',
    transaction_id: 'tx-1',
    type: 'receipt',
    file_url: `proof:${KEY}`,
    ocr_data: null,
    metadata: { file_name: 'r.jpg', mime_type: 'image/jpeg', size: 10 },
    status: 'pending',
    verified_by: null,
    verified_at: null,
    created_at: '2026-10-01T10:00:00Z',
    trust_points: 0,
    visibility_level: 'full',
    thumbnail_url: null,
    executor_name: null,
    gps_latitude: null,
    gps_longitude: null,
    gps_accuracy: null,
    location_name: null,
    description: 'Cement receipt',
    created_by: null,
    ...over,
  }
}

beforeEach(() => {
  fetchMock.mockReset()
  vi.stubGlobal('fetch', fetchMock)
  const storage = memoryStorage()
  storage.setItem(SPACETIME_TOKEN_KEY, 'stdb-token')
  vi.stubGlobal('localStorage', storage)
  vi.stubEnv('VITE_PROOFS_URL', BASE)
  vi.stubEnv('VITE_SPACETIMEDB_ORG', '')
})

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
  vi.unstubAllEnvs()
})

describe('VerificationViewer with a stored proof', () => {
  it('shows and downloads the file through a signed URL', async () => {
    fetchMock.mockResolvedValue(
      json(200, { url: SIGNED, expires_at: 1_900_000_000 }),
    )
    withQuery(<VerificationViewer verification={proof()} onClose={() => {}} />)
    expect(screen.getByText('Opening the file…')).toBeTruthy()
    const img = await screen.findByAltText('Cement receipt')
    expect(img.getAttribute('src')).toBe(SIGNED)
    expect(
      screen.getByText('Download original').closest('a')?.getAttribute('href'),
    ).toBe(SIGNED)
    expect(fetchMock.mock.calls[0][0]).toBe(`${BASE}/file/${KEY}`)
  })

  it("says the file could not be loaded with the Worker's reason", async () => {
    fetchMock.mockResolvedValue(json(403, { error: 'Not your organisation.' }))
    withQuery(<VerificationViewer verification={proof()} onClose={() => {}} />)
    expect(await screen.findByText('The file could not be loaded')).toBeTruthy()
    expect(screen.getByText(/Not your organisation\./)).toBeTruthy()
    expect(screen.queryByText('Download original')).toBeNull()
  })

  it('uses any other URL as it is', () => {
    withQuery(
      <VerificationViewer
        verification={proof({ file_url: 'https://cdn.example.test/r.jpg' })}
        onClose={() => {}}
      />,
    )
    expect(screen.getByAltText('Cement receipt').getAttribute('src')).toBe(
      'https://cdn.example.test/r.jpg',
    )
    expect(fetchMock).not.toHaveBeenCalled()
  })
})

describe('FileUpload with VITE_PROOFS_URL set', () => {
  it('uploads to the Worker and removes without deleting', async () => {
    fetchMock.mockResolvedValue(
      json(201, {
        key: KEY,
        sha256: SHA,
        size: 12,
        content_type: 'image/jpeg',
        thumbnail_key: `org/org-a/thumb/${SHA}.webp`,
      }),
    )
    const onUploadComplete = vi.fn()
    const { container } = withQuery(
      <FileUpload
        organizationId="org-a"
        transactionId="tx-1"
        verificationType="receipt"
        onUploadComplete={onUploadComplete}
      />,
    )
    const input = container.querySelector<HTMLInputElement>('input[type=file]')!
    const file = new File(['jpeg bytes!!'], 'r.jpg', { type: 'image/jpeg' })
    fireEvent.change(input, { target: { files: [file] } })

    await waitFor(() => expect(onUploadComplete).toHaveBeenCalledTimes(1))
    expect(onUploadComplete.mock.calls[0][0]).toMatchObject({
      url: `proof:${KEY}`,
      thumbnailUrl: `proof:org/org-a/thumb/${SHA}.webp`,
      status: 'success',
    })
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${BASE}/upload`)
    expect((init?.body as FormData).get('link_kind')).toBe('transaction')

    fireEvent.click(screen.getByLabelText('Remove r.jpg'))
    await waitFor(() => expect(screen.queryByText('r.jpg')).toBeNull())
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it("shows the Worker's refusal on the file row", async () => {
    fetchMock.mockResolvedValue(json(413, { error: 'File is over 25 MB.' }))
    const { container } = withQuery(
      <FileUpload
        organizationId="org-a"
        transactionId="tx-1"
        verificationType="receipt"
      />,
    )
    const input = container.querySelector<HTMLInputElement>('input[type=file]')!
    fireEvent.change(input, {
      target: { files: [new File(['x'], 'r.jpg', { type: 'image/jpeg' })] },
    })
    expect(
      await screen.findByText(/r\.jpg did not upload: File is over 25 MB\./),
    ).toBeTruthy()
  })
})
