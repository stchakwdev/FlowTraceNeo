import { readFileSync } from 'node:fs'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  SITEMAP_PATHS,
  SITE_URL,
  defaultHeadMeta,
  organizationJsonLd,
  pageHead,
  resolveSiteUrl,
  robotsTxt,
  sitemapXml,
} from './site'

describe('SITE_URL', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('defaults to the Vercel host', () => {
    expect(resolveSiteUrl(undefined)).toBe('https://flow-trace-neo.vercel.app')
    expect(resolveSiteUrl('  ')).toBe('https://flow-trace-neo.vercel.app')
    expect(SITE_URL).toBe('https://flow-trace-neo.vercel.app')
  })

  it('takes VITE_SITE_URL without its trailing slash', async () => {
    expect(resolveSiteUrl('https://flowtrace.example/')).toBe(
      'https://flowtrace.example',
    )
    vi.stubEnv('VITE_SITE_URL', 'https://flowtrace.example/')
    vi.resetModules()
    const site = await import('./site')
    expect(site.SITE_URL).toBe('https://flowtrace.example')
    expect(site.absoluteUrl('/pricing')).toBe(
      'https://flowtrace.example/pricing',
    )
  })
})

describe('pageHead', () => {
  const head = pageHead({
    title: 'Pricing',
    description: 'Free during the pilot.',
    path: '/pricing',
  })
  const content = (key: string) =>
    head.meta.find(
      (m) =>
        ('name' in m && m.name === key) ||
        ('property' in m && m.property === key),
    )

  it('titles the page "<page> · FlowTrace"', () => {
    expect(head.meta[0]).toEqual({ title: 'Pricing · FlowTrace' })
  })

  it('sets the description, canonical and Open Graph from SITE_URL', () => {
    expect(content('description')).toMatchObject({
      content: 'Free during the pilot.',
    })
    expect(head.links).toEqual([
      { rel: 'canonical', href: `${SITE_URL}/pricing` },
    ])
    expect(content('og:url')).toMatchObject({ content: `${SITE_URL}/pricing` })
    expect(content('og:image')).toMatchObject({
      content: `${SITE_URL}/og/flowtrace.png`,
    })
    expect(content('og:image:alt')).toBeDefined()
    expect(content('og:site_name')).toMatchObject({ content: 'FlowTrace' })
    expect(content('og:locale')).toMatchObject({ content: 'en' })
    expect(content('og:type')).toMatchObject({ content: 'website' })
    expect(content('twitter:card')).toMatchObject({
      content: 'summary_large_image',
    })
  })

  it('keeps an absolute title as given', () => {
    const home = pageHead({
      title: 'FlowTrace · Every payment',
      description: 'x',
      path: '/',
      absoluteTitle: true,
    })
    expect(home.meta[0]).toEqual({ title: 'FlowTrace · Every payment' })
  })

  it('root defaults carry no og:url', () => {
    const meta = defaultHeadMeta()
    expect(meta.some((m) => 'property' in m && m.property === 'og:url')).toBe(
      false,
    )
    expect(meta.some((m) => 'name' in m && m.name === 'description')).toBe(true)
  })
})

describe('structured data, sitemap and robots', () => {
  it('describes the organisation with its GitHub repository', () => {
    const ld = JSON.parse(organizationJsonLd())
    expect(ld).toMatchObject({
      '@type': 'Organization',
      name: 'FlowTrace',
      url: SITE_URL,
      sameAs: ['https://github.com/stchakwdev/FlowTraceNeo'],
    })
  })

  it('lists every sitemap path under SITE_URL', () => {
    const xml = sitemapXml()
    for (const path of SITEMAP_PATHS) {
      expect(xml).toContain(`<loc>${SITE_URL}${path}</loc>`)
    }
    expect(robotsTxt()).toContain(`Sitemap: ${SITE_URL}/sitemap.xml`)
  })

  it('lists only paths that are real routes', () => {
    const tree = readFileSync('src/routeTree.gen.ts', 'utf8')
    const block = tree.slice(
      tree.indexOf('export interface FileRoutesByFullPath'),
    )
    const routes = block.slice(0, block.indexOf('}'))
    for (const path of SITEMAP_PATHS) {
      expect(routes).toContain(`'${path}':`)
    }
  })
})
