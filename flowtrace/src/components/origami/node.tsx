import React, { useEffect, useMemo, useRef, useState } from 'react'
import { calculatePatternIndex, getPattern } from './patterns'
import { calculateFloatingIntensity, getRandomDelay } from './animations'
import type { NodeSize, OrigamiPattern } from './patterns'
import type { AnimationState, NodeActivityMetrics } from './animations'

export interface OrigamiNodeData {
  id: string
  name: string
  type: string
  balance: number
  currency: string
  transactions: number
  size: NodeSize
  x: number
  y: number
  icon?: string
  level: number
  parent?: string
  children?: Array<string>
  verified?: boolean
  trustScore?: number
}

interface OrigamiNodeProps {
  data: OrigamiNodeData
  isHighlighted?: boolean
  isExpanded?: boolean
  isVisible?: boolean
  onHover?: (nodeId: string, event: 'enter' | 'leave') => void
  onClick?: (nodeId: string) => void
  onExpand?: (nodeId: string) => void
}

const sizeConfig = {
  xl: {
    width: 200,
    height: 200,
    fontSize: { value: 30, title: 17, transactions: 13 },
    iconSize: 36,
  },
  lg: {
    width: 160,
    height: 160,
    fontSize: { value: 26, title: 16, transactions: 12 },
    iconSize: 32,
  },
  md: {
    width: 130,
    height: 130,
    fontSize: { value: 22, title: 15, transactions: 12 },
    iconSize: 28,
  },
  sm: {
    width: 100,
    height: 100,
    fontSize: { value: 19, title: 14, transactions: 11 },
    iconSize: 24,
  },
  xs: {
    width: 80,
    height: 80,
    fontSize: { value: 17, title: 13, transactions: 10 },
    iconSize: 20,
  },
}

export function OrigamiNode({
  data,
  isHighlighted = false,
  isExpanded = false,
  isVisible = true,
  onHover,
  onClick,
  onExpand,
}: OrigamiNodeProps) {
  const nodeRef = useRef<HTMLDivElement>(null)
  const [animationState, setAnimationState] = useState<AnimationState>('idle')
  const [isLoaded, setIsLoaded] = useState(false)

  // Calculate metrics for animation intensity
  const activityMetrics: NodeActivityMetrics = useMemo(
    () => ({
      transactionCount: data.transactions,
      monetaryValue: data.balance,
      frequency:
        data.transactions > 100
          ? 'high'
          : data.transactions > 50
            ? 'medium'
            : 'low',
    }),
    [data.transactions, data.balance],
  )

  // Get floating intensity and pattern
  const floatIntensity = useMemo(
    () => calculateFloatingIntensity(activityMetrics),
    [activityMetrics],
  )

  const pattern = useMemo(() => {
    const patternType = calculatePatternIndex(data.id, data.type)
    return getPattern(data.size, patternType)
  }, [data.id, data.type, data.size])

  const size = sizeConfig[data.size]
  const animationDelay = useMemo(() => getRandomDelay(), [])

  // Format currency value
  const formatValue = (value: number): string => {
    if (value >= 1000000) return `${(value / 1000000).toFixed(1)}M`
    if (value >= 1000) return `${(value / 1000).toFixed(0)}K`
    return value.toString()
  }

  // Handle mouse events
  const handleMouseEnter = () => {
    if (onHover) onHover(data.id, 'enter')
    setAnimationState('hovering')
  }

  const handleMouseLeave = () => {
    if (onHover) onHover(data.id, 'leave')
    setAnimationState('idle')
  }

  const handleClick = () => {
    setAnimationState('rustling')
    setTimeout(() => setAnimationState('idle'), 400)
    if (onClick) onClick(data.id)
  }

  const handleExpandClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    if (onExpand) onExpand(data.id)
  }

  // Update animation state based on props
  useEffect(() => {
    if (isHighlighted) {
      setAnimationState('highlighted')
    } else if (isExpanded) {
      setAnimationState('expanded')
    } else {
      setAnimationState('idle')
    }
  }, [isHighlighted, isExpanded])

  // Load animation on mount
  useEffect(() => {
    const timer = setTimeout(() => setIsLoaded(true), 100)
    return () => clearTimeout(timer)
  }, [])

  // Improve reactivity by re-calculating patterns when data changes
  useEffect(() => {
    if (
      data.transactions !== activityMetrics.transactionCount ||
      data.balance !== activityMetrics.monetaryValue
    ) {
      // Data has changed, trigger re-render
      setAnimationState('idle')
    }
  }, [data.transactions, data.balance, activityMetrics])

  // Generate CSS classes
  const getNodeClasses = () => {
    const classes = [
      'origami-node',
      `origami-size-${data.size}`,
      `origami-float-${floatIntensity}`,
      'origami-transition-all',
    ]

    if (animationState === 'rustling') classes.push('origami-rustling')
    if (animationState === 'expanded') classes.push('origami-expanded')
    if (animationState === 'highlighted') classes.push('origami-highlighted')
    if (!isVisible) classes.push('origami-hidden')
    if (isLoaded) classes.push('origami-loaded')

    return classes.join(' ')
  }

  // Get dynamic style with animation delay
  const getNodeStyle = () => ({
    position: 'absolute' as const,
    left: data.x - size.width / 2,
    top: data.y - size.height / 2,
    width: size.width,
    height: size.height,
    cursor: 'pointer',
    animationDelay: `${animationDelay}ms`,
    filter: isHighlighted
      ? 'drop-shadow(0 0 30px color-mix(in oklch, var(--success) 80%, transparent))'
      : 'drop-shadow(0 10px 30px color-mix(in oklch, var(--primary) 30%, transparent))',
    zIndex: isHighlighted ? 100 : 10 + data.level,
    transition: 'filter 0.3s ease, z-index 0s',
  })

  // Generate SVG faces from pattern
  const renderPatternFaces = (pattern: OrigamiPattern) => {
    return pattern.faces.map((face, index) => {
      const fillUrl =
        face.fillType === 'shadow'
          ? 'url(#origami-shadow)'
          : face.fillType === 'secondary'
            ? 'url(#origami-secondary)'
            : 'url(#origami-primary)'

      return (
        <polygon
          key={index}
          className="origami-fold-face"
          points={face.points}
          fill={isHighlighted ? 'url(#origami-highlight)' : fillUrl}
          fillOpacity={face.opacity}
          strokeWidth="0.5"
          style={{
            stroke: 'color-mix(in oklch, var(--foreground) 12%, transparent)',
            transition: 'all 0.3s ease',
            transform: `translateZ(${face.index * 2}px)`,
          }}
        />
      )
    })
  }

  // Paper texture component
  const PaperTexture = () => (
    <div
      className="origami-paper-texture"
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        opacity: 0.15,
        background: `
        repeating-linear-gradient(
          45deg,
          transparent,
          transparent 2px,
          color-mix(in oklch, var(--foreground) 3%, transparent) 2px,
          color-mix(in oklch, var(--foreground) 3%, transparent) 4px
        ),
        repeating-linear-gradient(
          -45deg,
          transparent,
          transparent 2px,
          color-mix(in oklch, var(--foreground) 2%, transparent) 2px,
          color-mix(in oklch, var(--foreground) 2%, transparent) 4px
        )
      `,
        pointerEvents: 'none',
      }}
    />
  )

  return (
    <div
      ref={nodeRef}
      className={getNodeClasses()}
      style={getNodeStyle()}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onClick={handleClick}
    >
      {/* SVG Origami Shape */}
      <svg
        className="origami-shape"
        viewBox={pattern.viewBox}
        preserveAspectRatio="xMidYMid meet"
        style={{
          width: '100%',
          height: '100%',
          transformStyle: 'preserve-3d',
          transition: 'all 0.6s cubic-bezier(0.4, 0, 0.2, 1)',
        }}
      >
        {/* Gradient definitions */}
        <defs>
          <linearGradient
            id="origami-primary"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop
              offset="0%"
              style={{ stopColor: 'var(--ori-face)' }}
              stopOpacity="1"
            />
            <stop
              offset="50%"
              style={{ stopColor: 'var(--ori-face-mid)' }}
              stopOpacity="1"
            />
            <stop
              offset="100%"
              style={{ stopColor: 'var(--ori-face)' }}
              stopOpacity="1"
            />
          </linearGradient>

          <linearGradient
            id="origami-secondary"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop
              offset="0%"
              style={{ stopColor: 'var(--ori-face-2)' }}
              stopOpacity="1"
            />
            <stop
              offset="50%"
              style={{ stopColor: 'var(--ori-face-2-mid)' }}
              stopOpacity="1"
            />
            <stop
              offset="100%"
              style={{ stopColor: 'var(--ori-face-2)' }}
              stopOpacity="1"
            />
          </linearGradient>

          <linearGradient
            id="origami-shadow"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop
              offset="0%"
              style={{ stopColor: 'var(--foreground)' }}
              stopOpacity="0.3"
            />
            <stop
              offset="100%"
              style={{ stopColor: 'var(--foreground)' }}
              stopOpacity="0.1"
            />
          </linearGradient>

          <linearGradient
            id="origami-highlight"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop
              offset="0%"
              style={{ stopColor: 'var(--success)' }}
              stopOpacity="0.3"
            />
            <stop
              offset="50%"
              style={{ stopColor: 'var(--success)' }}
              stopOpacity="0.4"
            />
            <stop
              offset="100%"
              style={{ stopColor: 'var(--primary)' }}
              stopOpacity="0.3"
            />
          </linearGradient>
        </defs>

        {/* Render pattern faces */}
        {renderPatternFaces(pattern)}
      </svg>

      {/* Paper Texture Overlay */}
      <PaperTexture />

      {/* Node Content */}
      <div
        className="origami-content"
        style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          textAlign: 'center',
          pointerEvents: 'none',
          zIndex: 10,
          color: 'var(--foreground)',
        }}
      >
        {/* Icon */}
        {data.icon && data.size !== 'xs' && (
          <div
            className="origami-icon"
            style={{
              width: size.iconSize,
              height: size.iconSize,
              background:
                'linear-gradient(135deg, var(--primary), var(--success))',
              borderRadius: 8,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              margin: '0 auto 8px',
              fontSize: size.iconSize * 0.6,
              boxShadow:
                '0 4px 15px color-mix(in oklch, var(--primary) 40%, transparent)',
            }}
          >
            {data.icon}
          </div>
        )}

        {/* Value */}
        <div
          className="origami-value"
          style={{
            fontFamily: "'IBM Plex Mono', monospace",
            fontSize: size.fontSize.value,
            fontWeight: 600,
            whiteSpace: 'nowrap',
            color: 'var(--foreground)',
            marginBottom: 4,
          }}
        >
          {data.currency}
          {formatValue(data.balance)}
        </div>

        {/* Title */}
        <div
          className="origami-title"
          style={{
            fontSize: size.fontSize.title,
            fontWeight: 500,
            color: 'var(--foreground)',
            lineHeight: 1.15,
            maxWidth: size.width * 0.8,
            display: '-webkit-box',
            WebkitLineClamp: 2,
            WebkitBoxOrient: 'vertical',
            overflow: 'hidden',
            marginBottom: data.size === 'xs' ? 0 : 2,
          }}
        >
          {data.name}
        </div>

        {/* Transaction Count */}
        {data.size !== 'xs' && (
          <div
            className="origami-transactions"
            style={{
              fontSize: size.fontSize.transactions,
              color: 'color-mix(in oklch, var(--success) 80%, transparent)',
              fontFamily: "'IBM Plex Mono', monospace",
            }}
          >
            {data.transactions} txns/mo
          </div>
        )}

        {/* Trust Score Indicator */}
        {data.trustScore && data.size !== 'xs' && (
          <div
            className="origami-trust-indicator"
            style={{
              position: 'absolute',
              top: -8,
              right: -8,
              width: 20,
              height: 20,
              borderRadius: '50%',
              background:
                data.trustScore > 80
                  ? 'var(--success)'
                  : data.trustScore > 60
                    ? 'var(--warning)'
                    : 'var(--destructive)',
              fontSize: 10,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontWeight: 600,
              color: 'var(--primary-foreground)',
              boxShadow: '0 2px 8px rgba(0, 0, 0, 0.3)',
            }}
          >
            {data.trustScore}
          </div>
        )}
      </div>

      {/* Expand/Collapse Button */}
      {data.children && data.children.length > 0 && (
        <button
          className="origami-expand-btn"
          onClick={handleExpandClick}
          style={{
            position: 'absolute',
            bottom: -12,
            right: -12,
            width: 24,
            height: 24,
            borderRadius: '50%',
            background: 'color-mix(in oklch, var(--primary) 80%, transparent)',
            border: 'none',
            color: 'var(--primary-foreground)',
            fontSize: 12,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.2s ease',
            pointerEvents: 'all',
          }}
        >
          {isExpanded ? '−' : '+'}
        </button>
      )}
    </div>
  )
}
