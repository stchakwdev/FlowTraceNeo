import { defineEventHandler, setResponseHeader } from 'h3'
import { robotsTxt } from '../../src/lib/site'

// Served rather than static so its Sitemap line follows SITE_URL.
export default defineEventHandler((event) => {
  setResponseHeader(event, 'Content-Type', 'text/plain; charset=utf-8')
  setResponseHeader(event, 'Cache-Control', 'public, max-age=3600')
  return robotsTxt()
})
