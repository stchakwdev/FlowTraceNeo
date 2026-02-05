import { defineEventHandler, setResponseHeader } from 'h3'

// OAuth Protected Resource Metadata endpoint
// MCP clients use this to discover the authorization server
// See: https://datatracker.ietf.org/doc/html/draft-ietf-oauth-resource-metadata-13

export default defineEventHandler((event) => {
  const mcpServerUrl = process.env.MCP_SERVER_URL || 'http://localhost:3000'
  const authkitDomain = process.env.WORKOS_AUTHKIT_DOMAIN

  if (!authkitDomain) {
    setResponseHeader(event, 'Content-Type', 'application/json')
    return {
      error: 'WORKOS_AUTHKIT_DOMAIN not configured',
    }
  }

  setResponseHeader(event, 'Content-Type', 'application/json')
  setResponseHeader(event, 'Access-Control-Allow-Origin', '*')
  setResponseHeader(event, 'Access-Control-Allow-Methods', 'GET, OPTIONS')
  setResponseHeader(
    event,
    'Access-Control-Allow-Headers',
    'Content-Type, Authorization'
  )

  return {
    resource: mcpServerUrl,
    authorization_servers: [`https://${authkitDomain}`],
    bearer_methods_supported: ['header'],
  }
})
