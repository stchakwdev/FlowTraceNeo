import { Outlet, createFileRoute } from '@tanstack/react-router'
import { AppShell } from '@/components/layout/app-shell'

export const Route = createFileRoute('/_authenticated')({
  component: AuthenticatedLayout,
})

function AuthenticatedLayout() {
  // In dev mode (WorkOS not configured), always render the shell
  // When WorkOS is added, this will handle auth checks
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  )
}
