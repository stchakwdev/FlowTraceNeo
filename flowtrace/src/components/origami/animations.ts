/**
 * Origami Animation System
 * Manages floating animations, interactions, and transitions
 */

export type FloatIntensity = 'slow' | 'medium' | 'fast' | 'intense'
export type AnimationState =
  | 'idle'
  | 'hovering'
  | 'expanded'
  | 'rustling'
  | 'highlighted'

export interface AnimationConfig {
  duration: number
  amplitude: {
    x: number
    y: number
    rotation: number
  }
  easing: string
  delay?: number
}

export interface NodeActivityMetrics {
  transactionCount: number
  monetaryValue: number
  frequency: 'low' | 'medium' | 'high'
  lastUpdated?: Date
}

// Floating animation configurations
const floatingAnimations: Record<FloatIntensity, AnimationConfig> = {
  slow: {
    duration: 6000,
    amplitude: { x: 1, y: 3, rotation: 0 },
    easing: 'ease-in-out',
  },
  medium: {
    duration: 4000,
    amplitude: { x: 2, y: 5, rotation: 0 },
    easing: 'ease-in-out',
  },
  fast: {
    duration: 3000,
    amplitude: { x: 3, y: 8, rotation: 0 },
    easing: 'ease-in-out',
  },
  intense: {
    duration: 2000,
    amplitude: { x: 4, y: 12, rotation: 1 },
    easing: 'ease-in-out',
  },
}

/**
 * Calculate floating intensity based on financial activity
 * Algorithm weights monetary value (66%) and transaction volume (34%)
 */
export function calculateFloatingIntensity(
  metrics: NodeActivityMetrics,
): FloatIntensity {
  const { transactionCount, monetaryValue } = metrics

  // Normalize values (adjust these thresholds based on your data)
  const normalizedValue = Math.min(monetaryValue / 500000, 1) // Cap at 500k
  const normalizedTransactions = Math.min(transactionCount / 200, 1) // Cap at 200 transactions

  // Calculate weighted score
  const score = normalizedValue * 0.66 + normalizedTransactions * 0.34

  if (score < 0.25) return 'slow'
  if (score < 0.5) return 'medium'
  if (score < 0.75) return 'fast'
  return 'intense'
}

/**
 * Get floating animation configuration
 */
export function getFloatingAnimation(
  intensity: FloatIntensity,
): AnimationConfig {
  return floatingAnimations[intensity]
}

/**
 * Generate CSS keyframes for floating animations
 */
export function generateFloatingKeyframes(): string {
  return `
    @keyframes origami-float-slow {
      0%, 100% { transform: translateY(0px) translateX(0px); }
      33% { transform: translateY(-3px) translateX(1px); }
      66% { transform: translateY(2px) translateX(-1px); }
    }

    @keyframes origami-float-medium {
      0%, 100% { transform: translateY(0px) translateX(0px); }
      25% { transform: translateY(-5px) translateX(2px); }
      50% { transform: translateY(3px) translateX(-1px); }
      75% { transform: translateY(-2px) translateX(-2px); }
    }

    @keyframes origami-float-fast {
      0%, 100% { transform: translateY(0px) translateX(0px); }
      20% { transform: translateY(-8px) translateX(3px); }
      40% { transform: translateY(5px) translateX(-2px); }
      60% { transform: translateY(-4px) translateX(-3px); }
      80% { transform: translateY(6px) translateX(2px); }
    }

    @keyframes origami-float-intense {
      0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
      16% { transform: translateY(-12px) translateX(4px) rotate(1deg); }
      33% { transform: translateY(8px) translateX(-3px) rotate(-1deg); }
      50% { transform: translateY(-6px) translateX(-4px) rotate(0.5deg); }
      66% { transform: translateY(10px) translateX(3px) rotate(-0.5deg); }
      83% { transform: translateY(-4px) translateX(-2px) rotate(0deg); }
    }
    
    /* Paper rustle effect on click */
    @keyframes origami-rustle {
      0% { transform: rotate(0deg) scale(1); }
      25% { transform: rotate(-1deg) scale(0.98); }
      50% { transform: rotate(1deg) scale(1.02); }
      75% { transform: rotate(-0.5deg) scale(0.99); }
      100% { transform: rotate(0deg) scale(1); }
    }
    
    /* Expand animation for unfolding */
    @keyframes origami-unfold {
      0% { 
        transform: rotateX(0deg) rotateY(0deg) scale(1);
        filter: brightness(1);
      }
      100% { 
        transform: rotateX(15deg) rotateY(-15deg) scale(0.95);
        filter: brightness(1.1);
      }
    }
    
    /* Path highlight pulse */
    @keyframes origami-pulse {
      0%, 100% { 
        filter: drop-shadow(0 0 20px rgba(0, 212, 170, 0.6));
      }
      50% { 
        filter: drop-shadow(0 0 40px rgba(0, 212, 170, 0.9));
      }
    }
  `
}

/**
 * Get hover animation properties
 */
export function getHoverAnimation(): AnimationConfig {
  return {
    duration: 300,
    amplitude: { x: 0, y: -5, rotation: 0 },
    easing: 'cubic-bezier(0.4, 0, 0.2, 1)',
  }
}

/**
 * Get click animation properties
 */
export function getRustleAnimation(): AnimationConfig {
  return {
    duration: 400,
    amplitude: { x: 0, y: 0, rotation: 2 },
    easing: 'ease-out',
  }
}

/**
 * Calculate random animation delay for organic movement
 */
export function getRandomDelay(maxDelay: number = 2000): number {
  return Math.random() * maxDelay
}

/**
 * Generate CSS classes for animations
 */
export function generateAnimationClasses(): string {
  return `
    .origami-float-slow {
      animation: origami-float-slow 6s ease-in-out infinite;
    }
    
    .origami-float-medium {
      animation: origami-float-medium 4s ease-in-out infinite;
    }
    
    .origami-float-fast {
      animation: origami-float-fast 3s ease-in-out infinite;
    }
    
    .origami-float-intense {
      animation: origami-float-intense 2s ease-in-out infinite;
    }
    
    .origami-rustling {
      animation: origami-rustle 0.4s ease-out !important;
    }
    
    .origami-expanded {
      animation: origami-unfold 0.6s cubic-bezier(0.4, 0, 0.2, 1) forwards;
    }
    
    .origami-highlighted {
      animation: origami-pulse 2s ease-in-out infinite;
    }
    
    /* Hover state */
    .origami-node:hover {
      transform: translateY(-5px) scale(1.02);
      filter: drop-shadow(0 15px 40px rgba(0, 102, 255, 0.5));
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }
    
    /* Transition classes */
    .origami-transition-transform {
      transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }
    
    .origami-transition-filter {
      transition: filter 0.3s ease;
    }
    
    .origami-transition-all {
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }
  `
}

/**
 * Performance optimization: Check if animation should be disabled
 */
export function shouldDisableAnimations(nodeCount: number): boolean {
  // Disable animations if too many nodes or user prefers reduced motion
  const performanceThreshold = 150 // Disable animations above this node count

  // Check for reduced motion preference (SSR safe)
  let prefersReducedMotion = false
  if (typeof window !== 'undefined') {
    try {
      prefersReducedMotion = window.matchMedia(
        '(prefers-reduced-motion: reduce)',
      ).matches
    } catch (error) {
      // Fallback if matchMedia is not supported
      prefersReducedMotion = false
    }
  }

  return prefersReducedMotion || nodeCount > performanceThreshold
}

/**
 * Get optimized animation duration based on performance
 */
export function getOptimizedDuration(
  baseDuration: number,
  nodeCount: number,
): number {
  if (nodeCount > 100) {
    return baseDuration * 1.5 // Slow down animations for better performance
  }
  return baseDuration
}
