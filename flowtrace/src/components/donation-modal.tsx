import type { CampaignReportEarmark } from '@/lib/data'
import type { CampaignWithDetails } from '@/lib/supabase'
import { ReportDialog } from '@/components/report-dialog'
import { CampaignCoverage } from '@/components/campaign-progress-bar'
import { PublicDonationForm } from '@/components/public-donation-form'

interface DonationModalProps {
  campaign: CampaignWithDetails
  earmarks?: Array<CampaignReportEarmark>
  isOpen: boolean
  onClose: () => void
}

/** Record a gift to a campaign from inside the app. */
export function DonationModal({
  campaign,
  earmarks,
  isOpen,
  onClose,
}: DonationModalProps) {
  return (
    <ReportDialog
      bodyClassName=""
      open={isOpen}
      onOpenChange={(open) => !open && onClose()}
      title="Record a donation"
      description={campaign.name}
    >
      <div className="border-b border-border bg-card px-5 py-4">
        <CampaignCoverage
          raised={campaign.current_amount}
          target={campaign.target_amount}
          currency={campaign.currency}
          size="sm"
          showKey={false}
        />
      </div>
      <div className="px-5 py-4">
        <PublicDonationForm
          campaign={campaign}
          earmarks={earmarks}
          source="website"
          requireEmail={false}
          onDone={onClose}
          idPrefix="record-donation"
        />
      </div>
    </ReportDialog>
  )
}
