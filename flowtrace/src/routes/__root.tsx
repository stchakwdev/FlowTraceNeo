import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
} from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { getAuth } from '@workos/authkit-tanstack-react-start'
import appCss from '../styles.css?url'
import { ThemeProvider } from '@/lib/theme'
import { OrgProvider } from '@/lib/org-context'
import { AppAuthProvider } from '@/lib/auth-context'
import { DEV_MODE } from '@/lib/auth'
import { umamiScript } from '@/lib/analytics'
import { defaultHeadMeta } from '@/lib/site'

// Create a client for React Query
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000, // 30 seconds
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
})

// Script to prevent theme flash on page load
const themeScript = `
(function() {
  const theme = localStorage.getItem('flowtrace-theme') || 'system';
  const resolved = theme === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : theme;
  document.documentElement.classList.add(resolved);
})();
`

export const Route = createRootRoute({
  beforeLoad: async () => {
    // Server-side auth check - returns null if not authenticated.
    // Dummy auth mode skips WorkOS entirely.
    const auth = DEV_MODE ? undefined : await getAuth()
    return { auth }
  },
  head: () => ({
    meta: [
      {
        charSet: 'utf-8',
      },
      {
        name: 'viewport',
        content: 'width=device-width, initial-scale=1',
      },
      { name: 'theme-color', content: '#1d2431' },
      // Site-wide title, description and Open Graph defaults; public routes
      // override them with pageHead() from lib/site.ts.
      ...defaultHeadMeta(),
    ],
    links: [
      {
        rel: 'stylesheet',
        href: appCss,
      },
      { rel: 'manifest', href: '/manifest.json' },
      { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' },
      { rel: 'icon', href: '/favicon.ico', sizes: '32x32' },
      { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' },
    ],
    scripts: [
      {
        children: themeScript,
      },
      // Offline claim capture. Production only: in dev the worker would
      // cache Vite's unbundled modules.
      ...(import.meta.env.PROD
        ? [
            {
              children:
                "if('serviceWorker' in navigator){addEventListener('load',function(){navigator.serviceWorker.register('/sw.js').catch(function(){})})}",
            },
          ]
        : []),
      // Umami, only when VITE_UMAMI_WEBSITE_ID is set.
      ...umamiScript(),
    ],
  }),

  component: RootComponent,
  shellComponent: RootDocument,
})

function RootComponent() {
  const { auth } = Route.useRouteContext()

  return (
    <AppAuthProvider initialAuth={auth}>
      <QueryClientProvider client={queryClient}>
        <OrgProvider>
          <ThemeProvider>
            <Outlet />
          </ThemeProvider>
        </OrgProvider>
      </QueryClientProvider>
    </AppAuthProvider>
  )
}

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  )
}
