-- FlowTrace Database Schema
-- Migration 005: Access Groups & Visibility System
-- Phase 2 Implementation: Custom visibility controls

-- ============================================================================
-- ACCESS GROUPS
-- Organizations define custom groups with granular permissions
-- ============================================================================

CREATE TABLE access_groups (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  description TEXT,
  icon TEXT DEFAULT 'users',
  color TEXT DEFAULT '#6366f1',

  -- Permissions (what this group can do/see)
  permissions JSONB NOT NULL DEFAULT '{
    "can_view_nodes": true,
    "can_view_amounts": true,
    "can_view_proofs": false,
    "can_view_executor": false,
    "can_view_full_proofs": false,
    "can_edit_nodes": false,
    "can_edit_transactions": false,
    "can_manage_members": false,
    "can_manage_groups": false
  }',

  -- Scope control
  node_scope TEXT DEFAULT 'assigned', -- 'all', 'assigned', 'downstream'

  -- System flags
  is_default BOOLEAN DEFAULT FALSE,
  is_public BOOLEAN DEFAULT FALSE, -- Public group = no login required
  priority INTEGER DEFAULT 100, -- Lower = higher priority for conflicts

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),

  UNIQUE(organization_id, slug)
);

-- ============================================================================
-- GROUP MEMBERS
-- Link users to groups with optional time-based access
-- ============================================================================

CREATE TABLE group_members (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  group_id UUID NOT NULL REFERENCES access_groups(id) ON DELETE CASCADE,

  -- User identification (can be profile or external email)
  user_id TEXT REFERENCES profiles(id) ON DELETE CASCADE,
  user_email TEXT,

  -- Access control
  granted_by TEXT REFERENCES profiles(id),
  valid_from TIMESTAMPTZ DEFAULT NOW(),
  valid_until TIMESTAMPTZ, -- NULL = never expires

  -- Metadata
  notes TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),

  -- Ensure either user_id or user_email is set
  CHECK (user_id IS NOT NULL OR user_email IS NOT NULL),

  -- Unique per user per group
  UNIQUE NULLS NOT DISTINCT (group_id, user_id),
  UNIQUE NULLS NOT DISTINCT (group_id, user_email)
);

-- ============================================================================
-- NODE VISIBILITY
-- Define which groups can see which nodes (overrides group defaults)
-- ============================================================================

CREATE TABLE node_visibility (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  node_id UUID NOT NULL REFERENCES flow_nodes(id) ON DELETE CASCADE,
  group_id UUID NOT NULL REFERENCES access_groups(id) ON DELETE CASCADE,

  -- Permission overrides (NULL = use group default)
  can_view BOOLEAN DEFAULT TRUE,
  can_view_amounts BOOLEAN,
  can_view_proofs BOOLEAN,
  can_view_executor BOOLEAN,

  -- Display customization for this group
  display_name TEXT, -- Alternative name for limited visibility

  created_at TIMESTAMPTZ DEFAULT NOW(),

  UNIQUE(node_id, group_id)
);

-- ============================================================================
-- TRANSACTION VISIBILITY
-- Optional overrides for transaction-level visibility
-- ============================================================================

CREATE TABLE transaction_visibility (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  group_id UUID NOT NULL REFERENCES access_groups(id) ON DELETE CASCADE,

  -- Permission overrides
  can_view BOOLEAN DEFAULT TRUE,
  can_view_amount BOOLEAN,
  can_view_proofs BOOLEAN,
  can_view_executor BOOLEAN,

  created_at TIMESTAMPTZ DEFAULT NOW(),

  UNIQUE(transaction_id, group_id)
);

-- ============================================================================
-- PUBLIC EMBED CONFIGURATION
-- Customize what the public sees on embeddable widgets
-- ============================================================================

CREATE TABLE public_embed_config (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

  -- Display settings
  display_mode TEXT DEFAULT 'simplified' CHECK (display_mode IN ('simplified', 'detailed', 'custom')),
  show_amounts BOOLEAN DEFAULT TRUE,
  amount_format TEXT DEFAULT 'ranges' CHECK (amount_format IN ('exact', 'ranges', 'percentages', 'hidden')),
  show_node_names BOOLEAN DEFAULT TRUE,
  show_trust_score BOOLEAN DEFAULT TRUE,
  show_transaction_count BOOLEAN DEFAULT TRUE,
  show_last_updated BOOLEAN DEFAULT TRUE,

  -- Node filtering
  visible_node_types TEXT[] DEFAULT ARRAY['donor', 'department', 'project', 'beneficiary'],
  hidden_node_ids UUID[] DEFAULT '{}',

  -- Styling
  theme JSONB DEFAULT '{
    "primary_color": "#6366f1",
    "background": "light",
    "border_radius": "lg",
    "show_logo": true
  }',

  -- Embed settings
  allow_embed BOOLEAN DEFAULT TRUE,
  allowed_domains TEXT[] DEFAULT '{}', -- Empty = allow all
  embed_token TEXT UNIQUE DEFAULT encode(gen_random_bytes(16), 'hex'),

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),

  UNIQUE(organization_id)
);

-- ============================================================================
-- INDEXES
-- ============================================================================

CREATE INDEX idx_access_groups_org ON access_groups(organization_id);
CREATE INDEX idx_access_groups_slug ON access_groups(organization_id, slug);
CREATE INDEX idx_access_groups_public ON access_groups(is_public) WHERE is_public = TRUE;

CREATE INDEX idx_group_members_group ON group_members(group_id);
CREATE INDEX idx_group_members_user ON group_members(user_id);
CREATE INDEX idx_group_members_email ON group_members(user_email);
CREATE INDEX idx_group_members_valid ON group_members(valid_from, valid_until);

CREATE INDEX idx_node_visibility_node ON node_visibility(node_id);
CREATE INDEX idx_node_visibility_group ON node_visibility(group_id);

CREATE INDEX idx_transaction_visibility_txn ON transaction_visibility(transaction_id);
CREATE INDEX idx_transaction_visibility_group ON transaction_visibility(group_id);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

CREATE TRIGGER access_groups_updated_at
  BEFORE UPDATE ON access_groups
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER public_embed_config_updated_at
  BEFORE UPDATE ON public_embed_config
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- DEFAULT GROUPS FUNCTION
-- Auto-create default groups when organization is created
-- ============================================================================

CREATE OR REPLACE FUNCTION create_default_access_groups()
RETURNS TRIGGER AS $$
BEGIN
  -- Board: Full access
  INSERT INTO access_groups (organization_id, name, slug, description, icon, color, is_default, priority, permissions)
  VALUES (
    NEW.id,
    'Board',
    'board',
    'Board members and executives with full access',
    'shield-check',
    '#10b981',
    TRUE,
    10,
    '{
      "can_view_nodes": true,
      "can_view_amounts": true,
      "can_view_proofs": true,
      "can_view_executor": true,
      "can_view_full_proofs": true,
      "can_edit_nodes": true,
      "can_edit_transactions": true,
      "can_manage_members": true,
      "can_manage_groups": true
    }'::jsonb
  );

  -- Staff: Operational access
  INSERT INTO access_groups (organization_id, name, slug, description, icon, color, is_default, priority, permissions)
  VALUES (
    NEW.id,
    'Staff',
    'staff',
    'Internal staff with operational access',
    'users',
    '#6366f1',
    TRUE,
    20,
    '{
      "can_view_nodes": true,
      "can_view_amounts": true,
      "can_view_proofs": true,
      "can_view_executor": true,
      "can_view_full_proofs": false,
      "can_edit_nodes": true,
      "can_edit_transactions": true,
      "can_manage_members": false,
      "can_manage_groups": false
    }'::jsonb
  );

  -- Donors: Downstream visibility
  INSERT INTO access_groups (organization_id, name, slug, description, icon, color, is_default, priority, permissions, node_scope)
  VALUES (
    NEW.id,
    'Donors',
    'donors',
    'Donors can see where their money goes',
    'heart',
    '#ec4899',
    TRUE,
    30,
    '{
      "can_view_nodes": true,
      "can_view_amounts": true,
      "can_view_proofs": true,
      "can_view_executor": false,
      "can_view_full_proofs": false,
      "can_edit_nodes": false,
      "can_edit_transactions": false,
      "can_manage_members": false,
      "can_manage_groups": false
    }'::jsonb,
    'downstream'
  );

  -- Partners: Limited to their transactions
  INSERT INTO access_groups (organization_id, name, slug, description, icon, color, is_default, priority, permissions, node_scope)
  VALUES (
    NEW.id,
    'Partners',
    'partners',
    'Partner organizations see transactions involving them',
    'handshake',
    '#f59e0b',
    TRUE,
    40,
    '{
      "can_view_nodes": true,
      "can_view_amounts": true,
      "can_view_proofs": true,
      "can_view_executor": false,
      "can_view_full_proofs": false,
      "can_edit_nodes": false,
      "can_edit_transactions": false,
      "can_manage_members": false,
      "can_manage_groups": false
    }'::jsonb,
    'assigned'
  );

  -- Public: Customizable public view
  INSERT INTO access_groups (organization_id, name, slug, description, icon, color, is_default, is_public, priority, permissions)
  VALUES (
    NEW.id,
    'Public',
    'public',
    'Public view - customize what visitors see',
    'globe',
    '#8b5cf6',
    TRUE,
    TRUE,
    100,
    '{
      "can_view_nodes": true,
      "can_view_amounts": false,
      "can_view_proofs": false,
      "can_view_executor": false,
      "can_view_full_proofs": false,
      "can_edit_nodes": false,
      "can_edit_transactions": false,
      "can_manage_members": false,
      "can_manage_groups": false
    }'::jsonb
  );

  -- Also create default public embed config
  INSERT INTO public_embed_config (organization_id)
  VALUES (NEW.id);

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER organizations_create_default_groups
  AFTER INSERT ON organizations
  FOR EACH ROW EXECUTE FUNCTION create_default_access_groups();

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

-- Get user's effective permissions for an organization
CREATE OR REPLACE FUNCTION get_user_permissions(
  p_user_id TEXT,
  p_org_id UUID
)
RETURNS JSONB AS $$
DECLARE
  v_permissions JSONB := '{}';
  v_group RECORD;
BEGIN
  -- Aggregate permissions from all active group memberships
  -- Higher priority groups override lower priority
  FOR v_group IN
    SELECT ag.permissions, ag.priority, ag.node_scope
    FROM access_groups ag
    JOIN group_members gm ON gm.group_id = ag.id
    WHERE ag.organization_id = p_org_id
      AND gm.user_id = p_user_id
      AND gm.valid_from <= NOW()
      AND (gm.valid_until IS NULL OR gm.valid_until > NOW())
    ORDER BY ag.priority ASC
  LOOP
    -- Merge permissions, first (highest priority) wins
    v_permissions := v_group.permissions || v_permissions;
  END LOOP;

  RETURN v_permissions;
END;
$$ LANGUAGE plpgsql STABLE;

-- Check if user can view a specific node
CREATE OR REPLACE FUNCTION can_view_node(
  p_user_id TEXT,
  p_node_id UUID
)
RETURNS BOOLEAN AS $$
DECLARE
  v_org_id UUID;
  v_can_view BOOLEAN := FALSE;
BEGIN
  -- Get node's organization
  SELECT organization_id INTO v_org_id
  FROM flow_nodes
  WHERE id = p_node_id;

  IF v_org_id IS NULL THEN
    RETURN FALSE;
  END IF;

  -- Check explicit node visibility grants
  SELECT EXISTS (
    SELECT 1
    FROM node_visibility nv
    JOIN group_members gm ON gm.group_id = nv.group_id
    WHERE nv.node_id = p_node_id
      AND nv.can_view = TRUE
      AND gm.user_id = p_user_id
      AND gm.valid_from <= NOW()
      AND (gm.valid_until IS NULL OR gm.valid_until > NOW())
  ) INTO v_can_view;

  IF v_can_view THEN
    RETURN TRUE;
  END IF;

  -- Check group-level permissions
  SELECT (ag.permissions->>'can_view_nodes')::boolean INTO v_can_view
  FROM access_groups ag
  JOIN group_members gm ON gm.group_id = ag.id
  WHERE ag.organization_id = v_org_id
    AND gm.user_id = p_user_id
    AND gm.valid_from <= NOW()
    AND (gm.valid_until IS NULL OR gm.valid_until > NOW())
    AND ag.node_scope = 'all'
  ORDER BY ag.priority ASC
  LIMIT 1;

  RETURN COALESCE(v_can_view, FALSE);
END;
$$ LANGUAGE plpgsql STABLE;

-- Get visible node count context for trust score display
CREATE OR REPLACE FUNCTION get_visibility_context(
  p_user_id TEXT,
  p_org_id UUID
)
RETURNS JSONB AS $$
DECLARE
  v_total_nodes INTEGER;
  v_visible_nodes INTEGER;
  v_total_transactions INTEGER;
  v_visible_transactions INTEGER;
BEGIN
  -- Count total nodes
  SELECT COUNT(*) INTO v_total_nodes
  FROM flow_nodes
  WHERE organization_id = p_org_id;

  -- Count visible nodes for user
  SELECT COUNT(*) INTO v_visible_nodes
  FROM flow_nodes fn
  WHERE fn.organization_id = p_org_id
    AND can_view_node(p_user_id, fn.id);

  -- Count total transactions
  SELECT COUNT(*) INTO v_total_transactions
  FROM transactions
  WHERE organization_id = p_org_id;

  -- For now, visible transactions = same ratio (simplified)
  v_visible_transactions := CASE
    WHEN v_total_nodes > 0
    THEN ROUND(v_total_transactions * v_visible_nodes::numeric / v_total_nodes)
    ELSE 0
  END;

  RETURN jsonb_build_object(
    'total_nodes', v_total_nodes,
    'visible_nodes', v_visible_nodes,
    'total_transactions', v_total_transactions,
    'visible_transactions', v_visible_transactions,
    'visibility_percentage', CASE
      WHEN v_total_nodes > 0
      THEN ROUND(v_visible_nodes::numeric / v_total_nodes * 100)
      ELSE 0
    END
  );
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- VIEWS
-- ============================================================================

-- Active group memberships view
CREATE VIEW active_group_memberships AS
SELECT
  gm.*,
  ag.name AS group_name,
  ag.slug AS group_slug,
  ag.permissions,
  ag.node_scope,
  ag.organization_id
FROM group_members gm
JOIN access_groups ag ON ag.id = gm.group_id
WHERE gm.valid_from <= NOW()
  AND (gm.valid_until IS NULL OR gm.valid_until > NOW());

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE access_groups IS 'Custom access groups defined by organizations for visibility control';
COMMENT ON TABLE group_members IS 'User membership in access groups with time-based access';
COMMENT ON TABLE node_visibility IS 'Per-node visibility overrides for specific groups';
COMMENT ON TABLE transaction_visibility IS 'Per-transaction visibility overrides for specific groups';
COMMENT ON TABLE public_embed_config IS 'Configuration for public embeddable widgets';

COMMENT ON FUNCTION get_user_permissions IS 'Returns aggregated permissions for a user in an organization';
COMMENT ON FUNCTION can_view_node IS 'Checks if a user can view a specific node';
COMMENT ON FUNCTION get_visibility_context IS 'Returns visibility metrics for trust score context display';
