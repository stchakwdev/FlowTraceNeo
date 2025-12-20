import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'

export const Route = createFileRoute('/_authenticated/trust-score')({
  component: TrustScore,
})

// Mock trust score data
const mockTrustData = {
  overallScore: 87,
  grade: 'A',
  lastUpdated: '2024-12-20T10:00:00Z',
  components: {
    verification: { score: 92, weight: 40, label: 'Transaction Verification' },
    documentation: { score: 85, weight: 30, label: 'Documentation Quality' },
    consistency: { score: 82, weight: 30, label: 'Financial Consistency' },
  },
  history: [
    { month: 'Jul', score: 72 },
    { month: 'Aug', score: 75 },
    { month: 'Sep', score: 78 },
    { month: 'Oct', score: 82 },
    { month: 'Nov', score: 85 },
    { month: 'Dec', score: 87 },
  ],
  recommendations: [
    {
      id: '1',
      priority: 'high',
      title: 'Upload missing receipts',
      description: '23 transactions from November are missing verification documents',
      impact: '+5 points',
    },
    {
      id: '2',
      priority: 'medium',
      title: 'Complete vendor verification',
      description: '3 vendors have incomplete KYC documentation',
      impact: '+3 points',
    },
    {
      id: '3',
      priority: 'low',
      title: 'Add transaction notes',
      description: 'Adding context to transactions improves documentation score',
      impact: '+2 points',
    },
  ],
  breakdown: {
    verified: 234,
    pending: 23,
    flagged: 2,
    total: 259,
  },
}

function TrustScore() {
  const [selectedPeriod, setSelectedPeriod] = useState('6m')

  const getGradeColor = (grade: string) => {
    switch (grade) {
      case 'A+':
      case 'A':
        return 'text-green-600'
      case 'B':
        return 'text-blue-600'
      case 'C':
        return 'text-yellow-600'
      case 'D':
        return 'text-orange-600'
      default:
        return 'text-red-600'
    }
  }

  const getScoreColor = (score: number) => {
    if (score >= 80) return 'stroke-green-500'
    if (score >= 60) return 'stroke-yellow-500'
    return 'stroke-red-500'
  }

  const getPriorityBadge = (priority: string) => {
    const styles = {
      high: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
      medium: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
      low: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
    }
    return (
      <span
        className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${styles[priority as keyof typeof styles]}`}
      >
        {priority}
      </span>
    )
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-KE', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  // Calculate circumference for circular progress
  const radius = 80
  const circumference = 2 * Math.PI * radius
  const progress = (mockTrustData.overallScore / 100) * circumference

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Trust Score</h1>
        <p className="text-muted-foreground">
          Your organization&apos;s financial transparency rating
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Main Score Card */}
        <div className="lg:col-span-2">
          <div className="rounded-lg border border-border bg-card p-8">
            <div className="flex flex-col items-center gap-8 sm:flex-row">
              {/* Circular Progress */}
              <div className="relative">
                <svg width="200" height="200" className="-rotate-90">
                  {/* Background circle */}
                  <circle
                    cx="100"
                    cy="100"
                    r={radius}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="12"
                    className="text-muted/20"
                  />
                  {/* Progress circle */}
                  <circle
                    cx="100"
                    cy="100"
                    r={radius}
                    fill="none"
                    strokeWidth="12"
                    strokeLinecap="round"
                    strokeDasharray={circumference}
                    strokeDashoffset={circumference - progress}
                    className={`${getScoreColor(mockTrustData.overallScore)} transition-all duration-1000`}
                  />
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-5xl font-bold">{mockTrustData.overallScore}</span>
                  <span
                    className={`text-2xl font-bold ${getGradeColor(mockTrustData.grade)}`}
                  >
                    Grade {mockTrustData.grade}
                  </span>
                </div>
              </div>

              {/* Score Breakdown */}
              <div className="flex-1 space-y-4">
                <p className="text-sm text-muted-foreground">
                  Last updated: {formatDate(mockTrustData.lastUpdated)}
                </p>

                {Object.entries(mockTrustData.components).map(([key, data]) => (
                  <div key={key}>
                    <div className="mb-1 flex items-center justify-between">
                      <span className="text-sm font-medium">{data.label}</span>
                      <span className="text-sm">
                        {data.score}% ({data.weight}% weight)
                      </span>
                    </div>
                    <div className="h-2 overflow-hidden rounded-full bg-muted/20">
                      <div
                        className={`h-full rounded-full transition-all duration-500 ${
                          data.score >= 80
                            ? 'bg-green-500'
                            : data.score >= 60
                              ? 'bg-yellow-500'
                              : 'bg-red-500'
                        }`}
                        style={{ width: `${data.score}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* History Chart */}
          <div className="mt-6 rounded-lg border border-border bg-card p-6">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-lg font-semibold">Score History</h2>
              <div className="flex gap-2">
                {['3m', '6m', '1y'].map((period) => (
                  <button
                    key={period}
                    onClick={() => setSelectedPeriod(period)}
                    className={`rounded-lg px-3 py-1 text-sm font-medium transition-colors ${
                      selectedPeriod === period
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent'
                    }`}
                  >
                    {period}
                  </button>
                ))}
              </div>
            </div>

            {/* Simple bar chart */}
            <div className="flex h-48 items-end gap-4">
              {mockTrustData.history.map((item, i) => (
                <div key={i} className="flex flex-1 flex-col items-center">
                  <div
                    className={`w-full rounded-t-lg transition-all duration-500 ${
                      item.score >= 80
                        ? 'bg-green-500'
                        : item.score >= 60
                          ? 'bg-yellow-500'
                          : 'bg-red-500'
                    }`}
                    style={{ height: `${(item.score / 100) * 160}px` }}
                  />
                  <div className="mt-2 text-center">
                    <p className="text-xs font-medium">{item.score}</p>
                    <p className="text-xs text-muted-foreground">{item.month}</p>
                  </div>
                </div>
              ))}
            </div>

            <div className="mt-4 flex items-center justify-center gap-2 text-sm text-muted-foreground">
              <TrendUpIcon className="h-4 w-4 text-green-500" />
              <span>+15 points improvement over 6 months</span>
            </div>
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Quick Stats */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 text-lg font-semibold">Transaction Status</h2>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-3 w-3 rounded-full bg-green-500" />
                  <span className="text-sm">Verified</span>
                </div>
                <span className="font-medium">{mockTrustData.breakdown.verified}</span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-3 w-3 rounded-full bg-yellow-500" />
                  <span className="text-sm">Pending</span>
                </div>
                <span className="font-medium">{mockTrustData.breakdown.pending}</span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-3 w-3 rounded-full bg-red-500" />
                  <span className="text-sm">Flagged</span>
                </div>
                <span className="font-medium">{mockTrustData.breakdown.flagged}</span>
              </div>
              <div className="mt-4 border-t border-border pt-4">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">Total Transactions</span>
                  <span className="font-bold">{mockTrustData.breakdown.total}</span>
                </div>
                <div className="mt-2 flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Verification Rate</span>
                  <span className="font-medium text-green-600">
                    {Math.round(
                      (mockTrustData.breakdown.verified / mockTrustData.breakdown.total) *
                        100
                    )}
                    %
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* Recommendations */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 text-lg font-semibold">Recommendations</h2>
            <ul className="space-y-4">
              {mockTrustData.recommendations.map((rec) => (
                <li key={rec.id} className="border-b border-border pb-4 last:border-0 last:pb-0">
                  <div className="mb-1 flex items-center gap-2">
                    {getPriorityBadge(rec.priority)}
                    <span className="text-xs text-primary">{rec.impact}</span>
                  </div>
                  <p className="font-medium">{rec.title}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{rec.description}</p>
                </li>
              ))}
            </ul>
            <button className="mt-4 w-full rounded-lg border border-border py-2 text-sm font-medium hover:bg-accent">
              View All Recommendations
            </button>
          </div>

          {/* Share Badge */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 text-lg font-semibold">Trust Badge</h2>
            <div className="flex items-center justify-center rounded-lg bg-gradient-to-br from-primary/10 to-primary/5 p-6">
              <div className="text-center">
                <ShieldIcon className="mx-auto h-12 w-12 text-primary" />
                <p className="mt-2 text-sm font-medium">FlowTrace Verified</p>
                <p className={`text-2xl font-bold ${getGradeColor(mockTrustData.grade)}`}>
                  Grade {mockTrustData.grade}
                </p>
              </div>
            </div>
            <button className="mt-4 w-full rounded-lg bg-primary py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">
              Share Badge
            </button>
            <p className="mt-2 text-center text-xs text-muted-foreground">
              Embed on your website or share with donors
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

// Icon components
function TrendUpIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
      />
    </svg>
  )
}

function ShieldIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
      />
    </svg>
  )
}
