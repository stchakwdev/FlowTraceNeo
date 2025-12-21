-- FlowTrace Development Seed Data
-- Migration 004: Seed Data for Development
-- This data is used when DEV_BYPASS_AUTH is active

-- Demo organization (fixed UUID for dev mode)
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
) ON CONFLICT (id) DO NOTHING;

-- Demo flow nodes
INSERT INTO flow_nodes (id, organization_id, name, type, balance) VALUES
  -- Accounts
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', '550e8400-e29b-41d4-a716-446655440000', 'General Fund', 'account', 5000000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', '550e8400-e29b-41d4-a716-446655440000', 'Operations Budget', 'account', 1500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', '550e8400-e29b-41d4-a716-446655440000', 'Project Fund', 'account', 2500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a19', '550e8400-e29b-41d4-a716-446655440000', 'Payroll', 'account', 0),
  -- Donors
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', '550e8400-e29b-41d4-a716-446655440000', 'USAID Grant', 'donor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a20', '550e8400-e29b-41d4-a716-446655440000', 'Gates Foundation', 'donor', 0),
  -- Departments
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', '550e8400-e29b-41d4-a716-446655440000', 'Training Department', 'department', 250000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', '550e8400-e29b-41d4-a716-446655440000', 'Field Operations', 'department', 500000),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a21', '550e8400-e29b-41d4-a716-446655440000', 'Admin', 'department', 100000),
  -- Vendors
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17', '550e8400-e29b-41d4-a716-446655440000', 'Staples Kenya', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18', '550e8400-e29b-41d4-a716-446655440000', 'AutoXpress Ltd', 'vendor', 0),
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22', '550e8400-e29b-41d4-a716-446655440000', 'Training Co Ltd', 'vendor', 0),
  -- Beneficiaries
  ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a23', '550e8400-e29b-41d4-a716-446655440000', 'Communities', 'beneficiary', 0)
ON CONFLICT (id) DO NOTHING;

-- Demo transactions
INSERT INTO transactions (
  id, organization_id, from_node_id, to_node_id, amount, currency,
  description, category, verification_status, verification_count, transaction_date
) VALUES
  -- Verified transactions
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', -- Operations Budget
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17', -- Staples Kenya
    15000, 'KES',
    'Office Supplies Purchase',
    'Operations',
    'verified', 2,
    '2024-12-18'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b03',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', -- Operations Budget
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18', -- AutoXpress Ltd
    45000, 'KES',
    'Vehicle Maintenance',
    'Transport',
    'verified', 1,
    '2024-12-12'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b04',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', -- General Fund
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a19', -- Payroll
    1200000, 'KES',
    'Staff Salary - November',
    'Salaries',
    'verified', 1,
    '2024-11-30'
  ),
  -- Pending transaction
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b02',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', -- USAID Grant
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', -- Training Department
    250000, 'KES',
    'Community Health Training',
    'Programs',
    'pending', 0,
    '2024-12-15'
  ),
  -- Flagged transaction
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b05',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', -- Project Fund
    NULL, -- Unknown destination
    500000, 'KES',
    'Suspicious Cash Withdrawal',
    'Unclassified',
    'flagged', 0,
    '2024-12-10'
  ),
  -- Additional flow transactions for visualization
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b06',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', -- USAID Grant
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', -- General Fund
    5000000, 'KES',
    'Q4 Grant Disbursement',
    'Funding',
    'verified', 2,
    '2024-10-01'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b07',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a20', -- Gates Foundation
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', -- General Fund
    2500000, 'KES',
    'Annual Grant',
    'Funding',
    'verified', 2,
    '2024-09-15'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b08',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', -- General Fund
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', -- Field Operations
    1500000, 'KES',
    'Field Operations Budget Allocation',
    'Budget',
    'verified', 1,
    '2024-10-15'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b09',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', -- Field Operations
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a23', -- Communities
    3000000, 'KES',
    'Direct Aid Distribution',
    'Programs',
    'verified', 2,
    '2024-11-20'
  ),
  (
    'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b10',
    '550e8400-e29b-41d4-a716-446655440000',
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', -- Training Department
    'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22', -- Training Co Ltd
    500000, 'KES',
    'Workshop Facilitation',
    'Training',
    'verified', 1,
    '2024-11-25'
  )
ON CONFLICT (id) DO NOTHING;

-- Demo verifications for some transactions
INSERT INTO verifications (id, transaction_id, type, status, created_at) VALUES
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c01', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01', 'receipt', 'verified', '2024-12-18 10:30:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c02', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b01', 'photo', 'verified', '2024-12-18 10:35:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c03', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b03', 'receipt', 'verified', '2024-12-12 14:00:00'),
  ('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380c04', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380b04', 'bank_statement', 'verified', '2024-11-30 16:00:00')
ON CONFLICT (id) DO NOTHING;

-- Demo trust score history
INSERT INTO trust_score_history (organization_id, score, breakdown, verification_rate, documentation_rate, consistency_score, calculated_at) VALUES
  ('550e8400-e29b-41d4-a716-446655440000', 72, '{"verification": 70, "documentation": 68, "consistency": 78}', 70.00, 68.00, 78.00, '2024-07-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 75, '{"verification": 74, "documentation": 72, "consistency": 80}', 74.00, 72.00, 80.00, '2024-08-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 78, '{"verification": 78, "documentation": 75, "consistency": 81}', 78.00, 75.00, 81.00, '2024-09-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 82, '{"verification": 82, "documentation": 80, "consistency": 84}', 82.00, 80.00, 84.00, '2024-10-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 85, '{"verification": 88, "documentation": 82, "consistency": 85}', 88.00, 82.00, 85.00, '2024-11-01'),
  ('550e8400-e29b-41d4-a716-446655440000', 87, '{"verification": 92, "documentation": 85, "consistency": 82}', 92.00, 85.00, 82.00, '2024-12-01');
