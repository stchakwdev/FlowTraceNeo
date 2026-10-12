import { Link } from '@tanstack/react-router'
import { Warning } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'

interface AuthErrorProps {
  error?: string
  message?: string
}

const errorMessages: Record<string, string> = {
  access_denied: 'Access was denied. Please try again.',
  invalid_request: 'Invalid authentication request.',
  server_error: 'Authentication server error. Please try again later.',
  session_expired: 'Your session has expired. Please sign in again.',
  unauthorized: 'You are not authorized to access this resource.',
  invalid_token: 'Invalid authentication token. Please sign in again.',
}

export function AuthError({ error, message }: AuthErrorProps) {
  const displayMessage = error
    ? errorMessages[error] ?? message ?? 'An authentication error occurred.'
    : message ?? 'An authentication error occurred.'

  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <div className="text-center">
        <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-destructive/10">
          <Warning className="h-8 w-8 text-destructive" weight="fill" />
        </div>
        <h1 className="mt-4 text-2xl font-bold">Authentication Error</h1>
        <p className="mt-2 max-w-md text-muted-foreground">{displayMessage}</p>
        <div className="mt-6 flex justify-center gap-4">
          <Link to="/">
            <Button variant="outline">Go Home</Button>
          </Link>
          <Link to="/dashboard">
            <Button>Try Again</Button>
          </Link>
        </div>
      </div>
    </div>
  )
}
