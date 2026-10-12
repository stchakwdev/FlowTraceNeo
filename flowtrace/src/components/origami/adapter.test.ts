import { describe, expect, it } from 'vitest'
import { determineNodeSize, toOrigamiNodes } from './adapter'
import { calculateFitToViewTransform } from './bounds'
import type { FlowDataNode, FlowDataResult } from '@/lib/supabase'

function node(id: string, type: string, flow = 0): FlowDataNode {
  return { id, name: id, type, inflow: flow, outflow: flow, balance: 0 }
}

function flowData(
  nodes: Array<FlowDataNode>,
  edges: Array<[string, string, number]>,
): FlowDataResult {
  return {
    nodes,
    edges: edges.map(([source, target, value]) => ({
      source,
      target,
      value,
      count: 2,
      currency: 'KES',
    })),
    summary: {
      total_nodes: nodes.length,
      total_edges: edges.length,
      total_flow: 0,
      transaction_count: 0,
      currency: 'KES',
    },
  }
}

describe('determineNodeSize', () => {
  it('scales with the order of magnitude of the value', () => {
    expect(determineNodeSize(0)).toBe('xs')
    expect(determineNodeSize(50_000)).toBe('sm')
    expect(determineNodeSize(500_000)).toBe('md')
    expect(determineNodeSize(5_000_000)).toBe('lg')
    expect(determineNodeSize(50_000_000)).toBe('xl')
  })
})

describe('toOrigamiNodes', () => {
  it('parents each node to the source of its largest inflow', () => {
    const { nodes } = toOrigamiNodes(
      flowData(
        [
          node('donor', 'donor'),
          node('grant', 'donor'),
          node('acct', 'account'),
        ],
        [
          ['donor', 'acct', 100],
          ['grant', 'acct', 900],
        ],
      ),
    )
    const acct = nodes.find((n) => n.id === 'acct')!
    expect(acct.parent).toBe('grant')
    expect(acct.level).toBe(1)
    expect(acct.transactions).toBe(4)
    expect(nodes.find((n) => n.id === 'grant')!.children).toEqual(['acct'])
  })

  it('breaks cycles so every node still resolves to a root', () => {
    const { nodes } = toOrigamiNodes(
      flowData(
        [node('a', 'account'), node('b', 'department')],
        [
          ['a', 'b', 10],
          ['b', 'a', 5],
        ],
      ),
    )
    expect(nodes.some((n) => n.parent === undefined)).toBe(true)
    expect(nodes.every((n) => Number.isFinite(n.level))).toBe(true)
  })

  it('places every node inside the canvas with a deterministic layout', () => {
    const data = flowData(
      [
        node('d', 'donor', 1_000_000),
        node('x', 'account'),
        node('y', 'vendor'),
      ],
      [
        ['d', 'x', 10],
        ['x', 'y', 10],
      ],
    )
    const first = toOrigamiNodes(data)
    const second = toOrigamiNodes(data)
    expect(first.nodes).toEqual(second.nodes)
    for (const n of first.nodes) {
      expect(n.x).toBeGreaterThanOrEqual(0)
      expect(n.x).toBeLessThanOrEqual(first.width)
      expect(n.y).toBeGreaterThanOrEqual(0)
      expect(n.y).toBeLessThanOrEqual(first.height)
    }
    expect(first.nodes.find((n) => n.id === 'd')!.currency).toBe('KES ')
  })
})

describe('calculateFitToViewTransform', () => {
  it('keeps a positive scale when padding exceeds the container', () => {
    const { nodes } = toOrigamiNodes(
      flowData(
        [node('a', 'donor', 1_000_000), node('b', 'account')],
        [['a', 'b', 10]],
      ),
    )
    const { scale } = calculateFitToViewTransform(nodes, 800, 500, 250)
    expect(scale).toBeGreaterThan(0.1)
  })
})
