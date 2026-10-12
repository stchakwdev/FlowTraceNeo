import { createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import {
  ArrowsClockwise,
  Check,
  Shield,
  TrendUp,
  TrendDown,
  CheckCircle,
  Clock,
  XCircle,
} from '@phosphor-icons/react'
import { useOrg } from '@/lib/org-context'
import { useOrgStats, useRecalculateTrustScore, useTrustHistory } from '@/lib/queries'
import { getTrustGrade } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/trust-score')({
  component: TrustScore,
})

function TrustScore() {
  const [selectedPeriod, setSelectedPeriod] = useState('6m')

  const { organization, isLoading: orgLoading, refetch: refetchOrg } = useOrg()
  const { data: stats, isLoading: statsLoading } = useOrgStats()
  const { data: trustHistory, isLoading: historyLoading, refetch: refetchHistory } = useTrustHistory()

  const recalculateMutation = useRecalculateTrustScore()

  const handleRecalculate = async () => {
    try {
      await recalculateMutation.mutateAsync()
      await Promise.all([refetchOrg(), refetchHistory()])
    } catch (error) {
      console.error('Failed to recalculate trust score:', error)
    }
  }

  const isLoading = orgLoading || statsLoading || historyLoading

  const trustData = useMemo(() => {
    const overallScore = organization?.trust_score ?? 0
    const grade = getTrustGrade(overallScore)

    const latestHistory = trustHistory?.[trustHistory.length - 1]
    const proofCoverage = stats?.proofStats?.proofCoverageRate ?? 0

    const components = {
      verification: {
        score: latestHistory?.verification_rate ?? stats?.verificationRate ?? 0,
        weight: 40,
        label: 'Transaction Verification',
      },
      documentation: {
        score: latestHistory?.documentation_rate ?? proofCoverage,
        weight: 30,
        label: 'Proof Documentation',
      },
      consistency: {
        score: latestHistory?.consistency_score ?? 80,
        weight: 30,
        label: 'Financial Consistency',
      },
    }

    const history = (trustHistory ?? []).map((h) => {
      const date = new Date(h.calculated_at)
      return {
        month: date.toLocaleDateString('en-US', { month: 'short' }),
        score: h.score,
      }
    })

    const recommendations: { id: string; priority: 'high' | 'medium' | 'low'; title: string; description: string; impact: string }[] = []
    if (stats && stats.pending > 0) {
      recommendations.push({
        id: '1',
        priority: 'high',
        title: 'Verify pending transactions',
        description: `${stats.pending} transactions are awaiting verification`,
        impact: '+5 points',
      })
    }
    if (stats && stats.flagged > 0) {
      recommendations.push({
        id: '2',
        priority: 'high',
        title: 'Review flagged transactions',
        description: `${stats.flagged} transactions have been flagged for review`,
        impact: '+3 points',
      })
    }
    if (proofCoverage < 50) {
      recommendations.push({
        id: '3',
        priority: 'high',
        title: 'Add proof documentation',
        description: `Only ${proofCoverage}% of transactions have proof attached. Add receipts, photos, or other evidence.`,
        impact: '+10 points',
      })
    } else if (proofCoverage < 80) {
      recommendations.push({
        id: '3',
        priority: 'medium',
        title: 'Improve proof coverage',
        description: `${100 - proofCoverage}% of transactions are missing proof documentation`,
        impact: '+5 points',
      })
    }
    if (stats?.proofStats && stats.proofStats.pendingProofs > 0) {
      recommendations.push({
        id: '4',
        priority: 'medium',
        title: 'Review pending proofs',
        description: `${stats.proofStats.pendingProofs} proof documents are awaiting verification`,
        impact: '+3 points',
      })
    }

    return {
      overallScore,
      grade,
      lastUpdated: latestHistory?.calculated_at ?? new Date().toISOString(),
      components,
      history,
      recommendations,
      breakdown: {
        verified: stats?.verified ?? 0,
        pending: stats?.pending ?? 0,
        flagged: stats?.flagged ?? 0,
        total: stats?.total ?? 0,
      },
      proofStats: stats?.proofStats ?? {
        totalProofs: 0,
        verifiedProofs: 0,
        pendingProofs: 0,
        rejectedProofs: 0,
        totalTrustPoints: 0,
        transactionsWithProofs: 0,
        proofCoverageRate: 0,
      },
    }
  }, [organization, stats, trustHistory])

  const getGradeColor = (grade: string) => {
    switch (grade) {
      case 'A+':
      case 'A':
        return 'text-success'
      case 'B':
        return 'text-primary'
      case 'C':
        return 'text-warning'
      case 'D':
        return 'text-accent'
      default:
        return 'text-destructive'
    }
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

  // Circular progress
  const radius = 80
  const circumference = 2 * Math.PI * radius
  const progress = (trustData.overallScore / 100) * circumference

  const scoreImprovement = useMemo(() => {
    if (trustData.history.length < 2) return 0
    return (
      trustData.history[trustData.history.length - 1].score -
      trustData.history[0].score
    )
  }, [trustData.history])

  if (isLoading) {
    return (
      <div className="px-6 py-6">
        <div className="mb-8">
          <p className="section-label">ANALYTICS</p>
          <h1 className="mt-1 font-display text-2xl font-bold">Trust Score</h1>
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="lg:col-span-2">
            <div className="h-64 skeleton-shimmer rounded-lg" />
          </div>
          <div className="h-64 skeleton-shimmer rounded-lg" />
        </div>
      </div>
    )
  }

  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="mb-8 flex items-start justify-between">
        <div>
          <p className="section-label">ANALYTICS</p>
          <h1 className="mt-1 font-display text-2xl font-bold">Trust Score</h1>
          <p className="text-sm text-muted-foreground">
            Your organization&apos;s financial transparency rating
          </p>
        </div>
        <button
          onClick={handleRecalculate}
          disabled={recalculateMutation.isPending}
          className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
        >
          <ArrowsClockwise
            className={`h-4 w-4 ${recalculateMutation.isPending ? 'animate-spin' : ''}`}
          />
          {recalculateMutation.isPending ? 'Calculating...' : 'Recalculate'}
        </button>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Main Score Card */}
        <div className="lg:col-span-2">
          <div className="rounded-lg border border-border bg-card p-8">
            <div className="flex flex-col items-center gap-8 sm:flex-row">
              {/* Circular Progress with gradient stroke */}
              <div className="relative">
                <svg width="200" height="200" className="-rotate-90">
                  <defs>
                    <linearGradient id="scoreGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                      <stop offset="0%" stopColor="oklch(0.55 0.15 175)" />
                      <stop offset="100%" stopColor="oklch(0.72 0.16 45)" />
                    </linearGradient>
                  </defs>
                  {/* Background circle */}
                  <circle
                    cx="100"
                    cy="100"
                    r={radius}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="8"
                    className="text-muted/20"
                  />
                  {/* Progress circle */}
                  <circle
                    cx="100"
                    cy="100"
                    r={radius}
                    fill="none"
                    stroke="url(#scoreGradient)"
                    strokeWidth="8"
                    strokeLinecap="round"
                    strokeDasharray={circumference}
                    strokeDashoffset={circumference - progress}
                    className="transition-all duration-1000"
                  />
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="font-display text-5xl font-bold">
                    {trustData.overallScore}
                  </span>
                  <span
                    className={`font-display text-2xl font-bold ${getGradeColor(trustData.grade)}`}
                  >
                    Grade {trustData.grade}
                  </span>
                </div>
              </div>

              {/* Score Components - battery-indicator style */}
              <div className="flex-1 space-y-5">
                <p className="text-sm text-muted-foreground">
                  Last updated: {formatDate(trustData.lastUpdated)}
                </p>

                {Object.entries(trustData.components).map(([key, data]) => (
                  <div key={key}>
                    <div className="mb-1.5 flex items-center justify-between">
                      <span className="text-sm font-medium">{data.label}</span>
                      <span className="financial-figure text-sm">
                        {Math.round(data.score)}% ({data.weight}% weight)
                      </span>
                    </div>
                    {/* Segmented battery indicator */}
                    <div className="flex gap-0.5">
                      {Array.from({ length: 10 }).map((_, i) => (
                        <div
                          key={i}
                          className={`h-2 flex-1 rounded-sm transition-all duration-500 ${
                            i < Math.round(data.score / 10)
                              ? data.score >= 80
                                ? 'bg-success'
                                : data.score >= 60
                                  ? 'bg-warning'
                                  : 'bg-destructive'
                              : 'bg-muted/20'
                          }`}
                        />
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* History Chart */}
          <div className="mt-6 rounded-lg border border-border bg-card p-6">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="font-display text-lg font-semibold">Score History</h2>
              <div className="flex gap-1 rounded-lg border border-border p-0.5">
                {['3m', '6m', '1y'].map((period) => (
                  <button
                    key={period}
                    onClick={() => setSelectedPeriod(period)}
                    className={`rounded-md px-3 py-1 text-sm font-medium transition-colors ${
                      selectedPeriod === period
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted'
                    }`}
                  >
                    {period}
                  </button>
                ))}
              </div>
            </div>

            {/* Bar chart with thinner bars */}
            <div className="flex h-48 items-end gap-4">
              {trustData.history.length === 0 ? (
                <div className="flex flex-1 items-center justify-center">
                  <p className="text-muted-foreground">
                    No history data available
                  </p>
                </div>
              ) : (
                trustData.history.map((item, i) => (
                  <div key={i} className="flex flex-1 flex-col items-center">
                    <div
                      className="w-1.5 rounded-t-full bg-primary transition-all duration-500"
                      style={{ height: `${(item.score / 100) * 160}px` }}
                    />
                    <div className="mt-2 text-center">
                      <p className="financial-figure text-xs font-medium">{item.score}</p>
                      <p className="text-xs text-muted-foreground">
                        {item.month}
                      </p>
                    </div>
                  </div>
                ))
              )}
            </div>

            {scoreImprovement !== 0 && (
              <div className="mt-4 flex items-center justify-center gap-2 text-sm text-muted-foreground">
                {scoreImprovement > 0 ? (
                  <TrendUp className="h-4 w-4 text-success" weight="bold" />
                ) : (
                  <TrendDown className="h-4 w-4 text-destructive" weight="bold" />
                )}
                <span>
                  {scoreImprovement > 0 ? '+' : ''}
                  {scoreImprovement} points over {trustData.history.length}{' '}
                  months
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Quick Stats */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 font-display text-lg font-semibold">Transaction Status</h2>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-1.5 w-1.5 rounded-full bg-success" />
                  <span className="text-sm">Verified</span>
                </div>
                <span className="financial-figure font-medium">
                  {trustData.breakdown.verified}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-1.5 w-1.5 rounded-full bg-warning" />
                  <span className="text-sm">Pending</span>
                </div>
                <span className="financial-figure font-medium">
                  {trustData.breakdown.pending}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="h-1.5 w-1.5 rounded-full bg-destructive" />
                  <span className="text-sm">Flagged</span>
                </div>
                <span className="financial-figure font-medium">
                  {trustData.breakdown.flagged}
                </span>
              </div>
              <div className="mt-4 border-t border-border pt-4">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">
                    Total Transactions
                  </span>
                  <span className="financial-figure font-bold">{trustData.breakdown.total}</span>
                </div>
                <div className="mt-2 flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Verification Rate
                  </span>
                  <span className="financial-figure font-medium text-success">
                    {trustData.breakdown.total > 0
                      ? Math.round(
                          (trustData.breakdown.verified /
                            trustData.breakdown.total) *
                            100,
                        )
                      : 0}
                    %
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* Proof Documentation Stats */}
          <div className="rounded-lg border border-border bg-card p-6">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="font-display text-lg font-semibold">Proof Documentation</h2>
              <div className="flex items-center gap-1 text-success">
                <Shield className="h-4 w-4" weight="fill" />
                <span className="financial-figure text-sm font-bold">+{trustData.proofStats.totalTrustPoints}</span>
              </div>
            </div>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <CheckCircle className="h-4 w-4 text-success" weight="fill" />
                  <span className="text-sm">Verified Proofs</span>
                </div>
                <span className="financial-figure font-medium">{trustData.proofStats.verifiedProofs}</span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-warning" weight="fill" />
                  <span className="text-sm">Pending</span>
                </div>
                <span className="financial-figure font-medium">{trustData.proofStats.pendingProofs}</span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <XCircle className="h-4 w-4 text-destructive" weight="fill" />
                  <span className="text-sm">Rejected</span>
                </div>
                <span className="financial-figure font-medium">{trustData.proofStats.rejectedProofs}</span>
              </div>
              <div className="mt-4 border-t border-border pt-4">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">Total Proofs</span>
                  <span className="financial-figure font-bold">{trustData.proofStats.totalProofs}</span>
                </div>
                <div className="mt-2 flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Proof Coverage</span>
                  <span className={`financial-figure font-medium ${trustData.proofStats.proofCoverageRate >= 80 ? 'text-success' : trustData.proofStats.proofCoverageRate >= 50 ? 'text-warning' : 'text-destructive'}`}>
                    {trustData.proofStats.proofCoverageRate}%
                  </span>
                </div>
                <div className="mt-2 flex gap-0.5">
                  {Array.from({ length: 10 }).map((_, i) => (
                    <div
                      key={i}
                      className={`h-2 flex-1 rounded-sm transition-all duration-500 ${
                        i < Math.round(trustData.proofStats.proofCoverageRate / 10)
                          ? trustData.proofStats.proofCoverageRate >= 80
                            ? 'bg-success'
                            : trustData.proofStats.proofCoverageRate >= 50
                              ? 'bg-warning'
                              : 'bg-destructive'
                          : 'bg-muted/20'
                      }`}
                    />
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Recommendations - numbered list */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 font-display text-lg font-semibold">Recommendations</h2>
            {trustData.recommendations.length === 0 ? (
              <div className="text-center text-muted-foreground">
                <Check className="mx-auto h-8 w-8 text-success" weight="bold" />
                <p className="mt-2">
                  Great job! No recommendations at this time.
                </p>
              </div>
            ) : (
              <ol className="space-y-4">
                {trustData.recommendations.map((rec, idx) => (
                  <li
                    key={rec.id}
                    className="border-b border-border pb-4 last:border-0 last:pb-0"
                  >
                    <div className="mb-1 flex items-center gap-2">
                      <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-bold text-primary">
                        {idx + 1}
                      </span>
                      <span className={`text-xs font-medium ${
                        rec.priority === 'high' ? 'text-destructive' : rec.priority === 'medium' ? 'text-warning' : 'text-primary'
                      }`}>
                        {rec.priority}
                      </span>
                      <span className="text-xs text-primary">{rec.impact}</span>
                    </div>
                    <p className="font-medium">{rec.title}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {rec.description}
                    </p>
                  </li>
                ))}
              </ol>
            )}
          </div>

          {/* Share Badge */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 font-display text-lg font-semibold">Trust Badge</h2>
            <div className="flex items-center justify-center rounded-lg bg-primary/5 p-6">
              <div className="text-center">
                <Shield className="mx-auto h-12 w-12 text-primary" weight="duotone" />
                <p className="mt-2 text-sm font-medium">FlowTrace Verified</p>
                <p
                  className={`font-display text-2xl font-bold ${getGradeColor(trustData.grade)}`}
                >
                  Grade {trustData.grade}
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
