import { useState } from 'react'
import {
  X,
  MagnifyingGlass,
  Buildings,
  Shield,
  MapPin,
  PaperPlaneTilt,
  CircleNotch,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { useSearchOrganizations, useRequestChain } from '@/lib/queries'
import type { Organization } from '@/lib/supabase'

interface ChainRequestModalProps {
  isOpen: boolean
  onClose: () => void
  requestedBy: string
}

export function ChainRequestModal({
  isOpen,
  onClose,
  requestedBy,
}: ChainRequestModalProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedOrg, setSelectedOrg] = useState<Pick<
    Organization,
    'id' | 'name' | 'logo_url' | 'trust_score' | 'type' | 'country'
  > | null>(null)
  const [message, setMessage] = useState('')

  const { data: searchResults, isLoading: isSearching } =
    useSearchOrganizations(searchQuery)
  const requestChain = useRequestChain()

  const handleSubmit = async () => {
    if (!selectedOrg) return

    try {
      await requestChain.mutateAsync({
        targetOrgId: selectedOrg.id,
        requestedBy,
        message: message.trim() || undefined,
      })
      onClose()
      setSelectedOrg(null)
      setMessage('')
      setSearchQuery('')
    } catch (error) {
      console.error('Failed to request chain:', error)
    }
  }

  const getTrustGrade = (score: number) => {
    if (score >= 90) return 'A+'
    if (score >= 80) return 'A'
    if (score >= 70) return 'B'
    if (score >= 60) return 'C'
    if (score >= 50) return 'D'
    return 'F'
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <Card className="w-full max-w-lg mx-4 p-0 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between border-b p-4">
          <h2 className="text-lg font-semibold">Request Chain Link</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-5 w-5" />
          </Button>
        </div>

        <div className="p-4 space-y-4">
          {/* Search */}
          {!selectedOrg && (
            <div className="space-y-2">
              <Label>Search Organization</Label>
              <div className="relative">
                <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search by organization name..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9"
                />
              </div>

              {/* Search Results */}
              {searchQuery.length >= 2 && (
                <div className="max-h-60 overflow-y-auto border rounded-lg">
                  {isSearching ? (
                    <div className="flex items-center justify-center p-4">
                      <CircleNotch className="h-5 w-5 animate-spin text-muted-foreground" />
                    </div>
                  ) : searchResults && searchResults.length > 0 ? (
                    <div className="divide-y">
                      {searchResults.map((org) => (
                        <button
                          key={org.id}
                          className="w-full flex items-center gap-3 p-3 hover:bg-muted/50 transition-colors text-left"
                          onClick={() => {
                            setSelectedOrg(org)
                            setSearchQuery('')
                          }}
                        >
                          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                            {org.logo_url ? (
                              <img
                                src={org.logo_url}
                                alt={org.name}
                                className="h-8 w-8 rounded object-cover"
                              />
                            ) : (
                              <Buildings className="h-5 w-5 text-primary" />
                            )}
                          </div>
                          <div className="flex-1 min-w-0">
                            <p className="font-medium truncate">{org.name}</p>
                            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                              {org.country && (
                                <span className="flex items-center gap-1">
                                  <MapPin className="h-3 w-3" />
                                  {org.country}
                                </span>
                              )}
                              {org.type && (
                                <span className="px-1.5 py-0.5 rounded bg-muted">
                                  {org.type}
                                </span>
                              )}
                            </div>
                          </div>
                          <div className="flex items-center gap-1 text-sm">
                            <Shield className="h-4 w-4 text-emerald-600" />
                            <span className="font-medium">
                              {getTrustGrade(org.trust_score)}
                            </span>
                          </div>
                        </button>
                      ))}
                    </div>
                  ) : (
                    <p className="p-4 text-center text-sm text-muted-foreground">
                      No organizations found
                    </p>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Selected Organization */}
          {selectedOrg && (
            <div className="space-y-2">
              <Label>Selected Organization</Label>
              <div className="flex items-center gap-3 p-3 border rounded-lg bg-muted/30">
                <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                  {selectedOrg.logo_url ? (
                    <img
                      src={selectedOrg.logo_url}
                      alt={selectedOrg.name}
                      className="h-10 w-10 rounded object-cover"
                    />
                  ) : (
                    <Buildings className="h-6 w-6 text-primary" />
                  )}
                </div>
                <div className="flex-1">
                  <p className="font-medium">{selectedOrg.name}</p>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    {selectedOrg.country && (
                      <span className="flex items-center gap-1">
                        <MapPin className="h-3 w-3" />
                        {selectedOrg.country}
                      </span>
                    )}
                    <span className="flex items-center gap-1">
                      <Shield className="h-3 w-3 text-emerald-600" />
                      Trust Score: {selectedOrg.trust_score}
                    </span>
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelectedOrg(null)}
                >
                  Change
                </Button>
              </div>
            </div>
          )}

          {/* Message */}
          <div className="space-y-2">
            <Label htmlFor="message">Message (optional)</Label>
            <Textarea
              id="message"
              placeholder="Explain why you'd like to link with this organization..."
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
            />
            <p className="text-xs text-muted-foreground">
              This message will be visible to the organization when they review
              your request.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 border-t p-4">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={!selectedOrg || requestChain.isPending}
          >
            {requestChain.isPending ? (
              <CircleNotch className="h-4 w-4 animate-spin mr-2" />
            ) : (
              <PaperPlaneTilt className="h-4 w-4 mr-2" />
            )}
            Send Request
          </Button>
        </div>
      </Card>
    </div>
  )
}
