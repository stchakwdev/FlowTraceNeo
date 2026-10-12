import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import {
  Globe,
  Handshake,
  Heart,
  Plus,
  ShieldCheck,
  Trash,
  Users,
  UserPlus,
  PencilSimple,
  X,
  CaretDown,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  useAccessGroups,
  useGroupMembers,
  useCreateAccessGroup,
  useDeleteAccessGroup,
  useInviteMemberByEmail,
  useRemoveGroupMember,
} from '@/lib/queries'
import type { AccessGroup } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/settings/access-groups')({
  component: AccessGroupsPage,
})

const iconMap: Record<string, React.ElementType> = {
  'shield-check': ShieldCheck,
  users: Users,
  heart: Heart,
  handshake: Handshake,
  globe: Globe,
}

function AccessGroupsPage() {
  const { data: groups, isLoading } = useAccessGroups()
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)

  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-primary/70 shadow-lg shadow-primary/20">
              <Users className="h-6 w-6 text-primary-foreground" weight="bold" />
            </div>
            <div>
              <h1 className="text-3xl font-bold">Access Groups</h1>
              <p className="text-muted-foreground">
                Manage who can see and interact with your financial data
              </p>
            </div>
          </div>
          <Button onClick={() => setShowCreateModal(true)}>
            <Plus className="h-4 w-4" />
            Create Group
          </Button>
        </div>
      </div>

      {/* Groups Grid */}
      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {[...Array(5)].map((_, i) => (
            <Card key={i} className="p-6">
              <div className="flex items-start gap-4">
                <div className="h-10 w-10 skeleton-shimmer rounded-lg bg-muted" />
                <div className="flex-1 space-y-2">
                  <div className="h-5 w-24 skeleton-shimmer rounded bg-muted" />
                  <div className="h-4 w-full skeleton-shimmer rounded bg-muted" />
                </div>
              </div>
            </Card>
          ))}
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {groups?.map((group) => (
            <GroupCard
              key={group.id}
              group={group}
              isSelected={selectedGroup === group.id}
              onSelect={() => setSelectedGroup(selectedGroup === group.id ? null : group.id)}
            />
          ))}
        </div>
      )}

      {/* Selected Group Details */}
      {selectedGroup && (
        <GroupDetails
          groupId={selectedGroup}
          onClose={() => setSelectedGroup(null)}
        />
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <CreateGroupModal onClose={() => setShowCreateModal(false)} />
      )}
    </div>
  )
}

function GroupCard({
  group,
  isSelected,
  onSelect,
}: {
  group: AccessGroup
  isSelected: boolean
  onSelect: () => void
}) {
  const Icon = iconMap[group.icon] || Users
  const deleteGroup = useDeleteAccessGroup()

  const permissionCount = Object.values(group.permissions).filter(Boolean).length

  return (
    <Card
      className={`group relative cursor-pointer overflow-hidden p-6 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5 ${
        isSelected ? 'border-primary ring-2 ring-primary/20' : ''
      }`}
      onClick={onSelect}
    >
      {/* Gradient overlay */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

      <div className="relative">
        <div className="flex items-start justify-between">
          <div className="flex items-start gap-3">
            <div
              className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg"
              style={{ backgroundColor: `${group.color}20` }}
            >
              <Icon className="h-5 w-5" style={{ color: group.color }} weight="duotone" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="font-semibold">{group.name}</h3>
                {group.is_default && (
                  <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                    Default
                  </span>
                )}
                {group.is_public && (
                  <span className="rounded-full bg-violet-100 px-2 py-0.5 text-xs text-violet-700 dark:bg-violet-900/30 dark:text-violet-400">
                    Public
                  </span>
                )}
              </div>
              <p className="mt-1 text-sm text-muted-foreground line-clamp-2">
                {group.description || 'No description'}
              </p>
            </div>
          </div>

          {!group.is_default && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 opacity-0 transition-opacity group-hover:opacity-100"
              onClick={(e) => {
                e.stopPropagation()
                if (confirm('Delete this group?')) {
                  deleteGroup.mutate(group.id)
                }
              }}
            >
              <Trash className="h-4 w-4 text-destructive" />
            </Button>
          )}
        </div>

        <div className="mt-4 flex items-center gap-4 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <ShieldCheck className="h-3.5 w-3.5" />
            {permissionCount} permissions
          </span>
          <span className="flex items-center gap-1">
            <Users className="h-3.5 w-3.5" />
            {group.node_scope} scope
          </span>
        </div>
      </div>
    </Card>
  )
}

function GroupDetails({
  groupId,
  onClose,
}: {
  groupId: string
  onClose: () => void
}) {
  const { data: groups } = useAccessGroups()
  const { data: members, isLoading: membersLoading } = useGroupMembers(groupId)
  const inviteMember = useInviteMemberByEmail()
  const removeMember = useRemoveGroupMember()

  const [showInvite, setShowInvite] = useState(false)
  const [inviteEmail, setInviteEmail] = useState('')

  const group = groups?.find((g) => g.id === groupId)
  if (!group) return null

  const Icon = iconMap[group.icon] || Users

  const handleInvite = () => {
    if (!inviteEmail) return
    inviteMember.mutate(
      { groupId, email: inviteEmail, grantedBy: 'dev-user' },
      {
        onSuccess: () => {
          setInviteEmail('')
          setShowInvite(false)
        },
      }
    )
  }

  return (
    <Card className="mt-6 p-6">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div
            className="flex h-12 w-12 items-center justify-center rounded-xl"
            style={{ backgroundColor: `${group.color}20` }}
          >
            <Icon className="h-6 w-6" style={{ color: group.color }} weight="duotone" />
          </div>
          <div>
            <h2 className="text-xl font-semibold">{group.name}</h2>
            <p className="text-sm text-muted-foreground">{group.description}</p>
          </div>
        </div>
        <Button variant="ghost" size="icon" onClick={onClose}>
          <X className="h-5 w-5" />
        </Button>
      </div>

      {/* Permissions */}
      <div className="mt-6">
        <h3 className="mb-3 font-medium">Permissions</h3>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {Object.entries(group.permissions).map(([key, value]) => (
            <div
              key={key}
              className={`flex items-center gap-2 rounded-lg border px-3 py-2 text-sm ${
                value
                  ? 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900/50 dark:bg-emerald-900/20 dark:text-emerald-400'
                  : 'border-border bg-muted/50 text-muted-foreground'
              }`}
            >
              {value ? (
                <ShieldCheck className="h-4 w-4 shrink-0" weight="fill" />
              ) : (
                <X className="h-4 w-4 shrink-0" />
              )}
              <span className="truncate">
                {key.replace(/_/g, ' ').replace('can ', '')}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Members */}
      <div className="mt-6">
        <div className="mb-3 flex items-center justify-between">
          <h3 className="font-medium">Members</h3>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowInvite(!showInvite)}
          >
            <UserPlus className="h-4 w-4" />
            Invite Member
          </Button>
        </div>

        {showInvite && (
          <div className="mb-4 flex gap-2">
            <Input
              type="email"
              placeholder="Enter email address"
              value={inviteEmail}
              onChange={(e) => setInviteEmail(e.target.value)}
              className="flex-1"
            />
            <Button onClick={handleInvite} disabled={inviteMember.isPending}>
              {inviteMember.isPending ? 'Inviting...' : 'Send Invite'}
            </Button>
          </div>
        )}

        {membersLoading ? (
          <div className="space-y-2">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="flex items-center gap-3 rounded-lg border border-border p-3">
                <div className="h-8 w-8 skeleton-shimmer rounded-full bg-muted" />
                <div className="h-4 w-32 skeleton-shimmer rounded bg-muted" />
              </div>
            ))}
          </div>
        ) : members?.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border bg-muted/30 p-6 text-center">
            <Users className="mx-auto h-8 w-8 text-muted-foreground" />
            <p className="mt-2 text-sm text-muted-foreground">
              No members in this group yet
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {members?.map((member) => (
              <div
                key={member.id}
                className="flex items-center justify-between rounded-lg border border-border bg-card p-3"
              >
                <div className="flex items-center gap-3">
                  <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/10 text-sm font-medium text-primary">
                    {(member.profile?.full_name?.[0] || member.user_email?.[0] || '?').toUpperCase()}
                  </div>
                  <div>
                    <p className="text-sm font-medium">
                      {member.profile?.full_name || member.user_email}
                    </p>
                    {member.profile?.email && member.profile.email !== member.user_email && (
                      <p className="text-xs text-muted-foreground">{member.profile.email}</p>
                    )}
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8"
                  onClick={() => removeMember.mutate({ id: member.id, groupId })}
                >
                  <Trash className="h-4 w-4 text-destructive" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  )
}

function CreateGroupModal({ onClose }: { onClose: () => void }) {
  const createGroup = useCreateAccessGroup()
  const [name, setName] = useState('')
  const [slug, setSlug] = useState('')
  const [description, setDescription] = useState('')
  const [color, setColor] = useState('#6366f1')

  const handleCreate = () => {
    if (!name || !slug) return
    createGroup.mutate(
      {
        name,
        slug,
        description,
        color,
        icon: 'users',
        permissions: {
          can_view_nodes: true,
          can_view_amounts: true,
          can_view_proofs: false,
          can_view_executor: false,
          can_view_full_proofs: false,
          can_edit_nodes: false,
          can_edit_transactions: false,
          can_manage_members: false,
          can_manage_groups: false,
        },
        node_scope: 'assigned',
        is_default: false,
        is_public: false,
        priority: 50,
      },
      { onSuccess: onClose }
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <Card className="w-full max-w-md p-6">
        <div className="mb-6 flex items-center justify-between">
          <h2 className="text-xl font-semibold">Create Access Group</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-5 w-5" />
          </Button>
        </div>

        <div className="space-y-4">
          <div>
            <Label htmlFor="name">Group Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => {
                setName(e.target.value)
                setSlug(e.target.value.toLowerCase().replace(/\s+/g, '-'))
              }}
              placeholder="e.g., USAID Team"
            />
          </div>

          <div>
            <Label htmlFor="slug">Slug</Label>
            <Input
              id="slug"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="e.g., usaid-team"
            />
          </div>

          <div>
            <Label htmlFor="description">Description</Label>
            <Input
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What is this group for?"
            />
          </div>

          <div>
            <Label htmlFor="color">Color</Label>
            <div className="flex gap-2">
              <input
                type="color"
                id="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                className="h-10 w-14 cursor-pointer rounded border border-border"
              />
              <Input
                value={color}
                onChange={(e) => setColor(e.target.value)}
                className="flex-1"
              />
            </div>
          </div>
        </div>

        <div className="mt-6 flex gap-3">
          <Button variant="outline" className="flex-1" onClick={onClose}>
            Cancel
          </Button>
          <Button
            className="flex-1"
            onClick={handleCreate}
            disabled={!name || !slug || createGroup.isPending}
          >
            {createGroup.isPending ? 'Creating...' : 'Create Group'}
          </Button>
        </div>
      </Card>
    </div>
  )
}
