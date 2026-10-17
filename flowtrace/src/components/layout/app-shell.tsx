import { useEffect, useState } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import { CaretDown, CaretLeft, CaretRight, SignOut } from '@phosphor-icons/react'
import { ThemeToggle } from './theme-toggle'
import { MobileNav, MobileNavTrigger } from './mobile-nav'
import { Wordmark } from './wordmark'
import type { AuthUser } from '@/lib/auth'
import { useAppAuth } from '@/lib/auth-context'
import { DEV_MODE, getUserDisplayName, getUserInitials } from '@/lib/auth'
import { useOrg } from '@/lib/org-context'
import { DEMO_BOOKS, useMastheadOrgName } from '@/lib/ledger-client'
import { isNavItemActive, navSections } from '@/lib/navigation'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'

interface AppShellProps {
  children: React.ReactNode
  user?: AuthUser | null
  organizationId?: string | null
  devMode?: boolean
}

function useSidebarState() {
  const [collapsed, setCollapsed] = useState(() => {
    if (typeof window === 'undefined') return false
    try {
      return localStorage.getItem('sidebar-collapsed') === 'true'
    } catch {
      return false
    }
  })

  useEffect(() => {
    try {
      localStorage.setItem('sidebar-collapsed', String(collapsed))
    } catch {
      // Storage unavailable (private mode); keep the in-memory state.
    }
  }, [collapsed])

  return [collapsed, setCollapsed] as const
}

const mastheadButton =
  'text-masthead-foreground hover:bg-masthead-foreground/12 hover:text-masthead-foreground aria-expanded:bg-masthead-foreground/12 aria-expanded:text-masthead-foreground'

export function AppShell({
  children,
  user,
  devMode = DEV_MODE,
}: AppShellProps) {
  const { signOut } = useAppAuth()
  const { organization } = useOrg()
  const orgName = useMastheadOrgName(organization?.name)
  const location = useLocation()
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const [collapsed, setCollapsed] = useSidebarState()

  return (
    <div className="min-h-screen bg-background">
      {/* Masthead: the committed ink band carrying identity and context */}
      <header className="sticky top-0 z-40 bg-masthead text-masthead-foreground print:hidden">
        <div className="flex h-14 items-center gap-4 px-4 md:px-5">
          <Link to="/dashboard" aria-label="FlowTrace dashboard">
            <Wordmark />
          </Link>
          <span
            aria-hidden="true"
            className="hidden h-6 w-px bg-masthead-foreground/25 sm:block"
          />
          <p className="hidden min-w-0 truncate text-sm font-semibold sm:block">
            {orgName}
          </p>
          <div className="ml-auto flex items-center gap-1">
            <ThemeToggle className={mastheadButton} />
            <UserMenu
              user={user ?? null}
              devMode={devMode}
              onSignOut={() => signOut()}
            />
            <MobileNavTrigger
              onClick={() => setMobileNavOpen(true)}
              className={mastheadButton}
            />
          </div>
        </div>
        {devMode && (
          <div className="border-t border-masthead-foreground/15 bg-primary px-4 py-1.5 text-xs leading-4 text-primary-foreground md:truncate md:px-5">
            {DEMO_BOOKS
              ? "Demo: you're viewing Demo NGO Kenya. Changes reset when you reload. "
              : "Demo: you're viewing Demo NGO Kenya. The books are live in SpacetimeDB; other changes reset when you reload. "}
            <Link to="/" className="font-semibold underline">
              Back to the overview
            </Link>
          </div>
        )}
      </header>

      <div className="flex">
        {/* Desktop navigation rail */}
        <aside
          className={cn(
            'sticky hidden shrink-0 flex-col border-r border-sidebar-border bg-sidebar transition-[width] duration-200 md:flex print:hidden',
            devMode
              ? 'top-[5.25rem] h-[calc(100vh-5.25rem)]'
              : 'top-14 h-[calc(100vh-3.5rem)]',
            collapsed ? 'w-16' : 'w-56',
          )}
        >
          <nav aria-label="Main" className="flex-1 overflow-y-auto py-3">
            {navSections.map((section, i) => (
              <div key={section.title ?? i} className={cn(i > 0 && 'mt-3')}>
                {section.title &&
                  (collapsed ? (
                    <div className="mx-4 mb-3 border-t border-sidebar-border" />
                  ) : (
                    <p className="section-label mx-5 mb-1 text-[0.6875rem] text-muted-foreground">
                      {section.title}
                    </p>
                  ))}
                <ul className="flex flex-col">
                  {section.items.map((item) => {
                    const Icon = item.icon
                    const active = isNavItemActive(item.href, location.pathname)
                    return (
                      <li key={item.href}>
                        <Link
                          to={item.href}
                          aria-current={active ? 'page' : undefined}
                          title={collapsed ? item.label : undefined}
                          className={cn(
                            'mx-2 flex h-10 items-center gap-3 px-3 text-sm font-medium transition-colors',
                            collapsed && 'justify-center px-0',
                            active
                              ? 'bg-foreground text-background'
                              : 'text-sidebar-foreground hover:bg-sidebar-accent',
                          )}
                        >
                          <Icon
                            className="size-[1.125rem] shrink-0"
                            weight={active ? 'bold' : 'regular'}
                          />
                          {!collapsed && item.label}
                        </Link>
                      </li>
                    )
                  })}
                </ul>
              </div>
            ))}
          </nav>
          <div className="border-t border-sidebar-border p-2">
            <Button
              variant="ghost"
              size={collapsed ? 'icon' : 'sm'}
              onClick={() => setCollapsed(!collapsed)}
              aria-label={collapsed ? 'Expand navigation' : 'Collapse navigation'}
              className={cn(!collapsed && 'w-full justify-start')}
            >
              {collapsed ? (
                <CaretRight className="size-4" />
              ) : (
                <>
                  <CaretLeft className="size-4" />
                  Collapse
                </>
              )}
            </Button>
          </div>
        </aside>

        <MobileNav
          isOpen={mobileNavOpen}
          onClose={() => setMobileNavOpen(false)}
        />

        <main className="min-w-0 flex-1">
          <div className="page-enter">{children}</div>
        </main>
      </div>
    </div>
  )
}

function UserMenu({
  user,
  devMode,
  onSignOut,
}: {
  user: AuthUser | null
  devMode: boolean
  onSignOut: () => void
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <button
            type="button"
            className={cn(
              'flex h-9 items-center gap-2 px-2 text-sm transition-colors',
              mastheadButton,
            )}
            aria-label="Account menu"
          >
            <span className="flex size-7 items-center justify-center bg-masthead-foreground text-xs font-bold text-masthead">
              {getUserInitials(user)}
            </span>
            <span className="hidden max-w-36 truncate font-medium lg:inline">
              {getUserDisplayName(user)}
            </span>
            <CaretDown className="size-3 shrink-0 opacity-70" />
          </button>
        }
      />
      <DropdownMenuContent align="end" sideOffset={8}>
        <div className="px-2 py-2">
          <p className="text-sm font-semibold">{getUserDisplayName(user)}</p>
          {user?.email && (
            <p className="text-xs text-muted-foreground">{user.email}</p>
          )}
          {devMode && (
            <p className="mt-1 text-xs text-muted-foreground">Demo account</p>
          )}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={onSignOut}>
          <SignOut className="size-4" />
          {devMode ? 'Leave the demo' : 'Sign out'}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
