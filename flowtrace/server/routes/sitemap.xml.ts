import { defineEventHandler, setResponseHeader } from 'h3'
import { DOC_PATHS } from '../../src/lib/docs/content'
import { SITEMAP_PATHS, sitemapXml } from '../../src/lib/site'

// Prerendering is off (see docs/marketing-site.md), so the sitemap is served
// from here. Paths and host come from SITEMAP_PATHS, DOC_PATHS and SITE_URL.
export default defineEventHandler((event) => {
  setResponseHeader(event, 'Content-Type', 'application/xml; charset=utf-8')
  setResponseHeader(event, 'Cache-Control', 'public, max-age=3600')
  return sitemapXml([...SITEMAP_PATHS, ...DOC_PATHS])
})
