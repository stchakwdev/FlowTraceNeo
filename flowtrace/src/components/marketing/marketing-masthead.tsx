import { useEffect, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { List, X } from '@phosphor-icons/react'
import { AsOf } from '@/components/report'
import { ThemeToggle } from '@/components/layout/theme-toggle'
import { Wordmark } from '@/components/layout/wordmark'
import { Button, buttonVariants } from '@/components/ui/button'
import { track } from '@/lib/analytics'
import {
  DEMO_FIGURES_AS_OF,
  DEMO_PATH,
  MASTHEAD_NAV,
  SIGN_IN_PATH,
} from '@/lib/marketing-nav'
import { cn } from '@/lib/utils'

/** "Explore the demo": counts as a CTA click and as the demo starting. */
export function demoStarted(location: string) {
  track('cta_click', { location })
  track('demo_started', { location })
}

const NAV_LINK =
  'px-2 py-1 text-sm underline-offset-4 hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring'

/** The masthead of the public pages: wordmark, dateline, nav and the two CTAs. */
export function MarketingMasthead({ signedIn }: { signedIn: boolean }) {
  const [open, setOpen] = useState(false)

  return (
    <header className="sticky top-0 z-40 border-b-[3px] border-foreground bg-background">
      <div className="mx-auto flex h-14 max-w-7xl items-center gap-6 px-4 sm:px-8">
        <Link to="/" aria-label="FlowTrace home">
          <Wordmark />
        </Link>
        <p className="hidden text-xs tracking-[0.14em] text-muted-foreground uppercase xl:block font-narrow">
          Situation report · Demo NGO Kenya ·{' '}
          <AsOf date={DEMO_FIGURES_AS_OF} className="text-xs" />
        </p>
        <div className="ml-auto flex items-center gap-1 sm:gap-3">
          <ThemeToggle className="hidden sm:inline-flex" />
          <Link
            to={SIGN_IN_PATH}
            onClick={() => track('cta_click', { location: 'masthead_sign_in' })}
            className="hidden px-2 py-1 text-sm whitespace-nowrap underline decoration-foreground/30 underline-offset-4 hover:decoration-foreground sm:inline"
          >
            {signedIn ? 'Open app' : 'Sign in'}
          </Link>
          <Link
            to={DEMO_PATH}
            onClick={() => demoStarted('masthead')}
            className={buttonVariants({ size: 'sm' })}
          >
            Explore the demo
          </Link>
          <Button
            variant="ghost"
            size="icon"
            className="lg:hidden"
            aria-label="Open menu"
            aria-expanded={open}
            aria-controls="marketing-drawer"
            onClick={() => setOpen(true)}
          >
            <List className="size-5" />
          </Button>
        </div>
      </div>
      <nav aria-label="Main" className="hidden border-t border-border lg:block">
        <ul className="mx-auto flex h-10 max-w-7xl items-center gap-2 px-6">
          {MASTHEAD_NAV.map((item) => (
            <li key={item.href}>
              <a href={item.href} className={NAV_LINK}>
                {item.label}
              </a>
            </li>
          ))}
        </ul>
      </nav>
      <MarketingDrawer
        open={open}
        onClose={() => setOpen(false)}
        signedIn={signedIn}
      />
    </header>
  )
}

/** Below lg the nav moves into a drawer, after the app's mobile-nav pattern. */
function MarketingDrawer({
  open,
  onClose,
  signedIn,
}: {
  open: boolean
  onClose: () => void
  signedIn: boolean
}) {
  const drawerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    drawerRef.current?.querySelector<HTMLButtonElement>('button')?.focus()
    document.body.style.overflow = 'hidden'
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => {
      document.body.style.overflow = ''
      document.removeEventListener('keydown', onKey)
    }
  }, [open, onClose])

  return (
    <>
      <div
        className={cn(
          'fixed inset-0 z-40 bg-masthead/40 transition-opacity duration-200 motion-reduce:transition-none lg:hidden',
          open ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        onClick={onClose}
        aria-hidden="true"
      />
      <div
        id="marketing-drawer"
        ref={drawerRef}
        role="dialog"
        aria-modal="true"
        aria-label="Menu"
        inert={!open}
        className={cn(
          'fixed top-0 right-0 z-50 flex h-full w-72 flex-col border-l-[3px] border-foreground bg-background shadow-[-8px_0_24px_-12px_rgb(0_0_0/0.35)] transition-transform duration-200 motion-reduce:transition-none lg:hidden',
          open ? 'translate-x-0' : 'translate-x-full',
        )}
      >
        <div className="flex h-14 items-center justify-between border-b border-border px-4">
          <span className="section-label">Menu</span>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close menu"
          >
            <X className="size-5" />
          </Button>
        </div>
        <nav aria-label="Main, mobile" className="flex-1 overflow-y-auto py-2">
          <ul>
            {MASTHEAD_NAV.map((item) => (
              <li key={item.href}>
                <a
                  href={item.href}
                  onClick={onClose}
                  className="flex min-h-11 items-center px-5 text-base hover:bg-muted focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
                >
                  {item.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
        <div className="flex flex-col gap-3 border-t border-border p-4">
          <Link
            to={DEMO_PATH}
            onClick={() => demoStarted('drawer')}
            className={buttonVariants()}
          >
            Explore the demo
          </Link>
          <div className="flex items-center justify-between">
            <Link
              to={SIGN_IN_PATH}
              onClick={() => track('cta_click', { location: 'drawer_sign_in' })}
              className="py-2 text-sm underline decoration-foreground/30 underline-offset-4 hover:decoration-foreground"
            >
              {signedIn ? 'Open app' : 'Sign in'}
            </Link>
            <ThemeToggle />
          </div>
        </div>
      </div>
    </>
  )
}
