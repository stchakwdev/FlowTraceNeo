// The web flow map's layout pinned on the demo data (DT2): periodWindow, filterByPeriod,
// buildFlowGraph and connectedSet of flowtrace/src/components/flows/flow-graph.ts over the demo
// API's getTransactions, for every period, plus wrapName on chosen names. The desktop's
// `desktop::flows` ports the file and replays this fixture. Amounts are the web's major units.
import { canonical } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import { TRANSPARENCY_ENV, withFrozenClock } from './transparency'

const GRAPH = 'flowtrace/src/components/flows/flow-graph.ts'
const API = 'flowtrace/src/lib/demo/api.ts'

/** Names that exercise every branch of wrapName: short, two lines, three, cut, hard-cut word. */
const NAMES = [
  '',
  'General Fund',
  'Kenya Water Partnership Fund',
  'Turkana Community Water Project Phase Two Borehole Drilling and Maintenance',
  'Supercalifragilisticexpialidociousandmorewordsjoined',
  'A Supercalifragilisticexpialidociousandmorewordsjoined B',
  '  spaced   out   name  ',
  'Ministry of Health Pwani County Office for Maternal and Child Health',
]

export async function exportFlowGraph(outDir: string) {
  const seed = await import('../../../flowtrace/src/lib/demo/dataset.ts')
  const api = await import('../../../flowtrace/src/lib/demo/api.ts')
  const g = await import('../../../flowtrace/src/components/flows/flow-graph.ts')

  api.resetDemoStore()
  const all = await withFrozenClock(() => api.getTransactions(seed.DEMO_ORG_ID))
  api.resetDemoStore()

  const periods = g.PERIODS.map((p) => {
    const window = g.periodWindow(all, p.id)
    const inPeriod = g.filterByPeriod(all, p.id)
    const graph = g.buildFlowGraph(inPeriod)
    const selections = [
      ...graph.nodes.map((n) => ({ kind: 'node' as const, id: n.id })),
      ...graph.edges.map((e) => ({ kind: 'edge' as const, id: e.id })),
    ]
    return {
      period: p.id,
      window: window && { start: window.start.toISOString(), end: window.end.toISOString() },
      transaction_ids: inPeriod.map((t) => t.id),
      graph: {
        ...graph,
        edges: graph.edges.map(({ transactions, ...e }) => ({
          ...e,
          transaction_ids: transactions.map((t) => t.id),
        })),
      },
      connected: selections.map((s) => {
        const set = g.connectedSet(graph, s)
        return { selection: s, nodes: set && [...set.nodes], edges: set && [...set.edges] }
      }),
    }
  })

  writeFixture(outDir, 'transparency-flow-graph', {
    description:
      'The web flow map pinned on the demo transactions (getTransactions order, major units): for each ' +
      'period the window, the transactions in it, buildFlowGraph whole (nodes, edges with their ' +
      'transaction ids, columns, size, totals) and connectedSet for every node and edge; and wrapName on ' +
      'names that hit each branch.',
    sources: [`${GRAPH}#PERIODS`, `${GRAPH}#periodWindow`, `${GRAPH}#filterByPeriod`, `${GRAPH}#buildFlowGraph`, `${GRAPH}#connectedSet`, `${GRAPH}#wrapName`, `${API}#getTransactions`],
    covers: ['periodWindow', 'filterByPeriod', 'buildFlowGraph', 'connectedSet', 'wrapName'],
    env: TRANSPARENCY_ENV,
    constants: canonical({ periods: g.PERIODS, bands: g.BANDS, node_type_label: g.NODE_TYPE_LABEL, node_w: g.NODE_W }),
    wrap_name: NAMES.map((n) => [n, g.wrapName(n)]) as Json,
    periods: canonical(periods),
  })
}
