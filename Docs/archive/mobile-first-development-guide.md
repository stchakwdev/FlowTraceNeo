# FlowTrace Mobile-First Development Guide for African Markets

## Understanding the African Mobile Landscape

### Key Market Characteristics

**Device Demographics:**
- **70% use Android** (primarily versions 5-10)
- **20% use feature phones** with basic browsers
- **10% use iOS** (mainly in urban areas)
- **Average RAM**: 1-2GB
- **Average Storage**: 8-16GB
- **Screen Sizes**: 5-6.5 inches (720x1280 to 1080x2340)

**Network Realities:**
- **2G**: Still prevalent in rural areas (50-250 Kbps)
- **3G**: Most common (0.5-2 Mbps)
- **4G**: Growing in cities (5-20 Mbps)
- **Data Costs**: $2-10 per GB (significant expense)
- **Network Reliability**: Frequent disconnections

**Usage Patterns:**
- **Data Conservation**: Users actively manage data usage
- **Offline Periods**: 30-40% of time without connectivity
- **Shared Devices**: Common in families
- **Mobile Money**: Primary payment method
- **WhatsApp**: Universal communication tool

## Mobile-First Architecture

### Progressive Web App (PWA) Strategy

```javascript
// next.config.js - PWA Configuration
const withPWA = require('next-pwa')({
  dest: 'public',
  register: true,
  skipWaiting: true,
  disable: process.env.NODE_ENV === 'development',
  runtimeCaching: [
    {
      urlPattern: /^https:\/\/api\.flowtrace\.io\/v1\/transactions/,
      handler: 'NetworkFirst',
      options: {
        cacheName: 'transactions-cache',
        expiration: {
          maxEntries: 100,
          maxAgeSeconds: 60 * 60 * 24 // 24 hours
        },
        networkTimeoutSeconds: 10
      }
    },
    {
      urlPattern: /\.(png|jpg|jpeg|svg|webp)$/,
      handler: 'CacheFirst',
      options: {
        cacheName: 'images-cache',
        expiration: {
          maxEntries: 50,
          maxAgeSeconds: 60 * 60 * 24 * 30 // 30 days
        }
      }
    }
  ]
})

module.exports = withPWA({
  // ... other config
})
```

### Service Worker Implementation

```javascript
// public/sw.js
const CACHE_VERSION = 'v1'
const STATIC_CACHE = `static-${CACHE_VERSION}`
const DYNAMIC_CACHE = `dynamic-${CACHE_VERSION}`
const OFFLINE_URL = '/offline'

// Critical resources to cache
const STATIC_ASSETS = [
  '/',
  '/offline',
  '/manifest.json',
  '/icons/icon-192x192.png',
  '/fonts/inter-var.woff2'
]

// Install event - cache critical resources
self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(STATIC_CACHE)
      .then(cache => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting())
  )
})

// Activate event - clean old caches
self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys().then(cacheNames => {
      return Promise.all(
        cacheNames
          .filter(name => name !== STATIC_CACHE && name !== DYNAMIC_CACHE)
          .map(name => caches.delete(name))
      )
    }).then(() => self.clients.claim())
  )
})

// Fetch event - network first, fallback to cache
self.addEventListener('fetch', event => {
  const { request } = event
  const url = new URL(request.url)

  // API calls - network first with timeout
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(
      Promise.race([
        fetch(request),
        new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Timeout')), 5000)
        )
      ])
      .then(response => {
        // Cache successful responses
        if (response.ok) {
          const responseClone = response.clone()
          caches.open(DYNAMIC_CACHE).then(cache => {
            cache.put(request, responseClone)
          })
        }
        return response
      })
      .catch(() => {
        // Fallback to cache
        return caches.match(request)
      })
    )
    return
  }

  // Static assets - cache first
  event.respondWith(
    caches.match(request)
      .then(response => response || fetch(request))
      .catch(() => {
        if (request.mode === 'navigate') {
          return caches.match(OFFLINE_URL)
        }
      })
  )
})

// Background sync for offline transactions
self.addEventListener('sync', event => {
  if (event.tag === 'sync-transactions') {
    event.waitUntil(syncOfflineTransactions())
  }
})

async function syncOfflineTransactions() {
  const cache = await caches.open('offline-transactions')
  const requests = await cache.keys()
  
  for (const request of requests) {
    try {
      const response = await fetch(request.clone())
      if (response.ok) {
        await cache.delete(request)
      }
    } catch (error) {
      console.error('Sync failed:', error)
    }
  }
}
```

## Responsive Design System

### Mobile-First Breakpoints

```scss
// styles/breakpoints.scss
$breakpoints: (
  'xs': 320px,   // Small phones
  'sm': 375px,   // Standard phones
  'md': 768px,   // Tablets
  'lg': 1024px,  // Desktop
  'xl': 1280px   // Large screens
);

@mixin respond-to($breakpoint) {
  @media (min-width: map-get($breakpoints, $breakpoint)) {
    @content;
  }
}

// Usage
.container {
  padding: 16px; // Mobile first
  
  @include respond-to('md') {
    padding: 24px;
  }
  
  @include respond-to('lg') {
    padding: 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
}
```

### Touch-Optimized Components

```tsx
// components/TouchButton.tsx
import { forwardRef, ButtonHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

interface TouchButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost'
  size?: 'small' | 'medium' | 'large'
  fullWidth?: boolean
}

export const TouchButton = forwardRef<HTMLButtonElement, TouchButtonProps>(({
  className,
  variant = 'primary',
  size = 'medium',
  fullWidth = false,
  children,
  ...props
}, ref) => {
  const sizes = {
    small: 'min-h-[40px] px-3 text-sm',
    medium: 'min-h-[48px] px-4 text-base', // 48px minimum for touch
    large: 'min-h-[56px] px-6 text-lg'
  }

  const variants = {
    primary: 'bg-blue-600 text-white active:bg-blue-700',
    secondary: 'bg-gray-200 text-gray-900 active:bg-gray-300',
    ghost: 'bg-transparent text-blue-600 active:bg-gray-100'
  }

  return (
    <button
      ref={ref}
      className={cn(
        'inline-flex items-center justify-center',
        'font-medium rounded-lg',
        'transition-colors duration-150',
        'touch-manipulation', // Prevents zoom on double tap
        'select-none', // Prevents text selection
        'outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2',
        sizes[size],
        variants[variant],
        fullWidth && 'w-full',
        className
      )}
      {...props}
    >
      {children}
    </button>
  )
})

TouchButton.displayName = 'TouchButton'
```

### Swipeable Transaction Card

```tsx
// components/SwipeableTransactionCard.tsx
import { useSwipeable } from 'react-swipeable'
import { useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'

export function SwipeableTransactionCard({ 
  transaction, 
  onVerify, 
  onDelete 
}: SwipeableTransactionCardProps) {
  const [swipeDirection, setSwipeDirection] = useState<'left' | 'right' | null>(null)
  
  const handlers = useSwipeable({
    onSwipedLeft: () => setSwipeDirection('left'),
    onSwipedRight: () => setSwipeDirection('right'),
    preventDefaultTouchmoveEvent: true,
    trackMouse: false,
    delta: 50 // Minimum swipe distance
  })

  const handleAction = () => {
    if (swipeDirection === 'left') {
      onDelete(transaction.id)
    } else if (swipeDirection === 'right') {
      onVerify(transaction.id)
    }
    setSwipeDirection(null)
  }

  return (
    <div className="relative overflow-hidden" {...handlers}>
      <AnimatePresence>
        {swipeDirection && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className={cn(
              'absolute inset-0 flex items-center px-4',
              swipeDirection === 'right' 
                ? 'bg-green-500 justify-start' 
                : 'bg-red-500 justify-end'
            )}
            onClick={handleAction}
          >
            <span className="text-white font-medium">
              {swipeDirection === 'right' ? 'Verify' : 'Delete'}
            </span>
          </motion.div>
        )}
      </AnimatePresence>
      
      <motion.div
        animate={{ 
          x: swipeDirection === 'right' ? 100 : swipeDirection === 'left' ? -100 : 0 
        }}
        transition={{ type: 'spring', stiffness: 300, damping: 30 }}
        className="relative bg-white p-4 border-b"
      >
        <div className="flex justify-between items-start">
          <div className="flex-1">
            <p className="font-medium text-gray-900">
              {formatMoney(transaction.amount, transaction.currency)}
            </p>
            <p className="text-sm text-gray-600 mt-1">
              {transaction.description}
            </p>
            <p className="text-xs text-gray-500 mt-2">
              {formatRelativeTime(transaction.createdAt)}
            </p>
          </div>
          
          {transaction.verificationStatus === 'verified' && (
            <CheckCircleIcon className="w-5 h-5 text-green-500 flex-shrink-0" />
          )}
        </div>
      </motion.div>
    </div>
  )
}
```

## Data Optimization

### Image Optimization Strategy

```tsx
// components/OptimizedImage.tsx
import Image from 'next/image'
import { useState, useEffect } from 'react'
import { useNetworkQuality } from '@/hooks/useNetworkQuality'

interface OptimizedImageProps {
  src: string
  alt: string
  width: number
  height: number
  priority?: boolean
}

export function OptimizedImage({ 
  src, 
  alt, 
  width, 
  height, 
  priority = false 
}: OptimizedImageProps) {
  const networkQuality = useNetworkQuality()
  const [imageSrc, setImageSrc] = useState<string>('/placeholder.png')
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    // Adjust quality based on network
    const quality = networkQuality === 'slow' ? 40 : 
                   networkQuality === 'medium' ? 60 : 80

    // For slow connections, use smaller size
    const maxWidth = networkQuality === 'slow' ? 400 : 
                    networkQuality === 'medium' ? 800 : 1200

    const optimizedSrc = `${src}?w=${Math.min(width, maxWidth)}&q=${quality}`
    setImageSrc(optimizedSrc)
  }, [src, width, networkQuality])

  return (
    <div className="relative">
      {isLoading && (
        <div className="absolute inset-0 bg-gray-200 animate-pulse" />
      )}
      
      <Image
        src={imageSrc}
        alt={alt}
        width={width}
        height={height}
        loading={priority ? 'eager' : 'lazy'}
        onLoadingComplete={() => setIsLoading(false)}
        className={cn(
          'transition-opacity duration-300',
          isLoading ? 'opacity-0' : 'opacity-100'
        )}
      />
    </div>
  )
}
```

### Efficient Data Fetching

```typescript
// hooks/useOptimizedQuery.ts
import { useQuery, UseQueryOptions } from '@tanstack/react-query'
import { useNetworkQuality } from './useNetworkQuality'
import { useOfflineQueue } from './useOfflineQueue'

export function useOptimizedQuery<T>(
  key: any[],
  fetcher: () => Promise<T>,
  options?: UseQueryOptions<T>
) {
  const networkQuality = useNetworkQuality()
  const { queueRequest } = useOfflineQueue()
  
  // Adjust caching based on network
  const cacheTime = networkQuality === 'slow' ? 
    30 * 60 * 1000 : // 30 minutes for slow
    5 * 60 * 1000    // 5 minutes for fast

  const staleTime = networkQuality === 'slow' ? 
    15 * 60 * 1000 : // 15 minutes for slow
    1 * 60 * 1000    // 1 minute for fast

  return useQuery({
    queryKey: key,
    queryFn: async () => {
      try {
        return await fetcher()
      } catch (error) {
        // If offline, return cached data if available
        if (!navigator.onLine) {
          const cached = await getCachedData(key)
          if (cached) return cached
          
          // Queue for later
          queueRequest({ key, fetcher })
          throw new Error('Offline - request queued')
        }
        throw error
      }
    },
    cacheTime,
    staleTime,
    retry: (failureCount, error) => {
      // Don't retry if offline
      if (!navigator.onLine) return false
      return failureCount < 3
    },
    ...options
  })
}
```

### Pagination for Large Lists

```tsx
// components/InfiniteTransactionList.tsx
import { useInfiniteQuery } from '@tanstack/react-query'
import { useInView } from 'react-intersection-observer'
import { useVirtualizer } from '@tanstack/react-virtual'

export function InfiniteTransactionList({ organizationId }: Props) {
  const { ref, inView } = useInView()
  
  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
    isError
  } = useInfiniteQuery({
    queryKey: ['transactions', organizationId],
    queryFn: ({ pageParam = 1 }) => 
      fetchTransactions({ organizationId, page: pageParam, pageSize: 20 }),
    getNextPageParam: (lastPage) => 
      lastPage.hasMore ? lastPage.page + 1 : undefined,
  })

  // Fetch more when scrolled to bottom
  useEffect(() => {
    if (inView && hasNextPage && !isFetchingNextPage) {
      fetchNextPage()
    }
  }, [inView, hasNextPage, isFetchingNextPage, fetchNextPage])

  const allTransactions = data?.pages.flatMap(page => page.data) ?? []

  // Virtual scrolling for performance
  const parentRef = useRef<HTMLDivElement>(null)
  const virtualizer = useVirtualizer({
    count: allTransactions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 80, // Estimated row height
    overscan: 5
  })

  if (isLoading) return <TransactionListSkeleton />
  if (isError) return <ErrorState />

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative'
        }}
      >
        {virtualizer.getVirtualItems().map(virtualItem => (
          <div
            key={virtualItem.key}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualItem.size}px`,
              transform: `translateY(${virtualItem.start}px)`
            }}
          >
            <TransactionCard 
              transaction={allTransactions[virtualItem.index]} 
            />
          </div>
        ))}
      </div>
      
      {/* Intersection observer trigger */}
      <div ref={ref} className="h-10" />
      
      {isFetchingNextPage && (
        <div className="p-4 text-center">
          <Spinner />
        </div>
      )}
    </div>
  )
}
```

## Offline Functionality

### Offline Transaction Queue

```typescript
// lib/offline/transactionQueue.ts
import Dexie, { Table } from 'dexie'

interface QueuedTransaction {
  id: string
  data: CreateTransactionData
  status: 'pending' | 'syncing' | 'failed'
  attempts: number
  createdAt: Date
  error?: string
}

class OfflineDatabase extends Dexie {
  transactions!: Table<QueuedTransaction>
  
  constructor() {
    super('FlowTraceOffline')
    
    this.version(1).stores({
      transactions: 'id, status, createdAt'
    })
  }
}

export class OfflineTransactionQueue {
  private db = new OfflineDatabase()
  
  async addTransaction(data: CreateTransactionData): Promise<string> {
    const id = `offline_${Date.now()}_${Math.random()}`
    
    await this.db.transactions.add({
      id,
      data,
      status: 'pending',
      attempts: 0,
      createdAt: new Date()
    })
    
    // Try to sync immediately if online
    if (navigator.onLine) {
      this.processPendingTransactions()
    }
    
    return id
  }
  
  async processPendingTransactions() {
    const pending = await this.db.transactions
      .where('status')
      .equals('pending')
      .toArray()
    
    for (const transaction of pending) {
      await this.syncTransaction(transaction)
    }
  }
  
  private async syncTransaction(queued: QueuedTransaction) {
    try {
      // Update status
      await this.db.transactions.update(queued.id, { 
        status: 'syncing' 
      })
      
      // Send to API
      const response = await fetch('/api/transactions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Offline-Id': queued.id
        },
        body: JSON.stringify(queued.data)
      })
      
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }
      
      const result = await response.json()
      
      // Success - remove from queue
      await this.db.transactions.delete(queued.id)
      
      // Notify UI
      window.dispatchEvent(new CustomEvent('transaction-synced', {
        detail: { offlineId: queued.id, transaction: result }
      }))
      
    } catch (error) {
      // Failed - update status and retry later
      await this.db.transactions.update(queued.id, {
        status: 'pending',
        attempts: queued.attempts + 1,
        error: error.message
      })
      
      // Exponential backoff
      const delay = Math.min(1000 * Math.pow(2, queued.attempts), 60000)
      setTimeout(() => {
        if (navigator.onLine) {
          this.syncTransaction(queued)
        }
      }, delay)
    }
  }
}
```

### Offline-First Hook

```typescript
// hooks/useOfflineFirst.ts
import { useState, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { OfflineTransactionQueue } from '@/lib/offline/transactionQueue'

export function useCreateTransaction() {
  const queryClient = useQueryClient()
  const queue = new OfflineTransactionQueue()
  const [isCreating, setIsCreating] = useState(false)
  
  const createTransaction = async (data: CreateTransactionData) => {
    setIsCreating(true)
    
    try {
      if (navigator.onLine) {
        // Try online first
        const response = await fetch('/api/transactions', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(data)
        })
        
        if (!response.ok) {
          throw new Error('Failed to create transaction')
        }
        
        const transaction = await response.json()
        
        // Update cache
        queryClient.invalidateQueries(['transactions'])
        
        toast.success('Transaction created')
        return transaction
        
      } else {
        // Offline - queue it
        const offlineId = await queue.addTransaction(data)
        
        // Optimistic update
        queryClient.setQueryData(['transactions'], (old: any) => {
          return {
            ...old,
            data: [
              {
                ...data,
                id: offlineId,
                isOffline: true,
                createdAt: new Date().toISOString()
              },
              ...(old?.data || [])
            ]
          }
        })
        
        toast.info('Transaction saved offline')
        return { id: offlineId, isOffline: true }
      }
      
    } catch (error) {
      // Fallback to offline
      if (!navigator.onLine) {
        const offlineId = await queue.addTransaction(data)
        toast.info('Transaction saved offline')
        return { id: offlineId, isOffline: true }
      }
      
      toast.error('Failed to create transaction')
      throw error
      
    } finally {
      setIsCreating(false)
    }
  }
  
  // Listen for sync events
  useEffect(() => {
    const handleSync = (event: CustomEvent) => {
      const { offlineId, transaction } = event.detail
      
      // Update optimistic data with real data
      queryClient.setQueryData(['transactions'], (old: any) => {
        return {
          ...old,
          data: old?.data.map((t: any) => 
            t.id === offlineId ? transaction : t
          )
        }
      })
      
      toast.success('Transaction synced')
    }
    
    window.addEventListener('transaction-synced', handleSync as any)
    return () => {
      window.removeEventListener('transaction-synced', handleSync as any)
    }
  }, [queryClient])
  
  return { createTransaction, isCreating }
}
```

## Performance Optimization

### Bundle Size Optimization

```javascript
// next.config.js
module.exports = {
  webpack: (config, { dev, isServer }) => {
    // Replace large libraries with smaller alternatives
    config.resolve.alias = {
      ...config.resolve.alias,
      'moment': 'dayjs',
      'lodash': 'lodash-es'
    }
    
    // Tree shake unused icons
    config.module.rules.push({
      test: /lucide-react/,
      sideEffects: false
    })
    
    // Analyze bundle in production
    if (!dev && !isServer) {
      const { BundleAnalyzerPlugin } = require('webpack-bundle-analyzer')
      config.plugins.push(
        new BundleAnalyzerPlugin({
          analyzerMode: 'static',
          openAnalyzer: false
        })
      )
    }
    
    return config
  },
  
  // Optimize images
  images: {
    formats: ['image/avif', 'image/webp'],
    deviceSizes: [320, 420, 640, 768, 1024],
    imageSizes: [16, 32, 48, 64, 96]
  }
}
```

### Code Splitting Strategy

```tsx
// app/dashboard/page.tsx
import dynamic from 'next/dynamic'
import { Suspense } from 'react'

// Heavy components loaded on demand
const FlowVisualization = dynamic(
  () => import('@/components/FlowVisualization'),
  {
    loading: () => <FlowVisualizationSkeleton />,
    ssr: false // Don't render on server
  }
)

const TransactionAnalytics = dynamic(
  () => import('@/components/TransactionAnalytics'),
  {
    loading: () => <AnalyticsSkeleton />
  }
)

export default function DashboardPage() {
  return (
    <div className="space-y-6">
      {/* Critical content loads immediately */}
      <DashboardHeader />
      <TrustScoreCard />
      
      {/* Heavy components load after */}
      <Suspense fallback={<TransactionListSkeleton />}>
        <TransactionList />
      </Suspense>
      
      {/* Visualization loads on demand */}
      <FlowVisualization />
      
      {/* Analytics loads when scrolled into view */}
      <LazyLoad height={400} once>
        <TransactionAnalytics />
      </LazyLoad>
    </div>
  )
}
```

### Memory Management

```typescript
// hooks/useMemoryOptimizedList.ts
export function useMemoryOptimizedList<T>(
  items: T[],
  renderItem: (item: T) => React.ReactNode
) {
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: 50 })
  const containerRef = useRef<HTMLDivElement>(null)
  
  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    
    const observer = new IntersectionObserver(
      (entries) => {
        // Adjust visible range based on scroll
        const containerRect = container.getBoundingClientRect()
        const scrollTop = container.scrollTop
        const itemHeight = 80 // Approximate
        
        const start = Math.floor(scrollTop / itemHeight)
        const visibleCount = Math.ceil(containerRect.height / itemHeight)
        const end = start + visibleCount + 10 // Buffer
        
        setVisibleRange({ start, end })
      },
      { root: container, threshold: 0 }
    )
    
    observer.observe(container)
    return () => observer.disconnect()
  }, [])
  
  // Only render visible items
  const visibleItems = items.slice(
    visibleRange.start,
    visibleRange.end
  )
  
  return {
    containerRef,
    visibleItems,
    totalHeight: items.length * 80,
    offsetY: visibleRange.start * 80
  }
}
```

## Network Optimization

### Adaptive Loading

```typescript
// hooks/useNetworkQuality.ts
export function useNetworkQuality() {
  const [quality, setQuality] = useState<'slow' | 'medium' | 'fast'>('medium')
  
  useEffect(() => {
    // Check initial connection
    const connection = (navigator as any).connection
    
    if (connection) {
      const updateQuality = () => {
        const effectiveType = connection.effectiveType
        const downlink = connection.downlink
        
        if (effectiveType === 'slow-2g' || effectiveType === '2g' || downlink < 0.5) {
          setQuality('slow')
        } else if (effectiveType === '3g' || downlink < 2) {
          setQuality('medium')
        } else {
          setQuality('fast')
        }
      }
      
      updateQuality()
      connection.addEventListener('change', updateQuality)
      
      return () => {
        connection.removeEventListener('change', updateQuality)
      }
    }
    
    // Fallback: measure actual speed
    measureNetworkSpeed().then(speed => {
      if (speed < 0.5) setQuality('slow')
      else if (speed < 2) setQuality('medium')
      else setQuality('fast')
    })
  }, [])
  
  return quality
}

async function measureNetworkSpeed(): Promise<number> {
  const startTime = Date.now()
  const testImageSize = 50000 // 50KB test image
  
  try {
    await fetch(`/test-image.jpg?t=${Date.now()}`, { cache: 'no-store' })
    const duration = (Date.now() - startTime) / 1000
    const speedMbps = (testImageSize * 8) / (duration * 1000000)
    return speedMbps
  } catch {
    return 0.5 // Assume slow on error
  }
}
```

### Request Batching

```typescript
// lib/api/batchRequests.ts
class RequestBatcher {
  private queue: Map<string, {
    resolver: (value: any) => void
    rejecter: (error: any) => void
  }[]> = new Map()
  
  private timer: NodeJS.Timeout | null = null
  
  async get(url: string): Promise<any> {
    return new Promise((resolve, reject) => {
      if (!this.queue.has(url)) {
        this.queue.set(url, [])
      }
      
      this.queue.get(url)!.push({ resolver: resolve, rejecter: reject })
      
      // Batch requests every 50ms
      if (!this.timer) {
        this.timer = setTimeout(() => this.flush(), 50)
      }
    })
  }
  
  private async flush() {
    const batch = Array.from(this.queue.entries())
    this.queue.clear()
    this.timer = null
    
    // Create batch request
    const requests = batch.map(([url]) => ({ url }))
    
    try {
      const response = await fetch('/api/batch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ requests })
      })
      
      const results = await response.json()
      
      // Resolve individual promises
      batch.forEach(([url, resolvers], index) => {
        const result = results[index]
        
        if (result.error) {
          resolvers.forEach(({ rejecter }) => rejecter(result.error))
        } else {
          resolvers.forEach(({ resolver }) => resolver(result.data))
        }
      })
    } catch (error) {
      // Reject all promises
      batch.forEach(([, resolvers]) => {
        resolvers.forEach(({ rejecter }) => rejecter(error))
      })
    }
  }
}

export const batcher = new RequestBatcher()
```

## USSD Integration

### USSD Menu Structure

```typescript
// lib/ussd/menu.ts
export class USSDMenu {
  private sessions = new Map<string, USSDSession>()
  
  async handleRequest(
    sessionId: string,
    phoneNumber: string,
    input: string
  ): Promise<string> {
    let session = this.sessions.get(sessionId)
    
    if (!session) {
      session = {
        id: sessionId,
        phoneNumber,
        state: 'MAIN_MENU',
        data: {}
      }
      this.sessions.set(sessionId, session)
    }
    
    // Route based on state
    switch (session.state) {
      case 'MAIN_MENU':
        return this.mainMenu(session, input)
      
      case 'CHECK_BALANCE':
        return this.checkBalance(session, input)
      
      case 'RECORD_TRANSACTION':
        return this.recordTransaction(session, input)
      
      case 'CHECK_TRUST_SCORE':
        return this.checkTrustScore(session, input)
      
      default:
        return 'END Invalid option'
    }
  }
  
  private mainMenu(session: USSDSession, input: string): string {
    if (input === '') {
      return `CON Welcome to FlowTrace
1. Check Balance
2. Record Transaction
3. Check Trust Score
4. Recent Transactions`
    }
    
    switch (input) {
      case '1':
        session.state = 'CHECK_BALANCE'
        return this.checkBalance(session, '')
      
      case '2':
        session.state = 'RECORD_TRANSACTION'
        return 'CON Enter amount:'
      
      case '3':
        session.state = 'CHECK_TRUST_SCORE'
        return this.checkTrustScore(session, '')
      
      case '4':
        return this.getRecentTransactions(session)
      
      default:
        return 'END Invalid option'
    }
  }
  
  private async checkBalance(session: USSDSession, input: string): Promise<string> {
    const user = await this.getUser(session.phoneNumber)
    const balance = await this.getBalance(user.organizationId)
    
    return `END Your balance:
Main Account: KES ${formatNumber(balance.main)}
Petty Cash: KES ${formatNumber(balance.pettyCash)}
Total: KES ${formatNumber(balance.total)}`
  }
  
  private async checkTrustScore(session: USSDSession, input: string): Promise<string> {
    const user = await this.getUser(session.phoneNumber)
    const score = await this.getTrustScore(user.organizationId)
    
    return `END Trust Score: ${score.score}/100
Grade: ${score.grade}
Trend: ${score.trend}
Tips: ${score.tips[0]}`
  }
}
```

## Mobile Testing Strategy

### Device Testing Matrix

```yaml
# .github/workflows/mobile-tests.yml
name: Mobile Testing

on: [push, pull_request]

jobs:
  browserstack:
    runs-on: ubuntu-latest
    
    strategy:
      matrix:
        device:
          - { name: 'Samsung Galaxy A51', os: 'Android', version: '11' }
          - { name: 'Xiaomi Redmi Note 8', os: 'Android', version: '9' }
          - { name: 'iPhone SE 2020', os: 'iOS', version: '14' }
          - { name: 'iPad 8th', os: 'iOS', version: '14' }
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Run BrowserStack tests
        env:
          BROWSERSTACK_USERNAME: ${{ secrets.BROWSERSTACK_USERNAME }}
          BROWSERSTACK_ACCESS_KEY: ${{ secrets.BROWSERSTACK_ACCESS_KEY }}
        run: |
          npm run test:mobile -- \
            --device "${{ matrix.device.name }}" \
            --os "${{ matrix.device.os }}" \
            --os_version "${{ matrix.device.version }}"
```

### Performance Budget

```javascript
// lighthouse.config.js
module.exports = {
  ci: {
    collect: {
      url: ['http://localhost:3000'],
      numberOfRuns: 3,
      settings: {
        preset: 'mobile',
        throttling: {
          cpuSlowdownMultiplier: 4,
          requestLatencyMs: 150,
          downloadThroughputKbps: 1638.4, // 3G
          uploadThroughputKbps: 675
        }
      }
    },
    assert: {
      assertions: {
        'categories:performance': ['error', { minScore: 0.8 }],
        'categories:accessibility': ['error', { minScore: 0.9 }],
        'categories:seo': ['error', { minScore: 0.9 }],
        'categories:pwa': ['error', { minScore: 0.9 }],
        
        // Specific metrics
        'first-contentful-paint': ['error', { maxNumericValue: 2000 }],
        'interactive': ['error', { maxNumericValue: 5000 }],
        'speed-index': ['error', { maxNumericValue: 3000 }],
        'total-blocking-time': ['error', { maxNumericValue: 300 }],
        
        // Resource budgets
        'resource-summary:script:size': ['error', { maxNumericValue: 200000 }],
        'resource-summary:image:size': ['error', { maxNumericValue: 300000 }],
        'resource-summary:total:size': ['error', { maxNumericValue: 600000 }]
      }
    }
  }
}
```

## Mobile-Specific Features

### SMS Integration

```typescript
// lib/sms/notifications.ts
export class SMSNotifications {
  async sendTransactionAlert(
    phoneNumber: string,
    transaction: Transaction
  ) {
    const message = `FlowTrace Alert:
${transaction.type === 'credit' ? 'Received' : 'Sent'} ${formatMoney(transaction.amount, transaction.currency)}
From: ${transaction.fromNode.name}
To: ${transaction.toNode.name}
Balance: ${formatMoney(transaction.newBalance, transaction.currency)}
Ref: ${transaction.id.slice(-6)}`

    await this.sendSMS(phoneNumber, message)
  }
  
  async sendDailyDigest(
    phoneNumber: string,
    summary: DailySummary
  ) {
    const message = `FlowTrace Daily:
Transactions: ${summary.count}
Inflow: ${formatMoney(summary.inflow, 'KES')}
Outflow: ${formatMoney(summary.outflow, 'KES')}
Trust Score: ${summary.trustScore}
View details: ftrace.co/${summary.shortLink}`

    await this.sendSMS(phoneNumber, message)
  }
}
```

### WhatsApp Business Integration

```typescript
// lib/whatsapp/client.ts
export class WhatsAppClient {
  async sendTransactionReceipt(
    phoneNumber: string,
    transaction: Transaction,
    receiptUrl: string
  ) {
    // Send text message
    await this.sendMessage({
      to: phoneNumber,
      type: 'text',
      text: {
        body: `Transaction Receipt:\n\nAmount: ${formatMoney(transaction.amount, transaction.currency)}\nDate: ${formatDate(transaction.createdAt)}\nReference: ${transaction.reference}\n\nView online: ${transaction.publicUrl}`
      }
    })
    
    // Send receipt image
    await this.sendMessage({
      to: phoneNumber,
      type: 'image',
      image: {
        link: receiptUrl,
        caption: `Receipt for ${transaction.reference}`
      }
    })
  }
  
  async sendInteractiveMenu(phoneNumber: string) {
    await this.sendMessage({
      to: phoneNumber,
      type: 'interactive',
      interactive: {
        type: 'list',
        header: {
          type: 'text',
          text: 'FlowTrace Menu'
        },
        body: {
          text: 'What would you like to do?'
        },
        action: {
          button: 'Select Option',
          sections: [
            {
              title: 'Transactions',
              rows: [
                {
                  id: 'new_transaction',
                  title: 'Record Transaction',
                  description: 'Add a new transaction'
                },
                {
                  id: 'recent_transactions',
                  title: 'Recent Transactions',
                  description: 'View last 5 transactions'
                }
              ]
            },
            {
              title: 'Reports',
              rows: [
                {
                  id: 'trust_score',
                  title: 'Trust Score',
                  description: 'Check your current score'
                },
                {
                  id: 'daily_summary',
                  title: 'Daily Summary',
                  description: 'Get today\'s summary'
                }
              ]
            }
          ]
        }
      }
    })
  }
}
```

## Accessibility for African Users

### Multi-Language Support

```typescript
// lib/i18n/config.ts
export const languages = {
  en: 'English',
  sw: 'Kiswahili',
  yo: 'Yoruba',
  am: 'Amharic',
  fr: 'Français',
  ar: 'العربية'
}

// Quick language switcher for mobile
export function LanguageSwitcher() {
  const { locale, setLocale } = useLocale()
  
  return (
    <select
      value={locale}
      onChange={(e) => setLocale(e.target.value)}
      className="text-sm border rounded px-2 py-1"
      aria-label="Select language"
    >
      {Object.entries(languages).map(([code, name]) => (
        <option key={code} value={code}>
          {name}
        </option>
      ))}
    </select>
  )
}
```

### Low-Literacy Mode

```tsx
// components/LowLiteracyMode.tsx
export function TransactionFormSimple() {
  const [amount, setAmount] = useState('')
  const [type, setType] = useState<'in' | 'out'>('out')
  
  return (
    <div className="p-4 space-y-6">
      {/* Visual type selector */}
      <div className="flex gap-4">
        <button
          onClick={() => setType('in')}
          className={cn(
            'flex-1 p-6 border-2 rounded-lg transition',
            type === 'in' 
              ? 'border-green-500 bg-green-50' 
              : 'border-gray-300'
          )}
        >
          <ArrowDownIcon className="w-12 h-12 mx-auto mb-2 text-green-500" />
          <span className="text-lg font-medium">Money In</span>
        </button>
        
        <button
          onClick={() => setType('out')}
          className={cn(
            'flex-1 p-6 border-2 rounded-lg transition',
            type === 'out' 
              ? 'border-red-500 bg-red-50' 
              : 'border-gray-300'
          )}
        >
          <ArrowUpIcon className="w-12 h-12 mx-auto mb-2 text-red-500" />
          <span className="text-lg font-medium">Money Out</span>
        </button>
      </div>
      
      {/* Visual number pad */}
      <div>
        <div className="text-center mb-4">
          <span className="text-4xl font-bold">
            {formatMoney(Number(amount) || 0, 'KES')}
          </span>
        </div>
        
        <NumberPad
          value={amount}
          onChange={setAmount}
          maxLength={7}
        />
      </div>
      
      {/* Action buttons with icons */}
      <div className="flex gap-4">
        <button className="flex-1 btn-secondary h-16">
          <CameraIcon className="w-6 h-6 mr-2" />
          Add Photo
        </button>
        
        <button className="flex-1 btn-primary h-16">
          <CheckIcon className="w-6 h-6 mr-2" />
          Save
        </button>
      </div>
    </div>
  )
}
```

## Performance Monitoring

```typescript
// lib/monitoring/mobile-metrics.ts
export class MobileMetrics {
  trackPageLoad() {
    if ('PerformanceObserver' in window) {
      const observer = new PerformanceObserver((list) => {
        const entries = list.getEntries()
        
        entries.forEach((entry) => {
          if (entry.entryType === 'navigation') {
            const nav = entry as PerformanceNavigationTiming
            
            this.sendMetric('page_load', {
              dns: nav.domainLookupEnd - nav.domainLookupStart,
              tcp: nav.connectEnd - nav.connectStart,
              request: nav.responseStart - nav.requestStart,
              response: nav.responseEnd - nav.responseStart,
              dom: nav.domComplete - nav.responseEnd,
              total: nav.loadEventEnd - nav.fetchStart,
              
              // Mobile-specific
              connection: (navigator as any).connection?.effectiveType,
              deviceMemory: (navigator as any).deviceMemory,
              hardwareConcurrency: navigator.hardwareConcurrency
            })
          }
        })
      })
      
      observer.observe({ entryTypes: ['navigation'] })
    }
  }
  
  trackInteraction(name: string, duration: number) {
    this.sendMetric('interaction', {
      name,
      duration,
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight
      },
      touch: 'ontouchstart' in window
    })
  }
}
```

## Mobile Development Checklist

### Pre-Launch Mobile Testing
- [ ] Test on real devices (not just emulators)
- [ ] Test with 2G/3G network throttling
- [ ] Test offline functionality
- [ ] Test with low battery mode
- [ ] Test with data saver mode enabled
- [ ] Test on devices with 1GB RAM
- [ ] Test with screen readers
- [ ] Test in bright sunlight (contrast)
- [ ] Test one-handed operation
- [ ] Test with gloves (touch targets)

### Performance Targets
- [ ] First paint < 1.5s on 3G
- [ ] Interactive < 5s on 3G
- [ ] Bundle size < 200KB (gzipped)
- [ ] Image optimization working
- [ ] Offline mode functional
- [ ] No memory leaks
- [ ] Smooth scrolling (60fps)
- [ ] Touch response < 100ms

### Mobile-Specific Features
- [ ] PWA installable
- [ ] Push notifications working
- [ ] SMS integration tested
- [ ] WhatsApp integration tested
- [ ] USSD menu functional
- [ ] Mobile money working
- [ ] Camera integration smooth
- [ ] GPS location accurate
- [ ] Offline queue syncing
- [ ] Low-literacy mode available