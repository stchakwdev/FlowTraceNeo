import { createFileRoute } from '@tanstack/react-router'
import { useState, useMemo } from 'react'

export const Route = createFileRoute('/_authenticated/flows')({
  component: FlowVisualization,
})

// Mock flow data
const mockNodes = [
  { id: 'donor1', name: 'USAID Grant', type: 'donor', x: 100, y: 150 },
  { id: 'donor2', name: 'Gates Foundation', type: 'donor', x: 100, y: 350 },
  { id: 'main', name: 'General Fund', type: 'account', x: 350, y: 250 },
  { id: 'ops', name: 'Operations', type: 'department', x: 600, y: 100 },
  { id: 'programs', name: 'Programs', type: 'department', x: 600, y: 250 },
  { id: 'admin', name: 'Admin', type: 'department', x: 600, y: 400 },
  { id: 'vendor1', name: 'Staples Kenya', type: 'vendor', x: 850, y: 100 },
  { id: 'vendor2', name: 'Training Co', type: 'vendor', x: 850, y: 250 },
  { id: 'beneficiary', name: 'Communities', type: 'beneficiary', x: 850, y: 400 },
]

const mockEdges = [
  { id: 'e1', from: 'donor1', to: 'main', amount: 5000000, label: 'Q4 Grant' },
  { id: 'e2', from: 'donor2', to: 'main', amount: 2500000, label: 'Annual' },
  { id: 'e3', from: 'main', to: 'ops', amount: 1500000, label: 'Budget' },
  { id: 'e4', from: 'main', to: 'programs', amount: 4000000, label: 'Programs' },
  { id: 'e5', from: 'main', to: 'admin', amount: 2000000, label: 'Admin' },
  { id: 'e6', from: 'ops', to: 'vendor1', amount: 150000, label: 'Supplies' },
  { id: 'e7', from: 'programs', to: 'vendor2', amount: 500000, label: 'Training' },
  { id: 'e8', from: 'programs', to: 'beneficiary', amount: 3000000, label: 'Direct Aid' },
]

const nodeColors: Record<string, { bg: string; border: string; text: string }> = {
  donor: { bg: 'fill-blue-100', border: 'stroke-blue-500', text: 'text-blue-700' },
  account: { bg: 'fill-purple-100', border: 'stroke-purple-500', text: 'text-purple-700' },
  department: { bg: 'fill-green-100', border: 'stroke-green-500', text: 'text-green-700' },
  vendor: { bg: 'fill-orange-100', border: 'stroke-orange-500', text: 'text-orange-700' },
  beneficiary: { bg: 'fill-pink-100', border: 'stroke-pink-500', text: 'text-pink-700' },
}

function FlowVisualization() {
  const [selectedNode, setSelectedNode] = useState<string | null>(null)
  const [timeRange, setTimeRange] = useState('month')

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `KES ${(amount / 1000000).toFixed(1)}M`
    if (amount >= 1000) return `KES ${(amount / 1000).toFixed(0)}K`
    return `KES ${amount}`
  }

  const selectedNodeData = useMemo(() => {
    if (!selectedNode) return null
    const node = mockNodes.find((n) => n.id === selectedNode)
    if (!node) return null

    const inflows = mockEdges.filter((e) => e.to === selectedNode)
    const outflows = mockEdges.filter((e) => e.from === selectedNode)
    const totalIn = inflows.reduce((sum, e) => sum + e.amount, 0)
    const totalOut = outflows.reduce((sum, e) => sum + e.amount, 0)

    return { node, inflows, outflows, totalIn, totalOut }
  }, [selectedNode])

  const totalFlow = mockEdges.reduce((sum, e) => sum + e.amount, 0)

  return (
    <div className="min-h-screen bg-background">
      <div className="mx-auto max-w-7xl px-4 py-8">
        {/* Header */}
        <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-3xl font-bold">Money Flow</h1>
            <p className="text-muted-foreground">
              Visualize how funds move through your organization
            </p>
          </div>
          <div className="flex gap-2">
            {['week', 'month', 'quarter', 'year'].map((range) => (
              <button
                key={range}
                onClick={() => setTimeRange(range)}
                className={`rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                  timeRange === range
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-card text-muted-foreground hover:bg-accent'
                }`}
              >
                {range.charAt(0).toUpperCase() + range.slice(1)}
              </button>
            ))}
          </div>
        </div>

        {/* Stats */}
        <div className="mb-6 grid gap-4 sm:grid-cols-4">
          <div className="rounded-lg border border-border bg-card p-4">
            <p className="text-sm text-muted-foreground">Total Flow</p>
            <p className="text-2xl font-bold">{formatCurrency(totalFlow)}</p>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <p className="text-sm text-muted-foreground">Active Nodes</p>
            <p className="text-2xl font-bold">{mockNodes.length}</p>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <p className="text-sm text-muted-foreground">Transactions</p>
            <p className="text-2xl font-bold">{mockEdges.length}</p>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <p className="text-sm text-muted-foreground">Avg Transaction</p>
            <p className="text-2xl font-bold">{formatCurrency(totalFlow / mockEdges.length)}</p>
          </div>
        </div>

        {/* Legend */}
        <div className="mb-4 flex flex-wrap gap-4">
          {Object.entries(nodeColors).map(([type, colors]) => (
            <div key={type} className="flex items-center gap-2">
              <div className={`h-3 w-3 rounded-full ${colors.bg.replace('fill-', 'bg-')}`} />
              <span className="text-sm capitalize text-muted-foreground">{type}</span>
            </div>
          ))}
        </div>

        <div className="grid gap-6 lg:grid-cols-3">
          {/* Flow Diagram */}
          <div className="rounded-lg border border-border bg-card p-4 lg:col-span-2">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-lg font-semibold">Flow Diagram</h2>
              <div className="flex gap-2">
                <button className="rounded-lg border border-border p-2 hover:bg-accent">
                  <ZoomOutIcon className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-border p-2 hover:bg-accent">
                  <ZoomInIcon className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-border p-2 hover:bg-accent">
                  <MaximizeIcon className="h-4 w-4" />
                </button>
              </div>
            </div>

            <div className="overflow-auto rounded-lg border border-border bg-accent/30">
              <svg
                width="950"
                height="500"
                className="min-w-[950px]"
                viewBox="0 0 950 500"
              >
                {/* Edges */}
                {mockEdges.map((edge) => {
                  const fromNode = mockNodes.find((n) => n.id === edge.from)
                  const toNode = mockNodes.find((n) => n.id === edge.to)
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
                          isHighlighted ? 'stroke-primary' : 'stroke-muted-foreground/30'
                        } transition-colors`}
                        strokeWidth={isHighlighted ? 3 : 2}
                        strokeDasharray={isHighlighted ? undefined : '5,5'}
                      />
                      {/* Arrow */}
                      <polygon
                        points={`${endX},${endY} ${endX - 8},${endY - 4} ${endX - 8},${endY + 4}`}
                        className={isHighlighted ? 'fill-primary' : 'fill-muted-foreground/30'}
                      />
                      {/* Amount label */}
                      <text
                        x={midX}
                        y={(startY + endY) / 2 - 8}
                        textAnchor="middle"
                        className={`fill-current text-xs ${
                          isHighlighted ? 'text-primary font-medium' : 'text-muted-foreground'
                        }`}
                      >
                        {formatCurrency(edge.amount)}
                      </text>
                    </g>
                  )
                })}

                {/* Nodes */}
                {mockNodes.map((node) => {
                  const colors = nodeColors[node.type] || nodeColors.account
                  const isSelected = selectedNode === node.id

                  return (
                    <g
                      key={node.id}
                      onClick={() =>
                        setSelectedNode(selectedNode === node.id ? null : node.id)
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
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              Click on a node to see details. Scroll to pan the diagram.
            </p>
          </div>

          {/* Node Details Panel */}
          <div className="space-y-4">
            {selectedNodeData ? (
              <>
                <div className="rounded-lg border border-border bg-card p-6">
                  <h2 className="mb-4 text-lg font-semibold">
                    {selectedNodeData.node.name}
                  </h2>
                  <p className="mb-4 text-sm capitalize text-muted-foreground">
                    {selectedNodeData.node.type}
                  </p>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <p className="text-sm text-muted-foreground">Inflows</p>
                      <p className="text-xl font-bold text-green-600">
                        {formatCurrency(selectedNodeData.totalIn)}
                      </p>
                    </div>
                    <div>
                      <p className="text-sm text-muted-foreground">Outflows</p>
                      <p className="text-xl font-bold text-red-600">
                        {formatCurrency(selectedNodeData.totalOut)}
                      </p>
                    </div>
                  </div>
                  <div className="mt-4">
                    <p className="text-sm text-muted-foreground">Net</p>
                    <p
                      className={`text-xl font-bold ${
                        selectedNodeData.totalIn - selectedNodeData.totalOut >= 0
                          ? 'text-green-600'
                          : 'text-red-600'
                      }`}
                    >
                      {formatCurrency(
                        selectedNodeData.totalIn - selectedNodeData.totalOut
                      )}
                    </p>
                  </div>
                </div>

                {selectedNodeData.inflows.length > 0 && (
                  <div className="rounded-lg border border-border bg-card p-6">
                    <h3 className="mb-3 font-semibold text-green-600">
                      Inflows ({selectedNodeData.inflows.length})
                    </h3>
                    <ul className="space-y-2">
                      {selectedNodeData.inflows.map((flow) => {
                        const fromNode = mockNodes.find((n) => n.id === flow.from)
                        return (
                          <li
                            key={flow.id}
                            className="flex items-center justify-between text-sm"
                          >
                            <span>{fromNode?.name}</span>
                            <span className="font-medium">
                              {formatCurrency(flow.amount)}
                            </span>
                          </li>
                        )
                      })}
                    </ul>
                  </div>
                )}

                {selectedNodeData.outflows.length > 0 && (
                  <div className="rounded-lg border border-border bg-card p-6">
                    <h3 className="mb-3 font-semibold text-red-600">
                      Outflows ({selectedNodeData.outflows.length})
                    </h3>
                    <ul className="space-y-2">
                      {selectedNodeData.outflows.map((flow) => {
                        const toNode = mockNodes.find((n) => n.id === flow.to)
                        return (
                          <li
                            key={flow.id}
                            className="flex items-center justify-between text-sm"
                          >
                            <span>{toNode?.name}</span>
                            <span className="font-medium">
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
              <div className="rounded-lg border border-border bg-card p-6 text-center">
                <GraphIcon className="mx-auto mb-4 h-12 w-12 text-muted-foreground" />
                <h3 className="font-semibold">Select a Node</h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  Click on any node in the diagram to see its flow details
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

// Icon components
function ZoomInIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v3m0 0v3m0-3h3m-3 0H7" />
    </svg>
  )
}

function ZoomOutIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
    </svg>
  )
}

function MaximizeIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
    </svg>
  )
}

function GraphIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  )
}
