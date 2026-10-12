import { Monitor, Moon, Sun } from '@phosphor-icons/react'
import { useTheme } from '@/lib/theme'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

interface ThemeToggleProps {
  collapsed?: boolean
  className?: string
}

export function ThemeToggle({ collapsed, className }: ThemeToggleProps) {
  const { theme, setTheme, resolvedTheme } = useTheme()

  // When collapsed in sidebar, cycle on click instead of dropdown
  if (collapsed) {
    const cycle = () => {
      const order = ['light', 'dark', 'system'] as const
      const idx = order.indexOf(theme as (typeof order)[number])
      setTheme(order[(idx + 1) % order.length])
    }

    return (
      <Button
        variant="ghost"
        size="icon"
        onClick={cycle}
        aria-label="Toggle theme"
        className={className}
      >
        {resolvedTheme === 'dark' ? (
          <Moon className="size-4" weight="fill" />
        ) : (
          <Sun className="size-4" weight="fill" />
        )}
      </Button>
    )
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            aria-label="Toggle theme"
            className={className}
          >
            {resolvedTheme === 'dark' ? (
              <Moon className="size-4" weight="fill" />
            ) : (
              <Sun className="size-4" weight="fill" />
            )}
          </Button>
        }
      />
      <DropdownMenuContent align="end" sideOffset={8}>
        <DropdownMenuItem
          onClick={() => setTheme('light')}
          className={theme === 'light' ? 'bg-muted' : ''}
        >
          <Sun className="size-4" weight="regular" />
          Light
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() => setTheme('dark')}
          className={theme === 'dark' ? 'bg-muted' : ''}
        >
          <Moon className="size-4" weight="regular" />
          Dark
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() => setTheme('system')}
          className={theme === 'system' ? 'bg-muted' : ''}
        >
          <Monitor className="size-4" weight="regular" />
          System
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
