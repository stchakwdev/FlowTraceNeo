# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FlowTrace is a financial transparency platform for visualizing and verifying money flows through organizations, targeting African markets. The platform features interactive flow visualizations, AI-powered Trust Scores (0-100), real-time transaction verification, and mobile-first design optimized for African infrastructure.

## Development Commands

**Use `bun` instead of `npm` for all commands.**

```bash
# Development
cd flowtrace && bun run dev    # Start dev server on port 3000

# Build & Preview
bun run build                  # Build for production
bun run preview                # Preview production build

# Testing & Quality
bun run test                   # Run Vitest tests
bun run lint                   # Run ESLint
bun run format                 # Run Prettier
bun run check                  # Format + lint fix combined

# Install dependencies
bun add <package>              # Add dependency
bun add -d <package>           # Add dev dependency
```

## Tech Stack

**Frontend:**
- TanStack Start with React 19 and TypeScript
- TanStack Router for file-based routing (routes auto-generated in `routeTree.gen.ts`)
- Tailwind CSS v4 with `@tailwindcss/vite` plugin
- shadcn/ui (base-lyra style) with Base UI React primitives
- Phosphor Icons (`@phosphor-icons/react`)
- Vite 7 with Nitro for SSR

**Backend:**
- **Auth**: WorkOS AuthKit (SSO for nonprofits)
- **Database**: Supabase PostgreSQL with PostGIS, pgvector
- **Storage**: Supabase Storage for receipts/documents
- **Functions**: Supabase Edge Functions (Deno)
- **Real-time**: Supabase Realtime subscriptions

## Architecture

```
V2/
├── flowtrace/           # Main application
│   ├── src/
│   │   ├── components/
│   │   │   ├── ui/      # shadcn/ui base components
│   │   │   └── *.tsx    # Feature components
│   │   ├── lib/         # Utilities (utils.ts with cn helper)
│   │   ├── routes/      # TanStack Router file-based routes
│   │   │   ├── __root.tsx   # Root layout with devtools
│   │   │   └── index.tsx    # Home route
│   │   ├── router.tsx       # Router configuration
│   │   ├── routeTree.gen.ts # Auto-generated route tree
│   │   └── styles.css       # Global styles + Tailwind
│   ├── public/          # Static assets
│   └── vite.config.ts   # Vite + TanStack Start + Nitro config
└── Docs/                    # Project documentation
    ├── flowtrace-prd.md             # Product requirements
    ├── flowtrace-architecture.md    # Tech stack, routing, deployment
    ├── flowtrace-backend.md         # Database, APIs, storage
    ├── flowtrace-standards.md       # Code style, git, testing
    ├── flowtrace-user-docs.md       # End-user documentation
    ├── security-implementation-guide.md  # Security + WorkOS auth
    └── archive/                     # Archived reference docs
```

## Key Patterns

### Path Aliases
Use `@/*` for imports from `src/`:
```typescript
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
```

### TanStack Router Routes
Routes are file-based in `src/routes/`. Create new routes as:
```typescript
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/path')({
  component: YourComponent,
})
```

### Protected Routes (WorkOS AuthKit)
Use `_authenticated` layout for protected routes:
```typescript
// src/routes/_authenticated.tsx
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/_authenticated')({
  beforeLoad: async ({ location }) => {
    const { user } = await getAuth()
    if (!user) {
      throw redirect({ href: await getSignInUrl() })
    }
    return { user }
  },
})

// Child routes: src/routes/_authenticated/dashboard.tsx
```

### Adding UI Components
Use shadcn CLI:
```bash
cd flowtrace && npx shadcn add <component-name>
```
Components use Base UI React (`@base-ui/react`), Phosphor icons, and `class-variance-authority` for variants.

### Styling
- Tailwind v4 CSS-first configuration (no tailwind.config.js)
- Use `cn()` helper from `@/lib/utils` for conditional classes
- CSS variables defined in `styles.css` for theming

## Code Style

- TypeScript strict mode enabled
- No semicolons, single quotes (Prettier config)
- ESLint uses TanStack config (`@tanstack/eslint-config`)
- Mobile-first responsive design
- Optimize for low-bandwidth African networks

## Git Commits

- Never use Co-Authored-By lines in commits
- Keep commit messages concise and descriptive
- Push to remote: `git push origin main`

## Domain Concepts

- **Trust Score**: 0-100 metric based on transaction verification (40%), documentation (30%), consistency (30%)
- **Flow Visualization**: Interactive diagrams showing money movement through nodes
- **Nodes**: Entities in flow diagrams (donors, departments, vendors)
- **Verifications**: Proof attached to transactions (receipts, photos, GPS, biometric)

## Mobile Money Integrations (Planned)

- M-Pesa (Kenya, Tanzania) via Daraja API
- MTN Money (Uganda, Ghana, Nigeria)
- Airtel Money
- Flutterwave/Paystack for fallback

## Performance Requirements

- Page load < 3 seconds on 3G
- PWA with offline-first capability
- Support for 2G/3G networks
- Works on low-end Android devices
