-- FlowTrace Database Schema
-- Migration 001: Initial Schema

-- Organizations table
CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  type TEXT CHECK (type IN ('NGO', 'SME', 'SOCIAL_ENTERPRISE', 'GOVERNMENT')),
  country CHAR(2) NOT NULL DEFAULT 'KE',
  currency CHAR(3) NOT NULL DEFAULT 'KES',
  trust_score INTEGER DEFAULT 50 CHECK (trust_score >= 0 AND trust_score <= 100),
  verified BOOLEAN DEFAULT FALSE,
  public_slug TEXT UNIQUE,
  logo_url TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User profiles (linked to WorkOS user IDs)
CREATE TABLE profiles (
  id TEXT PRIMARY KEY,  -- WorkOS user ID (e.g., "user_01HXXXXXX")
  organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
  email TEXT NOT NULL,
  full_name TEXT,
  phone TEXT,
  role TEXT DEFAULT 'member' CHECK (role IN ('admin', 'manager', 'member', 'viewer')),
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Flow nodes (accounts, departments, vendors, etc.)
CREATE TABLE flow_nodes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('donor', 'department', 'vendor', 'beneficiary', 'account', 'project')),
  balance DECIMAL(15,2) DEFAULT 0,
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Transactions
CREATE TABLE transactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  from_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,
  to_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,
  amount DECIMAL(15,2) NOT NULL CHECK (amount > 0),
  currency CHAR(3) NOT NULL DEFAULT 'KES',
  exchange_rate DECIMAL(10,6),
  amount_usd DECIMAL(15,2),
  description TEXT NOT NULL,
  category TEXT,
  reference TEXT,
  notes TEXT,
  verification_status TEXT DEFAULT 'pending'
    CHECK (verification_status IN ('pending', 'verified', 'flagged', 'rejected')),
  verification_count INTEGER DEFAULT 0,
  transaction_date DATE NOT NULL DEFAULT CURRENT_DATE,
  created_by TEXT REFERENCES profiles(id),
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Verifications (proof attached to transactions)
CREATE TABLE verifications (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  type TEXT NOT NULL CHECK (type IN ('receipt', 'photo', 'gps', 'signature', 'biometric', 'bank_statement', 'document')),
  file_url TEXT,
  ocr_data JSONB,
  metadata JSONB DEFAULT '{}',
  status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'verified', 'rejected')),
  verified_by TEXT REFERENCES profiles(id),
  verified_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Trust score history for tracking over time
CREATE TABLE trust_score_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  score INTEGER NOT NULL CHECK (score >= 0 AND score <= 100),
  breakdown JSONB NOT NULL DEFAULT '{}',
  verification_rate DECIMAL(5,2),
  documentation_rate DECIMAL(5,2),
  consistency_score DECIMAL(5,2),
  calculated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_transactions_org ON transactions(organization_id);
CREATE INDEX idx_transactions_date ON transactions(transaction_date DESC);
CREATE INDEX idx_transactions_status ON transactions(verification_status);
CREATE INDEX idx_flow_nodes_org ON flow_nodes(organization_id);
CREATE INDEX idx_flow_nodes_type ON flow_nodes(type);
CREATE INDEX idx_verifications_txn ON verifications(transaction_id);
CREATE INDEX idx_profiles_org ON profiles(organization_id);
CREATE INDEX idx_trust_history_org ON trust_score_history(organization_id);
CREATE INDEX idx_trust_history_date ON trust_score_history(calculated_at DESC);

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to tables with updated_at
CREATE TRIGGER organizations_updated_at
  BEFORE UPDATE ON organizations
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER transactions_updated_at
  BEFORE UPDATE ON transactions
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Function to update verification count on transaction
CREATE OR REPLACE FUNCTION update_verification_count()
RETURNS TRIGGER AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    UPDATE transactions
    SET verification_count = verification_count + 1
    WHERE id = NEW.transaction_id;
  ELSIF TG_OP = 'DELETE' THEN
    UPDATE transactions
    SET verification_count = GREATEST(verification_count - 1, 0)
    WHERE id = OLD.transaction_id;
  END IF;
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER verifications_count_trigger
  AFTER INSERT OR DELETE ON verifications
  FOR EACH ROW EXECUTE FUNCTION update_verification_count();

-- Function to auto-verify transaction when enough verifications
CREATE OR REPLACE FUNCTION auto_verify_transaction()
RETURNS TRIGGER AS $$
BEGIN
  -- Auto-verify if verification count reaches 2 and status is pending
  IF NEW.verification_count >= 2 THEN
    UPDATE transactions
    SET verification_status = 'verified'
    WHERE id = NEW.id AND verification_status = 'pending';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER transactions_auto_verify
  AFTER UPDATE OF verification_count ON transactions
  FOR EACH ROW EXECUTE FUNCTION auto_verify_transaction();
