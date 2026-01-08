import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  ChartLineUp,
  CheckCircle,
  FlowArrow,
  Globe,
  ShieldCheck,
  Sparkle,
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
    color: 'text-blue-600 dark:text-blue-400',
    bg: 'bg-blue-100 dark:bg-blue-900/30',
  },
  {
    icon: ShieldCheck,
    title: 'Build Trust Scores',
    description:
      'Establish credibility with transparent, verifiable financial records that stakeholders can trust.',
    color: 'text-emerald-600 dark:text-emerald-400',
    bg: 'bg-emerald-100 dark:bg-emerald-900/30',
  },
  {
    icon: ChartLineUp,
    title: 'Real-time Analytics',
    description:
      'Get instant insights into your financial health with comprehensive dashboards and reporting.',
    color: 'text-violet-600 dark:text-violet-400',
    bg: 'bg-violet-100 dark:bg-violet-900/30',
  },
  {
    icon: UsersThree,
    title: 'Stakeholder Reports',
    description:
      'Generate professional reports for donors, board members, and regulatory bodies with ease.',
    color: 'text-amber-600 dark:text-amber-400',
    bg: 'bg-amber-100 dark:bg-amber-900/30',
  },
]

const stats = [
  { value: '100+', label: 'Organizations' },
  { value: '$50M+', label: 'Tracked' },
  { value: '99.9%', label: 'Uptime' },
  { value: '15+', label: 'Countries' },
]

function LandingPage() {
  // In dev bypass mode, always show as authenticated
  const isAuthenticated = DEV_BYPASS_AUTH

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-50 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-primary/70 shadow-lg shadow-primary/20">
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
      <section className="relative overflow-hidden border-b border-border">
        {/* Background decoration */}
        <div className="absolute inset-0 -z-10">
          <div className="absolute left-1/2 top-0 -translate-x-1/2 -translate-y-1/2">
            <div className="h-[600px] w-[600px] rounded-full bg-gradient-to-br from-primary/20 to-transparent blur-3xl" />
          </div>
          <div className="absolute bottom-0 right-0 translate-x-1/4 translate-y-1/4">
            <div className="h-[400px] w-[400px] rounded-full bg-gradient-to-tl from-primary/10 to-transparent blur-3xl" />
          </div>
        </div>

        <div className="mx-auto max-w-7xl px-4 py-24 text-center lg:py-32">
          {/* Badge */}
          <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-primary/20 bg-primary/5 px-4 py-1.5 text-sm font-medium text-primary">
            <Sparkle className="h-4 w-4" weight="fill" />
            Trusted by 100+ organizations across Africa
          </div>

          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl lg:text-7xl">
            Financial Transparency
            <br />
            <span className="bg-gradient-to-r from-primary via-primary/80 to-primary bg-clip-text text-transparent">
              Made Verifiable
            </span>
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground md:text-xl">
            FlowTrace helps NGOs, nonprofits, and social enterprises build trust
            through transparent financial tracking. Visualize fund flows,
            generate trust scores, and provide stakeholders with verifiable
            records.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Link to="/dashboard">
              <Button size="lg" className="group">
                {isAuthenticated ? 'Open Dashboard' : 'Get Started Free'}
                <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" data-icon="inline-end" />
              </Button>
            </Link>
            <Button variant="outline" size="lg" className="gap-2">
              <Globe className="size-4" />
              View Demo
            </Button>
          </div>

          {/* Trust indicators */}
          <div className="mt-12 flex flex-wrap items-center justify-center gap-x-8 gap-y-3 text-sm text-muted-foreground">
            <div className="flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-emerald-500" weight="fill" />
              No credit card required
            </div>
            <div className="flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-emerald-500" weight="fill" />
              Free tier available
            </div>
            <div className="flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-emerald-500" weight="fill" />
              SOC 2 compliant
            </div>
          </div>
        </div>
      </section>

      {/* Stats Section */}
      <section className="border-b border-border bg-card/50">
        <div className="mx-auto max-w-7xl px-4 py-12">
          <div className="grid grid-cols-2 gap-8 md:grid-cols-4">
            {stats.map((stat) => (
              <div key={stat.label} className="text-center">
                <p className="text-3xl font-bold text-primary md:text-4xl">{stat.value}</p>
                <p className="mt-1 text-sm text-muted-foreground">{stat.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="py-24">
        <div className="mx-auto max-w-7xl px-4">
          <div className="text-center">
            <span className="inline-block rounded-full border border-primary/20 bg-primary/5 px-3 py-1 text-sm font-medium text-primary">
              Features
            </span>
            <h2 className="mt-4 text-3xl font-bold md:text-4xl">
              Everything you need for financial transparency
            </h2>
            <p className="mx-auto mt-4 max-w-2xl text-muted-foreground">
              Comprehensive tools to track, verify, and report your
              organization&apos;s financial activities.
            </p>
          </div>

          <div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
            {features.map((feature) => {
              const Icon = feature.icon
              return (
                <div
                  key={feature.title}
                  className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
                >
                  {/* Subtle gradient overlay on hover */}
                  <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

                  <div className="relative">
                    <div className={`flex h-12 w-12 items-center justify-center rounded-xl ${feature.bg} transition-transform group-hover:scale-110`}>
                      <Icon className={`size-6 ${feature.color}`} weight="duotone" />
                    </div>
                    <h3 className="mt-4 font-semibold">{feature.title}</h3>
                    <p className="mt-2 text-sm text-muted-foreground">
                      {feature.description}
                    </p>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="relative overflow-hidden border-t border-border bg-gradient-to-br from-card to-card/50 py-20">
        {/* Background decoration */}
        <div className="absolute inset-0 -z-10">
          <div className="absolute left-1/4 top-1/2 -translate-y-1/2">
            <div className="h-[300px] w-[300px] rounded-full bg-gradient-to-br from-primary/10 to-transparent blur-3xl" />
          </div>
        </div>

        <div className="mx-auto max-w-7xl px-4 text-center">
          <h2 className="text-3xl font-bold md:text-4xl">Ready to build trust?</h2>
          <p className="mx-auto mt-4 max-w-xl text-lg text-muted-foreground">
            Join hundreds of organizations already using FlowTrace to demonstrate
            financial accountability and transparency.
          </p>
          <div className="mt-8 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Link to="/dashboard">
              <Button size="lg" className="group">
                {isAuthenticated ? 'Go to Dashboard' : 'Get Started Free'}
                <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" data-icon="inline-end" />
              </Button>
            </Link>
            <Button variant="outline" size="lg">
              Schedule a Demo
            </Button>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border bg-card/50 py-12">
        <div className="mx-auto max-w-7xl px-4">
          <div className="flex flex-col items-center justify-between gap-6 md:flex-row">
            <div className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-primary to-primary/70">
                <FlowArrow className="size-4 text-primary-foreground" weight="bold" />
              </div>
              <span className="font-bold">FlowTrace</span>
            </div>
            <p className="text-sm text-muted-foreground">
              &copy; {new Date().getFullYear()} FlowTrace. All rights reserved.
            </p>
            <div className="flex gap-6 text-sm text-muted-foreground">
              <a href="#" className="transition-colors hover:text-foreground">Privacy</a>
              <a href="#" className="transition-colors hover:text-foreground">Terms</a>
              <a href="#" className="transition-colors hover:text-foreground">Contact</a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  )
}
