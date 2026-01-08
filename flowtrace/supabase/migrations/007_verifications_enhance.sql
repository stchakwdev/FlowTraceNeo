-- FlowTrace Database Schema
-- Migration 007: Enhance Verifications for Proof System
-- Adds trust points, visibility levels, GPS, and more verification types

-- Add new columns to existing verifications table
ALTER TABLE verifications
ADD COLUMN IF NOT EXISTS trust_points INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS visibility_level VARCHAR(20) DEFAULT 'full',
ADD COLUMN IF NOT EXISTS thumbnail_url TEXT,
ADD COLUMN IF NOT EXISTS executor_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS gps_latitude DECIMAL(10, 8),
ADD COLUMN IF NOT EXISTS gps_longitude DECIMAL(11, 8),
ADD COLUMN IF NOT EXISTS gps_accuracy DECIMAL(10, 2),
ADD COLUMN IF NOT EXISTS location_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS description TEXT,
ADD COLUMN IF NOT EXISTS created_by TEXT REFERENCES profiles(id);

-- Add visibility level constraint
ALTER TABLE verifications
ADD CONSTRAINT verifications_visibility_check
CHECK (visibility_level IN ('full', 'thumbnail', 'summary', 'hidden'));

-- Drop old type constraint and add expanded one
ALTER TABLE verifications
DROP CONSTRAINT IF EXISTS verifications_type_check;

ALTER TABLE verifications
ADD CONSTRAINT verifications_type_check
CHECK (type IN (
  'receipt', 'photo', 'gps', 'signature', 'biometric',
  'bank_statement', 'document', 'invoice', 'video',
  'audit', 'contract', 'government_doc'
));

-- Create verification access log for audit trail
CREATE TABLE IF NOT EXISTS verification_access_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  verification_id UUID NOT NULL REFERENCES verifications(id) ON DELETE CASCADE,
  accessed_by TEXT NOT NULL REFERENCES profiles(id),
  access_type VARCHAR(20) NOT NULL CHECK (access_type IN ('view', 'download')),
  access_level VARCHAR(20) NOT NULL CHECK (access_level IN ('full', 'thumbnail', 'summary')),
  ip_address INET,
  user_agent TEXT,
  accessed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for new columns and access log
CREATE INDEX IF NOT EXISTS idx_verifications_visibility ON verifications(visibility_level);
CREATE INDEX IF NOT EXISTS idx_verifications_created_by ON verifications(created_by);
CREATE INDEX IF NOT EXISTS idx_verifications_trust_points ON verifications(trust_points);
CREATE INDEX IF NOT EXISTS idx_verification_access_log_verification ON verification_access_log(verification_id);
CREATE INDEX IF NOT EXISTS idx_verification_access_log_accessed_by ON verification_access_log(accessed_by);
CREATE INDEX IF NOT EXISTS idx_verification_access_log_date ON verification_access_log(accessed_at DESC);

-- Comments for documentation
COMMENT ON COLUMN verifications.trust_points IS 'Trust score contribution based on verification type';
COMMENT ON COLUMN verifications.visibility_level IS 'Who can see this verification: full, thumbnail, summary, hidden';
COMMENT ON COLUMN verifications.thumbnail_url IS 'Blurred/low-res version for limited access';
COMMENT ON COLUMN verifications.executor_name IS 'Name of person who executed the transaction';
COMMENT ON COLUMN verifications.gps_latitude IS 'GPS latitude of verification location';
COMMENT ON COLUMN verifications.gps_longitude IS 'GPS longitude of verification location';
COMMENT ON COLUMN verifications.gps_accuracy IS 'GPS accuracy in meters';
COMMENT ON COLUMN verifications.location_name IS 'Human-readable location name';
COMMENT ON COLUMN verifications.description IS 'User-provided description of the verification';
COMMENT ON COLUMN verifications.created_by IS 'User who created this verification';
