import { useMutation } from '@tanstack/react-query'
import type { Organization } from '@/lib/types'
import { Switch } from '@/components/ui/switch'
import { updateOrganization } from '@/lib/data'

/**
 * "Show on public pages": the profile's `is_public`, sent alone through
 * update_organization_profile. While it is on, the module publishes the
 * profile and the badge (public_organization, public_badge), which the donate,
 * badge and embed pages read with no sign-in.
 */
export function PublicPagesSwitch({
  org,
  onSaved,
}: {
  org: Organization
  onSaved: (org: Organization) => void
}) {
  const save = useMutation({
    mutationFn: (isPublic: boolean) =>
      updateOrganization(org.id, { is_public: isPublic }),
    onSuccess: onSaved,
  })
  const on = save.isPending ? save.variables : org.is_public === true
  return (
    <div className="flex flex-col gap-2 border-b border-border py-4">
      <div className="flex items-center justify-between gap-4">
        <label
          id="org-public-label"
          htmlFor="org-public"
          className="font-semibold"
        >
          Show on public pages
        </label>
        <Switch
          id="org-public"
          checked={on}
          disabled={save.isPending}
          onCheckedChange={(checked) => save.mutate(checked)}
          aria-labelledby="org-public-label"
          aria-describedby="org-public-msg"
        />
      </div>
      <p id="org-public-msg" aria-live="polite" className="text-sm">
        {save.isError ? (
          <span className="text-destructive">
            Not changed.{' '}
            {save.error instanceof Error
              ? save.error.message
              : 'Try again in a moment.'}
          </span>
        ) : (
          <span className="text-muted-foreground">
            {on
              ? 'Your name, Trust Score and public campaigns can be seen by anyone with the link.'
              : 'Hidden. Donation pages, badges and embeds show nothing about you.'}
          </span>
        )}
      </p>
    </div>
  )
}
