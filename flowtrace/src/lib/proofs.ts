// Proof files: the Cloudflare Worker in front of R2 (workers/proofs).
//
// With VITE_PROOFS_URL set, files go to the Worker and verification records
// keep a `proof:<key>` reference instead of a URL; viewers trade it for a
// short-lived signed URL with GET /file/<key>. Unset, files stay in this
// browser session (blob URLs from the demo API) so the demo needs no
// backend. Proof files never go to Supabase Storage.
import { useQuery } from '@tanstack/react-query'
import { AUTH_MODE } from './auth'
import {
  uploadVerificationFile as keepLocalFile,
  deleteVerificationFile as releaseLocalFile,
} from './demo/api'

/** What the Worker accepts: 25 MB, images and PDF only. */
export const MAX_PROOF_MB = 25
export const PROOF_STORE_TYPES = [
  'image/jpeg',
  'image/png',
  'image/webp',
  'image/gif',
  'image/heic',
  'application/pdf',
]

export type ProofLinkKind = 'claim' | 'bill' | 'transaction'

export interface UploadedProof {
  key: string
  sha256: string
  size: number
  content_type: string
  thumbnail_key: string | null
}

export interface SignedProofUrl {
  url: string
  /** Unix seconds. */
  expires_at: number
}

// ---- References -----------------------------------------------------------

const REF_PREFIX = 'proof:'

/** True for a `proof:<key>` reference to a Worker object. */
export function isProofRef(value: string | null | undefined): value is string {
  return (
    typeof value === 'string' &&
    value.startsWith(REF_PREFIX) &&
    value.length > REF_PREFIX.length
  )
}

export function proofRef(key: string): string {
  return REF_PREFIX + key
}

/** The Worker object key in a reference, or null for any other URL. */
export function proofKey(ref: string | null | undefined): string | null {
  return isProofRef(ref) ? ref.slice(REF_PREFIX.length) : null
}

// ---- Config ---------------------------------------------------------------

function baseUrl(): string | null {
  const url = (import.meta.env.VITE_PROOFS_URL as string | undefined) ?? ''
  return url ? url.replace(/\/+$/, '') : null
}

/** True when proof files go to the Worker. */
export function proofsEnabled(): boolean {
  return baseUrl() !== null
}

function requireBaseUrl(): string {
  const url = baseUrl()
  if (!url) {
    throw new Error('Set VITE_PROOFS_URL to store and open proof files.')
  }
  return url
}

/**
 * The module organisation a proof belongs to. Mirrors ledger-client.ts:
 * VITE_SPACETIMEDB_ORG when set, else the organisation the app has.
 */
export function proofOrg(organizationId: string): string {
  return (
    (import.meta.env.VITE_SPACETIMEDB_ORG as string | undefined) ||
    organizationId
  )
}

// ---- Token ----------------------------------------------------------------

// Same key as TOKEN_KEY in src/lib/ledger/spacetime-backend.ts:29, which is
// not exported (and importing that module would pull in the SpacetimeDB SDK).
export const SPACETIME_TOKEN_KEY = 'flowtrace.spacetimedb.token'

let workosTokenProvider: (() => Promise<string | null>) | null = null

/**
 * Registers where WorkOS access tokens come from.
 * AUTH1: register the AuthKit access token provider here, once, when the
 * signed-in session is available (e.g. in AppAuthProvider for workos mode).
 * Pass null to clear it on sign-out.
 */
export function setWorkosTokenProvider(
  fn: (() => Promise<string | null>) | null,
) {
  workosTokenProvider = fn
}

/** The bearer token the Worker checks for this user. */
export async function proofToken(
  mode: 'dummy' | 'workos' = AUTH_MODE,
): Promise<string> {
  if (mode === 'workos') {
    if (!workosTokenProvider) {
      throw new Error(
        'FlowTrace could not confirm who you are yet. Sign in again and retry.',
      )
    }
    const token = await workosTokenProvider()
    if (!token) {
      throw new Error('Your session has ended. Sign in again and retry.')
    }
    return token
  }
  let token: string | null = null
  try {
    token = localStorage.getItem(SPACETIME_TOKEN_KEY)
  } catch {
    // Blocked storage: treated as no identity yet.
  }
  if (!token) {
    throw new Error(
      'Connect to the books first so FlowTrace can confirm who you are.',
    )
  }
  return token
}

// ---- Worker calls ---------------------------------------------------------

/** A failed Worker call; `status` is absent when the Worker was unreachable. */
export class ProofStoreError extends Error {
  readonly status?: number
  constructor(message: string, status?: number) {
    super(message)
    this.name = 'ProofStoreError'
    this.status = status
  }
}

async function call(path: string, init: RequestInit = {}): Promise<unknown> {
  const url = requireBaseUrl() + path
  const token = await proofToken()
  const headers = new Headers(init.headers)
  headers.set('Authorization', `Bearer ${token}`)
  let response: Response
  try {
    response = await fetch(url, { ...init, headers })
  } catch {
    throw new ProofStoreError(
      'Could not reach the proof store. Check your connection.',
    )
  }
  let body: unknown = null
  try {
    body = await response.json()
  } catch {
    // Not JSON; reported below by status.
  }
  if (!response.ok) {
    const message =
      body && typeof (body as { error?: unknown }).error === 'string'
        ? (body as { error: string }).error
        : `The proof store answered ${response.status}.`
    throw new ProofStoreError(message, response.status)
  }
  return body
}

function unexpected(): Error {
  return new Error('The proof store sent an unexpected answer.')
}

/** POST /upload: stores one file and links it to a claim, bill or transaction. */
export async function uploadProof(input: {
  org: string
  linkKind: ProofLinkKind
  linkId: string
  file: File
}): Promise<UploadedProof> {
  const form = new FormData()
  form.set('org', input.org)
  form.set('link_kind', input.linkKind)
  form.set('link_id', input.linkId)
  form.set('file', input.file, input.file.name)
  const body = (await call('/upload', {
    method: 'POST',
    body: form,
  })) as Partial<UploadedProof> | null
  if (
    !body ||
    typeof body.key !== 'string' ||
    typeof body.sha256 !== 'string' ||
    typeof body.size !== 'number' ||
    typeof body.content_type !== 'string' ||
    !(typeof body.thumbnail_key === 'string' || body.thumbnail_key === null)
  ) {
    throw unexpected()
  }
  return body as UploadedProof
}

/** GET /file/<key>: a signed URL for the file, valid about 10 minutes. */
export async function proofSignedUrl(key: string): Promise<SignedProofUrl> {
  const path = key.split('/').map(encodeURIComponent).join('/')
  const body = (await call(`/file/${path}`)) as Partial<SignedProofUrl> | null
  if (
    !body ||
    typeof body.url !== 'string' ||
    typeof body.expires_at !== 'number'
  ) {
    throw unexpected()
  }
  return body as SignedProofUrl
}

// ---- What the components call ---------------------------------------------

export interface StoredProofFile {
  /** `proof:<key>` on the Worker, else a session-local URL. */
  url: string
  thumbnailUrl: string | null
  objectKey?: string
  sha256?: string
}

/**
 * Stores a proof file for a transaction: on the Worker when VITE_PROOFS_URL
 * is set, else as a session-local file for the demo.
 */
export async function storeProofFile(
  organizationId: string,
  transactionId: string,
  file: File,
): Promise<StoredProofFile> {
  if (!proofsEnabled()) {
    const url = await keepLocalFile(organizationId, transactionId, file)
    return { url, thumbnailUrl: null }
  }
  if (!PROOF_STORE_TYPES.includes(file.type)) {
    throw new Error('Proof files must be a PDF or an image')
  }
  if (file.size > MAX_PROOF_MB * 1024 * 1024) {
    throw new Error(`The limit is ${MAX_PROOF_MB} MB`)
  }
  const stored = await uploadProof({
    org: proofOrg(organizationId),
    linkKind: 'transaction',
    linkId: transactionId,
    file,
  })
  return {
    url: proofRef(stored.key),
    thumbnailUrl: stored.thumbnail_key ? proofRef(stored.thumbnail_key) : null,
    objectKey: stored.key,
    sha256: stored.sha256,
  }
}

/**
 * Lets go of a file the user removed before attaching it. Worker objects are
 * left alone: they are content-addressed, so another proof may hold the same
 * key, and the Worker has no delete route.
 */
export async function releaseProofFile(url: string): Promise<void> {
  if (isProofRef(url)) return
  await releaseLocalFile(url)
}

// Signed URLs last 10 minutes; refetch well before that.
const SIGNED_URL_STALE_MS = 8 * 60 * 1000

export type ProofSrc =
  | { status: 'none'; src: null }
  | { status: 'loading'; src: null }
  | { status: 'ready'; src: string }
  | { status: 'error'; src: null; error: string }

/**
 * A usable src for a proof file: a `proof:` reference becomes a signed URL,
 * any other URL (blob:, https:, demo://) is returned as it is.
 */
export function useProofSrc(ref: string | null | undefined): ProofSrc {
  const key = proofKey(ref)
  const query = useQuery({
    queryKey: ['proof-url', key],
    queryFn: () => proofSignedUrl(key!),
    enabled: key !== null,
    staleTime: SIGNED_URL_STALE_MS,
    gcTime: SIGNED_URL_STALE_MS,
    refetchInterval: SIGNED_URL_STALE_MS,
    // Retry once if the Worker was unreachable or failed; a refusal stands.
    retry: (count, err) =>
      count < 1 &&
      err instanceof ProofStoreError &&
      (err.status === undefined || err.status >= 500),
  })
  if (!ref) return { status: 'none', src: null }
  if (key === null) return { status: 'ready', src: ref }
  if (query.data) return { status: 'ready', src: query.data.url }
  if (query.isError) {
    return {
      status: 'error',
      src: null,
      error: query.error.message,
    }
  }
  return { status: 'loading', src: null }
}
