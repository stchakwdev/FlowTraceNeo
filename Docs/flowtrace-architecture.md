# FlowTrace Technical Architecture

## Technology Stack

### Frontend
- **Framework**: TanStack Start v1.132+ with TanStack Router
- **UI**: React 19 + TypeScript (strict mode)
- **Components**: Base UI React + shadcn/ui (base-lyra style)
- **Styling**: Tailwind CSS v4 with OKLch color system
- **Icons**: Phosphor Icons (@phosphor-icons/react)
- **Build**: Vite 7 with Nitro SSR

### Backend
- **Database**: Supabase PostgreSQL with PostGIS, pgvector
- **Auth**: WorkOS AuthKit (SSO for nonprofits)
- **Storage**: Supabase Storage (receipts, documents)
- **Edge Functions**: Deno runtime for OCR, webhooks
- **Real-time**: Supabase Realtime subscriptions

### Mobile Money
- M-Pesa (Kenya/Tanzania) via Daraja API
- MTN Money (Uganda, Ghana, Nigeria)
- Airtel Money, Flutterwave/Paystack fallback

---

## Project Structure

```
flowtrace/
├── src/
│   ├── components/
│   │   ├── ui/                    # shadcn/ui base components
│   │   ├── flows/                 # Flow visualization
│   │   ├── transactions/          # Transaction components
│   │   ├── dashboard/             # Dashboard widgets
│   │   └── trust-score/           # Trust score displays
│   ├── routes/
│   │   ├── __root.tsx             # Root layout + AuthKitProvider
│   │   ├── index.tsx              # Landing page
│   │   ├── api/
│   │   │   └── auth/
│   │   │       └── callback.tsx   # WorkOS OAuth callback
│   │   └── _authenticated/        # Protected routes
│   │       ├── dashboard.tsx
│   │       ├── transactions/
│   │       ├── flows.tsx
│   │       ├── trust-score.tsx
│   │       └── settings/
│   ├── lib/
│   │   ├── utils.ts               # cn() helper
│   │   ├── auth.ts                # WorkOS utilities
│   │   ├── supabase.ts            # Supabase client
│   │   └── trust-score.ts         # Score calculations
│   ├── hooks/                     # Custom React hooks
│   ├── types/                     # TypeScript types
│   ├── stores/                    # Zustand stores
│   ├── router.tsx                 # Router config
│   ├── routeTree.gen.ts           # Auto-generated routes
│   └── styles.css                 # Global + Tailwind
├── public/                        # Static assets
├── supabase/
│   ├── functions/                 # Edge functions
│   ├── migrations/                # DB migrations
│   └── seed.sql                   # Seed data
└── e2e/                           # Playwright tests
```

---

## Routing Architecture

### TanStack Router Patterns

Routes are file-based. Create new routes as:

```typescript
// src/routes/example.tsx
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/example')({
  component: ExampleComponent,
  loader: async () => {
    // Fetch data before render
  },
})
```

### Protected Routes

Use layout routes with `_authenticated` prefix:

```typescript
// src/routes/_authenticated.tsx
import { createFileRoute, redirect, Outlet } from '@tanstack/react-router'
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/_authenticated')({
  beforeLoad: async ({ location }) => {
    const { user } = await getAuth()
    if (!user) {
      const signInUrl = await getSignInUrl({
        data: { returnPathname: location.pathname },
      })
      throw redirect({ href: signInUrl })
    }
  },
  component: () => <Outlet />,
})
```

---

## Authentication (WorkOS AuthKit)

### Setup

```bash
npm install @workos/authkit-tanstack-react-start
```

### Environment Variables

```env
WORKOS_CLIENT_ID=client_xxx
WORKOS_API_KEY=sk_xxx
WORKOS_COOKIE_PASSWORD=32+_character_secret_here
WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback
```

### Root Layout Integration

```typescript
// src/routes/__root.tsx
import { AuthKitProvider } from '@workos/authkit-tanstack-react-start/client'
import { getAuth } from '@workos/authkit-tanstack-react-start'

export const Route = createRootRoute({
  beforeLoad: async () => {
    const auth = await getAuth()
    return { auth }
  },
  component: RootLayout,
})

function RootLayout() {
  const { auth } = Route.useRouteContext()
  return (
    <AuthKitProvider initialAuth={auth}>
      {/* App shell */}
    </AuthKitProvider>
  )
}
```

### OAuth Callback

```typescript
// src/routes/api/auth/callback.tsx
import { createFileRoute } from '@tanstack/react-router'
import { handleCallbackRoute } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/api/auth/callback')({
  loader: handleCallbackRoute(),
})
```

---

## Development

### Commands

```bash
cd flowtrace

# Development
npm run dev              # Start dev server (port 3000)

# Build & Preview
npm run build            # Production build
npm run preview          # Preview build

# Quality
npm run test             # Run Vitest
npm run lint             # ESLint
npm run format           # Prettier
npm run check            # Format + lint fix
```

### Path Aliases

```typescript
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useAuth } from '@/hooks/useAuth'
```

### Adding UI Components

```bash
npx shadcn add button card input
```

---

## Deployment

### Vercel (Recommended)

```bash
npm i -g vercel
vercel
```

**Environment Variables in Vercel:**
- `WORKOS_CLIENT_ID`
- `WORKOS_API_KEY`
- `WORKOS_COOKIE_PASSWORD`
- `WORKOS_REDIRECT_URI` (production URL)
- `SUPABASE_URL`
- `SUPABASE_ANON_KEY`

### Build Output

TanStack Start with Nitro outputs to `.output/` directory with server and static assets.

---

## Performance Requirements

### Network Optimization (African Markets)

| Metric | Target |
|--------|--------|
| Page Load (3G) | < 3 seconds |
| First Contentful Paint | < 1.5 seconds |
| Time to Interactive | < 3.5 seconds |
| Lighthouse Score | 90+ |

### Implementation

- **Image Optimization**: Use lazy loading, WebP format, responsive sizes
- **Code Splitting**: Dynamic imports for heavy components (D3, charts)
- **Caching**: Service worker for offline-first PWA
- **Bundle Size**: Monitor with `npm run build` output
- **API Responses**: Paginate (20 items/page), compress, cache

### PWA Support

Configure in `vite.config.ts` with `vite-plugin-pwa`:

```typescript
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    VitePWA({
      registerType: 'autoUpdate',
      manifest: {
        name: 'FlowTrace',
        short_name: 'FlowTrace',
        theme_color: '#6366f1',
      },
      workbox: {
        runtimeCaching: [/* caching strategies */],
      },
    }),
  ],
})
```

---

## Styling System

### Tailwind v4 with OKLch

Colors defined in `styles.css`:

```css
:root {
  --primary: oklch(0.488 0.243 264.376);
  --secondary: oklch(0.967 0.001 286.375);
  --destructive: oklch(0.577 0.245 27.325);
  /* ... */
}
```

### Class Merging

Use `cn()` helper from `@/lib/utils`:

```typescript
import { cn } from '@/lib/utils'

<div className={cn('base-class', isActive && 'active-class')} />
```

### Component Variants (CVA)

```typescript
import { cva } from 'class-variance-authority'

const buttonVariants = cva('base-button-styles', {
  variants: {
    variant: {
      default: 'bg-primary text-white',
      outline: 'border border-input',
    },
    size: {
      default: 'h-10 px-4',
      sm: 'h-8 px-3',
    },
  },
  defaultVariants: {
    variant: 'default',
    size: 'default',
  },
})
```

---

## State Management

### Server State (TanStack Query)

```typescript
import { useQuery, useMutation } from '@tanstack/react-query'

function useTransactions(orgId: string) {
  return useQuery({
    queryKey: ['transactions', orgId],
    queryFn: () => fetchTransactions(orgId),
    staleTime: 5 * 60 * 1000,
  })
}
```

### Client State (Zustand)

```typescript
import { create } from 'zustand'

const useUIStore = create((set) => ({
  sidebarOpen: false,
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
}))
```

---

## Error Handling

### API Errors

```typescript
class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message)
  }
}

// Usage
try {
  await api.createTransaction(data)
} catch (e) {
  if (e instanceof ApiError && e.status === 422) {
    setErrors(e.details)
  }
}
```

### Error Boundaries

```typescript
import { ErrorBoundary } from 'react-error-boundary'

<ErrorBoundary fallback={<ErrorFallback />}>
  <Component />
</ErrorBoundary>
```
