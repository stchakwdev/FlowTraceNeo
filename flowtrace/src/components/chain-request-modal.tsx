import { useEffect, useRef, useState } from 'react'
import { CircleNotch, MagnifyingGlass } from '@phosphor-icons/react'
import type { Organization } from '@/lib/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { ReportDialog } from '@/components/report-dialog'
import { useRequestChain, useSearchOrganizations } from '@/lib/queries'
import { cn } from '@/lib/utils'

type OrgSummary = Pick<
  Organization,
  'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'
>

const ORG_TYPE_LABEL: Record<NonNullable<Organization['type']>, string> = {
  NGO: 'NGO',
  SME: 'Business',
  SOCIAL_ENTERPRISE: 'Social enterprise',
  GOVERNMENT: 'Government',
}

interface ChainRequestModalProps {
  isOpen: boolean
  onClose: () => void
  requestedBy: string
  /** Organisations you already have a pending or active link with. */
  linkedOrgIds?: Array<string>
}

export function ChainRequestModal({
  isOpen,
  onClose,
  requestedBy,
  linkedOrgIds = [],
}: ChainRequestModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedOrg, setSelectedOrg] = useState<OrgSummary | null>(null)
  const [message, setMessage] = useState('')
  const searchRef = useRef<HTMLInputElement>(null)

  const trimmed = searchQuery.trim()
  const {
    data: searchResults,
    isLoading: isSearching,
    isError: searchFailed,
  } = useSearchOrganizations(trimmed)
  const requestChain = useRequestChain()

  useEffect(() => {
    if (!isOpen) {
      setSearchQuery('')
      setSelectedOrg(null)
      setMessage('')
      requestChain.reset()
    }
  }, [isOpen])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedOrg) return
    try {
      await requestChain.mutateAsync({
        targetOrgId: selectedOrg.id,
        requestedBy,
        message: message.trim() || undefined,
      })
      onClose()
    } catch {
      // The error is shown in the dialog from requestChain.error
    }
  }

  return (
    <ReportDialog
      open={isOpen}
      onOpenChange={(open) => !open && onClose()}
      title="Request a chain link"
      description="Ask another organisation for a view of their flows. They choose what depth you see, and either side can end the link later."
      initialFocus={searchRef}
      footer={
        <>
          <Button variant="outline" onClick={onClose} className="h-10">
            Cancel
          </Button>
          <Button
            type="submit"
            form="chain-request-form"
            className="h-10"
            disabled={!selectedOrg || requestChain.isPending}
          >
            {requestChain.isPending && (
              <CircleNotch className="animate-spin" aria-hidden="true" />
            )}
            {requestChain.isPending ? 'Sending request' : 'Send request'}
          </Button>
        </>
      }
    >
      <form
        id="chain-request-form"
        onSubmit={handleSubmit}
        className="flex flex-col gap-5"
      >
        <div className="flex flex-col gap-2">
          <label htmlFor="chain-org-search" className="text-sm font-semibold">
            Organisation
          </label>
          {selectedOrg ? (
            <div className="flex items-center justify-between gap-3 border-y border-border py-3">
              <OrgLine org={selectedOrg} />
              <Button
                variant="outline"
                size="sm"
                className="h-10"
                onClick={() => {
                  setSelectedOrg(null)
                  setTimeout(() => searchRef.current?.focus(), 0)
                }}
              >
                Change
              </Button>
            </div>
          ) : (
            <>
              <div className="relative">
                <MagnifyingGlass
                  aria-hidden="true"
                  className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
                />
                <Input
                  id="chain-org-search"
                  ref={searchRef}
                  type="search"
                  autoComplete="off"
                  placeholder="Search by name, e.g. Maji Safi"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="h-10 pl-9"
                  aria-describedby="chain-org-search-hint"
                  aria-controls="chain-org-results"
                />
              </div>
              <p
                id="chain-org-search-hint"
                className="text-xs text-muted-foreground"
              >
                Type at least two letters.
              </p>
              <div id="chain-org-results" aria-live="polite">
                {trimmed.length >= 2 &&
                  (isSearching ? (
                    <p className="flex items-center gap-2 py-3 text-sm text-muted-foreground">
                      <CircleNotch className="size-4 animate-spin" />
                      Searching organisations
                    </p>
                  ) : searchFailed ? (
                    <p role="alert" className="py-3 text-sm text-destructive">
                      Search failed. Check your connection and try again.
                    </p>
                  ) : searchResults && searchResults.length > 0 ? (
                    <ul className="max-h-64 divide-y divide-border overflow-y-auto border-y border-border">
                      {searchResults.map((org) => {
                        const linked = linkedOrgIds.includes(org.id)
                        return (
                          <li key={org.id}>
                            <button
                              type="button"
                              disabled={linked}
                              onClick={() => {
                                setSelectedOrg(org)
                                setSearchQuery('')
                              }}
                              className={cn(
                                'flex min-h-12 w-full items-center justify-between gap-3 px-2 py-2 text-left outline-none focus-visible:bg-accent',
                                linked
                                  ? 'cursor-not-allowed opacity-60'
                                  : 'hover:bg-accent',
                              )}
                            >
                              <OrgLine org={org} />
                              {linked && (
                                <span className="shrink-0 text-xs text-muted-foreground">
                                  Already linked
                                </span>
                              )}
                            </button>
                          </li>
                        )
                      })}
                    </ul>
                  ) : (
                    <p className="py-3 text-sm text-muted-foreground">
                      No organisation matches “{trimmed}”. Check the spelling,
                      or ask them to join FlowTrace first.
                    </p>
                  ))}
              </div>
            </>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <label htmlFor="chain-message" className="text-sm font-semibold">
            Message{' '}
            <span className="font-normal text-muted-foreground">
              (optional)
            </span>
          </label>
          <Textarea
            id="chain-message"
            placeholder="Say which programme or grant you want to follow, and why."
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            rows={3}
            aria-describedby="chain-message-hint"
          />
          <p id="chain-message-hint" className="text-xs text-muted-foreground">
            They see this when they review your request.
          </p>
        </div>

        {requestChain.isError && (
          <p role="alert" className="text-sm text-destructive">
            The request wasn't sent.{' '}
            {requestChain.error instanceof Error
              ? requestChain.error.message
              : 'Try again in a moment.'}
          </p>
        )}
      </form>
    </ReportDialog>
  )
}

function OrgLine({ org }: { org: OrgSummary }) {
  return (
    <span className="flex min-w-0 flex-col">
      <span className="truncate font-semibold">{org.name}</span>
      <span className="text-xs text-muted-foreground">
        {[org.type ? ORG_TYPE_LABEL[org.type] : null, org.country]
          .filter(Boolean)
          .join(' · ')}
        {' · '}Trust Score{' '}
        <span className="tabular-nums">{org.trust_score}</span>
      </span>
    </span>
  )
}
