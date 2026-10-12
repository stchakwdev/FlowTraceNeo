import { useState } from 'react'
import { CircleNotch } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  DateField,
  localTodayISO,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  CheckboxField,
  FormField,
  ReportDialog,
  fieldAria,
} from '@/components/report-dialog'
import { useCreateCampaign, useNodes } from '@/lib/queries'

// ---------------------------------------------------------------------------
// Create campaign
// ---------------------------------------------------------------------------

interface CampaignCreateModalProps {
  isOpen: boolean
  onClose: () => void
}

const CATEGORIES = [
  { value: 'water', label: 'Water and sanitation' },
  { value: 'education', label: 'Education' },
  { value: 'health', label: 'Health' },
  { value: 'infrastructure', label: 'Infrastructure' },
  { value: 'emergency', label: 'Emergency relief' },
  { value: 'environment', label: 'Environment' },
  { value: 'community', label: 'Community development' },
  { value: 'other', label: 'Other' },
]

const CURRENCIES = [
  { value: 'KES', label: 'KES' },
  { value: 'USD', label: 'USD' },
  { value: 'EUR', label: 'EUR' },
  { value: 'GBP', label: 'GBP' },
]

const EMPTY_FORM = {
  name: '',
  description: '',
  short_description: '',
  target_amount: '',
  currency: 'KES',
  target_node_id: '',
  category: '',
  is_public: true,
  allow_overflow: false,
  ends_at: '',
}

type FormErrors = Partial<Record<'name' | 'target_amount' | 'ends_at', string>>

function endDateIso(text: string) {
  return parseDayMonthYear(text)?.iso ?? null
}

function validate(form: typeof EMPTY_FORM): FormErrors {
  const errors: FormErrors = {}
  if (!form.name.trim()) errors.name = 'Give the campaign a name donors will recognise.'
  const amount = Number(form.target_amount)
  if (!form.target_amount) errors.target_amount = 'Enter the amount you need to raise.'
  else if (!Number.isFinite(amount) || amount <= 0)
    errors.target_amount = 'The target must be more than zero.'
  const end = parseDayMonthYear(form.ends_at)
  if (end?.error) errors.ends_at = end.error
  else if (end?.iso && end.iso < localTodayISO())
    errors.ends_at = 'The end date has passed. Pick today or a later date.'
  return errors
}

export function CampaignCreateModal({ isOpen, onClose }: CampaignCreateModalProps) {
  const createCampaign = useCreateCampaign()
  const { data: nodes } = useNodes()
  const [form, setForm] = useState(EMPTY_FORM)
  const [errors, setErrors] = useState<FormErrors>({})
  const [submitError, setSubmitError] = useState<string | null>(null)

  const projects = (nodes ?? []).filter(
    (n) => n.type === 'project' || n.type === 'department',
  )

  const set = <TKey extends keyof typeof EMPTY_FORM>(
    key: TKey,
    value: (typeof EMPTY_FORM)[TKey],
  ) => {
    setForm((f) => ({ ...f, [key]: value }))
    if (key in errors) setErrors((e) => ({ ...e, [key]: undefined }))
  }

  const close = () => {
    setForm(EMPTY_FORM)
    setErrors({})
    setSubmitError(null)
    onClose()
  }

  const submit = async (asDraft: boolean) => {
    const found = validate(form)
    setErrors(found)
    if (Object.values(found).some(Boolean)) return
    setSubmitError(null)
    try {
      await createCampaign.mutateAsync({
        name: form.name.trim(),
        description: form.description.trim() || null,
        short_description: form.short_description.trim() || null,
        image_url: null,
        target_amount: Number(form.target_amount),
        currency: form.currency,
        target_node_id: form.target_node_id || null,
        status: asDraft ? 'draft' : 'active',
        allow_overflow: form.allow_overflow,
        overflow_campaign_id: null,
        overflow_consent_message: null,
        starts_at: new Date().toISOString(),
        ends_at: endDateIso(form.ends_at)
          ? `${endDateIso(form.ends_at)}T23:59:59Z`
          : null,
        is_public: form.is_public,
        public_slug: null,
        category: form.category || null,
        tags: null,
        metadata: {},
        created_by: null,
      })
      close()
    } catch (error) {
      setSubmitError(
        error instanceof Error
          ? `The campaign was not saved: ${error.message}`
          : 'The campaign was not saved. Check your connection and try again.',
      )
    }
  }

  return (
    <ReportDialog
      bodyClassName=""
      open={isOpen}
      onOpenChange={(open) => !open && close()}
      title="New campaign"
      description="Set what you are raising for. You can save it as a draft and open it to donors later."
    >
      <form
        noValidate
        onSubmit={(e) => {
          e.preventDefault()
          void submit(false)
        }}
        className="space-y-4 px-5 py-4"
      >
        <FormField id="campaign-name" label="Campaign name" required error={errors.name}>
          <Input
            id="campaign-name"
            value={form.name}
            onChange={(e) => set('name', e.target.value)}
            placeholder="e.g. Clean water for Turkana schools"
            className="h-10"
            {...fieldAria('campaign-name', errors.name)}
          />
        </FormField>

        <FormField
          id="campaign-summary"
          label="One-line summary"
          hint="Shown on the donate page and in embeds. Up to 150 characters."
        >
          <Input
            id="campaign-summary"
            value={form.short_description}
            onChange={(e) => set('short_description', e.target.value)}
            maxLength={150}
            className="h-10"
            {...fieldAria('campaign-summary', undefined, true)}
          />
        </FormField>

        <FormField id="campaign-story" label="The story">
          <Textarea
            id="campaign-story"
            value={form.description}
            onChange={(e) => set('description', e.target.value)}
            placeholder="What the money will do, where, and for whom."
            rows={3}
          />
        </FormField>

        <div className="grid grid-cols-[1fr_6.5rem] gap-3">
          <FormField
            id="campaign-target"
            label="Target"
            required
            error={errors.target_amount}
          >
            <Input
              id="campaign-target"
              type="number"
              inputMode="numeric"
              min="1"
              step="1"
              value={form.target_amount}
              onChange={(e) => set('target_amount', e.target.value)}
              placeholder="e.g. 500,000"
              className="h-10 tabular-nums"
              {...fieldAria('campaign-target', errors.target_amount)}
            />
          </FormField>
          <FormField id="campaign-currency" label="Currency">
            <Select
              value={form.currency}
              items={CURRENCIES}
              onValueChange={(v) => set('currency', v ?? 'KES')}
            >
              <SelectTrigger id="campaign-currency" className="h-10 w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CURRENCIES.map((c) => (
                  <SelectItem key={c.value} value={c.value}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </FormField>
        </div>

        <FormField
          id="campaign-project"
          label="Project it funds"
          hint="Donations flow into this project, so spending from it shows as proof on the campaign."
        >
          <Select
            value={form.target_node_id}
            items={{
              '': 'Not tied to one project',
              ...Object.fromEntries(projects.map((n) => [n.id, n.name])),
            }}
            onValueChange={(v) => set('target_node_id', v ?? '')}
          >
            <SelectTrigger
              id="campaign-project"
              className="h-10 w-full"
              {...fieldAria('campaign-project', undefined, true)}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">Not tied to one project</SelectItem>
              {projects.map((n) => (
                <SelectItem key={n.id} value={n.id}>
                  {n.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>

        <div className="grid gap-3 sm:grid-cols-2">
          <FormField id="campaign-category" label="Category">
            <Select
              value={form.category}
              items={[{ value: '', label: 'None' }, ...CATEGORIES]}
              onValueChange={(v) => set('category', v ?? '')}
            >
              <SelectTrigger id="campaign-category" className="h-10 w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="">None</SelectItem>
                {CATEGORIES.map((c) => (
                  <SelectItem key={c.value} value={c.value}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </FormField>
          <FormField
            id="campaign-ends"
            label="End date"
            error={errors.ends_at}
            hint="Day, month, year. Leave empty for no end date."
          >
            <DateField
              id="campaign-ends"
              value={form.ends_at}
              min={localTodayISO()}
              onValueChange={(v) => set('ends_at', v)}
              pickerLabel="Choose the end date from a calendar"
              className="h-10"
              {...fieldAria('campaign-ends', errors.ends_at, true)}
            />
          </FormField>
        </div>

        <fieldset className="border-t border-border pt-2">
          <legend className="sr-only">Options</legend>
          <CheckboxField
            checked={form.is_public}
            onCheckedChange={(v) => set('is_public', v)}
            label="Public donate page"
            hint="Anyone with the link can see the campaign and give."
          />
          <CheckboxField
            checked={form.allow_overflow}
            onCheckedChange={(v) => set('allow_overflow', v)}
            label="Keep accepting gifts after the target is reached"
          />
        </fieldset>

        {submitError && (
          <p role="alert" className="text-sm font-semibold text-destructive">
            {submitError}
          </p>
        )}

        <div className="-mx-5 flex flex-col-reverse gap-2 border-t border-border px-5 pt-4 sm:flex-row sm:justify-end">
          <Button type="button" variant="ghost" className="h-10" onClick={close}>
            Cancel
          </Button>
          <Button
            type="button"
            variant="outline"
            className="h-10"
            disabled={createCampaign.isPending}
            onClick={() => void submit(true)}
          >
            Save as draft
          </Button>
          <Button type="submit" className="h-10" disabled={createCampaign.isPending}>
            {createCampaign.isPending && <CircleNotch className="animate-spin" />}
            Create and open to donors
          </Button>
        </div>
      </form>
    </ReportDialog>
  )
}
