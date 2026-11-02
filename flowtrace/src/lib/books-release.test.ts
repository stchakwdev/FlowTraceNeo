import { describe, expect, it, vi } from 'vitest'
import {
  RELEASE_CACHE_MS,
  createReleaseLoader,
  formatBytes,
  parseLatestRelease,
} from './books-release'

// The shape of GET /repos/{owner}/{repo}/releases/latest, cut to the fields the page reads
// (https://docs.github.com/en/rest/releases/releases#get-the-latest-release).
const latest = {
  tag_name: 'v0.1.0',
  html_url: 'https://github.com/stchakwdev/FlowTraceNeo/releases/tag/v0.1.0',
  published_at: '2026-11-03T09:30:00Z',
  draft: false,
  prerelease: false,
  assets: [
    {
      name: 'flowtrace-books-0.1.0-macos-arm64.tar.gz',
      size: 20_123_456,
      browser_download_url:
        'https://github.com/stchakwdev/FlowTraceNeo/releases/download/v0.1.0/flowtrace-books-0.1.0-macos-arm64.tar.gz',
    },
    {
      name: 'flowtrace-books-0.1.0-linux-x86_64.tar.gz',
      size: 21_654_321,
      browser_download_url:
        'https://github.com/stchakwdev/FlowTraceNeo/releases/download/v0.1.0/flowtrace-books-0.1.0-linux-x86_64.tar.gz',
    },
    {
      name: 'SHA256SUMS',
      size: 190,
      browser_download_url:
        'https://github.com/stchakwdev/FlowTraceNeo/releases/download/v0.1.0/SHA256SUMS',
    },
  ],
}

function response(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

describe('parseLatestRelease', () => {
  it('lists the Linux and macOS archives with version, size and checksums', () => {
    const release = parseLatestRelease(latest)
    expect(release).toEqual({
      kind: 'available',
      tag: 'v0.1.0',
      version: '0.1.0',
      publishedAt: '2026-11-03T09:30:00Z',
      url: latest.html_url,
      checksumsUrl: latest.assets[2].browser_download_url,
      assets: [
        {
          name: 'flowtrace-books-0.1.0-linux-x86_64.tar.gz',
          platform: 'Linux',
          arch: 'x86_64',
          size: 21_654_321,
          url: latest.assets[1].browser_download_url,
        },
        {
          name: 'flowtrace-books-0.1.0-macos-arm64.tar.gz',
          platform: 'macOS',
          arch: 'arm64',
          size: 20_123_456,
          url: latest.assets[0].browser_download_url,
        },
      ],
    })
  })

  it('treats a release without Books archives as no release', () => {
    expect(
      parseLatestRelease({ ...latest, assets: [latest.assets[2]] }),
    ).toEqual({
      kind: 'none',
    })
  })

  it('treats a draft, a pre-release or a malformed body as no release', () => {
    expect(parseLatestRelease({ ...latest, draft: true })).toEqual({
      kind: 'none',
    })
    expect(parseLatestRelease({ ...latest, prerelease: true })).toEqual({
      kind: 'none',
    })
    expect(parseLatestRelease(null)).toEqual({ kind: 'none' })
    expect(parseLatestRelease({ tag_name: 3, assets: 'x' })).toEqual({
      kind: 'none',
    })
  })

  it('skips assets with a missing size or link', () => {
    const release = parseLatestRelease({
      ...latest,
      assets: [{ ...latest.assets[0], size: undefined }, latest.assets[1]],
    })
    expect(
      release.kind === 'available' && release.assets.map((a) => a.platform),
    ).toEqual(['Linux'])
  })
})

describe('createReleaseLoader', () => {
  it('reads the latest release once and serves it from cache for ten minutes', async () => {
    const fetchImpl = vi.fn(() => Promise.resolve(response(200, latest)))
    let now = 1_000_000
    const load = createReleaseLoader({ fetchImpl, now: () => now })

    expect((await load()).kind).toBe('available')
    now += RELEASE_CACHE_MS - 1
    await load()
    expect(fetchImpl).toHaveBeenCalledTimes(1)
    expect(fetchImpl.mock.calls[0]).toEqual([
      'https://api.github.com/repos/stchakwdev/FlowTraceNeo/releases/latest',
      expect.objectContaining({
        headers: expect.objectContaining({
          Accept: 'application/vnd.github+json',
        }),
      }),
    ])
    now += 2
    await load()
    expect(fetchImpl).toHaveBeenCalledTimes(2)
  })

  it('returns no release on 404 (no release published yet)', async () => {
    const load = createReleaseLoader({
      fetchImpl: () => Promise.resolve(response(404, { message: 'Not Found' })),
    })
    expect(await load()).toEqual({ kind: 'none' })
  })

  it('returns no release when the network fails, and does not throw', async () => {
    const load = createReleaseLoader({
      fetchImpl: () => Promise.reject(new TypeError('fetch failed')),
    })
    expect(await load()).toEqual({ kind: 'none' })
  })

  it('returns no release on a rate limit or server error', async () => {
    const load = createReleaseLoader({
      fetchImpl: () =>
        Promise.resolve(response(403, { message: 'rate limit' })),
    })
    expect(await load()).toEqual({ kind: 'none' })
  })

  it('keeps serving the last good release when a later read fails', async () => {
    let fail = false
    let now = 0
    const load = createReleaseLoader({
      fetchImpl: () =>
        fail
          ? Promise.reject(new Error('down'))
          : Promise.resolve(response(200, latest)),
      now: () => now,
    })
    expect((await load()).kind).toBe('available')
    fail = true
    now += RELEASE_CACHE_MS + 1
    expect((await load()).kind).toBe('available')
  })
})

describe('formatBytes', () => {
  it('writes sizes in MB with one decimal', () => {
    expect(formatBytes(21_654_321)).toBe('21.7 MB')
    expect(formatBytes(512_000)).toBe('0.5 MB')
  })
})
