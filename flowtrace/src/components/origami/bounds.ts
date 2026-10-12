/**
 * Origami Bounds Calculation and Auto-Fitting Utilities
 * Handles dynamic sizing and clipping prevention for origami nodes
 */

import type { OrigamiNodeData } from './node'

export interface NodeBounds {
  minX: number
  maxX: number
  minY: number
  maxY: number
  width: number
  height: number
  centerX: number
  centerY: number
}

export interface CanvasSize {
  width: number
  height: number
  offsetX: number
  offsetY: number
  scale: number
}

const sizeConfig = {
  xl: { width: 200, height: 200 },
  lg: { width: 160, height: 160 },
  md: { width: 130, height: 130 },
  sm: { width: 100, height: 100 },
  xs: { width: 80, height: 80 },
}

/**
 * Calculate the bounds of all origami nodes including their sizes
 */
export function calculateNodeBounds(nodes: Array<OrigamiNodeData>): NodeBounds {
  if (nodes.length === 0) {
    return {
      minX: 0,
      maxX: 800,
      minY: 0,
      maxY: 600,
      width: 800,
      height: 600,
      centerX: 400,
      centerY: 300,
    }
  }

  let minX = Infinity
  let maxX = -Infinity
  let minY = Infinity
  let maxY = -Infinity

  nodes.forEach((node) => {
    const nodeSize = sizeConfig[node.size]
    const halfWidth = nodeSize.width / 2
    const halfHeight = nodeSize.height / 2

    // Calculate the actual bounds including node size
    const nodeMinX = node.x - halfWidth
    const nodeMaxX = node.x + halfWidth
    const nodeMinY = node.y - halfHeight
    const nodeMaxY = node.y + halfHeight

    minX = Math.min(minX, nodeMinX)
    maxX = Math.max(maxX, nodeMaxX)
    minY = Math.min(minY, nodeMinY)
    maxY = Math.max(maxY, nodeMaxY)
  })

  const width = maxX - minX
  const height = maxY - minY
  const centerX = minX + width / 2
  const centerY = minY + height / 2

  return {
    minX,
    maxX,
    minY,
    maxY,
    width,
    height,
    centerX,
    centerY,
  }
}

/**
 * Calculate optimal canvas size and positioning to fit all nodes
 */
export function calculateOptimalCanvas(
  nodes: Array<OrigamiNodeData>,
  containerWidth: number,
  containerHeight: number,
  padding: number = 100,
  maxScale: number = 1.5,
  minScale: number = 0.5,
): CanvasSize {
  const bounds = calculateNodeBounds(nodes)

  // Add padding to the bounds
  const requiredWidth = bounds.width + padding * 2
  const requiredHeight = bounds.height + padding * 2

  // Calculate scale to fit within container
  const scaleX = containerWidth / requiredWidth
  const scaleY = containerHeight / requiredHeight
  const optimalScale = Math.min(scaleX, scaleY)

  // Clamp scale within min/max limits
  const scale = Math.max(minScale, Math.min(maxScale, optimalScale))

  // Calculate the scaled canvas size
  const scaledWidth = Math.max(containerWidth, requiredWidth * scale)
  const scaledHeight = Math.max(containerHeight, requiredHeight * scale)

  // Calculate offset to center the content
  const offsetX = (scaledWidth - bounds.width * scale) / 2 - bounds.minX * scale
  const offsetY =
    (scaledHeight - bounds.height * scale) / 2 - bounds.minY * scale

  return {
    width: scaledWidth,
    height: scaledHeight,
    offsetX,
    offsetY,
    scale,
  }
}

/**
 * Generate auto-layout positions for nodes to prevent overlap
 */
export function generateAutoLayout(
  nodes: Array<OrigamiNodeData>,
  containerWidth: number,
  containerHeight: number,
  padding: number = 50,
): Array<OrigamiNodeData> {
  if (nodes.length === 0) return nodes

  // Group nodes by level for hierarchical layout
  const nodesByLevel = new Map<number, Array<OrigamiNodeData>>()
  nodes.forEach((node) => {
    const level = node.level || 0
    if (!nodesByLevel.has(level)) {
      nodesByLevel.set(level, [])
    }
    nodesByLevel.get(level)!.push(node)
  })

  const levels = Array.from(nodesByLevel.keys()).sort((a, b) => a - b)
  const levelHeight =
    (containerHeight - padding * 2) / Math.max(1, levels.length - 1)

  const updatedNodes: Array<OrigamiNodeData> = []

  levels.forEach((level, levelIndex) => {
    const levelNodes = nodesByLevel.get(level)!
    const nodeCount = levelNodes.length

    if (nodeCount === 1) {
      // Single node - center it
      updatedNodes.push({
        ...levelNodes[0],
        x: containerWidth / 2,
        y: padding + levelIndex * levelHeight,
      })
    } else {
      // Multiple nodes - distribute horizontally
      const nodeWidth =
        (containerWidth - padding * 2) / Math.max(1, nodeCount - 1)

      levelNodes.forEach((node, nodeIndex) => {
        updatedNodes.push({
          ...node,
          x: padding + nodeIndex * nodeWidth,
          y: padding + levelIndex * levelHeight,
        })
      })
    }
  })

  return updatedNodes
}

/**
 * Calculate transform needed to fit all nodes in view
 */
export function calculateFitToViewTransform(
  nodes: Array<OrigamiNodeData>,
  containerWidth: number,
  containerHeight: number,
  padding: number = 100,
): { x: number; y: number; scale: number } {
  const bounds = calculateNodeBounds(nodes)

  if (bounds.width === 0 || bounds.height === 0) {
    return { x: 0, y: 0, scale: 1 }
  }

  // Cap padding so it can never consume the whole container
  const padX = Math.min(padding, containerWidth / 4)
  const padY = Math.min(padding, containerHeight / 4)
  const availableWidth = containerWidth - padX * 2
  const availableHeight = containerHeight - padY * 2

  // Calculate scale to fit with padding, but don't scale up beyond 100%
  const scaleX = availableWidth / bounds.width
  const scaleY = availableHeight / bounds.height
  const scale = Math.min(scaleX, scaleY, 1) // Don't scale up beyond 100%

  // Calculate translation to center the scaled content
  const scaledWidth = bounds.width * scale
  const scaledHeight = bounds.height * scale

  // Center the content in the available space
  const x = (containerWidth - scaledWidth) / 2 - bounds.minX * scale
  const y = (containerHeight - scaledHeight) / 2 - bounds.minY * scale

  return { x, y, scale }
}

/**
 * Check if any nodes are clipped and return adjustment recommendations
 */
export function checkForClipping(
  nodes: Array<OrigamiNodeData>,
  containerWidth: number,
  containerHeight: number,
  currentTransform: { x: number; y: number; scale: number },
): {
  isClipped: boolean
  clippedNodes: Array<string>
  recommendedTransform: { x: number; y: number; scale: number } | null
} {
  const clippedNodes: Array<string> = []
  const { x: transformX, y: transformY, scale } = currentTransform

  nodes.forEach((node) => {
    const nodeSize = sizeConfig[node.size]
    const halfWidth = nodeSize.width / 2
    const halfHeight = nodeSize.height / 2

    // Apply current transform to node position
    const screenX = node.x * scale + transformX
    const screenY = node.y * scale + transformY

    // Check if node is clipped
    const left = screenX - halfWidth * scale
    const right = screenX + halfWidth * scale
    const top = screenY - halfHeight * scale
    const bottom = screenY + halfHeight * scale

    if (
      left < 0 ||
      right > containerWidth ||
      top < 0 ||
      bottom > containerHeight
    ) {
      clippedNodes.push(node.id)
    }
  })

  const isClipped = clippedNodes.length > 0
  let recommendedTransform = null

  if (isClipped) {
    // Calculate a better transform to fit all nodes
    recommendedTransform = calculateFitToViewTransform(
      nodes,
      containerWidth,
      containerHeight,
    )
  }

  return {
    isClipped,
    clippedNodes,
    recommendedTransform,
  }
}

export default {
  calculateNodeBounds,
  calculateOptimalCanvas,
  generateAutoLayout,
  calculateFitToViewTransform,
  checkForClipping,
}
