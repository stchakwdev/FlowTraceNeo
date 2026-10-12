import { Link } from '@tanstack/react-router'
import { buttonVariants } from '@/components/ui/button'

interface AuthErrorProps {
  error?: string
  message?: string
}

const errorMessages: Record<string, string> = {
  access_denied:
    'Sign-in was cancelled or access was refused. Try signing in again.',
  invalid_request:
    'The sign-in request was incomplete. Start again from the sign-in page.',
  server_error:
    'The sign-in service had a problem on its side. Wait a minute and try again.',
  session_expired: 'Your session has ended. Sign in again to carry on.',
  unauthorized:
    'Your account doesn’t have access to this page. Ask an admin in your organisation to add you to a group.',
  invalid_token: 'Your sign-in link is no longer valid. Sign in again.',
}

export function AuthError({ error, message }: AuthErrorProps) {
  const displayMessage =
    (error ? errorMessages[error] : undefined) ??
    message ??
    'Something went wrong while signing you in. Try again, and if it keeps happening, contact your FlowTrace admin.'

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
      <div role="alert" className="rule-heavy w-full max-w-md pt-6">
        <h1 className="text-4xl">We couldn’t sign you in</h1>
        <p className="mt-3 text-base text-muted-foreground">{displayMessage}</p>
        {error && (
          <p className="mt-2 text-xs text-muted-foreground">
            Reference: <span className="font-mono">{error}</span>
          </p>
        )}
        <div className="mt-8 flex flex-wrap gap-3">
          <Link
            to="/dashboard"
            className={buttonVariants({ className: 'h-10' })}
          >
            Try again
          </Link>
          <Link
            to="/"
            className={buttonVariants({
              variant: 'outline',
              className: 'h-10',
            })}
          >
            Go to the home page
          </Link>
        </div>
      </div>
    </main>
  )
}
