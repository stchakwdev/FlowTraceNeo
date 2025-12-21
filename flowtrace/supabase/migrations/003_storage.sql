-- FlowTrace Storage Configuration
-- Migration 003: Storage Buckets and Policies

-- Create storage buckets for verification documents
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES
  (
    'verifications',
    'verifications',
    false,
    10485760, -- 10MB limit
    ARRAY['image/jpeg', 'image/png', 'image/webp', 'application/pdf']
  ),
  (
    'documents',
    'documents',
    false,
    52428800, -- 50MB limit
    ARRAY['application/pdf', 'image/jpeg', 'image/png', 'application/msword', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document']
  ),
  (
    'logos',
    'logos',
    true, -- Public bucket for organization logos
    5242880, -- 5MB limit
    ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/svg+xml']
  )
ON CONFLICT (id) DO NOTHING;

-- Storage RLS policies for verifications bucket
-- Files are stored as: {organization_id}/{transaction_id}/{filename}

CREATE POLICY "Org members can upload verifications"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'verifications' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can view verifications"
  ON storage.objects FOR SELECT
  USING (
    bucket_id = 'verifications' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can update verifications"
  ON storage.objects FOR UPDATE
  USING (
    bucket_id = 'verifications' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can delete verifications"
  ON storage.objects FOR DELETE
  USING (
    bucket_id = 'verifications' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

-- Storage RLS policies for documents bucket
CREATE POLICY "Org members can upload documents"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'documents' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can view documents"
  ON storage.objects FOR SELECT
  USING (
    bucket_id = 'documents' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can delete documents"
  ON storage.objects FOR DELETE
  USING (
    bucket_id = 'documents' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

-- Storage RLS policies for logos bucket (public read, org write)
CREATE POLICY "Anyone can view logos"
  ON storage.objects FOR SELECT
  USING (bucket_id = 'logos');

CREATE POLICY "Org members can upload logos"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'logos' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can update logos"
  ON storage.objects FOR UPDATE
  USING (
    bucket_id = 'logos' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );

CREATE POLICY "Org members can delete logos"
  ON storage.objects FOR DELETE
  USING (
    bucket_id = 'logos' AND
    (storage.foldername(name))[1] = get_current_org_id()::text
  );
