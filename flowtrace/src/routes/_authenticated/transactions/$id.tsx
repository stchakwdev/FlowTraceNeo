import { createFileRoute, useNavigate } from '@tanstack/react-router'

export const Route = createFileRoute('/_authenticated/transactions/$id')({
  component: TransactionDetail,
})

// Mock transaction data
const mockTransaction = {
  id: '1',
  description: 'Office Supplies Purchase',
  amount: 15000,
  currency: 'KES',
  from: { id: '3', name: 'Operations Budget', type: 'account' },
  to: { id: '7', name: 'Staples Kenya', type: 'vendor' },
  date: '2024-12-18',
  status: 'verified' as const,
  category: 'Operations',
  notes: 'Monthly office supplies including printer paper, pens, and folders.',
  verifications: [
    {
      id: 'v1',
      type: 'receipt',
      fileName: 'receipt-001.pdf',
      uploadedAt: '2024-12-18T10:30:00Z',
      uploadedBy: 'John Doe',
    },
    {
      id: 'v2',
      type: 'photo',
      fileName: 'supplies-photo.jpg',
      uploadedAt: '2024-12-18T11:00:00Z',
      uploadedBy: 'Jane Smith',
    },
  ],
  auditLog: [
    { action: 'Created', user: 'John Doe', timestamp: '2024-12-18T09:00:00Z' },
    { action: 'Receipt uploaded', user: 'John Doe', timestamp: '2024-12-18T10:30:00Z' },
    { action: 'Photo uploaded', user: 'Jane Smith', timestamp: '2024-12-18T11:00:00Z' },
    { action: 'Verified', user: 'Finance Manager', timestamp: '2024-12-18T14:00:00Z' },
  ],
}

function TransactionDetail() {
  const { id } = Route.useParams()
  const navigate = useNavigate()

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

  const getStatusBadge = (status: 'pending' | 'verified' | 'flagged') => {
    const styles = {
      pending: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
      verified: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
      flagged: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    }
    return (
      <span className={`inline-flex items-center rounded-full px-3 py-1 text-sm font-medium ${styles[status]}`}>
        {status === 'verified' && <CheckIcon className="mr-1 h-4 w-4" />}
        {status === 'pending' && <ClockIcon className="mr-1 h-4 w-4" />}
        {status === 'flagged' && <AlertIcon className="mr-1 h-4 w-4" />}
        {status.charAt(0).toUpperCase() + status.slice(1)}
      </span>
    )
  }

  const getVerificationIcon = (type: string) => {
    switch (type) {
      case 'receipt':
        return <DocumentIcon className="h-5 w-5" />
      case 'photo':
        return <CameraIcon className="h-5 w-5" />
      default:
        return <DocumentIcon className="h-5 w-5" />
    }
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="mx-auto max-w-4xl px-4 py-8">
        {/* Header */}
        <div className="mb-8">
          <button
            onClick={() => navigate({ to: '/transactions' })}
            className="mb-4 flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
          >
            <ArrowLeftIcon className="h-4 w-4" />
            Back to Transactions
          </button>
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className="flex items-center gap-3">
                <h1 className="text-3xl font-bold">{mockTransaction.description}</h1>
                {getStatusBadge(mockTransaction.status)}
              </div>
              <p className="mt-1 text-muted-foreground">Transaction #{id}</p>
            </div>
            <div className="flex gap-2">
              <button className="rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent">
                Edit
              </button>
              {mockTransaction.status === 'pending' && (
                <button className="rounded-lg bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700">
                  Verify
                </button>
              )}
            </div>
          </div>
        </div>

        <div className="grid gap-6 lg:grid-cols-3">
          {/* Main Content */}
          <div className="space-y-6 lg:col-span-2">
            {/* Transaction Details */}
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold">Transaction Details</h2>
              <dl className="grid gap-4 sm:grid-cols-2">
                <div>
                  <dt className="text-sm text-muted-foreground">Amount</dt>
                  <dd className="text-2xl font-bold">
                    {formatCurrency(mockTransaction.amount, mockTransaction.currency)}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-muted-foreground">Date</dt>
                  <dd className="font-medium">{formatDate(mockTransaction.date)}</dd>
                </div>
                <div>
                  <dt className="text-sm text-muted-foreground">Category</dt>
                  <dd className="font-medium">{mockTransaction.category}</dd>
                </div>
                <div>
                  <dt className="text-sm text-muted-foreground">Status</dt>
                  <dd>{getStatusBadge(mockTransaction.status)}</dd>
                </div>
              </dl>
            </div>

            {/* Flow */}
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold">Money Flow</h2>
              <div className="flex items-center justify-between rounded-lg bg-accent/50 p-4">
                <div className="text-center">
                  <div className="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
                    <WalletIcon className="h-6 w-6 text-primary" />
                  </div>
                  <p className="font-medium">{mockTransaction.from.name}</p>
                  <p className="text-xs text-muted-foreground">{mockTransaction.from.type}</p>
                </div>
                <div className="flex-1 px-4">
                  <div className="flex items-center justify-center">
                    <div className="h-0.5 flex-1 bg-primary/30" />
                    <ArrowRightIcon className="mx-2 h-5 w-5 text-primary" />
                    <div className="h-0.5 flex-1 bg-primary/30" />
                  </div>
                  <p className="mt-1 text-center text-sm font-medium text-primary">
                    {formatCurrency(mockTransaction.amount, mockTransaction.currency)}
                  </p>
                </div>
                <div className="text-center">
                  <div className="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
                    <BuildingIcon className="h-6 w-6 text-primary" />
                  </div>
                  <p className="font-medium">{mockTransaction.to.name}</p>
                  <p className="text-xs text-muted-foreground">{mockTransaction.to.type}</p>
                </div>
              </div>
            </div>

            {/* Notes */}
            {mockTransaction.notes && (
              <div className="rounded-lg border border-border bg-card p-6">
                <h2 className="mb-4 text-lg font-semibold">Notes</h2>
                <p className="text-muted-foreground">{mockTransaction.notes}</p>
              </div>
            )}

            {/* Verifications */}
            <div className="rounded-lg border border-border bg-card p-6">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-lg font-semibold">Verifications</h2>
                <button className="flex items-center gap-1 text-sm text-primary hover:underline">
                  <PlusIcon className="h-4 w-4" />
                  Add
                </button>
              </div>
              {mockTransaction.verifications.length > 0 ? (
                <ul className="divide-y divide-border">
                  {mockTransaction.verifications.map((v) => (
                    <li key={v.id} className="flex items-center justify-between py-3">
                      <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent">
                          {getVerificationIcon(v.type)}
                        </div>
                        <div>
                          <p className="font-medium">{v.fileName}</p>
                          <p className="text-xs text-muted-foreground">
                            {v.uploadedBy} • {formatDateTime(v.uploadedAt)}
                          </p>
                        </div>
                      </div>
                      <button className="text-sm text-primary hover:underline">View</button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted-foreground">No verifications yet</p>
              )}
            </div>
          </div>

          {/* Sidebar */}
          <div className="space-y-6">
            {/* Quick Actions */}
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold">Actions</h2>
              <div className="space-y-2">
                <button className="flex w-full items-center gap-2 rounded-lg border border-border px-4 py-2 text-sm hover:bg-accent">
                  <UploadIcon className="h-4 w-4" />
                  Upload Document
                </button>
                <button className="flex w-full items-center gap-2 rounded-lg border border-border px-4 py-2 text-sm hover:bg-accent">
                  <ShareIcon className="h-4 w-4" />
                  Share
                </button>
                <button className="flex w-full items-center gap-2 rounded-lg border border-border px-4 py-2 text-sm hover:bg-accent">
                  <DownloadIcon className="h-4 w-4" />
                  Export PDF
                </button>
              </div>
            </div>

            {/* Audit Log */}
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold">Activity</h2>
              <ul className="space-y-4">
                {mockTransaction.auditLog.map((log, i) => (
                  <li key={i} className="relative pl-6">
                    <div className="absolute left-0 top-1 h-3 w-3 rounded-full border-2 border-primary bg-background" />
                    {i < mockTransaction.auditLog.length - 1 && (
                      <div className="absolute left-1.5 top-4 h-full w-0 border-l border-dashed border-border" />
                    )}
                    <p className="text-sm font-medium">{log.action}</p>
                    <p className="text-xs text-muted-foreground">
                      {log.user} • {formatDateTime(log.timestamp)}
                    </p>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// Icon components
function ArrowLeftIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
    </svg>
  )
}

function ArrowRightIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" />
    </svg>
  )
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
    </svg>
  )
}

function ClockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  )
}

function AlertIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
    </svg>
  )
}

function WalletIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z" />
    </svg>
  )
}

function BuildingIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
    </svg>
  )
}

function DocumentIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
    </svg>
  )
}

function CameraIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
  )
}

function PlusIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
    </svg>
  )
}

function UploadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
    </svg>
  )
}

function ShareIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
    </svg>
  )
}

function DownloadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
    </svg>
  )
}
