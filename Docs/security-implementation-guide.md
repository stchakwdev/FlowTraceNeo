# FlowTrace Security Implementation Guide

## Security Architecture Overview

### Defense in Depth Strategy

```
┌─────────────────────────────────────────────────────────┐
│                   External Layer                         │
│  - WAF (Web Application Firewall)                       │
│  - DDoS Protection                                      │
│  - Rate Limiting                                        │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                  Application Layer                       │
│  - Authentication (JWT + Refresh Tokens)                │
│  - Authorization (RBAC)                                 │
│  - Input Validation                                     │
│  - CSRF Protection                                      │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                    Data Layer                           │
│  - Encryption at Rest (AES-256)                        │
│  - Encryption in Transit (TLS 1.3)                     │
│  - Database RLS Policies                               │
│  - Sensitive Data Masking                              │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                 Infrastructure Layer                     │
│  - Network Isolation                                    │
│  - Secrets Management                                   │
│  - Audit Logging                                        │
│  - Backup Encryption                                    │
└─────────────────────────────────────────────────────────┘
```

## Authentication Implementation

FlowTrace uses **WorkOS AuthKit** for authentication, providing enterprise-grade SSO and MFA without custom implementation.

### Why WorkOS AuthKit

- **Enterprise SSO**: SAML, OIDC, Google, Microsoft out of the box
- **Nonprofit-friendly**: Better pricing for mission-driven organizations
- **MFA built-in**: TOTP, SMS, WebAuthn supported automatically
- **Session management**: Secure cookies, automatic refresh
- **Organization management**: Multi-tenant support native

### Setup

```bash
npm install @workos/authkit-tanstack-react-start
```

### Environment Variables

```env
WORKOS_CLIENT_ID=client_xxx
WORKOS_API_KEY=sk_xxx
WORKOS_COOKIE_PASSWORD=32+_character_secret_for_session_encryption
WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback
```

### Integration with TanStack Start

```typescript
// src/routes/__root.tsx
import { AuthKitProvider } from '@workos/authkit-tanstack-react-start/client'
import { getAuth } from '@workos/authkit-tanstack-react-start'

export const Route = createRootRoute({
  beforeLoad: async () => {
    const auth = await getAuth()
    return { auth }
  },
  component: RootLayout,
})

function RootLayout() {
  const { auth } = Route.useRouteContext()
  return (
    <AuthKitProvider initialAuth={auth}>
      {/* App */}
    </AuthKitProvider>
  )
}
```

### OAuth Callback Handler

```typescript
// src/routes/api/auth/callback.tsx
import { createFileRoute } from '@tanstack/react-router'
import { handleCallbackRoute } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/api/auth/callback')({
  loader: handleCallbackRoute(),
})
```

### Protected Routes

```typescript
// src/routes/_authenticated.tsx
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/_authenticated')({
  beforeLoad: async ({ location }) => {
    const { user, organizationId } = await getAuth()

    if (!user) {
      const signInUrl = await getSignInUrl({
        data: { returnPathname: location.pathname },
      })
      throw redirect({ href: signInUrl })
    }

    return { user, organizationId }
  },
  component: () => <Outlet />,
})
```

### Getting User in Components

```typescript
import { useAuth } from '@workos/authkit-tanstack-react-start/client'

function UserProfile() {
  const { user, isLoading, signOut } = useAuth()

  if (isLoading) return <Spinner />
  if (!user) return null

  return (
    <div>
      <p>{user.email}</p>
      <p>{user.firstName} {user.lastName}</p>
      <button onClick={signOut}>Sign Out</button>
    </div>
  )
}
```

### Organization Context

WorkOS provides organization context for multi-tenant access:

```typescript
const { organizationId } = await getAuth()

// Use organizationId for RLS policies
const { data } = await supabase
  .from('transactions')
  .select('*')
  .eq('organization_id', organizationId)
```

### Session Security

WorkOS AuthKit handles:
- Secure HTTP-only cookies
- Automatic session refresh
- CSRF protection
- Session invalidation on logout

### SSO Configuration (Admin)

Configure SSO connections in WorkOS Dashboard:
1. Create Organization in WorkOS
2. Configure SAML/OIDC connection
3. Link FlowTrace organization to WorkOS organization ID
4. Users authenticate via their IdP

---

## Legacy Reference: Custom Session Management

> **Note**: The following is preserved for reference. WorkOS handles sessions automatically.

```typescript
// For custom session validation if needed
// lib/auth/session.ts (DEPRECATED - use WorkOS)
      7 * 24 * 60 * 60, // 7 days
      JSON.stringify(sessionData)
    )

    // Track user sessions
    await this.redis.sadd(`user:${userId}:sessions`, sessionId)

    return sessionId
  }

  async getSession(sessionId: string) {
    const data = await this.redis.get(`session:${sessionId}`)
    if (!data) return null

    const session = JSON.parse(data)
    
    // Update last activity
    session.lastActivity = new Date().toISOString()
    await this.redis.setex(
      `session:${sessionId}`,
      7 * 24 * 60 * 60,
      JSON.stringify(session)
    )

    return session
  }

  async invalidateSession(sessionId: string) {
    const session = await this.getSession(sessionId)
    if (!session) return

    // Remove from Redis
    await this.redis.del(`session:${sessionId}`)
    await this.redis.srem(`user:${session.userId}:sessions`, sessionId)

    // Audit log
    await this.logSessionEnd(sessionId, session.userId)
  }

  async invalidateAllUserSessions(userId: string) {
    const sessions = await this.redis.smembers(`user:${userId}:sessions`)
    
    for (const sessionId of sessions) {
      await this.redis.del(`session:${sessionId}`)
    }
    
    await this.redis.del(`user:${userId}:sessions`)
  }

  private async logSessionEnd(sessionId: string, userId: string) {
    // Implementation for audit logging
  }
}
```

## Authorization & Access Control

### Role-Based Access Control (RBAC)

```typescript
// lib/auth/rbac.ts
export enum Role {
  OWNER = 'owner',
  ADMIN = 'admin',
  MANAGER = 'manager',
  MEMBER = 'member',
  VIEWER = 'viewer'
}

export enum Permission {
  // Organization
  ORG_VIEW = 'org:view',
  ORG_EDIT = 'org:edit',
  ORG_DELETE = 'org:delete',
  
  // Transactions
  TRANSACTION_VIEW = 'transaction:view',
  TRANSACTION_CREATE = 'transaction:create',
  TRANSACTION_EDIT = 'transaction:edit',
  TRANSACTION_DELETE = 'transaction:delete',
  TRANSACTION_VERIFY = 'transaction:verify',
  
  // Users
  USER_VIEW = 'user:view',
  USER_INVITE = 'user:invite',
  USER_EDIT = 'user:edit',
  USER_DELETE = 'user:delete',
  
  // Reports
  REPORT_VIEW = 'report:view',
  REPORT_CREATE = 'report:create',
  REPORT_EXPORT = 'report:export',
  
  // Settings
  SETTINGS_VIEW = 'settings:view',
  SETTINGS_EDIT = 'settings:edit'
}

export class RBACManager {
  private readonly rolePermissions: Record<Role, Permission[]> = {
    [Role.OWNER]: [
      // All permissions
      ...Object.values(Permission)
    ],
    [Role.ADMIN]: [
      // All except org delete
      ...Object.values(Permission).filter(p => p !== Permission.ORG_DELETE)
    ],
    [Role.MANAGER]: [
      Permission.ORG_VIEW,
      Permission.TRANSACTION_VIEW,
      Permission.TRANSACTION_CREATE,
      Permission.TRANSACTION_EDIT,
      Permission.TRANSACTION_VERIFY,
      Permission.USER_VIEW,
      Permission.REPORT_VIEW,
      Permission.REPORT_CREATE,
      Permission.SETTINGS_VIEW
    ],
    [Role.MEMBER]: [
      Permission.ORG_VIEW,
      Permission.TRANSACTION_VIEW,
      Permission.TRANSACTION_CREATE,
      Permission.USER_VIEW,
      Permission.REPORT_VIEW
    ],
    [Role.VIEWER]: [
      Permission.ORG_VIEW,
      Permission.TRANSACTION_VIEW,
      Permission.USER_VIEW,
      Permission.REPORT_VIEW
    ]
  }

  hasPermission(role: Role, permission: Permission): boolean {
    return this.rolePermissions[role]?.includes(permission) ?? false
  }

  hasAnyPermission(role: Role, permissions: Permission[]): boolean {
    return permissions.some(p => this.hasPermission(role, p))
  }

  hasAllPermissions(role: Role, permissions: Permission[]): boolean {
    return permissions.every(p => this.hasPermission(role, p))
  }
}
```

### Resource-Based Access Control

```typescript
// lib/auth/resource-access.ts
export class ResourceAccessControl {
  async canAccessTransaction(
    userId: string,
    transactionId: string,
    action: 'view' | 'edit' | 'delete'
  ): Promise<boolean> {
    // Get user's organization
    const user = await getUserWithOrganization(userId)
    
    // Get transaction
    const transaction = await getTransaction(transactionId)
    
    // Check organization match
    if (transaction.organizationId !== user.organizationId) {
      return false
    }
    
    // Check role permissions
    const rbac = new RBACManager()
    const requiredPermission = {
      view: Permission.TRANSACTION_VIEW,
      edit: Permission.TRANSACTION_EDIT,
      delete: Permission.TRANSACTION_DELETE
    }[action]
    
    return rbac.hasPermission(user.role, requiredPermission)
  }

  async canAccessReport(
    userId: string,
    reportId: string,
    action: 'view' | 'export'
  ): Promise<boolean> {
    // Similar implementation
    return true
  }
}
```

## Input Validation & Sanitization

### Request Validation Middleware

```typescript
// middleware/validation.ts
import { z } from 'zod'
import DOMPurify from 'isomorphic-dompurify'

export function validateRequest(schema: z.ZodSchema) {
  return async (req: Request, res: Response, next: NextFunction) => {
    try {
      // Validate request body
      const validated = schema.parse(req.body)
      
      // Sanitize string inputs
      req.body = sanitizeObject(validated)
      
      next()
    } catch (error) {
      if (error instanceof z.ZodError) {
        return res.status(400).json({
          success: false,
          error: {
            code: 'VALIDATION_ERROR',
            message: 'Invalid request data',
            details: error.errors
          }
        })
      }
      next(error)
    }
  }
}

function sanitizeObject(obj: any): any {
  if (typeof obj === 'string') {
    // Sanitize HTML
    return DOMPurify.sanitize(obj, { 
      ALLOWED_TAGS: [],
      ALLOWED_ATTR: []
    })
  }
  
  if (Array.isArray(obj)) {
    return obj.map(sanitizeObject)
  }
  
  if (obj && typeof obj === 'object') {
    const sanitized: any = {}
    for (const [key, value] of Object.entries(obj)) {
      sanitized[key] = sanitizeObject(value)
    }
    return sanitized
  }
  
  return obj
}
```

### SQL Injection Prevention

```typescript
// lib/db/query-builder.ts
import { sql } from '@vercel/postgres'

export class SafeQueryBuilder {
  // Never use string concatenation for queries
  async getTransactionsByOrg(orgId: string, limit: number = 20) {
    // ✅ Safe: Uses parameterized queries
    return sql`
      SELECT * FROM transactions 
      WHERE organization_id = ${orgId}
      ORDER BY created_at DESC
      LIMIT ${limit}
    `
  }

  // For dynamic queries, use safe builders
  async searchTransactions(filters: TransactionFilters) {
    const conditions = []
    const values = []
    
    if (filters.status) {
      conditions.push(`verification_status = $${values.length + 1}`)
      values.push(filters.status)
    }
    
    if (filters.minAmount) {
      conditions.push(`amount >= $${values.length + 1}`)
      values.push(filters.minAmount)
    }
    
    const query = `
      SELECT * FROM transactions
      WHERE ${conditions.join(' AND ')}
      ORDER BY created_at DESC
    `
    
    return db.query(query, values)
  }
}
```

### XSS Prevention

```typescript
// components/SafeHTML.tsx
import DOMPurify from 'isomorphic-dompurify'

interface SafeHTMLProps {
  html: string
  allowedTags?: string[]
  allowedAttributes?: string[]
}

export function SafeHTML({ 
  html, 
  allowedTags = ['p', 'br', 'strong', 'em'],
  allowedAttributes = []
}: SafeHTMLProps) {
  const clean = DOMPurify.sanitize(html, {
    ALLOWED_TAGS: allowedTags,
    ALLOWED_ATTR: allowedAttributes,
    ALLOW_DATA_ATTR: false,
    ALLOW_UNKNOWN_PROTOCOLS: false
  })
  
  return <div dangerouslySetInnerHTML={{ __html: clean }} />
}

// Usage
<SafeHTML html={userGeneratedContent} />
```

## CSRF Protection

### CSRF Token Implementation

```typescript
// lib/security/csrf.ts
import { randomBytes } from 'crypto'
import { Redis } from 'ioredis'

export class CSRFProtection {
  constructor(private redis: Redis) {}

  async generateToken(sessionId: string): Promise<string> {
    const token = randomBytes(32).toString('hex')
    
    // Store with session
    await this.redis.setex(
      `csrf:${sessionId}`,
      60 * 60, // 1 hour
      token
    )
    
    return token
  }

  async verifyToken(sessionId: string, token: string): Promise<boolean> {
    const storedToken = await this.redis.get(`csrf:${sessionId}`)
    
    if (!storedToken || storedToken !== token) {
      return false
    }
    
    // Rotate token after use
    await this.redis.del(`csrf:${sessionId}`)
    
    return true
  }
}

// Middleware
export function csrfProtection(csrf: CSRFProtection) {
  return async (req: Request, res: Response, next: NextFunction) => {
    // Skip for safe methods
    if (['GET', 'HEAD', 'OPTIONS'].includes(req.method)) {
      return next()
    }
    
    const token = req.headers['x-csrf-token'] as string
    const sessionId = req.session?.id
    
    if (!token || !sessionId) {
      return res.status(403).json({
        error: 'CSRF token missing'
      })
    }
    
    const valid = await csrf.verifyToken(sessionId, token)
    
    if (!valid) {
      return res.status(403).json({
        error: 'Invalid CSRF token'
      })
    }
    
    next()
  }
}
```

## Data Encryption

### Field-Level Encryption

```typescript
// lib/security/encryption.ts
import crypto from 'crypto'

export class FieldEncryption {
  private algorithm = 'aes-256-gcm'
  private keyDerivationSalt: Buffer
  
  constructor(private masterKey: string) {
    this.keyDerivationSalt = Buffer.from(process.env.ENCRYPTION_SALT!, 'hex')
  }

  private deriveKey(purpose: string): Buffer {
    return crypto.pbkdf2Sync(
      this.masterKey,
      Buffer.concat([this.keyDerivationSalt, Buffer.from(purpose)]),
      100000,
      32,
      'sha256'
    )
  }

  encrypt(plaintext: string, purpose: string = 'general'): string {
    const key = this.deriveKey(purpose)
    const iv = crypto.randomBytes(16)
    const cipher = crypto.createCipheriv(this.algorithm, key, iv)
    
    let encrypted = cipher.update(plaintext, 'utf8', 'hex')
    encrypted += cipher.final('hex')
    
    const authTag = cipher.getAuthTag()
    
    // Combine IV + authTag + encrypted
    return iv.toString('hex') + ':' + authTag.toString('hex') + ':' + encrypted
  }

  decrypt(ciphertext: string, purpose: string = 'general'): string {
    const parts = ciphertext.split(':')
    const iv = Buffer.from(parts[0], 'hex')
    const authTag = Buffer.from(parts[1], 'hex')
    const encrypted = parts[2]
    
    const key = this.deriveKey(purpose)
    const decipher = crypto.createDecipheriv(this.algorithm, key, iv)
    decipher.setAuthTag(authTag)
    
    let decrypted = decipher.update(encrypted, 'hex', 'utf8')
    decrypted += decipher.final('utf8')
    
    return decrypted
  }
}

// Usage for sensitive fields
export function encryptSensitiveFields(data: any, encryption: FieldEncryption) {
  const sensitiveFields = ['bankAccount', 'taxId', 'phoneNumber']
  
  const encrypted = { ...data }
  
  for (const field of sensitiveFields) {
    if (encrypted[field]) {
      encrypted[field] = encryption.encrypt(encrypted[field], field)
    }
  }
  
  return encrypted
}
```

### Database Encryption

```sql
-- Enable pgcrypto extension
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Encrypt sensitive columns
CREATE TABLE payment_methods (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID REFERENCES organizations(id),
  -- Encrypted fields
  account_number TEXT, -- Will be encrypted
  routing_number TEXT, -- Will be encrypted
  account_holder_name TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Encryption functions
CREATE OR REPLACE FUNCTION encrypt_sensitive(data TEXT)
RETURNS TEXT AS $$
BEGIN
  RETURN pgp_sym_encrypt(
    data, 
    current_setting('app.encryption_key')
  );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE OR REPLACE FUNCTION decrypt_sensitive(data TEXT)
RETURNS TEXT AS $$
BEGIN
  RETURN pgp_sym_decrypt(
    data::bytea,
    current_setting('app.encryption_key')
  );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Trigger to auto-encrypt on insert/update
CREATE OR REPLACE FUNCTION encrypt_payment_method()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.account_number IS NOT NULL THEN
    NEW.account_number = encrypt_sensitive(NEW.account_number);
  END IF;
  
  IF NEW.routing_number IS NOT NULL THEN
    NEW.routing_number = encrypt_sensitive(NEW.routing_number);
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER encrypt_before_insert
BEFORE INSERT OR UPDATE ON payment_methods
FOR EACH ROW EXECUTE FUNCTION encrypt_payment_method();
```

## API Security

### Rate Limiting

```typescript
// middleware/rate-limit.ts
import { RateLimiterRedis } from 'rate-limiter-flexible'
import Redis from 'ioredis'

const redis = new Redis({
  host: process.env.REDIS_HOST,
  port: parseInt(process.env.REDIS_PORT!),
  password: process.env.REDIS_PASSWORD
})

// Different limits for different endpoints
const rateLimiters = {
  // General API limit
  api: new RateLimiterRedis({
    storeClient: redis,
    keyPrefix: 'rl:api',
    points: 100, // requests
    duration: 60, // per minute
    blockDuration: 60 // block for 1 minute
  }),
  
  // Auth endpoints (stricter)
  auth: new RateLimiterRedis({
    storeClient: redis,
    keyPrefix: 'rl:auth',
    points: 5,
    duration: 60,
    blockDuration: 900 // block for 15 minutes
  }),
  
  // Transaction creation
  transaction: new RateLimiterRedis({
    storeClient: redis,
    keyPrefix: 'rl:transaction',
    points: 50,
    duration: 60,
    blockDuration: 60
  }),
  
  // File uploads
  upload: new RateLimiterRedis({
    storeClient: redis,
    keyPrefix: 'rl:upload',
    points: 10,
    duration: 300, // 5 minutes
    blockDuration: 300
  })
}

export function rateLimitMiddleware(type: keyof typeof rateLimiters) {
  return async (req: Request, res: Response, next: NextFunction) => {
    const limiter = rateLimiters[type]
    const key = req.user?.id || req.ip
    
    try {
      await limiter.consume(key)
      next()
    } catch (rejRes) {
      res.status(429).json({
        error: 'Too many requests',
        retryAfter: Math.round(rejRes.msBeforeNext / 1000) || 60
      })
    }
  }
}
```

### API Key Management

```typescript
// lib/security/api-keys.ts
import { randomBytes } from 'crypto'
import { hash, verify } from 'argon2'

export class APIKeyManager {
  async generateAPIKey(organizationId: string, name: string) {
    // Generate key
    const key = `flt_${randomBytes(32).toString('hex')}`
    
    // Hash for storage
    const hashedKey = await hash(key, {
      type: argon2.argon2id,
      memoryCost: 2 ** 16,
      timeCost: 3,
      parallelism: 1
    })
    
    // Store in database
    const apiKey = await db.apiKeys.create({
      organizationId,
      name,
      keyHash: hashedKey,
      prefix: key.substring(0, 10),
      lastUsed: null,
      expiresAt: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000) // 1 year
    })
    
    // Return key only once
    return {
      id: apiKey.id,
      key, // Only returned on creation
      name,
      createdAt: apiKey.createdAt
    }
  }

  async verifyAPIKey(key: string) {
    // Extract prefix
    const prefix = key.substring(0, 10)
    
    // Find potential matches
    const apiKeys = await db.apiKeys.findMany({
      where: { prefix }
    })
    
    // Verify against hashes
    for (const apiKey of apiKeys) {
      const valid = await verify(apiKey.keyHash, key)
      
      if (valid) {
        // Check expiration
        if (apiKey.expiresAt && apiKey.expiresAt < new Date()) {
          throw new Error('API key expired')
        }
        
        // Update last used
        await db.apiKeys.update({
          where: { id: apiKey.id },
          data: { lastUsed: new Date() }
        })
        
        return apiKey
      }
    }
    
    throw new Error('Invalid API key')
  }
}
```

## Security Headers

```typescript
// middleware/security-headers.ts
export function securityHeaders() {
  return (req: Request, res: Response, next: NextFunction) => {
    // Strict Transport Security
    res.setHeader(
      'Strict-Transport-Security',
      'max-age=31536000; includeSubDomains; preload'
    )
    
    // Prevent MIME type sniffing
    res.setHeader('X-Content-Type-Options', 'nosniff')
    
    // XSS Protection (legacy browsers)
    res.setHeader('X-XSS-Protection', '1; mode=block')
    
    // Prevent clickjacking
    res.setHeader('X-Frame-Options', 'DENY')
    
    // Referrer Policy
    res.setHeader('Referrer-Policy', 'strict-origin-when-cross-origin')
    
    // Permissions Policy
    res.setHeader(
      'Permissions-Policy',
      'camera=(), microphone=(), geolocation=(self), interest-cohort=()'
    )
    
    // Content Security Policy
    res.setHeader(
      'Content-Security-Policy',
      [
        "default-src 'self'",
        "script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.jsdelivr.net",
        "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
        "font-src 'self' https://fonts.gstatic.com",
        "img-src 'self' data: https: blob:",
        "connect-src 'self' https://*.supabase.co wss://*.supabase.co https://api.flowtrace.io",
        "frame-ancestors 'none'",
        "base-uri 'self'",
        "form-action 'self'",
        "upgrade-insecure-requests"
      ].join('; ')
    )
    
    next()
  }
}
```

## Audit Logging

### Comprehensive Audit System

```typescript
// lib/security/audit.ts
export enum AuditAction {
  // Auth
  LOGIN = 'auth.login',
  LOGOUT = 'auth.logout',
  PASSWORD_CHANGE = 'auth.password_change',
  MFA_ENABLE = 'auth.mfa_enable',
  
  // Transactions
  TRANSACTION_CREATE = 'transaction.create',
  TRANSACTION_UPDATE = 'transaction.update',
  TRANSACTION_DELETE = 'transaction.delete',
  TRANSACTION_VERIFY = 'transaction.verify',
  
  // Organization
  ORG_UPDATE = 'org.update',
  ORG_MEMBER_ADD = 'org.member_add',
  ORG_MEMBER_REMOVE = 'org.member_remove',
  
  // Data Access
  REPORT_GENERATE = 'report.generate',
  DATA_EXPORT = 'data.export',
  
  // Security
  PERMISSION_CHANGE = 'security.permission_change',
  API_KEY_CREATE = 'security.api_key_create',
  SUSPICIOUS_ACTIVITY = 'security.suspicious_activity'
}

export class AuditLogger {
  async log(params: {
    action: AuditAction
    userId: string
    organizationId: string
    resourceType?: string
    resourceId?: string
    metadata?: any
    ipAddress: string
    userAgent: string
  }) {
    const entry = {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      ...params,
      metadata: {
        ...params.metadata,
        // Add request context
        requestId: getCurrentRequestId(),
        sessionId: getCurrentSessionId()
      }
    }
    
    // Write to database
    await db.auditLogs.create({ data: entry })
    
    // Send to SIEM if configured
    if (process.env.SIEM_ENDPOINT) {
      await this.sendToSIEM(entry)
    }
    
    // Check for suspicious patterns
    await this.checkSuspiciousActivity(entry)
  }
  
  private async checkSuspiciousActivity(entry: AuditLogEntry) {
    // Multiple failed logins
    if (entry.action === AuditAction.LOGIN && entry.metadata?.failed) {
      const recentFailures = await db.auditLogs.count({
        where: {
          userId: entry.userId,
          action: AuditAction.LOGIN,
          'metadata.failed': true,
          timestamp: { gte: new Date(Date.now() - 15 * 60 * 1000) }
        }
      })
      
      if (recentFailures >= 5) {
        await this.triggerSecurityAlert('Multiple failed login attempts', entry)
      }
    }
    
    // Unusual export activity
    if (entry.action === AuditAction.DATA_EXPORT) {
      const recentExports = await db.auditLogs.count({
        where: {
          userId: entry.userId,
          action: AuditAction.DATA_EXPORT,
          timestamp: { gte: new Date(Date.now() - 60 * 60 * 1000) }
        }
      })
      
      if (recentExports >= 10) {
        await this.triggerSecurityAlert('Excessive data exports', entry)
      }
    }
  }
}
```

## File Upload Security

```typescript
// lib/security/file-upload.ts
import multer from 'multer'
import { Magic, MAGIC_MIME_TYPE } from 'mmmagic'
import sharp from 'sharp'
import ClamScan from 'clamscan'

export class SecureFileUpload {
  private magic = new Magic(MAGIC_MIME_TYPE)
  private clamscan: any
  
  constructor() {
    this.initializeAntivirus()
  }
  
  private async initializeAntivirus() {
    this.clamscan = await new ClamScan().init({
      clamdscan: {
        host: process.env.CLAMAV_HOST,
        port: process.env.CLAMAV_PORT
      }
    })
  }
  
  getMulterConfig() {
    return multer({
      storage: multer.memoryStorage(),
      limits: {
        fileSize: 10 * 1024 * 1024, // 10MB
        files: 1
      },
      fileFilter: async (req, file, cb) => {
        // Check file extension
        const allowedExtensions = ['.jpg', '.jpeg', '.png', '.pdf']
        const ext = path.extname(file.originalname).toLowerCase()
        
        if (!allowedExtensions.includes(ext)) {
          return cb(new Error('Invalid file type'))
        }
        
        cb(null, true)
      }
    })
  }
  
  async processUpload(file: Express.Multer.File) {
    // Verify MIME type
    const mimeType = await this.detectMimeType(file.buffer)
    const allowedMimes = ['image/jpeg', 'image/png', 'application/pdf']
    
    if (!allowedMimes.includes(mimeType)) {
      throw new Error('Invalid file type detected')
    }
    
    // Scan for viruses
    const { isInfected } = await this.clamscan.scanBuffer(file.buffer)
    if (isInfected) {
      throw new Error('Malicious file detected')
    }
    
    // Process images
    if (mimeType.startsWith('image/')) {
      return this.processImage(file.buffer)
    }
    
    return file.buffer
  }
  
  private async processImage(buffer: Buffer) {
    // Remove EXIF data and resize
    return sharp(buffer)
      .rotate() // Auto-rotate based on EXIF
      .resize(2000, 2000, {
        fit: 'inside',
        withoutEnlargement: true
      })
      .jpeg({ quality: 85 })
      .toBuffer()
  }
  
  private detectMimeType(buffer: Buffer): Promise<string> {
    return new Promise((resolve, reject) => {
      this.magic.detect(buffer, (err, result) => {
        if (err) reject(err)
        else resolve(result as string)
      })
    })
  }
}
```

## Secrets Management

```typescript
// lib/security/secrets.ts
import { SecretManagerServiceClient } from '@google-cloud/secret-manager'
import { Redis } from 'ioredis'

export class SecretsManager {
  private client = new SecretManagerServiceClient()
  private cache = new Redis()
  private projectId = process.env.GCP_PROJECT_ID!
  
  async getSecret(name: string): Promise<string> {
    // Check cache first
    const cached = await this.cache.get(`secret:${name}`)
    if (cached) return cached
    
    // Fetch from Secret Manager
    const [version] = await this.client.accessSecretVersion({
      name: `projects/${this.projectId}/secrets/${name}/versions/latest`
    })
    
    const secret = version.payload?.data?.toString()
    if (!secret) {
      throw new Error(`Secret ${name} not found`)
    }
    
    // Cache for 5 minutes
    await this.cache.setex(`secret:${name}`, 300, secret)
    
    return secret
  }
  
  async rotateSecret(name: string, newValue: string) {
    // Add new version
    await this.client.addSecretVersion({
      parent: `projects/${this.projectId}/secrets/${name}`,
      payload: {
        data: Buffer.from(newValue)
      }
    })
    
    // Clear cache
    await this.cache.del(`secret:${name}`)
    
    // Audit log
    await auditLogger.log({
      action: AuditAction.SECRET_ROTATION,
      metadata: { secretName: name }
    })
  }
}

// Environment variable validation
export function validateEnvironment() {
  const required = [
    'DATABASE_URL',
    'JWT_ACCESS_SECRET',
    'JWT_REFRESH_SECRET',
    'ENCRYPTION_KEY',
    'ENCRYPTION_SALT',
    'SUPABASE_SERVICE_ROLE_KEY'
  ]
  
  const missing = required.filter(key => !process.env[key])
  
  if (missing.length > 0) {
    throw new Error(`Missing required environment variables: ${missing.join(', ')}`)
  }
  
  // Validate format
  if (process.env.ENCRYPTION_KEY!.length < 32) {
    throw new Error('ENCRYPTION_KEY must be at least 32 characters')
  }
}
```

## Security Monitoring

```typescript
// lib/security/monitoring.ts
export class SecurityMonitor {
  async detectAnomalies(userId: string) {
    const recentActivity = await this.getUserActivity(userId, 24) // 24 hours
    
    // Check for unusual patterns
    const anomalies = []
    
    // Unusual login location
    const locations = recentActivity
      .filter(a => a.action === 'login')
      .map(a => a.metadata.location)
    
    if (this.hasUnusualLocation(locations)) {
      anomalies.push({
        type: 'unusual_location',
        severity: 'medium',
        description: 'Login from new location detected'
      })
    }
    
    // Excessive data access
    const dataAccess = recentActivity.filter(a => 
      ['report.generate', 'data.export'].includes(a.action)
    ).length
    
    if (dataAccess > 50) {
      anomalies.push({
        type: 'excessive_data_access',
        severity: 'high',
        description: 'Unusual amount of data access'
      })
    }
    
    // Time-based anomalies
    const nightActivity = recentActivity.filter(a => {
      const hour = new Date(a.timestamp).getHours()
      return hour >= 0 && hour <= 6
    })
    
    if (nightActivity.length > 10) {
      anomalies.push({
        type: 'unusual_time',
        severity: 'low',
        description: 'Activity during unusual hours'
      })
    }
    
    return anomalies
  }
  
  async generateSecurityReport(organizationId: string) {
    const report = {
      period: {
        start: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000),
        end: new Date()
      },
      summary: {
        totalLogins: 0,
        failedLogins: 0,
        suspiciousActivities: 0,
        dataExports: 0
      },
      incidents: [],
      recommendations: []
    }
    
    // Populate report...
    
    return report
  }
}
```

## Security Checklist

### Pre-Launch Security Audit

- [ ] **Authentication**
  - [ ] Password policy enforced
  - [ ] Account lockout after failed attempts
  - [ ] Session timeout configured
  - [ ] MFA available for all users

- [ ] **Authorization**
  - [ ] RBAC properly configured
  - [ ] API permissions checked
  - [ ] Resource-level access control
  - [ ] Admin actions require MFA

- [ ] **Data Protection**
  - [ ] Sensitive fields encrypted
  - [ ] Backups encrypted
  - [ ] TLS 1.3 enforced
  - [ ] PII data masked in logs

- [ ] **Input Validation**
  - [ ] All inputs validated
  - [ ] SQL injection prevented
  - [ ] XSS protection in place
  - [ ] File upload restrictions

- [ ] **API Security**
  - [ ] Rate limiting configured
  - [ ] API keys properly managed
  - [ ] CORS configured correctly
  - [ ] Webhook signatures verified

- [ ] **Monitoring**
  - [ ] Audit logging enabled
  - [ ] Security alerts configured
  - [ ] Anomaly detection active
  - [ ] Incident response plan

- [ ] **Compliance**
  - [ ] GDPR compliance
  - [ ] PCI DSS if handling cards
  - [ ] Local data protection laws
  - [ ] Security policies documented