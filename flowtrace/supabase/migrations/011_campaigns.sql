-- =============================================================================
-- Phase 5: Donation Campaigns
-- =============================================================================
-- Creates tables for campaign management, donations, and donor updates
-- enabling end-to-end transparency from donor contribution to impact.
-- =============================================================================

-- Campaigns table
CREATE TABLE IF NOT EXISTS campaigns (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

  -- Campaign details
  name TEXT NOT NULL,
  description TEXT,
  short_description TEXT, -- For cards/previews (max ~150 chars)
  image_url TEXT, -- Hero image

  -- Funding targets
  target_amount DECIMAL(15,2) NOT NULL CHECK (target_amount > 0),
  current_amount DECIMAL(15,2) DEFAULT 0 CHECK (current_amount >= 0),
  currency CHAR(3) NOT NULL DEFAULT 'KES',

  -- Target flow node (what project/department receives funds)
  target_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,

  -- Campaign status
  status VARCHAR(20) NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft', 'active', 'paused', 'completed', 'cancelled')),

  -- Overflow handling
  allow_overflow BOOLEAN DEFAULT FALSE, -- Accept donations beyond target
  overflow_campaign_id UUID REFERENCES campaigns(id) ON DELETE SET NULL, -- Redirect overflow
  overflow_consent_message TEXT, -- Custom message for overflow consent

  -- Time bounds
  starts_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ends_at TIMESTAMPTZ,

  -- Visibility
  is_public BOOLEAN DEFAULT TRUE, -- Public donation page
  public_slug TEXT UNIQUE, -- e.g., /donate/clean-water-2024

  -- Categorization
  category TEXT, -- e.g., 'education', 'health', 'infrastructure'
  tags TEXT[], -- Array of tags for filtering

  -- Metadata
  metadata JSONB DEFAULT '{}',

  -- Audit
  created_by TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Donations table
CREATE TABLE IF NOT EXISTS donations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,

  -- Donor info (can be anonymous or registered)
  donor_email TEXT, -- Optional: for updates
  donor_name TEXT,
  donor_phone TEXT,
  is_anonymous BOOLEAN DEFAULT FALSE, -- Hide name in public displays

  -- Donation amount
  amount DECIMAL(15,2) NOT NULL CHECK (amount > 0),
  currency CHAR(3) NOT NULL DEFAULT 'KES',
  amount_usd DECIMAL(15,2), -- Converted for reporting
  exchange_rate DECIMAL(10,6),

  -- Earmarking (specific node within the campaign's scope)
  earmarked_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,
  earmark_notes TEXT, -- "For classroom supplies"

  -- Overflow consent
  allow_reallocation BOOLEAN DEFAULT TRUE, -- If campaign full, allow redirect

  -- Flow tracking
  flow_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL, -- Donor's personal node in flow
  impact_percentage DECIMAL(5,4), -- e.g., 0.0050 = 0.50%

  -- Payment info
  payment_method TEXT, -- 'mpesa', 'card', 'bank', 'paypal'
  payment_reference TEXT, -- External payment ID
  payment_status VARCHAR(20) DEFAULT 'pending'
    CHECK (payment_status IN ('pending', 'confirmed', 'failed', 'refunded')),

  -- Tracking
  transaction_id UUID REFERENCES transactions(id) ON DELETE SET NULL, -- Linked transaction

  -- Source tracking
  source TEXT, -- 'website', 'email', 'social', 'direct'
  utm_source TEXT,
  utm_medium TEXT,
  utm_campaign TEXT,

  -- Metadata
  metadata JSONB DEFAULT '{}',

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Donor updates table
CREATE TABLE IF NOT EXISTS donor_updates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  donation_id UUID NOT NULL REFERENCES donations(id) ON DELETE CASCADE,
  campaign_id UUID REFERENCES campaigns(id) ON DELETE CASCADE,

  -- Update details
  update_type VARCHAR(30) NOT NULL
    CHECK (update_type IN (
      'donation_received',
      'funds_allocated',
      'milestone_reached',
      'project_update',
      'impact_report',
      'campaign_complete',
      'thank_you',
      'custom'
    )),

  -- Content
  title TEXT NOT NULL,
  message TEXT NOT NULL,
  image_url TEXT,

  -- Related entities
  related_node_id UUID REFERENCES flow_nodes(id) ON DELETE SET NULL,
  related_transaction_id UUID REFERENCES transactions(id) ON DELETE SET NULL,

  -- Delivery
  sent_at TIMESTAMPTZ,
  delivery_method VARCHAR(20) DEFAULT 'email'
    CHECK (delivery_method IN ('email', 'sms', 'push', 'in_app')),

  -- Metadata
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- =============================================================================
-- INDEXES
-- =============================================================================

-- Campaigns indexes
CREATE INDEX IF NOT EXISTS idx_campaigns_org ON campaigns(organization_id);
CREATE INDEX IF NOT EXISTS idx_campaigns_status ON campaigns(status);
CREATE INDEX IF NOT EXISTS idx_campaigns_public_slug ON campaigns(public_slug) WHERE public_slug IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_campaigns_target_node ON campaigns(target_node_id) WHERE target_node_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_campaigns_dates ON campaigns(starts_at, ends_at);
CREATE INDEX IF NOT EXISTS idx_campaigns_category ON campaigns(category) WHERE category IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_campaigns_created ON campaigns(created_at DESC);

-- Donations indexes
CREATE INDEX IF NOT EXISTS idx_donations_campaign ON donations(campaign_id);
CREATE INDEX IF NOT EXISTS idx_donations_donor_email ON donations(donor_email) WHERE donor_email IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_donations_flow_node ON donations(flow_node_id) WHERE flow_node_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_donations_payment_status ON donations(payment_status);
CREATE INDEX IF NOT EXISTS idx_donations_created ON donations(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_donations_transaction ON donations(transaction_id) WHERE transaction_id IS NOT NULL;

-- Donor updates indexes
CREATE INDEX IF NOT EXISTS idx_donor_updates_donation ON donor_updates(donation_id);
CREATE INDEX IF NOT EXISTS idx_donor_updates_campaign ON donor_updates(campaign_id) WHERE campaign_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_donor_updates_type ON donor_updates(update_type);
CREATE INDEX IF NOT EXISTS idx_donor_updates_created ON donor_updates(created_at DESC);

-- =============================================================================
-- TRIGGERS
-- =============================================================================

-- Updated_at trigger for campaigns
CREATE TRIGGER campaigns_updated_at
  BEFORE UPDATE ON campaigns
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Updated_at trigger for donations
CREATE TRIGGER donations_updated_at
  BEFORE UPDATE ON donations
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Function to update campaign current_amount when donation is confirmed
CREATE OR REPLACE FUNCTION update_campaign_amount()
RETURNS TRIGGER AS $$
BEGIN
  IF TG_OP = 'INSERT' AND NEW.payment_status = 'confirmed' THEN
    UPDATE campaigns
    SET current_amount = current_amount + NEW.amount
    WHERE id = NEW.campaign_id;
  ELSIF TG_OP = 'UPDATE' THEN
    -- Handle status changes
    IF OLD.payment_status != 'confirmed' AND NEW.payment_status = 'confirmed' THEN
      UPDATE campaigns
      SET current_amount = current_amount + NEW.amount
      WHERE id = NEW.campaign_id;
    ELSIF OLD.payment_status = 'confirmed' AND NEW.payment_status != 'confirmed' THEN
      UPDATE campaigns
      SET current_amount = GREATEST(current_amount - OLD.amount, 0)
      WHERE id = OLD.campaign_id;
    END IF;
  ELSIF TG_OP = 'DELETE' AND OLD.payment_status = 'confirmed' THEN
    UPDATE campaigns
    SET current_amount = GREATEST(current_amount - OLD.amount, 0)
    WHERE id = OLD.campaign_id;
  END IF;

  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER donations_update_campaign_amount
  AFTER INSERT OR UPDATE OF payment_status OR DELETE ON donations
  FOR EACH ROW EXECUTE FUNCTION update_campaign_amount();

-- Function to auto-complete campaign when target reached (if overflow not allowed)
CREATE OR REPLACE FUNCTION check_campaign_completion()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.current_amount >= NEW.target_amount
     AND NEW.status = 'active'
     AND NOT NEW.allow_overflow THEN
    NEW.status = 'completed';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER campaigns_check_completion
  BEFORE UPDATE OF current_amount ON campaigns
  FOR EACH ROW EXECUTE FUNCTION check_campaign_completion();

-- Function to generate public slug from campaign name
CREATE OR REPLACE FUNCTION generate_campaign_slug(p_name TEXT, p_org_id UUID)
RETURNS TEXT AS $$
DECLARE
  v_slug TEXT;
  v_base_slug TEXT;
  v_counter INTEGER := 0;
BEGIN
  -- Create base slug from name
  v_base_slug := lower(regexp_replace(p_name, '[^a-zA-Z0-9]+', '-', 'g'));
  v_base_slug := trim(both '-' from v_base_slug);
  v_slug := v_base_slug;

  -- Check for uniqueness and add counter if needed
  WHILE EXISTS (SELECT 1 FROM campaigns WHERE public_slug = v_slug) LOOP
    v_counter := v_counter + 1;
    v_slug := v_base_slug || '-' || v_counter;
  END LOOP;

  RETURN v_slug;
END;
$$ LANGUAGE plpgsql;

-- Function to get campaign statistics
CREATE OR REPLACE FUNCTION get_campaign_stats(p_campaign_id UUID)
RETURNS TABLE(
  donation_count INTEGER,
  unique_donors INTEGER,
  total_raised DECIMAL(15,2),
  funding_percentage DECIMAL(5,2),
  avg_donation DECIMAL(15,2),
  largest_donation DECIMAL(15,2),
  smallest_donation DECIMAL(15,2),
  days_remaining INTEGER,
  days_active INTEGER
) AS $$
DECLARE
  v_campaign campaigns;
BEGIN
  SELECT * INTO v_campaign FROM campaigns WHERE id = p_campaign_id;

  RETURN QUERY
  SELECT
    COUNT(*)::INTEGER AS donation_count,
    COUNT(DISTINCT COALESCE(d.donor_email, d.id::TEXT))::INTEGER AS unique_donors,
    COALESCE(SUM(d.amount), 0)::DECIMAL(15,2) AS total_raised,
    CASE
      WHEN v_campaign.target_amount > 0
      THEN LEAST((v_campaign.current_amount / v_campaign.target_amount * 100), 100)::DECIMAL(5,2)
      ELSE 0::DECIMAL(5,2)
    END AS funding_percentage,
    COALESCE(AVG(d.amount), 0)::DECIMAL(15,2) AS avg_donation,
    COALESCE(MAX(d.amount), 0)::DECIMAL(15,2) AS largest_donation,
    COALESCE(MIN(d.amount), 0)::DECIMAL(15,2) AS smallest_donation,
    CASE
      WHEN v_campaign.ends_at IS NOT NULL AND v_campaign.ends_at > NOW()
      THEN EXTRACT(DAY FROM (v_campaign.ends_at - NOW()))::INTEGER
      WHEN v_campaign.ends_at IS NOT NULL AND v_campaign.ends_at <= NOW()
      THEN 0
      ELSE NULL
    END AS days_remaining,
    EXTRACT(DAY FROM (NOW() - v_campaign.starts_at))::INTEGER AS days_active
  FROM donations d
  WHERE d.campaign_id = p_campaign_id AND d.payment_status = 'confirmed';
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to get underfunded campaigns (for smart suggestions)
CREATE OR REPLACE FUNCTION get_underfunded_campaigns(
  p_org_id UUID,
  p_limit INTEGER DEFAULT 5
)
RETURNS SETOF campaigns AS $$
BEGIN
  RETURN QUERY
  SELECT c.*
  FROM campaigns c
  WHERE c.organization_id = p_org_id
    AND c.status = 'active'
    AND c.is_public = TRUE
    AND (c.ends_at IS NULL OR c.ends_at > NOW())
    AND (c.current_amount / NULLIF(c.target_amount, 0)) < 0.5
  ORDER BY (c.current_amount / NULLIF(c.target_amount, 0)) ASC NULLS LAST
  LIMIT p_limit;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to calculate impact percentage for a donation
CREATE OR REPLACE FUNCTION calculate_donation_impact(p_donation_id UUID)
RETURNS DECIMAL(5,4) AS $$
DECLARE
  v_donation donations;
  v_campaign campaigns;
BEGIN
  SELECT * INTO v_donation FROM donations WHERE id = p_donation_id;
  SELECT * INTO v_campaign FROM campaigns WHERE id = v_donation.campaign_id;

  IF v_campaign.current_amount > 0 THEN
    RETURN (v_donation.amount / v_campaign.current_amount)::DECIMAL(5,4);
  END IF;

  RETURN 0;
END;
$$ LANGUAGE plpgsql;

-- Function to create donor flow node
CREATE OR REPLACE FUNCTION create_donor_node(
  p_org_id UUID,
  p_donor_name TEXT,
  p_donor_email TEXT
)
RETURNS UUID AS $$
DECLARE
  v_node_id UUID;
BEGIN
  -- Check if donor node already exists for this email
  IF p_donor_email IS NOT NULL THEN
    SELECT id INTO v_node_id
    FROM flow_nodes
    WHERE organization_id = p_org_id
      AND type = 'donor'
      AND metadata->>'donor_email' = p_donor_email;
  END IF;

  IF v_node_id IS NULL THEN
    INSERT INTO flow_nodes (organization_id, name, type, metadata)
    VALUES (
      p_org_id,
      COALESCE(p_donor_name, 'Anonymous Donor'),
      'donor',
      jsonb_build_object(
        'donor_email', p_donor_email,
        'is_individual_donor', true
      )
    )
    RETURNING id INTO v_node_id;
  END IF;

  RETURN v_node_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to get organization campaign counts
CREATE OR REPLACE FUNCTION get_org_campaign_counts(p_org_id UUID)
RETURNS TABLE(
  total_campaigns INTEGER,
  active_campaigns INTEGER,
  completed_campaigns INTEGER,
  total_raised DECIMAL(15,2),
  total_donations INTEGER
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    COUNT(*)::INTEGER AS total_campaigns,
    COUNT(*) FILTER (WHERE c.status = 'active')::INTEGER AS active_campaigns,
    COUNT(*) FILTER (WHERE c.status = 'completed')::INTEGER AS completed_campaigns,
    COALESCE(SUM(c.current_amount), 0)::DECIMAL(15,2) AS total_raised,
    (SELECT COUNT(*)::INTEGER FROM donations d
     JOIN campaigns camp ON d.campaign_id = camp.id
     WHERE camp.organization_id = p_org_id AND d.payment_status = 'confirmed') AS total_donations
  FROM campaigns c
  WHERE c.organization_id = p_org_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
