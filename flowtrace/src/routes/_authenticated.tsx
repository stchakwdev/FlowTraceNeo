import { createFileRoute, Outlet } from '@tanstack/react-router'
import { useAuth } from '@workos-inc/authkit-react'
import { AppShell } from '@/components/layout/app-shell'

// Bypass auth in development when WorkOS is not configured
const DEV_BYPASS_AUTH = !import.meta.env.VITE_WORKOS_CLIENT_ID

export const Route = createFileRoute('/_authenticated')({
  component: AuthenticatedLayout,
})

function AuthenticatedLayout() {
  const { isLoading, user, signIn } = useAuth()

  // In dev mode without WorkOS, bypass auth
  if (DEV_BYPASS_AUTH) {
    return (
      <AppShell>
        <Outlet />
      </AppShell>
    )
  }

  // Show loading state while checking auth
  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>
      </div>
    )
  }

  // Redirect to sign in if not authenticated
  if (!user) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="flex flex-col items-center gap-4 text-center">
          <h1 className="text-2xl font-bold">Sign in required</h1>
          <p className="text-muted-foreground">
            You need to sign in to access this page.
          </p>
          <button
            onClick={() => signIn()}
            className="rounded-lg bg-primary px-6 py-2 font-medium text-primary-foreground hover:bg-primary/90"
          >
            Sign In
          </button>
        </div>
      </div>
    )
  }

  // Render authenticated content
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  )
}
