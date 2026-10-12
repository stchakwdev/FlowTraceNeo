import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { OrigamiControls, OrigamiVisualization } from './visualization'
import { PanZoomContainer } from './pan-zoom'
import { calculateFitToViewTransform, calculateNodeBounds } from './bounds'
import { toOrigamiNodes } from './adapter'
import type { FlowDataResult } from '@/lib/supabase'

const FIT_PADDING = 40
// Keep fitted content clear of the fold/unfold control bar
const CONTROLS_HEIGHT = 90
// Below this zoom node labels are too small to read, so the initial view
// starts here and anchors on the top of the tree instead of fitting it all
const MIN_READABLE_SCALE = 0.7

interface OrigamiFlowProps {
  data: FlowDataResult
  height?: number
  performanceMode?: boolean
  minZoom?: number
  maxZoom?: number
  onNodeClick?: (nodeId: string) => void
  onNodeHover?: (nodeId: string | null) => void
}

export function OrigamiFlow({
  data,
  height = 600,
  performanceMode = false,
  minZoom = 0.2,
  maxZoom = 3,
  onNodeClick,
  onNodeHover,
}: OrigamiFlowProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [viewportWidth, setViewportWidth] = useState(1000)
  const [animationsEnabled, setAnimationsEnabled] = useState(!performanceMode)
  const [expandAllTrigger, setExpandAllTrigger] = useState(0)
  const [collapseAllTrigger, setCollapseAllTrigger] = useState(0)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const observer = new ResizeObserver(([entry]) => {
      setViewportWidth(Math.max(320, Math.floor(entry.contentRect.width)))
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  const layout = useMemo(() => toOrigamiNodes(data), [data])

  const handleFitToView = useCallback(() => {
    if (layout.nodes.length === 0) return null
    return calculateFitToViewTransform(
      layout.nodes,
      viewportWidth,
      height - CONTROLS_HEIGHT,
      FIT_PADDING,
    )
  }, [layout.nodes, viewportWidth, height])

  const initialTransform = useMemo(() => {
    const fit = handleFitToView()
    if (!fit || fit.scale >= MIN_READABLE_SCALE) return fit
    const bounds = calculateNodeBounds(layout.nodes)
    const scale = MIN_READABLE_SCALE
    return {
      scale,
      x: viewportWidth / 2 - bounds.centerX * scale,
      y: FIT_PADDING - bounds.minY * scale,
    }
  }, [handleFitToView, layout.nodes, viewportWidth])

  return (
    <div
      ref={containerRef}
      className="origami-theme relative w-full"
      style={{ height }}
    >
      <PanZoomContainer
        // Remount when the layout changes so the initial fit is re-applied
        key={`${layout.nodes.length}-${viewportWidth}`}
        width={viewportWidth}
        height={height}
        minZoom={minZoom}
        maxZoom={maxZoom}
        zoomStep={0.2}
        initialZoom={initialTransform?.scale ?? 1}
        initialX={initialTransform?.x ?? 0}
        initialY={initialTransform?.y ?? 0}
        onFitToView={handleFitToView}
      >
        <OrigamiVisualization
          nodes={layout.nodes}
          width={layout.width}
          height={layout.height}
          performanceMode={performanceMode}
          enableAnimations={animationsEnabled}
          onNodeClick={onNodeClick}
          onNodeHover={onNodeHover}
          expandAllTrigger={expandAllTrigger}
          collapseAllTrigger={collapseAllTrigger}
        />
      </PanZoomContainer>

      <OrigamiControls
        onExpandAll={() => setExpandAllTrigger(Date.now())}
        onCollapseAll={() => setCollapseAllTrigger(Date.now())}
        onToggleAnimations={() => setAnimationsEnabled((on) => !on)}
        animationsEnabled={animationsEnabled}
      />
    </div>
  )
}
