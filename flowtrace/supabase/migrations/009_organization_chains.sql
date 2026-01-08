-- =============================================================================
-- Phase 4: Organization Chains - Cross-Org Flow Linking
-- =============================================================================
-- Creates tables for linking organizations together, enabling end-to-end
-- transparency from donor → NGO → partner → beneficiary.
-- =============================================================================

-- Organization chains table
CREATE TABLE IF NOT EXISTS organization_chains (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  -- The org requesting the link (funder/donor)
  source_org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

  -- The org being linked to (recipient/partner)
  target_org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

  -- Optional: Link to a specific node in target org's flow
  link_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,

  -- Status workflow
  status VARCHAR(20) NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'active', 'rejected', 'revoked')),

  -- What access level does source org get? (Uses existing access group system)
  granted_group_id UUID REFERENCES access_groups(id) ON DELETE SET NULL,

  -- Request metadata
  request_message TEXT, -- Message from source org explaining why
  rejection_reason TEXT, -- Reason if rejected

  -- Audit fields
  requested_by UUID NOT NULL, -- User who initiated
  requested_at TIMESTAMPTZ DEFAULT NOW(),

  approved_by UUID,
  approved_at TIMESTAMPTZ,

  revoked_by UUID,
  revoked_at TIMESTAMPTZ,

  -- Timestamps
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),

  -- Prevent duplicate chains between same orgs
  UNIQUE(source_org_id, target_org_id),

  -- Ensure source and target are different orgs
  CHECK (source_org_id != target_org_id)
);

-- Chain activity log for audit trail
CREATE TABLE IF NOT EXISTS chain_activity_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chain_id UUID NOT NULL REFERENCES organization_chains(id) ON DELETE CASCADE,
  action VARCHAR(50) NOT NULL
    CHECK (action IN ('requested', 'approved', 'rejected', 'revoked', 'permissions_updated', 'node_linked')),
  actor_id UUID NOT NULL,
  actor_org_id UUID NOT NULL REFERENCES organizations(id),
  details JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_chains_source_org ON organization_chains(source_org_id);
CREATE INDEX IF NOT EXISTS idx_chains_target_org ON organization_chains(target_org_id);
CREATE INDEX IF NOT EXISTS idx_chains_status ON organization_chains(status);
CREATE INDEX IF NOT EXISTS idx_chains_requested_at ON organization_chains(requested_at DESC);
CREATE INDEX IF NOT EXISTS idx_chain_activity_chain ON chain_activity_log(chain_id);
CREATE INDEX IF NOT EXISTS idx_chain_activity_actor ON chain_activity_log(actor_id);
CREATE INDEX IF NOT EXISTS idx_chain_activity_created ON chain_activity_log(created_at DESC);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_chain_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER chain_updated_at
BEFORE UPDATE ON organization_chains
FOR EACH ROW EXECUTE FUNCTION update_chain_timestamp();

-- Function to log chain activity
CREATE OR REPLACE FUNCTION log_chain_activity(
  p_chain_id UUID,
  p_action VARCHAR(50),
  p_actor_id UUID,
  p_actor_org_id UUID,
  p_details JSONB DEFAULT '{}'
)
RETURNS UUID AS $$
DECLARE
  v_log_id UUID;
BEGIN
  INSERT INTO chain_activity_log (chain_id, action, actor_id, actor_org_id, details)
  VALUES (p_chain_id, p_action, p_actor_id, p_actor_org_id, p_details)
  RETURNING id INTO v_log_id;

  RETURN v_log_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to get chain count for an organization
CREATE OR REPLACE FUNCTION get_org_chain_counts(p_org_id UUID)
RETURNS TABLE(
  outgoing_pending INTEGER,
  outgoing_active INTEGER,
  incoming_pending INTEGER,
  incoming_active INTEGER
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    (SELECT COUNT(*)::INTEGER FROM organization_chains WHERE source_org_id = p_org_id AND status = 'pending'),
    (SELECT COUNT(*)::INTEGER FROM organization_chains WHERE source_org_id = p_org_id AND status = 'active'),
    (SELECT COUNT(*)::INTEGER FROM organization_chains WHERE target_org_id = p_org_id AND status = 'pending'),
    (SELECT COUNT(*)::INTEGER FROM organization_chains WHERE target_org_id = p_org_id AND status = 'active');
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to calculate aggregated trust score for a chain
CREATE OR REPLACE FUNCTION get_chain_trust_score(p_chain_id UUID)
RETURNS TABLE(
  aggregated_score NUMERIC,
  org_count INTEGER,
  scores JSONB
) AS $$
DECLARE
  v_chain organization_chains;
  v_source_score NUMERIC;
  v_target_score NUMERIC;
BEGIN
  SELECT * INTO v_chain FROM organization_chains WHERE id = p_chain_id AND status = 'active';

  IF v_chain IS NULL THEN
    RETURN;
  END IF;

  SELECT trust_score INTO v_source_score FROM organizations WHERE id = v_chain.source_org_id;
  SELECT trust_score INTO v_target_score FROM organizations WHERE id = v_chain.target_org_id;

  RETURN QUERY
  SELECT
    ((v_source_score + v_target_score) / 2)::NUMERIC AS aggregated_score,
    2 AS org_count,
    jsonb_build_object(
      'source_org', v_source_score,
      'target_org', v_target_score
    ) AS scores;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
