# FlowTrace: Product Roadmap & Feature Specification

## Version 2.0 | January 2025

### Document Control
- **Author**: FlowTrace Product Team
- **Last Updated**: January 8, 2025
- **Status**: Approved
- **Purpose**: Comprehensive implementation guide for FlowTrace's advanced features

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Vision & Strategic Goals](#vision--strategic-goals)
3. [Core Feature Specifications](#core-feature-specifications)
   - [Custom Access Groups](#1-custom-access-groups-visibility-system)
   - [Chain-able Flows](#2-chain-able-flows)
   - [Proof Verification System](#3-proof-verification-system)
   - [Donation Campaigns](#4-donation-campaigns--donor-tracking)
   - [Trust Score System](#5-trust-score-system)
4. [User Personas & Journeys](#user-personas--journeys)
5. [Database Schema](#database-schema)
6. [API Specifications](#api-specifications)
7. [Frontend Components](#frontend-components)
8. [Implementation Phases](#implementation-phases)
9. [Success Metrics](#success-metrics)
10. [Appendix: UI Wireframes](#appendix-ui-wireframes)

---

## Executive Summary

FlowTrace is evolving from a basic financial visualization tool into a **radical transparency platform** that creates trust through:

1. **Custom Access Groups** - Organizations define who sees what, with granular control
2. **Chain-able Flows** - Link flows across organizations for end-to-end transparency
3. **Proof Verification** - Attach receipts, photos, GPS, biometrics to transactions
4. **Donation Campaigns** - Donors track their exact contribution to impact
5. **Enhanced Trust Score** - Quantifiable transparency metric that reflects verification depth

**Target Users**: NGOs, nonprofits, foundations, social enterprises, and impact-focused donors

**Key Differentiator**: No other platform enables donors to follow their specific $80 contribution through an entire funding chain to see "Your donation built 0.5% of this school."

---

## Vision & Strategic Goals

### Vision Statement
> "To become the global standard for financial transparency in the nonprofit sector, enabling radical accountability where every dollar can be traced from donor to impact."

### Strategic Goals

| Timeframe | Goal | Metric |
|-----------|------|--------|
| 6 Months | Launch visibility & proof systems | 50 orgs with verified flows |
| 1 Year | Enable cross-org chains | 100+ active chain links |
| 2 Years | Process $50M+ in donations | 10,000+ donors tracking impact |
| 3 Years | Industry standard for transparency | UN agencies mandate adoption |

### Growth Model

```
1. NGO adopts FlowTrace for internal tracking
         ↓
2. Major donor requests visibility access
         ↓
3. NGO invites donor with custom permissions
         ↓
4. Donor requires other grantees to use FlowTrace
         ↓
5. Network effect expands adoption
         ↓
6. Chains form across organizations
         ↓
7. Public can follow money from source to impact
```

---

## Core Feature Specifications

### 1. Custom Access Groups (Visibility System)

#### Overview
Organizations create custom access groups with granular permissions. Unlike fixed tiers (Level 1-4), groups are fully customizable to match organizational structure and stakeholder relationships.

#### Default Groups (Auto-Created)

| Group Name | Icon | Default Permissions |
|------------|------|---------------------|
| **Board** | Shield | Full access: all nodes, transactions, proofs, executor names |
| **Staff** | Users | Operational nodes, transaction summaries, limited proofs |
| **Donors** | Heart | Their donation path + downstream nodes, proofs on their path |
| **Partners** | Handshake | Transactions involving them, relevant proofs |
| **Public** | Globe | Customized public view defined by organization |

#### Custom Group Creation

Organizations can create unlimited custom groups:

**Example Custom Groups:**
- "USAID Compliance Team" - Full access to USAID-funded programs only
- "Field Auditors" - All operational transactions + GPS proofs
- "Annual Report Readers" - Aggregated data, no individual transactions
- "Board Finance Committee" - All financial details, limited program details

#### Permission Matrix

| Permission | Description | Configurable Per Group |
|------------|-------------|------------------------|
| `view_nodes` | See flow nodes | Yes + can limit to specific node IDs |
| `view_amounts` | See transaction amounts | Yes (exact / ranges / hidden) |
| `view_proofs` | Access attached proofs | Yes (full / thumbnails / none) |
| `view_executor` | See who executed transaction | Yes |
| `view_timestamps` | See exact dates | Yes (exact / month-year / none) |
| `download_reports` | Export data | Yes |
| `receive_updates` | Get email notifications | Yes |

#### Group Assignment Flow

```
1. Admin creates group with name + permissions
2. Admin invites users by email
3. User receives invitation link
4. User accepts → added to group
5. User's view automatically filtered by group permissions
6. Admin can modify group permissions anytime (changes apply immediately)
```

#### Technical Implementation

**Database Tables:**

```sql
-- access_groups: Custom groups per organization
CREATE TABLE access_groups (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  name VARCHAR(100) NOT NULL,
  description TEXT,
  icon VARCHAR(50),              -- Icon identifier for UI
  color VARCHAR(7),              -- Hex color for UI theming
  is_default BOOLEAN DEFAULT FALSE,
  is_public BOOLEAN DEFAULT FALSE, -- True for "Public" group
  permissions JSONB NOT NULL DEFAULT '{
    "view_nodes": true,
    "view_amounts": "exact",
    "view_proofs": "full",
    "view_executor": false,
    "view_timestamps": "exact",
    "download_reports": false,
    "receive_updates": false
  }',
  node_scope UUID[],             -- NULL = all nodes, or specific node IDs
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(organization_id, name)
);

-- group_members: Users assigned to groups
CREATE TABLE group_members (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  group_id UUID NOT NULL REFERENCES access_groups(id) ON DELETE CASCADE,
  user_id UUID REFERENCES users(id),  -- NULL if invited but not yet joined
  user_email VARCHAR(255) NOT NULL,
  invited_by UUID REFERENCES users(id),
  invitation_token VARCHAR(100),
  status VARCHAR(20) DEFAULT 'pending', -- pending, active, revoked
  joined_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(group_id, user_email)
);

-- node_visibility: Override permissions per node
CREATE TABLE node_visibility (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  node_id UUID NOT NULL REFERENCES flow_nodes(id) ON DELETE CASCADE,
  group_id UUID NOT NULL REFERENCES access_groups(id) ON DELETE CASCADE,
  can_view BOOLEAN DEFAULT TRUE,
  can_view_amounts BOOLEAN DEFAULT TRUE,
  can_view_proofs BOOLEAN DEFAULT FALSE,
  can_view_executor BOOLEAN DEFAULT FALSE,
  amount_display VARCHAR(20) DEFAULT 'exact', -- exact, ranges, percentages, hidden
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(node_id, group_id)
);
```

**API Endpoints:**

```typescript
// Create access group
POST /api/organizations/:orgId/groups
Body: {
  name: string,
  description?: string,
  icon?: string,
  color?: string,
  permissions: PermissionObject,
  nodeScope?: string[]  // UUIDs of allowed nodes
}

// Invite user to group
POST /api/groups/:groupId/invite
Body: {
  email: string,
  sendEmail?: boolean
}

// Get flows filtered by user's group
GET /api/organizations/:orgId/flows?userId=xxx
// Returns flows filtered by user's highest-permission group

// Update node visibility for group
PUT /api/nodes/:nodeId/visibility/:groupId
Body: {
  canView: boolean,
  canViewAmounts: boolean,
  amountDisplay: 'exact' | 'ranges' | 'percentages' | 'hidden'
}
```

**UI Components:**

| Component | Purpose | Location |
|-----------|---------|----------|
| `GroupManager` | Create/edit/delete groups | Settings → Access Control |
| `GroupMemberList` | View/manage group members | Settings → Groups → [Group] |
| `InviteModal` | Invite users by email | Groups → Invite |
| `PermissionEditor` | Configure group permissions | Groups → Edit → Permissions |
| `NodeVisibilityToggle` | Per-node visibility override | Flow Diagram → Node → Settings |
| `AccessLevelBadge` | Show current user's access | Header / Flow page |
| `VisibilityIndicator` | "15 of 24 nodes visible" | Flow page stats |

---

### 2. Chain-able Flows

#### Overview
Organizations can link their FlowTrace accounts to create end-to-end transparency across the entire funding chain. A foundation can see how their grant flows through an NGO to local partners to beneficiaries.

#### Chain Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CHAIN EXAMPLE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐   │
│  │   USAID      │────▶│  NGO Kenya   │────▶│ Local Partner │   │
│  │  (Donor)     │     │ (Grantee)    │     │  (Implementer)│   │
│  └──────────────┘     └──────────────┘     └──────────────┘   │
│        │                    │                     │             │
│        ▼                    ▼                     ▼             │
│   Their FlowTrace      Their FlowTrace      Their FlowTrace    │
│   - Shows outflows     - Shows all flows    - Shows delivery   │
│   - Tracks grants      - Internal ops       - Beneficiary data │
│                                                                 │
│  CHAIN LINK 1          CHAIN LINK 2                            │
│  ─────────────         ─────────────                           │
│  USAID → NGO Kenya    NGO Kenya → Local Partner                │
│  (NGO granted access)  (Partner granted access)                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### Chain Consent Flow (Explicit Permission Required)

```
Step 1: REQUEST
┌─────────────────────────────────────────────────────────┐
│ USAID Foundation requests to link to NGO Kenya          │
│                                                         │
│ Request includes:                                       │
│ - Link point: "USAID Grant 2024" node                  │
│ - Requested access: "View downstream flows"             │
│ - Purpose: "Monitor grant utilization"                  │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
Step 2: REVIEW
┌─────────────────────────────────────────────────────────┐
│ NGO Kenya admin reviews request                         │
│                                                         │
│ Options:                                                │
│ ○ Approve with requested permissions                    │
│ ○ Approve with modified permissions                     │
│ ○ Reject with reason                                    │
│                                                         │
│ If approving, NGO defines:                             │
│ - Which group's permissions to grant (e.g., "Donors")  │
│ - Additional restrictions (date range, specific nodes)  │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
Step 3: ACTIVE CHAIN
┌─────────────────────────────────────────────────────────┐
│ Chain link is now active                                │
│                                                         │
│ USAID can now:                                          │
│ - View NGO Kenya flows with "Donors" group permissions │
│ - See their grant flow through NGO's internal nodes    │
│ - Access proofs according to granted permissions       │
│                                                         │
│ NGO Kenya can:                                          │
│ - Revoke access anytime                                │
│ - Modify granted permissions                           │
│ - See chain activity logs                              │
└─────────────────────────────────────────────────────────┘
```

#### Chain Viewer (Unified View)

When viewing a chain, users see a unified flow diagram spanning multiple organizations:

```
┌────────────────────────────────────────────────────────────────────┐
│                     CHAIN FLOW VIEWER                              │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  [USAID Foundation]           [NGO Kenya]           [Local Partner]│
│  ─────────────────           ───────────           ───────────────│
│                                                                    │
│  ┌─────────┐                ┌─────────┐           ┌─────────┐    │
│  │ Grant   │───────────────▶│ General │──────────▶│ Field   │    │
│  │ Fund    │  $500,000      │ Fund    │ $200,000  │ Office  │    │
│  └─────────┘                └────┬────┘           └────┬────┘    │
│                                  │                      │         │
│                                  ▼                      ▼         │
│                            ┌─────────┐           ┌─────────┐     │
│                            │Education│──────────▶│ School  │     │
│                            │ Program │ $150,000  │ Build   │     │
│                            └─────────┘           └─────────┘     │
│                                                        │          │
│  ──────────────────────────────────────────────────────┼──────── │
│  Organization Boundaries                               │          │
│                                                        ▼          │
│                                                  [PROOF: Photo    │
│                                                   of completed    │
│                                                   school]         │
│                                                                    │
│  Legend:                                                          │
│  ● Your organization's nodes                                      │
│  ○ Partner organization's nodes (based on your access level)      │
│  ─ Organization boundary                                          │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

#### Technical Implementation

**Database Tables:**

```sql
-- organization_chains: Chain links between organizations
CREATE TABLE organization_chains (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  -- Source organization (the one requesting to link)
  source_org_id UUID NOT NULL REFERENCES organizations(id),
  source_node_id UUID REFERENCES flow_nodes(id), -- Outflow node from source

  -- Target organization (the one being linked to)
  target_org_id UUID NOT NULL REFERENCES organizations(id),
  target_node_id UUID REFERENCES flow_nodes(id), -- Inflow node in target

  -- Chain configuration
  status VARCHAR(20) DEFAULT 'pending', -- pending, active, rejected, revoked
  granted_group_id UUID REFERENCES access_groups(id), -- Permissions granted to source

  -- Request details
  request_message TEXT,
  request_purpose VARCHAR(255),

  -- Approval details
  rejection_reason TEXT,

  -- Timestamps
  requested_at TIMESTAMPTZ DEFAULT NOW(),
  reviewed_at TIMESTAMPTZ,
  reviewed_by UUID REFERENCES users(id),
  revoked_at TIMESTAMPTZ,
  revoked_by UUID REFERENCES users(id),

  -- Constraints
  UNIQUE(source_org_id, target_org_id, source_node_id, target_node_id)
);

-- chain_activity_log: Audit trail for chain actions
CREATE TABLE chain_activity_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chain_id UUID NOT NULL REFERENCES organization_chains(id),
  action VARCHAR(50) NOT NULL, -- requested, approved, rejected, revoked, accessed
  actor_id UUID REFERENCES users(id),
  actor_org_id UUID REFERENCES organizations(id),
  details JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**API Endpoints:**

```typescript
// Request to link to another organization
POST /api/chains/request
Body: {
  targetOrgId: string,
  sourceNodeId?: string,      // Optional: specific node to link from
  targetNodeId?: string,      // Optional: specific node to link to
  purpose: string,
  message?: string
}

// List chain requests (incoming)
GET /api/organizations/:orgId/chains/requests?status=pending

// Approve/reject chain request
PUT /api/chains/:chainId/review
Body: {
  action: 'approve' | 'reject',
  grantedGroupId?: string,    // Required if approving
  additionalRestrictions?: {
    nodeScope?: string[],
    expiresAt?: string
  },
  rejectionReason?: string    // Required if rejecting
}

// Get chained flow view
GET /api/chains/:chainId/flows
// Returns unified flow from source through target org

// Revoke chain access
DELETE /api/chains/:chainId

// Get all chains for organization
GET /api/organizations/:orgId/chains?direction=incoming|outgoing|both
```

---

### 3. Proof Verification System

#### Overview
Every transaction can have attached proofs that verify the transaction occurred. Proofs contribute to the Trust Score and provide donors/stakeholders with confidence.

#### Proof Types

| Type | Icon | Description | Trust Points | Example |
|------|------|-------------|--------------|---------|
| **Receipt** | Receipt | Scanned/photographed receipt | +5 | Expense receipt |
| **Bank Statement** | Bank | Official bank transaction record | +10 | Wire transfer confirmation |
| **Invoice** | FileText | Formal invoice from vendor | +4 | Supplier invoice |
| **Photo Evidence** | Camera | Photos of goods/services delivered | +8 | Photo of school being built |
| **GPS Verification** | MapPin | Location stamp of transaction | +3 | GPS coordinates of delivery |
| **Video Evidence** | Video | Video proof of delivery/service | +10 | Video of training session |
| **Biometric** | Fingerprint | Fingerprint/signature of recipient | +7 | Beneficiary thumb signature |
| **Third-party Audit** | Shield | External auditor verification | +15 | Audit report attachment |
| **Contract** | FileContract | Signed agreement | +6 | MOU with partner |
| **Government Doc** | Building | Official government document | +8 | Registration certificate |

#### Proof Visibility Levels

Proofs follow node visibility, but can have additional restrictions:

| Visibility | Who Can See | What They See |
|------------|-------------|---------------|
| **Full** | Board, Staff | Original file, all metadata, executor name |
| **Thumbnail** | Donors | Blurred/low-res version, basic metadata |
| **Summary** | Partners | "Receipt attached" indicator, no file access |
| **Hidden** | Public | Nothing, proof not disclosed |

#### Proof Upload Flow

```
1. User navigates to transaction detail
2. Clicks "Add Proof" button
3. Selects proof type from dropdown
4. Uploads file (image, PDF, video)
5. Optionally adds:
   - Executor name (who performed this?)
   - Notes/description
   - GPS coordinates (auto or manual)
6. Sets visibility level
7. Proof attached and trust score updated
```

#### Technical Implementation

**Database Tables:**

```sql
-- proofs: Attached verification documents
CREATE TABLE proofs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  organization_id UUID NOT NULL REFERENCES organizations(id),

  -- Proof classification
  proof_type VARCHAR(50) NOT NULL,
  trust_points INTEGER DEFAULT 0,

  -- File storage
  file_url TEXT NOT NULL,           -- Original file URL (Supabase Storage)
  thumbnail_url TEXT,               -- Blurred/low-res version
  file_type VARCHAR(50),            -- image/jpeg, application/pdf, etc.
  file_size INTEGER,                -- Bytes

  -- Verification details
  executor_name VARCHAR(255),       -- Person who executed transaction
  executor_id UUID REFERENCES users(id),
  description TEXT,

  -- Location data
  gps_latitude DECIMAL(10, 8),
  gps_longitude DECIMAL(11, 8),
  gps_accuracy DECIMAL(10, 2),      -- Meters
  location_name VARCHAR(255),       -- Reverse geocoded address

  -- Verification status
  verified_at TIMESTAMPTZ,
  verified_by UUID REFERENCES users(id),
  verification_method VARCHAR(50),  -- manual, automated, third_party

  -- Visibility
  visibility_level VARCHAR(20) DEFAULT 'full', -- full, thumbnail, summary, hidden

  -- Metadata
  metadata JSONB DEFAULT '{}',      -- Additional type-specific data

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  created_by UUID REFERENCES users(id)
);

-- proof_access_log: Track who viewed proofs (for audit)
CREATE TABLE proof_access_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  proof_id UUID NOT NULL REFERENCES proofs(id),
  accessed_by UUID NOT NULL REFERENCES users(id),
  access_type VARCHAR(20), -- view, download
  access_level VARCHAR(20), -- full, thumbnail, summary
  accessed_at TIMESTAMPTZ DEFAULT NOW()
);
```

**API Endpoints:**

```typescript
// Upload proof to transaction
POST /api/transactions/:transactionId/proofs
Content-Type: multipart/form-data
Body: {
  file: File,
  proofType: string,
  executorName?: string,
  description?: string,
  gpsLatitude?: number,
  gpsLongitude?: number,
  visibilityLevel?: 'full' | 'thumbnail' | 'summary' | 'hidden'
}

// Get proofs for transaction (filtered by user's access)
GET /api/transactions/:transactionId/proofs

// Get proof file (respects visibility)
GET /api/proofs/:proofId/file
// Returns full or thumbnail based on user's access level

// Verify proof (admin action)
PUT /api/proofs/:proofId/verify
Body: {
  verified: boolean,
  verificationMethod: string,
  notes?: string
}

// Delete proof
DELETE /api/proofs/:proofId
```

**UI Components:**

| Component | Purpose |
|-----------|---------|
| `ProofUploader` | Drag-drop file upload with type selection |
| `ProofGallery` | Grid view of all proofs on a transaction |
| `ProofViewer` | Full-screen proof viewer with zoom |
| `ProofIndicator` | Small badge on node showing proof count |
| `ProofTypeSelector` | Dropdown with proof type icons |
| `GPSCapture` | Auto-capture GPS or manual entry |
| `ExecutorInput` | Input for executor name with privacy warning |

---

### 4. Donation Campaigns & Donor Tracking

#### Overview
Organizations can create fundraising campaigns linked to specific outcomes. Individual donors can then track their exact contribution through the entire flow to see their impact.

#### Campaign Structure

```
┌────────────────────────────────────────────────────────────────┐
│                    DONATION CAMPAIGN                           │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Campaign: "Build 10 Schools in Rural Kenya"                   │
│  ─────────────────────────────────────────                     │
│                                                                │
│  Target: $500,000          Current: $387,500 (77.5%)          │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░                      │
│                                                                │
│  Linked Flow Node: "Education Program - School Construction"   │
│                                                                │
│  Options for Donors:                                           │
│  ┌────────────────────────────────────────────────────────┐   │
│  │ □ I want to earmark my donation to:                    │   │
│  │   ○ Any school (recommended - highest impact)          │   │
│  │   ○ School #3 - Kitui District (67% funded)           │   │
│  │   ○ School #7 - Machakos District (12% funded)        │   │
│  │                                                        │   │
│  │ □ If my chosen project is fully funded, you may:      │   │
│  │   ○ Reallocate to another school in this campaign     │   │
│  │   ○ Reallocate to any education program               │   │
│  │   ○ Hold until I decide / Refund me                   │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                │
│  [Donate Now - $___________]                                   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

#### Donor Tracking Experience

After donating, donors can log in and track their contribution:

```
┌────────────────────────────────────────────────────────────────┐
│                    MY DONATION TRACKER                          │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Your Donation: $80 on Jan 5, 2025                             │
│  Campaign: "Build 10 Schools in Rural Kenya"                   │
│  Earmarked to: School #3 - Kitui District                      │
│                                                                │
│  ═══════════════════════════════════════════════════════════  │
│                                                                │
│  FLOW OF YOUR $80:                                             │
│                                                                │
│  [Your Donation]                                               │
│       │ $80                                                    │
│       ▼                                                        │
│  [NGO General Fund] ← Proof: Bank deposit receipt ✓           │
│       │ $80                                                    │
│       ▼                                                        │
│  [Education Program]                                           │
│       │ $80                                                    │
│       ▼                                                        │
│  [School #3 Construction] ← Proof: Invoice for cement ✓       │
│       │ $80                                                    │
│       ▼                                                        │
│  [Completed School] ← Proof: Photo of finished school ✓       │
│                                                                │
│  ═══════════════════════════════════════════════════════════  │
│                                                                │
│  YOUR IMPACT:                                                  │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  Your $80 contributed 0.16% of School #3                 │ │
│  │  Total school cost: $50,000                              │ │
│  │  Your portion: ~8 sq ft of classroom space               │ │
│  │                                                          │ │
│  │  [View Photo of Your Portion] [Share My Impact]          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  UPDATES:                                                      │
│  • Jan 6: Your donation received and deposited                │
│  • Jan 10: Funds transferred to Education Program             │
│  • Jan 15: Construction materials purchased                   │
│  • Feb 20: School construction completed! 🎉                   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

#### Smart Allocation System

Prevents over-subscription and maximizes impact:

```
┌────────────────────────────────────────────────────────────────┐
│                  SMART ALLOCATION                               │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ⚠️  Heads up!                                                 │
│                                                                │
│  School #3 (your selection) is now 95% funded.                │
│  Remaining need: $2,500                                        │
│                                                                │
│  You're donating: $5,000                                       │
│                                                                │
│  SUGGESTED ALLOCATION:                                         │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  $2,500 → School #3 (completes funding!)                 │ │
│  │  $2,500 → School #7 (only 12% funded - high impact!)    │ │
│  │                                                          │ │
│  │  [Accept Suggestion]  [Allocate All to School #3 Anyway] │ │
│  │                       (Overflow will be held)            │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

#### Technical Implementation

**Database Tables:**

```sql
-- campaigns: Fundraising campaigns
CREATE TABLE campaigns (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id),

  -- Campaign details
  name VARCHAR(255) NOT NULL,
  slug VARCHAR(100) UNIQUE,         -- URL-friendly identifier
  description TEXT,
  detailed_description TEXT,         -- Rich text / HTML
  cover_image_url TEXT,

  -- Financial targets
  target_amount DECIMAL(15, 2) NOT NULL,
  current_amount DECIMAL(15, 2) DEFAULT 0,
  currency VARCHAR(3) DEFAULT 'USD',
  minimum_donation DECIMAL(15, 2) DEFAULT 1,

  -- Linked flow
  target_node_id UUID REFERENCES flow_nodes(id), -- What this funds

  -- Campaign status
  status VARCHAR(20) DEFAULT 'draft', -- draft, active, paused, completed, cancelled

  -- Overflow handling
  allow_overflow BOOLEAN DEFAULT TRUE,
  overflow_behavior VARCHAR(50) DEFAULT 'reallocate_campaign',
    -- reallocate_campaign: Move to other sub-targets in same campaign
    -- reallocate_org: Move to other org programs
    -- hold: Hold until donor decides
    -- refund: Refund excess
  overflow_campaign_id UUID REFERENCES campaigns(id),

  -- Timeline
  starts_at TIMESTAMPTZ,
  ends_at TIMESTAMPTZ,

  -- Settings
  allow_earmarking BOOLEAN DEFAULT TRUE,
  show_donor_names BOOLEAN DEFAULT FALSE, -- Public donor wall
  send_updates BOOLEAN DEFAULT TRUE,
  update_frequency VARCHAR(20) DEFAULT 'milestone', -- milestone, weekly, monthly

  -- SEO
  meta_title VARCHAR(100),
  meta_description VARCHAR(200),

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  created_by UUID REFERENCES users(id)
);

-- campaign_sub_targets: Earmarkable sub-projects within campaign
CREATE TABLE campaign_sub_targets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,

  name VARCHAR(255) NOT NULL,
  description TEXT,
  image_url TEXT,

  target_amount DECIMAL(15, 2) NOT NULL,
  current_amount DECIMAL(15, 2) DEFAULT 0,

  node_id UUID REFERENCES flow_nodes(id),

  display_order INTEGER DEFAULT 0,
  is_active BOOLEAN DEFAULT TRUE,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- donations: Individual donor contributions
CREATE TABLE donations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id),
  organization_id UUID NOT NULL REFERENCES organizations(id),

  -- Donor info
  donor_email VARCHAR(255) NOT NULL,
  donor_name VARCHAR(255),
  donor_user_id UUID REFERENCES users(id), -- If registered user
  is_anonymous BOOLEAN DEFAULT FALSE,

  -- Amount
  amount DECIMAL(15, 2) NOT NULL,
  currency VARCHAR(3) DEFAULT 'USD',
  processing_fee DECIMAL(15, 2) DEFAULT 0,
  net_amount DECIMAL(15, 2),              -- After fees

  -- Earmarking
  sub_target_id UUID REFERENCES campaign_sub_targets(id),
  earmarked_node_id UUID REFERENCES flow_nodes(id),

  -- Overflow preferences
  allow_reallocation BOOLEAN DEFAULT TRUE,
  reallocation_preference VARCHAR(50) DEFAULT 'campaign',
    -- campaign: Within same campaign
    -- org: Any org program
    -- hold: Hold for donor decision
    -- refund: Refund if overflow

  -- Flow tracking
  flow_node_id UUID REFERENCES flow_nodes(id), -- Donor's node in flow
  impact_percentage DECIMAL(10, 6),            -- Calculated % of outcome

  -- Payment
  payment_provider VARCHAR(50),      -- stripe, paypal, mpesa, etc.
  payment_id VARCHAR(255),
  payment_status VARCHAR(20) DEFAULT 'pending',

  -- Tracking token (for non-registered donors)
  tracking_token VARCHAR(100) UNIQUE,

  created_at TIMESTAMPTZ DEFAULT NOW(),
  completed_at TIMESTAMPTZ
);

-- donor_updates: Updates sent to donors
CREATE TABLE donor_updates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  donation_id UUID REFERENCES donations(id),
  campaign_id UUID REFERENCES campaigns(id),

  update_type VARCHAR(50) NOT NULL,
    -- money_received, money_moved, proof_added,
    -- milestone_reached, outcome_achieved, campaign_completed

  title VARCHAR(255),
  message TEXT,

  related_node_id UUID REFERENCES flow_nodes(id),
  related_proof_id UUID REFERENCES proofs(id),

  -- Delivery
  email_sent BOOLEAN DEFAULT FALSE,
  email_sent_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- donor_impact: Calculated impact attribution
CREATE TABLE donor_impact (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  donation_id UUID NOT NULL REFERENCES donations(id),

  outcome_node_id UUID NOT NULL REFERENCES flow_nodes(id),
  outcome_description TEXT,

  contribution_amount DECIMAL(15, 2),
  contribution_percentage DECIMAL(10, 6),

  -- Human-readable impact
  impact_description TEXT,  -- "Your $80 built ~8 sq ft of classroom"

  -- Proof of impact
  proof_ids UUID[],

  calculated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**API Endpoints:**

```typescript
// Campaign Management
POST   /api/campaigns                    // Create campaign
GET    /api/campaigns/:id                // Get campaign details
PUT    /api/campaigns/:id                // Update campaign
DELETE /api/campaigns/:id                // Delete campaign
GET    /api/organizations/:orgId/campaigns // List org's campaigns

// Sub-targets
POST   /api/campaigns/:id/sub-targets    // Add sub-target
PUT    /api/campaigns/:campaignId/sub-targets/:id
DELETE /api/campaigns/:campaignId/sub-targets/:id

// Donations
POST   /api/campaigns/:id/donate         // Make donation
GET    /api/donations/:id                // Get donation details
GET    /api/donations/:id/tracking       // Get donor tracking view
GET    /api/donations/track/:token       // Track by token (non-registered)

// Smart Allocation
GET    /api/campaigns/:id/allocation-suggestion?amount=5000
// Returns suggested allocation based on funding levels

// Impact
GET    /api/donations/:id/impact         // Calculate impact attribution
POST   /api/donations/:id/impact/share   // Generate shareable impact card

// Donor Updates
GET    /api/donations/:id/updates        // Get all updates for donation
POST   /api/donations/:id/updates        // Send manual update (admin)
```

---

### 5. Trust Score System

#### Overview
The Trust Score (0-100) quantifies an organization's financial transparency. It's calculated based on verification depth, documentation, consistency, and chain completeness.

#### Score Calculation Formula

```
Trust Score = (Verification × 0.40) + (Documentation × 0.30) +
              (Consistency × 0.20) + (Chain × 0.10)

Where:
- Verification (0-100): % of transactions with attached proofs
- Documentation (0-100): Quality and completeness of proofs
- Consistency (0-100): Flow balance accuracy (inflows ≈ outflows)
- Chain (0-100): % of linked orgs also verified
```

#### Detailed Scoring Components

**Verification Score (40%)**
```
base_score = (transactions_with_proofs / total_transactions) × 100

Bonuses:
+ 5 if > 90% have proofs
+ 3 if average proof quality > 80%

Penalties:
- 10 if any transaction > $10,000 lacks proof
- 5 for each flagged transaction
```

**Documentation Score (30%)**
```
For each proof:
  quality_score = base_type_points + recency_bonus + verification_bonus

base_type_points:
  Receipt: 5
  Bank Statement: 10
  Photo Evidence: 8
  GPS: 3
  Biometric: 7
  Third-party Audit: 15

recency_bonus: +2 if uploaded within 7 days of transaction
verification_bonus: +3 if verified by third party

documentation_score = sum(quality_scores) / max_possible_score × 100
```

**Consistency Score (20%)**
```
For each node:
  balance = total_inflows - total_outflows
  expected_balance = 0 (for pass-through) or declared_balance
  variance = abs(balance - expected_balance) / total_inflows

consistency_score = 100 - (average_variance × 100)

Penalties:
- 10 for any node with > 20% unexplained variance
- 20 for missing transactions in sequence
```

**Chain Score (10%)**
```
If organization has outgoing chains:
  chain_score = (verified_linked_orgs / total_linked_orgs) × 100

  Bonuses:
  + 10 if all linked orgs have Trust Score > 70

If no chains:
  chain_score = 50 (neutral)
```

#### Trust Score Display

```
┌─────────────────────────────────────────────────────────────┐
│                     TRUST SCORE                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│                         78                                  │
│                     ╭───────╮                               │
│                    ╱  GOOD   ╲                              │
│                   ╱           ╲                             │
│                                                             │
│  Verification     Documentation    Consistency    Chain     │
│      85%              72%             80%         70%       │
│  ▓▓▓▓▓▓▓▓░░        ▓▓▓▓▓▓▓░░░      ▓▓▓▓▓▓▓▓░░    ▓▓▓▓▓▓▓░░░ │
│                                                             │
│  Based on: 15 of 24 nodes visible to your group            │
│                                                             │
│  [View Full Breakdown] [Download Report]                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Score Ranges:
90-100: Excellent (Green) - "Verified and Transparent"
70-89:  Good (Blue) - "Well Documented"
50-69:  Fair (Yellow) - "Partially Verified"
30-49:  Needs Work (Orange) - "Limited Documentation"
0-29:   Poor (Red) - "Insufficient Transparency"
```

#### Visibility-Aware Scoring

Trust Score adapts based on viewer's access level:

| Access Level | What They See | Score Basis |
|--------------|---------------|-------------|
| Board | Full score with all details | All nodes, all proofs |
| Donors | Score for visible nodes | Their path + downstream |
| Public | Public-visible score | Public nodes only |

**Display Examples:**
- Board sees: "Trust Score: 78 (full organization)"
- Donor sees: "Trust Score: 82 (based on your donation path)"
- Public sees: "Trust Score: 75 (15 of 24 nodes visible)"

---

## User Personas & Journeys

### Persona 1: NGO Executive Director

**Name**: Sarah Mwangi
**Role**: Executive Director, Water Access Kenya
**Goals**: Increase donor confidence, reduce reporting burden, track spending

**Journey:**
```
1. Signs up for FlowTrace
2. Imports existing transactions from spreadsheet
3. Creates access groups: Board, Staff, Major Donors, Public
4. Invites board members to Board group
5. Staff begin adding proofs to transactions
6. Trust Score improves from 45 → 72
7. Embeds public widget on website
8. Major donor (USAID) requests chain link
9. Approves chain with "Donors" group permissions
10. Creates first donation campaign
```

### Persona 2: Major Donor (Foundation)

**Name**: Michael Chen
**Role**: Program Officer, Global Impact Foundation
**Goals**: Track grant utilization, verify impact, report to board

**Journey:**
```
1. Receives grant request from NGO Kenya
2. Sees NGO's Trust Score badge (72) on their website
3. Requests chain link to see grant flow
4. NGO approves with "Donors" permissions
5. Views unified chain flow: Foundation → NGO → Projects → Impact
6. Sees proofs attached at each stage
7. Generates quarterly report from FlowTrace data
8. Recommends FlowTrace to other grantees
```

### Persona 3: Individual Donor

**Name**: Lisa Thompson
**Role**: Individual donor, annual giving $500-1000
**Goals**: Know where money goes, feel connected to impact

**Journey:**
```
1. Discovers NGO through GiveWell recommendation
2. Visits NGO website, sees FlowTrace widget with Trust Score
3. Clicks "Donate" on "Build Schools" campaign
4. Chooses to earmark $80 to School #3 in Kitui
5. Receives confirmation with tracking link
6. Logs in monthly to see money progress through flow
7. Receives update when school construction completes
8. Sees: "Your $80 built 8 sq ft of classroom (0.16%)"
9. Shares impact card on social media
10. Increases annual giving to $2000
```

---

## Database Schema

### Complete ERD

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FLOWTRACE DATABASE SCHEMA                           │
└─────────────────────────────────────────────────────────────────────────────┘

CORE TABLES (Existing)
─────────────────────
organizations ─────┬──── flow_nodes ────── transactions
                   │
                   └──── users

NEW TABLES (This Roadmap)
─────────────────────────

ACCESS CONTROL:
organizations ──┬── access_groups ──── group_members
                │         │
                │         └── node_visibility
                │
                └── organization_chains ──── chain_activity_log

PROOFS:
transactions ──── proofs ──── proof_access_log

DONATIONS:
organizations ──── campaigns ──┬── campaign_sub_targets
                               │
                               └── donations ──┬── donor_updates
                                               │
                                               └── donor_impact

PUBLIC EMBEDS:
organizations ──── public_embed_config
```

### Migration Order

```sql
-- Migration 001: Access Groups
CREATE TABLE access_groups (...);
CREATE TABLE group_members (...);
CREATE TABLE node_visibility (...);

-- Migration 002: Organization Chains
CREATE TABLE organization_chains (...);
CREATE TABLE chain_activity_log (...);

-- Migration 003: Proofs
CREATE TABLE proofs (...);
CREATE TABLE proof_access_log (...);

-- Migration 004: Campaigns & Donations
CREATE TABLE campaigns (...);
CREATE TABLE campaign_sub_targets (...);
CREATE TABLE donations (...);
CREATE TABLE donor_updates (...);
CREATE TABLE donor_impact (...);

-- Migration 005: Public Embeds
CREATE TABLE public_embed_config (...);

-- Migration 006: Indexes
CREATE INDEX idx_access_groups_org ON access_groups(organization_id);
CREATE INDEX idx_group_members_group ON group_members(group_id);
CREATE INDEX idx_proofs_transaction ON proofs(transaction_id);
CREATE INDEX idx_donations_campaign ON donations(campaign_id);
CREATE INDEX idx_donations_tracking ON donations(tracking_token);
-- ... additional indexes
```

---

## API Specifications

### Authentication

All API endpoints require authentication except:
- `GET /api/public/embed/:orgId` - Public embed data
- `GET /api/public/campaigns/:slug` - Public campaign page
- `GET /api/donations/track/:token` - Donation tracking (token-based)

### Response Format

```typescript
// Success
{
  success: true,
  data: T,
  meta?: {
    pagination?: { page, limit, total, totalPages }
  }
}

// Error
{
  success: false,
  error: {
    code: string,
    message: string,
    details?: object
  }
}
```

### Rate Limits

| Endpoint Category | Rate Limit |
|-------------------|------------|
| Read operations | 1000/min |
| Write operations | 100/min |
| File uploads | 20/min |
| Public endpoints | 60/min |

---

## Frontend Components

### Component Library

| Component | Props | Purpose |
|-----------|-------|---------|
| `<FlowDiagram>` | nodes, edges, selectedNode, onNodeClick | Main flow visualization |
| `<TrustScoreBadge>` | score, size, showBreakdown | Display trust score |
| `<ProofUploader>` | transactionId, onUpload | Upload proofs |
| `<ProofGallery>` | proofs, accessLevel | Display proof thumbnails |
| `<DonationForm>` | campaignId, subTargets, onDonate | Donation flow |
| `<DonorTracker>` | donationId, token | Track donation progress |
| `<GroupManager>` | orgId | Manage access groups |
| `<ChainViewer>` | chainId | View cross-org flow |
| `<CampaignCard>` | campaign | Campaign preview card |
| `<ImpactCard>` | impact, shareable | Show donor impact |
| `<EmbedBuilder>` | orgId | Configure public embed |

### Page Structure

```
/dashboard                    - Main dashboard
/flows                        - Flow visualization
/flows/:nodeId               - Node detail
/transactions                 - Transaction list
/transactions/:id            - Transaction detail + proofs
/settings/groups             - Access group management
/settings/groups/:id         - Group detail + members
/settings/chains             - Chain management
/settings/embed              - Public embed configuration
/campaigns                    - Campaign list
/campaigns/new               - Create campaign
/campaigns/:id               - Campaign detail
/campaigns/:id/donations     - Campaign donations
/donate/:slug                - Public donation page
/track/:token                - Donor tracking page
/embed/:orgId                - Iframe embed endpoint
```

---

## Implementation Phases

### Phase 1: Foundation ✅ (Complete)
- Basic flow visualization
- Transaction tracking
- Trust score display
- User authentication

### Phase 2: Access Groups (Weeks 1-3)
**Week 1:**
- [ ] Create access_groups, group_members tables
- [ ] Build GroupManager component
- [ ] Implement group CRUD API

**Week 2:**
- [ ] Create node_visibility table
- [ ] Implement visibility filtering in queries
- [ ] Build NodeVisibilityToggle component

**Week 3:**
- [ ] Email invitation flow
- [ ] Access level badge/indicator
- [ ] "X of Y nodes visible" display
- [ ] Testing & bug fixes

### Phase 3: Proofs (Weeks 4-6)
**Week 4:**
- [ ] Create proofs table
- [ ] Supabase Storage setup for files
- [ ] ProofUploader component

**Week 5:**
- [ ] ProofGallery component
- [ ] Visibility-aware proof display
- [ ] GPS capture integration

**Week 6:**
- [ ] Trust score calculation update
- [ ] Proof indicators on nodes
- [ ] Testing & bug fixes

### Phase 4: Chains (Weeks 7-9)
**Week 7:**
- [ ] Create organization_chains table
- [ ] Chain request/approval API
- [ ] Chain invitation UI

**Week 8:**
- [ ] ChainViewer component
- [ ] Cross-org flow queries
- [ ] Chain activity logging

**Week 9:**
- [ ] Chain trust score integration
- [ ] Chain management dashboard
- [ ] Testing & bug fixes

### Phase 5: Donations (Weeks 10-13)
**Week 10:**
- [ ] Create campaigns, donations tables
- [ ] Campaign creation UI
- [ ] Stripe/PayPal integration

**Week 11:**
- [ ] Sub-targets & earmarking
- [ ] Smart allocation logic
- [ ] Donation form component

**Week 12:**
- [ ] Donor tracking portal
- [ ] Impact attribution calculator
- [ ] Donor updates system

**Week 13:**
- [ ] Shareable impact cards
- [ ] Email notifications
- [ ] Testing & bug fixes

### Phase 6: Public Features (Weeks 14-16)
**Week 14:**
- [ ] public_embed_config table
- [ ] Embed builder UI
- [ ] Public embed endpoint

**Week 15:**
- [ ] Public campaign pages
- [ ] SEO optimization
- [ ] Social sharing

**Week 16:**
- [ ] Trust score badge generator
- [ ] Performance optimization
- [ ] Final testing

---

## Success Metrics

### Key Performance Indicators (KPIs)

| Metric | 3 Months | 6 Months | 12 Months |
|--------|----------|----------|-----------|
| Organizations onboarded | 25 | 75 | 200 |
| Active users | 100 | 500 | 2,000 |
| Transactions tracked | $1M | $10M | $50M |
| Proofs uploaded | 500 | 5,000 | 25,000 |
| Average Trust Score | 60 | 68 | 75 |
| Active chain links | 5 | 25 | 100 |
| Donation campaigns | 10 | 50 | 200 |
| Individual donations | 100 | 1,000 | 10,000 |
| Public embeds deployed | 10 | 50 | 200 |

### Health Metrics

| Metric | Target |
|--------|--------|
| Page load time | < 2s on 3G |
| API response time | < 200ms |
| Uptime | 99.9% |
| Mobile usage | > 60% |
| User retention (30-day) | > 70% |

---

## Appendix: UI Wireframes

### Flow Diagram with Proofs

```
┌────────────────────────────────────────────────────────────────────────┐
│  Money Flow                                        Week | Month | Year │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────┐         ┌─────────────┐         ┌─────────────┐     │
│  │   USAID     │────────▶│   General   │────────▶│  Education  │     │
│  │   Grant     │ $500K   │    Fund     │ $200K   │   Program   │     │
│  │    📎3      │         │    📎5      │         │    📎8      │     │
│  └─────────────┘         └──────┬──────┘         └──────┬──────┘     │
│                                 │                       │             │
│                                 │ $150K                 │ $150K       │
│                                 ▼                       ▼             │
│                          ┌─────────────┐         ┌─────────────┐     │
│                          │   Health    │         │   School    │     │
│                          │   Program   │         │   Build     │     │
│                          │    📎4      │         │    📎12     │     │
│                          └─────────────┘         └─────────────┘     │
│                                                                        │
│  📎 = Number of attached proofs                                       │
│                                                                        │
│  [+ Add Node]  [Export]  [Share]                                      │
└────────────────────────────────────────────────────────────────────────┘
```

### Donor Tracking View

```
┌────────────────────────────────────────────────────────────────────────┐
│  Track Your Donation                                    Lisa Thompson  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Your $80 donation on January 5, 2025                                 │
│  Campaign: Build 10 Schools in Rural Kenya                            │
│  Earmarked: School #3 - Kitui District                                │
│                                                                        │
│  ────────────────────────────────────────────────────────────────────  │
│                                                                        │
│  JOURNEY OF YOUR DONATION:                                            │
│                                                                        │
│  ✅ Received ──▶ ✅ Transferred ──▶ ✅ Allocated ──▶ ✅ Impact!       │
│     Jan 5          Jan 7             Jan 10           Feb 20          │
│                                                                        │
│  ────────────────────────────────────────────────────────────────────  │
│                                                                        │
│  YOUR IMPACT:                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │                                                                  │ │
│  │   🏫  Your $80 contributed to:                                   │ │
│  │       School #3 - Kitui District                                │ │
│  │                                                                  │ │
│  │       Total Cost: $50,000                                       │ │
│  │       Your Share: 0.16% (~8 sq ft of classroom)                │ │
│  │                                                                  │ │
│  │   [View Photo]  [Share on Twitter]  [Share on LinkedIn]         │ │
│  │                                                                  │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
│  PROOFS ALONG THE WAY:                                                │
│  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐                             │
│  │ 🧾   │  │ 🏦   │  │ 📦   │  │ 🏫   │                             │
│  │Receipt│  │Bank  │  │Cement│  │School│                             │
│  └──────┘  └──────┘  └──────┘  └──────┘                             │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

*Document Version: 2.0*
*Last Updated: January 8, 2025*
*Next Review: April 2025*
