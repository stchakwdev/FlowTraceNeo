import type { TransactionWithNodes } from '@/lib/types'
import type { ProofStatus } from '@/components/report'

// ---------------------------------------------------------------------------
// Periods
// ---------------------------------------------------------------------------

export const PERIODS = [
  { id: 'week', label: 'Week', days: 7, phrase: 'last week' },
  { id: 'month', label: 'Month', days: 30, phrase: 'last month' },
  { id: 'quarter', label: 'Quarter', days: 91, phrase: 'last quarter' },
  { id: 'year', label: 'Year', days: 365, phrase: 'last year' },
] as const

export type PeriodId = (typeof PERIODS)[number]['id']
export const WIDEST_PERIOD: PeriodId = 'year'

const DAY = 24 * 60 * 60 * 1000

/** Calendar date ('2026-01-18' or a timestamp) as UTC midnight milliseconds. */
function dayValue(date: string) {
  return Date.parse(`${date.slice(0, 10)}T00:00:00Z`)
}

/**
 * The window a period covers, counted back from the latest transaction date in
 * the data (not from today), so a quiet dataset still shows its recent work.
 */
export function periodWindow(
  transactions: Array<TransactionWithNodes>,
  period: PeriodId,
) {
  if (transactions.length === 0) return null
  const latest = Math.max(...transactions.map((t) => dayValue(t.transaction_date)))
  const days = PERIODS.find((p) => p.id === period)?.days ?? 30
  const start = latest - (days - 1) * DAY
  return { start: new Date(start), end: new Date(latest) }
}

export function filterByPeriod(
  transactions: Array<TransactionWithNodes>,
  period: PeriodId,
) {
  const window = periodWindow(transactions, period)
  if (!window) return []
  const start = window.start.getTime()
  const end = window.end.getTime()
  return transactions.filter((t) => {
    const d = dayValue(t.transaction_date)
    return d >= start && d <= end
  })
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

export type NodeType =
  | 'donor'
  | 'account'
  | 'department'
  | 'project'
  | 'vendor'
  | 'beneficiary'
  | 'unassigned'

/** Money order: donors, accounts, departments and projects, then recipients. */
export const BANDS = [
  { label: 'Donors', types: ['donor'] },
  { label: 'Accounts', types: ['account'] },
  { label: 'Departments, projects', types: ['department', 'project'] },
  { label: 'Recipients', types: ['vendor', 'beneficiary', 'unassigned'] },
] as const

export const NODE_TYPE_LABEL: Record<NodeType, string> = {
  donor: 'Donor',
  account: 'Account',
  department: 'Department',
  project: 'Project',
  vendor: 'Vendor',
  beneficiary: 'Beneficiary',
  unassigned: 'No recipient',
}

function bandOf(type: string) {
  const i = BANDS.findIndex((b) => (b.types as ReadonlyArray<string>).includes(type))
  return i === -1 ? 1 : i
}

export const UNASSIGNED_ID = '__no-recipient__'

export interface GraphNode {
  id: string
  name: string
  type: NodeType
  band: number
  column: number
  inflow: number
  outflow: number
  x: number
  y: number
  /** The name wrapped to the node's width, one entry per rendered line. */
  lines: Array<string>
  h: number
}

export interface GraphEdge {
  id: string
  source: string
  target: string
  amount: number
  status: ProofStatus
  transactions: Array<TransactionWithNodes>
  width: number
  path: string
  mid: { x: number; y: number }
}

export interface FlowGraph {
  nodes: Array<GraphNode>
  edges: Array<GraphEdge>
  columns: Array<{ x: number; label: string | null }>
  width: number
  height: number
  totals: {
    moved: number
    evidenced: number
    movements: number
  }
}

export const NODE_W = 196
export const NODE_PAD = 12
/** Name type size and leading, in map units (rendered size = size x zoom). */
export const NAME_SIZE = 14
export const NAME_LEADING = 18
const NAME_TOP = 25 // first name baseline below the node's top edge
const META_GAP = 22 // last name baseline to the type and amount baseline
const META_BOTTOM = 13 // meta baseline to the node's bottom edge
const MAX_LINES = 3
// Generous per-character width for Archivo semibold at NAME_SIZE, so the
// estimate errs towards wrapping early rather than overflowing the box.
const CHAR_W = NAME_SIZE * 0.56
const COL_GAP = 40
const ROW_GAP = 18
const PAD_X = 16
const PAD_BOTTOM = 16
const TOP = 40
const MIN_STROKE = 2
const MAX_STROKE = 16

/** Wrap a name onto at most MAX_LINES lines that fit inside a node. */
export function wrapName(name: string): Array<string> {
  const perLine = Math.floor((NODE_W - NODE_PAD * 2) / CHAR_W)
  const lines: Array<string> = []
  let line = ''
  for (const word of name.split(/\s+/).filter(Boolean)) {
    const next = line ? `${line} ${word}` : word
    if (next.length <= perLine) {
      line = next
    } else {
      if (line) lines.push(line)
      // A single word longer than a line is cut hard.
      line = word.length > perLine ? `${word.slice(0, perLine - 1)}-` : word
    }
  }
  if (line) lines.push(line)
  if (lines.length > MAX_LINES) {
    const kept = lines.slice(0, MAX_LINES)
    const last = kept[MAX_LINES - 1]
    kept[MAX_LINES - 1] =
      last.length >= perLine ? `${last.slice(0, perLine - 1).trimEnd()}…` : `${last}…`
    return kept
  }
  return lines.length ? lines : ['']
}

/** Node height for a given number of name lines. */
function nodeHeight(lines: number) {
  return NAME_TOP + (lines - 1) * NAME_LEADING + META_GAP + META_BOTTOM
}

/** Baselines inside a node, relative to its top edge. */
export function nameBaseline(line: number) {
  return NAME_TOP + line * NAME_LEADING
}
export function metaBaseline(node: GraphNode) {
  return node.h - META_BOTTOM
}

const STATUS_RANK: Record<ProofStatus, number> = {
  verified: 0,
  pending: 1,
  flagged: 2,
  rejected: 3,
}

export function buildFlowGraph(
  transactions: Array<TransactionWithNodes>,
): FlowGraph {
  // Nodes touched by the period's movements.
  const nodes = new Map<string, GraphNode>()
  // `id` without a resolved `ref` means the node exists but is out of reach;
  // no id at all means the movement has no recipient recorded.
  const touch = (
    ref: TransactionWithNodes['from_node'],
    id: string | null,
  ): string | null => {
    if (!ref) {
      if (id) return null
      if (!nodes.has(UNASSIGNED_ID)) {
        nodes.set(UNASSIGNED_ID, {
          id: UNASSIGNED_ID,
          name: 'No recipient recorded',
          type: 'unassigned',
          band: bandOf('unassigned'),
          column: 0,
          inflow: 0,
          outflow: 0,
          x: 0,
          y: 0,
          lines: [],
          h: 0,
        })
      }
      return UNASSIGNED_ID
    }
    if (!nodes.has(ref.id)) {
      nodes.set(ref.id, {
        id: ref.id,
        name: ref.name,
        type: ref.type as NodeType,
        band: bandOf(ref.type),
        column: 0,
        inflow: 0,
        outflow: 0,
        x: 0,
        y: 0,
        lines: [],
        h: 0,
      })
    }
    return ref.id
  }

  // Aggregate movements into one edge per source and target pair.
  const edgeMap = new Map<string, Omit<GraphEdge, 'width' | 'path' | 'mid'>>()
  let moved = 0
  let evidenced = 0
  for (const t of transactions) {
    if (!t.from_node) continue
    const source = touch(t.from_node, t.from_node_id)
    const target = touch(t.to_node, t.to_node_id)
    if (!source || !target) continue
    const amount = Number(t.amount) || 0
    moved += amount
    if (t.verification_status === 'verified') evidenced += amount
    nodes.get(source)!.outflow += amount
    nodes.get(target)!.inflow += amount
    const id = `${source}->${target}`
    const existing = edgeMap.get(id)
    if (existing) {
      existing.amount += amount
      existing.transactions.push(t)
      if (STATUS_RANK[t.verification_status] > STATUS_RANK[existing.status]) {
        existing.status = t.verification_status
      }
    } else {
      edgeMap.set(id, {
        id,
        source,
        target,
        amount,
        status: t.verification_status,
        transactions: [t],
      })
    }
  }

  // Within a band, a node fed by another node of the same band (an account
  // funding another account) moves one sub-column to the right.
  const depth = new Map<string, number>()
  for (const n of nodes.values()) depth.set(n.id, 0)
  for (let pass = 0; pass < nodes.size; pass++) {
    let changed = false
    for (const e of edgeMap.values()) {
      const s = nodes.get(e.source)!
      const t = nodes.get(e.target)!
      if (s.band !== t.band) continue
      const want = depth.get(s.id)! + 1
      if (want > depth.get(t.id)! && want < nodes.size) {
        depth.set(t.id, want)
        changed = true
      }
    }
    if (!changed) break
  }

  const columnKeys = [
    ...new Set([...nodes.values()].map((n) => `${n.band}:${depth.get(n.id)}`)),
  ].sort((a, b) => {
    const [ab, ad] = a.split(':').map(Number)
    const [bb, bd] = b.split(':').map(Number)
    return ab - bb || ad - bd
  })
  for (const n of nodes.values()) {
    n.column = columnKeys.indexOf(`${n.band}:${depth.get(n.id)}`)
  }

  // Order rows: the first column by amount, later columns by the average row
  // of whatever feeds them, which keeps lines from crossing more than needed.
  const byColumn: Array<Array<GraphNode>> = columnKeys.map(() => [])
  for (const n of nodes.values()) byColumn[n.column].push(n)
  const rowOf = new Map<string, number>()
  const edges = [...edgeMap.values()]
  byColumn.forEach((col, ci) => {
    const score = (n: GraphNode) => {
      const feeds = edges.filter(
        (e) => e.target === n.id && rowOf.has(e.source),
      )
      if (ci === 0 || feeds.length === 0) return Number.POSITIVE_INFINITY
      const weight = feeds.reduce((s, e) => s + e.amount, 0) || 1
      return (
        feeds.reduce((s, e) => s + rowOf.get(e.source)! * e.amount, 0) / weight
      )
    }
    col.sort(
      (a, b) =>
        score(a) - score(b) ||
        Math.max(b.inflow, b.outflow) - Math.max(a.inflow, a.outflow) ||
        a.name.localeCompare(b.name),
    )
    // Unassigned recipients always sit at the foot of their column.
    col.sort(
      (a, b) =>
        Number(a.type === 'unassigned') - Number(b.type === 'unassigned'),
    )
    col.forEach((n, i) => rowOf.set(n.id, i))
  })

  // Every column hangs from the same top line so the map has no empty
  // quadrant; the canvas is as tall as the tallest column.
  let contentH = 0
  byColumn.forEach((col, ci) => {
    let y = TOP
    col.forEach((n) => {
      n.lines = wrapName(n.name)
      n.h = nodeHeight(n.lines.length)
      n.x = PAD_X + ci * (NODE_W + COL_GAP)
      n.y = y
      y += n.h + ROW_GAP
    })
    contentH = Math.max(contentH, y - ROW_GAP - TOP)
  })

  // Line weight carries amount (square-root scale so small movements stay
  // visible beside multi-million grants).
  const maxAmount = Math.max(1, ...edges.map((e) => e.amount))
  const widthOf = (amount: number) =>
    MIN_STROKE + (MAX_STROKE - MIN_STROKE) * Math.sqrt(amount / maxAmount)

  // Spread each node's lines across its edge so they do not pile on one point.
  const ports = (list: Array<(typeof edges)[number]>, key: 'source' | 'target') => {
    const map = new Map<string, number>()
    const groups = new Map<string, typeof list>()
    for (const e of list) {
      const owner = e[key]
      if (!groups.has(owner)) groups.set(owner, [])
      groups.get(owner)!.push(e)
    }
    for (const [owner, group] of groups) {
      const other = key === 'source' ? 'target' : 'source'
      group.sort((a, b) => nodes.get(a[other])!.y - nodes.get(b[other])!.y)
      const n = nodes.get(owner)!
      group.forEach((e, i) =>
        map.set(e.id, n.y + (n.h * (i + 1)) / (group.length + 1)),
      )
    }
    return map
  }
  const outPorts = ports(edges, 'source')
  const inPorts = ports(edges, 'target')

  const laidOut: Array<GraphEdge> = edges.map((e) => {
    const s = nodes.get(e.source)!
    const t = nodes.get(e.target)!
    const x1 = s.x + NODE_W
    const y1 = outPorts.get(e.id)!
    const x2 = t.x
    const y2 = inPorts.get(e.id)!
    const mx = (x1 + x2) / 2
    return {
      ...e,
      width: widthOf(e.amount),
      path: `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`,
      mid: { x: mx, y: (y1 + y2) / 2 },
    }
  })

  const columns = columnKeys.map((key, ci) => {
    const [band, d] = key.split(':').map(Number)
    return {
      x: PAD_X + ci * (NODE_W + COL_GAP),
      label: d === 0 ? BANDS[band].label : null,
    }
  })

  return {
    nodes: [...nodes.values()],
    edges: laidOut,
    columns,
    width: PAD_X * 2 + columnKeys.length * NODE_W + (columnKeys.length - 1) * COL_GAP,
    height: TOP + contentH + PAD_BOTTOM,
    totals: { moved, evidenced, movements: transactions.length },
  }
}

/** Every node and edge upstream and downstream of the selection. */
export function connectedSet(
  graph: FlowGraph,
  selection: Selection | null,
): { nodes: Set<string>; edges: Set<string> } | null {
  if (!selection) return null
  const nodes = new Set<string>()
  const edges = new Set<string>()
  const walk = (start: string, dir: 'up' | 'down') => {
    const queue = [start]
    const seen = new Set<string>([start])
    while (queue.length) {
      const id = queue.shift()!
      nodes.add(id)
      for (const e of graph.edges) {
        const from = dir === 'down' ? e.source : e.target
        const to = dir === 'down' ? e.target : e.source
        if (from !== id) continue
        edges.add(e.id)
        if (!seen.has(to)) {
          seen.add(to)
          queue.push(to)
        }
      }
    }
  }
  if (selection.kind === 'node') {
    walk(selection.id, 'up')
    walk(selection.id, 'down')
  } else {
    const edge = graph.edges.find((e) => e.id === selection.id)
    if (!edge) return null
    edges.add(edge.id)
    walk(edge.source, 'up')
    walk(edge.target, 'down')
  }
  return { nodes, edges }
}

export type Selection = { kind: 'node' | 'edge'; id: string }
