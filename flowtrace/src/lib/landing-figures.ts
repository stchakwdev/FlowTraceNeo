// Every figure the landing page shows, computed from the bundled demo seed (lib/demo/dataset.ts)
// when the module loads, so the page cannot drift from the demo it opens. The hero uses the
// dashboard's own summary (lib/dashboard-summary.ts) and the live ledger block uses the donate
// page's report (lib/demo/report.ts).
import { summarise } from './dashboard-summary'
import { formatCompact } from './format'
import { VERIFICATION_TRUST_POINTS, getTrustGrade } from './types'
import * as seed from './demo/dataset'
import { campaignReport, transactionsOf } from './demo/report'
import type { CampaignReport } from './demo/report'
import type { VerificationType } from './types'

export const LANDING_ORG = seed.demoOrganization

export const DEMO_SUMMARY = summarise(transactionsOf(seed, seed.DEMO_ORG_ID))

export const HERO_FIGURES = {
  evidencedTotal: DEMO_SUMMARY.evidencedTotal,
  evidencedCompact: formatCompact(DEMO_SUMMARY.evidencedTotal),
  movements: `${DEMO_SUMMARY.verified}/${DEMO_SUMMARY.total}`,
  awaiting: DEMO_SUMMARY.pending,
  flagged: DEMO_SUMMARY.flagged,
  // The dashboard shows the organisation's stored score (routes/_authenticated/dashboard.tsx).
  trustScore: LANDING_ORG.trust_score,
  trustGrade: getTrustGrade(LANDING_ORG.trust_score),
}

/** The demo campaign whose public page the landing shows inline. */
export const LANDING_CAMPAIGN_SLUG = 'clean-water-turkana'

const landingCampaign = seed.campaigns.find(
  (c) => c.public_slug === LANDING_CAMPAIGN_SLUG,
)
if (!landingCampaign)
  throw new Error(`Demo campaign ${LANDING_CAMPAIGN_SLUG} is missing.`)

export const LANDING_CAMPAIGN: CampaignReport = (() => {
  const report = campaignReport(seed, landingCampaign.id)
  if (!report) throw new Error('Demo campaign report could not be built.')
  return report
})()

export interface TraceStep {
  from: string
  to: string
  amount: number
  date: string
  description: string
  status: 'verified' | 'pending' | 'flagged' | 'rejected'
  proofs: Array<VerificationType>
}

/** The borehole: the Kisima grant into the Project Fund, then Phase 1 drilling. */
const TRACE_REFERENCES = ['KWT-WS-2026-01', 'PROJ-WATER-001']

export const BOREHOLE_TRACE: Array<TraceStep> = TRACE_REFERENCES.map((ref) => {
  const t = seed.transactions.find((x) => x.reference === ref)
  if (!t) throw new Error(`Demo transaction ${ref} is missing.`)
  const name = (id: string | null) =>
    seed.flowNodes.find((n) => n.id === id)?.name ?? 'No recipient recorded'
  return {
    from: name(t.from_node_id),
    to: name(t.to_node_id),
    amount: t.amount,
    date: t.transaction_date,
    description: t.description,
    status: t.verification_status,
    proofs: seed.verifications
      .filter((v) => v.transaction_id === t.id)
      .map((v) => v.type),
  }
})

/**
 * The proof kinds a member can attach, with the trust points each adds (types.ts
 * VERIFICATION_TRUST_POINTS). Biometric is left out, as the attach panel
 * (routes/_authenticated/transactions/$id.tsx ATTACH_TYPES) leaves it out.
 */
export const PROOF_KINDS: Array<{ type: VerificationType; points: number }> = (
  Object.entries(VERIFICATION_TRUST_POINTS) as Array<[VerificationType, number]>
)
  .filter(([type]) => type !== 'biometric')
  .map(([type, points]) => ({ type, points }))
  .sort((a, b) => b.points - a.points)

/**
 * The module's four weighted parts (routes/_authenticated/trust-score.tsx, decision P16:
 * verification 32, documentation 24, consistency 24, independent review 20).
 */
export const TRUST_PARTS = [
  { label: 'Verification', weight: 32 },
  { label: 'Documentation', weight: 24 },
  { label: 'Consistency', weight: 24 },
  { label: 'Independent review', weight: 20 },
] as const

/** Grade floors, as getTrustGrade (types.ts) applies them. */
export const TRUST_GRADES = [
  { grade: 'A+', min: 90 },
  { grade: 'A', min: 80 },
  { grade: 'B', min: 70 },
  { grade: 'C', min: 60 },
  { grade: 'D', min: 50 },
  { grade: 'F', min: 0 },
] as const
