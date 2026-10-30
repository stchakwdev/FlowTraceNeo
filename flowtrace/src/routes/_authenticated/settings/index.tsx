import { useEffect, useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowRight, CircleNotch } from '@phosphor-icons/react'
import type { Organization } from '@/lib/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ReportNotice, ReportSection } from '@/components/report'
import { PublicPagesSwitch } from '@/components/public-pages-switch'
import { useOrg } from '@/lib/org-context'
import { useAccessGroups } from '@/lib/queries'
import { getOrganization, updateOrganization } from '@/lib/data'
import { formatDate } from '@/lib/format'

export const Route = createFileRoute('/_authenticated/settings/')({
  component: SettingsPage,
})

const ORG_TYPES: Array<[NonNullable<Organization['type']>, string]> = [
  ['NGO', 'NGO or nonprofit'],
  ['SOCIAL_ENTERPRISE', 'Social enterprise'],
  ['SME', 'Business'],
  ['GOVERNMENT', 'Government body'],
]

const COUNTRIES: Array<[string, string]> = [
  ['KE', 'Kenya'],
  ['UG', 'Uganda'],
  ['TZ', 'Tanzania'],
  ['RW', 'Rwanda'],
  ['ET', 'Ethiopia'],
  ['NG', 'Nigeria'],
  ['GH', 'Ghana'],
  ['ZA', 'South Africa'],
]

const CURRENCIES: Array<[string, string]> = [
  ['KES', 'KES · Kenyan shilling'],
  ['UGX', 'UGX · Ugandan shilling'],
  ['TZS', 'TZS · Tanzanian shilling'],
  ['RWF', 'RWF · Rwandan franc'],
  ['ETB', 'ETB · Ethiopian birr'],
  ['NGN', 'NGN · Nigerian naira'],
  ['GHS', 'GHS · Ghanaian cedi'],
  ['ZAR', 'ZAR · South African rand'],
  ['USD', 'USD · US dollar'],
  ['EUR', 'EUR · Euro'],
  ['GBP', 'GBP · Pound sterling'],
]

const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

type ProfileForm = {
  name: string
  type: NonNullable<Organization['type']>
  country: string
  currency: string
  public_slug: string
}

function toForm(org: Organization): ProfileForm {
  return {
    name: org.name,
    type: org.type ?? 'NGO',
    country: org.country,
    currency: org.currency,
    public_slug: org.public_slug ?? '',
  }
}

function validate(form: ProfileForm) {
  const errors: Partial<Record<keyof ProfileForm, string>> = {}
  if (form.name.trim().length < 2)
    errors.name = 'Enter your organisation’s name.'
  if (!form.public_slug) errors.public_slug = 'Choose a public link name.'
  else if (!SLUG_PATTERN.test(form.public_slug))
    errors.public_slug =
      'Use lowercase letters, numbers and single hyphens, like demo-ngo.'
  else if (form.public_slug.length > 48)
    errors.public_slug = 'Keep it under 48 characters.'
  return errors
}

function SettingsPage() {
  const { organizationId, refetch: refetchOrg } = useOrg()
  const queryClient = useQueryClient()
  const groups = useAccessGroups()

  const orgQuery = useQuery({
    queryKey: ['organization', organizationId],
    queryFn: () => getOrganization(organizationId!),
    enabled: !!organizationId,
  })

  const [form, setForm] = useState<ProfileForm | null>(null)
  const [submitted, setSubmitted] = useState(false)
  const [savedAt, setSavedAt] = useState<string | null>(null)

  useEffect(() => {
    if (orgQuery.data && !form) setForm(toForm(orgQuery.data))
  }, [orgQuery.data, form])

  const save = useMutation({
    mutationFn: (values: ProfileForm) =>
      updateOrganization(organizationId!, {
        ...values,
        name: values.name.trim(),
      }),
    onSuccess: (org) => {
      queryClient.setQueryData(['organization', organizationId], org)
      setForm(toForm(org))
      setSubmitted(false)
      setSavedAt(org.updated_at)
      void refetchOrg()
    },
  })

  const errors = form ? validate(form) : {}
  const dirty =
    !!form &&
    !!orgQuery.data &&
    JSON.stringify(form) !== JSON.stringify(toForm(orgQuery.data))

  const set = <TKey extends keyof ProfileForm>(
    key: TKey,
    value: ProfileForm[TKey],
  ) => {
    setForm((f) => (f ? { ...f, [key]: value } : f))
    setSavedAt(null)
    save.reset()
  }

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitted(true)
    if (!form || Object.keys(errors).length > 0) return
    save.mutate(form)
  }

  const showError = (key: keyof ProfileForm) =>
    submitted || key === 'public_slug' ? errors[key] : undefined

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-10 px-4 py-6 sm:px-8 sm:py-8">
      <header>
        <h1 className="text-4xl sm:text-5xl">Settings</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          {orgQuery.data?.name ?? 'Your organisation'}: profile, access and what
          you share publicly.
        </p>
      </header>

      <div className="grid gap-10 lg:grid-cols-12">
        <ReportSection
          title="Organisation profile"
          className="lg:col-span-7"
          meta={
            orgQuery.data ? (
              <span>Last changed {formatDate(orgQuery.data.updated_at)}</span>
            ) : undefined
          }
        >
          {orgQuery.isError ? (
            <ReportNotice
              tone="error"
              title="Your organisation profile could not be loaded"
              action={
                <Button
                  variant="outline"
                  className="h-10"
                  onClick={() => orgQuery.refetch()}
                >
                  Try again
                </Button>
              }
            >
              {orgQuery.error instanceof Error
                ? orgQuery.error.message
                : 'Check your connection and try again.'}
            </ReportNotice>
          ) : !form ? (
            <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
              <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
              Loading your profile
            </p>
          ) : (
            <form
              onSubmit={onSubmit}
              noValidate
              className="flex flex-col gap-5 pt-5"
            >
              <Field
                id="org-name"
                label="Name"
                hint="As funders and the public will see it."
                error={showError('name')}
              >
                <Input
                  id="org-name"
                  value={form.name}
                  onChange={(e) => set('name', e.target.value)}
                  className="h-10"
                  autoComplete="organization"
                  aria-invalid={!!showError('name') || undefined}
                  aria-describedby="org-name-msg"
                />
              </Field>

              <div className="grid gap-5 sm:grid-cols-2">
                <Field id="org-type" label="Type">
                  <Select
                    value={form.type}
                    items={Object.fromEntries(ORG_TYPES)}
                    onValueChange={(v) =>
                      v && set('type', v as ProfileForm['type'])
                    }
                  >
                    <SelectTrigger
                      id="org-type"
                      className="h-10 w-full"
                      aria-labelledby="org-type-label"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {ORG_TYPES.map(([value, label]) => (
                        <SelectItem key={value} value={value}>
                          {label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>

                <Field id="org-country" label="Country">
                  <Select
                    value={form.country}
                    items={Object.fromEntries(COUNTRIES)}
                    onValueChange={(v) => v && set('country', v)}
                  >
                    <SelectTrigger
                      id="org-country"
                      className="h-10 w-full"
                      aria-labelledby="org-country-label"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {COUNTRIES.map(([value, label]) => (
                        <SelectItem key={value} value={value}>
                          {label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              </div>

              <Field
                id="org-currency"
                label="Reporting currency"
                hint="Amounts across your reports are shown in this currency."
              >
                <Select
                  value={form.currency}
                  items={Object.fromEntries(CURRENCIES)}
                  onValueChange={(v) => v && set('currency', v)}
                >
                  <SelectTrigger
                    id="org-currency"
                    className="h-10 w-full sm:w-80"
                    aria-labelledby="org-currency-label"
                    aria-describedby="org-currency-msg"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {CURRENCIES.map(([value, label]) => (
                      <SelectItem key={value} value={value}>
                        {label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              <Field
                id="org-slug"
                label="Public link name"
                hint={`Used in your public campaign pages and badges: flowtrace.app/${form.public_slug || 'your-name'}`}
                error={showError('public_slug')}
              >
                <Input
                  id="org-slug"
                  value={form.public_slug}
                  onChange={(e) =>
                    set('public_slug', e.target.value.toLowerCase().trim())
                  }
                  className="h-10 font-mono sm:w-80"
                  spellCheck={false}
                  autoCapitalize="none"
                  aria-invalid={!!showError('public_slug') || undefined}
                  aria-describedby="org-slug-msg"
                />
              </Field>

              <div className="flex flex-wrap items-center gap-4 border-t border-border pt-4">
                <Button
                  type="submit"
                  className="h-10"
                  disabled={!dirty || save.isPending}
                >
                  {save.isPending && (
                    <CircleNotch className="animate-spin" aria-hidden="true" />
                  )}
                  {save.isPending ? 'Saving' : 'Save profile'}
                </Button>
                {dirty && !save.isPending && (
                  <Button
                    variant="ghost"
                    className="h-10"
                    onClick={() => {
                      setForm(null)
                      setSubmitted(false)
                      save.reset()
                    }}
                  >
                    Discard changes
                  </Button>
                )}
                <p aria-live="polite" className="text-sm">
                  {save.isError ? (
                    <span className="text-destructive">
                      Not saved.{' '}
                      {save.error instanceof Error
                        ? save.error.message
                        : 'Try again in a moment.'}
                    </span>
                  ) : savedAt ? (
                    <span className="text-muted-foreground">
                      Saved. Your profile is up to date.
                    </span>
                  ) : dirty ? (
                    <span className="text-muted-foreground">
                      You have unsaved changes.
                    </span>
                  ) : null}
                </p>
              </div>
            </form>
          )}
        </ReportSection>

        <ReportSection title="Access and sharing" className="lg:col-span-5">
          {orgQuery.data && (
            <PublicPagesSwitch
              org={orgQuery.data}
              onSaved={(org) => {
                queryClient.setQueryData(['organization', organizationId], org)
                void refetchOrg()
              }}
            />
          )}
          <ul className="divide-y divide-border">
            <SettingsLink
              to="/settings/access-groups"
              title="Access groups"
              description={
                groups.data
                  ? `Who sees which depth of your flows. ${groups.data.length} groups, from board to public.`
                  : 'Who sees which depth of your flows, from board to public.'
              }
            />
            <SettingsLink
              to="/settings/embed"
              title="Embed & badges"
              description="Campaign widgets and Trust Score badges for your website."
            />
            <SettingsLink
              to="/chains"
              title="Chain links"
              description="Partners and funders whose flows are linked to yours."
            />
          </ul>
        </ReportSection>
      </div>
    </div>
  )
}

function Field({
  id,
  label,
  hint,
  error,
  children,
}: {
  id: string
  label: string
  hint?: string
  error?: string
  children: React.ReactNode
}) {
  return (
    <div className="flex flex-col gap-2">
      <label id={`${id}-label`} htmlFor={id} className="text-sm font-semibold">
        {label}
      </label>
      {children}
      {(error || hint) && (
        <p
          id={`${id}-msg`}
          className={
            error
              ? 'text-xs text-destructive'
              : 'text-xs text-muted-foreground [overflow-wrap:anywhere]'
          }
        >
          {error ?? hint}
        </p>
      )}
    </div>
  )
}

function SettingsLink({
  to,
  title,
  description,
}: {
  to: '/settings/access-groups' | '/settings/embed' | '/chains'
  title: string
  description: string
}) {
  return (
    <li>
      <Link
        to={to}
        className="group flex min-h-16 items-center justify-between gap-4 py-4 outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <span>
          <span className="block font-semibold group-hover:text-primary group-hover:underline group-hover:underline-offset-4">
            {title}
          </span>
          <span className="mt-1 block text-sm text-muted-foreground">
            {description}
          </span>
        </span>
        <ArrowRight
          aria-hidden="true"
          className="size-5 shrink-0 text-muted-foreground transition-transform duration-150 group-hover:translate-x-0.5 group-hover:text-primary"
        />
      </Link>
    </li>
  )
}
