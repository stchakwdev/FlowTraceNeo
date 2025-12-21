import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  ChartLineUp,
  FlowArrow,
  ShieldCheck,
  UsersThree,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { ThemeToggle } from '@/components/layout/theme-toggle'

// Check if WorkOS is configured - always bypass in current setup
const DEV_BYPASS_AUTH = !import.meta.env.VITE_WORKOS_CLIENT_ID

export const Route = createFileRoute('/')({
  component: LandingPage,
})

const features = [
  {
    icon: FlowArrow,
    title: 'Track Money Flows',
    description:
      'Visualize how funds move through your organization with interactive flow diagrams and real-time tracking.',
  },
  {
    icon: ShieldCheck,
    title: 'Build Trust Scores',
    description:
      'Establish credibility with transparent, verifiable financial records that stakeholders can trust.',
  },
  {
    icon: ChartLineUp,
    title: 'Real-time Analytics',
    description:
      'Get instant insights into your financial health with comprehensive dashboards and reporting.',
  },
  {
    icon: UsersThree,
    title: 'Stakeholder Reports',
    description:
      'Generate professional reports for donors, board members, and regulatory bodies with ease.',
  },
]

function LandingPage() {
  // In dev bypass mode, always show as authenticated
  const isAuthenticated = DEV_BYPASS_AUTH

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
          <div className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
              <FlowArrow
                className="size-5 text-primary-foreground"
                weight="bold"
              />
            </div>
            <span className="text-xl font-bold">FlowTrace</span>
          </div>

          <div className="flex items-center gap-3">
            <ThemeToggle />
            <Link to="/dashboard">
              <Button>
                {isAuthenticated ? 'Go to Dashboard' : 'Sign In'}
                <ArrowRight className="size-4" data-icon="inline-end" />
              </Button>
            </Link>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="border-b border-border">
        <div className="mx-auto max-w-7xl px-4 py-24 text-center">
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
            Financial Transparency
            <br />
            <span className="text-primary">Made Verifiable</span>
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            FlowTrace helps NGOs, nonprofits, and social enterprises build trust
            through transparent financial tracking. Visualize fund flows,
            generate trust scores, and provide stakeholders with verifiable
            records.
          </p>
          <div className="mt-10 flex items-center justify-center gap-4">
            <Link to="/dashboard">
              <Button size="lg">
                {isAuthenticated ? 'Open Dashboard' : 'Get Started'}
                <ArrowRight className="size-4" data-icon="inline-end" />
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="py-24">
        <div className="mx-auto max-w-7xl px-4">
          <div className="text-center">
            <h2 className="text-3xl font-bold">
              Everything you need for financial transparency
            </h2>
            <p className="mt-4 text-muted-foreground">
              Comprehensive tools to track, verify, and report your
              organization's financial activities.
            </p>
          </div>

          <div className="mt-16 grid gap-8 sm:grid-cols-2 lg:grid-cols-4">
            {features.map((feature) => {
              const Icon = feature.icon
              return (
                <div
                  key={feature.title}
                  className="rounded-lg border border-border bg-card p-6 transition-colors hover:border-primary/50"
                >
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <Icon className="size-5 text-primary" weight="duotone" />
                  </div>
                  <h3 className="mt-4 font-semibold">{feature.title}</h3>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {feature.description}
                  </p>
                </div>
              )
            })}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="border-t border-border bg-card py-16">
        <div className="mx-auto max-w-7xl px-4 text-center">
          <h2 className="text-2xl font-bold">Ready to build trust?</h2>
          <p className="mt-2 text-muted-foreground">
            Start tracking your financial flows today.
          </p>
          <div className="mt-8">
            <Link to="/dashboard">
              <Button size="lg">
                {isAuthenticated ? 'Go to Dashboard' : 'Get Started Free'}
                <ArrowRight className="size-4" data-icon="inline-end" />
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border py-8">
        <div className="mx-auto max-w-7xl px-4 text-center text-sm text-muted-foreground">
          &copy; {new Date().getFullYear()} FlowTrace. All rights reserved.
        </div>
      </footer>
    </div>
  )
}
