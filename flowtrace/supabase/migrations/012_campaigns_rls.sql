-- =============================================================================
-- Phase 5: Campaigns Row Level Security Policies
-- =============================================================================

-- Enable RLS on all campaign tables
ALTER TABLE campaigns ENABLE ROW LEVEL SECURITY;
ALTER TABLE donations ENABLE ROW LEVEL SECURITY;
ALTER TABLE donor_updates ENABLE ROW LEVEL SECURITY;

-- =============================================================================
-- CAMPAIGNS POLICIES
-- =============================================================================

-- Organization members can view their campaigns
CREATE POLICY "Users can view org campaigns"
  ON campaigns FOR SELECT
  USING (organization_id = get_current_org_id());

-- Public can view public active campaigns (for donation pages)
CREATE POLICY "Public can view public campaigns"
  ON campaigns FOR SELECT
  USING (
    is_public = TRUE
    AND status = 'active'
    AND (ends_at IS NULL OR ends_at > NOW())
  );

-- Organization members can create campaigns
CREATE POLICY "Users can create org campaigns"
  ON campaigns FOR INSERT
  WITH CHECK (organization_id = get_current_org_id());

-- Organization members can update their campaigns
CREATE POLICY "Users can update org campaigns"
  ON campaigns FOR UPDATE
  USING (organization_id = get_current_org_id())
  WITH CHECK (organization_id = get_current_org_id());

-- Organization members can delete draft campaigns only
CREATE POLICY "Users can delete draft campaigns"
  ON campaigns FOR DELETE
  USING (
    organization_id = get_current_org_id()
    AND status = 'draft'
  );

-- =============================================================================
-- DONATIONS POLICIES
-- =============================================================================

-- Organization members can view donations to their campaigns
CREATE POLICY "Users can view org donations"
  ON donations FOR SELECT
  USING (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
  );

-- Donors can view their own donations (by email)
CREATE POLICY "Donors can view own donations"
  ON donations FOR SELECT
  USING (
    donor_email IS NOT NULL
    AND donor_email = NULLIF(current_setting('app.current_user_email', true), '')
  );

-- Anyone can create donations to active public campaigns
CREATE POLICY "Anyone can create donations"
  ON donations FOR INSERT
  WITH CHECK (
    campaign_id IN (
      SELECT id FROM campaigns
      WHERE status = 'active'
        AND is_public = TRUE
        AND (ends_at IS NULL OR ends_at > NOW())
        AND (allow_overflow = TRUE OR current_amount < target_amount)
    )
  );

-- Organization members can create donations (for manual entry)
CREATE POLICY "Org members can create donations"
  ON donations FOR INSERT
  WITH CHECK (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
  );

-- Organization members can update donation status
CREATE POLICY "Users can update org donations"
  ON donations FOR UPDATE
  USING (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
  )
  WITH CHECK (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
  );

-- =============================================================================
-- DONOR UPDATES POLICIES
-- =============================================================================

-- Organization members can view updates for their campaigns
CREATE POLICY "Users can view org donor updates"
  ON donor_updates FOR SELECT
  USING (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
    OR
    donation_id IN (
      SELECT d.id FROM donations d
      JOIN campaigns c ON d.campaign_id = c.id
      WHERE c.organization_id = get_current_org_id()
    )
  );

-- Donors can view updates for their donations
CREATE POLICY "Donors can view own updates"
  ON donor_updates FOR SELECT
  USING (
    donation_id IN (
      SELECT id FROM donations
      WHERE donor_email IS NOT NULL
        AND donor_email = NULLIF(current_setting('app.current_user_email', true), '')
    )
  );

-- Organization members can create updates
CREATE POLICY "Users can create donor updates"
  ON donor_updates FOR INSERT
  WITH CHECK (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
    OR
    donation_id IN (
      SELECT d.id FROM donations d
      JOIN campaigns c ON d.campaign_id = c.id
      WHERE c.organization_id = get_current_org_id()
    )
  );

-- Organization members can update donor updates (e.g., mark as sent)
CREATE POLICY "Users can update org donor updates"
  ON donor_updates FOR UPDATE
  USING (
    campaign_id IN (
      SELECT id FROM campaigns WHERE organization_id = get_current_org_id()
    )
  );

-- =============================================================================
-- GRANT PERMISSIONS
-- =============================================================================

GRANT ALL ON campaigns TO anon, authenticated;
GRANT ALL ON donations TO anon, authenticated;
GRANT ALL ON donor_updates TO anon, authenticated;
