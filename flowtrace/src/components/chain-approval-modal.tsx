import { useEffect, useState } from 'react'
import { CircleNotch } from '@phosphor-icons/react'
import type {
  AccessGroupPermissions,
  OrganizationChainWithDetails,
} from '@/lib/supabase'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ReportDialog } from '@/components/report-dialog'
import {
  useApproveChain,
  useChainAccessGroups,
  useNodes,
  useRejectChain,
} from '@/lib/queries'

interface ChainApprovalModalProps {
  chain: OrganizationChainWithDetails | null
  /** Approve grants an access group; reject declines, with an optional reason. */
  mode: 'approve' | 'reject'
  isOpen: boolean
  onClose: () => void
  approvedBy: string
}

/** One plain sentence for what a group's permissions let the other org see. */
export function describeAccess(p: AccessGroupPermissions) {
  const parts: Array<string> = []
  if (p.can_view_nodes) parts.push('the flow')
  if (p.can_view_amounts) parts.push('amounts')
  if (p.can_view_full_proofs) parts.push('original proofs')
  else if (p.can_view_proofs) parts.push('proof summaries')
  if (p.can_view_executor) parts.push('who handled each payment')
  if (parts.length === 0) return 'Sees nothing yet.'
  const list =
    parts.length === 1
      ? parts[0]
      : `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}`
  return `Sees ${list}.`
}

export function ChainApprovalModal({
  chain,
  mode,
  isOpen,
  onClose,
  approvedBy,
}: ChainApprovalModalProps) {
  const [selectedGroupId, setSelectedGroupId] = useState('')
  const [selectedNodeId, setSelectedNodeId] = useState('')
  const [rejectionReason, setRejectionReason] = useState('')
  const [showGroupError, setShowGroupError] = useState(false)

  const { data: accessGroups, isError: groupsFailed } = useChainAccessGroups()
  const { data: nodes } = useNodes()
  const approveChain = useApproveChain()
  const rejectChain = useRejectChain()

  useEffect(() => {
    if (!isOpen) {
      setSelectedGroupId('')
      setSelectedNodeId('')
      setRejectionReason('')
      setShowGroupError(false)
      approveChain.reset()
      rejectChain.reset()
    }
  }, [isOpen])

  if (!chain) return null

  const requester = chain.source_org
  const selectedGroup = accessGroups?.find((g) => g.id === selectedGroupId)
  const mutation = mode === 'approve' ? approveChain : rejectChain

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      if (mode === 'approve') {
        if (!selectedGroupId) {
          setShowGroupError(true)
          return
        }
        await approveChain.mutateAsync({
          chainId: chain.id,
          approvedBy,
          grantedGroupId: selectedGroupId,
          linkNodeId: selectedNodeId || undefined,
        })
      } else {
        await rejectChain.mutateAsync({
          chainId: chain.id,
          rejectedBy: approvedBy,
          reason: rejectionReason.trim() || undefined,
        })
      }
      onClose()
    } catch {
      // Shown below from mutation.error
    }
  }

  return (
    <ReportDialog
      open={isOpen}
      onOpenChange={(open) => !open && onClose()}
      title={
        mode === 'approve'
          ? `Approve ${requester.name}`
          : `Reject ${requester.name}`
      }
      description={
        mode === 'approve'
          ? 'Choose the access group they join. They see your flows at that group’s depth, and you can revoke the link at any time.'
          : 'They will see that you declined. They can send a new request later.'
      }
      footer={
        <>
          <Button variant="outline" onClick={onClose} className="h-10">
            Keep pending
          </Button>
          <Button
            type="submit"
            form="chain-review-form"
            variant={mode === 'approve' ? 'default' : 'destructive'}
            className="h-10"
            disabled={mutation.isPending}
          >
            {mutation.isPending && (
              <CircleNotch className="animate-spin" aria-hidden="true" />
            )}
            {mode === 'approve'
              ? mutation.isPending
                ? 'Approving'
                : 'Approve and grant access'
              : mutation.isPending
                ? 'Rejecting'
                : 'Reject request'}
          </Button>
        </>
      }
    >
      <form
        id="chain-review-form"
        onSubmit={handleSubmit}
        className="flex flex-col gap-5"
        noValidate
      >
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 border-y border-border py-3 text-sm">
          <dt className="text-muted-foreground">From</dt>
          <dd className="font-semibold">{requester.name}</dd>
          <dt className="text-muted-foreground">Trust Score</dt>
          <dd className="tabular-nums">{requester.trust_score} of 100</dd>
          {chain.request_message && (
            <>
              <dt className="text-muted-foreground">Their message</dt>
              <dd>{chain.request_message}</dd>
            </>
          )}
        </dl>

        {mode === 'approve' ? (
          <>
            <div className="flex flex-col gap-2">
              <span id="grant-group-label" className="text-sm font-semibold">
                Access group
              </span>
              <Select
                value={selectedGroupId}
                items={{
                  '': 'Choose a group',
                  ...Object.fromEntries(
                    (accessGroups ?? []).map((g) => [g.id, g.name]),
                  ),
                }}
                onValueChange={(value) => {
                  setSelectedGroupId(value ?? '')
                  setShowGroupError(false)
                }}
              >
                <SelectTrigger
                  className="h-10 w-full"
                  aria-labelledby="grant-group-label"
                  aria-invalid={showGroupError || undefined}
                  aria-describedby="grant-group-hint"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {accessGroups?.map((group) => (
                    <SelectItem key={group.id} value={group.id}>
                      {group.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p
                id="grant-group-hint"
                className={
                  showGroupError
                    ? 'text-xs text-destructive'
                    : 'text-xs text-muted-foreground'
                }
                role={showGroupError ? 'alert' : undefined}
              >
                {showGroupError
                  ? 'Choose an access group before approving.'
                  : groupsFailed
                    ? 'Your access groups could not be loaded. Close this and try again.'
                    : selectedGroup
                      ? describeAccess(selectedGroup.permissions)
                      : 'This sets what they can see in your flows.'}
              </p>
            </div>

            <div className="flex flex-col gap-2">
              <span id="link-node-label" className="text-sm font-semibold">
                Join at node{' '}
                <span className="font-normal text-muted-foreground">
                  (optional)
                </span>
              </span>
              <Select
                value={selectedNodeId}
                items={{
                  '': 'No specific node',
                  ...Object.fromEntries(
                    (nodes ?? []).map((node) => [node.id, node.name]),
                  ),
                }}
                onValueChange={(value) => setSelectedNodeId(value ?? '')}
              >
                <SelectTrigger
                  className="h-10 w-full"
                  aria-labelledby="link-node-label"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">No specific node</SelectItem>
                  {nodes?.map((node) => (
                    <SelectItem key={node.id} value={node.id}>
                      {node.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Where their flow joins yours, so a funder can follow the money
                across both.
              </p>
            </div>
          </>
        ) : (
          <div className="flex flex-col gap-2">
            <label htmlFor="reject-reason" className="text-sm font-semibold">
              Reason{' '}
              <span className="font-normal text-muted-foreground">
                (optional)
              </span>
            </label>
            <Textarea
              id="reject-reason"
              placeholder="For example: we only link with organisations we fund directly."
              value={rejectionReason}
              onChange={(e) => setRejectionReason(e.target.value)}
              rows={3}
            />
          </div>
        )}

        {mutation.isError && (
          <p role="alert" className="text-sm text-destructive">
            {mutation.error instanceof Error
              ? mutation.error.message
              : 'That didn’t go through. Try again in a moment.'}
          </p>
        )}
      </form>
    </ReportDialog>
  )
}
