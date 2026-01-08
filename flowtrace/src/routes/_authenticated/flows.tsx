import { createFileRoute, Link } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import {
  ArrowsIn,
  ArrowsOut,
  CurrencyCircleDollar,
  FlowArrow,
  GitBranch,
  Lightning,
  MagnifyingGlassMinus,
  MagnifyingGlassPlus,
  Plus,
  TrendDown,
  TrendUp,
} from '@phosphor-icons/react'
import { useFlowData } from '@/lib/queries'
import type { FlowDataNode } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/flows')({
  component: FlowVisualization,
})

interface PositionedNode extends FlowDataNode {
  x: number
  y: number
}

interface FlowEdge {
  id: string
  from: string
  to: string
  amount: number
  count: number
}

const nodeColors: Record<string, { bg: string; border: string; text: string }> =
  {
    donor: {
      bg: 'fill-blue-100',
      border: 'stroke-blue-500',
      text: 'text-blue-700',
    },
    account: {
      bg: 'fill-purple-100',
      border: 'stroke-purple-500',
      text: 'text-purple-700',
    },
    department: {
      bg: 'fill-green-100',
      border: 'stroke-green-500',
      text: 'text-green-700',
    },
    vendor: {
      bg: 'fill-orange-100',
      border: 'stroke-orange-500',
      text: 'text-orange-700',
    },
    beneficiary: {
      bg: 'fill-pink-100',
      border: 'stroke-pink-500',
      text: 'text-pink-700',
    },
    project: {
      bg: 'fill-cyan-100',
      border: 'stroke-cyan-500',
      text: 'text-cyan-700',
    },
  }

// Position nodes in columns by type
function positionNodes(nodes: Array<FlowDataNode>): Array<PositionedNode> {
  const columns: Record<string, number> = {
    donor: 100,
    account: 350,
    department: 600,
    project: 600,
    vendor: 850,
    beneficiary: 850,
  }

  const typeCounts: Record<string, number> = {}

  return nodes.map((node) => {
    const col = columns[node.type] ?? 350
    typeCounts[node.type] = (typeCounts[node.type] || 0) + 1
    const row = typeCounts[node.type]
    const y = 50 + row * 80

    return { ...node, x: col, y }
  })
}

function FlowVisualization() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null)
  const [timeRange, setTimeRange] = useState('month')

  // Use Edge Function for flow data
  const { data: flowData, isLoading } = useFlowData()

  // Position nodes from Edge Function data
  const positionedNodes = useMemo(() => {
    if (!flowData?.nodes) return []
    return positionNodes(flowData.nodes)
  }, [flowData?.nodes])

  // Transform edges from Edge Function format
  const edges = useMemo((): FlowEdge[] => {
    if (!flowData?.edges) return []
    return flowData.edges.map((edge) => ({
      id: `${edge.source}-${edge.target}`,
      from: edge.source,
      to: edge.target,
      amount: edge.value,
      count: edge.count,
    }))
  }, [flowData?.edges])

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `KES ${(amount / 1000000).toFixed(1)}M`
    if (amount >= 1000) return `KES ${(amount / 1000).toFixed(0)}K`
    return `KES ${amount}`
  }

  const selectedNodeData = useMemo(() => {
    if (!selectedNode) return null
    const node = positionedNodes.find((n) => n.id === selectedNode)
    if (!node) return null

    const inflows = edges.filter((e) => e.to === selectedNode)
    const outflows = edges.filter((e) => e.from === selectedNode)
    // Use pre-calculated values from Edge Function
    const totalIn = node.inflow
    const totalOut = node.outflow

    return { node, inflows, outflows, totalIn, totalOut }
  }, [selectedNode, positionedNodes, edges])

  // Use summary from Edge Function
  const totalFlow = flowData?.summary?.total_flow ?? 0

  // Calculate SVG dimensions based on node positions
  const svgHeight = useMemo(() => {
    if (positionedNodes.length === 0) return 500
    const maxY = Math.max(...positionedNodes.map((n) => n.y))
    return Math.max(500, maxY + 100)
  }, [positionedNodes])

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-blue-500 to-blue-600 shadow-lg shadow-blue-500/20">
            <FlowArrow className="h-6 w-6 text-white" weight="bold" />
          </div>
          <div>
            <h1 className="text-3xl font-bold">Money Flow</h1>
            <p className="text-muted-foreground">
              Visualize how funds move through your organization
            </p>
          </div>
        </div>
        <div className="flex gap-2 rounded-lg border border-border bg-card p-1">
          {['week', 'month', 'quarter', 'year'].map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`rounded-md px-3 py-1.5 text-sm font-medium transition-all ${
                timeRange === range
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {range.charAt(0).toUpperCase() + range.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Stats */}
      <div className="mb-6 grid gap-4 sm:grid-cols-4">
        {isLoading ? (
          <>
            {[...Array(4)].map((_, i) => (
              <div
                key={i}
                className="rounded-xl border border-border bg-card p-4"
              >
                <div className="flex items-center justify-between">
                  <div className="h-4 w-20 animate-pulse rounded bg-muted" />
                  <div className="h-8 w-8 animate-pulse rounded-lg bg-muted" />
                </div>
                <div className="mt-2 h-7 w-24 animate-pulse rounded bg-muted" />
              </div>
            ))}
          </>
        ) : (
          <>
            <div className="group relative overflow-hidden rounded-xl border border-border bg-card p-4 transition-all hover:border-primary/30 hover:shadow-md">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
              <div className="relative flex items-center justify-between">
                <p className="text-sm font-medium text-muted-foreground">Total Flow</p>
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-100 dark:bg-emerald-900/30">
                  <CurrencyCircleDollar className="h-4 w-4 text-emerald-600 dark:text-emerald-400" weight="duotone" />
                </div>
              </div>
              <p className="relative mt-1 text-2xl font-bold">{formatCurrency(totalFlow)}</p>
            </div>
            <div className="group relative overflow-hidden rounded-xl border border-border bg-card p-4 transition-all hover:border-primary/30 hover:shadow-md">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
              <div className="relative flex items-center justify-between">
                <p className="text-sm font-medium text-muted-foreground">Active Nodes</p>
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30">
                  <GitBranch className="h-4 w-4 text-blue-600 dark:text-blue-400" weight="duotone" />
                </div>
              </div>
              <p className="relative mt-1 text-2xl font-bold">{positionedNodes.length}</p>
            </div>
            <div className="group relative overflow-hidden rounded-xl border border-border bg-card p-4 transition-all hover:border-primary/30 hover:shadow-md">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
              <div className="relative flex items-center justify-between">
                <p className="text-sm font-medium text-muted-foreground">Flow Links</p>
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-100 dark:bg-violet-900/30">
                  <Lightning className="h-4 w-4 text-violet-600 dark:text-violet-400" weight="duotone" />
                </div>
              </div>
              <p className="relative mt-1 text-2xl font-bold">{edges.length}</p>
            </div>
            <div className="group relative overflow-hidden rounded-xl border border-border bg-card p-4 transition-all hover:border-primary/30 hover:shadow-md">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
              <div className="relative flex items-center justify-between">
                <p className="text-sm font-medium text-muted-foreground">Avg Flow</p>
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-100 dark:bg-amber-900/30">
                  <TrendUp className="h-4 w-4 text-amber-600 dark:text-amber-400" weight="duotone" />
                </div>
              </div>
              <p className="relative mt-1 text-2xl font-bold">
                {edges.length > 0
                  ? formatCurrency(totalFlow / edges.length)
                  : 'N/A'}
              </p>
            </div>
          </>
        )}
      </div>

      {/* Legend */}
      <div className="mb-4 flex flex-wrap gap-4">
        {Object.entries(nodeColors).map(([type, colors]) => (
          <div key={type} className="flex items-center gap-2">
            <div
              className={`h-3 w-3 rounded-full ${colors.bg.replace('fill-', 'bg-')}`}
            />
            <span className="text-sm capitalize text-muted-foreground">
              {type}
            </span>
          </div>
        ))}
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Flow Diagram */}
        <div className="rounded-xl border border-border bg-card p-4 lg:col-span-2">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Flow Diagram</h2>
            <div className="flex gap-1 rounded-lg border border-border bg-muted/30 p-1">
              <button className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                <MagnifyingGlassMinus className="h-4 w-4" />
              </button>
              <button className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                <MagnifyingGlassPlus className="h-4 w-4" />
              </button>
              <button className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                <ArrowsOut className="h-4 w-4" />
              </button>
            </div>
          </div>

          <div className="overflow-auto rounded-xl border border-border bg-gradient-to-br from-muted/30 to-muted/10">
            {isLoading ? (
              <div className="flex h-[500px] items-center justify-center">
                <div className="text-center">
                  <div className="mx-auto h-10 w-10 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                  <p className="mt-4 font-medium text-muted-foreground">
                    Loading flow data...
                  </p>
                </div>
              </div>
            ) : positionedNodes.length === 0 ? (
              <div className="flex h-[500px] items-center justify-center">
                <div className="text-center">
                  <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-muted">
                    <FlowArrow className="h-8 w-8 text-muted-foreground" weight="duotone" />
                  </div>
                  <h3 className="mt-4 font-semibold">No flow nodes found</h3>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Add transactions to start visualizing your flows
                  </p>
                  <Link
                    to="/transactions/new"
                    className="mt-4 inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <Plus className="h-4 w-4" />
                    Add Transaction
                  </Link>
                </div>
              </div>
            ) : (
              <svg
                width="950"
                height={svgHeight}
                className="min-w-[950px]"
                viewBox={`0 0 950 ${svgHeight}`}
              >
                {/* Edges */}
                {edges.map((edge) => {
                  const fromNode = positionedNodes.find(
                    (n) => n.id === edge.from,
                  )
                  const toNode = positionedNodes.find((n) => n.id === edge.to)
                  if (!fromNode || !toNode) return null

                  const isHighlighted =
                    selectedNode === edge.from || selectedNode === edge.to

                  // Calculate edge path
                  const startX = fromNode.x + 60
                  const startY = fromNode.y + 25
                  const endX = toNode.x
                  const endY = toNode.y + 25
                  const midX = (startX + endX) / 2

                  return (
                    <g key={edge.id}>
                      <path
                        d={`M ${startX} ${startY} C ${midX} ${startY}, ${midX} ${endY}, ${endX} ${endY}`}
                        fill="none"
                        className={`${
                          isHighlighted
                            ? 'stroke-primary'
                            : 'stroke-muted-foreground/30'
                        } transition-colors`}
                        strokeWidth={isHighlighted ? 3 : 2}
                        strokeDasharray={isHighlighted ? undefined : '5,5'}
                      />
                      {/* Arrow */}
                      <polygon
                        points={`${endX},${endY} ${endX - 8},${endY - 4} ${endX - 8},${endY + 4}`}
                        className={
                          isHighlighted
                            ? 'fill-primary'
                            : 'fill-muted-foreground/30'
                        }
                      />
                      {/* Amount label */}
                      <text
                        x={midX}
                        y={(startY + endY) / 2 - 8}
                        textAnchor="middle"
                        className={`fill-current text-xs ${
                          isHighlighted
                            ? 'text-primary font-medium'
                            : 'text-muted-foreground'
                        }`}
                      >
                        {formatCurrency(edge.amount)}
                      </text>
                    </g>
                  )
                })}

                {/* Nodes */}
                {positionedNodes.map((node) => {
                  const colors = nodeColors[node.type]
                  const isSelected = selectedNode === node.id

                  return (
                    <g
                      key={node.id}
                      onClick={() =>
                        setSelectedNode(
                          selectedNode === node.id ? null : node.id,
                        )
                      }
                      className="cursor-pointer"
                    >
                      <rect
                        x={node.x}
                        y={node.y}
                        width={120}
                        height={50}
                        rx={8}
                        className={`${colors.bg} ${colors.border} ${
                          isSelected ? 'stroke-[3px]' : 'stroke-2'
                        } transition-all`}
                      />
                      <text
                        x={node.x + 60}
                        y={node.y + 22}
                        textAnchor="middle"
                        className={`fill-current text-sm font-medium ${colors.text}`}
                      >
                        {node.name.length > 12
                          ? node.name.slice(0, 12) + '...'
                          : node.name}
                      </text>
                      <text
                        x={node.x + 60}
                        y={node.y + 38}
                        textAnchor="middle"
                        className="fill-current text-xs capitalize text-muted-foreground"
                      >
                        {node.type}
                      </text>
                    </g>
                  )
                })}
              </svg>
            )}
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            Click on a node to see details. Scroll to pan the diagram.
          </p>
        </div>

        {/* Node Details Panel */}
        <div className="space-y-4">
          {selectedNodeData ? (
            <>
              <div className="rounded-xl border border-border bg-card p-6">
                <div className="flex items-start justify-between">
                  <div>
                    <h2 className="text-lg font-semibold">
                      {selectedNodeData.node.name}
                    </h2>
                    <span className="mt-1 inline-block rounded-full bg-muted px-2 py-0.5 text-xs capitalize text-muted-foreground">
                      {selectedNodeData.node.type}
                    </span>
                  </div>
                  <button
                    onClick={() => setSelectedNode(null)}
                    className="rounded-lg p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                  >
                    <ArrowsIn className="h-4 w-4" />
                  </button>
                </div>
                <div className="mt-4 grid grid-cols-2 gap-4">
                  <div className="rounded-lg bg-emerald-50 p-3 dark:bg-emerald-900/20">
                    <div className="flex items-center gap-2">
                      <TrendUp className="h-4 w-4 text-emerald-600 dark:text-emerald-400" weight="bold" />
                      <p className="text-xs font-medium text-emerald-700 dark:text-emerald-400">Inflows</p>
                    </div>
                    <p className="mt-1 text-xl font-bold text-emerald-700 dark:text-emerald-400">
                      {formatCurrency(selectedNodeData.totalIn)}
                    </p>
                  </div>
                  <div className="rounded-lg bg-red-50 p-3 dark:bg-red-900/20">
                    <div className="flex items-center gap-2">
                      <TrendDown className="h-4 w-4 text-red-600 dark:text-red-400" weight="bold" />
                      <p className="text-xs font-medium text-red-700 dark:text-red-400">Outflows</p>
                    </div>
                    <p className="mt-1 text-xl font-bold text-red-700 dark:text-red-400">
                      {formatCurrency(selectedNodeData.totalOut)}
                    </p>
                  </div>
                </div>
                <div className="mt-4 rounded-lg border border-border bg-muted/30 p-3">
                  <p className="text-xs font-medium text-muted-foreground">Net Balance</p>
                  <p
                    className={`mt-1 text-2xl font-bold ${
                      selectedNodeData.totalIn - selectedNodeData.totalOut >= 0
                        ? 'text-emerald-600 dark:text-emerald-400'
                        : 'text-red-600 dark:text-red-400'
                    }`}
                  >
                    {formatCurrency(
                      selectedNodeData.totalIn - selectedNodeData.totalOut,
                    )}
                  </p>
                </div>
              </div>

              {selectedNodeData.inflows.length > 0 && (
                <div className="rounded-xl border border-border bg-card p-4">
                  <h3 className="mb-3 flex items-center gap-2 font-semibold text-emerald-600 dark:text-emerald-400">
                    <TrendUp className="h-4 w-4" weight="bold" />
                    Inflows ({selectedNodeData.inflows.length})
                  </h3>
                  <ul className="space-y-2">
                    {selectedNodeData.inflows.map((flow) => {
                      const fromNode = positionedNodes.find(
                        (n) => n.id === flow.from,
                      )
                      return (
                        <li
                          key={flow.id}
                          className="flex items-center justify-between rounded-lg bg-muted/30 px-3 py-2 text-sm"
                        >
                          <span className="truncate">{fromNode?.name ?? 'Unknown'}</span>
                          <span className="shrink-0 font-medium text-emerald-600 dark:text-emerald-400">
                            {formatCurrency(flow.amount)}
                          </span>
                        </li>
                      )
                    })}
                  </ul>
                </div>
              )}

              {selectedNodeData.outflows.length > 0 && (
                <div className="rounded-xl border border-border bg-card p-4">
                  <h3 className="mb-3 flex items-center gap-2 font-semibold text-red-600 dark:text-red-400">
                    <TrendDown className="h-4 w-4" weight="bold" />
                    Outflows ({selectedNodeData.outflows.length})
                  </h3>
                  <ul className="space-y-2">
                    {selectedNodeData.outflows.map((flow) => {
                      const toNode = positionedNodes.find(
                        (n) => n.id === flow.to,
                      )
                      return (
                        <li
                          key={flow.id}
                          className="flex items-center justify-between rounded-lg bg-muted/30 px-3 py-2 text-sm"
                        >
                          <span className="truncate">{toNode?.name ?? 'Unknown'}</span>
                          <span className="shrink-0 font-medium text-red-600 dark:text-red-400">
                            {formatCurrency(flow.amount)}
                          </span>
                        </li>
                      )
                    })}
                  </ul>
                </div>
              )}
            </>
          ) : (
            <div className="rounded-xl border border-dashed border-border bg-card/50 p-8 text-center">
              <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-muted">
                <GitBranch className="h-7 w-7 text-muted-foreground" weight="duotone" />
              </div>
              <h3 className="mt-4 font-semibold">Select a Node</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Click on any node in the diagram to see its flow details
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

