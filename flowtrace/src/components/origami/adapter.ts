import type { FlowDataResult } from '@/lib/supabase'
import type { OrigamiNodeData } from './node'
import type { NodeSize } from './patterns'

const nodeIcons: Record<string, string> = {
  donor: '💝',
  account: '💰',
  department: '🏢',
  project: '📋',
  vendor: '📦',
  beneficiary: '👥',
}

const NODE_SPACING_X = 240
const LEVEL_SPACING_Y = 260

export function determineNodeSize(value: number): NodeSize {
  const magnitude = Math.log10(Math.max(value, 1))
  if (magnitude >= 7) return 'xl'
  if (magnitude >= 6) return 'lg'
  if (magnitude >= 5) return 'md'
  if (magnitude >= 4) return 'sm'
  return 'xs'
}

// Converts V2 flow data into origami nodes. Each node's parent is the source
// of its largest inflow, so the graph collapses into a tree the origami view
// can fold and unfold. Positions come from a level-based auto layout.
export function toOrigamiNodes(data: FlowDataResult): {
  nodes: Array<OrigamiNodeData>
  width: number
  height: number
} {
  const nodeIds = new Set(data.nodes.map((n) => n.id))
  const edges = data.edges.filter(
    (e) =>
      nodeIds.has(e.source) && nodeIds.has(e.target) && e.source !== e.target,
  )

  const parentOf = new Map<string, string>()
  const largestInflow = new Map<string, number>()
  const txCount = new Map<string, number>()
  for (const edge of edges) {
    if (edge.value > (largestInflow.get(edge.target) ?? -1)) {
      largestInflow.set(edge.target, edge.value)
      parentOf.set(edge.target, edge.source)
    }
    txCount.set(edge.source, (txCount.get(edge.source) ?? 0) + edge.count)
    txCount.set(edge.target, (txCount.get(edge.target) ?? 0) + edge.count)
  }

  // Break cycles: drop the parent link that closes a loop
  for (const id of nodeIds) {
    const seen = new Set<string>([id])
    let current = parentOf.get(id)
    while (current) {
      if (seen.has(current)) {
        parentOf.delete(id)
        break
      }
      seen.add(current)
      current = parentOf.get(current)
    }
  }

  const childrenOf = new Map<string, Array<string>>()
  for (const [child, parent] of parentOf) {
    childrenOf.set(parent, [...(childrenOf.get(parent) ?? []), child])
  }

  const levelOf = (id: string): number => {
    let level = 0
    let current = parentOf.get(id)
    while (current) {
      level++
      current = parentOf.get(current)
    }
    return level
  }

  const currency = data.summary.currency || data.edges[0]?.currency || ''

  const unpositioned: Array<OrigamiNodeData> = data.nodes.map((node) => {
    // Show what flows through the node; donors and vendors often hold no balance
    const value = Math.max(node.balance, node.inflow, node.outflow)
    const verificationCount = node.verification_count ?? 0
    return {
      id: node.id,
      name: node.name,
      type: node.type,
      balance: value,
      currency: currency ? `${currency} ` : '',
      transactions: txCount.get(node.id) ?? 0,
      size: determineNodeSize(value),
      x: 0,
      y: 0,
      icon: nodeIcons[node.type] ?? '💼',
      level: levelOf(node.id),
      parent: parentOf.get(node.id),
      children: childrenOf.get(node.id) ?? [],
      verified:
        verificationCount > 0 && node.verified_count === verificationCount,
    }
  })

  // Lay out level by level with fixed spacing, centring each row and ordering
  // nodes by their parent's position so links cross as little as possible
  const byLevel = new Map<number, Array<OrigamiNodeData>>()
  for (const node of unpositioned) {
    byLevel.set(node.level, [...(byLevel.get(node.level) ?? []), node])
  }
  const levels = [...byLevel.keys()].sort((a, b) => a - b)
  const widest = Math.max(1, ...[...byLevel.values()].map((row) => row.length))
  const width = Math.max(1000, (widest + 1) * NODE_SPACING_X)
  const height = Math.max(600, (levels.length + 1) * LEVEL_SPACING_Y)

  const xOf = new Map<string, number>()
  const nodes: Array<OrigamiNodeData> = []
  for (const level of levels) {
    const row = byLevel.get(level)!
    const parentX = (n: OrigamiNodeData) =>
      (n.parent ? xOf.get(n.parent) : 0) ?? 0
    row.sort((a, b) => parentX(a) - parentX(b) || b.balance - a.balance)
    row.forEach((node, i) => {
      const x = width / 2 + (i - (row.length - 1) / 2) * NODE_SPACING_X
      xOf.set(node.id, x)
      nodes.push({
        ...node,
        x,
        y: LEVEL_SPACING_Y / 2 + level * LEVEL_SPACING_Y,
      })
    })
  }

  return { nodes, width, height }
}
