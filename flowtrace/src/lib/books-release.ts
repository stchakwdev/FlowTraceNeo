// The latest FlowTrace Books release, read from the GitHub Releases API. A v* tag publishes
// the Linux and macOS archives and a SHA256SUMS file there (.github/workflows/rust.yml, job
// github-release; rust/docs/module-release.md, section 7). Anything other than a release with
// at least one archive reads as "no release", so the /books page never shows an empty table.

export const BOOKS_REPO = 'stchakwdev/FlowTraceNeo'
export const BOOKS_REPO_URL = `https://github.com/${BOOKS_REPO}`
const LATEST_URL = `https://api.github.com/repos/${BOOKS_REPO}/releases/latest`

/** How long one read of the API is reused; also the route loader's staleTime. */
export const RELEASE_CACHE_MS = 10 * 60 * 1000
/** A failed read is retried sooner than a good one is refreshed. */
const FAILURE_CACHE_MS = 60 * 1000
const FETCH_TIMEOUT_MS = 4000

// rust/tools/release.sh: flowtrace-books-<version>-<os>-<arch>.tar.gz
const ARCHIVE = /^flowtrace-books-(.+)-(linux|macos)-([a-z0-9_]+)\.tar\.gz$/

export interface BooksAsset {
  name: string
  platform: 'Linux' | 'macOS'
  arch: string
  size: number
  url: string
}

export type BooksRelease =
  | {
      kind: 'available'
      tag: string
      version: string
      publishedAt: string
      url: string
      checksumsUrl: string | null
      assets: Array<BooksAsset>
    }
  | { kind: 'none' }

const NONE: BooksRelease = { kind: 'none' }

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

/** Reads the body of GET /repos/{owner}/{repo}/releases/latest. */
export function parseLatestRelease(body: unknown): BooksRelease {
  if (!isRecord(body) || body.draft === true || body.prerelease === true)
    return NONE
  const {
    tag_name: tag,
    html_url: url,
    published_at: publishedAt,
    assets,
  } = body
  if (
    typeof tag !== 'string' ||
    typeof url !== 'string' ||
    !Array.isArray(assets)
  ) {
    return NONE
  }

  const archives: Array<BooksAsset> = []
  let checksumsUrl: string | null = null
  for (const asset of assets) {
    if (!isRecord(asset)) continue
    const { name, size, browser_download_url: link } = asset
    if (typeof name !== 'string' || typeof link !== 'string') continue
    if (name === 'SHA256SUMS') {
      checksumsUrl = link
      continue
    }
    const match = ARCHIVE.exec(name)
    if (!match || typeof size !== 'number') continue
    archives.push({
      name,
      platform: match[2] === 'linux' ? 'Linux' : 'macOS',
      arch: match[3],
      size,
      url: link,
    })
  }
  if (archives.length === 0) return NONE
  archives.sort(
    (a, b) =>
      a.platform.localeCompare(b.platform) || a.arch.localeCompare(b.arch),
  )

  return {
    kind: 'available',
    tag,
    version: tag.replace(/^v/, ''),
    publishedAt: typeof publishedAt === 'string' ? publishedAt : '',
    url,
    checksumsUrl,
    assets: archives,
  }
}

interface LoaderOptions {
  fetchImpl?: (input: string, init?: RequestInit) => Promise<Response>
  now?: () => number
}

/**
 * A cached, failure-tolerant reader of the latest release. It never throws: a 404 (no release
 * yet, or the repository is private), a rate limit, a timeout or a network error all give
 * "no release", or the last good release when there was one.
 */
export function createReleaseLoader({
  fetchImpl = (input, init) => fetch(input, init),
  now = Date.now,
}: LoaderOptions = {}) {
  let cached: { value: BooksRelease; until: number } | null = null
  let lastGood: BooksRelease | null = null

  return async function loadLatestRelease(): Promise<BooksRelease> {
    if (cached && now() < cached.until) return cached.value
    let value: BooksRelease
    let ttl = RELEASE_CACHE_MS
    try {
      const res = await fetchImpl(LATEST_URL, {
        headers: {
          Accept: 'application/vnd.github+json',
          'X-GitHub-Api-Version': '2022-11-28',
          'User-Agent': 'flowtrace-books-page',
        },
        signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
      })
      if (res.status === 404) {
        value = NONE
      } else if (res.ok) {
        value = parseLatestRelease(await res.json())
      } else {
        throw new Error(`GitHub releases answered ${res.status}`)
      }
      lastGood = value
    } catch {
      value = lastGood ?? NONE
      ttl = FAILURE_CACHE_MS
    }
    cached = { value, until: now() + ttl }
    return value
  }
}

/** 21.7 MB */
export function formatBytes(bytes: number) {
  return `${(bytes / 1_000_000).toFixed(1)} MB`
}
