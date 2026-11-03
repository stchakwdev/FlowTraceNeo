// Names of the proof kinds, shared by the proof screens and the landing page. Kept apart from
// components/verification-gallery.tsx so a page can name proof kinds without loading the viewer.
import type { VerificationType } from './types'

export const PROOF_TYPE_LABEL: Record<VerificationType, string> = {
  receipt: 'Receipt',
  photo: 'Photo',
  document: 'Document',
  bank_statement: 'Bank statement',
  signature: 'Signature',
  gps: 'GPS reading',
  biometric: 'Biometric',
  invoice: 'Invoice',
  video: 'Video',
  audit: 'Third-party audit',
  contract: 'Contract',
  government_doc: 'Government document',
}
