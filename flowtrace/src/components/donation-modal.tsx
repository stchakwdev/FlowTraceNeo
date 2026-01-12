import { useState } from 'react'
import { CircleNotch, Heart, X } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { CampaignProgressBar } from './campaign-progress-bar'
import { useCreateDonation, useNodes } from '@/lib/queries'
import type { CampaignWithDetails } from '@/lib/supabase'

interface DonationModalProps {
  campaign: CampaignWithDetails
  isOpen: boolean
  onClose: () => void
}

const PAYMENT_METHODS = [
  { value: 'mpesa', label: 'M-Pesa' },
  { value: 'card', label: 'Credit/Debit Card' },
  { value: 'bank', label: 'Bank Transfer' },
  { value: 'paypal', label: 'PayPal' },
]

const SUGGESTED_AMOUNTS = [500, 1000, 2500, 5000, 10000]

export function DonationModal({ campaign, isOpen, onClose }: DonationModalProps) {
  const createDonation = useCreateDonation()
  const { data: nodes } = useNodes()

  const [formData, setFormData] = useState({
    amount: '',
    donor_name: '',
    donor_email: '',
    donor_phone: '',
    is_anonymous: false,
    payment_method: 'mpesa',
    earmarked_node_id: '',
    earmark_notes: '',
    allow_reallocation: true,
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    try {
      await createDonation.mutateAsync({
        campaign_id: campaign.id,
        amount: parseFloat(formData.amount),
        currency: campaign.currency,
        donor_name: formData.is_anonymous ? null : formData.donor_name || null,
        donor_email: formData.donor_email || null,
        donor_phone: formData.donor_phone || null,
        is_anonymous: formData.is_anonymous,
        payment_method: formData.payment_method,
        payment_status: 'pending',
        payment_reference: null,
        amount_usd: null,
        exchange_rate: null,
        earmarked_node_id: formData.earmarked_node_id || null,
        earmark_notes: formData.earmark_notes || null,
        allow_reallocation: formData.allow_reallocation,
        flow_node_id: null,
        transaction_id: null,
        source: 'website',
        utm_source: null,
        utm_medium: null,
        utm_campaign: null,
        metadata: {},
      })
      onClose()
      resetForm()
    } catch (error) {
      console.error('Failed to create donation:', error)
    }
  }

  const resetForm = () => {
    setFormData({
      amount: '',
      donor_name: '',
      donor_email: '',
      donor_phone: '',
      is_anonymous: false,
      payment_method: 'mpesa',
      earmarked_node_id: '',
      earmark_notes: '',
      allow_reallocation: true,
    })
  }

  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency: campaign.currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  // Get nodes that can be earmarked for this campaign
  const earmarkableNodes = nodes?.filter(
    (n) => n.type === 'project' || n.type === 'department',
  )

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative z-10 w-full max-w-md max-h-[90vh] overflow-auto rounded-lg border bg-card shadow-lg">
        {/* Header */}
        <div className="border-b px-6 py-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">Make a Donation</h2>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-5 w-5" />
            </Button>
          </div>
          <p className="mt-1 text-sm text-muted-foreground">{campaign.name}</p>
        </div>

        {/* Campaign Progress */}
        <div className="px-6 py-4 bg-muted/50">
          <CampaignProgressBar
            current={campaign.current_amount}
            target={campaign.target_amount}
            currency={campaign.currency}
            size="sm"
          />
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {/* Amount */}
          <div className="space-y-2">
            <Label htmlFor="amount">Donation Amount *</Label>
            <div className="flex gap-2">
              <Input
                id="amount"
                type="number"
                min="1"
                step="0.01"
                value={formData.amount}
                onChange={(e) =>
                  setFormData({ ...formData, amount: e.target.value })
                }
                placeholder="Enter amount"
                className="flex-1"
                required
              />
              <span className="flex items-center px-3 bg-muted rounded-md text-sm font-medium">
                {campaign.currency}
              </span>
            </div>

            {/* Suggested amounts */}
            <div className="flex flex-wrap gap-2 pt-1">
              {SUGGESTED_AMOUNTS.map((amount) => (
                <button
                  key={amount}
                  type="button"
                  onClick={() =>
                    setFormData({ ...formData, amount: amount.toString() })
                  }
                  className="px-3 py-1 text-xs rounded-full border hover:bg-accent transition-colors"
                >
                  {formatAmount(amount)}
                </button>
              ))}
            </div>
          </div>

          {/* Donor Info */}
          <div className="space-y-3">
            <div className="space-y-2">
              <Label htmlFor="donor_name">Your Name</Label>
              <Input
                id="donor_name"
                value={formData.donor_name}
                onChange={(e) =>
                  setFormData({ ...formData, donor_name: e.target.value })
                }
                placeholder="John Doe"
                disabled={formData.is_anonymous}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="donor_email">Email (for updates)</Label>
              <Input
                id="donor_email"
                type="email"
                value={formData.donor_email}
                onChange={(e) =>
                  setFormData({ ...formData, donor_email: e.target.value })
                }
                placeholder="john@example.com"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="donor_phone">Phone (Optional)</Label>
              <Input
                id="donor_phone"
                type="tel"
                value={formData.donor_phone}
                onChange={(e) =>
                  setFormData({ ...formData, donor_phone: e.target.value })
                }
                placeholder="+254 700 000000"
              />
            </div>
          </div>

          {/* Payment Method */}
          <div className="space-y-2">
            <Label htmlFor="payment_method">Payment Method</Label>
            <Select
              value={formData.payment_method}
              onValueChange={(value) =>
                setFormData({ ...formData, payment_method: value })
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PAYMENT_METHODS.map((m) => (
                  <SelectItem key={m.value} value={m.value}>
                    {m.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Earmarking */}
          {earmarkableNodes && earmarkableNodes.length > 0 && (
            <div className="space-y-2">
              <Label htmlFor="earmarked_node">Earmark for (Optional)</Label>
              <Select
                value={formData.earmarked_node_id}
                onValueChange={(value) =>
                  setFormData({ ...formData, earmarked_node_id: value })
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder="Any area of need" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">Any area of need</SelectItem>
                  {earmarkableNodes.map((node) => (
                    <SelectItem key={node.id} value={node.id}>
                      {node.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {formData.earmarked_node_id && (
                <div className="space-y-2">
                  <Label htmlFor="earmark_notes">Earmark Notes</Label>
                  <Textarea
                    id="earmark_notes"
                    value={formData.earmark_notes}
                    onChange={(e) =>
                      setFormData({ ...formData, earmark_notes: e.target.value })
                    }
                    placeholder="e.g., For classroom supplies"
                    rows={2}
                  />
                </div>
              )}
            </div>
          )}

          {/* Options */}
          <div className="space-y-3 pt-2">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.is_anonymous}
                onChange={(e) =>
                  setFormData({ ...formData, is_anonymous: e.target.checked })
                }
                className="rounded border-input"
              />
              <span className="text-sm">Make my donation anonymous</span>
            </label>

            {!campaign.allow_overflow && (
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={formData.allow_reallocation}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      allow_reallocation: e.target.checked,
                    })
                  }
                  className="rounded border-input"
                />
                <span className="text-sm">
                  Allow reallocation if campaign is fully funded
                </span>
              </label>
            )}
          </div>

          {/* Actions */}
          <div className="flex flex-col gap-3 pt-4 border-t">
            <Button
              type="submit"
              size="lg"
              disabled={createDonation.isPending || !formData.amount}
              className="w-full"
            >
              {createDonation.isPending ? (
                <CircleNotch className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Heart className="h-4 w-4 mr-2" weight="fill" />
              )}
              {formData.amount
                ? `Donate ${formatAmount(parseFloat(formData.amount))}`
                : 'Donate'}
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={onClose}
              className="w-full"
            >
              Cancel
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
