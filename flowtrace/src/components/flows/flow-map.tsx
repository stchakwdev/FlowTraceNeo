import {
  NAME_SIZE,
  NODE_PAD,
  NODE_TYPE_LABEL,
  NODE_W,
  metaBaseline,
  nameBaseline,
} from './flow-graph'
import type { KeyboardEvent } from 'react'
import type { FlowGraph, GraphEdge, GraphNode, Selection } from './flow-graph'
import { PROOF_STATUS_LABEL, proofStroke } from '@/components/report'
import { formatCompact, formatMoney } from '@/lib/format'
import { cn } from '@/lib/utils'

const DIM = 0.12

/** Scale a dash pattern with the line so thick lines still read as dashed. */
function scaleDash(dash: string | undefined, width: number) {
  if (!dash) return undefined
  const k = Math.max(1, width / 3)
  return dash
    .split(' ')
    .map((n) => Number(n) * k)
    .join(' ')
}

function activate(e: KeyboardEvent, run: () => void) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    run()
  }
}

export function FlowMap({
  graph,
  zoom,
  selection,
  active,
  onSelect,
  onClear,
}: {
  graph: FlowGraph
  zoom: number
  selection: Selection | null
  active: { nodes: Set<string>; edges: Set<string> } | null
  onSelect: (selection: Selection) => void
  onClear: () => void
}) {
  const nodeById = new Map(graph.nodes.map((n) => [n.id, n]))
  const columns = graph.columns.map((_, ci) => ({
    nodes: graph.nodes.filter((n) => n.column === ci),
    edges: graph.edges.filter((e) => nodeById.get(e.source)?.column === ci),
  }))
  const selectedEdge =
    selection?.kind === 'edge'
      ? graph.edges.find((e) => e.id === selection.id)
      : undefined

  return (
    <svg
      width={graph.width * zoom}
      height={graph.height * zoom}
      viewBox={`0 0 ${graph.width} ${graph.height}`}
      role="group"
      aria-label="Flow map. Each box is a node; each line is money moving between two nodes. Line weight carries the amount and line pattern carries the proof state."
      className="block select-none"
    >
      <rect
        width={graph.width}
        height={graph.height}
        className="fill-transparent"
        onClick={onClear}
      />

      {graph.columns.map((col, ci) =>
        col.label ? (
          <text
            key={`label-${ci}`}
            x={col.x}
            y={22}
            className="fill-muted-foreground text-[13.5px] font-bold tracking-[0.12em] uppercase"
          >
            {col.label}
          </text>
        ) : null,
      )}

      {/* Interleaved by column: a column's nodes, then the lines leaving it.
          Lines only travel rightwards, so each one paints under the nodes it
          crosses, and Tab order follows the money. */}
      {columns.map((col, ci) => (
        <g key={`col-${ci}`}>
          {col.nodes.map((node) => (
            <MapNode
              key={node.id}
              node={node}
              selected={selection?.kind === 'node' && selection.id === node.id}
              dimmed={!!active && !active.nodes.has(node.id)}
              onSelect={() => onSelect({ kind: 'node', id: node.id })}
            />
          ))}
          {col.edges.map((edge) => (
            <MapEdge
              key={edge.id}
              edge={edge}
              from={nodeById.get(edge.source)!}
              to={nodeById.get(edge.target)!}
              selected={selection?.kind === 'edge' && selection.id === edge.id}
              dimmed={!!active && !active.edges.has(edge.id)}
              onSelect={() => onSelect({ kind: 'edge', id: edge.id })}
            />
          ))}
        </g>
      ))}

      {selectedEdge && <AmountTag edge={selectedEdge} />}
    </svg>
  )
}

function MapNode({
  node,
  selected,
  dimmed,
  onSelect,
}: {
  node: GraphNode
  selected: boolean
  dimmed: boolean
  onSelect: () => void
}) {
  // Show whichever side of the node carried more; the panel lists both.
  const sent = node.outflow > node.inflow
  const amount = sent ? node.outflow : node.inflow
  const unassigned = node.type === 'unassigned'
  const label = `${node.name}, ${NODE_TYPE_LABEL[node.type].toLowerCase()}. Received ${formatMoney(node.inflow)}, sent ${formatMoney(node.outflow)}.`

  return (
    <g
      role="button"
      tabIndex={0}
      aria-label={label}
      aria-pressed={selected}
      onFocus={onSelect}
      onClick={onSelect}
      onKeyDown={(e) => activate(e, onSelect)}
      style={{ opacity: dimmed ? DIM : 1 }}
      className="group cursor-pointer outline-none transition-opacity duration-200 ease-out motion-reduce:transition-none"
    >
      <title>{node.name}</title>
      <rect
        x={node.x - 4}
        y={node.y - 4}
        width={NODE_W + 8}
        height={node.h + 8}
        fill="none"
        strokeWidth={2}
        className="stroke-transparent group-focus-visible:stroke-ring"
      />
      <rect
        x={node.x}
        y={node.y}
        width={NODE_W}
        height={node.h}
        strokeWidth={selected ? 3 : 1.5}
        strokeDasharray={unassigned ? '5 4' : undefined}
        className={cn(
          'transition-colors duration-150',
          selected ? 'fill-report/30' : 'fill-card',
          selected
            ? 'stroke-primary'
            : unassigned
              ? 'stroke-flag'
              : 'stroke-foreground group-hover:stroke-primary',
        )}
      />
      <text
        x={node.x + NODE_PAD}
        y={node.y + nameBaseline(0)}
        className="fill-foreground font-semibold"
        style={{ fontSize: NAME_SIZE }}
      >
        {node.lines.map((line, i) => (
          <tspan
            key={i}
            x={node.x + NODE_PAD}
            y={node.y + nameBaseline(i)}
          >
            {line}
          </tspan>
        ))}
      </text>
      <text
        x={node.x + NODE_PAD}
        y={node.y + metaBaseline(node)}
        className="fill-muted-foreground text-[13.5px]"
      >
        {NODE_TYPE_LABEL[node.type]}
      </text>
      <text
        x={node.x + NODE_W - NODE_PAD}
        y={node.y + metaBaseline(node)}
        textAnchor="end"
        className="fill-foreground text-[13.5px] font-semibold tabular-nums"
      >
        <tspan className="fill-muted-foreground font-normal">
          {sent ? 'out KES ' : 'in KES '}
        </tspan>
        {formatCompact(amount)}
      </text>
    </g>
  )
}

function MapEdge({
  edge,
  from,
  to,
  selected,
  dimmed,
  onSelect,
}: {
  edge: GraphEdge
  from: GraphNode
  to: GraphNode
  selected: boolean
  dimmed: boolean
  onSelect: () => void
}) {
  const stroke = proofStroke(edge.status)
  const count = edge.transactions.length
  const label = `${from.name} to ${to.name}: ${formatMoney(edge.amount)}, ${count} ${count === 1 ? 'movement' : 'movements'}, ${PROOF_STATUS_LABEL[edge.status].toLowerCase()}.`
  const cross = Math.max(5, edge.width / 2 + 3)

  return (
    <g
      role="button"
      tabIndex={0}
      aria-label={label}
      aria-pressed={selected}
      onFocus={onSelect}
      onClick={onSelect}
      onKeyDown={(e) => activate(e, onSelect)}
      style={{ opacity: dimmed ? DIM : 1 }}
      className="group cursor-pointer outline-none transition-opacity duration-200 ease-out motion-reduce:transition-none"
    >
      <title>{label}</title>
      {/* Selection halo in report blue, focus halo in the ring colour. */}
      <path
        d={edge.path}
        fill="none"
        strokeWidth={edge.width + 10}
        className={cn(
          'stroke-transparent group-focus-visible:stroke-ring/40',
          selected && 'stroke-report',
        )}
      />
      <path
        d={edge.path}
        fill="none"
        strokeWidth={edge.width}
        strokeDasharray={scaleDash(stroke.strokeDasharray, edge.width)}
        className={cn(stroke.className, 'opacity-85 group-hover:opacity-100')}
      />
      {edge.status === 'flagged' && (
        <path
          d={`M ${edge.mid.x - cross} ${edge.mid.y - cross} L ${edge.mid.x + cross} ${edge.mid.y + cross} M ${edge.mid.x + cross} ${edge.mid.y - cross} L ${edge.mid.x - cross} ${edge.mid.y + cross}`}
          strokeWidth={3}
          className="stroke-flag"
        />
      )}
      {/* Wide transparent hit area so thin lines are easy to click. */}
      <path
        d={edge.path}
        fill="none"
        strokeWidth={Math.max(16, edge.width + 8)}
        stroke="transparent"
        pointerEvents="stroke"
      />
    </g>
  )
}

function AmountTag({ edge }: { edge: GraphEdge }) {
  const text = formatMoney(edge.amount)
  const w = text.length * 8.4 + 18
  return (
    <g aria-hidden="true" pointerEvents="none">
      <rect
        x={edge.mid.x - w / 2}
        y={edge.mid.y - 26 - edge.width / 2}
        width={w}
        height={24}
        className="fill-foreground"
      />
      <text
        x={edge.mid.x}
        y={edge.mid.y - 9 - edge.width / 2}
        textAnchor="middle"
        className="fill-background text-[13.5px] font-semibold tabular-nums"
      >
        {text}
      </text>
    </g>
  )
}
