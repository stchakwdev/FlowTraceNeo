import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  CheckCircle,
  Clock,
  CurrencyCircleDollar,
  FileArrowUp,
  FlowArrow,
  Plus,
  ShieldCheck,
  TrendUp,
  Warning,
} from '@phosphor-icons/react'
import type { ReactNode } from 'react'
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
  const trustScore = organization?.trust_score ?? 0

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header with gradient accent */}
      <div className="mb-8">
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-primary/70 shadow-lg shadow-primary/20">
            <TrendUp className="h-6 w-6 text-primary-foreground" weight="bold" />
          </div>
          <div>
            <h1 className="text-3xl font-bold">Dashboard</h1>
            <p className="text-muted-foreground">
              Welcome back{organization?.name ? `, ${organization.name}` : ''}
            </p>
          </div>
        </div>
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
              icon={<ShieldCheck className="h-5 w-5" weight="duotone" />}
              iconColor="text-emerald-600 dark:text-emerald-400"
              iconBg="bg-emerald-100 dark:bg-emerald-900/30"
              title="Trust Score"
              value={String(trustScore)}
              subtitle={`${getTrustGrade(trustScore)} Grade`}
              trend={
                organization?.verified
                  ? 'Verified organization'
                  : 'Pending verification'
              }
              trendPositive={organization?.verified}
            />
            <StatCard
              icon={<CurrencyCircleDollar className="h-5 w-5" weight="duotone" />}
              iconColor="text-blue-600 dark:text-blue-400"
              iconBg="bg-blue-100 dark:bg-blue-900/30"
              title="Total Transactions"
              value={stats?.total.toLocaleString() ?? '0'}
              subtitle="All time"
              trend={`${stats?.verified ?? 0} verified`}
              trendPositive
            />
            <StatCard
              icon={<CheckCircle className="h-5 w-5" weight="duotone" />}
              iconColor="text-violet-600 dark:text-violet-400"
              iconBg="bg-violet-100 dark:bg-violet-900/30"
              title="Verified"
              value={`${stats?.verificationRate ?? 0}%`}
              subtitle="Verification rate"
              trend={`${stats?.verified ?? 0} of ${stats?.total ?? 0} transactions`}
              trendPositive={(stats?.verificationRate ?? 0) >= 70}
            />
            <StatCard
              icon={<Clock className="h-5 w-5" weight="duotone" />}
              iconColor="text-amber-600 dark:text-amber-400"
              iconBg="bg-amber-100 dark:bg-amber-900/30"
              title="Pending"
              value={String(stats?.pending ?? 0)}
              subtitle="Awaiting verification"
              trend={
                stats?.flagged ? `${stats.flagged} flagged` : 'No flagged items'
              }
              trendPositive={!stats?.flagged}
            />
          </>
        )}
      </div>

      {/* Quick Actions */}
      <div className="mb-8">
        <h2 className="mb-4 text-xl font-semibold">Quick Actions</h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <ActionCard
            icon={<Plus className="h-5 w-5" weight="bold" />}
            iconBg="bg-emerald-100 dark:bg-emerald-900/30"
            iconColor="text-emerald-600 dark:text-emerald-400"
            title="Add Transaction"
            description="Record a new financial transaction"
            href="/transactions/new"
          />
          <ActionCard
            icon={<FlowArrow className="h-5 w-5" weight="bold" />}
            iconBg="bg-blue-100 dark:bg-blue-900/30"
            iconColor="text-blue-600 dark:text-blue-400"
            title="View Flow Diagram"
            description="Visualize money flow through your organization"
            href="/flows"
          />
          <ActionCard
            icon={<FileArrowUp className="h-5 w-5" weight="bold" />}
            iconBg="bg-violet-100 dark:bg-violet-900/30"
            iconColor="text-violet-600 dark:text-violet-400"
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
          <div className="space-y-3">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="flex items-center gap-4 rounded-xl border border-border bg-card p-4">
                <div className="h-10 w-10 animate-pulse rounded-lg bg-muted" />
                <div className="flex-1 space-y-2">
                  <div className="h-4 w-48 animate-pulse rounded bg-muted" />
                  <div className="h-3 w-32 animate-pulse rounded bg-muted" />
                </div>
                <div className="h-6 w-20 animate-pulse rounded bg-muted" />
              </div>
            ))}
          </div>
        ) : recentTransactions.length === 0 ? (
          <div className="rounded-xl border border-dashed border-border bg-card/50 p-12 text-center">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-muted">
              <CurrencyCircleDollar className="h-8 w-8 text-muted-foreground" weight="duotone" />
            </div>
            <h3 className="mt-4 font-semibold">No transactions yet</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Add your first transaction to start tracking your finances.
            </p>
            <Link
              to="/transactions/new"
              className="mt-4 inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
            >
              <Plus className="h-4 w-4" />
              Add Transaction
            </Link>
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
  icon,
  iconColor,
  iconBg,
  title,
  value,
  subtitle,
  trend,
  trendPositive,
}: {
  icon: ReactNode
  iconColor: string
  iconBg: string
  title: string
  value: string
  subtitle: string
  trend: string
  trendPositive?: boolean
}) {
  return (
    <div className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5">
      {/* Subtle gradient overlay on hover */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

      <div className="relative">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium text-muted-foreground">{title}</p>
          <div className={`flex h-9 w-9 items-center justify-center rounded-lg ${iconBg}`}>
            <span className={iconColor}>{icon}</span>
          </div>
        </div>
        <p className="mt-2 text-3xl font-bold tracking-tight">{value}</p>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
        <div className="mt-3 flex items-center gap-1.5">
          <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${
            trendPositive
              ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400'
              : 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400'
          }`}>
            {trendPositive ? (
              <CheckCircle className="h-3 w-3" weight="fill" />
            ) : (
              <Warning className="h-3 w-3" weight="fill" />
            )}
            {trend}
          </span>
        </div>
      </div>
    </div>
  )
}

function ActionCard({
  icon,
  iconBg,
  iconColor,
  title,
  description,
  href,
}: {
  icon: ReactNode
  iconBg: string
  iconColor: string
  title: string
  description: string
  href: string
}) {
  return (
    <Link
      to={href}
      className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
    >
      {/* Subtle gradient overlay on hover */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

      <div className="relative flex items-start gap-4">
        <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ${iconBg} transition-transform group-hover:scale-110`}>
          <span className={iconColor}>{icon}</span>
        </div>
        <div className="flex-1">
          <h3 className="font-semibold">{title}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        </div>
        <ArrowRight className="h-5 w-5 text-muted-foreground/50 transition-all group-hover:translate-x-1 group-hover:text-primary" />
      </div>
    </Link>
  )
}

function StatCardSkeleton() {
  return (
    <div className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between">
        <div className="h-4 w-24 animate-pulse rounded bg-muted" />
        <div className="h-9 w-9 animate-pulse rounded-lg bg-muted" />
      </div>
      <div className="mt-3 h-8 w-20 animate-pulse rounded bg-muted" />
      <div className="mt-2 h-4 w-28 animate-pulse rounded bg-muted" />
      <div className="mt-3 h-5 w-32 animate-pulse rounded-full bg-muted" />
    </div>
  )
}

function TransactionRow({
  transaction,
}: {
  transaction: TransactionWithNodes
}) {
  const statusConfig = {
    verified: {
      bg: 'bg-emerald-100 dark:bg-emerald-900/30',
      text: 'text-emerald-700 dark:text-emerald-400',
      icon: <CheckCircle className="h-3 w-3" weight="fill" />,
    },
    pending: {
      bg: 'bg-amber-100 dark:bg-amber-900/30',
      text: 'text-amber-700 dark:text-amber-400',
      icon: <Clock className="h-3 w-3" weight="fill" />,
    },
    flagged: {
      bg: 'bg-red-100 dark:bg-red-900/30',
      text: 'text-red-700 dark:text-red-400',
      icon: <Warning className="h-3 w-3" weight="fill" />,
    },
    rejected: {
      bg: 'bg-gray-100 dark:bg-gray-900/30',
      text: 'text-gray-700 dark:text-gray-400',
      icon: <Warning className="h-3 w-3" weight="fill" />,
    },
  }

  const status = statusConfig[transaction.verification_status]

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
      className="group flex items-center gap-4 rounded-xl border border-border bg-card p-4 transition-all hover:border-primary/30 hover:shadow-md hover:shadow-primary/5"
    >
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
        <CurrencyCircleDollar className="h-5 w-5 text-primary" weight="duotone" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="font-medium truncate">{transaction.description}</p>
        <p className="text-sm text-muted-foreground truncate">
          {transaction.from_node?.name ?? 'Unknown'} → {transaction.to_node?.name ?? 'Unknown'}
        </p>
      </div>
      <div className="flex items-center gap-3 shrink-0">
        <span className="font-semibold tabular-nums">
          {formatAmount(transaction.amount, transaction.currency)}
        </span>
        <span
          className={`inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium ${status.bg} ${status.text}`}
        >
          {status.icon}
          {transaction.verification_status}
        </span>
        <ArrowRight className="h-4 w-4 text-muted-foreground/50 transition-all group-hover:translate-x-0.5 group-hover:text-primary" />
      </div>
    </Link>
  )
}
