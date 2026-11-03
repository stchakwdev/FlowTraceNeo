import { defineEventHandler, setResponseHeader } from 'h3'
import { sitemapXml } from '../../src/lib/site'

// Prerendering is off (see docs/marketing-site.md), so the sitemap is served
// from here. Paths and host come from SITEMAP_PATHS and SITE_URL.
export default defineEventHandler((event) => {
  setResponseHeader(event, 'Content-Type', 'application/xml; charset=utf-8')
  setResponseHeader(event, 'Cache-Control', 'public, max-age=3600')
  return sitemapXml()
})
