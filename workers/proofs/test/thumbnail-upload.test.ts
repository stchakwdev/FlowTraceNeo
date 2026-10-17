/**
 * Upload with an Images binding present. Miniflare 4 has no stub for the binding, so this calls
 * the Worker's `fetch` directly with a stub `IMAGES`, an in-memory R2 and a stubbed `fetch`.
 */
import { afterAll, beforeAll, expect, test } from 'bun:test'
import { clearAuthCache } from '../src/auth'
import type { Env, ImagesLike } from '../src/env'
import worker from '../src/index'
import { ID, JPEG, ORG, PDF, bytesOf, ledgerStub, memberRow, multipart, newState, stdbToken } from './fixtures'

interface Stored {
  bytes: Uint8Array
  httpMetadata?: { contentType?: string }
  customMetadata?: Record<string, string>
}

/** Just enough of R2 for the upload route. */
function memoryR2() {
  const objects = new Map<string, Stored>()
  return {
    objects,
    async head(key: string) {
      const o = objects.get(key)
      return o ? { key, size: o.bytes.byteLength, customMetadata: o.customMetadata } : null
    },
    async put(key: string, value: ArrayBuffer | Uint8Array, opts: Omit<Stored, 'bytes'> = {}) {
      objects.set(key, { bytes: new Uint8Array(value), ...opts })
      return { key }
    },
  }
}

const WEBP = bytesOf('RIFF\x00\x00\x00\x00WEBPVP8 ', 48)
const images: ImagesLike = {
  input: () => ({
    transform: () => ({ output: async () => ({ response: () => new Response(WEBP) }) }),
  }),
}

const token = stdbToken(ID.alice, 'thumb-test')
const state = newState()
state.tokens.set(token, ID.alice)
state.members.push({ org: ORG, row: memberRow(ID.alice, 'alice') })
const realFetch = globalThis.fetch

beforeAll(() => {
  clearAuthCache()
  const stub = ledgerStub(state, { moduleToken: 'mt' })
  globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) =>
    stub(new Request(input, init))) as typeof fetch
})
afterAll(() => {
  globalThis.fetch = realFetch
})

function env(r2: ReturnType<typeof memoryR2>, withImages: boolean): Env {
  return {
    PROOFS: r2 as unknown as R2Bucket,
    IMAGES: withImages ? images : undefined,
    SPACETIMEDB_URL: 'https://stdb.test',
    SPACETIMEDB_DATABASE: 'flowtrace-ledger-dev',
    MODULE_TOKEN: 'mt',
    URL_SIGNING_KEY: 'k',
  }
}

function upload(bytes: Uint8Array, name: string) {
  const { body, contentType } = multipart(
    { org: ORG, link_kind: 'bill', link_id: 'bill-7' },
    { name, type: 'application/octet-stream', bytes },
  )
  return new Request('https://proofs.test/upload', {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': contentType },
    body,
  })
}

test('an image gets a WebP thumbnail when the binding is present', async () => {
  const r2 = memoryR2()
  const res = await worker.fetch(upload(JPEG, 'r.jpg'), env(r2, true))
  expect(res.status).toBe(201)
  const body = (await res.json()) as { sha256: string; thumbnail_key: string }
  expect(body.thumbnail_key).toBe(`org/${ORG}/thumb/${body.sha256}.webp`)
  const thumb = r2.objects.get(body.thumbnail_key)!
  expect(thumb.httpMetadata?.contentType).toBe('image/webp')
  expect(Array.from(thumb.bytes)).toEqual(Array.from(WEBP))
  expect(r2.objects.size).toBe(2)
})

test('without the binding only the original is stored', async () => {
  const r2 = memoryR2()
  const res = await worker.fetch(upload(JPEG, 'r.jpg'), env(r2, false))
  expect(((await res.json()) as { thumbnail_key: unknown }).thumbnail_key).toBeNull()
  expect(r2.objects.size).toBe(1)
})

test('a PDF gets no thumbnail even with the binding', async () => {
  const r2 = memoryR2()
  const res = await worker.fetch(upload(PDF, 'i.pdf'), env(r2, true))
  expect(res.status).toBe(201)
  expect(((await res.json()) as { thumbnail_key: unknown }).thumbnail_key).toBeNull()
  expect(r2.objects.size).toBe(1)
})

test('a second upload of the same bytes does not rewrite the object', async () => {
  const r2 = memoryR2()
  await worker.fetch(upload(JPEG, 'first.jpg'), env(r2, false))
  await worker.fetch(upload(JPEG, 'second.jpg'), env(r2, false))
  const [only] = [...r2.objects.values()]
  expect(only.customMetadata?.original_name).toBe('first.jpg')
})
