import { defineEventHandler, setResponseHeader } from 'h3'

// OAuth Authorization Server Metadata proxy
// Proxies to AuthKit for compatibility with older MCP clients

export default defineEventHandler(async (event) => {
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

  try {
    const response = await fetch(
      `https://${authkitDomain}/.well-known/oauth-authorization-server`
    )

    if (!response.ok) {
      throw new Error(`AuthKit returned ${response.status}`)
    }

    return await response.json()
  } catch (error) {
    console.error('Failed to fetch AuthKit metadata:', error)
    return { error: 'Failed to fetch authorization server metadata' }
  }
})
