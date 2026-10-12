import { useState } from 'react'
import { CircleNotch } from '@phosphor-icons/react'
import type { CampaignReportEarmark } from '@/lib/data'
import type { CampaignWithDetails } from '@/lib/supabase'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StatusMark } from '@/components/report'
import {
  CheckboxField,
  FormField,
  fieldAria,
} from '@/components/report-dialog'
import { useCreateDonation } from '@/lib/queries'
import { formatAmount, formatMoney } from '@/lib/format'
import { cn } from '@/lib/utils'

const PAYMENT_METHODS = [
  { value: 'mpesa', label: 'M-Pesa' },
  { value: 'card', label: 'Debit or credit card' },
  { value: 'bank', label: 'Bank transfer' },
  { value: 'paypal', label: 'PayPal' },
]

const SUGGESTED_AMOUNTS = [500, 1000, 2500, 5000, 10000]

const EMPTY = {
  amount: '',
  donor_name: '',
  donor_email: '',
  donor_phone: '',
  is_anonymous: false,
  payment_method: 'mpesa',
  earmarked_node_id: '',
  allow_reallocation: true,
}

type Errors = Partial<Record<'amount' | 'donor_email' | 'donor_phone', string>>

interface DonationFormProps {
  campaign: CampaignWithDetails
  /** Projects a donor can direct the gift to. */
  earmarks?: Array<CampaignReportEarmark>
  /** Where the gift came from, recorded on the donation. */
  source?: string
  /** Email is needed to send a donor their proof updates. */
  requireEmail?: boolean
  onDone?: () => void
  idPrefix?: string
}

/** The donation form used on the public donate page and in the app. */
export function PublicDonationForm({
  campaign,
  earmarks = [],
  source = 'public_page',
  requireEmail = true,
  onDone,
  idPrefix = 'donate',
}: DonationFormProps) {
  const createDonation = useCreateDonation()
  const [form, setForm] = useState(EMPTY)
  const [errors, setErrors] = useState<Errors>({})
  const [submitError, setSubmitError] = useState<string | null>(null)
  const [submitted, setSubmitted] = useState<number | null>(null)
  const id = (name: string) => `${idPrefix}-${name}`
  const currency = campaign.currency

  const set = <TKey extends keyof typeof EMPTY>(key: TKey, value: (typeof EMPTY)[TKey]) => {
    setForm((f) => ({ ...f, [key]: value }))
    if (key in errors) setErrors((e) => ({ ...e, [key]: undefined }))
  }

  const validate = (): Errors => {
    const e: Errors = {}
    const amount = Number(form.amount)
    if (!form.amount) e.amount = 'Enter how much you would like to give.'
    else if (!Number.isFinite(amount) || amount < 1)
      e.amount = `The smallest gift is ${formatMoney(1, currency)}.`
    if (requireEmail && !form.donor_email.trim())
      e.donor_email = 'Add an email so we can send you proof of how your gift is spent.'
    else if (form.donor_email && !/^\S+@\S+\.\S+$/.test(form.donor_email))
      e.donor_email = 'This email address looks incomplete.'
    if (form.payment_method === 'mpesa' && !form.donor_phone.trim())
      e.donor_phone = 'M-Pesa needs the phone number that will receive the payment prompt.'
    return e
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const found = validate()
    setErrors(found)
    if (Object.values(found).some(Boolean)) return
    setSubmitError(null)
    try {
      await createDonation.mutateAsync({
        campaign_id: campaign.id,
        amount: Number(form.amount),
        currency,
        donor_name: form.is_anonymous ? null : form.donor_name.trim() || null,
        donor_email: form.donor_email.trim() || null,
        donor_phone: form.donor_phone.trim() || null,
        is_anonymous: form.is_anonymous,
        payment_method: form.payment_method,
        payment_status: 'pending',
        payment_reference: null,
        amount_usd: null,
        exchange_rate: null,
        earmarked_node_id: form.earmarked_node_id || null,
        earmark_notes: null,
        allow_reallocation: form.allow_reallocation,
        flow_node_id: null,
        transaction_id: null,
        source,
        utm_source: null,
        utm_medium: null,
        utm_campaign: null,
        metadata: {},
      })
      setSubmitted(Number(form.amount))
    } catch (error) {
      setSubmitError(
        error instanceof Error
          ? `Your gift was not recorded: ${error.message}`
          : 'Your gift was not recorded. Check your connection and try again.',
      )
    }
  }

  if (submitted !== null) {
    return (
      <div role="status" className="py-2">
        <p className="text-2xl font-[750] font-condensed">
          Thank you. {formatMoney(submitted, currency)} pledged.
        </p>
        <StatusMark status="pending" label="Awaiting payment" className="mt-3" />
        <p className="mt-3 text-sm text-muted-foreground">
          {form.payment_method === 'mpesa'
            ? 'Check your phone for the M-Pesa prompt. '
            : 'Payment instructions are on their way to your email. '}
          Once the payment is confirmed it joins the campaign total, and you will
          get an update each time your gift is spent, with the proof attached.
        </p>
        <div className="mt-5 flex flex-wrap gap-2">
          <Button
            variant="outline"
            className="h-10"
            onClick={() => {
              setSubmitted(null)
              setForm(EMPTY)
            }}
          >
            Give again
          </Button>
          {onDone && (
            <Button variant="ghost" className="h-10" onClick={onDone}>
              Close
            </Button>
          )}
        </div>
      </div>
    )
  }

  const amountValue = Number(form.amount)

  return (
    <form noValidate onSubmit={handleSubmit} className="space-y-4">
      <FormField id={id('amount')} label="Amount" required error={errors.amount}>
        <div
          role="group"
          aria-label="Suggested amounts"
          className="grid grid-cols-3 gap-1.5 sm:grid-cols-5"
        >
          {SUGGESTED_AMOUNTS.map((amount) => {
            const selected = form.amount === String(amount)
            return (
              <button
                key={amount}
                type="button"
                aria-pressed={selected}
                onClick={() => set('amount', String(amount))}
                className={cn(
                  'h-10 border text-sm font-semibold tabular-nums transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50',
                  selected
                    ? 'border-primary bg-primary text-primary-foreground'
                    : 'border-border bg-card hover:border-foreground',
                )}
              >
                {formatAmount(amount)}
              </button>
            )
          })}
        </div>
        <div className="relative mt-1.5">
          <span
            aria-hidden="true"
            className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-sm font-semibold text-muted-foreground"
          >
            {currency}
          </span>
          <Input
            id={id('amount')}
            type="number"
            inputMode="numeric"
            min="1"
            step="1"
            value={form.amount}
            onChange={(e) => set('amount', e.target.value)}
            placeholder="Other amount"
            className="h-10 pl-12 tabular-nums"
            {...fieldAria(id('amount'), errors.amount)}
          />
        </div>
      </FormField>

      {earmarks.length > 0 && (
        <FormField
          id={id('earmark')}
          label="Where it goes"
          hint="Direct your gift to one project, or let the team send it where it is needed most."
        >
          <Select
            value={form.earmarked_node_id}
            items={{
              '': 'Where it is needed most',
              ...Object.fromEntries(earmarks.map((n) => [n.id, n.name])),
            }}
            onValueChange={(v) => set('earmarked_node_id', v ?? '')}
          >
            <SelectTrigger
              id={id('earmark')}
              className="h-10 w-full"
              {...fieldAria(id('earmark'), undefined, true)}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">Where it is needed most</SelectItem>
              {earmarks.map((n) => (
                <SelectItem key={n.id} value={n.id}>
                  {n.name}
                  {n.is_target ? ' (this campaign)' : ''}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>
      )}

      <FormField id={id('method')} label="Pay with">
        <Select
          value={form.payment_method}
          items={PAYMENT_METHODS}
          onValueChange={(v) => set('payment_method', v ?? 'mpesa')}
        >
          <SelectTrigger id={id('method')} className="h-10 w-full">
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
      </FormField>

      {form.payment_method === 'mpesa' && (
        <FormField
          id={id('phone')}
          label="M-Pesa phone number"
          required
          error={errors.donor_phone}
        >
          <Input
            id={id('phone')}
            type="tel"
            autoComplete="tel"
            value={form.donor_phone}
            onChange={(e) => set('donor_phone', e.target.value)}
            placeholder="+254 712 345 678"
            className="h-10"
            {...fieldAria(id('phone'), errors.donor_phone)}
          />
        </FormField>
      )}

      <FormField
        id={id('email')}
        label="Email"
        required={requireEmail}
        error={errors.donor_email}
        hint="For your receipt and updates when the money is spent."
      >
        <Input
          id={id('email')}
          type="email"
          autoComplete="email"
          value={form.donor_email}
          onChange={(e) => set('donor_email', e.target.value)}
          className="h-10"
          {...fieldAria(id('email'), errors.donor_email, true)}
        />
      </FormField>

      <FormField id={id('name')} label="Your name">
        <Input
          id={id('name')}
          autoComplete="name"
          value={form.is_anonymous ? '' : form.donor_name}
          onChange={(e) => set('donor_name', e.target.value)}
          disabled={form.is_anonymous}
          placeholder={form.is_anonymous ? 'Shown as Anonymous' : undefined}
          className="h-10"
        />
      </FormField>

      <div>
        <CheckboxField
          checked={form.is_anonymous}
          onCheckedChange={(v) => set('is_anonymous', v)}
          label="Give anonymously"
          hint="Your name is not shown publicly. The organisation still sees it."
        />
        {!campaign.allow_overflow && (
          <CheckboxField
            checked={form.allow_reallocation}
            onCheckedChange={(v) => set('allow_reallocation', v)}
            label="If the target is already met, use my gift on another project"
          />
        )}
      </div>

      {submitError && (
        <p role="alert" className="text-sm font-semibold text-destructive">
          {submitError}
        </p>
      )}

      <Button
        type="submit"
        size="lg"
        disabled={createDonation.isPending}
        className="w-full"
      >
        {createDonation.isPending && <CircleNotch className="animate-spin" />}
        {amountValue > 0 ? `Give ${formatMoney(amountValue, currency)}` : 'Give'}
      </Button>
      <p className="text-xs text-muted-foreground">
        Your gift is recorded as awaiting payment until the payment provider
        confirms it.
      </p>
    </form>
  )
}
