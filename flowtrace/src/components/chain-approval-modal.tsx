import { useState } from 'react'
import {
  X,
  Buildings,
  Shield,
  MapPin,
  CheckCircle,
  XCircle,
  CircleNotch,
  UsersThree,
  TreeStructure,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useAccessGroups, useNodes, useApproveChain, useRejectChain } from '@/lib/queries'
import type { OrganizationChainWithDetails } from '@/lib/supabase'

interface ChainApprovalModalProps {
  chain: OrganizationChainWithDetails | null
  isOpen: boolean
  onClose: () => void
  approvedBy: string
}

export function ChainApprovalModal({
  chain,
  isOpen,
  onClose,
  approvedBy,
}: ChainApprovalModalProps) {
  const [selectedGroupId, setSelectedGroupId] = useState('')
  const [selectedNodeId, setSelectedNodeId] = useState('')
  const [rejectionReason, setRejectionReason] = useState('')
  const [mode, setMode] = useState<'approve' | 'reject'>('approve')

  const { data: accessGroups } = useAccessGroups()
  const { data: nodes } = useNodes()
  const approveChain = useApproveChain()
  const rejectChain = useRejectChain()

  const handleApprove = async () => {
    if (!chain || !selectedGroupId) return

    try {
      await approveChain.mutateAsync({
        chainId: chain.id,
        approvedBy,
        grantedGroupId: selectedGroupId,
        linkNodeId: selectedNodeId || undefined,
      })
      onClose()
      resetForm()
    } catch (error) {
      console.error('Failed to approve chain:', error)
    }
  }

  const handleReject = async () => {
    if (!chain) return

    try {
      await rejectChain.mutateAsync({
        chainId: chain.id,
        rejectedBy: approvedBy,
        reason: rejectionReason.trim() || undefined,
      })
      onClose()
      resetForm()
    } catch (error) {
      console.error('Failed to reject chain:', error)
    }
  }

  const resetForm = () => {
    setSelectedGroupId('')
    setSelectedNodeId('')
    setRejectionReason('')
    setMode('approve')
  }

  const getTrustGrade = (score: number) => {
    if (score >= 90) return 'A+'
    if (score >= 80) return 'A'
    if (score >= 70) return 'B'
    if (score >= 60) return 'C'
    if (score >= 50) return 'D'
    return 'F'
  }

  if (!isOpen || !chain) return null

  const sourceOrg = chain.source_org

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <Card className="w-full max-w-lg mx-4 p-0 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between border-b p-4">
          <h2 className="text-lg font-semibold">Review Chain Request</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-5 w-5" />
          </Button>
        </div>

        <div className="p-4 space-y-4">
          {/* Requesting Organization */}
          <div className="space-y-2">
            <Label>Requesting Organization</Label>
            <div className="flex items-center gap-3 p-3 border rounded-lg bg-muted/30">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                {sourceOrg.logo_url ? (
                  <img
                    src={sourceOrg.logo_url}
                    alt={sourceOrg.name}
                    className="h-10 w-10 rounded object-cover"
                  />
                ) : (
                  <Buildings className="h-6 w-6 text-primary" />
                )}
              </div>
              <div className="flex-1">
                <p className="font-medium">{sourceOrg.name}</p>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  {sourceOrg.country && (
                    <span className="flex items-center gap-1">
                      <MapPin className="h-3 w-3" />
                      {sourceOrg.country}
                    </span>
                  )}
                  <span className="flex items-center gap-1">
                    <Shield className="h-3 w-3 text-emerald-600" />
                    Trust: {getTrustGrade(sourceOrg.trust_score)} ({sourceOrg.trust_score})
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* Request Message */}
          {chain.request_message && (
            <div className="space-y-2">
              <Label>Their Message</Label>
              <div className="p-3 border rounded-lg bg-muted/30">
                <p className="text-sm">{chain.request_message}</p>
              </div>
            </div>
          )}

          {/* Mode Toggle */}
          <div className="flex gap-2">
            <Button
              variant={mode === 'approve' ? 'default' : 'outline'}
              size="sm"
              className="flex-1"
              onClick={() => setMode('approve')}
            >
              <CheckCircle className="h-4 w-4 mr-2" />
              Approve
            </Button>
            <Button
              variant={mode === 'reject' ? 'destructive' : 'outline'}
              size="sm"
              className="flex-1"
              onClick={() => setMode('reject')}
            >
              <XCircle className="h-4 w-4 mr-2" />
              Reject
            </Button>
          </div>

          {mode === 'approve' ? (
            <>
              {/* Access Group Selection */}
              <div className="space-y-2">
                <Label>Grant Access Level</Label>
                <Select
                  value={selectedGroupId}
                  items={{
                    '': 'Select an access group...',
                    ...Object.fromEntries(
                      (accessGroups ?? []).map((group) => [group.id, group.name]),
                    ),
                  }}
                  onValueChange={(value) => setSelectedGroupId(value ?? '')}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {accessGroups?.map((group) => (
                      <SelectItem key={group.id} value={group.id}>
                        <div className="flex items-center gap-2">
                          <UsersThree className="h-4 w-4" />
                          {group.name}
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  This determines what data the requesting organization can see in
                  your flows.
                </p>
              </div>

              {/* Link Node Selection (Optional) */}
              <div className="space-y-2">
                <Label>Link to Node (optional)</Label>
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
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">No specific node</SelectItem>
                    {nodes?.map((node) => (
                      <SelectItem key={node.id} value={node.id}>
                        <div className="flex items-center gap-2">
                          <TreeStructure className="h-4 w-4" />
                          {node.name}
                          <span className="text-xs text-muted-foreground">
                            ({node.type})
                          </span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Optionally link this chain to a specific entry node in your flow.
                </p>
              </div>
            </>
          ) : (
            /* Rejection Reason */
            <div className="space-y-2">
              <Label htmlFor="reason">Reason (optional)</Label>
              <Textarea
                id="reason"
                placeholder="Explain why you're declining this request..."
                value={rejectionReason}
                onChange={(e) => setRejectionReason(e.target.value)}
                rows={3}
              />
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 border-t p-4">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          {mode === 'approve' ? (
            <Button
              onClick={handleApprove}
              disabled={!selectedGroupId || approveChain.isPending}
            >
              {approveChain.isPending ? (
                <CircleNotch className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <CheckCircle className="h-4 w-4 mr-2" />
              )}
              Approve Request
            </Button>
          ) : (
            <Button
              variant="destructive"
              onClick={handleReject}
              disabled={rejectChain.isPending}
            >
              {rejectChain.isPending ? (
                <CircleNotch className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <XCircle className="h-4 w-4 mr-2" />
              )}
              Reject Request
            </Button>
          )}
        </div>
      </Card>
    </div>
  )
}
