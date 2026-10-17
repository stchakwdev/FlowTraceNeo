import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  MAX_PROOF_MB,
  SPACETIME_TOKEN_KEY,
  isProofRef,
  proofKey,
  proofOrg,
  proofRef,
  proofSignedUrl,
  proofToken,
  releaseProofFile,
  setWorkosTokenProvider,
  storeProofFile,
  uploadProof,
} from './proofs'

const BASE = 'https://proofs.example.test'
const KEY = 'org/org-a/' + 'a'.repeat(64)
const THUMB = 'org/org-a/thumb/' + 'a'.repeat(64) + '.webp'

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function receipt(type = 'image/jpeg') {
  return new File(['receipt bytes'], 'receipt.jpg', { type })
}

const fetchMock = vi.fn<typeof fetch>()

// The Node test runtime has no working localStorage, so stub a small one.
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

beforeEach(() => {
  fetchMock.mockReset()
  vi.stubGlobal('fetch', fetchMock)
  vi.stubGlobal('localStorage', memoryStorage())
  vi.stubEnv('VITE_PROOFS_URL', BASE)
  vi.stubEnv('VITE_SPACETIMEDB_ORG', '')
  localStorage.clear()
  localStorage.setItem(SPACETIME_TOKEN_KEY, 'stdb-token')
  setWorkosTokenProvider(null)
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.unstubAllEnvs()
})

describe('proof refs', () => {
  it('round-trips a Worker key', () => {
    expect(proofRef(KEY)).toBe(`proof:${KEY}`)
    expect(proofKey(proofRef(KEY))).toBe(KEY)
    expect(isProofRef(proofRef(KEY))).toBe(true)
  })

  it('leaves other URLs alone', () => {
    for (const url of [
      'blob:http://x/1',
      'https://x/a.png',
      'demo://uploads/a',
    ]) {
      expect(isProofRef(url)).toBe(false)
      expect(proofKey(url)).toBeNull()
    }
    expect(isProofRef(null)).toBe(false)
    expect(isProofRef('proof:')).toBe(false)
  })
})

describe('proofToken', () => {
  it('uses the SpacetimeDB token in dummy mode', async () => {
    await expect(proofToken('dummy')).resolves.toBe('stdb-token')
  })

  it('asks the user to connect when no SpacetimeDB token is stored', async () => {
    localStorage.clear()
    await expect(proofToken('dummy')).rejects.toThrow(
      /Connect to the books first/,
    )
  })

  it('awaits the registered WorkOS provider in workos mode', async () => {
    setWorkosTokenProvider(() => Promise.resolve('workos-access'))
    await expect(proofToken('workos')).resolves.toBe('workos-access')
  })

  it('errors in workos mode with no provider or no token', async () => {
    await expect(proofToken('workos')).rejects.toThrow(/sign/i)
    setWorkosTokenProvider(() => Promise.resolve(null))
    await expect(proofToken('workos')).rejects.toThrow(/Sign in again/)
  })
})

describe('proofOrg', () => {
  it('prefers VITE_SPACETIMEDB_ORG over the app organisation', () => {
    expect(proofOrg('app-org')).toBe('app-org')
    vi.stubEnv('VITE_SPACETIMEDB_ORG', 'org-a')
    expect(proofOrg('app-org')).toBe('org-a')
  })
})

describe('uploadProof', () => {
  const body = {
    key: KEY,
    sha256: 'a'.repeat(64),
    size: 13,
    content_type: 'image/jpeg',
    thumbnail_key: THUMB,
  }

  it('posts the multipart fields with the bearer token', async () => {
    fetchMock.mockResolvedValue(json(201, body))
    const file = receipt()
    const out = await uploadProof({
      org: 'org-a',
      linkKind: 'transaction',
      linkId: 'tx-1',
      file,
    })
    expect(out).toEqual(body)
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${BASE}/upload`)
    expect(init?.method).toBe('POST')
    expect(new Headers(init?.headers).get('Authorization')).toBe(
      'Bearer stdb-token',
    )
    const form = init?.body as FormData
    expect(form.get('org')).toBe('org-a')
    expect(form.get('link_kind')).toBe('transaction')
    expect(form.get('link_id')).toBe('tx-1')
    const sent = form.get('file') as File
    expect(sent.name).toBe('receipt.jpg')
    expect(sent.size).toBe(file.size)
  })

  it("surfaces the Worker's error text", async () => {
    fetchMock.mockResolvedValue(
      json(403, { error: 'You are not a member of org-a.' }),
    )
    await expect(
      uploadProof({
        org: 'org-a',
        linkKind: 'transaction',
        linkId: 'tx-1',
        file: receipt(),
      }),
    ).rejects.toThrow('You are not a member of org-a.')
  })

  it('explains a non-JSON failure and an unreachable Worker', async () => {
    fetchMock.mockResolvedValueOnce(new Response('oops', { status: 502 }))
    await expect(
      uploadProof({
        org: 'org-a',
        linkKind: 'transaction',
        linkId: 'tx-1',
        file: receipt(),
      }),
    ).rejects.toThrow(/502/)
    fetchMock.mockRejectedValueOnce(new TypeError('Failed to fetch'))
    await expect(
      uploadProof({
        org: 'org-a',
        linkKind: 'transaction',
        linkId: 'tx-1',
        file: receipt(),
      }),
    ).rejects.toThrow(/Could not reach the proof store/)
  })

  it('rejects a malformed success body', async () => {
    fetchMock.mockResolvedValue(json(201, { key: 1 }))
    await expect(
      uploadProof({
        org: 'org-a',
        linkKind: 'transaction',
        linkId: 'tx-1',
        file: receipt(),
      }),
    ).rejects.toThrow(/unexpected/)
  })

  it('does not call the Worker without a token', async () => {
    localStorage.clear()
    await expect(
      uploadProof({
        org: 'org-a',
        linkKind: 'transaction',
        linkId: 'tx-1',
        file: receipt(),
      }),
    ).rejects.toThrow(/Connect to the books first/)
    expect(fetchMock).not.toHaveBeenCalled()
  })
})

describe('proofSignedUrl', () => {
  it('GETs /file/<key> with each segment encoded', async () => {
    fetchMock.mockResolvedValue(
      json(200, {
        url: 'https://signed.test/f?sig=1',
        expires_at: 1_900_000_000,
      }),
    )
    const out = await proofSignedUrl('org/org a/' + 'b'.repeat(64))
    expect(out).toEqual({
      url: 'https://signed.test/f?sig=1',
      expires_at: 1_900_000_000,
    })
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${BASE}/file/org/org%20a/${'b'.repeat(64)}`)
    expect(new Headers(init?.headers).get('Authorization')).toBe(
      'Bearer stdb-token',
    )
  })

  it('surfaces a 404 error message', async () => {
    fetchMock.mockResolvedValue(json(404, { error: 'No such proof.' }))
    await expect(proofSignedUrl(KEY)).rejects.toThrow('No such proof.')
  })

  it('needs VITE_PROOFS_URL', async () => {
    vi.stubEnv('VITE_PROOFS_URL', '')
    await expect(proofSignedUrl(KEY)).rejects.toThrow(/VITE_PROOFS_URL/)
  })
})

describe('storeProofFile', () => {
  it('stores through the Worker and returns proof refs', async () => {
    vi.stubEnv('VITE_SPACETIMEDB_ORG', 'org-a')
    vi.stubEnv('VITE_PROOFS_URL', `${BASE}/`)
    fetchMock.mockResolvedValue(
      json(201, {
        key: KEY,
        sha256: 'a'.repeat(64),
        size: 13,
        content_type: 'image/jpeg',
        thumbnail_key: THUMB,
      }),
    )
    const out = await storeProofFile('app-org', 'tx-1', receipt())
    expect(out).toEqual({
      url: `proof:${KEY}`,
      thumbnailUrl: `proof:${THUMB}`,
      objectKey: KEY,
      sha256: 'a'.repeat(64),
    })
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${BASE}/upload`)
    const form = init?.body as FormData
    expect(form.get('org')).toBe('org-a')
    expect(form.get('link_id')).toBe('tx-1')
  })

  it('refuses types and sizes the Worker will not take, before uploading', async () => {
    await expect(
      storeProofFile('org-a', 'tx-1', receipt('application/msword')),
    ).rejects.toThrow(/PDF or an image/)
    const big = new File([new Uint8Array(1)], 'big.pdf', {
      type: 'application/pdf',
    })
    Object.defineProperty(big, 'size', {
      value: MAX_PROOF_MB * 1024 * 1024 + 1,
    })
    await expect(storeProofFile('org-a', 'tx-1', big)).rejects.toThrow(/25 MB/)
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('keeps a session-local file when VITE_PROOFS_URL is unset', async () => {
    vi.stubEnv('VITE_PROOFS_URL', '')
    const out = await storeProofFile('org-a', 'tx-1', receipt())
    expect(isProofRef(out.url)).toBe(false)
    expect(out.url).toMatch(/^(blob:|demo:\/\/)/)
    expect(out.thumbnailUrl).toBeNull()
    expect(fetchMock).not.toHaveBeenCalled()
  })
})

describe('releaseProofFile', () => {
  it('never calls the Worker for a proof ref', async () => {
    await releaseProofFile(`proof:${KEY}`)
    expect(fetchMock).not.toHaveBeenCalled()
  })
})
