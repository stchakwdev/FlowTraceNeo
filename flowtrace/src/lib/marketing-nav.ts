/**
 * Navigation for the public marketing pages: the masthead links, the footer
 * columns and the two calls to action. Several targets arrive in later LP
 * threads; until then they are plain anchors to the future path.
 */

export interface NavLink {
  label: string
  href: string
  external?: boolean
}

/** "Explore the demo" opens the app as the demo organisation. */
export const DEMO_PATH = '/dashboard'

/**
 * AUTH_ROUTES.signIn in lib/auth.ts is '/sign-in', but no such route exists
 * on main yet, so "Sign in" goes to /dashboard, which signs a visitor in
 * when WorkOS is on and opens the demo when it is off.
 */
export const SIGN_IN_PATH = '/dashboard'

/** Date of the bundled demo figures; equals DEMO_AS_OF in lib/demo/ledger.ts (tested). */
export const DEMO_FIGURES_AS_OF = '2026-02-04'

export const MASTHEAD_NAV: ReadonlyArray<NavLink> = [
  { label: 'Product', href: '/#product' },
  { label: 'Books', href: '/books' },
  { label: 'For funders', href: '/funders' },
  { label: 'Pricing', href: '/pricing' },
  { label: 'Security', href: '/security' },
  { label: 'Docs', href: '/docs' },
  { label: 'Blog', href: '/blog' },
  { label: 'Changelog', href: '/changelog' },
]

export const GITHUB_REPO_URL = 'https://github.com/stchakwdev/FlowTraceNeo'

export const FOOTER_COLUMNS: ReadonlyArray<{
  title: string
  links: ReadonlyArray<NavLink>
}> = [
  {
    title: 'Product',
    links: [
      { label: 'Overview', href: '/#product' },
      { label: 'Books', href: '/books' },
      { label: 'For funders', href: '/funders' },
      { label: 'Pricing', href: '/pricing' },
    ],
  },
  {
    title: 'Resources',
    links: [
      { label: 'Docs', href: '/docs' },
      { label: 'Blog', href: '/blog' },
      { label: 'Changelog', href: '/changelog' },
      { label: 'GitHub repository', href: GITHUB_REPO_URL, external: true },
    ],
  },
  {
    title: 'Company',
    links: [
      { label: 'About', href: '/about' },
      { label: 'Contact', href: '/about#contact' },
      { label: 'Security', href: '/security' },
    ],
  },
  {
    title: 'Legal',
    links: [
      { label: 'Privacy', href: '/privacy' },
      { label: 'Terms', href: '/terms' },
    ],
  },
]
