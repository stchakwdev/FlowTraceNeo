# FlowTrace Backend Architecture

## Overview

> Superseded: the Supabase project is deleted and the web app's Supabase client, edge function calls and `VITE_SUPABASE_*` settings are removed (RW3). Every record (books, campaigns, donations, the organisation profile, transactions, flows, chains, proofs, access groups, the trust score, the embed settings) runs on the SpacetimeDB module (Docs/adr/0001-native-ledger.md, rust/docs/adr/0004-transparency-tables.md, flowtrace/README.md). The text below is kept as history; `flowtrace/supabase/migrations/` keeps the old RLS rules the module mirrors.

FlowTrace used **Supabase** for database, storage, and real-time features (historical). Authentication is handled by **WorkOS AuthKit** (see flowtrace-architecture.md).

---

## Database Schema

### Core Tables

```sql
-- Organizations
CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  type TEXT CHECK (type IN ('NGO', 'SME', 'SOCIAL_ENTERPRISE', 'GOVERNMENT')),
  country CHAR(2) NOT NULL,
  currency CHAR(3) NOT NULL DEFAULT 'USD',
  trust_score INTEGER DEFAULT 50 CHECK (trust_score >= 0 AND trust_score <= 100),
  verified BOOLEAN DEFAULT FALSE,
  public_slug TEXT UNIQUE,
  logo_url TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User Profiles (linked to WorkOS user IDs)
CREATE TABLE profiles (
  id TEXT PRIMARY KEY,  -- WorkOS user ID
  organization_id UUID REFERENCES organizations(id),
  full_name TEXT NOT NULL,
  phone TEXT,
  role TEXT DEFAULT 'member' CHECK (role IN ('admin', 'manager', 'member', 'viewer')),
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Flow Nodes (accounts, departments, vendors, etc.)
CREATE TABLE flow_nodes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT CHECK (type IN ('account', 'department', 'vendor', 'beneficiary', 'project')),
  balance DECIMAL(15,2) DEFAULT 0,
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Transactions
CREATE TABLE transactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
  from_node_id UUID REFERENCES flow_nodes(id),
  to_node_id UUID REFERENCES flow_nodes(id),
  amount DECIMAL(15,2) NOT NULL CHECK (amount > 0),
  currency CHAR(3) NOT NULL,
  exchange_rate DECIMAL(10,6),
  amount_usd DECIMAL(15,2),
  description TEXT,
  category TEXT,
  reference TEXT,
  verification_status TEXT DEFAULT 'pending'
    CHECK (verification_status IN ('pending', 'verified', 'flagged', 'rejected')),
  metadata JSONB DEFAULT '{}',
  transaction_date TIMESTAMPTZ DEFAULT NOW(),
  created_by TEXT REFERENCES profiles(id),
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Verifications
CREATE TABLE verifications (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
  type TEXT CHECK (type IN ('receipt', 'photo', 'gps', 'signature', 'biometric', 'bank_statement')),
  file_url TEXT,
  ocr_data JSONB,
  status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'verified', 'rejected')),
  verified_by TEXT REFERENCES profiles(id),
  verified_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Trust Score History
CREATE TABLE trust_score_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
  score INTEGER NOT NULL,
  breakdown JSONB NOT NULL,
  calculated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Indexes

```sql
CREATE INDEX idx_transactions_org ON transactions(organization_id);
CREATE INDEX idx_transactions_date ON transactions(transaction_date DESC);
CREATE INDEX idx_transactions_status ON transactions(verification_status);
CREATE INDEX idx_flow_nodes_org ON flow_nodes(organization_id);
CREATE INDEX idx_verifications_txn ON verifications(transaction_id);
```

---

## Row Level Security (RLS)

```sql
-- Enable RLS
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE transactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE verifications ENABLE ROW LEVEL SECURITY;

-- Organization access
CREATE POLICY "Users can view own org"
  ON organizations FOR SELECT
  USING (id IN (
    SELECT organization_id FROM profiles WHERE id = current_user_id()
  ));

-- Transaction access
CREATE POLICY "Users can view org transactions"
  ON transactions FOR SELECT
  USING (organization_id IN (
    SELECT organization_id FROM profiles WHERE id = current_user_id()
  ));

CREATE POLICY "Managers can create transactions"
  ON transactions FOR INSERT
  WITH CHECK (
    organization_id IN (
      SELECT organization_id FROM profiles
      WHERE id = current_user_id() AND role IN ('admin', 'manager')
    )
  );

-- Public organization data (for transparency pages)
CREATE POLICY "Anyone can view public orgs"
  ON organizations FOR SELECT
  USING (verified = TRUE AND public_slug IS NOT NULL);
```

---

## Supabase Client Setup

> Removed in RW3: there is no `src/lib/supabase.ts` and no `@supabase/supabase-js` dependency.

```typescript
// src/lib/supabase.ts
import { createClient } from '@supabase/supabase-js'
import type { Database } from '@/types/database.types'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

export const supabase = createClient<Database>(supabaseUrl, supabaseAnonKey)

// Server-side with service role (Edge Functions only)
export const supabaseAdmin = createClient<Database>(
  supabaseUrl,
  import.meta.env.SUPABASE_SERVICE_ROLE_KEY
)
```

---

## API Endpoints

### Base URLs
- **Development**: `http://localhost:3000/api/v1`
- **Production**: `https://api.flowtrace.io/v1`

### Response Format

```json
{
  "success": true,
  "data": { },
  "meta": {
    "timestamp": "2025-06-04T10:30:00Z",
    "version": "1.0"
  }
}
```

### Error Format

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed",
    "details": { "amount": ["Amount must be positive"] }
  }
}
```

---

## Transaction Endpoints

### POST /transactions

Create a new transaction.

```json
// Request
{
  "amount": 50000,
  "currency": "KES",
  "fromNodeId": "node_abc123",
  "toNodeId": "node_def456",
  "description": "Water pump maintenance",
  "category": "operations"
}

// Response (201)
{
  "success": true,
  "data": {
    "id": "txn_xyz789",
    "amount": 50000,
    "verificationStatus": "pending",
    "trustScoreImpact": -2
  }
}
```

### GET /transactions

List with filtering and pagination.

**Query Parameters:**
- `page`, `pageSize` (default: 20)
- `status` - verification status
- `category` - transaction category
- `fromDate`, `toDate` - date range
- `search` - text search

### POST /transactions/:id/verify

Add verification to transaction.

```json
// Request
{
  "type": "receipt",
  "fileUrl": "https://storage.flowtrace.io/receipts/ver_123.jpg"
}

// Response
{
  "success": true,
  "data": {
    "verificationId": "ver_456",
    "status": "processing",
    "trustScoreImpact": +3
  }
}
```

---

## Trust Score Endpoints

### GET /trust-score

Get current score with breakdown.

```json
{
  "success": true,
  "data": {
    "score": 87,
    "grade": "A",
    "trend": "improving",
    "breakdown": {
      "transactionVerification": { "score": 92, "weight": 40 },
      "documentationCompleteness": { "score": 85, "weight": 30 },
      "financialConsistency": { "score": 82, "weight": 30 }
    },
    "recommendations": [
      { "action": "Upload missing receipts", "impact": 5, "priority": "high" }
    ]
  }
}
```

---

## Flow Node Endpoints

### POST /flow-nodes

```json
// Request
{
  "name": "Safaricom Ltd",
  "type": "vendor",
  "metadata": { "vendorId": "V-001", "category": "Telecom" }
}
```

### GET /flow-nodes

List all nodes with balances and transaction counts.

---

## Public API (No Auth Required)

### GET /public/organizations/:slug

Public transparency data.

### GET /public/organizations/:slug/flow

Flow visualization data for public embed.

```json
{
  "nodes": [{ "id": "node_1", "label": "Donations", "value": 2500000 }],
  "edges": [{ "source": "node_1", "target": "node_2", "value": 1200000 }]
}
```

---

## Webhooks

### POST /webhooks/mpesa

M-Pesa payment notifications (Daraja API).

```json
// Request (from M-Pesa)
{
  "TransactionType": "Pay Bill",
  "TransID": "RKTQDM7W6S",
  "TransAmount": "1000.00",
  "BusinessShortCode": "123456",
  "MSISDN": "254712345678"
}

// Response
{ "ResultCode": 0, "ResultDesc": "Accepted" }
```

---

## Storage

### Buckets

```sql
-- Create storage buckets
INSERT INTO storage.buckets (id, name, public) VALUES
  ('receipts', 'receipts', false),
  ('documents', 'documents', false),
  ('logos', 'logos', true);
```

### Upload Pattern

```typescript
const { data, error } = await supabase.storage
  .from('receipts')
  .upload(`${orgId}/${transactionId}/${filename}`, file, {
    contentType: 'image/jpeg',
    upsert: false,
  })
```

### RLS for Storage

```sql
CREATE POLICY "Users can upload to own org"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'receipts' AND
    (storage.foldername(name))[1] IN (
      SELECT organization_id::text FROM profiles WHERE id = current_user_id()
    )
  );
```

---

## Edge Functions

### OCR Processing

```typescript
// supabase/functions/process-receipt/index.ts
import { serve } from 'https://deno.land/std@0.168.0/http/server.ts'

serve(async (req) => {
  const { fileUrl, transactionId } = await req.json()

  // Call OCR service (Google Vision, Textract, etc.)
  const ocrResult = await processOCR(fileUrl)

  // Update verification record
  await supabase
    .from('verifications')
    .update({ ocr_data: ocrResult, status: 'verified' })
    .eq('transaction_id', transactionId)

  return new Response(JSON.stringify({ success: true }))
})
```

### Trust Score Calculation

```typescript
// supabase/functions/calculate-trust-score/index.ts
// Runs on schedule via pg_cron

const WEIGHTS = {
  transactionVerification: 0.4,
  documentationCompleteness: 0.3,
  financialConsistency: 0.3,
}

function calculateScore(orgId: string) {
  const verificationRate = await getVerificationRate(orgId)
  const docScore = await getDocumentationScore(orgId)
  const consistencyScore = await getConsistencyScore(orgId)

  return Math.round(
    verificationRate * WEIGHTS.transactionVerification +
    docScore * WEIGHTS.documentationCompleteness +
    consistencyScore * WEIGHTS.financialConsistency
  )
}
```

---

## Real-time Subscriptions

```typescript
// Subscribe to transaction updates
const subscription = supabase
  .channel('transactions')
  .on(
    'postgres_changes',
    {
      event: '*',
      schema: 'public',
      table: 'transactions',
      filter: `organization_id=eq.${orgId}`,
    },
    (payload) => {
      console.log('Transaction changed:', payload)
    }
  )
  .subscribe()
```

---

## Error Codes

| Code | HTTP | Description |
|------|------|-------------|
| `UNAUTHORIZED` | 401 | Missing/invalid auth |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid request data |
| `INSUFFICIENT_BALANCE` | 400 | Not enough funds |
| `RATE_LIMITED` | 429 | Too many requests |

---

## Rate Limiting

- **Authenticated**: 1000 requests/hour/user
- **Unauthenticated**: 100 requests/hour/IP
- **Webhooks**: 10,000 requests/hour/org
