-- FlowTrace Comprehensive Seed Data
-- Migration 013: Realistic test data for development & demo
-- Run after all schema migrations (001-012)

-- =====================================================
-- 1. ORGANIZATION (upsert demo org)
-- =====================================================
INSERT INTO organizations (id, name, type, country, currency, trust_score, verified, public_slug)
VALUES (
  '550e8400-e29b-41d4-a716-446655440000',
  'Demo NGO Kenya',
  'NGO',
  'KE',
  'KES',
  87,
  TRUE,
  'demo-ngo'
) ON CONFLICT (id) DO UPDATE SET
  trust_score = EXCLUDED.trust_score,
  verified = EXCLUDED.verified;

-- Partner organizations for chain links
INSERT INTO organizations (id, name, type, country, currency, trust_score, verified, public_slug) VALUES
  ('660e8400-e29b-41d4-a716-446655440001', 'WaterAid East Africa', 'NGO', 'KE', 'KES', 92, TRUE, 'wateraid-ea'),
  ('660e8400-e29b-41d4-a716-446655440002', 'Amref Health Africa', 'NGO', 'KE', 'KES', 95, TRUE, 'amref'),
  ('660e8400-e29b-41d4-a716-446655440003', 'TechBridge Solutions', 'SME', 'KE', 'KES', 78, FALSE, 'techbridge'),
  ('660e8400-e29b-41d4-a716-446655440004', 'Kilifi County Government', 'GOVERNMENT', 'KE', 'KES', 65, TRUE, 'kilifi-gov')
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 2. FLOW NODES (upsert + add more)
-- =====================================================
INSERT INTO flow_nodes (id, organization_id, name, type, balance) VALUES
  -- Accounts
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', '550e8400-e29b-41d4-a716-446655440000', 'General Fund', 'account', 5000000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', '550e8400-e29b-41d4-a716-446655440000', 'Operations Budget', 'account', 1500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', '550e8400-e29b-41d4-a716-446655440000', 'Project Fund', 'account', 2500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a19', '550e8400-e29b-41d4-a716-446655440000', 'Payroll', 'account', 0),
  -- Donors
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', '550e8400-e29b-41d4-a716-446655440000', 'USAID Grant', 'donor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a20', '550e8400-e29b-41d4-a716-446655440000', 'Gates Foundation', 'donor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a30', '550e8400-e29b-41d4-a716-446655440000', 'DFID UK Aid', 'donor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a31', '550e8400-e29b-41d4-a716-446655440000', 'World Bank IDA', 'donor', 0),
  -- Departments
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', '550e8400-e29b-41d4-a716-446655440000', 'Training Department', 'department', 250000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', '550e8400-e29b-41d4-a716-446655440000', 'Field Operations', 'department', 500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a21', '550e8400-e29b-41d4-a716-446655440000', 'Admin', 'department', 100000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a32', '550e8400-e29b-41d4-a716-446655440000', 'M&E Unit', 'department', 180000),
  -- Vendors
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17', '550e8400-e29b-41d4-a716-446655440000', 'Staples Kenya', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18', '550e8400-e29b-41d4-a716-446655440000', 'AutoXpress Ltd', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22', '550e8400-e29b-41d4-a716-446655440000', 'Training Co Ltd', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a33', '550e8400-e29b-41d4-a716-446655440000', 'Safaricom M-Pesa', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a34', '550e8400-e29b-41d4-a716-446655440000', 'Kenya Power', 'vendor', 0),
  -- Projects
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a35', '550e8400-e29b-41d4-a716-446655440000', 'Clean Water Turkana', 'project', 1200000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a36', '550e8400-e29b-41d4-a716-446655440000', 'Maternal Health Kilifi', 'project', 800000),
  -- Beneficiaries
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a23', '550e8400-e29b-41d4-a716-446655440000', 'Communities', 'beneficiary', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a37', '550e8400-e29b-41d4-a716-446655440000', 'Turkana Schools', 'beneficiary', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a38', '550e8400-e29b-41d4-a716-446655440000', 'Kilifi Health Centers', 'beneficiary', 0)
ON CONFLICT (id) DO UPDATE SET
  balance = EXCLUDED.balance;


-- =====================================================
-- 3. TRANSACTIONS (recent dates - Jan/Feb 2026)
-- =====================================================
-- Delete old seed transactions first
DELETE FROM transactions WHERE organization_id = '550e8400-e29b-41d4-a716-446655440000';

INSERT INTO transactions (
  id, organization_id, from_node_id, to_node_id, amount, currency,
  description, category, reference, notes, verification_status, verification_count, transaction_date
) VALUES
  -- Verified transactions
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    8500000, 'KES',
    'USAID Q1 2026 Grant Disbursement', 'Funding', 'USAID-2026-Q1-001',
    'Quarterly grant for community health programs', 'verified', 3, '2026-01-05'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b02',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a20', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11',
    3200000, 'KES',
    'Gates Foundation Annual Grant', 'Funding', 'GATES-2026-001',
    'Annual support for maternal health initiatives', 'verified', 2, '2026-01-08'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b03',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a30', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14',
    4000000, 'KES',
    'DFID Water & Sanitation Fund', 'Funding', 'DFID-WS-2026-01',
    'Dedicated funding for Turkana water project', 'verified', 2, '2026-01-10'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b04',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13',
    2000000, 'KES',
    'Operations Budget Allocation - January', 'Budget', 'INT-OPS-2026-01',
    'Monthly operations budget transfer', 'verified', 1, '2026-01-12'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b05',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16',
    1800000, 'KES',
    'Field Operations Q1 Budget', 'Budget', 'INT-FIELD-2026-Q1',
    'Quarterly field operations allocation', 'verified', 2, '2026-01-12'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b06',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a19',
    1450000, 'KES',
    'Staff Salaries - January 2026', 'Salaries', 'PAY-2026-01',
    'Monthly payroll for 23 staff members', 'verified', 1, '2026-01-31'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b07',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17',
    28500, 'KES',
    'Office Supplies - Nairobi HQ', 'Operations', 'PO-2026-0042',
    'Monthly office supplies order', 'verified', 2, '2026-01-15'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b08',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18',
    67000, 'KES',
    'Vehicle Maintenance - 3 Field Vehicles', 'Transport', 'PO-2026-0051',
    'Quarterly servicing for Land Cruisers', 'verified', 1, '2026-01-20'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b09',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a34',
    42000, 'KES',
    'Electricity Bill - January', 'Utilities', 'UTIL-2026-01',
    'Nairobi office + Mombasa sub-office', 'verified', 1, '2026-01-25'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b10',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a35',
    2500000, 'KES',
    'Clean Water Turkana - Borehole Phase 1', 'Programs', 'PROJ-WATER-001',
    'Drilling and installation of 5 boreholes in Turkana South', 'verified', 3, '2026-01-18'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b11',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a23',
    750000, 'KES',
    'Community Health Worker Stipends', 'Programs', 'CHW-2026-01',
    'Monthly stipends for 50 CHWs across 3 counties', 'verified', 2, '2026-01-28'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b12',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22',
    380000, 'KES',
    'Financial Literacy Workshop - Mombasa', 'Training', 'TRN-2026-003',
    '3-day workshop for 45 community leaders', 'verified', 1, '2026-01-22'
  ),
  -- Pending transactions
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b13',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a38',
    1200000, 'KES',
    'Maternal Health Equipment - Kilifi', 'Programs', 'PROJ-MH-002',
    'Medical equipment for 8 health centers', 'pending', 0, '2026-02-01'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b14',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a33',
    15000, 'KES',
    'M-Pesa Float Top-up', 'Operations', 'MPESA-2026-02',
    'Mobile money float for field disbursements', 'pending', 0, '2026-02-03'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b15',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a31', 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14',
    6000000, 'KES',
    'World Bank IDA - Education Grant', 'Funding', 'WB-IDA-2026-01',
    'First tranche of education support grant', 'pending', 0, '2026-02-04'
  ),
  -- Flagged transaction
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b16',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', NULL,
    500000, 'KES',
    'Unreconciled Cash Withdrawal', 'Unclassified', NULL,
    'Cash withdrawal without matching receipt - flagged for review', 'flagged', 0, '2026-01-29'
  );


-- =====================================================
-- 4. VERIFICATIONS
-- =====================================================
DELETE FROM verifications WHERE transaction_id IN (
  SELECT id FROM transactions WHERE organization_id = '550e8400-e29b-41d4-a716-446655440000'
);

INSERT INTO verifications (id, transaction_id, type, status, trust_points, description, created_at) VALUES
  -- USAID grant (3 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c01', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01', 'bank_statement', 'verified', 10, 'Bank statement showing USAID transfer', '2026-01-05 09:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c02', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01', 'document', 'verified', 4, 'Grant agreement letter', '2026-01-05 09:15:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c03', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01', 'receipt', 'verified', 5, 'Acknowledgment receipt', '2026-01-06 10:00:00'),
  -- Gates Foundation (2 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c04', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b02', 'bank_statement', 'verified', 10, 'Wire transfer confirmation', '2026-01-08 11:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c05', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b02', 'contract', 'verified', 6, 'Signed grant contract', '2026-01-08 11:30:00'),
  -- DFID fund (2 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c06', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b03', 'bank_statement', 'verified', 10, 'DFID transfer confirmation', '2026-01-10 10:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c07', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b03', 'government_doc', 'verified', 8, 'UK Aid official disbursement letter', '2026-01-10 10:30:00'),
  -- Office supplies (2 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c08', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b07', 'receipt', 'verified', 5, 'Staples Kenya receipt', '2026-01-15 14:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c09', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b07', 'photo', 'verified', 8, 'Photo of delivered supplies', '2026-01-15 15:00:00'),
  -- Vehicle maintenance (1 verification)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c10', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b08', 'invoice', 'verified', 4, 'AutoXpress service invoice', '2026-01-20 16:00:00'),
  -- Borehole project (3 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c11', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b10', 'photo', 'verified', 8, 'Drilling site progress photos', '2026-01-19 08:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c12', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b10', 'gps', 'verified', 3, 'GPS coordinates of borehole sites', '2026-01-19 08:15:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b10', 'contract', 'verified', 6, 'Drilling contractor agreement', '2026-01-18 09:00:00'),
  -- Community health worker stipends (2 verifications)
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c14', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b11', 'document', 'verified', 4, 'CHW payment register', '2026-01-28 12:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c15', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b11', 'signature', 'verified', 7, 'Signed attendance sheets', '2026-01-28 12:30:00'),
  -- Salary verification
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c16', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b06', 'bank_statement', 'verified', 10, 'Payroll bank transfer records', '2026-01-31 17:00:00'),
  -- Training workshop
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c17', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b12', 'photo', 'verified', 8, 'Workshop group photo with participants', '2026-01-22 16:00:00')
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 5. TRUST SCORE HISTORY (updated to 2025-2026)
-- =====================================================
DELETE FROM trust_score_history WHERE organization_id = '550e8400-e29b-41d4-a716-446655440000';

INSERT INTO trust_score_history (organization_id, score, breakdown, verification_rate, documentation_rate, consistency_score, calculated_at) VALUES
  ('550e8400-e29b-41d4-a716-446655440000', 62, '{"verification": 58, "documentation": 60, "consistency": 70}', 58.00, 60.00, 70.00, '2025-07-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 68, '{"verification": 65, "documentation": 66, "consistency": 74}', 65.00, 66.00, 74.00, '2025-08-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 72, '{"verification": 70, "documentation": 68, "consistency": 78}', 70.00, 68.00, 78.00, '2025-09-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 75, '{"verification": 74, "documentation": 72, "consistency": 80}', 74.00, 72.00, 80.00, '2025-10-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 78, '{"verification": 78, "documentation": 75, "consistency": 81}', 78.00, 75.00, 81.00, '2025-11-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 82, '{"verification": 82, "documentation": 80, "consistency": 84}', 82.00, 80.00, 84.00, '2025-12-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 85, '{"verification": 88, "documentation": 82, "consistency": 85}', 88.00, 82.00, 85.00, '2026-01-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 87, '{"verification": 92, "documentation": 85, "consistency": 82}', 92.00, 85.00, 82.00, '2026-02-01');


-- =====================================================
-- 6. ACCESS GROUPS
-- =====================================================
INSERT INTO access_groups (id, organization_id, name, slug, description, icon, color, permissions, node_scope, is_default, is_public, priority) VALUES
  (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d01',
    '550e8400-e29b-41d4-a716-446655440000',
    'Board of Directors', 'board',
    'Full oversight access for board members',
    'shield-check', '#10b981',
    '{"can_view_nodes": true, "can_view_amounts": true, "can_view_proofs": true, "can_view_executor": true, "can_view_full_proofs": true, "can_edit_nodes": true, "can_edit_transactions": true, "can_manage_members": true, "can_manage_groups": true}',
    'all', TRUE, FALSE, 10
  ),
  (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d02',
    '550e8400-e29b-41d4-a716-446655440000',
    'Staff', 'staff',
    'Internal staff with operational access',
    'users', '#3b82f6',
    '{"can_view_nodes": true, "can_view_amounts": true, "can_view_proofs": true, "can_view_executor": true, "can_view_full_proofs": false, "can_edit_nodes": true, "can_edit_transactions": true, "can_manage_members": false, "can_manage_groups": false}',
    'all', TRUE, FALSE, 20
  ),
  (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d03',
    '550e8400-e29b-41d4-a716-446655440000',
    'Donors', 'donors',
    'Donor organizations and individuals with downstream visibility',
    'heart', '#ec4899',
    '{"can_view_nodes": true, "can_view_amounts": true, "can_view_proofs": true, "can_view_executor": false, "can_view_full_proofs": false, "can_edit_nodes": false, "can_edit_transactions": false, "can_manage_members": false, "can_manage_groups": false}',
    'downstream', FALSE, FALSE, 30
  ),
  (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d04',
    '550e8400-e29b-41d4-a716-446655440000',
    'Partners', 'partners',
    'Partner organizations with assigned node access',
    'handshake', '#f59e0b',
    '{"can_view_nodes": true, "can_view_amounts": true, "can_view_proofs": false, "can_view_executor": false, "can_view_full_proofs": false, "can_edit_nodes": false, "can_edit_transactions": false, "can_manage_members": false, "can_manage_groups": false}',
    'assigned', FALSE, FALSE, 40
  ),
  (
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d05',
    '550e8400-e29b-41d4-a716-446655440000',
    'Public', 'public',
    'Public view for transparency reporting',
    'globe', '#8b5cf6',
    '{"can_view_nodes": true, "can_view_amounts": false, "can_view_proofs": false, "can_view_executor": false, "can_view_full_proofs": false, "can_edit_nodes": false, "can_edit_transactions": false, "can_manage_members": false, "can_manage_groups": false}',
    'all', FALSE, TRUE, 100
  )
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 7. CAMPAIGNS
-- =====================================================
INSERT INTO campaigns (id, organization_id, name, description, short_description, target_amount, current_amount, currency, target_node_id, status, starts_at, ends_at, is_public, public_slug, category) VALUES
  (
    'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01',
    '550e8400-e29b-41d4-a716-446655440000',
    'Clean Water for Turkana',
    'Help us bring clean drinking water to 15,000 people in Turkana County. Funds will be used to drill boreholes, install solar-powered pumps, and train local maintenance teams.',
    'Bringing clean water to 15,000 people in Turkana',
    5000000, 3200000, 'KES',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a35',
    'active',
    '2025-12-01', '2026-06-30',
    TRUE, 'clean-water-turkana', 'water'
  ),
  (
    'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02',
    '550e8400-e29b-41d4-a716-446655440000',
    'Maternal Health Kilifi',
    'Equip 8 health centers in Kilifi County with essential maternal health equipment. Reduce maternal mortality by improving access to quality care.',
    'Equipping 8 health centers for better maternal care',
    3500000, 1800000, 'KES',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a36',
    'active',
    '2026-01-15', '2026-09-30',
    TRUE, 'maternal-health-kilifi', 'health'
  ),
  (
    'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e03',
    '550e8400-e29b-41d4-a716-446655440000',
    'Digital Literacy for Youth',
    'Provide 200 tablets and internet connectivity to schools in underserved communities. Includes 6-month training program for teachers.',
    'Bringing digital education to 200 students',
    2000000, 2000000, 'KES',
    NULL,
    'completed',
    '2025-06-01', '2025-12-31',
    TRUE, 'digital-literacy', 'education'
  ),
  (
    'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e04',
    '550e8400-e29b-41d4-a716-446655440000',
    'Emergency Drought Relief',
    'Emergency food and water supplies for drought-affected communities in Northern Kenya.',
    'Emergency relief for drought-affected families',
    8000000, 0, 'KES',
    NULL,
    'draft',
    '2026-03-01', NULL,
    FALSE, NULL, 'emergency'
  )
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 8. DONATIONS (for active campaigns)
-- =====================================================
INSERT INTO donations (id, campaign_id, donor_name, donor_email, is_anonymous, amount, currency, payment_method, payment_status, source, created_at) VALUES
  -- Clean Water Turkana donations
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f01', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Sarah Kimani', 'sarah.k@gmail.com', FALSE, 50000, 'KES', 'mpesa', 'confirmed', 'website', '2025-12-15 10:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f02', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'James Ochieng', 'j.ochieng@yahoo.com', FALSE, 100000, 'KES', 'mpesa', 'confirmed', 'social', '2025-12-20 14:30:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f03', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', NULL, NULL, TRUE, 500000, 'KES', 'bank', 'confirmed', 'direct', '2026-01-05 09:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f04', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Dr. Wanjiku Muthoni', 'wanjiku.m@hospitals.ke', FALSE, 250000, 'KES', 'card', 'confirmed', 'email', '2026-01-12 16:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f05', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Peter Njoroge', 'peter.n@techcorp.co.ke', FALSE, 75000, 'KES', 'mpesa', 'confirmed', 'website', '2026-01-18 11:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f06', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Mary Akinyi', 'mary.a@gmail.com', FALSE, 25000, 'KES', 'mpesa', 'confirmed', 'social', '2026-01-25 08:30:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f07', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Rotary Club Nairobi', 'rotary.nbi@rotary.org', FALSE, 200000, 'KES', 'bank', 'confirmed', 'direct', '2026-02-01 10:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f08', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e01', 'Samuel Tchakwera', 'stchakwera@gmail.com', FALSE, 10000, 'KES', 'mpesa', 'pending', 'website', '2026-02-05 12:00:00'),
  -- Maternal Health Kilifi donations
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f09', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02', 'Dr. Fatuma Hassan', 'fatuma.h@amref.org', FALSE, 300000, 'KES', 'bank', 'confirmed', 'email', '2026-01-20 09:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f10', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02', NULL, NULL, TRUE, 1000000, 'KES', 'bank', 'confirmed', 'direct', '2026-01-22 14:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f11', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02', 'Lions Club Mombasa', 'lions.msa@lionsclubs.org', FALSE, 150000, 'KES', 'bank', 'confirmed', 'direct', '2026-01-28 10:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f12', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02', 'Grace Mwangi', 'grace.m@gmail.com', FALSE, 20000, 'KES', 'mpesa', 'confirmed', 'social', '2026-02-02 15:00:00'),
  ('f0eebc99-9c0b-4ef8-bb6d-6bb9bd380f13', 'e0eebc99-9c0b-4ef8-bb6d-6bb9bd380e02', 'John Kiprotich', 'john.k@outlook.com', FALSE, 5000, 'KES', 'mpesa', 'pending', 'website', '2026-02-04 18:00:00')
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 9. ORGANIZATION CHAINS
-- =====================================================
INSERT INTO organization_chains (id, source_org_id, target_org_id, status, granted_group_id, request_message, requested_by, requested_at, approved_by, approved_at) VALUES
  (
    'g0eebc99-9c0b-4ef8-bb6d-6bb9bd380g01',
    '550e8400-e29b-41d4-a716-446655440000',
    '660e8400-e29b-41d4-a716-446655440001',
    'active',
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d04',
    'We would like to link our water programs for end-to-end fund tracking.',
    'system', '2025-11-15', 'system', '2025-11-18'
  ),
  (
    'g0eebc99-9c0b-4ef8-bb6d-6bb9bd380g02',
    '550e8400-e29b-41d4-a716-446655440000',
    '660e8400-e29b-41d4-a716-446655440002',
    'active',
    'd0eebc99-9c0b-4ef8-bb6d-6bb9bd380d03',
    'Partnership for maternal health data sharing across our organizations.',
    'system', '2025-12-01', 'system', '2025-12-05'
  ),
  (
    'g0eebc99-9c0b-4ef8-bb6d-6bb9bd380g03',
    '660e8400-e29b-41d4-a716-446655440003',
    '550e8400-e29b-41d4-a716-446655440000',
    'pending',
    NULL,
    'We are developing a technology solution for your field operations and would like visibility into relevant fund flows.',
    'system', '2026-01-20', NULL, NULL
  ),
  (
    'g0eebc99-9c0b-4ef8-bb6d-6bb9bd380g04',
    '660e8400-e29b-41d4-a716-446655440004',
    '550e8400-e29b-41d4-a716-446655440000',
    'pending',
    NULL,
    'Kilifi County Government requests transparency chain for the maternal health program funded through your organization.',
    'system', '2026-02-01', NULL, NULL
  )
ON CONFLICT (id) DO NOTHING;


-- =====================================================
-- 10. PUBLIC EMBED CONFIG
-- =====================================================
INSERT INTO public_embed_config (id, organization_id, display_mode, show_amounts, amount_format, show_node_names, show_trust_score, show_transaction_count, show_last_updated, visible_node_types, theme, allow_embed, embed_token) VALUES
  (
    'h0eebc99-9c0b-4ef8-bb6d-6bb9bd380h01',
    '550e8400-e29b-41d4-a716-446655440000',
    'simplified',
    TRUE, 'ranges',
    TRUE, TRUE, TRUE, TRUE,
    ARRAY['donor', 'department', 'project', 'beneficiary'],
    '{"primary_color": "#0d9488", "background": "#f8fafc", "border_radius": "0.5rem", "show_logo": true}',
    TRUE,
    encode(gen_random_bytes(16), 'hex')
  )
ON CONFLICT (id) DO NOTHING;
