import { createFileRoute, redirect } from '@tanstack/react-router'
import { signOut } from '@workos/authkit-tanstack-react-start'
import { DEV_MODE } from '@/lib/auth'

export const Route = createFileRoute('/sign-out')({
  loader: async () => {
    if (!DEV_MODE) await signOut()
    throw redirect({ to: '/' })
  },
})
