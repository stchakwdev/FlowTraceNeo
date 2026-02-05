import { defineEventHandler, setResponseHeader } from 'h3'

// OAuth Protected Resource Metadata endpoint
// MCP clients use this to discover the authorization server

export default defineEventHandler((event) => {
  const mcpServerUrl = process.env.MCP_SERVER_URL || 'http://localhost:3000'
  const authkitDomain = process.env.WORKOS_AUTHKIT_DOMAIN

  setResponseHeader(event, 'Content-Type', 'application/json')
  setResponseHeader(event, 'Access-Control-Allow-Origin', '*')
  setResponseHeader(event, 'Access-Control-Allow-Methods', 'GET, OPTIONS')
  setResponseHeader(
    event,
    'Access-Control-Allow-Headers',
    'Content-Type, Authorization'
  )

  if (!authkitDomain) {
    return { error: 'WORKOS_AUTHKIT_DOMAIN not configured' }
  }

  return {
    resource: mcpServerUrl,
    authorization_servers: [`https://${authkitDomain}`],
    bearer_methods_supported: ['header'],
  }
})
