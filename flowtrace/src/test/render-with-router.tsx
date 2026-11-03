import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRouter,
} from '@tanstack/react-router'
import { render } from '@testing-library/react'
import { vi } from 'vitest'
import { ThemeProvider } from '@/lib/theme'

/** Renders a component inside a one-route memory router and the theme provider. */
export async function renderWithRouter(ui: () => React.ReactNode) {
  // jsdom here has neither; ThemeProvider reads both.
  const items = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => items.get(k) ?? null,
    setItem: (k: string, v: string) => void items.set(k, v),
    removeItem: (k: string) => void items.delete(k),
  })
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }))
  const rootRoute = createRootRoute({
    component: () => <ThemeProvider>{ui()}</ThemeProvider>,
  })
  const router = createRouter({
    routeTree: rootRoute,
    history: createMemoryHistory({ initialEntries: ['/'] }),
  })
  await router.load()
  return render(<RouterProvider router={router} />)
}
