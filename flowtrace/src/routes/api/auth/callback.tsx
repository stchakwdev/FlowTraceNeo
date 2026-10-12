import { createFileRoute } from '@tanstack/react-router'
import { handleCallbackRoute } from '@workos/authkit-tanstack-react-start'
import { DEV_MODE } from '@/lib/auth'

export const Route = createFileRoute('/api/auth/callback')({
  server: {
    handlers: {
      // No WorkOS round-trip in dummy auth mode, so just go to the app
      GET: DEV_MODE
        ? () =>
            new Response(null, {
              status: 302,
              headers: { Location: '/dashboard' },
            })
        : handleCallbackRoute(),
    },
  },
})
