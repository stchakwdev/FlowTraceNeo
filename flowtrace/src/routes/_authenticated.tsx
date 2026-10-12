import { Outlet, createFileRoute, redirect } from '@tanstack/react-router'
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'
import { AppShell } from '@/components/layout/app-shell'
import { DEV_MODE, DUMMY_USER, mapWorkOSUser } from '@/lib/auth'

export const Route = createFileRoute('/_authenticated')({
  // Use loader for server-side auth checks (not beforeLoad)
  loader: async ({ location }) => {
    // Dummy auth mode: always signed in as the demo user
    if (DEV_MODE) {
      return {
        user: DUMMY_USER,
        organizationId: null,
        devMode: true,
      }
    }

    // Production: Check auth
    const auth = await getAuth()

    if (!auth.user) {
      // Redirect to WorkOS sign-in with return URL
      const signInUrl = await getSignInUrl({
        data: { returnPathname: location.pathname },
      })
      throw redirect({ href: signInUrl })
    }

    return {
      user: mapWorkOSUser(auth.user, auth.organizationId),
      organizationId: auth.organizationId ?? null,
      devMode: false,
    }
  },
  component: AuthenticatedLayout,
})

function AuthenticatedLayout() {
  const { user, organizationId, devMode } = Route.useLoaderData()

  return (
    <AppShell user={user} organizationId={organizationId} devMode={devMode}>
      <Outlet />
    </AppShell>
  )
}
