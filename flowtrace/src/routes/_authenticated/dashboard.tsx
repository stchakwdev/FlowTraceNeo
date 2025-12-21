import { Link, createFileRoute } from '@tanstack/react-router'
import type { TransactionWithNodes } from '@/lib/supabase'
import { useOrg } from '@/lib/org-context'
import { useOrgStats, useTransactions } from '@/lib/queries'
import { getTrustGrade } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/dashboard')({
  component: Dashboard,
})

function Dashboard() {
  const { organization, isLoading: orgLoading } = useOrg()
  const { data: stats, isLoading: statsLoading } = useOrgStats()
  const { data: transactions, isLoading: txnLoading } = useTransactions()

  const isLoading = orgLoading || statsLoading || txnLoading
  const recentTransactions = transactions?.slice(0, 5) ?? []

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Dashboard</h1>
        <p className="text-muted-foreground">
          Welcome back{organization?.name ? `, ${organization.name}` : ''}
        </p>
      </div>

      {/* Stats Cards */}
      <div className="mb-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {isLoading ? (
          <>
            <StatCardSkeleton />
            <StatCardSkeleton />
            <StatCardSkeleton />
            <StatCardSkeleton />
          </>
        ) : (
          <>
            <StatCard
              title="Trust Score"
              value={String(organization?.trust_score ?? 0)}
              subtitle={`${getTrustGrade(organization?.trust_score ?? 0)} Grade`}
              trend={
                organization?.verified
                  ? 'Verified organization'
                  : 'Pending verification'
              }
            />
            <StatCard
              title="Total Transactions"
              value={stats?.total.toLocaleString() ?? '0'}
              subtitle="All time"
              trend={`${stats?.verified ?? 0} verified`}
            />
            <StatCard
              title="Verified"
              value={`${stats?.verificationRate ?? 0}%`}
              subtitle="Verification rate"
              trend={`${stats?.verified ?? 0} of ${stats?.total ?? 0} transactions`}
            />
            <StatCard
              title="Pending"
              value={String(stats?.pending ?? 0)}
              subtitle="Awaiting verification"
              trend={
                stats?.flagged ? `${stats.flagged} flagged` : 'No flagged items'
              }
            />
          </>
        )}
      </div>

      {/* Quick Actions */}
      <div className="mb-8">
        <h2 className="mb-4 text-xl font-semibold">Quick Actions</h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <ActionCard
            title="Add Transaction"
            description="Record a new financial transaction"
            href="/transactions/new"
          />
          <ActionCard
            title="View Flow Diagram"
            description="Visualize money flow through your organization"
            href="/flows"
          />
          <ActionCard
            title="Upload Verification"
            description="Add receipts or documentation"
            href="/transactions?filter=pending"
          />
        </div>
      </div>

      {/* Recent Transactions */}
      <div>
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-xl font-semibold">Recent Transactions</h2>
          <Link
            to="/transactions"
            className="text-sm text-primary hover:underline"
          >
            View all
          </Link>
        </div>
        {txnLoading ? (
          <div className="space-y-2">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="h-16 animate-pulse rounded-lg bg-muted" />
            ))}
          </div>
        ) : recentTransactions.length === 0 ? (
          <div className="rounded-lg border border-border bg-card p-8 text-center">
            <p className="text-muted-foreground">
              No transactions yet. Add your first transaction to get started.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {recentTransactions.map((txn) => (
              <TransactionRow key={txn.id} transaction={txn} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function StatCard({
  title,
  value,
  subtitle,
  trend,
}: {
  title: string
  value: string
  subtitle: string
  trend: string
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-6">
      <p className="text-sm text-muted-foreground">{title}</p>
      <p className="mt-1 text-3xl font-bold">{value}</p>
      <p className="text-sm text-muted-foreground">{subtitle}</p>
      <p className="mt-2 text-xs text-primary">{trend}</p>
    </div>
  )
}

function ActionCard({
  title,
  description,
  href,
}: {
  title: string
  description: string
  href: string
}) {
  return (
    <Link
      to={href}
      className="block rounded-lg border border-border bg-card p-6 transition-colors hover:border-primary hover:bg-accent"
    >
      <h3 className="font-semibold">{title}</h3>
      <p className="mt-1 text-sm text-muted-foreground">{description}</p>
    </Link>
  )
}

function StatCardSkeleton() {
  return (
    <div className="rounded-lg border border-border bg-card p-6">
      <div className="h-4 w-24 animate-pulse rounded bg-muted" />
      <div className="mt-2 h-8 w-16 animate-pulse rounded bg-muted" />
      <div className="mt-1 h-4 w-20 animate-pulse rounded bg-muted" />
      <div className="mt-2 h-3 w-28 animate-pulse rounded bg-muted" />
    </div>
  )
}

function TransactionRow({
  transaction,
}: {
  transaction: TransactionWithNodes
}) {
  const statusColors = {
    verified:
      'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
    pending:
      'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
    flagged: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    rejected:
      'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400',
  }

  const formatAmount = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  return (
    <Link
      to="/transactions/$id"
      params={{ id: transaction.id }}
      className="flex items-center justify-between rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent"
    >
      <div className="flex-1">
        <p className="font-medium">{transaction.description}</p>
        <p className="text-sm text-muted-foreground">
          {transaction.from_node?.name ?? 'Unknown'} →{' '}
          {transaction.to_node?.name ?? 'Unknown'}
        </p>
      </div>
      <div className="flex items-center gap-4">
        <span className="font-semibold">
          {formatAmount(transaction.amount, transaction.currency)}
        </span>
        <span
          className={`rounded-full px-2 py-1 text-xs font-medium ${statusColors[transaction.verification_status]}`}
        >
          {transaction.verification_status}
        </span>
      </div>
    </Link>
  )
}
