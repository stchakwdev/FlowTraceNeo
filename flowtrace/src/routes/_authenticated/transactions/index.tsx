import { createFileRoute, Link } from '@tanstack/react-router'
import { useState } from 'react'

export const Route = createFileRoute('/_authenticated/transactions/')({
  component: TransactionList,
})

// Mock data for development
const mockTransactions = [
  {
    id: '1',
    description: 'Office Supplies Purchase',
    amount: 15000,
    currency: 'KES',
    from: 'Operations Budget',
    to: 'Staples Kenya',
    date: '2024-12-18',
    status: 'verified' as const,
    category: 'Operations',
  },
  {
    id: '2',
    description: 'Community Health Training',
    amount: 250000,
    currency: 'KES',
    from: 'USAID Grant',
    to: 'Training Department',
    date: '2024-12-15',
    status: 'pending' as const,
    category: 'Programs',
  },
  {
    id: '3',
    description: 'Vehicle Maintenance',
    amount: 45000,
    currency: 'KES',
    from: 'Operations Budget',
    to: 'AutoXpress Ltd',
    date: '2024-12-12',
    status: 'verified' as const,
    category: 'Transport',
  },
  {
    id: '4',
    description: 'Staff Salary - November',
    amount: 1200000,
    currency: 'KES',
    from: 'General Fund',
    to: 'Payroll',
    date: '2024-11-30',
    status: 'verified' as const,
    category: 'Salaries',
  },
  {
    id: '5',
    description: 'Suspicious Cash Withdrawal',
    amount: 500000,
    currency: 'KES',
    from: 'Project Fund',
    to: 'Unknown',
    date: '2024-12-10',
    status: 'flagged' as const,
    category: 'Unclassified',
  },
]

function TransactionList() {
  const [filter, setFilter] = useState<'all' | 'pending' | 'verified' | 'flagged'>('all')
  const [search, setSearch] = useState('')

  const filteredTransactions = mockTransactions.filter((t) => {
    const matchesFilter = filter === 'all' || t.status === filter
    const matchesSearch =
      t.description.toLowerCase().includes(search.toLowerCase()) ||
      t.from.toLowerCase().includes(search.toLowerCase()) ||
      t.to.toLowerCase().includes(search.toLowerCase())
    return matchesFilter && matchesSearch
  })

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
    }).format(amount)
  }

  const getStatusBadge = (status: 'pending' | 'verified' | 'flagged') => {
    const styles = {
      pending: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
      verified: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
      flagged: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    }
    return (
      <span className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ${styles[status]}`}>
        {status.charAt(0).toUpperCase() + status.slice(1)}
      </span>
    )
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold">Transactions</h1>
          <p className="text-muted-foreground">
            Track and verify all financial transactions
          </p>
        </div>
        <Link
          to="/transactions/new"
          className="inline-flex items-center justify-center rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          <PlusIcon className="mr-2 h-4 w-4" />
          Add Transaction
        </Link>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center">
        <div className="relative flex-1">
          <SearchIcon className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search transactions..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full rounded-lg border border-border bg-background py-2 pl-10 pr-4 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
        <div className="flex gap-2">
          {(['all', 'pending', 'verified', 'flagged'] as const).map((status) => (
            <button
              key={status}
              onClick={() => setFilter(status)}
              className={`rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                filter === status
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-card text-muted-foreground hover:bg-accent'
              }`}
            >
              {status.charAt(0).toUpperCase() + status.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Stats Row */}
      <div className="mb-6 grid gap-4 sm:grid-cols-4">
        <div className="rounded-lg border border-border bg-card p-4">
          <p className="text-sm text-muted-foreground">Total</p>
          <p className="text-2xl font-bold">{mockTransactions.length}</p>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <p className="text-sm text-muted-foreground">Pending</p>
          <p className="text-2xl font-bold text-yellow-600">
            {mockTransactions.filter((t) => t.status === 'pending').length}
          </p>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <p className="text-sm text-muted-foreground">Verified</p>
          <p className="text-2xl font-bold text-green-600">
            {mockTransactions.filter((t) => t.status === 'verified').length}
          </p>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <p className="text-sm text-muted-foreground">Flagged</p>
          <p className="text-2xl font-bold text-red-600">
            {mockTransactions.filter((t) => t.status === 'flagged').length}
          </p>
        </div>
      </div>

      {/* Transaction List */}
      <div className="rounded-lg border border-border bg-card">
        {filteredTransactions.length === 0 ? (
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
                className="flex items-center justify-between p-4 transition-colors hover:bg-accent"
              >
                <div className="flex-1">
                  <div className="flex items-center gap-3">
                    <p className="font-medium">{transaction.description}</p>
                    {getStatusBadge(transaction.status)}
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-sm text-muted-foreground">
                    <span>{transaction.from}</span>
                    <ArrowRightIcon className="h-3 w-3" />
                    <span>{transaction.to}</span>
                    <span className="mx-2">•</span>
                    <span>{transaction.date}</span>
                  </div>
                </div>
                <div className="text-right">
                  <p className="font-semibold">
                    {formatCurrency(transaction.amount, transaction.currency)}
                  </p>
                  <p className="text-xs text-muted-foreground">{transaction.category}</p>
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// Simple icon components
function PlusIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
    </svg>
  )
}

function SearchIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
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
