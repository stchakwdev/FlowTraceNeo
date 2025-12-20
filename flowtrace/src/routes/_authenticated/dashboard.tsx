import { createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@workos-inc/authkit-react'
import { getUserDisplayName, getUserInitials } from '@/lib/auth'

export const Route = createFileRoute('/_authenticated/dashboard')({
  component: Dashboard,
})

function Dashboard() {
  const { user, signOut } = useAuth()

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border bg-card">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-4">
          <div className="flex items-center gap-2">
            <div className="h-8 w-8 rounded-lg bg-primary" />
            <span className="text-xl font-bold">FlowTrace</span>
          </div>

          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-sm font-medium text-primary-foreground">
                {getUserInitials(user as any)}
              </div>
              <span className="text-sm font-medium">
                {getUserDisplayName(user as any)}
              </span>
            </div>
            <button
              onClick={() => signOut()}
              className="rounded-lg border border-border px-3 py-1.5 text-sm font-medium hover:bg-accent"
            >
              Sign Out
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="mx-auto max-w-7xl px-4 py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold">Dashboard</h1>
          <p className="text-muted-foreground">
            Welcome back, {getUserDisplayName(user as any)}
          </p>
        </div>

        {/* Stats Cards */}
        <div className="mb-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <StatCard
            title="Trust Score"
            value="87"
            subtitle="A Grade"
            trend="+2 this week"
          />
          <StatCard
            title="Total Transactions"
            value="1,234"
            subtitle="This month"
            trend="+12% from last month"
          />
          <StatCard
            title="Verified"
            value="92%"
            subtitle="Verification rate"
            trend="+5% from last month"
          />
          <StatCard
            title="Pending"
            value="23"
            subtitle="Awaiting verification"
            trend="Action required"
          />
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

        {/* Recent Transactions Placeholder */}
        <div>
          <h2 className="mb-4 text-xl font-semibold">Recent Transactions</h2>
          <div className="rounded-lg border border-border bg-card p-8 text-center">
            <p className="text-muted-foreground">
              No transactions yet. Add your first transaction to get started.
            </p>
          </div>
        </div>
      </main>
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
    <a
      href={href}
      className="block rounded-lg border border-border bg-card p-6 transition-colors hover:border-primary hover:bg-accent"
    >
      <h3 className="font-semibold">{title}</h3>
      <p className="mt-1 text-sm text-muted-foreground">{description}</p>
    </a>
  )
}
