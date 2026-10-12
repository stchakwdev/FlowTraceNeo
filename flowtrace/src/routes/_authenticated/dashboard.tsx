import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  CurrencyCircleDollar,
  FileArrowUp,
  FlowArrow,
  Plus,
} from '@phosphor-icons/react'
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
    <div className="px-6 py-6">
      {/* Header */}
      <div className="mb-8">
        <p className="section-label">OVERVIEW</p>
        <h1 className="mt-1 font-display text-2xl font-bold">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          Welcome back{organization?.name ? `, ${organization.name}` : ''}
        </p>
      </div>

      {/* Stat Cards */}
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
              accentColor="bg-[oklch(0.65_0.2_145)]"
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
              accentColor="bg-primary"
              title="Total Transactions"
              value={stats?.total.toLocaleString() ?? '0'}
              subtitle="All time"
              trend={`${stats?.verified ?? 0} verified`}
              trendPositive
            />
            <StatCard
              accentColor="bg-[oklch(0.5_0.15_280)]"
              title="Verified"
              value={`${stats?.verificationRate ?? 0}%`}
              subtitle="Verification rate"
              trend={`${stats?.verified ?? 0} of ${stats?.total ?? 0} transactions`}
              trendPositive={(stats?.verificationRate ?? 0) >= 70}
            />
            <StatCard
              accentColor="bg-accent"
              title="Pending"
              value={String(stats?.pending ?? 0)}
              subtitle="Awaiting verification"
              trend={
                stats?.flagged
                  ? `${stats.flagged} flagged`
                  : 'No flagged items'
              }
              trendPositive={!stats?.flagged}
            />
          </>
        )}
      </div>

      {/* Quick Actions - Horizontal scrollable ribbon */}
      <div className="mb-8">
        <h2 className="mb-3 font-display text-lg font-semibold">
          Quick Actions
        </h2>
        <div className="flex gap-3 overflow-x-auto pb-2">
          <ActionPill
            icon={<Plus className="h-4 w-4" weight="bold" />}
            label="Add Transaction"
            href="/transactions/new"
          />
          <ActionPill
            icon={<FlowArrow className="h-4 w-4" weight="bold" />}
            label="View Flow Diagram"
            href="/flows"
          />
          <ActionPill
            icon={<FileArrowUp className="h-4 w-4" weight="bold" />}
            label="Upload Verification"
            href="/transactions?filter=pending"
          />
        </div>
      </div>

      {/* Recent Transactions */}
      <div>
        <div className="mb-4 flex items-center justify-between">
          <h2 className="font-display text-lg font-semibold">
            Recent Transactions
          </h2>
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
              <div
                key={i}
                className="flex items-center gap-4 rounded-lg border border-border bg-card p-4"
              >
                <div className="h-10 w-10 skeleton-shimmer rounded-lg" />
                <div className="flex-1 space-y-2">
                  <div className="h-4 w-48 skeleton-shimmer rounded" />
                  <div className="h-3 w-32 skeleton-shimmer rounded" />
                </div>
                <div className="h-6 w-20 skeleton-shimmer rounded" />
              </div>
            ))}
          </div>
        ) : recentTransactions.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border bg-card/50 p-12 text-center">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-muted">
              <CurrencyCircleDollar
                className="h-8 w-8 text-muted-foreground"
                weight="duotone"
              />
            </div>
            <h3 className="mt-4 font-display font-semibold">
              No transactions yet
            </h3>
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
  accentColor,
  title,
  value,
  subtitle,
  trend,
  trendPositive,
}: {
  accentColor: string
  title: string
  value: string
  subtitle: string
  trend: string
  trendPositive?: boolean
}) {
  return (
    <div className="relative overflow-hidden rounded-lg border border-border bg-card p-5 transition-all hover:-translate-y-0.5 hover:shadow-md">
      {/* Left accent bar */}
      <div
        className={`absolute left-0 top-0 h-full w-1 ${accentColor}`}
      />
      <div className="pl-2">
        <p className="text-sm font-medium text-muted-foreground">{title}</p>
        <p className="mt-1 font-display text-3xl font-bold tracking-tight">
          {value}
        </p>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
        <div className="mt-2 flex items-center gap-1.5 text-xs">
          {trendPositive ? (
            <span className="inline-flex h-1.5 w-1.5 rounded-full bg-[oklch(0.65_0.2_145)]" />
          ) : (
            <span className="inline-flex h-1.5 w-1.5 rounded-full bg-accent" />
          )}
          <span className="text-muted-foreground">{trend}</span>
        </div>
      </div>
    </div>
  )
}

function ActionPill({
  icon,
  label,
  href,
}: {
  icon: React.ReactNode
  label: string
  href: string
}) {
  return (
    <Link
      to={href}
      className="flex shrink-0 items-center gap-2 rounded-full border border-border bg-card px-4 py-2 text-sm font-medium transition-colors hover:bg-muted"
    >
      <span className="text-primary">{icon}</span>
      {label}
      <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
    </Link>
  )
}

function StatCardSkeleton() {
  return (
    <div className="relative overflow-hidden rounded-lg border border-border bg-card p-5">
      <div className="absolute left-0 top-0 h-full w-1 skeleton-shimmer" />
      <div className="pl-2">
        <div className="h-4 w-24 skeleton-shimmer rounded" />
        <div className="mt-2 h-8 w-20 skeleton-shimmer rounded" />
        <div className="mt-2 h-4 w-28 skeleton-shimmer rounded" />
        <div className="mt-2 h-3 w-32 skeleton-shimmer rounded" />
      </div>
    </div>
  )
}

function TransactionRow({
  transaction,
}: {
  transaction: TransactionWithNodes
}) {
  const statusDot = {
    verified: 'bg-[oklch(0.65_0.2_145)]',
    pending: 'bg-accent',
    flagged: 'bg-destructive',
    rejected: 'bg-muted-foreground',
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
      className="group flex items-center gap-4 rounded-lg border border-border bg-card p-4 transition-all hover:-translate-y-0.5 hover:shadow-md"
    >
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
        <CurrencyCircleDollar
          className="h-5 w-5 text-primary"
          weight="duotone"
        />
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium">{transaction.description}</p>
        <p className="truncate text-sm text-muted-foreground">
          {transaction.from_node?.name ?? 'Unknown'} &rarr;{' '}
          {transaction.to_node?.name ?? 'Unknown'}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <span className="financial-figure font-semibold">
          {formatAmount(transaction.amount, transaction.currency)}
        </span>
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span
            className={`inline-flex h-1.5 w-1.5 rounded-full ${statusDot[transaction.verification_status]}`}
          />
          {transaction.verification_status}
        </span>
        <ArrowRight className="h-4 w-4 text-muted-foreground/50 transition-all group-hover:translate-x-0.5 group-hover:text-primary" />
      </div>
    </Link>
  )
}
