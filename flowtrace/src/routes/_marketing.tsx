import { Outlet, createFileRoute, useMatches } from '@tanstack/react-router'
import SitemapFooter from '@/components/kokonutui-pro/sitemap-footer'
import {
  MarketingMasthead,
  demoStarted,
} from '@/components/marketing/marketing-masthead'
import { useAppAuth } from '@/lib/auth-context'
import {
  DEMO_FIGURES_AS_OF,
  DEMO_PATH,
  FOOTER_COLUMNS,
  GITHUB_REPO_URL,
} from '@/lib/marketing-nav'

declare module '@tanstack/react-router' {
  interface StaticDataRouteOption {
    /** The page shows demo figures; the footer then dates them. */
    demoFigures?: boolean
  }
}

/** Pathless layout for the public marketing pages: masthead, main, footer. */
export const Route = createFileRoute('/_marketing')({
  component: MarketingLayout,
})

export function MarketingLayout() {
  const { user } = useAppAuth()
  const demoFigures = useMatches({
    select: (matches) => matches.some((m) => m.staticData.demoFigures),
  })

  return (
    <MarketingFrame signedIn={!!user} demoFigures={demoFigures}>
      <Outlet />
    </MarketingFrame>
  )
}

export function MarketingFrame({
  signedIn,
  demoFigures,
  children,
}: {
  signedIn: boolean
  demoFigures: boolean
  children: React.ReactNode
}) {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:bg-background focus:px-4 focus:py-2 focus:outline-2 focus:outline-ring"
      >
        Skip to content
      </a>
      <MarketingMasthead signedIn={signedIn} />
      <main id="main" tabIndex={-1} className="outline-none">
        {children}
      </main>
      <SitemapFooter
        columns={FOOTER_COLUMNS}
        demoPath={DEMO_PATH}
        onDemo={() => demoStarted('footer')}
        githubUrl={GITHUB_REPO_URL}
        demoAsOf={demoFigures ? DEMO_FIGURES_AS_OF : undefined}
      />
    </div>
  )
}
