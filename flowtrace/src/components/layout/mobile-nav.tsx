import { useEffect } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import {
  CurrencyCircleDollar,
  FlowArrow,
  Gear,
  House,
  LinkSimple,
  List,
  Megaphone,
  ShieldCheck,
  X,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
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

interface MobileNavProps {
  isOpen: boolean
  onClose: () => void
}

export function MobileNav({ isOpen, onClose }: MobileNavProps) {
  const location = useLocation()

  // Close nav on route change
  useEffect(() => {
    onClose()
  }, [location.pathname, onClose])

  // Prevent body scroll when nav is open
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => {
      document.body.style.overflow = ''
    }
  }, [isOpen])

  return (
    <>
      {/* Backdrop */}
      <div
        className={cn(
          'fixed inset-0 z-40 bg-background/80 backdrop-blur-sm transition-opacity duration-300 md:hidden',
          isOpen ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Drawer */}
      <nav
        className={cn(
          'fixed right-0 top-0 z-50 h-full w-72 bg-card shadow-xl transition-transform duration-300 ease-in-out md:hidden',
          isOpen ? 'translate-x-0' : 'translate-x-full',
        )}
        aria-label="Mobile navigation"
      >
        <div className="flex h-16 items-center justify-between border-b border-border px-4">
          <span className="text-lg font-semibold">Menu</span>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close menu"
          >
            <X className="size-5" />
          </Button>
        </div>

        <div className="flex flex-col gap-1 p-4">
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
                  'flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-medium transition-colors',
                  isActive
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                )}
              >
                <Icon
                  className="size-5"
                  weight={isActive ? 'fill' : 'regular'}
                />
                {item.label}
              </Link>
            )
          })}
        </div>
      </nav>
    </>
  )
}

export function MobileNavTrigger({ onClick }: { onClick: () => void }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={onClick}
      className="md:hidden"
      aria-label="Open menu"
    >
      <List className="size-5" />
    </Button>
  )
}
