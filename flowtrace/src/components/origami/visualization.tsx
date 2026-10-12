import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { OrigamiNode } from './node'
import { shouldDisableAnimations } from './animations'
import type { OrigamiNodeData } from './node'
import type React from 'react'
import './styles.css'

export interface OrigamiVisualizationProps {
  nodes: Array<OrigamiNodeData>
  connections?: Array<{ source: string; target: string }>
  width?: number
  height?: number
  onNodeClick?: (nodeId: string) => void
  onNodeHover?: (nodeId: string | null) => void
  performanceMode?: boolean
  enableAnimations?: boolean
  onExpandAll?: () => void
  onCollapseAll?: () => void
  expandAllTrigger?: number
  collapseAllTrigger?: number
}

interface PathHighlight {
  nodeIds: Array<string>
  connectionIds: Array<string>
}

export function OrigamiVisualization({
  nodes,

  width = 1000,
  height = 600,
  onNodeClick,
  onNodeHover,
  performanceMode = false,
  enableAnimations = true,
  expandAllTrigger,
  collapseAllTrigger,
}: OrigamiVisualizationProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const svgRef = useRef<SVGSVGElement>(null)
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set())
  const [highlightedPath, setHighlightedPath] = useState<PathHighlight | null>(
    null,
  )
  const [visibleNodes, setVisibleNodes] = useState<Set<string>>(new Set())
  const [isInitialized, setIsInitialized] = useState(false)

  // Performance check
  const animationsDisabled = useMemo(
    () =>
      !enableAnimations ||
      performanceMode ||
      shouldDisableAnimations(nodes.length),
    [enableAnimations, performanceMode, nodes.length],
  )

  // Build node lookup and hierarchy
  const nodeMap = useMemo(() => {
    const map = new Map<string, OrigamiNodeData>()
    nodes.forEach((node) => map.set(node.id, node))
    return map
  }, [nodes])

  // Calculate initial visible nodes (root level)
  useEffect(() => {
    const rootNodes = nodes.filter((node) => !node.parent || node.level === 0)
    const rootIds = new Set(rootNodes.map((node) => node.id))
    setVisibleNodes(rootIds)
    setIsInitialized(true)
  }, [nodes])

  // Handle expand all trigger
  useEffect(() => {
    if (expandAllTrigger && isInitialized) {
      const allNodeIds = new Set(nodes.map((node) => node.id))
      setExpandedNodes(allNodeIds)
      setVisibleNodes(allNodeIds)
    }
  }, [expandAllTrigger, nodes, isInitialized])

  // Handle collapse all trigger
  useEffect(() => {
    if (collapseAllTrigger && isInitialized) {
      setExpandedNodes(new Set())
      const rootNodes = nodes.filter((node) => !node.parent || node.level === 0)
      const rootIds = new Set(rootNodes.map((node) => node.id))
      setVisibleNodes(rootIds)
    }
  }, [collapseAllTrigger, nodes, isInitialized])

  // Find path from node to root
  const findPathToRoot = useCallback(
    (nodeId: string): Array<string> => {
      const path: Array<string> = []
      let currentId: string | undefined = nodeId
      const visited = new Set<string>()

      while (currentId && !visited.has(currentId)) {
        visited.add(currentId)
        path.push(currentId)
        const node = nodeMap.get(currentId)
        currentId = node?.parent
      }

      return path
    },
    [nodeMap],
  )

  // Find connections in path
  const findConnectionsInPath = useCallback(
    (pathNodes: Array<string>): Array<string> => {
      const connectionIds: Array<string> = []

      for (let i = 0; i < pathNodes.length - 1; i++) {
        const sourceId = pathNodes[i + 1] // Parent
        const targetId = pathNodes[i] // Child
        connectionIds.push(`${sourceId}-${targetId}`)
      }

      return connectionIds
    },
    [],
  )

  // Handle node hover for path highlighting
  const handleNodeHover = useCallback(
    (nodeId: string, event: 'enter' | 'leave') => {
      if (event === 'enter') {
        const pathNodes = findPathToRoot(nodeId)
        const connectionIds = findConnectionsInPath(pathNodes)

        setHighlightedPath({
          nodeIds: pathNodes,
          connectionIds,
        })

        if (onNodeHover) onNodeHover(nodeId)
      } else {
        setHighlightedPath(null)
        if (onNodeHover) onNodeHover(null)
      }
    },
    [findPathToRoot, findConnectionsInPath, onNodeHover],
  )

  // Handle node click
  const handleNodeClick = useCallback(
    (nodeId: string) => {
      if (onNodeClick) onNodeClick(nodeId)
    },
    [onNodeClick],
  )

  // Handle node expansion
  const handleNodeExpand = useCallback(
    (nodeId: string) => {
      const node = nodeMap.get(nodeId)
      if (!node?.children || node.children.length === 0) return

      setExpandedNodes((prev) => {
        const newExpanded = new Set(prev)
        const isExpanded = newExpanded.has(nodeId)

        if (isExpanded) {
          // Collapse: remove node and all its descendants from expanded
          newExpanded.delete(nodeId)
          const collapseDescendants = (id: string) => {
            const current = nodeMap.get(id)
            if (current?.children) {
              current.children.forEach((childId) => {
                newExpanded.delete(childId)
                collapseDescendants(childId)
              })
            }
          }
          collapseDescendants(nodeId)

          // Update visible nodes
          setVisibleNodes((prevVisible) => {
            const newVisible = new Set(prevVisible)
            const hideDescendants = (id: string) => {
              const current = nodeMap.get(id)
              if (current?.children) {
                current.children.forEach((childId) => {
                  newVisible.delete(childId)
                  hideDescendants(childId)
                })
              }
            }
            hideDescendants(nodeId)
            return newVisible
          })
        } else {
          // Expand: add to expanded and show immediate children
          newExpanded.add(nodeId)

          setVisibleNodes((prevVisible) => {
            const newVisible = new Set(prevVisible)
            node.children?.forEach((childId) => {
              newVisible.add(childId)
            })
            return newVisible
          })
        }

        return newExpanded
      })
    },
    [nodeMap],
  )

  // Generate connection lines with better reactivity
  const renderConnections = useCallback(() => {
    if (!visibleNodes.size) return null

    const connectionElements: Array<React.JSX.Element> = []

    // Generate connections from parent-child relationships
    nodes.forEach((node) => {
      if (
        node.parent &&
        visibleNodes.has(node.id) &&
        visibleNodes.has(node.parent)
      ) {
        const parentNode = nodeMap.get(node.parent)
        if (!parentNode) return

        const connectionId = `${node.parent}-${node.id}`
        const isHighlighted =
          highlightedPath?.connectionIds.includes(connectionId)

        // Calculate positions
        const x1 = parentNode.x
        const y1 = parentNode.y
        const x2 = node.x
        const y2 = node.y

        // Create curved path with better curve calculation
        const dx = x2 - x1
        const dy = y2 - y1
        const controlOffset = Math.sqrt(dx * dx + dy * dy) * 0.2
        const cx = x1 + dx / 2
        const cy = y1 + dy / 2 - controlOffset

        connectionElements.push(
          <path
            key={connectionId}
            className={`origami-connection ${isHighlighted ? 'highlighted' : ''}`}
            d={`M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`}
            stroke={
              isHighlighted
                ? 'url(#connection-highlight)'
                : 'url(#connection-gradient)'
            }
            strokeWidth={isHighlighted ? 3 : 2}
            fill="none"
            style={{
              transition: 'all 0.3s ease',
              opacity: isHighlighted ? 1 : 0.6,
            }}
          />,
        )
      }
    })

    return connectionElements
  }, [nodes, visibleNodes, nodeMap, highlightedPath])

  // Container classes
  const containerClasses = [
    'origami-nodes-container',
    animationsDisabled && 'origami-performance-mode',
    !isInitialized && 'origami-loading',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <div
      ref={containerRef}
      className={containerClasses}
      style={{
        width,
        height,
        position: 'relative',
        background: 'transparent',
        overflow: 'hidden',
      }}
    >
      {/* SVG for connection lines */}
      <svg
        ref={svgRef}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          zIndex: 1,
        }}
      >
        <defs>
          {/* Gradient definitions for connections */}
          <linearGradient
            id="connection-gradient"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="0%"
          >
            <stop offset="0%" stopColor="rgba(0, 102, 255, 0.2)" />
            <stop offset="100%" stopColor="rgba(0, 212, 170, 0.2)" />
          </linearGradient>

          <linearGradient
            id="connection-highlight"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="0%"
          >
            <stop offset="0%" stopColor="rgba(0, 102, 255, 0.8)" />
            <stop offset="100%" stopColor="rgba(0, 212, 170, 0.8)" />
          </linearGradient>
        </defs>

        <g className="connections-group">{renderConnections()}</g>
      </svg>

      {/* Render nodes */}
      {nodes.map((node) => {
        const isVisible = visibleNodes.has(node.id)
        const isHighlighted =
          highlightedPath?.nodeIds.includes(node.id) || false
        const isExpanded = expandedNodes.has(node.id)

        return (
          <OrigamiNode
            key={node.id}
            data={node}
            isHighlighted={isHighlighted}
            isExpanded={isExpanded}
            isVisible={isVisible}
            onHover={handleNodeHover}
            onClick={handleNodeClick}
            onExpand={handleNodeExpand}
          />
        )
      })}

      {/* Debug info (development only) */}
      {import.meta.env.DEV && (
        <div
          style={{
            position: 'absolute',
            top: 10,
            left: 10,
            background: 'rgba(0, 0, 0, 0.7)',
            color: 'white',
            padding: '8px 12px',
            borderRadius: 4,
            fontSize: 12,
            fontFamily: 'monospace',
            zIndex: 1000,
            pointerEvents: 'none',
          }}
        >
          <div>Nodes: {nodes.length}</div>
          <div>Visible: {visibleNodes.size}</div>
          <div>Expanded: {expandedNodes.size}</div>
          <div>Animations: {animationsDisabled ? 'OFF' : 'ON'}</div>
          {highlightedPath && (
            <div>Highlighted Path: {highlightedPath.nodeIds.length} nodes</div>
          )}
        </div>
      )}
    </div>
  )
}

// Control buttons component
export function OrigamiControls({
  onExpandAll,
  onCollapseAll,
  onRandomizeActivity,
  onToggleAnimations,
  animationsEnabled = true,
}: {
  onExpandAll?: () => void
  onCollapseAll?: () => void
  onRandomizeActivity?: () => void
  onToggleAnimations?: () => void
  animationsEnabled?: boolean
}) {
  return (
    <div
      className="origami-controls"
      style={{
        position: 'absolute',
        bottom: 20,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        gap: 10,
        background: 'rgba(255, 255, 255, 0.05)',
        backdropFilter: 'blur(20px)',
        padding: 10,
        borderRadius: 8,
        border: '1px solid rgba(255, 255, 255, 0.1)',
        zIndex: 1000,
      }}
    >
      <button
        onClick={onExpandAll}
        style={{
          background: 'rgba(0, 102, 255, 0.2)',
          border: '1px solid rgba(0, 102, 255, 0.3)',
          color: '#fff',
          padding: '8px 16px',
          borderRadius: 6,
          cursor: 'pointer',
          fontSize: 14,
          transition: 'all 0.2s',
        }}
      >
        Unfold All
      </button>

      <button
        onClick={onCollapseAll}
        style={{
          background: 'rgba(0, 102, 255, 0.2)',
          border: '1px solid rgba(0, 102, 255, 0.3)',
          color: '#fff',
          padding: '8px 16px',
          borderRadius: 6,
          cursor: 'pointer',
          fontSize: 14,
          transition: 'all 0.2s',
        }}
      >
        Fold All
      </button>

      {onRandomizeActivity && (
        <button
          onClick={onRandomizeActivity}
          style={{
            background: 'rgba(0, 102, 255, 0.2)',
            border: '1px solid rgba(0, 102, 255, 0.3)',
            color: '#fff',
            padding: '8px 16px',
            borderRadius: 6,
            cursor: 'pointer',
            fontSize: 14,
            transition: 'all 0.2s',
          }}
        >
          Simulate Activity
        </button>
      )}

      <button
        onClick={onToggleAnimations}
        style={{
          background: animationsEnabled
            ? 'rgba(0, 212, 170, 0.2)'
            : 'rgba(255, 107, 0, 0.2)',
          border: animationsEnabled
            ? '1px solid rgba(0, 212, 170, 0.3)'
            : '1px solid rgba(255, 107, 0, 0.3)',
          color: '#fff',
          padding: '8px 16px',
          borderRadius: 6,
          cursor: 'pointer',
          fontSize: 14,
          transition: 'all 0.2s',
        }}
      >
        {animationsEnabled ? 'Disable' : 'Enable'} Animations
      </button>
    </div>
  )
}
