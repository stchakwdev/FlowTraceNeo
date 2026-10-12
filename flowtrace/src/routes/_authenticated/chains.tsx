import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { CircleNotch, Plus } from '@phosphor-icons/react'
import type { OrganizationChainWithDetails } from '@/lib/supabase'
import { Button } from '@/components/ui/button'
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import {
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import {
  CHAIN_STATUS_LABEL,
  ChainLegend,
  ChainLine,
} from '@/components/chain-status-badge'
import { ChainRequestModal } from '@/components/chain-request-modal'
import {
  ChainApprovalModal,
  describeAccess,
} from '@/components/chain-approval-modal'
import {
  useCancelChainRequest,
  useChainTrustScore,
  useIncomingChains,
  useOutgoingChains,
  useRevokeChain,
} from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'

export const Route = createFileRoute('/_authenticated/chains')({
  component: ChainsPage,
})

type Direction = 'outgoing' | 'incoming'
type Confirm = {
  kind: 'revoke' | 'cancel'
  chain: OrganizationChainWithDetails
}

function ChainsPage() {
  const { organization, profile, user } = useOrg()
  const actorId = profile?.id ?? user?.id ?? 'unknown-user'

  const [isRequestOpen, setIsRequestOpen] = useState(false)
  const [review, setReview] = useState<{
    chain: OrganizationChainWithDetails
    mode: 'approve' | 'reject'
  } | null>(null)
  const [confirm, setConfirm] = useState<Confirm | null>(null)

  const outgoing = useOutgoingChains()
  const incoming = useIncomingChains()
  const cancelChain = useCancelChainRequest()
  const revokeChain = useRevokeChain()

  const all = [...(outgoing.data ?? []), ...(incoming.data ?? [])]
  const count = (pred: (c: OrganizationChainWithDetails) => boolean) =>
    all.filter(pred).length
  const loaded = !!outgoing.data && !!incoming.data

  const linkedOrgIds = (outgoing.data ?? [])
    .filter((c) => c.status === 'pending' || c.status === 'active')
    .map((c) => c.target_org_id)

  const confirmMutation = confirm?.kind === 'revoke' ? revokeChain : cancelChain

  const runConfirm = async () => {
    if (!confirm) return
    try {
      if (confirm.kind === 'revoke') {
        await revokeChain.mutateAsync({
          chainId: confirm.chain.id,
          revokedBy: actorId,
        })
      } else {
        await cancelChain.mutateAsync(confirm.chain.id)
      }
      setConfirm(null)
    } catch {
      // Shown in the dialog
    }
  }

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-10 px-4 py-6 sm:px-8 sm:py-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-4xl sm:text-5xl">Chain links</h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            {organization?.name ?? 'Your organisation'} and the partners whose
            flows are linked to yours. A funder can follow a grant along each
            active link, seeing only what that link grants.
          </p>
        </div>
        <Button
          className="h-10"
          onClick={() => {
            cancelChain.reset()
            setIsRequestOpen(true)
          }}
        >
          <Plus aria-hidden="true" />
          Request a link
        </Button>
      </header>

      <KeyFigures>
        <KeyFigure
          scale="lg"
          value={loaded ? count((c) => c.status === 'active') : '–'}
          label="Active links"
        />
        <KeyFigure
          value={
            loaded
              ? incoming.data.filter((c) => c.status === 'pending').length
              : '–'
          }
          label="Awaiting your review"
        />
        <KeyFigure
          value={
            loaded
              ? outgoing.data.filter((c) => c.status === 'pending').length
              : '–'
          }
          label="Your requests pending"
        />
        <KeyFigure
          scale="sm"
          value={
            loaded
              ? count((c) => c.status === 'revoked' || c.status === 'rejected')
              : '–'
          }
          label="Ended"
        />
      </KeyFigures>

      <ChainLegend />

      <ChainSection
        title="Incoming: who can see your flows"
        direction="incoming"
        query={incoming}
        emptyTitle="No one has asked to link with you yet"
        emptyBody="When a funder or partner requests a view of your flows, the request appears here for you to approve or reject."
        onApprove={(chain) => setReview({ chain, mode: 'approve' })}
        onReject={(chain) => setReview({ chain, mode: 'reject' })}
        onRevoke={(chain) => {
          revokeChain.reset()
          setConfirm({ kind: 'revoke', chain })
        }}
      />

      <ChainSection
        title="Outgoing: flows you follow"
        direction="outgoing"
        query={outgoing}
        emptyTitle="You don't follow any partner's flows yet"
        emptyBody="Request a link to see how an implementing partner spends the money you pass on."
        emptyAction={
          <Button className="h-10" onClick={() => setIsRequestOpen(true)}>
            <Plus aria-hidden="true" />
            Request a link
          </Button>
        }
        onCancel={(chain) => {
          cancelChain.reset()
          setConfirm({ kind: 'cancel', chain })
        }}
      />

      <ChainRequestModal
        isOpen={isRequestOpen}
        onClose={() => setIsRequestOpen(false)}
        requestedBy={actorId}
        linkedOrgIds={linkedOrgIds}
      />

      <ChainApprovalModal
        chain={review?.chain ?? null}
        mode={review?.mode ?? 'approve'}
        isOpen={!!review}
        onClose={() => setReview(null)}
        approvedBy={actorId}
      />

      <AlertDialog
        open={!!confirm}
        onOpenChange={(open) => !open && setConfirm(null)}
      >
        <AlertDialogContent
          aria-modal="true"
          className="border-t-[3px] border-foreground"
        >
          <AlertDialogHeader>
            <AlertDialogTitle className="text-xl">
              {confirm?.kind === 'revoke'
                ? `Revoke ${confirm.chain.source_org.name}'s access?`
                : `Cancel your request to ${confirm?.chain.target_org.name ?? ''}?`}
            </AlertDialogTitle>
            <AlertDialogDescription className="text-sm">
              {confirm?.kind === 'revoke'
                ? 'They stop seeing your flows straight away. The link stays on record as revoked, and they would need to send a new request.'
                : 'The request is withdrawn before they review it. You can send a new one later.'}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {confirmMutation.isError && (
            <p role="alert" className="text-sm text-destructive">
              {confirmMutation.error instanceof Error
                ? confirmMutation.error.message
                : 'That didn’t go through. Try again in a moment.'}
            </p>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel className="h-10">
              {confirm?.kind === 'revoke' ? 'Keep access' : 'Keep request'}
            </AlertDialogCancel>
            <Button
              variant="destructive"
              className="h-10"
              disabled={confirmMutation.isPending}
              onClick={runConfirm}
            >
              {confirmMutation.isPending && (
                <CircleNotch className="animate-spin" aria-hidden="true" />
              )}
              {confirm?.kind === 'revoke' ? 'Revoke access' : 'Cancel request'}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

function ChainSection({
  title,
  direction,
  query,
  emptyTitle,
  emptyBody,
  emptyAction,
  onApprove,
  onReject,
  onRevoke,
  onCancel,
}: {
  title: string
  direction: Direction
  query: ReturnType<typeof useOutgoingChains>
  emptyTitle: string
  emptyBody: string
  emptyAction?: React.ReactNode
  onApprove?: (c: OrganizationChainWithDetails) => void
  onReject?: (c: OrganizationChainWithDetails) => void
  onRevoke?: (c: OrganizationChainWithDetails) => void
  onCancel?: (c: OrganizationChainWithDetails) => void
}) {
  const chains = query.data ?? []
  // Pending first: they need a decision.
  const order = { pending: 0, active: 1, rejected: 2, revoked: 3 }
  const sorted = [...chains].sort((a, b) => order[a.status] - order[b.status])

  return (
    <ReportSection
      title={title}
      meta={
        query.data ? (
          <span className="tabular-nums">
            {chains.length} {chains.length === 1 ? 'link' : 'links'}
          </span>
        ) : undefined
      }
    >
      {query.isError ? (
        <ReportNotice
          tone="error"
          title="These links could not be loaded"
          action={
            <Button
              variant="outline"
              className="h-10"
              onClick={() => query.refetch()}
            >
              Try again
            </Button>
          }
        >
          {query.error instanceof Error
            ? query.error.message
            : 'Check your connection and try again.'}
        </ReportNotice>
      ) : query.isLoading ? (
        <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
          <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
          Loading links
        </p>
      ) : sorted.length === 0 ? (
        <ReportNotice title={emptyTitle} action={emptyAction}>
          {emptyBody}
        </ReportNotice>
      ) : (
        <ul className="divide-y divide-border [&:has(>li:hover)>li:not(:hover)]:opacity-55 [&:has(>li:focus-within)>li:not(:focus-within)]:opacity-55">
          {sorted.map((chain) => (
            <ChainRow
              key={chain.id}
              chain={chain}
              direction={direction}
              onApprove={onApprove}
              onReject={onReject}
              onRevoke={onRevoke}
              onCancel={onCancel}
            />
          ))}
        </ul>
      )}
    </ReportSection>
  )
}

function ChainRow({
  chain,
  direction,
  onApprove,
  onReject,
  onRevoke,
  onCancel,
}: {
  chain: OrganizationChainWithDetails
  direction: Direction
  onApprove?: (c: OrganizationChainWithDetails) => void
  onReject?: (c: OrganizationChainWithDetails) => void
  onRevoke?: (c: OrganizationChainWithDetails) => void
  onCancel?: (c: OrganizationChainWithDetails) => void
}) {
  const { data: score, isError: scoreFailed } = useChainTrustScore(chain.id)
  const ended = chain.status === 'revoked' || chain.status === 'rejected'

  const when =
    chain.status === 'pending'
      ? `Requested ${formatDate(chain.requested_at)}`
      : chain.status === 'active' && chain.approved_at
        ? `Active since ${formatDate(chain.approved_at)}`
        : chain.status === 'revoked' && chain.revoked_at
          ? `Revoked ${formatDate(chain.revoked_at)}`
          : chain.status === 'rejected' && chain.approved_at
            ? `Rejected ${formatDate(chain.approved_at)}`
            : `Requested ${formatDate(chain.requested_at)}`

  const access =
    chain.granted_group && chain.status === 'active'
      ? chain.granted_group.name
      : chain.status === 'pending'
        ? direction === 'incoming'
          ? 'You choose on approval'
          : 'They choose on approval'
        : 'None'

  const accessNote =
    chain.granted_group && chain.status === 'active'
      ? describeAccess(chain.granted_group.permissions)
      : null

  return (
    <li
      className={cn(
        'grid gap-4 py-5 transition-opacity duration-200 lg:grid-cols-12 lg:items-start',
        ended && 'text-muted-foreground',
      )}
    >
      <div className="lg:col-span-6">
        <div
          className="grid grid-cols-[minmax(0,1fr)_minmax(3rem,7rem)_minmax(0,1fr)] items-center gap-3"
          aria-label={`${chain.source_org.name} to ${chain.target_org.name}: ${CHAIN_STATUS_LABEL[chain.status]}`}
          role="group"
        >
          <OrgName
            name={chain.source_org.name}
            you={direction === 'outgoing'}
          />
          <div className="flex flex-col items-center gap-1">
            <span
              className={cn(
                'text-[0.6875rem] font-semibold tracking-[0.08em] whitespace-nowrap uppercase',
                chain.status === 'active'
                  ? 'text-foreground'
                  : 'text-muted-foreground',
              )}
            >
              {CHAIN_STATUS_LABEL[chain.status]}
            </span>
            <ChainLine status={chain.status} />
          </div>
          <OrgName
            name={chain.target_org.name}
            you={direction === 'incoming'}
            align="right"
          />
        </div>
        <p className="mt-2 text-xs text-muted-foreground">{when}</p>
        {chain.request_message && chain.status === 'pending' && (
          <blockquote className="mt-3 max-w-prose border-l border-border pl-3 text-sm text-muted-foreground">
            {chain.request_message}
          </blockquote>
        )}
        {chain.rejection_reason && chain.status === 'rejected' && (
          <p className="mt-3 text-sm">
            <span className="font-semibold">Reason: </span>
            {chain.rejection_reason}
          </p>
        )}
      </div>

      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm lg:col-span-4">
        <div>
          <dt className="text-xs text-muted-foreground">Access granted</dt>
          <dd className="font-semibold">{access}</dd>
          {accessNote && (
            <dd className="mt-0.5 text-xs text-muted-foreground">
              {accessNote}
            </dd>
          )}
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">Chain Trust Score</dt>
          <dd>
            {scoreFailed ? (
              <span className="text-xs text-muted-foreground">
                Not available
              </span>
            ) : score ? (
              <span className="figure text-2xl">
                {score.aggregated_score}
                <span className="ml-1 text-xs font-normal text-muted-foreground [font-stretch:100%]">
                  / 100
                </span>
              </span>
            ) : (
              <span className="text-xs text-muted-foreground">–</span>
            )}
          </dd>
          {score && (
            <dd className="text-xs text-muted-foreground tabular-nums">
              Average of {score.scores.source_org} and {score.scores.target_org}
            </dd>
          )}
        </div>
      </dl>

      <div className="flex flex-wrap gap-2 lg:col-span-2 lg:justify-end">
        {direction === 'incoming' && chain.status === 'pending' && (
          <>
            <Button className="h-10" onClick={() => onApprove?.(chain)}>
              Approve
            </Button>
            <Button
              variant="outline"
              className="h-10"
              onClick={() => onReject?.(chain)}
            >
              Reject
            </Button>
          </>
        )}
        {direction === 'incoming' && chain.status === 'active' && (
          <Button
            variant="outline"
            className="h-10"
            onClick={() => onRevoke?.(chain)}
          >
            Revoke access
          </Button>
        )}
        {direction === 'outgoing' && chain.status === 'pending' && (
          <Button
            variant="outline"
            className="h-10"
            onClick={() => onCancel?.(chain)}
          >
            Cancel request
          </Button>
        )}
      </div>
    </li>
  )
}

function OrgName({
  name,
  you,
  align = 'left',
}: {
  name: string
  you: boolean
  align?: 'left' | 'right'
}) {
  return (
    <span
      className={cn(
        'flex min-w-0 flex-col',
        align === 'right' && 'items-end text-right',
      )}
    >
      <span className="font-semibold leading-tight [overflow-wrap:anywhere]">
        {name}
      </span>
      {you && <span className="text-xs text-muted-foreground">You</span>}
    </span>
  )
}
