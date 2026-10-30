import { useEffect, useRef, useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowLeft,
  Check,
  CircleNotch,
  Minus,
  PencilSimple,
  Plus,
  Trash,
  UserPlus,
} from '@phosphor-icons/react'
import type { AccessGroup, AccessGroupPermissions } from '@/lib/types'
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
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { ReportNotice, ReportSection } from '@/components/report'
import { ReportCheckbox, ReportDialog } from '@/components/report-dialog'
import { ChainLine } from '@/components/chain-status-badge'
import {
  useAccessGroups,
  useAddGroupMember,
  useCreateAccessGroup,
  useCreateDefaultAccessGroups,
  useDeleteAccessGroup,
  useGroupMembers,
  useInviteMemberByEmail,
  useOrgMembers,
  useRemoveGroupMember,
  useUpdateAccessGroup,
} from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'

export const Route = createFileRoute('/_authenticated/settings/access-groups')({
  component: AccessGroupsPage,
})

const SCOPE_LABEL: Record<AccessGroup['node_scope'], string> = {
  all: 'All flows',
  assigned: 'Assigned nodes',
  downstream: 'What their money funds',
}

const SCOPE_HINT: Record<AccessGroup['node_scope'], string> = {
  all: 'Every node in your flows.',
  assigned: 'Only the nodes you assign to this group.',
  downstream: 'The nodes downstream of the money they gave.',
}

type ProofDepth = 'none' | 'summary' | 'full'

const proofDepth = (p: AccessGroupPermissions): ProofDepth =>
  p.can_view_full_proofs ? 'full' : p.can_view_proofs ? 'summary' : 'none'

const ERROR_TEXT = (err: unknown) =>
  err instanceof Error ? err.message : 'Try again in a moment.'

function AccessGroupsPage() {
  const { organization } = useOrg()
  const groups = useAccessGroups()
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [editing, setEditing] = useState<AccessGroup | 'new' | null>(null)
  const [deleting, setDeleting] = useState<AccessGroup | null>(null)
  const deleteGroup = useDeleteAccessGroup()
  const defaults = useCreateDefaultAccessGroups()

  const list = groups.data ?? []
  const selected = list.find((g) => g.id === selectedId) ?? list.at(0) ?? null

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-10 px-4 py-6 sm:px-8 sm:py-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <Link
            to="/settings"
            className="mb-3 inline-flex min-h-10 items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
          >
            <ArrowLeft aria-hidden="true" className="size-4" />
            Settings
          </Link>
          <h1 className="text-4xl sm:text-5xl">Access groups</h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Who sees what in {organization?.name ?? 'your organisation'}. Each
            group sees its own depth of your flows, and FlowTrace holds the
            line.
          </p>
        </div>
        <Button className="h-10" onClick={() => setEditing('new')}>
          <Plus aria-hidden="true" />
          Create group
        </Button>
      </header>

      <ReportSection
        title="What each group can see and do"
        meta={
          groups.data ? (
            <span className="tabular-nums">{list.length} groups</span>
          ) : undefined
        }
      >
        {groups.isError ? (
          <ReportNotice
            tone="error"
            title="Access groups could not be loaded"
            action={
              <Button
                variant="outline"
                className="h-10"
                onClick={() => groups.refetch()}
              >
                Try again
              </Button>
            }
          >
            {ERROR_TEXT(groups.error)}
          </ReportNotice>
        ) : groups.isLoading ? (
          <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
            <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
            Loading access groups
          </p>
        ) : list.length === 0 ? (
          <ReportNotice
            title="No access groups yet"
            action={
              <div className="flex flex-wrap gap-3">
                <Button
                  className="h-10"
                  disabled={defaults.isPending}
                  onClick={() => defaults.mutate()}
                >
                  {defaults.isPending && (
                    <CircleNotch className="animate-spin" aria-hidden="true" />
                  )}
                  Create the standard groups
                </Button>
                <Button
                  variant="outline"
                  className="h-10"
                  onClick={() => setEditing('new')}
                >
                  <Plus aria-hidden="true" />
                  Create group
                </Button>
              </div>
            }
          >
            Groups decide who sees your flows and at what depth. The standard
            five run from your board to the public.
            {defaults.isError && (
              <span role="alert" className="mt-2 block text-destructive">
                {ERROR_TEXT(defaults.error)}
              </span>
            )}
          </ReportNotice>
        ) : (
          <>
            <div className="relative w-0 min-w-full overflow-x-auto">
              <table className="w-full min-w-[58rem] text-left text-sm">
                <caption className="sr-only">
                  Permissions for each access group. Select a group to see its
                  members.
                </caption>
                <thead>
                  <tr className="border-b border-border text-xs text-muted-foreground">
                    <th scope="col" className="py-3 pr-4 font-semibold">
                      Group
                    </th>
                    <th scope="col" className="py-3 pr-4 font-semibold">
                      Sees
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Amounts
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Proofs
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Who handled it
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Edits flows
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Edits transactions
                    </th>
                    <th scope="col" className="py-3 pr-3 font-semibold">
                      Manages people
                    </th>
                    <th
                      scope="col"
                      className="py-3 pr-3 text-right font-semibold"
                    >
                      Members
                    </th>
                    <th scope="col" className="py-3">
                      <span className="sr-only">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody className="[&:has(>tr:hover)>tr:not(:hover)]:opacity-55">
                  {list.map((group) => (
                    <GroupRow
                      key={group.id}
                      group={group}
                      selected={selected?.id === group.id}
                      dimmed={!!selectedId && selected?.id !== group.id}
                      onSelect={() => setSelectedId(group.id)}
                      onEdit={() => setEditing(group)}
                      onDelete={() => {
                        deleteGroup.reset()
                        setDeleting(group)
                      }}
                    />
                  ))}
                </tbody>
              </table>
            </div>
            <p className="mt-3 text-xs text-muted-foreground">
              Proofs: “Originals” means the receipts and documents themselves;
              “Summaries” means type, date and status only. Default groups can
              be edited but not deleted.
            </p>
          </>
        )}
      </ReportSection>

      {selected && <MembersSection group={selected} />}

      <GroupFormDialog
        key={editing === 'new' ? 'new' : (editing?.id ?? 'closed')}
        open={!!editing}
        group={editing === 'new' ? null : editing}
        existing={list}
        onClose={() => setEditing(null)}
        onSaved={(id) => setSelectedId(id)}
      />

      <AlertDialog
        open={!!deleting}
        onOpenChange={(open) => !open && setDeleting(null)}
      >
        <AlertDialogContent
          aria-modal="true"
          className="border-t-[3px] border-foreground"
        >
          <AlertDialogHeader>
            <AlertDialogTitle className="text-xl">
              Delete “{deleting?.name}”?
            </AlertDialogTitle>
            <AlertDialogDescription className="text-sm">
              Everyone in this group loses access straight away, and any chain
              link that granted this group stops showing your flows. This can’t
              be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          {deleteGroup.isError && (
            <p role="alert" className="text-sm text-destructive">
              Not deleted. {ERROR_TEXT(deleteGroup.error)}
            </p>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel className="h-10">Keep group</AlertDialogCancel>
            <Button
              variant="destructive"
              className="h-10"
              disabled={deleteGroup.isPending}
              onClick={() => {
                if (!deleting) return
                deleteGroup.mutate(deleting.id, {
                  onSuccess: () => {
                    if (selectedId === deleting.id) setSelectedId(null)
                    setDeleting(null)
                  },
                })
              }}
            >
              {deleteGroup.isPending && (
                <CircleNotch className="animate-spin" aria-hidden="true" />
              )}
              Delete group
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

function YesNo({ value }: { value: boolean }) {
  return value ? (
    <Check weight="bold" className="size-4" aria-label="Yes" role="img" />
  ) : (
    <Minus
      weight="bold"
      className="size-4 text-muted-foreground opacity-60"
      aria-label="No"
      role="img"
    />
  )
}

function GroupRow({
  group,
  selected,
  dimmed,
  onSelect,
  onEdit,
  onDelete,
}: {
  group: AccessGroup
  selected: boolean
  dimmed: boolean
  onSelect: () => void
  onEdit: () => void
  onDelete: () => void
}) {
  const members = useGroupMembers(group.id)
  const p = group.permissions
  const depth = proofDepth(p)

  return (
    <tr
      className={cn(
        'border-b border-border align-top transition-opacity duration-200',
        selected && 'bg-accent/60',
        dimmed && 'opacity-70',
      )}
    >
      <th scope="row" className="py-3 pr-4 font-normal">
        <button
          type="button"
          onClick={onSelect}
          aria-pressed={selected}
          className="flex min-h-10 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <span
            className={cn(
              'text-base font-semibold',
              selected && 'text-primary',
            )}
          >
            {group.name}
          </span>
          <span className="text-xs text-muted-foreground">
            {[group.is_default && 'Default', group.is_public && 'Public']
              .filter(Boolean)
              .join(' · ') || 'Custom'}
          </span>
        </button>
      </th>
      <td className="py-3 pr-4">
        {!p.can_view_nodes
          ? 'Nothing'
          : p.can_view_amounts
            ? SCOPE_LABEL[group.node_scope]
            : `${SCOPE_LABEL[group.node_scope]}, names only`}
      </td>
      <td className="py-3 pr-3">
        <YesNo value={p.can_view_amounts} />
      </td>
      <td className="py-3 pr-3">
        {depth === 'full' ? (
          'Originals'
        ) : depth === 'summary' ? (
          'Summaries'
        ) : (
          <YesNo value={false} />
        )}
      </td>
      <td className="py-3 pr-3">
        <YesNo value={p.can_view_executor} />
      </td>
      <td className="py-3 pr-3">
        <YesNo value={p.can_edit_nodes} />
      </td>
      <td className="py-3 pr-3">
        <YesNo value={p.can_edit_transactions} />
      </td>
      <td className="py-3 pr-3">
        {p.can_manage_groups ? (
          'Members and groups'
        ) : p.can_manage_members ? (
          'Members'
        ) : (
          <YesNo value={false} />
        )}
      </td>
      <td className="py-3 pr-3 text-right tabular-nums">
        {members.data ? members.data.length : members.isError ? '?' : '–'}
      </td>
      <td className="py-2">
        <div className="flex justify-end gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="size-10"
            onClick={onEdit}
            aria-label={`Edit ${group.name}`}
          >
            <PencilSimple />
          </Button>
          {!group.is_default ? (
            <Button
              variant="ghost"
              size="icon"
              className="size-10 text-destructive"
              onClick={onDelete}
              aria-label={`Delete ${group.name}`}
            >
              <Trash />
            </Button>
          ) : (
            <span aria-hidden="true" className="size-10" />
          )}
        </div>
      </td>
    </tr>
  )
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

function todayISO() {
  return localTodayISO()
}

type MemberRow = NonNullable<ReturnType<typeof useGroupMembers>['data']>[number]

/** Adds someone who is already a member of the organisation, picked by name. */
function AddMemberForm({
  group,
  members,
}: {
  group: AccessGroup
  members: Array<MemberRow>
}) {
  const people = useOrgMembers()
  const add = useAddGroupMember()
  const [chosen, setChosen] = useState('')
  const inGroup = new Set(members.map((m) => m.user_id).filter(Boolean))
  const open = (people.data ?? []).filter((p) => !inGroup.has(p.user_id))
  const pick = open.find((p) => p.user_id === chosen) ?? open.at(0) ?? null

  useEffect(() => {
    setChosen('')
    add.reset()
  }, [group.id])

  if (people.isLoading || !people.data || people.data.length === 0) return null

  return (
    <form
      noValidate
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault()
        if (!pick) return
        add.mutate({
          group_id: group.id,
          user_id: pick.user_id,
          user_email: null,
          granted_by: null,
          valid_from: new Date().toISOString(),
          valid_until: null,
          notes: null,
        })
      }}
    >
      <h3 className="text-xl">Add a member</h3>
      {open.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          Everyone in your organisation is already in {group.name}.
        </p>
      ) : (
        <>
          <div className="flex flex-col gap-2">
            <span id="add-member-label" className="text-sm font-semibold">
              Person
            </span>
            <Select
              value={pick?.user_id ?? ''}
              items={Object.fromEntries(open.map((p) => [p.user_id, p.name]))}
              onValueChange={(v) => {
                if (v) setChosen(v)
                add.reset()
              }}
            >
              <SelectTrigger
                className="h-10 w-full"
                aria-labelledby="add-member-label"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {open.map((p) => (
                  <SelectItem key={p.user_id} value={p.user_id}>
                    {p.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" className="h-10" disabled={add.isPending}>
              {add.isPending ? (
                <CircleNotch className="animate-spin" aria-hidden="true" />
              ) : (
                <UserPlus aria-hidden="true" />
              )}
              {add.isPending ? 'Adding' : `Add to ${group.name}`}
            </Button>
          </div>
        </>
      )}
      <p aria-live="polite" className="text-sm">
        {add.isError ? (
          <span className="text-destructive">
            Not added. {ERROR_TEXT(add.error)}
          </span>
        ) : add.isSuccess ? (
          <span className="text-muted-foreground">Added to {group.name}.</span>
        ) : null}
      </p>
    </form>
  )
}

function MembersSection({ group }: { group: AccessGroup }) {
  const members = useGroupMembers(group.id)
  const invite = useInviteMemberByEmail()
  const removeMember = useRemoveGroupMember()
  const { profile, user } = useOrg()

  const [email, setEmail] = useState('')
  const [until, setUntil] = useState('')
  const [touched, setTouched] = useState(false)
  const [lastInvited, setLastInvited] = useState<string | null>(null)
  const [removing, setRemoving] = useState<MemberRow | null>(null)

  useEffect(() => {
    setEmail('')
    setUntil('')
    setTouched(false)
    setLastInvited(null)
    invite.reset()
  }, [group.id])

  const emailError = !email.trim()
    ? 'Enter an email address.'
    : !EMAIL_PATTERN.test(email.trim())
      ? 'That doesn’t look like an email address. Check it and try again.'
      : (members.data ?? []).some(
            (m) =>
              (m.user_email ?? m.profile?.email)?.toLowerCase() ===
              email.trim().toLowerCase(),
          )
        ? 'This person is already in the group.'
        : null
  const untilParsed = parseDayMonthYear(until)
  const untilISO = untilParsed?.iso ?? ''
  const untilError = untilParsed?.error
    ? untilParsed.error
    : untilISO && untilISO < todayISO()
      ? 'That date has passed. Choose today or a later day.'
      : null

  const onInvite = (e: React.FormEvent) => {
    e.preventDefault()
    setTouched(true)
    if (emailError || untilError) return
    const address = email.trim()
    invite.mutate(
      {
        groupId: group.id,
        email: address,
        grantedBy: profile?.id ?? user?.id ?? 'unknown-user',
        validUntil: untilISO ? `${untilISO}T23:59:59Z` : undefined,
      },
      {
        onSuccess: () => {
          setEmail('')
          setUntil('')
          setTouched(false)
          setLastInvited(address)
        },
      },
    )
  }

  const now = new Date().toISOString()
  const list = members.data ?? []

  return (
    <ReportSection
      title={`Members of ${group.name}`}
      meta={
        members.data ? (
          <span className="tabular-nums">
            {list.length} {list.length === 1 ? 'member' : 'members'}
          </span>
        ) : undefined
      }
    >
      <div className="grid gap-8 pt-2 lg:grid-cols-12">
        <div className="lg:col-span-7">
          {members.isError ? (
            <ReportNotice
              tone="error"
              title="Members could not be loaded"
              action={
                <Button
                  variant="outline"
                  className="h-10"
                  onClick={() => members.refetch()}
                >
                  Try again
                </Button>
              }
            >
              {ERROR_TEXT(members.error)}
            </ReportNotice>
          ) : members.isLoading ? (
            <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
              <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
              Loading members
            </p>
          ) : list.length === 0 ? (
            <ReportNotice title="No one is in this group yet">
              Add a member of your organisation, or invite someone by email.
              An invite is recorded here; no email is sent.
            </ReportNotice>
          ) : (
            <ul className="divide-y divide-border">
              {list.map((m) => {
                const expired = !!m.valid_until && m.valid_until < now
                const limited = !!m.valid_until && !expired
                return (
                  <li
                    key={m.id}
                    className={cn(
                      'flex flex-wrap items-center justify-between gap-x-4 gap-y-2 py-3',
                      expired && 'text-muted-foreground',
                    )}
                  >
                    <div className="min-w-0">
                      <p className="font-semibold [overflow-wrap:anywhere]">
                        {m.profile?.full_name ?? m.user_email}
                      </p>
                      <p className="text-xs text-muted-foreground [overflow-wrap:anywhere]">
                        {m.profile?.full_name &&
                        (m.profile.email || m.user_email)
                          ? `${m.profile.email || m.user_email} · `
                          : ''}
                        {m.user_id ? 'Member' : 'Invited'} since{' '}
                        {formatDate(m.valid_from)}
                      </p>
                      {m.notes && (
                        <p className="mt-0.5 text-xs text-muted-foreground">
                          {m.notes}
                        </p>
                      )}
                    </div>
                    <div className="flex items-center gap-3">
                      <span
                        className={cn(
                          'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
                          !limited && !expired && 'text-foreground',
                          (limited || expired) && 'text-muted-foreground',
                        )}
                      >
                        <ChainLine
                          status={
                            expired ? 'revoked' : limited ? 'pending' : 'active'
                          }
                          className="w-6"
                        />
                        {expired
                          ? `Expired ${formatDate(m.valid_until!)}`
                          : limited
                            ? `Until ${formatDate(m.valid_until!)}`
                            : 'No end date'}
                      </span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-10 text-destructive"
                        aria-label={`Remove ${m.profile?.full_name ?? m.user_email ?? 'member'}`}
                        onClick={() => {
                          removeMember.reset()
                          setRemoving(m)
                        }}
                      >
                        <Trash />
                      </Button>
                    </div>
                  </li>
                )
              })}
            </ul>
          )}
          <div className="mt-4 flex flex-wrap gap-x-6 gap-y-2 text-xs text-muted-foreground">
            <span className="inline-flex items-center gap-2">
              <ChainLine status="active" className="w-7" /> No end date
            </span>
            <span className="inline-flex items-center gap-2">
              <ChainLine status="pending" className="w-7" /> Time-limited
            </span>
            <span className="inline-flex items-center gap-2">
              <ChainLine status="revoked" className="w-7" /> Expired, no access
            </span>
          </div>
        </div>

        <div className="flex flex-col gap-8 border-t border-border pt-5 lg:col-span-5 lg:border-t-0 lg:border-l lg:pt-0 lg:pl-8">
        <AddMemberForm group={group} members={list} />
        <form
          onSubmit={onInvite}
          noValidate
          className="flex flex-col gap-4"
        >
          <h3 className="text-xl">Invite someone</h3>
          <div className="flex flex-col gap-2">
            <label htmlFor="invite-email" className="text-sm font-semibold">
              Email
            </label>
            <Input
              id="invite-email"
              type="email"
              autoComplete="email"
              placeholder="name@organisation.org"
              value={email}
              onChange={(e) => {
                setEmail(e.target.value)
                setLastInvited(null)
              }}
              className="h-10"
              aria-invalid={(touched && !!emailError) || undefined}
              aria-describedby="invite-email-msg"
            />
            {touched && emailError && (
              <p id="invite-email-msg" className="text-xs text-destructive">
                {emailError}
              </p>
            )}
          </div>
          <div className="flex flex-col gap-2">
            <label htmlFor="invite-until" className="text-sm font-semibold">
              Access ends{' '}
              <span className="font-normal text-muted-foreground">
                (optional)
              </span>
            </label>
            <DateField
              id="invite-until"
              min={todayISO()}
              value={until}
              onValueChange={setUntil}
              pickerLabel="Choose the end date from a calendar"
              className="h-10"
              wrapperClassName="sm:w-56"
              aria-invalid={(touched && !!untilError) || undefined}
              aria-describedby="invite-until-msg"
            />
            <p
              id="invite-until-msg"
              className={cn(
                'text-xs',
                touched && untilError
                  ? 'text-destructive'
                  : 'text-muted-foreground',
              )}
            >
              {touched && untilError
                ? untilError
                : 'Day, month, year. For auditors and short reviews; access stops at the end of that day.'}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" className="h-10" disabled={invite.isPending}>
              {invite.isPending ? (
                <CircleNotch className="animate-spin" aria-hidden="true" />
              ) : (
                <UserPlus aria-hidden="true" />
              )}
              {invite.isPending ? 'Inviting' : `Invite to ${group.name}`}
            </Button>
          </div>
          <p aria-live="polite" className="text-sm">
            {invite.isError ? (
              <span className="text-destructive">
                Invite not sent. {ERROR_TEXT(invite.error)}
              </span>
            ) : lastInvited ? (
              <span className="text-muted-foreground">
                {lastInvited} is invited to {group.name}.
              </span>
            ) : null}
          </p>
        </form>
        </div>
      </div>

      <AlertDialog
        open={!!removing}
        onOpenChange={(open) => !open && setRemoving(null)}
      >
        <AlertDialogContent
          aria-modal="true"
          className="border-t-[3px] border-foreground"
        >
          <AlertDialogHeader>
            <AlertDialogTitle className="text-xl">
              Remove {removing?.profile?.full_name ?? removing?.user_email} from{' '}
              {group.name}?
            </AlertDialogTitle>
            <AlertDialogDescription className="text-sm">
              They lose this group’s access straight away. You can invite them
              again later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          {removeMember.isError && (
            <p role="alert" className="text-sm text-destructive">
              Not removed. {ERROR_TEXT(removeMember.error)}
            </p>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel className="h-10">Keep member</AlertDialogCancel>
            <Button
              variant="destructive"
              className="h-10"
              disabled={removeMember.isPending}
              onClick={() => {
                if (!removing) return
                removeMember.mutate(
                  { id: removing.id, groupId: group.id },
                  { onSuccess: () => setRemoving(null) },
                )
              }}
            >
              Remove member
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ReportSection>
  )
}

// ---------------------------------------------------------------------------
// Create / edit group
// ---------------------------------------------------------------------------

const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/
const slugify = (s: string) =>
  s
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')

const EMPTY_PERMISSIONS: AccessGroupPermissions = {
  can_view_nodes: true,
  can_view_amounts: true,
  can_view_proofs: false,
  can_view_executor: false,
  can_view_full_proofs: false,
  can_edit_nodes: false,
  can_edit_transactions: false,
  can_manage_members: false,
  can_manage_groups: false,
}

type FlagKey = Exclude<
  keyof AccessGroupPermissions,
  'can_view_proofs' | 'can_view_full_proofs'
>

const SEE_FLAGS: Array<[FlagKey, string, string]> = [
  ['can_view_nodes', 'The flow', 'Nodes and how money moves between them.'],
  ['can_view_amounts', 'Amounts', 'The KES value of each movement.'],
  [
    'can_view_executor',
    'Who handled it',
    'The staff member or vendor behind each payment.',
  ],
]

const DO_FLAGS: Array<[FlagKey, string, string]> = [
  ['can_edit_nodes', 'Edit flows', 'Add and change nodes.'],
  [
    'can_edit_transactions',
    'Edit transactions',
    'Record transactions and attach proof.',
  ],
  ['can_manage_members', 'Manage members', 'Invite and remove people.'],
  ['can_manage_groups', 'Manage groups', 'Create groups and change access.'],
]

function GroupFormDialog({
  open,
  group,
  existing,
  onClose,
  onSaved,
}: {
  open: boolean
  group: AccessGroup | null
  existing: Array<AccessGroup>
  onClose: () => void
  onSaved: (id: string) => void
}) {
  const create = useCreateAccessGroup()
  const update = useUpdateAccessGroup()
  const mutation = group ? update : create
  const nameRef = useRef<HTMLInputElement>(null)

  const [name, setName] = useState(group?.name ?? '')
  const [slug, setSlug] = useState(group?.slug ?? '')
  const [slugEdited, setSlugEdited] = useState(!!group)
  const [description, setDescription] = useState(group?.description ?? '')
  const [scope, setScope] = useState<AccessGroup['node_scope']>(
    group?.node_scope ?? 'assigned',
  )
  const [perms, setPerms] = useState<AccessGroupPermissions>(
    group?.permissions ?? EMPTY_PERMISSIONS,
  )
  const [submitted, setSubmitted] = useState(false)

  const others = existing.filter((g) => g.id !== group?.id)
  const errors: { name?: string; slug?: string } = {}
  if (name.trim().length < 2) errors.name = 'Give the group a name.'
  else if (
    others.some((g) => g.name.toLowerCase() === name.trim().toLowerCase())
  )
    errors.name = 'Another group already has this name.'
  if (!slug) errors.slug = 'Enter a short ID.'
  else if (!SLUG_PATTERN.test(slug))
    errors.slug = 'Use lowercase letters, numbers and single hyphens.'
  else if (others.some((g) => g.slug === slug))
    errors.slug = 'Another group already uses this ID.'

  const depth = proofDepth(perms)
  const setDepth = (d: ProofDepth) =>
    setPerms((p) => ({
      ...p,
      can_view_proofs: d !== 'none',
      can_view_full_proofs: d === 'full',
    }))

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitted(true)
    if (errors.name || errors.slug) {
      if (errors.name) nameRef.current?.focus()
      return
    }
    const values = {
      name: name.trim(),
      slug,
      description: description.trim() || null,
      node_scope: scope,
      permissions: perms,
    }
    if (group) {
      update.mutate(
        { id: group.id, updates: values },
        {
          onSuccess: () => {
            onSaved(group.id)
            onClose()
          },
        },
      )
    } else {
      create.mutate(
        {
          ...values,
          icon: 'users',
          color: '#1C5FA8',
          is_default: false,
          is_public: false,
          priority: 50,
        },
        {
          onSuccess: (row) => {
            onSaved(row.id)
            onClose()
          },
        },
      )
    }
  }

  const flag = ([key, label, hint]: [FlagKey, string, string]) => (
    <label
      key={key}
      className="flex min-h-10 cursor-pointer items-start gap-3 py-1.5"
    >
      <ReportCheckbox
        checked={perms[key]}
        onCheckedChange={(v) => setPerms((p) => ({ ...p, [key]: v }))}
        className="mt-0.5"
      />
      <span>
        <span className="block text-sm font-semibold">{label}</span>
        <span className="block text-xs text-muted-foreground">{hint}</span>
      </span>
    </label>
  )

  return (
    <ReportDialog
      open={open}
      onOpenChange={(o) => !o && onClose()}
      title={group ? `Edit ${group.name}` : 'Create an access group'}
      description={
        group?.is_default
          ? 'A default group. You can change its depth, but not delete it.'
          : 'Set what this group sees. You can change it at any time.'
      }
      initialFocus={nameRef}
      footer={
        <>
          <Button variant="outline" className="h-10" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form="group-form"
            className="h-10"
            disabled={mutation.isPending}
          >
            {mutation.isPending && (
              <CircleNotch className="animate-spin" aria-hidden="true" />
            )}
            {group
              ? mutation.isPending
                ? 'Saving'
                : 'Save group'
              : mutation.isPending
                ? 'Creating'
                : 'Create group'}
          </Button>
        </>
      }
    >
      <form
        id="group-form"
        onSubmit={onSubmit}
        noValidate
        className="flex flex-col gap-5"
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-2">
            <label htmlFor="group-name" className="text-sm font-semibold">
              Name
            </label>
            <Input
              id="group-name"
              ref={nameRef}
              value={name}
              placeholder="e.g. Harambee grant team"
              onChange={(e) => {
                setName(e.target.value)
                if (!slugEdited) setSlug(slugify(e.target.value))
              }}
              className="h-10"
              aria-invalid={(submitted && !!errors.name) || undefined}
              aria-describedby="group-name-msg"
            />
            {submitted && errors.name && (
              <p id="group-name-msg" className="text-xs text-destructive">
                {errors.name}
              </p>
            )}
          </div>
          <div className="flex flex-col gap-2">
            <label htmlFor="group-slug" className="text-sm font-semibold">
              Short ID
            </label>
            <Input
              id="group-slug"
              value={slug}
              disabled={!!group?.is_default}
              onChange={(e) => {
                setSlug(e.target.value.toLowerCase())
                setSlugEdited(true)
              }}
              className="h-10 font-mono"
              spellCheck={false}
              autoCapitalize="none"
              aria-invalid={(submitted && !!errors.slug) || undefined}
              aria-describedby="group-slug-msg"
            />
            <p
              id="group-slug-msg"
              className={cn(
                'text-xs',
                submitted && errors.slug
                  ? 'text-destructive'
                  : 'text-muted-foreground',
              )}
            >
              {submitted && errors.slug
                ? errors.slug
                : 'Used in links and the API.'}
            </p>
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <label htmlFor="group-description" className="text-sm font-semibold">
            Description{' '}
            <span className="font-normal text-muted-foreground">
              (optional)
            </span>
          </label>
          <Textarea
            id="group-description"
            rows={2}
            value={description}
            placeholder="Who is in this group and why"
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>

        <fieldset className="flex flex-col gap-1 border-t border-border pt-4">
          <legend className="pt-0 text-sm font-semibold">What they see</legend>
          <div className="mt-2 grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-2">
              <span
                id="group-scope-label"
                className="text-xs text-muted-foreground"
              >
                Which flows
              </span>
              <Select
                value={scope}
                items={SCOPE_LABEL}
                onValueChange={(v) => v && setScope(v)}
              >
                <SelectTrigger
                  className="h-10 w-full"
                  aria-labelledby="group-scope-label"
                  aria-describedby="group-scope-hint"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(
                    Object.keys(SCOPE_LABEL) as Array<AccessGroup['node_scope']>
                  ).map((s) => (
                    <SelectItem key={s} value={s}>
                      {SCOPE_LABEL[s]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p
                id="group-scope-hint"
                className="text-xs text-muted-foreground"
              >
                {SCOPE_HINT[scope]}
              </p>
            </div>
            <div className="flex flex-col gap-2">
              <span
                id="group-proofs-label"
                className="text-xs text-muted-foreground"
              >
                Proofs
              </span>
              <Select
                value={depth}
                items={{
                  none: 'No proofs',
                  summary: 'Summaries',
                  full: 'Originals',
                }}
                onValueChange={(v) => v && setDepth(v as ProofDepth)}
              >
                <SelectTrigger
                  className="h-10 w-full"
                  aria-labelledby="group-proofs-label"
                  aria-describedby="group-proofs-hint"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No proofs</SelectItem>
                  <SelectItem value="summary">Summaries</SelectItem>
                  <SelectItem value="full">Originals</SelectItem>
                </SelectContent>
              </Select>
              <p
                id="group-proofs-hint"
                className="text-xs text-muted-foreground"
              >
                {depth === 'full'
                  ? 'Receipts, photos and documents themselves.'
                  : depth === 'summary'
                    ? 'Type, date and status of each proof.'
                    : 'They see no proof attached to payments.'}
              </p>
            </div>
          </div>
          <div className="mt-2 grid sm:grid-cols-2 sm:gap-x-4">
            {SEE_FLAGS.map(flag)}
          </div>
        </fieldset>

        <fieldset className="flex flex-col gap-1 border-t border-border pt-4">
          <legend className="pt-0 text-sm font-semibold">
            What they can do
          </legend>
          <div className="mt-2 grid sm:grid-cols-2 sm:gap-x-4">
            {DO_FLAGS.map(flag)}
          </div>
        </fieldset>

        {mutation.isError && (
          <p role="alert" className="text-sm text-destructive">
            Not saved. {ERROR_TEXT(mutation.error)}
          </p>
        )}
      </form>
    </ReportDialog>
  )
}
