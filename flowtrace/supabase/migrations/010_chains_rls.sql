-- =============================================================================
-- Phase 4: Organization Chains - RLS Policies
-- =============================================================================
-- Row Level Security policies for organization_chains and chain_activity_log
-- =============================================================================

-- Enable RLS on organization_chains
ALTER TABLE organization_chains ENABLE ROW LEVEL SECURITY;

-- Policy: Both source and target orgs can view the chain
CREATE POLICY chains_select_policy ON organization_chains
  FOR SELECT USING (
    source_org_id = get_current_org_id() OR
    target_org_id = get_current_org_id()
  );

-- Policy: Source org can create chain requests to other orgs
CREATE POLICY chains_insert_policy ON organization_chains
  FOR INSERT WITH CHECK (
    source_org_id = get_current_org_id()
  );

-- Policy: Target org can update chain (approve/reject/revoke)
-- Source org can also update to cancel pending requests
CREATE POLICY chains_update_policy ON organization_chains
  FOR UPDATE USING (
    -- Target org can always update (approve, reject, revoke, update permissions)
    target_org_id = get_current_org_id()
    OR
    -- Source org can only update pending chains (to cancel)
    (source_org_id = get_current_org_id() AND status = 'pending')
  );

-- Policy: Source org can delete pending requests (cancel)
CREATE POLICY chains_delete_policy ON organization_chains
  FOR DELETE USING (
    source_org_id = get_current_org_id() AND status = 'pending'
  );

-- =============================================================================
-- Chain Activity Log RLS
-- =============================================================================

ALTER TABLE chain_activity_log ENABLE ROW LEVEL SECURITY;

-- Policy: View activity log if user can see the chain
CREATE POLICY chain_log_select_policy ON chain_activity_log
  FOR SELECT USING (
    EXISTS (
      SELECT 1 FROM organization_chains c
      WHERE c.id = chain_id
      AND (c.source_org_id = get_current_org_id() OR c.target_org_id = get_current_org_id())
    )
  );

-- Policy: Insert activity log for chains involving user's org
CREATE POLICY chain_log_insert_policy ON chain_activity_log
  FOR INSERT WITH CHECK (
    actor_org_id = get_current_org_id()
  );

-- =============================================================================
-- Grant permissions to authenticated users
-- =============================================================================

GRANT SELECT, INSERT, UPDATE, DELETE ON organization_chains TO authenticated;
GRANT SELECT, INSERT ON chain_activity_log TO authenticated;

-- Grant execute on helper functions
GRANT EXECUTE ON FUNCTION log_chain_activity TO authenticated;
GRANT EXECUTE ON FUNCTION get_org_chain_counts TO authenticated;
GRANT EXECUTE ON FUNCTION get_chain_trust_score TO authenticated;
