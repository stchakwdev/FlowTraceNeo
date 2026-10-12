import React, { useCallback, useEffect, useRef, useState } from 'react'
import {
  ArrowCounterClockwise,
  ArrowsOutCardinal,
  MagnifyingGlassMinus,
  MagnifyingGlassPlus,
} from '@phosphor-icons/react'

interface PanZoomState {
  x: number
  y: number
  scale: number
}

interface PanZoomProps {
  children: React.ReactNode
  minZoom?: number
  maxZoom?: number
  zoomStep?: number
  initialZoom?: number
  initialX?: number
  initialY?: number
  width: number
  height: number
  onTransformChange?: (transform: PanZoomState) => void
  onFitToView?: () => { x: number; y: number; scale: number } | null
}

export function PanZoomContainer({
  children,
  minZoom = 0.1,
  maxZoom = 3,
  zoomStep = 0.1,
  initialZoom = 1,
  initialX = 0,
  initialY = 0,
  width,
  height,
  onTransformChange,
  onFitToView,
}: PanZoomProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const contentRef = useRef<HTMLDivElement>(null)
  const [transform, setTransform] = useState<PanZoomState>({
    x: initialX,
    y: initialY,
    scale: initialZoom,
  })
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [lastPan, setLastPan] = useState({ x: 0, y: 0 })

  // Update external transform state
  useEffect(() => {
    if (onTransformChange) {
      onTransformChange(transform)
    }
  }, [transform, onTransformChange])

  // Handle wheel zoom
  const handleWheel = useCallback(
    (e: WheelEvent) => {
      e.preventDefault()

      if (!containerRef.current) return

      const rect = containerRef.current.getBoundingClientRect()
      const mouseX = e.clientX - rect.left
      const mouseY = e.clientY - rect.top

      // Calculate zoom
      const zoomFactor = e.deltaY > 0 ? 1 - zoomStep : 1 + zoomStep
      const newScale = Math.max(
        minZoom,
        Math.min(maxZoom, transform.scale * zoomFactor),
      )

      if (newScale === transform.scale) return

      // Zoom towards mouse position
      const scaleRatio = newScale / transform.scale
      const newX = mouseX - (mouseX - transform.x) * scaleRatio
      const newY = mouseY - (mouseY - transform.y) * scaleRatio

      setTransform({
        x: newX,
        y: newY,
        scale: newScale,
      })
    },
    [transform, minZoom, maxZoom, zoomStep],
  )

  // Handle mouse down for dragging
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return // Only left mouse button

      setIsDragging(true)
      setDragStart({ x: e.clientX, y: e.clientY })
      setLastPan({ x: transform.x, y: transform.y })

      // Prevent text selection
      e.preventDefault()
    },
    [transform.x, transform.y],
  )

  // Handle mouse move for dragging
  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging) return

      const deltaX = e.clientX - dragStart.x
      const deltaY = e.clientY - dragStart.y

      setTransform((prev) => ({
        ...prev,
        x: lastPan.x + deltaX,
        y: lastPan.y + deltaY,
      }))
    },
    [isDragging, dragStart, lastPan],
  )

  // Handle mouse up
  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  // Touch event handlers for mobile support
  const handleTouchStart = useCallback(
    (e: React.TouchEvent) => {
      if (e.touches.length === 1) {
        // Single touch - start panning
        const touch = e.touches[0]
        setIsDragging(true)
        setDragStart({ x: touch.clientX, y: touch.clientY })
        setLastPan({ x: transform.x, y: transform.y })
      }
      e.preventDefault()
    },
    [transform.x, transform.y],
  )

  const handleTouchMove = useCallback(
    (e: TouchEvent) => {
      if (e.touches.length === 1 && isDragging) {
        // Single touch - continue panning
        const touch = e.touches[0]
        const deltaX = touch.clientX - dragStart.x
        const deltaY = touch.clientY - dragStart.y

        setTransform((prev) => ({
          ...prev,
          x: lastPan.x + deltaX,
          y: lastPan.y + deltaY,
        }))
      }
      e.preventDefault()
    },
    [isDragging, dragStart, lastPan],
  )

  const handleTouchEnd = useCallback(() => {
    setIsDragging(false)
  }, [])

  // Add event listeners
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    container.addEventListener('wheel', handleWheel, { passive: false })
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
    document.addEventListener('touchmove', handleTouchMove, { passive: false })
    document.addEventListener('touchend', handleTouchEnd)

    return () => {
      container.removeEventListener('wheel', handleWheel)
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.removeEventListener('touchmove', handleTouchMove)
      document.removeEventListener('touchend', handleTouchEnd)
    }
  }, [
    handleWheel,
    handleMouseMove,
    handleMouseUp,
    handleTouchMove,
    handleTouchEnd,
  ])

  // Zoom control functions
  const zoomIn = useCallback(() => {
    const newScale = Math.min(maxZoom, transform.scale + zoomStep)
    const centerX = width / 2
    const centerY = height / 2

    const scaleRatio = newScale / transform.scale
    const newX = centerX - (centerX - transform.x) * scaleRatio
    const newY = centerY - (centerY - transform.y) * scaleRatio

    setTransform({
      x: newX,
      y: newY,
      scale: newScale,
    })
  }, [transform, maxZoom, zoomStep, width, height])

  const zoomOut = useCallback(() => {
    const newScale = Math.max(minZoom, transform.scale - zoomStep)
    const centerX = width / 2
    const centerY = height / 2

    const scaleRatio = newScale / transform.scale
    const newX = centerX - (centerX - transform.x) * scaleRatio
    const newY = centerY - (centerY - transform.y) * scaleRatio

    setTransform({
      x: newX,
      y: newY,
      scale: newScale,
    })
  }, [transform, minZoom, zoomStep, width, height])

  const resetView = useCallback(() => {
    setTransform({
      x: 0,
      y: 0,
      scale: initialZoom,
    })
  }, [initialZoom])

  const fitToView = useCallback(() => {
    if (onFitToView) {
      const fitTransform = onFitToView()
      if (fitTransform) {
        setTransform({
          x: fitTransform.x,
          y: fitTransform.y,
          scale: fitTransform.scale,
        })
        return
      }
    }

    // Fallback: Reset to show all content centered
    setTransform({
      x: (width - width * initialZoom) / 2,
      y: (height - height * initialZoom) / 2,
      scale: initialZoom,
    })
  }, [width, height, initialZoom, onFitToView])

  return (
    <div
      ref={containerRef}
      style={{
        width,
        height,
        position: 'relative',
        overflow: 'hidden',
        cursor: isDragging ? 'grabbing' : 'grab',
        touchAction: 'none',
        background: 'transparent',
      }}
      onMouseDown={handleMouseDown}
      onTouchStart={handleTouchStart}
    >
      {/* Transformed content */}
      <div
        ref={contentRef}
        style={{
          transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.scale})`,
          transformOrigin: '0 0',
          transition: isDragging ? 'none' : 'transform 0.1s ease-out',
          width: '100%',
          height: '100%',
        }}
      >
        {children}
      </div>

      {/* Zoom Controls */}
      <div
        style={{
          position: 'absolute',
          top: 20,
          right: 20,
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          background: 'rgba(0, 0, 0, 0.7)',
          backdropFilter: 'blur(10px)',
          borderRadius: 8,
          padding: 8,
          zIndex: 1000,
        }}
      >
        <button
          onClick={zoomIn}
          disabled={transform.scale >= maxZoom}
          style={{
            width: 40,
            height: 40,
            borderRadius: 6,
            border: 'none',
            background:
              transform.scale >= maxZoom
                ? 'rgba(255, 255, 255, 0.1)'
                : 'rgba(0, 102, 255, 0.8)',
            color: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: transform.scale >= maxZoom ? 'not-allowed' : 'pointer',
            transition: 'all 0.2s ease',
          }}
          title="Zoom In"
        >
          <MagnifyingGlassPlus size={20} />
        </button>

        <button
          onClick={zoomOut}
          disabled={transform.scale <= minZoom}
          style={{
            width: 40,
            height: 40,
            borderRadius: 6,
            border: 'none',
            background:
              transform.scale <= minZoom
                ? 'rgba(255, 255, 255, 0.1)'
                : 'rgba(0, 102, 255, 0.8)',
            color: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: transform.scale <= minZoom ? 'not-allowed' : 'pointer',
            transition: 'all 0.2s ease',
          }}
          title="Zoom Out"
        >
          <MagnifyingGlassMinus size={20} />
        </button>

        <button
          onClick={fitToView}
          style={{
            width: 40,
            height: 40,
            borderRadius: 6,
            border: 'none',
            background: 'rgba(0, 102, 255, 0.8)',
            color: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'all 0.2s ease',
          }}
          title="Fit to View"
        >
          <ArrowsOutCardinal size={20} />
        </button>

        <button
          onClick={resetView}
          style={{
            width: 40,
            height: 40,
            borderRadius: 6,
            border: 'none',
            background: 'rgba(0, 102, 255, 0.8)',
            color: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'all 0.2s ease',
          }}
          title="Reset View"
        >
          <ArrowCounterClockwise size={20} />
        </button>
      </div>

      {/* Zoom Level Indicator */}
      <div
        style={{
          position: 'absolute',
          bottom: 20,
          right: 20,
          background: 'rgba(0, 0, 0, 0.7)',
          backdropFilter: 'blur(10px)',
          padding: '8px 12px',
          borderRadius: 6,
          color: '#fff',
          fontSize: 14,
          fontFamily: 'JetBrains Mono, monospace',
          zIndex: 1000,
        }}
      >
        {Math.round(transform.scale * 100)}%
      </div>

      {/* Pan Instructions */}
      {!isDragging &&
        transform.scale === initialZoom &&
        transform.x === 0 &&
        transform.y === 0 && (
          <div
            style={{
              position: 'absolute',
              bottom: 60,
              left: 20,
              background: 'rgba(0, 0, 0, 0.6)',
              backdropFilter: 'blur(10px)',
              padding: '8px 12px',
              borderRadius: 6,
              color: 'rgba(255, 255, 255, 0.8)',
              fontSize: 12,
              zIndex: 1000,
              maxWidth: 200,
            }}
          >
            💡 Drag to pan • Scroll to zoom
          </div>
        )}
    </div>
  )
}

export default PanZoomContainer
