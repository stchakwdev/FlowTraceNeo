/**
 * The one place the public host and the site's head tags come from. Every
 * canonical, Open Graph URL, sitemap entry and JSON-LD URL derives from
 * SITE_URL; nothing else hardcodes the host.
 */

const DEFAULT_SITE_URL = 'https://flow-trace-neo.vercel.app'

/** Strips trailing slashes so `SITE_URL + path` never doubles them. */
export function resolveSiteUrl(raw: string | undefined): string {
  const value = raw?.trim()
  return (value || DEFAULT_SITE_URL).replace(/\/+$/, '')
}

export const SITE_URL = resolveSiteUrl(import.meta.env.VITE_SITE_URL)
export const SITE_NAME = 'FlowTrace'
export const DEFAULT_DESCRIPTION =
  'FlowTrace records every grant, transfer and payment with its proof attached, and shows each funder the money at the depth you choose.'
export const GITHUB_URL = 'https://github.com/stchakwdev/FlowTraceNeo'

export const DEFAULT_OG_IMAGE = {
  path: '/og/flowtrace.png',
  alt: 'FlowTrace: Every payment, traced to its proof.',
  width: 1200,
  height: 630,
}

/**
 * Marketing paths that exist as routes and belong in sitemap.xml. A thread
 * that adds a marketing route adds its path here; `site.test.ts` fails if a
 * path has no route in routeTree.gen.ts.
 */
export const SITEMAP_PATHS: ReadonlyArray<string> = ['/', '/books']

export function absoluteUrl(path: string): string {
  if (/^https?:\/\//.test(path)) return path
  return SITE_URL + (path.startsWith('/') ? path : `/${path}`)
}

export function pageTitle(title: string): string {
  return `${title} · ${SITE_NAME}`
}

type MetaTag =
  | { title: string }
  | { name: string; content: string }
  | { property: string; content: string }

export interface PageHeadOptions {
  /** The page name; rendered as "<title> · FlowTrace" unless `absoluteTitle`. */
  title: string
  description: string
  /** Path from the site root, for the canonical and og:url. */
  path: string
  image?: { path: string; alt: string; width?: number; height?: number }
  absoluteTitle?: boolean
}

/** The `head()` shape for a public route: title, description, canonical, Open Graph and Twitter. */
export function pageHead({
  title,
  description,
  path,
  image = DEFAULT_OG_IMAGE,
  absoluteTitle = false,
}: PageHeadOptions): {
  meta: Array<MetaTag>
  links: Array<{ rel: string; href: string }>
} {
  const fullTitle = absoluteTitle ? title : pageTitle(title)
  const url = absoluteUrl(path)
  const imageUrl = absoluteUrl(image.path)
  const meta: Array<MetaTag> = [
    { title: fullTitle },
    { name: 'description', content: description },
    { property: 'og:type', content: 'website' },
    { property: 'og:site_name', content: SITE_NAME },
    { property: 'og:locale', content: 'en' },
    { property: 'og:title', content: fullTitle },
    { property: 'og:description', content: description },
    { property: 'og:url', content: url },
    { property: 'og:image', content: imageUrl },
    { property: 'og:image:alt', content: image.alt },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:title', content: fullTitle },
    { name: 'twitter:description', content: description },
    { name: 'twitter:image', content: imageUrl },
    { name: 'twitter:image:alt', content: image.alt },
  ]
  if (image.width && image.height) {
    meta.push(
      { property: 'og:image:width', content: String(image.width) },
      { property: 'og:image:height', content: String(image.height) },
    )
  }
  return { meta, links: [{ rel: 'canonical', href: url }] }
}

/**
 * Site-wide defaults for the root route. No canonical or og:url here: links
 * are not deduplicated across routes, so each public route sets its own.
 */
export function defaultHeadMeta(): Array<MetaTag> {
  return pageHead({
    title: SITE_NAME,
    description: DEFAULT_DESCRIPTION,
    path: '/',
    absoluteTitle: true,
  }).meta.filter((m) => !('property' in m && m.property === 'og:url'))
}

/** Organization JSON-LD, for `/` only. */
export function organizationJsonLd(): string {
  return JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Organization',
    name: SITE_NAME,
    url: SITE_URL,
    logo: absoluteUrl('/logo512.png'),
    sameAs: [GITHUB_URL],
  })
}

export function sitemapXml(
  paths: ReadonlyArray<string> = SITEMAP_PATHS,
): string {
  const urls = paths
    .map((p) => `  <url><loc>${absoluteUrl(p)}</loc></url>`)
    .join('\n')
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`
}

export function robotsTxt(): string {
  return `User-agent: *\nDisallow:\n\nSitemap: ${absoluteUrl('/sitemap.xml')}\n`
}
