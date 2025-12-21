-- FlowTrace Row Level Security Policies
-- Migration 002: RLS Policies

-- Enable RLS on all tables
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE transactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE verifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE trust_score_history ENABLE ROW LEVEL SECURITY;

-- Helper functions for getting current context
-- These can be set via Supabase client headers or RPC calls

CREATE OR REPLACE FUNCTION get_current_org_id()
RETURNS UUID AS $$
BEGIN
  RETURN NULLIF(current_setting('app.current_organization_id', true), '')::UUID;
EXCEPTION
  WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER STABLE;

CREATE OR REPLACE FUNCTION get_current_user_id()
RETURNS TEXT AS $$
BEGIN
  RETURN NULLIF(current_setting('app.current_user_id', true), '');
EXCEPTION
  WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER STABLE;

-- Organizations policies
CREATE POLICY "Users can view own organization"
  ON organizations FOR SELECT
  USING (id = get_current_org_id());

CREATE POLICY "Public can view verified organizations"
  ON organizations FOR SELECT
  USING (verified = TRUE AND public_slug IS NOT NULL);

CREATE POLICY "Admins can update own organization"
  ON organizations FOR UPDATE
  USING (id = get_current_org_id())
  WITH CHECK (id = get_current_org_id());

-- Profiles policies
CREATE POLICY "Users can view profiles in own organization"
  ON profiles FOR SELECT
  USING (organization_id = get_current_org_id());

CREATE POLICY "Users can view own profile"
  ON profiles FOR SELECT
  USING (id = get_current_user_id());

CREATE POLICY "Users can update own profile"
  ON profiles FOR UPDATE
  USING (id = get_current_user_id())
  WITH CHECK (id = get_current_user_id());

CREATE POLICY "Users can insert own profile"
  ON profiles FOR INSERT
  WITH CHECK (id = get_current_user_id() OR get_current_user_id() IS NULL);

-- Flow nodes policies
CREATE POLICY "Users can view org flow nodes"
  ON flow_nodes FOR SELECT
  USING (organization_id = get_current_org_id());

CREATE POLICY "Users can create org flow nodes"
  ON flow_nodes FOR INSERT
  WITH CHECK (organization_id = get_current_org_id());

CREATE POLICY "Users can update org flow nodes"
  ON flow_nodes FOR UPDATE
  USING (organization_id = get_current_org_id())
  WITH CHECK (organization_id = get_current_org_id());

CREATE POLICY "Users can delete org flow nodes"
  ON flow_nodes FOR DELETE
  USING (organization_id = get_current_org_id());

-- Transactions policies
CREATE POLICY "Users can view org transactions"
  ON transactions FOR SELECT
  USING (organization_id = get_current_org_id());

CREATE POLICY "Users can create org transactions"
  ON transactions FOR INSERT
  WITH CHECK (organization_id = get_current_org_id());

CREATE POLICY "Users can update org transactions"
  ON transactions FOR UPDATE
  USING (organization_id = get_current_org_id())
  WITH CHECK (organization_id = get_current_org_id());

-- Verifications policies (access based on transaction's organization)
CREATE POLICY "Users can view org verifications"
  ON verifications FOR SELECT
  USING (
    transaction_id IN (
      SELECT id FROM transactions WHERE organization_id = get_current_org_id()
    )
  );

CREATE POLICY "Users can create org verifications"
  ON verifications FOR INSERT
  WITH CHECK (
    transaction_id IN (
      SELECT id FROM transactions WHERE organization_id = get_current_org_id()
    )
  );

CREATE POLICY "Users can update org verifications"
  ON verifications FOR UPDATE
  USING (
    transaction_id IN (
      SELECT id FROM transactions WHERE organization_id = get_current_org_id()
    )
  );

CREATE POLICY "Users can delete org verifications"
  ON verifications FOR DELETE
  USING (
    transaction_id IN (
      SELECT id FROM transactions WHERE organization_id = get_current_org_id()
    )
  );

-- Trust score history policies
CREATE POLICY "Users can view org trust history"
  ON trust_score_history FOR SELECT
  USING (organization_id = get_current_org_id());

CREATE POLICY "Public can view verified org trust history"
  ON trust_score_history FOR SELECT
  USING (
    organization_id IN (
      SELECT id FROM organizations WHERE verified = TRUE AND public_slug IS NOT NULL
    )
  );

-- System can insert trust history (no user restriction for scheduled jobs)
CREATE POLICY "System can insert trust history"
  ON trust_score_history FOR INSERT
  WITH CHECK (TRUE);

-- Grant usage to anon and authenticated roles
GRANT USAGE ON SCHEMA public TO anon, authenticated;
GRANT ALL ON ALL TABLES IN SCHEMA public TO anon, authenticated;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO anon, authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO anon, authenticated;
