import { useEffect, useState } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import {
  CaretDown,
  CaretLeft,
  CaretRight,
  FlowArrow,
  SignOut,
} from '@phosphor-icons/react'
import { ThemeToggle } from './theme-toggle'
import { MobileNav, MobileNavTrigger } from './mobile-nav'
import type {AuthUser} from '@/lib/auth';
import { useAppAuth } from '@/lib/auth-context'
import {
  
  DEV_MODE,
  getUserDisplayName,
  getUserInitials
} from '@/lib/auth'
import { navItems } from '@/lib/navigation'
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
    return localStorage.getItem('sidebar-collapsed') === 'true'
  })

  useEffect(() => {
    localStorage.setItem('sidebar-collapsed', String(collapsed))
  }, [collapsed])

  return [collapsed, setCollapsed] as const
}

export function AppShell({
  children,
  user: routeUser,
  devMode = DEV_MODE,
}: AppShellProps) {
  const { signOut } = useAppAuth()
  const user = routeUser

  const handleSignOut = signOut

  const location = useLocation()
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const [collapsed, setCollapsed] = useSidebarState()

  const sidebarWidth = collapsed ? 'w-16' : 'w-60'
  const mainMargin = collapsed ? 'md:ml-16' : 'md:ml-60'

  return (
    <div className="min-h-screen bg-background">
      {/* Desktop Sidebar */}
      <aside
        className={cn(
          'fixed inset-y-0 left-0 z-30 hidden flex-col border-r border-border bg-card/80 backdrop-blur-md transition-all duration-200 md:flex',
          sidebarWidth,
        )}
      >
        {/* Logo */}
        <div className="flex h-16 items-center gap-2 border-b border-border px-4">
          <Link to="/dashboard" className="flex items-center gap-2">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary">
              <FlowArrow
                className="size-5 text-primary-foreground"
                weight="bold"
              />
            </div>
            {!collapsed && (
              <span className="font-display text-lg font-bold">FlowTrace</span>
            )}
          </Link>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto px-2 py-4">
          <div className="flex flex-col gap-1">
            {navItems.map((item) => {
              const Icon = item.icon
              const isActive =
                location.pathname === item.href ||
                (item.href !== '/dashboard' &&
                  location.pathname.startsWith(item.href))

              return (
                <Link
                  key={item.href}
                  to={item.href}
                  className={cn(
                    'relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                    collapsed && 'justify-center px-0',
                    isActive
                      ? 'text-primary'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                  )}
                  title={collapsed ? item.label : undefined}
                >
                  {/* Active indicator bar */}
                  {isActive && (
                    <div className="absolute left-0 top-1/2 h-6 w-0.5 -translate-y-1/2 rounded-full bg-primary" />
                  )}
                  <Icon
                    className="size-4 shrink-0"
                    weight={isActive ? 'fill' : 'regular'}
                  />
                  {!collapsed && item.label}
                </Link>
              )
            })}
          </div>
        </nav>

        {/* Bottom section */}
        <div className="border-t border-border p-2">
          <div className={cn('flex items-center', collapsed ? 'flex-col gap-2' : 'gap-2')}>
            <ThemeToggle collapsed={collapsed} />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setCollapsed(!collapsed)}
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            >
              {collapsed ? (
                <CaretRight className="size-4" />
              ) : (
                <CaretLeft className="size-4" />
              )}
            </Button>
          </div>

          {/* User menu */}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <button
                  className={cn(
                    'mt-2 flex w-full items-center gap-2 rounded-lg p-2 text-sm transition-colors hover:bg-muted',
                    collapsed && 'justify-center',
                  )}
                >
                  <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-medium text-primary-foreground">
                    {getUserInitials(user ?? null)}
                  </div>
                  {!collapsed && (
                    <>
                      <span className="flex-1 truncate text-left text-sm font-medium">
                        {getUserDisplayName(user ?? null)}
                      </span>
                      <CaretDown className="size-3 shrink-0 text-muted-foreground" />
                    </>
                  )}
                </button>
              }
            />
            <DropdownMenuContent side="top" align="start" sideOffset={8}>
              <div className="px-2 py-2">
                <p className="text-sm font-medium">
                  {getUserDisplayName(user ?? null)}
                </p>
                {user?.email && (
                  <p className="text-xs text-muted-foreground">{user.email}</p>
                )}
                {devMode && (
                  <p className="text-xs text-muted-foreground">Dev Mode</p>
                )}
              </div>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => handleSignOut()}
                className="text-destructive focus:text-destructive"
              >
                <SignOut className="size-4" />
                Sign Out
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </aside>

      {/* Mobile Header */}
      <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b border-border bg-card/80 px-4 backdrop-blur-md md:hidden">
        <Link to="/dashboard" className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
            <FlowArrow
              className="size-5 text-primary-foreground"
              weight="bold"
            />
          </div>
          <span className="font-display text-lg font-bold">FlowTrace</span>
        </Link>
        <div className="flex items-center gap-2">
          <ThemeToggle />
          <MobileNavTrigger onClick={() => setMobileNavOpen(true)} />
        </div>
      </header>

      {/* Mobile Navigation Drawer */}
      <MobileNav
        isOpen={mobileNavOpen}
        onClose={() => setMobileNavOpen(false)}
      />

      {/* Main Content */}
      <main className={cn('transition-all duration-200', mainMargin)}>
        <div className="page-enter">{children}</div>
      </main>
    </div>
  )
}
