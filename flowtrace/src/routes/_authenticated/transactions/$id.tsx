import { useState } from 'react'
import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import {
  ArrowLeft,
  ArrowRight,
  Wallet,
  Buildings,
  CloudArrowUp,
  ShareNetwork,
  Download,
  CheckCircle,
  Clock,
  Warning,
  PencilSimple,
} from '@phosphor-icons/react'
import type { Verification } from '@/lib/supabase'
import { useTransaction, useVerifications } from '@/lib/queries'
import { VerificationGallery } from '@/components/verification-gallery'
import { VerificationViewer } from '@/components/verification-viewer'
import { FileUpload } from '@/components/file-upload'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export const Route = createFileRoute('/_authenticated/transactions/$id')({
  component: TransactionDetail,
})

function TransactionDetail() {
  const { id } = Route.useParams()
  const navigate = useNavigate()

  // Verification viewer state
  const [selectedVerification, setSelectedVerification] =
    useState<Verification | null>(null)
  const [showAddVerification, setShowAddVerification] = useState(false)

  const { data: transaction, isLoading, error } = useTransaction(id)
  const { data: verifications } = useVerifications(id)

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
    }).format(amount)
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-KE', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    })
  }

  const formatDateTime = (dateString: string) => {
    return new Date(dateString).toLocaleString('en-KE', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  const getStatusBadge = (
    status: 'pending' | 'verified' | 'flagged' | 'rejected',
  ) => {
    const styles = {
      pending:
        'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
      verified:
        'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
      flagged: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
      rejected:
        'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400',
    }
    const icons = {
      pending: Clock,
      verified: CheckCircle,
      flagged: Warning,
      rejected: Warning,
    }
    const Icon = icons[status]
    return (
      <span
        className={`inline-flex items-center rounded-full px-3 py-1 text-sm font-medium ${styles[status]}`}
      >
        <Icon className="mr-1 h-4 w-4" weight="fill" />
        {status.charAt(0).toUpperCase() + status.slice(1)}
      </span>
    )
  }

  const handleVerificationClick = (verification: Verification) => {
    setSelectedVerification(verification)
  }

  const handleVerificationNavigate = (verification: Verification) => {
    setSelectedVerification(verification)
  }

  const handleAddVerification = () => {
    setShowAddVerification(true)
  }

  const handleUploadComplete = () => {
    setShowAddVerification(false)
  }

  if (isLoading) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8">
        <div className="mb-8 h-8 w-48 skeleton-shimmer rounded bg-muted" />
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="space-y-6 lg:col-span-2">
            <div className="h-48 skeleton-shimmer rounded-lg bg-muted" />
            <div className="h-32 skeleton-shimmer rounded-lg bg-muted" />
          </div>
          <div className="h-64 skeleton-shimmer rounded-lg bg-muted" />
        </div>
      </div>
    )
  }

  if (error || !transaction) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8 text-center">
        <h1 className="text-2xl font-bold text-destructive">
          Transaction not found
        </h1>
        <p className="mt-2 text-muted-foreground">
          The transaction you're looking for doesn't exist.
        </p>
        <Link
          to="/transactions"
          className="mt-4 inline-block text-primary hover:underline"
        >
          Back to Transactions
        </Link>
      </div>
    )
  }

  // Calculate total trust points from verifications
  const totalTrustPoints =
    verifications?.reduce((sum, v) => sum + (v.trust_points || 0), 0) ?? 0

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <button
          onClick={() => navigate({ to: '/transactions' })}
          className="mb-4 flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Transactions
        </button>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold">{transaction.description}</h1>
              {getStatusBadge(transaction.verification_status)}
            </div>
            <p className="mt-1 text-muted-foreground">Transaction #{id}</p>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm">
              <PencilSimple className="h-4 w-4 mr-1" />
              Edit
            </Button>
            {transaction.verification_status === 'pending' && (
              <Button size="sm" className="bg-green-600 hover:bg-green-700">
                <CheckCircle className="h-4 w-4 mr-1" />
                Verify
              </Button>
            )}
          </div>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Main Content */}
        <div className="space-y-6 lg:col-span-2">
          {/* Transaction Details */}
          <Card className="p-6">
            <h2 className="mb-4 text-lg font-semibold">Transaction Details</h2>
            <dl className="grid gap-4 sm:grid-cols-2">
              <div>
                <dt className="text-sm text-muted-foreground">Amount</dt>
                <dd className="text-2xl font-bold">
                  {formatCurrency(transaction.amount, transaction.currency)}
                </dd>
              </div>
              <div>
                <dt className="text-sm text-muted-foreground">Date</dt>
                <dd className="font-medium">
                  {formatDate(transaction.transaction_date)}
                </dd>
              </div>
              <div>
                <dt className="text-sm text-muted-foreground">Category</dt>
                <dd className="font-medium">
                  {transaction.category ?? 'Uncategorized'}
                </dd>
              </div>
              <div>
                <dt className="text-sm text-muted-foreground">Status</dt>
                <dd>{getStatusBadge(transaction.verification_status)}</dd>
              </div>
            </dl>
          </Card>

          {/* Flow */}
          <Card className="p-6">
            <h2 className="mb-4 text-lg font-semibold">Money Flow</h2>
            <div className="flex items-center justify-between rounded-lg bg-accent/50 p-4">
              <div className="text-center">
                <div className="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
                  <Wallet className="h-6 w-6 text-primary" weight="duotone" />
                </div>
                <p className="font-medium">
                  {transaction.from_node?.name ?? 'Unknown'}
                </p>
                <p className="text-xs text-muted-foreground">
                  {transaction.from_node?.type ?? 'source'}
                </p>
              </div>
              <div className="flex-1 px-4">
                <div className="flex items-center justify-center">
                  <div className="h-0.5 flex-1 bg-primary/30" />
                  <ArrowRight className="mx-2 h-5 w-5 text-primary" />
                  <div className="h-0.5 flex-1 bg-primary/30" />
                </div>
                <p className="mt-1 text-center text-sm font-medium text-primary">
                  {formatCurrency(transaction.amount, transaction.currency)}
                </p>
              </div>
              <div className="text-center">
                <div className="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
                  <Buildings
                    className="h-6 w-6 text-primary"
                    weight="duotone"
                  />
                </div>
                <p className="font-medium">
                  {transaction.to_node?.name ?? 'Unknown'}
                </p>
                <p className="text-xs text-muted-foreground">
                  {transaction.to_node?.type ?? 'destination'}
                </p>
              </div>
            </div>
          </Card>

          {/* Notes */}
          {transaction.notes && (
            <Card className="p-6">
              <h2 className="mb-4 text-lg font-semibold">Notes</h2>
              <p className="text-muted-foreground">{transaction.notes}</p>
            </Card>
          )}

          {/* Verifications - Using new gallery component */}
          <Card className="p-6">
            <h2 className="mb-4 text-lg font-semibold">Proof & Verification</h2>
            <VerificationGallery
              verifications={verifications ?? []}
              onVerificationClick={handleVerificationClick}
              onAddClick={handleAddVerification}
              showAddButton={true}
            />
          </Card>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Quick Actions */}
          <Card className="p-6">
            <h2 className="mb-4 text-lg font-semibold">Actions</h2>
            <div className="space-y-2">
              <Button
                variant="outline"
                className="w-full justify-start"
                onClick={handleAddVerification}
              >
                <CloudArrowUp className="h-4 w-4 mr-2" />
                Upload Proof
              </Button>
              <Button variant="outline" className="w-full justify-start">
                <ShareNetwork className="h-4 w-4 mr-2" />
                Share
              </Button>
              <Button variant="outline" className="w-full justify-start">
                <Download className="h-4 w-4 mr-2" />
                Export PDF
              </Button>
            </div>
          </Card>

          {/* Transaction Info */}
          <Card className="p-6">
            <h2 className="mb-4 text-lg font-semibold">Information</h2>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-muted-foreground">Reference</dt>
                <dd className="font-medium">
                  {transaction.reference ?? 'N/A'}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Created</dt>
                <dd className="font-medium">
                  {formatDateTime(transaction.created_at)}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Last Updated</dt>
                <dd className="font-medium">
                  {formatDateTime(transaction.updated_at)}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Verification Count</dt>
                <dd className="font-medium">{verifications?.length ?? 0}</dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Trust Points</dt>
                <dd className="font-medium text-emerald-600">
                  +{totalTrustPoints}
                </dd>
              </div>
            </dl>
          </Card>
        </div>
      </div>

      {/* Verification Viewer Modal */}
      {selectedVerification && (
        <VerificationViewer
          verification={selectedVerification}
          verifications={verifications ?? []}
          onClose={() => setSelectedVerification(null)}
          onNavigate={handleVerificationNavigate}
        />
      )}

      {/* Add Verification Modal */}
      {showAddVerification && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
          <Card className="w-full max-w-lg max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold">Add Verification</h2>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setShowAddVerification(false)}
                >
                  <span className="sr-only">Close</span>
                  <svg
                    className="h-5 w-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </Button>
              </div>
              <FileUpload
                organizationId={transaction.organization_id}
                transactionId={id}
                verificationType="document"
                onUploadComplete={handleUploadComplete}
              />
            </div>
          </Card>
        </div>
      )}
    </div>
  )
}
