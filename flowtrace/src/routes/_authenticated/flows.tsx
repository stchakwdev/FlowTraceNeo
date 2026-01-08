import { createFileRoute, Link } from '@tanstack/react-router'
import { useMemo, useState, useCallback } from 'react'
import {
  ArrowsIn,
  ArrowsOut,
  Bank,
  Buildings,
  CurrencyCircleDollar,
  FlowArrow,
  GitBranch,
  Handshake,
  Heart,
  Lightning,
  MagnifyingGlassMinus,
  MagnifyingGlassPlus,
  Plus,
  Storefront,
  TrendDown,
  TrendUp,
  Wallet,
} from '@phosphor-icons/react'
import { useFlowData } from '@/lib/queries'
import type { FlowDataNode } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/flows')({
  component: FlowVisualization,
})

interface PositionedNode extends FlowDataNode {
  x: number
  y: number
  column: number
}

interface FlowEdge {
  id: string
  from: string
  to: string
  amount: number
  count: number
}

// Node type configurations with icons and colors
const nodeConfig: Record<
  string,
  { icon: typeof Bank; gradient: string; glow: string; label: string }
> = {
  donor: {
    icon: Handshake,
    gradient: 'from-blue-500/20 to-blue-600/10',
    glow: 'shadow-blue-500/20',
    label: 'Donor',
  },
  account: {
    icon: Wallet,
    gradient: 'from-emerald-500/20 to-emerald-600/10',
    glow: 'shadow-emerald-500/20',
    label: 'Account',
  },
  department: {
    icon: Buildings,
    gradient: 'from-violet-500/20 to-violet-600/10',
    glow: 'shadow-violet-500/20',
    label: 'Department',
  },
  vendor: {
    icon: Storefront,
    gradient: 'from-amber-500/20 to-amber-600/10',
    glow: 'shadow-amber-500/20',
    label: 'Vendor',
  },
  beneficiary: {
    icon: Heart,
    gradient: 'from-rose-500/20 to-rose-600/10',
    glow: 'shadow-rose-500/20',
    label: 'Beneficiary',
  },
  project: {
    icon: Bank,
    gradient: 'from-cyan-500/20 to-cyan-600/10',
    glow: 'shadow-cyan-500/20',
    label: 'Project',
  },
}

// Column positions for hierarchical layout
const columnPositions = {
  donor: 0,
  account: 1,
  department: 2,
  project: 2,
  vendor: 3,
  beneficiary: 3,
}

// Position nodes in a clean hierarchical layout
function positionNodes(nodes: FlowDataNode[]): PositionedNode[] {
  const columnWidth = 280
  const nodeHeight = 90
  const startX = 80
  const startY = 60

  // Group nodes by column
  const columns: Record<number, FlowDataNode[]> = {}
  nodes.forEach((node) => {
    const col = columnPositions[node.type as keyof typeof columnPositions] ?? 1
    if (!columns[col]) columns[col] = []
    columns[col].push(node)
  })

  const positioned: PositionedNode[] = []

  Object.entries(columns).forEach(([colStr, colNodes]) => {
    const col = parseInt(colStr)
    const x = startX + col * columnWidth

    // Center nodes vertically in their column
    const totalHeight = colNodes.length * nodeHeight
    const offsetY = Math.max(0, (400 - totalHeight) / 2)

    colNodes.forEach((node, idx) => {
      positioned.push({
        ...node,
        x,
        y: startY + offsetY + idx * nodeHeight,
        column: col,
      })
    })
  })

  return positioned
}

// Animated edge component
function AnimatedEdge({
  fromX,
  fromY,
  toX,
  toY,
  isHighlighted,
  amount,
  formatCurrency,
}: {
  fromX: number
  fromY: number
  toX: number
  toY: number
  isHighlighted: boolean
  amount: number
  formatCurrency: (n: number) => string
}) {
  // Create smooth bezier curve
  const midX = (fromX + toX) / 2

  const pathD = `M ${fromX} ${fromY} C ${midX} ${fromY}, ${midX} ${toY}, ${toX} ${toY}`

  return (
    <g className="edge-group">
      {/* Glow effect layer */}
      <path
        d={pathD}
        fill="none"
        className={`transition-all duration-300 ${
          isHighlighted ? 'stroke-cyan-400/40' : 'stroke-transparent'
        }`}
        strokeWidth={isHighlighted ? 8 : 0}
        strokeLinecap="round"
        filter="url(#glow)"
      />

      {/* Main edge line */}
      <path
        d={pathD}
        fill="none"
        className={`transition-all duration-300 ${
          isHighlighted
            ? 'stroke-cyan-400'
            : 'stroke-slate-600/60 dark:stroke-slate-500/40'
        }`}
        strokeWidth={isHighlighted ? 2.5 : 1.5}
        strokeLinecap="round"
      />

      {/* Animated flow particles */}
      <circle r={isHighlighted ? 4 : 3} className={isHighlighted ? 'fill-cyan-400' : 'fill-slate-500/60'}>
        <animateMotion dur="2s" repeatCount="indefinite" path={pathD} />
      </circle>
      <circle r={isHighlighted ? 3 : 2} className={isHighlighted ? 'fill-cyan-300' : 'fill-slate-400/40'}>
        <animateMotion dur="2s" repeatCount="indefinite" path={pathD} begin="0.7s" />
      </circle>
      <circle r={isHighlighted ? 2 : 1.5} className={isHighlighted ? 'fill-cyan-200' : 'fill-slate-400/30'}>
        <animateMotion dur="2s" repeatCount="indefinite" path={pathD} begin="1.4s" />
      </circle>

      {/* Amount label - only show on highlight */}
      {isHighlighted && (
        <g>
          <rect
            x={midX - 40}
            y={(fromY + toY) / 2 - 12}
            width={80}
            height={24}
            rx={12}
            className="fill-slate-900/90 dark:fill-slate-800/90"
          />
          <text
            x={midX}
            y={(fromY + toY) / 2 + 4}
            textAnchor="middle"
            className="fill-cyan-400 text-xs font-medium"
          >
            {formatCurrency(amount)}
          </text>
        </g>
      )}
    </g>
  )
}

// Glassmorphism node component
function GlassNode({
  node,
  isSelected,
  onClick,
}: {
  node: PositionedNode
  isSelected: boolean
  onClick: () => void
}) {
  const config = nodeConfig[node.type] || nodeConfig.account
  const Icon = config.icon

  return (
    <g
      onClick={onClick}
      className="cursor-pointer"
      style={{ transform: `translate(${node.x}px, ${node.y}px)` }}
    >
      {/* Outer glow on selection */}
      {isSelected && (
        <rect
          x={-4}
          y={-4}
          width={208}
          height={72}
          rx={18}
          className="fill-cyan-500/20 animate-pulse"
          filter="url(#glow)"
        />
      )}

      {/* Main glass container */}
      <foreignObject x={0} y={0} width={200} height={64}>
        <div
          className={`
            group h-full w-full rounded-2xl border backdrop-blur-xl
            transition-all duration-300 ease-out
            ${
              isSelected
                ? 'border-cyan-500/50 bg-slate-800/80 shadow-lg shadow-cyan-500/20 scale-105'
                : 'border-slate-700/50 bg-slate-900/60 hover:border-slate-600/60 hover:bg-slate-800/70 hover:scale-[1.02]'
            }
          `}
        >
          <div className="flex h-full items-center gap-3 px-4">
            {/* Icon container */}
            <div
              className={`
                flex h-10 w-10 shrink-0 items-center justify-center rounded-xl
                bg-gradient-to-br ${config.gradient}
                border border-white/10 shadow-lg ${config.glow}
                transition-transform duration-300 group-hover:scale-110
              `}
            >
              <Icon
                className={`h-5 w-5 ${
                  isSelected ? 'text-cyan-400' : 'text-slate-300'
                } transition-colors`}
                weight="duotone"
              />
            </div>

            {/* Text content */}
            <div className="min-w-0 flex-1">
              <p
                className={`truncate text-sm font-semibold ${
                  isSelected ? 'text-white' : 'text-slate-200'
                } transition-colors`}
              >
                {node.name}
              </p>
              <p className="text-xs text-slate-500">{config.label}</p>
            </div>
          </div>
        </div>
      </foreignObject>
    </g>
  )
}

function FlowVisualization() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null)
  const [timeRange, setTimeRange] = useState('month')
  const [zoom, setZoom] = useState(1)

  const { data: flowData, isLoading } = useFlowData()

  const positionedNodes = useMemo(() => {
    if (!flowData?.nodes) return []
    return positionNodes(flowData.nodes)
  }, [flowData?.nodes])

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

  const formatCurrency = useCallback((amount: number) => {
    if (amount >= 1000000) return `KES ${(amount / 1000000).toFixed(1)}M`
    if (amount >= 1000) return `KES ${(amount / 1000).toFixed(0)}K`
    return `KES ${amount}`
  }, [])

  const selectedNodeData = useMemo(() => {
    if (!selectedNode) return null
    const node = positionedNodes.find((n) => n.id === selectedNode)
    if (!node) return null

    const inflows = edges.filter((e) => e.to === selectedNode)
    const outflows = edges.filter((e) => e.from === selectedNode)
    const totalIn = node.inflow
    const totalOut = node.outflow

    return { node, inflows, outflows, totalIn, totalOut }
  }, [selectedNode, positionedNodes, edges])

  const totalFlow = flowData?.summary?.total_flow ?? 0

  const svgWidth = 1100
  const svgHeight = useMemo(() => {
    if (positionedNodes.length === 0) return 500
    const maxY = Math.max(...positionedNodes.map((n) => n.y))
    return Math.max(500, maxY + 120)
  }, [positionedNodes])

  const handleZoomIn = () => setZoom((z) => Math.min(z + 0.1, 1.5))
  const handleZoomOut = () => setZoom((z) => Math.max(z - 0.1, 0.5))
  const handleResetZoom = () => setZoom(1)

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 shadow-lg shadow-cyan-500/30">
            <FlowArrow className="h-6 w-6 text-white" weight="bold" />
          </div>
          <div>
            <h1 className="text-3xl font-bold">Money Flow</h1>
            <p className="text-muted-foreground">
              Visualize how funds move through your organization
            </p>
          </div>
        </div>
        <div className="flex gap-2 rounded-xl border border-border bg-card/50 backdrop-blur-sm p-1">
          {['week', 'month', 'quarter', 'year'].map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`rounded-lg px-4 py-2 text-sm font-medium transition-all ${
                timeRange === range
                  ? 'bg-gradient-to-r from-cyan-500 to-blue-500 text-white shadow-lg shadow-cyan-500/25'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
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
              <div key={i} className="rounded-2xl border border-border bg-card/50 backdrop-blur-sm p-5">
                <div className="flex items-center justify-between">
                  <div className="h-4 w-20 animate-pulse rounded bg-muted" />
                  <div className="h-10 w-10 animate-pulse rounded-xl bg-muted" />
                </div>
                <div className="mt-3 h-8 w-28 animate-pulse rounded bg-muted" />
              </div>
            ))}
          </>
        ) : (
          <>
            <StatCard
              label="Total Flow"
              value={formatCurrency(totalFlow)}
              icon={CurrencyCircleDollar}
              gradient="from-emerald-500 to-green-600"
              glow="shadow-emerald-500/25"
            />
            <StatCard
              label="Active Nodes"
              value={positionedNodes.length.toString()}
              icon={GitBranch}
              gradient="from-blue-500 to-indigo-600"
              glow="shadow-blue-500/25"
            />
            <StatCard
              label="Flow Links"
              value={edges.length.toString()}
              icon={Lightning}
              gradient="from-violet-500 to-purple-600"
              glow="shadow-violet-500/25"
            />
            <StatCard
              label="Avg Flow"
              value={edges.length > 0 ? formatCurrency(totalFlow / edges.length) : 'N/A'}
              icon={TrendUp}
              gradient="from-amber-500 to-orange-600"
              glow="shadow-amber-500/25"
            />
          </>
        )}
      </div>

      {/* Legend */}
      <div className="mb-4 flex flex-wrap gap-3">
        {Object.entries(nodeConfig).map(([type, config]) => {
          const Icon = config.icon
          return (
            <div
              key={type}
              className="flex items-center gap-2 rounded-full border border-slate-700/50 bg-slate-900/40 px-3 py-1.5 backdrop-blur-sm"
            >
              <div className={`flex h-5 w-5 items-center justify-center rounded-md bg-gradient-to-br ${config.gradient}`}>
                <Icon className="h-3 w-3 text-slate-300" weight="duotone" />
              </div>
              <span className="text-xs font-medium text-slate-400">{config.label}</span>
            </div>
          )
        })}
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        {/* Flow Diagram */}
        <div className="rounded-2xl border border-slate-800/60 bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 p-4 lg:col-span-2 overflow-hidden">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-slate-200">Flow Diagram</h2>
            <div className="flex gap-1 rounded-xl border border-slate-700/50 bg-slate-800/50 p-1">
              <button
                onClick={handleZoomOut}
                className="rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-700/50 hover:text-slate-200"
              >
                <MagnifyingGlassMinus className="h-4 w-4" />
              </button>
              <button
                onClick={handleZoomIn}
                className="rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-700/50 hover:text-slate-200"
              >
                <MagnifyingGlassPlus className="h-4 w-4" />
              </button>
              <button
                onClick={handleResetZoom}
                className="rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-700/50 hover:text-slate-200"
              >
                <ArrowsOut className="h-4 w-4" />
              </button>
            </div>
          </div>

          <div className="overflow-auto rounded-xl bg-slate-950/50 ring-1 ring-slate-800/50">
            {isLoading ? (
              <div className="flex h-[500px] items-center justify-center">
                <div className="text-center">
                  <div className="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-cyan-500/30 border-t-cyan-500" />
                  <p className="mt-4 font-medium text-slate-400">Loading flow data...</p>
                </div>
              </div>
            ) : positionedNodes.length === 0 ? (
              <div className="flex h-[500px] items-center justify-center">
                <div className="text-center">
                  <div className="mx-auto flex h-20 w-20 items-center justify-center rounded-2xl bg-slate-800/50 ring-1 ring-slate-700/50">
                    <FlowArrow className="h-10 w-10 text-slate-500" weight="duotone" />
                  </div>
                  <h3 className="mt-4 text-lg font-semibold text-slate-300">No flow nodes found</h3>
                  <p className="mt-1 text-sm text-slate-500">
                    Add transactions to start visualizing your flows
                  </p>
                  <Link
                    to="/transactions/new"
                    className="mt-4 inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-500 px-5 py-2.5 text-sm font-medium text-white shadow-lg shadow-cyan-500/25 transition-all hover:shadow-cyan-500/40"
                  >
                    <Plus className="h-4 w-4" />
                    Add Transaction
                  </Link>
                </div>
              </div>
            ) : (
              <svg
                width={svgWidth * zoom}
                height={svgHeight * zoom}
                viewBox={`0 0 ${svgWidth} ${svgHeight}`}
                className="min-w-[900px]"
                style={{ minHeight: svgHeight * zoom }}
              >
                {/* Definitions for filters and gradients */}
                <defs>
                  <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
                    <feGaussianBlur stdDeviation="4" result="coloredBlur" />
                    <feMerge>
                      <feMergeNode in="coloredBlur" />
                      <feMergeNode in="SourceGraphic" />
                    </feMerge>
                  </filter>
                  <linearGradient id="edgeGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stopColor="rgb(34, 211, 238)" stopOpacity="0.6" />
                    <stop offset="100%" stopColor="rgb(59, 130, 246)" stopOpacity="0.6" />
                  </linearGradient>
                </defs>

                {/* Background grid pattern */}
                <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                  <path
                    d="M 40 0 L 0 0 0 40"
                    fill="none"
                    stroke="rgba(148, 163, 184, 0.05)"
                    strokeWidth="1"
                  />
                </pattern>
                <rect width="100%" height="100%" fill="url(#grid)" />

                {/* Edges layer */}
                <g className="edges">
                  {edges.map((edge) => {
                    const fromNode = positionedNodes.find((n) => n.id === edge.from)
                    const toNode = positionedNodes.find((n) => n.id === edge.to)
                    if (!fromNode || !toNode) return null

                    const isHighlighted = selectedNode === edge.from || selectedNode === edge.to

                    return (
                      <AnimatedEdge
                        key={edge.id}
                        fromX={fromNode.x + 200}
                        fromY={fromNode.y + 32}
                        toX={toNode.x}
                        toY={toNode.y + 32}
                        isHighlighted={isHighlighted}
                        amount={edge.amount}
                        formatCurrency={formatCurrency}
                      />
                    )
                  })}
                </g>

                {/* Nodes layer */}
                <g className="nodes">
                  {positionedNodes.map((node) => (
                    <GlassNode
                      key={node.id}
                      node={node}
                      isSelected={selectedNode === node.id}
                      onClick={() => setSelectedNode(selectedNode === node.id ? null : node.id)}
                    />
                  ))}
                </g>
              </svg>
            )}
          </div>
          <p className="mt-3 text-xs text-slate-500">
            Click on a node to see details. Scroll to pan the diagram.
          </p>
        </div>

        {/* Node Details Panel */}
        <div className="space-y-4">
          {selectedNodeData ? (
            <>
              <div className="rounded-2xl border border-slate-800/60 bg-gradient-to-br from-slate-900 to-slate-950 p-6">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div
                      className={`flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br ${
                        nodeConfig[selectedNodeData.node.type]?.gradient || nodeConfig.account.gradient
                      } border border-white/10`}
                    >
                      {(() => {
                        const Icon = nodeConfig[selectedNodeData.node.type]?.icon || Bank
                        return <Icon className="h-6 w-6 text-slate-200" weight="duotone" />
                      })()}
                    </div>
                    <div>
                      <h2 className="text-lg font-semibold text-slate-100">
                        {selectedNodeData.node.name}
                      </h2>
                      <span className="text-sm text-slate-500">
                        {nodeConfig[selectedNodeData.node.type]?.label || 'Node'}
                      </span>
                    </div>
                  </div>
                  <button
                    onClick={() => setSelectedNode(null)}
                    className="rounded-lg p-2 text-slate-500 transition-colors hover:bg-slate-800 hover:text-slate-300"
                  >
                    <ArrowsIn className="h-4 w-4" />
                  </button>
                </div>

                <div className="mt-5 grid grid-cols-2 gap-3">
                  <div className="rounded-xl bg-emerald-500/10 p-4 ring-1 ring-emerald-500/20">
                    <div className="flex items-center gap-2">
                      <TrendUp className="h-4 w-4 text-emerald-400" weight="bold" />
                      <p className="text-xs font-medium text-emerald-400">Inflows</p>
                    </div>
                    <p className="mt-2 text-xl font-bold text-emerald-400">
                      {formatCurrency(selectedNodeData.totalIn)}
                    </p>
                  </div>
                  <div className="rounded-xl bg-red-500/10 p-4 ring-1 ring-red-500/20">
                    <div className="flex items-center gap-2">
                      <TrendDown className="h-4 w-4 text-red-400" weight="bold" />
                      <p className="text-xs font-medium text-red-400">Outflows</p>
                    </div>
                    <p className="mt-2 text-xl font-bold text-red-400">
                      {formatCurrency(selectedNodeData.totalOut)}
                    </p>
                  </div>
                </div>

                <div className="mt-4 rounded-xl bg-slate-800/50 p-4 ring-1 ring-slate-700/50">
                  <p className="text-xs font-medium text-slate-500">Net Balance</p>
                  <p
                    className={`mt-1 text-2xl font-bold ${
                      selectedNodeData.totalIn - selectedNodeData.totalOut >= 0
                        ? 'text-emerald-400'
                        : 'text-red-400'
                    }`}
                  >
                    {formatCurrency(selectedNodeData.totalIn - selectedNodeData.totalOut)}
                  </p>
                </div>
              </div>

              {selectedNodeData.inflows.length > 0 && (
                <div className="rounded-2xl border border-slate-800/60 bg-slate-900/50 p-4">
                  <h3 className="mb-3 flex items-center gap-2 font-semibold text-emerald-400">
                    <TrendUp className="h-4 w-4" weight="bold" />
                    Inflows ({selectedNodeData.inflows.length})
                  </h3>
                  <ul className="space-y-2">
                    {selectedNodeData.inflows.map((flow) => {
                      const fromNode = positionedNodes.find((n) => n.id === flow.from)
                      return (
                        <li
                          key={flow.id}
                          className="flex items-center justify-between rounded-xl bg-slate-800/50 px-4 py-3 text-sm ring-1 ring-slate-700/30"
                        >
                          <span className="truncate text-slate-300">{fromNode?.name ?? 'Unknown'}</span>
                          <span className="shrink-0 font-semibold text-emerald-400">
                            {formatCurrency(flow.amount)}
                          </span>
                        </li>
                      )
                    })}
                  </ul>
                </div>
              )}

              {selectedNodeData.outflows.length > 0 && (
                <div className="rounded-2xl border border-slate-800/60 bg-slate-900/50 p-4">
                  <h3 className="mb-3 flex items-center gap-2 font-semibold text-red-400">
                    <TrendDown className="h-4 w-4" weight="bold" />
                    Outflows ({selectedNodeData.outflows.length})
                  </h3>
                  <ul className="space-y-2">
                    {selectedNodeData.outflows.map((flow) => {
                      const toNode = positionedNodes.find((n) => n.id === flow.to)
                      return (
                        <li
                          key={flow.id}
                          className="flex items-center justify-between rounded-xl bg-slate-800/50 px-4 py-3 text-sm ring-1 ring-slate-700/30"
                        >
                          <span className="truncate text-slate-300">{toNode?.name ?? 'Unknown'}</span>
                          <span className="shrink-0 font-semibold text-red-400">
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
            <div className="rounded-2xl border border-dashed border-slate-700/50 bg-slate-900/30 p-8 text-center">
              <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-slate-800/50 ring-1 ring-slate-700/50">
                <GitBranch className="h-8 w-8 text-slate-500" weight="duotone" />
              </div>
              <h3 className="mt-4 font-semibold text-slate-300">Select a Node</h3>
              <p className="mt-1 text-sm text-slate-500">
                Click on any node in the diagram to see its flow details
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// Stat card component
function StatCard({
  label,
  value,
  icon: Icon,
  gradient,
  glow,
}: {
  label: string
  value: string
  icon: typeof CurrencyCircleDollar
  gradient: string
  glow: string
}) {
  return (
    <div className="group relative overflow-hidden rounded-2xl border border-slate-800/60 bg-gradient-to-br from-slate-900/80 to-slate-950/80 p-5 backdrop-blur-sm transition-all hover:border-slate-700/60">
      <div className="absolute inset-0 bg-gradient-to-br from-white/[0.02] to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
      <div className="relative flex items-center justify-between">
        <p className="text-sm font-medium text-slate-400">{label}</p>
        <div
          className={`flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br ${gradient} shadow-lg ${glow}`}
        >
          <Icon className="h-5 w-5 text-white" weight="duotone" />
        </div>
      </div>
      <p className="relative mt-2 text-2xl font-bold text-slate-100">{value}</p>
    </div>
  )
}
