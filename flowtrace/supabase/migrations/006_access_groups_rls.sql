-- FlowTrace Row Level Security Policies
-- Migration 006: RLS Policies for Access Groups

-- Enable RLS on new tables
ALTER TABLE access_groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE group_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE node_visibility ENABLE ROW LEVEL SECURITY;
ALTER TABLE transaction_visibility ENABLE ROW LEVEL SECURITY;
ALTER TABLE public_embed_config ENABLE ROW LEVEL SECURITY;

-- ============================================================================
-- ACCESS GROUPS POLICIES
-- ============================================================================

-- Users in org can view all access groups for their org
CREATE POLICY "Users can view org access groups"
  ON access_groups FOR SELECT
  USING (organization_id = get_current_org_id());

-- Public groups are viewable by anyone (for public embeds)
CREATE POLICY "Public can view public groups"
  ON access_groups FOR SELECT
  USING (is_public = TRUE);

-- Only admins can create access groups
CREATE POLICY "Admins can create access groups"
  ON access_groups FOR INSERT
  WITH CHECK (
    organization_id = get_current_org_id()
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  );

-- Only admins can update access groups
CREATE POLICY "Admins can update access groups"
  ON access_groups FOR UPDATE
  USING (
    organization_id = get_current_org_id()
    AND NOT is_default  -- Cannot modify default groups
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  )
  WITH CHECK (organization_id = get_current_org_id());

-- Only admins can delete non-default access groups
CREATE POLICY "Admins can delete access groups"
  ON access_groups FOR DELETE
  USING (
    organization_id = get_current_org_id()
    AND NOT is_default
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role = 'admin'
    )
  );

-- ============================================================================
-- GROUP MEMBERS POLICIES
-- ============================================================================

-- Users can view members of groups in their org
CREATE POLICY "Users can view group members"
  ON group_members FOR SELECT
  USING (
    group_id IN (
      SELECT id FROM access_groups
      WHERE organization_id = get_current_org_id()
    )
  );

-- Admins/managers can add members
CREATE POLICY "Admins can add group members"
  ON group_members FOR INSERT
  WITH CHECK (
    group_id IN (
      SELECT id FROM access_groups
      WHERE organization_id = get_current_org_id()
    )
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  );

-- Admins/managers can update memberships
CREATE POLICY "Admins can update group members"
  ON group_members FOR UPDATE
  USING (
    group_id IN (
      SELECT id FROM access_groups
      WHERE organization_id = get_current_org_id()
    )
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  );

-- Admins/managers can remove members
CREATE POLICY "Admins can remove group members"
  ON group_members FOR DELETE
  USING (
    group_id IN (
      SELECT id FROM access_groups
      WHERE organization_id = get_current_org_id()
    )
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  );

-- ============================================================================
-- NODE VISIBILITY POLICIES
-- ============================================================================

-- Users can view node visibility rules for their org
CREATE POLICY "Users can view node visibility"
  ON node_visibility FOR SELECT
  USING (
    node_id IN (
      SELECT id FROM flow_nodes
      WHERE organization_id = get_current_org_id()
    )
  );

-- Admins can manage node visibility
CREATE POLICY "Admins can manage node visibility"
  ON node_visibility FOR ALL
  USING (
    node_id IN (
      SELECT id FROM flow_nodes
      WHERE organization_id = get_current_org_id()
    )
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  )
  WITH CHECK (
    node_id IN (
      SELECT id FROM flow_nodes
      WHERE organization_id = get_current_org_id()
    )
  );

-- ============================================================================
-- TRANSACTION VISIBILITY POLICIES
-- ============================================================================

-- Users can view transaction visibility rules for their org
CREATE POLICY "Users can view transaction visibility"
  ON transaction_visibility FOR SELECT
  USING (
    transaction_id IN (
      SELECT id FROM transactions
      WHERE organization_id = get_current_org_id()
    )
  );

-- Admins can manage transaction visibility
CREATE POLICY "Admins can manage transaction visibility"
  ON transaction_visibility FOR ALL
  USING (
    transaction_id IN (
      SELECT id FROM transactions
      WHERE organization_id = get_current_org_id()
    )
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  )
  WITH CHECK (
    transaction_id IN (
      SELECT id FROM transactions
      WHERE organization_id = get_current_org_id()
    )
  );

-- ============================================================================
-- PUBLIC EMBED CONFIG POLICIES
-- ============================================================================

-- Anyone can view public embed config (needed for widget)
CREATE POLICY "Public can view embed config"
  ON public_embed_config FOR SELECT
  USING (allow_embed = TRUE);

-- Users can view their org's embed config
CREATE POLICY "Users can view own embed config"
  ON public_embed_config FOR SELECT
  USING (organization_id = get_current_org_id());

-- Admins can manage embed config
CREATE POLICY "Admins can manage embed config"
  ON public_embed_config FOR ALL
  USING (
    organization_id = get_current_org_id()
    AND EXISTS (
      SELECT 1 FROM profiles
      WHERE id = get_current_user_id()
      AND organization_id = get_current_org_id()
      AND role IN ('admin', 'manager')
    )
  )
  WITH CHECK (organization_id = get_current_org_id());

-- ============================================================================
-- GRANTS
-- ============================================================================

GRANT ALL ON access_groups TO anon, authenticated;
GRANT ALL ON group_members TO anon, authenticated;
GRANT ALL ON node_visibility TO anon, authenticated;
GRANT ALL ON transaction_visibility TO anon, authenticated;
GRANT ALL ON public_embed_config TO anon, authenticated;
GRANT ALL ON active_group_memberships TO anon, authenticated;
