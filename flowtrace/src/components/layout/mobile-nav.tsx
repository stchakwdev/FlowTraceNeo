import { useEffect } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import { List, X } from '@phosphor-icons/react'
import { isNavItemActive, navItems } from '@/lib/navigation'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface MobileNavProps {
  isOpen: boolean
  onClose: () => void
}

export function MobileNav({ isOpen, onClose }: MobileNavProps) {
  const location = useLocation()

  useEffect(() => {
    onClose()
  }, [location.pathname, onClose])

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
          'fixed inset-0 z-40 bg-masthead/40 transition-opacity duration-300 md:hidden',
          isOpen ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Drawer */}
      <nav
        className={cn(
          'fixed top-0 right-0 z-50 h-full w-72 border-l-[3px] border-foreground bg-background shadow-[-8px_0_24px_-12px_rgb(0_0_0/0.35)] transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] md:hidden',
          isOpen ? 'translate-x-0' : 'translate-x-full',
        )}
        aria-label="Mobile navigation"
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

        <div className="flex flex-col py-2">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = isNavItemActive(item.href, location.pathname)

            return (
              <Link
                key={item.href}
                to={item.href}
                aria-current={isActive ? 'page' : undefined}
                className={cn(
                  'mx-2 flex h-12 items-center gap-3 px-3 text-base font-medium transition-colors',
                  isActive
                    ? 'bg-foreground text-background'
                    : 'text-foreground hover:bg-muted',
                )}
              >
                <Icon
                  className="size-5"
                  weight={isActive ? 'bold' : 'regular'}
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

export function MobileNavTrigger({
  onClick,
  className,
}: {
  onClick: () => void
  className?: string
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={onClick}
      className={cn('md:hidden', className)}
      aria-label="Open menu"
    >
      <List className="size-5" />
    </Button>
  )
}
