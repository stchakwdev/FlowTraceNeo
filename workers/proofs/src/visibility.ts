/**
 * Who sees how much of an organisation's proofs. Mirrors
 * `rust/crates/ledger-module/src/transparency/plan.rs:583-618` (`user_permissions`,
 * `visibility`, `caller_visibility`) and `flowtrace/src/lib/supabase.ts:573-588`
 * (`getVerificationVisibility`). Field names follow `transparency/types.rs:100-205`.
 */

/** `AccessGroupPermissions` (`types.rs:100-119`); only the flags read here are named. */
export interface AccessGroupPermissions {
  can_view_nodes?: boolean
  can_view_amounts?: boolean
  can_view_proofs?: boolean
  can_view_executor?: boolean
  can_view_full_proofs?: boolean
  can_edit_nodes?: boolean
  can_edit_transactions?: boolean
  can_manage_members?: boolean
  can_manage_groups?: boolean
}

/** `AccessGroup` (`types.rs:153-182`), as stored in `access_group.data`. */
export interface AccessGroup {
  id: string
  organization_id: string
  name: string
  permissions: AccessGroupPermissions
}

/** `GroupMember` (`types.rs:186-205`), as stored in `access_group_member.data`. */
export interface GroupMember {
  id: string
  group_id: string
  user_id: string | null
  user_email?: string | null
}

/** A `member` row (`rust/crates/ledger-module/src/tables.rs:57-77`). */
export interface Member {
  key: string
  identity: string
  org_id: string
  user_id: string
  name: string
  can_post: boolean
  can_approve: boolean
  can_close: boolean
}

export type Visibility = 'full' | 'thumbnail' | 'summary' | 'hidden'

const FLAGS = [
  'can_view_nodes',
  'can_view_amounts',
  'can_view_proofs',
  'can_view_executor',
  'can_view_full_proofs',
  'can_edit_nodes',
  'can_edit_transactions',
  'can_manage_members',
  'can_manage_groups',
] as const

type Perms = Record<(typeof FLAGS)[number], boolean>

function none(): Perms {
  return Object.fromEntries(FLAGS.map((f) => [f, false])) as Perms
}

/**
 * The union of every flag of every group the user is in (`plan.rs:583-594`). Membership is a
 * `GroupMember` with `group_id == group.id` and `user_id == userId`; dates are not checked.
 */
export function userPermissions(
  groups: AccessGroup[],
  groupMembers: GroupMember[],
  userId: string,
): Perms {
  const acc = none()
  for (const g of groups) {
    const inGroup = groupMembers.some((m) => m.group_id === g.id && m.user_id === userId)
    if (!inGroup) continue
    for (const f of FLAGS) acc[f] = acc[f] || g.permissions?.[f] === true
  }
  return acc
}

/** `visibility` (`plan.rs:599-609`). The proof's own `visibility_level` is ignored, like the TS. */
export function visibility(p: AccessGroupPermissions): Visibility {
  if (p.can_view_full_proofs) return 'full'
  if (p.can_view_proofs) return 'thumbnail'
  if (p.can_view_nodes) return 'summary'
  return 'hidden'
}

/** `caller_visibility` (`plan.rs:614-618`): hidden for a non-member. */
export function callerVisibility(
  groups: AccessGroup[],
  groupMembers: GroupMember[],
  member: Member | null,
): Visibility {
  if (!member) return 'hidden'
  return visibility(userPermissions(groups, groupMembers, member.user_id))
}
