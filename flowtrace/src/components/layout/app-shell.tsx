import { useState } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import {
  CaretDown,
  CurrencyCircleDollar,
  FlowArrow,
  Gear,
  House,
  LinkSimple,
  Megaphone,
  ShieldCheck,
  SignOut,
} from '@phosphor-icons/react'
import { ThemeToggle } from './theme-toggle'
import { MobileNav, MobileNavTrigger } from './mobile-nav'
import { getUserDisplayName, getUserInitials } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'

const navItems = [
  { label: 'Dashboard', href: '/dashboard', icon: House },
  { label: 'Transactions', href: '/transactions', icon: CurrencyCircleDollar },
  { label: 'Flows', href: '/flows', icon: FlowArrow },
  { label: 'Campaigns', href: '/campaigns', icon: Megaphone },
  { label: 'Chains', href: '/chains', icon: LinkSimple },
  { label: 'Trust Score', href: '/trust-score', icon: ShieldCheck },
  { label: 'Settings', href: '/settings', icon: Gear },
]

interface AppShellProps {
  children: React.ReactNode
}

export function AppShell({ children }: AppShellProps) {
  // Dev mode - no WorkOS auth
  const user = null
  const signOut = () => {
    window.location.href = '/'
  }
  const location = useLocation()
  const [mobileNavOpen, setMobileNavOpen] = useState(false)

  // Dev mode is active when WorkOS is not configured
  const DEV_BYPASS_AUTH = true

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-30 border-b border-border bg-card/80 backdrop-blur-md">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
          {/* Logo and Nav */}
          <div className="flex items-center gap-6">
            <Link to="/dashboard" className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
                <FlowArrow
                  className="size-5 text-primary-foreground"
                  weight="bold"
                />
              </div>
              <span className="text-xl font-bold">FlowTrace</span>
            </Link>

            {/* Desktop Navigation */}
            <nav className="hidden items-center gap-1 md:flex">
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
                      'flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                      isActive
                        ? 'bg-accent text-foreground'
                        : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                    )}
                  >
                    <Icon
                      className="size-4"
                      weight={isActive ? 'fill' : 'regular'}
                    />
                    {item.label}
                  </Link>
                )
              })}
            </nav>
          </div>

          {/* Right side: Theme Toggle, User Menu, Mobile Nav Trigger */}
          <div className="flex items-center gap-2">
            <ThemeToggle />

            {/* User Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    variant="ghost"
                    className="hidden items-center gap-2 sm:flex"
                  >
                    <div className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-xs font-medium text-primary-foreground">
                      {getUserInitials(user as any)}
                    </div>
                    <span className="text-sm font-medium">
                      {getUserDisplayName(user as any)}
                    </span>
                    <CaretDown className="size-3 text-muted-foreground" />
                  </Button>
                }
              />
              <DropdownMenuContent align="end" sideOffset={8}>
                <div className="px-2 py-2">
                  <p className="text-sm font-medium">
                    {getUserDisplayName(user as any)}
                  </p>
                  {user?.email && (
                    <p className="text-xs text-muted-foreground">
                      {user.email}
                    </p>
                  )}
                  {DEV_BYPASS_AUTH && (
                    <p className="text-xs text-muted-foreground">Dev Mode</p>
                  )}
                </div>
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                  <Link to="/settings">
                    <Gear className="size-4" />
                    Settings
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={() => signOut()}
                  className="text-destructive focus:text-destructive"
                >
                  <SignOut className="size-4" />
                  Sign Out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Mobile: Compact User Avatar */}
            <Button
              variant="ghost"
              size="icon"
              className="flex sm:hidden"
              onClick={() => signOut()}
              aria-label="Sign out"
            >
              <div className="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-xs font-medium text-primary-foreground">
                {getUserInitials(user as any)}
              </div>
            </Button>

            {/* Mobile Nav Trigger */}
            <MobileNavTrigger onClick={() => setMobileNavOpen(true)} />
          </div>
        </div>
      </header>

      {/* Mobile Navigation Drawer */}
      <MobileNav
        isOpen={mobileNavOpen}
        onClose={() => setMobileNavOpen(false)}
      />

      {/* Main Content */}
      <main>{children}</main>
    </div>
  )
}
