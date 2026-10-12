import { createStart } from '@tanstack/react-start'
import { authkitMiddleware } from '@workos/authkit-tanstack-react-start'
import { AUTH_MODE } from '@/lib/auth'

export const startInstance = createStart(() => ({
  requestMiddleware: AUTH_MODE === 'workos' ? [authkitMiddleware()] : [],
}))
