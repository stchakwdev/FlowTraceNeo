import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
} from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthKitProvider } from '@workos/authkit-tanstack-react-start/client'
import { getAuth } from '@workos/authkit-tanstack-react-start'
import appCss from '../styles.css?url'
import { ThemeProvider } from '@/lib/theme'
import { OrgProvider } from '@/lib/org-context'

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
    // Server-side auth check - returns null if not authenticated
    const auth = await getAuth()
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
      {
        title: 'FlowTrace - Financial Transparency Platform',
      },
    ],
    links: [
      {
        rel: 'stylesheet',
        href: appCss,
      },
    ],
    scripts: [
      {
        children: themeScript,
      },
    ],
  }),

  component: RootComponent,
  shellComponent: RootDocument,
})

function RootComponent() {
  const { auth } = Route.useRouteContext()

  return (
    <AuthKitProvider initialAuth={auth}>
      <QueryClientProvider client={queryClient}>
        <OrgProvider>
          <ThemeProvider>
            <Outlet />
          </ThemeProvider>
        </OrgProvider>
      </QueryClientProvider>
    </AuthKitProvider>
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
