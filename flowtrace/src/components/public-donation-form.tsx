import { useState } from 'react'
import { CircleNotch, Heart, Check } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useCreateDonation } from '@/lib/queries'
import type { CampaignWithDetails } from '@/lib/supabase'

interface PublicDonationFormProps {
  campaign: CampaignWithDetails
}

const PAYMENT_METHODS = [
  { value: 'mpesa', label: 'M-Pesa' },
  { value: 'card', label: 'Credit/Debit Card' },
  { value: 'bank', label: 'Bank Transfer' },
  { value: 'paypal', label: 'PayPal' },
]

const SUGGESTED_AMOUNTS = [500, 1000, 2500, 5000, 10000]

export function PublicDonationForm({ campaign }: PublicDonationFormProps) {
  const createDonation = useCreateDonation()
  const [submitted, setSubmitted] = useState(false)

  const [formData, setFormData] = useState({
    amount: '',
    donor_name: '',
    donor_email: '',
    donor_phone: '',
    is_anonymous: false,
    payment_method: 'mpesa',
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
        earmarked_node_id: null,
        earmark_notes: null,
        allow_reallocation: true,
        flow_node_id: null,
        transaction_id: null,
        source: 'public_page',
        utm_source: null,
        utm_medium: null,
        utm_campaign: null,
        metadata: {},
      })
      setSubmitted(true)
    } catch (error) {
      console.error('Failed to create donation:', error)
    }
  }

  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency: campaign.currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  if (submitted) {
    return (
      <div className="text-center py-8">
        <div className="flex h-16 w-16 items-center justify-center rounded-full bg-emerald-100 dark:bg-emerald-900/30 mx-auto mb-4">
          <Check className="h-8 w-8 text-emerald-600" weight="bold" />
        </div>
        <h3 className="text-xl font-semibold mb-2">Thank You!</h3>
        <p className="text-muted-foreground mb-4">
          Your donation of {formatAmount(parseFloat(formData.amount))} has been received.
        </p>
        <p className="text-sm text-muted-foreground">
          You will receive a confirmation email shortly with payment instructions.
        </p>
        <Button
          variant="outline"
          className="mt-6"
          onClick={() => {
            setSubmitted(false)
            setFormData({
              amount: '',
              donor_name: '',
              donor_email: '',
              donor_phone: '',
              is_anonymous: false,
              payment_method: 'mpesa',
            })
          }}
        >
          Make Another Donation
        </Button>
      </div>
    )
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
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
          <Label htmlFor="donor_email">Email (for updates) *</Label>
          <Input
            id="donor_email"
            type="email"
            value={formData.donor_email}
            onChange={(e) =>
              setFormData({ ...formData, donor_email: e.target.value })
            }
            placeholder="john@example.com"
            required
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="donor_phone">Phone (for M-Pesa)</Label>
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
          items={PAYMENT_METHODS}
          onValueChange={(value) =>
            setFormData({ ...formData, payment_method: value ?? '' })
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

      {/* Anonymous Option */}
      <label className="flex items-center gap-2 pt-2">
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

      {/* Submit Button */}
      <Button
        type="submit"
        size="lg"
        disabled={createDonation.isPending || !formData.amount}
        className="w-full mt-4"
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

      <p className="text-xs text-center text-muted-foreground pt-2">
        Your donation is secure. You&apos;ll receive payment instructions after submission.
      </p>
    </form>
  )
}
