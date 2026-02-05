import { defineEventHandler, setResponseHeader } from 'h3'

// OAuth Authorization Server Metadata proxy
// Some MCP clients don't support Protected Resource Metadata and
// try to fetch this directly from the MCP server instead of AuthKit
// This proxies the request to AuthKit for compatibility

export default defineEventHandler(async (event) => {
  const authkitDomain = process.env.WORKOS_AUTHKIT_DOMAIN

  if (!authkitDomain) {
    setResponseHeader(event, 'Content-Type', 'application/json')
    return {
      error: 'WORKOS_AUTHKIT_DOMAIN not configured',
    }
  }

  try {
    const response = await fetch(
      `https://${authkitDomain}/.well-known/oauth-authorization-server`
    )

    if (!response.ok) {
      throw new Error(`AuthKit returned ${response.status}`)
    }

    const metadata = await response.json()

    setResponseHeader(event, 'Content-Type', 'application/json')
    setResponseHeader(event, 'Access-Control-Allow-Origin', '*')
    setResponseHeader(event, 'Access-Control-Allow-Methods', 'GET, OPTIONS')
    setResponseHeader(
      event,
      'Access-Control-Allow-Headers',
      'Content-Type, Authorization'
    )

    return metadata
  } catch (error) {
    console.error('Failed to fetch AuthKit metadata:', error)
    setResponseHeader(event, 'Content-Type', 'application/json')
    return {
      error: 'Failed to fetch authorization server metadata',
    }
  }
})
