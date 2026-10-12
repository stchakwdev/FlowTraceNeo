/**
 * Origami Pattern Generator
 * Creates dynamic SVG polygon patterns for different node sizes
 * Each size has 3 unique patterns: Geometric, Organic, and Crystalline
 */

export type NodeSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl'
export type PatternType = 'geometric' | 'organic' | 'crystalline'

interface PolygonFace {
  points: string
  opacity: number
  fillType: 'primary' | 'secondary' | 'shadow'
  index: number
}

export interface OrigamiPattern {
  faces: Array<PolygonFace>
  viewBox: string
  complexity: number
}

// Pattern definitions for each size
const patterns: Record<NodeSize, Record<PatternType, OrigamiPattern>> = {
  xl: {
    geometric: {
      viewBox: '0 0 200 200',
      complexity: 7,
      faces: [
        {
          points: '50,10 150,10 190,90 150,190 50,190 10,90',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '50,10 100,10 100,100 50,100',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '100,10 150,10 150,100 100,100',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '150,10 190,90 150,100 100,100',
          opacity: 0.8,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '100,100 150,100 150,190 100,190',
          opacity: 0.85,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '50,100 100,100 100,190 50,190',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
        {
          points: '10,90 50,100 50,190 10,90',
          opacity: 0.75,
          fillType: 'secondary',
          index: 6,
        },
      ],
    },
    organic: {
      viewBox: '0 0 200 200',
      complexity: 6,
      faces: [
        {
          points:
            '100,5 130,70 195,70 150,110 170,175 100,135 30,175 50,110 5,70 70,70',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '100,5 130,70 100,90 70,70',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '130,70 195,70 150,110 100,90',
          opacity: 0.85,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '100,90 150,110 170,175 100,135',
          opacity: 0.9,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '100,90 100,135 30,175 50,110',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
        {
          points: '70,70 100,90 50,110 5,70',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
      ],
    },
    crystalline: {
      viewBox: '0 0 200 200',
      complexity: 7,
      faces: [
        {
          points: '100,10 160,50 160,130 100,190 40,130 40,50',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '100,10 130,30 130,70 100,90 70,70 70,30',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '130,30 160,50 160,90 130,70',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '130,70 160,90 160,130 130,150 100,130 100,90',
          opacity: 0.85,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '70,30 70,70 40,90 40,50',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
        {
          points: '70,70 100,90 100,130 70,150 40,130 40,90',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
        {
          points: '100,130 130,150 100,190 70,150',
          opacity: 0.75,
          fillType: 'secondary',
          index: 6,
        },
      ],
    },
  },
  lg: {
    geometric: {
      viewBox: '0 0 160 160',
      complexity: 5,
      faces: [
        {
          points: '30,10 130,10 150,80 80,150 10,80',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '30,10 80,10 80,80 30,80',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '80,10 130,10 130,80 80,80',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '130,10 150,80 80,80',
          opacity: 0.8,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '30,80 80,80 80,150 30,150',
          opacity: 0.85,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '10,80 30,80 30,150 10,80',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
    organic: {
      viewBox: '0 0 160 160',
      complexity: 5,
      faces: [
        {
          points: '80,20 140,40 140,100 100,140 40,120 20,60',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '80,20 80,80 40,60 20,60',
          opacity: 0.9,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '80,20 140,40 140,80 80,80',
          opacity: 0.85,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '80,80 140,80 140,100 100,140',
          opacity: 0.8,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '80,80 100,140 40,120 40,80',
          opacity: 0.95,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '40,60 40,80 80,80',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
    crystalline: {
      viewBox: '0 0 160 160',
      complexity: 5,
      faces: [
        {
          points: '80,10 120,30 140,80 80,150 20,80 40,30',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '80,10 120,30 80,60 40,30',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '120,30 140,80 80,60',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '80,60 140,80 80,150',
          opacity: 0.85,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '80,60 80,150 20,80',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
        {
          points: '40,30 80,60 20,80',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
      ],
    },
  },
  md: {
    geometric: {
      viewBox: '0 0 130 130',
      complexity: 4,
      faces: [
        {
          points: '25,10 105,10 120,65 65,120 10,65',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '25,10 65,10 65,65 25,65',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '65,10 105,10 105,65 65,65',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '105,10 120,65 65,65',
          opacity: 0.8,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '25,65 65,65 65,120 25,120',
          opacity: 0.85,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '10,65 25,65 25,120 10,65',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
    organic: {
      viewBox: '0 0 130 130',
      complexity: 4,
      faces: [
        {
          points: '20,20 90,10 110,60 90,110 30,100 10,50',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '20,20 60,15 60,60 20,60',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '60,15 90,10 90,60 60,60',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '90,10 110,60 90,60',
          opacity: 0.85,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '60,60 90,60 90,110 60,100',
          opacity: 0.8,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '20,60 60,60 60,100 30,100',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
        {
          points: '10,50 20,60 20,20',
          opacity: 0.75,
          fillType: 'secondary',
          index: 6,
        },
      ],
    },
    crystalline: {
      viewBox: '0 0 130 130',
      complexity: 4,
      faces: [
        {
          points: '65,10 110,40 65,120 20,40',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '65,10 90,25 65,50 40,25',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '90,25 110,40 65,50',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '65,50 110,40 65,120',
          opacity: 0.85,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '65,50 65,120 20,40',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
        {
          points: '40,25 65,50 20,40',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
      ],
    },
  },
  sm: {
    geometric: {
      viewBox: '0 0 100 100',
      complexity: 3,
      faces: [
        {
          points: '20,10 80,10 90,50 50,90 10,50',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '20,10 50,10 50,50 20,50',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '50,10 80,10 80,50 50,50',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '80,10 90,50 50,50',
          opacity: 0.85,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '20,50 50,50 50,90 20,90',
          opacity: 0.8,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '10,50 20,50 20,90 10,50',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
    organic: {
      viewBox: '0 0 100 100',
      complexity: 3,
      faces: [
        {
          points: '50,10 80,30 80,60 50,90 20,60 20,30',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '50,10 65,20 50,40 35,20',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '65,20 80,30 80,60 65,50 50,40',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '50,40 65,50 50,90',
          opacity: 0.85,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '35,20 50,40 50,90 35,50 20,60 20,30',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
      ],
    },
    crystalline: {
      viewBox: '0 0 100 100',
      complexity: 3,
      faces: [
        {
          points: '50,20 70,10 85,40 70,70 50,80 30,70 15,40 30,10',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '50,20 70,10 70,40 50,50',
          opacity: 0.9,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '70,40 85,40 70,70 50,50',
          opacity: 0.85,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '50,50 70,70 50,80',
          opacity: 0.8,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '50,20 50,50 30,40 30,10',
          opacity: 0.95,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '30,40 50,50 50,80 30,70 15,40',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
  },
  xs: {
    geometric: {
      viewBox: '0 0 80 80',
      complexity: 3,
      faces: [
        {
          points: '15,10 65,10 70,40 40,70 10,40',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '15,10 40,10 40,40 15,40',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '40,10 65,10 65,40 40,40',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '65,10 70,40 40,40',
          opacity: 0.85,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '15,40 40,40 40,70 15,70',
          opacity: 0.8,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '10,40 15,40 15,70 10,40',
          opacity: 0.75,
          fillType: 'secondary',
          index: 5,
        },
      ],
    },
    organic: {
      viewBox: '0 0 80 80',
      complexity: 3,
      faces: [
        {
          points: '10,15 50,10 70,30 60,60 20,65 5,35',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '10,15 30,13 30,35 10,35',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '30,13 50,10 50,35 30,35',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '50,10 70,30 50,35',
          opacity: 0.85,
          fillType: 'secondary',
          index: 3,
        },
        {
          points: '30,35 50,35 60,60 30,55',
          opacity: 0.8,
          fillType: 'primary',
          index: 4,
        },
        {
          points: '10,35 30,35 30,55 20,65 5,35',
          opacity: 0.95,
          fillType: 'primary',
          index: 5,
        },
        {
          points: '30,55 60,60 50,35',
          opacity: 0.75,
          fillType: 'secondary',
          index: 6,
        },
      ],
    },
    crystalline: {
      viewBox: '0 0 80 80',
      complexity: 3,
      faces: [
        {
          points: '40,10 60,20 60,50 40,70 20,50 20,20',
          opacity: 0.3,
          fillType: 'shadow',
          index: 0,
        },
        {
          points: '40,10 50,15 40,30 30,15',
          opacity: 1,
          fillType: 'primary',
          index: 1,
        },
        {
          points: '50,15 60,20 60,50 50,45 40,30',
          opacity: 0.9,
          fillType: 'secondary',
          index: 2,
        },
        {
          points: '40,30 50,45 40,70',
          opacity: 0.85,
          fillType: 'primary',
          index: 3,
        },
        {
          points: '30,15 40,30 40,70 30,45 20,50 20,20',
          opacity: 0.8,
          fillType: 'secondary',
          index: 4,
        },
      ],
    },
  },
}

/**
 * Get a random origami pattern for a given size
 */
export function getRandomPattern(size: NodeSize): OrigamiPattern {
  const sizePatterns = patterns[size]
  const patternTypes: Array<PatternType> = [
    'geometric',
    'organic',
    'crystalline',
  ]
  const randomType =
    patternTypes[Math.floor(Math.random() * patternTypes.length)]
  return sizePatterns[randomType]
}

/**
 * Get a specific pattern by size and type
 */
export function getPattern(size: NodeSize, type: PatternType): OrigamiPattern {
  return patterns[size][type]
}

/**
 * Generate gradient definitions for SVG
 */
export function generateGradientDefs(): string {
  return `
    <defs>
      <!-- Primary gradient for main faces -->
      <linearGradient id="origami-primary" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#1a2332;stop-opacity:1" />
        <stop offset="50%" style="stop-color:#243142;stop-opacity:1" />
        <stop offset="100%" style="stop-color:#1a2332;stop-opacity:1" />
      </linearGradient>
      
      <!-- Secondary gradient for side faces -->
      <linearGradient id="origami-secondary" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#1e2938;stop-opacity:1" />
        <stop offset="50%" style="stop-color:#2a3848;stop-opacity:1" />
        <stop offset="100%" style="stop-color:#1e2938;stop-opacity:1" />
      </linearGradient>
      
      <!-- Shadow gradient for depth -->
      <linearGradient id="origami-shadow" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#000000;stop-opacity:0.3" />
        <stop offset="100%" style="stop-color:#000000;stop-opacity:0.1" />
      </linearGradient>
      
      <!-- Highlight gradient for active/hovered state -->
      <linearGradient id="origami-highlight" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#00D4AA;stop-opacity:0.3" />
        <stop offset="50%" style="stop-color:#00D4AA;stop-opacity:0.4" />
        <stop offset="100%" style="stop-color:#0066FF;stop-opacity:0.3" />
      </linearGradient>
    </defs>
  `
}

/**
 * Calculate pattern index based on node data for consistency
 */
export function calculatePatternIndex(
  nodeId: string,
  nodeType: string,
): PatternType {
  // Use node characteristics to determine pattern type
  const hash = nodeId
    .split('')
    .reduce((acc, char) => acc + char.charCodeAt(0), 0)
  const index = hash % 3

  // You can also use node type to influence pattern selection
  if (nodeType === 'headquarters' || nodeType === 'hub') {
    return 'crystalline'
  }

  const types: Array<PatternType> = ['geometric', 'organic', 'crystalline']
  return types[index]
}
