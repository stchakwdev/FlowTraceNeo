import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import {
  ArrowRight,
  MagnifyingGlass,
  Plus,
} from '@phosphor-icons/react'
import { useOrgStats, useTransactions } from '@/lib/queries'

export const Route = createFileRoute('/_authenticated/transactions/')({
  component: TransactionList,
})

function TransactionList() {
  const [filter, setFilter] = useState<
    'all' | 'pending' | 'verified' | 'flagged'
  >('all')
  const [search, setSearch] = useState('')

  const { data: transactions, isLoading, error } = useTransactions()
  const { data: stats } = useOrgStats()

  const filteredTransactions = useMemo(() => {
    if (!transactions) return []
    return transactions.filter((t) => {
      const matchesFilter = filter === 'all' || t.verification_status === filter
      const fromName = t.from_node?.name ?? ''
      const toName = t.to_node?.name ?? ''
      const matchesSearch =
        t.description.toLowerCase().includes(search.toLowerCase()) ||
        fromName.toLowerCase().includes(search.toLowerCase()) ||
        toName.toLowerCase().includes(search.toLowerCase())
      return matchesFilter && matchesSearch
    })
  }, [transactions, filter, search])

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
    }).format(amount)
  }

  const statusDot = {
    pending: 'bg-warning',
    verified: 'bg-success',
    flagged: 'bg-destructive',
    rejected: 'bg-muted-foreground',
  }

  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="section-label">FINANCE</p>
          <h1 className="mt-1 font-display text-2xl font-bold">Transactions</h1>
          <p className="text-sm text-muted-foreground">
            Track and verify all financial transactions
          </p>
        </div>
        <Link
          to="/transactions/new"
          className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          <Plus className="h-4 w-4" />
          Add Transaction
        </Link>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center">
        <div className="relative flex-1">
          <MagnifyingGlass className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search transactions..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full rounded-lg border border-border bg-background py-2 pl-10 pr-4 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
        <div className="flex gap-1 rounded-lg border border-border p-0.5">
          {(['all', 'pending', 'verified', 'flagged'] as const).map(
            (status) => (
              <button
                key={status}
                onClick={() => setFilter(status)}
                className={`rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
                  filter === status
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted'
                }`}
              >
                {status.charAt(0).toUpperCase() + status.slice(1)}
              </button>
            ),
          )}
        </div>
      </div>

      {/* Stats Row - with left accent bars */}
      <div className="mb-6 grid gap-4 sm:grid-cols-4">
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-primary" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Total</p>
            <p className="financial-figure text-2xl font-bold">{stats?.total ?? 0}</p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-warning" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Pending</p>
            <p className="financial-figure text-2xl font-bold">
              {stats?.pending ?? 0}
            </p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-success" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Verified</p>
            <p className="financial-figure text-2xl font-bold">
              {stats?.verified ?? 0}
            </p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-destructive" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Flagged</p>
            <p className="financial-figure text-2xl font-bold">
              {stats?.flagged ?? 0}
            </p>
          </div>
        </div>
      </div>

      {/* Transaction List */}
      <div className="rounded-lg border border-border bg-card">
        {isLoading ? (
          <div className="divide-y divide-border">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="flex items-center justify-between p-4">
                <div className="flex-1 space-y-2">
                  <div className="h-5 w-48 skeleton-shimmer rounded" />
                  <div className="h-4 w-64 skeleton-shimmer rounded" />
                </div>
                <div className="h-6 w-24 skeleton-shimmer rounded" />
              </div>
            ))}
          </div>
        ) : error ? (
          <div className="p-8 text-center">
            <p className="text-destructive">Error loading transactions</p>
          </div>
        ) : filteredTransactions.length === 0 ? (
          <div className="p-8 text-center">
            <p className="text-muted-foreground">No transactions found</p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {filteredTransactions.map((transaction) => (
              <Link
                key={transaction.id}
                to="/transactions/$id"
                params={{ id: transaction.id }}
                className="flex items-center justify-between p-4 transition-colors hover:bg-muted"
              >
                <div className="flex-1">
                  <div className="flex items-center gap-3">
                    <p className="font-medium">{transaction.description}</p>
                    <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                      <span
                        className={`inline-flex h-1.5 w-1.5 rounded-full ${statusDot[transaction.verification_status]}`}
                      />
                      {transaction.verification_status}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-sm text-muted-foreground">
                    <span>{transaction.from_node?.name ?? 'Unknown'}</span>
                    <ArrowRight className="h-3 w-3" />
                    <span>{transaction.to_node?.name ?? 'Unknown'}</span>
                    <span className="mx-2">&middot;</span>
                    <span>{transaction.transaction_date}</span>
                  </div>
                </div>
                <div className="text-right">
                  <p className="financial-figure font-semibold">
                    {formatCurrency(transaction.amount, transaction.currency)}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {transaction.category ?? 'Uncategorized'}
                  </p>
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
