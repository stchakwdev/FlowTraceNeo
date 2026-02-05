import { jwtVerify, createRemoteJWKSet } from 'jose'
import type { H3Event } from 'h3'
import { getHeader } from 'h3'

// MCP Authentication utilities for Nitro server routes
// Verifies Bearer tokens from MCP clients against AuthKit

// Cache the JWKS
let jwksCache: ReturnType<typeof createRemoteJWKSet> | null = null

function getJWKS(authkitDomain: string) {
  if (!jwksCache) {
    jwksCache = createRemoteJWKSet(
      new URL(`https://${authkitDomain}/oauth2/jwks`)
    )
  }
  return jwksCache
}

// User context from MCP token
export interface MCPUser {
  id: string
  email?: string
  orgId?: string
}

// Helper to generate WWW-Authenticate header
function getWWWAuthenticateHeader(): string {
  const mcpServerUrl = process.env.MCP_SERVER_URL || 'http://localhost:3000'
  return [
    'Bearer error="unauthorized"',
    'error_description="Authorization needed"',
    `resource_metadata="${mcpServerUrl}/.well-known/oauth-protected-resource"`,
  ].join(', ')
}

// Create 401 response with proper MCP headers
function create401Response(message: string): Response {
  return new Response(JSON.stringify({ error: message }), {
    status: 401,
    headers: {
      'Content-Type': 'application/json',
      'WWW-Authenticate': getWWWAuthenticateHeader(),
    },
  })
}

// Auth result type
type AuthResult =
  | { success: true; user: MCPUser }
  | { success: false; response: Response }

// Verify MCP authentication and return user or error response
export async function verifyMCPAuth(event: H3Event): Promise<AuthResult> {
  const authkitDomain = process.env.WORKOS_AUTHKIT_DOMAIN

  if (!authkitDomain) {
    return {
      success: false,
      response: new Response(
        JSON.stringify({ error: 'MCP authentication not configured' }),
        { status: 500, headers: { 'Content-Type': 'application/json' } }
      ),
    }
  }

  // Extract Bearer token
  const authHeader = getHeader(event, 'authorization')
  const token = authHeader?.match(/^Bearer (.+)$/)?.[1]

  if (!token) {
    return {
      success: false,
      response: create401Response('No token provided'),
    }
  }

  try {
    const JWKS = getJWKS(authkitDomain)

    const { payload } = await jwtVerify(token, JWKS, {
      issuer: `https://${authkitDomain}`,
    })

    return {
      success: true,
      user: {
        id: payload.sub as string,
        email: payload.email as string | undefined,
        orgId: payload.org_id as string | undefined,
      },
    }
  } catch (error) {
    console.error('MCP token verification failed:', error)
    return {
      success: false,
      response: create401Response('Invalid bearer token'),
    }
  }
}
