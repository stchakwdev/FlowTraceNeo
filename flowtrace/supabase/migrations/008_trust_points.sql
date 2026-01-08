-- FlowTrace Database Schema
-- Migration 008: Trust Points Calculation System
-- Auto-calculates trust points based on verification type

-- Function to calculate trust points based on verification type
CREATE OR REPLACE FUNCTION get_trust_points(verification_type TEXT)
RETURNS INTEGER AS $$
BEGIN
  RETURN CASE verification_type
    WHEN 'receipt' THEN 5
    WHEN 'bank_statement' THEN 10
    WHEN 'invoice' THEN 4
    WHEN 'photo' THEN 8
    WHEN 'gps' THEN 3
    WHEN 'video' THEN 10
    WHEN 'biometric' THEN 7
    WHEN 'signature' THEN 7
    WHEN 'audit' THEN 15
    WHEN 'contract' THEN 6
    WHEN 'government_doc' THEN 8
    WHEN 'document' THEN 4
    ELSE 0
  END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Trigger function to auto-set trust points on insert/update
CREATE OR REPLACE FUNCTION set_verification_trust_points()
RETURNS TRIGGER AS $$
BEGIN
  NEW.trust_points := get_trust_points(NEW.type);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop existing trigger if exists (idempotent)
DROP TRIGGER IF EXISTS verification_trust_points_trigger ON verifications;

-- Create trigger for auto-setting trust points
CREATE TRIGGER verification_trust_points_trigger
BEFORE INSERT OR UPDATE OF type ON verifications
FOR EACH ROW EXECUTE FUNCTION set_verification_trust_points();

-- Update existing records to have correct trust points
UPDATE verifications SET trust_points = get_trust_points(type);

-- Function to calculate total trust points for a transaction
CREATE OR REPLACE FUNCTION get_transaction_trust_points(txn_id UUID)
RETURNS INTEGER AS $$
DECLARE
  total INTEGER;
BEGIN
  SELECT COALESCE(SUM(trust_points), 0) INTO total
  FROM verifications
  WHERE transaction_id = txn_id AND status = 'verified';
  RETURN total;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to calculate organization-wide verification stats
CREATE OR REPLACE FUNCTION get_org_verification_stats(org_id UUID)
RETURNS TABLE (
  total_transactions BIGINT,
  verified_transactions BIGINT,
  total_verifications BIGINT,
  total_trust_points BIGINT,
  verification_rate DECIMAL(5,2),
  avg_trust_per_transaction DECIMAL(5,2)
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    COUNT(DISTINCT t.id) AS total_transactions,
    COUNT(DISTINCT CASE WHEN t.verification_status = 'verified' THEN t.id END) AS verified_transactions,
    COUNT(v.id) AS total_verifications,
    COALESCE(SUM(v.trust_points), 0) AS total_trust_points,
    CASE WHEN COUNT(DISTINCT t.id) > 0
      THEN ROUND((COUNT(DISTINCT CASE WHEN v.id IS NOT NULL THEN t.id END)::DECIMAL / COUNT(DISTINCT t.id)::DECIMAL) * 100, 2)
      ELSE 0
    END AS verification_rate,
    CASE WHEN COUNT(DISTINCT t.id) > 0
      THEN ROUND(COALESCE(SUM(v.trust_points), 0)::DECIMAL / COUNT(DISTINCT t.id)::DECIMAL, 2)
      ELSE 0
    END AS avg_trust_per_transaction
  FROM transactions t
  LEFT JOIN verifications v ON v.transaction_id = t.id
  WHERE t.organization_id = org_id;
END;
$$ LANGUAGE plpgsql STABLE;

-- Create materialized view for trust score components (can be refreshed periodically)
CREATE MATERIALIZED VIEW IF NOT EXISTS org_trust_metrics AS
SELECT
  o.id AS organization_id,
  o.name AS organization_name,
  stats.total_transactions,
  stats.verified_transactions,
  stats.total_verifications,
  stats.total_trust_points,
  stats.verification_rate,
  stats.avg_trust_per_transaction,
  NOW() AS calculated_at
FROM organizations o
CROSS JOIN LATERAL get_org_verification_stats(o.id) stats;

-- Index on materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_org_trust_metrics_org ON org_trust_metrics(organization_id);

-- Function to refresh trust metrics for an org
CREATE OR REPLACE FUNCTION refresh_org_trust_metrics()
RETURNS VOID AS $$
BEGIN
  REFRESH MATERIALIZED VIEW CONCURRENTLY org_trust_metrics;
END;
$$ LANGUAGE plpgsql;

-- RLS policies for verification_access_log
ALTER TABLE verification_access_log ENABLE ROW LEVEL SECURITY;

-- Allow users to see their own access logs
CREATE POLICY "Users can view own access logs"
ON verification_access_log FOR SELECT
USING (accessed_by = current_setting('app.current_user_id', TRUE));

-- Allow insert for any authenticated user (system records access)
CREATE POLICY "System can insert access logs"
ON verification_access_log FOR INSERT
WITH CHECK (TRUE);

-- Dev mode policies
CREATE POLICY "Dev mode select access logs"
ON verification_access_log FOR SELECT
USING (
  EXISTS (
    SELECT 1 FROM verifications v
    JOIN transactions t ON t.id = v.transaction_id
    WHERE v.id = verification_access_log.verification_id
    AND t.organization_id = '550e8400-e29b-41d4-a716-446655440000'::uuid
  )
);

CREATE POLICY "Dev mode insert access logs"
ON verification_access_log FOR INSERT
WITH CHECK (TRUE);

-- Comments for documentation
COMMENT ON FUNCTION get_trust_points IS 'Returns trust points for a verification type';
COMMENT ON FUNCTION set_verification_trust_points IS 'Trigger function to auto-set trust points';
COMMENT ON FUNCTION get_transaction_trust_points IS 'Calculate total trust points for a transaction';
COMMENT ON FUNCTION get_org_verification_stats IS 'Get comprehensive verification statistics for an org';
COMMENT ON MATERIALIZED VIEW org_trust_metrics IS 'Cached trust metrics per organization';
