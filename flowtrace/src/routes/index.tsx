import { Link, createFileRoute } from '@tanstack/react-router'
import { useAuth } from '@workos/authkit-tanstack-react-start/client'
import {
  ArrowRight,
  ChartLineUp,
  CheckCircle,
  FlowArrow,
  Globe,
  ShieldCheck,
  UsersThree,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { ThemeToggle } from '@/components/layout/theme-toggle'
import { DEV_MODE } from '@/lib/auth'

export const Route = createFileRoute('/')({
  component: LandingPage,
})

const features = [
  {
    icon: FlowArrow,
    title: 'Track Money Flows',
    description:
      'Visualize how funds move through your organization with interactive flow diagrams and real-time tracking.',
    accent: 'bg-primary',
    span: 'sm:col-span-1 sm:row-span-2',
  },
  {
    icon: ShieldCheck,
    title: 'Build Trust Scores',
    description:
      'Establish credibility with transparent, verifiable financial records that stakeholders can trust.',
    accent: 'bg-[oklch(0.65_0.2_145)]',
    span: '',
  },
  {
    icon: ChartLineUp,
    title: 'Real-time Analytics',
    description:
      'Get instant insights into your financial health with comprehensive dashboards and reporting.',
    accent: 'bg-accent',
    span: '',
  },
  {
    icon: UsersThree,
    title: 'Stakeholder Reports',
    description:
      'Generate professional reports for donors, board members, and regulatory bodies with ease.',
    accent: 'bg-[oklch(0.5_0.15_280)]',
    span: 'sm:col-span-2',
  },
]

const stats = [
  { value: '100+', label: 'Organizations', accent: 'border-primary' },
  { value: '$50M+', label: 'Tracked', accent: 'border-accent' },
  { value: '99.9%', label: 'Uptime', accent: 'border-[oklch(0.65_0.2_145)]' },
  { value: '15+', label: 'Countries', accent: 'border-[oklch(0.5_0.15_280)]' },
]

function LandingPage() {
  const { user } = useAuth()
  const isAuthenticated = DEV_MODE || !!user

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-50 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-6">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary">
              <FlowArrow
                className="size-5 text-primary-foreground"
                weight="bold"
              />
            </div>
            <span className="font-display text-xl font-bold">FlowTrace</span>
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

      {/* Hero Section - Asymmetric 2-col */}
      <section className="relative overflow-hidden border-b border-border">
        <div className="dot-grid absolute inset-0 -z-10" />

        <div className="mx-auto grid max-w-6xl gap-12 px-6 py-20 lg:grid-cols-2 lg:py-28">
          {/* Text - LEFT */}
          <div className="flex flex-col justify-center">
            <h1 className="font-display text-4xl font-bold leading-[1.1] tracking-tight sm:text-5xl lg:text-6xl">
              Financial Transparency
              <br />
              <span className="text-primary">Made Verifiable</span>
            </h1>
            <p className="mt-6 max-w-lg text-lg text-muted-foreground">
              FlowTrace helps NGOs, nonprofits, and social enterprises build
              trust through transparent financial tracking. Visualize fund flows,
              generate trust scores, and provide stakeholders with verifiable
              records.
            </p>
            <div className="mt-8 flex flex-wrap gap-4">
              <Link to="/dashboard">
                <Button size="lg" className="group">
                  {isAuthenticated ? 'Open Dashboard' : 'Get Started Free'}
                  <ArrowRight
                    className="size-4 transition-transform group-hover:translate-x-0.5"
                    data-icon="inline-end"
                  />
                </Button>
              </Link>
              <Button variant="outline" size="lg" className="gap-2">
                <Globe className="size-4" />
                View Demo
              </Button>
            </div>

            {/* Trust indicators */}
            <div className="mt-8 flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted-foreground">
              <div className="flex items-center gap-2">
                <CheckCircle
                  className="h-4 w-4 text-success"
                  weight="fill"
                />
                No credit card required
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle
                  className="h-4 w-4 text-success"
                  weight="fill"
                />
                Free tier available
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle
                  className="h-4 w-4 text-success"
                  weight="fill"
                />
                SOC 2 compliant
              </div>
            </div>
          </div>

          {/* Flow visualization placeholder - RIGHT */}
          <div className="hidden items-center justify-center lg:flex">
            <div className="relative flex h-80 w-full items-center justify-center rounded-2xl border border-border bg-card/50">
              <div className="absolute inset-0 rounded-2xl bg-gradient-to-br from-primary/5 to-accent/5" />
              <div className="relative text-center">
                <FlowArrow
                  className="mx-auto h-16 w-16 text-primary/40"
                  weight="duotone"
                />
                <p className="mt-4 text-sm text-muted-foreground">
                  Interactive flow visualization
                </p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Stats Bar - Left-aligned with thick colored left borders */}
      <section className="border-b border-border">
        <div className="mx-auto max-w-6xl px-6 py-10">
          <div className="flex flex-wrap gap-8">
            {stats.map((stat) => (
              <div
                key={stat.label}
                className={`border-l-4 ${stat.accent} pl-4`}
              >
                <p className="font-display text-3xl font-bold">{stat.value}</p>
                <p className="text-sm text-muted-foreground">{stat.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features - Bento grid */}
      <section className="py-20">
        <div className="mx-auto max-w-6xl px-6">
          <p className="section-label">FEATURES</p>
          <h2 className="mt-3 font-display text-3xl font-bold">
            Everything you need for financial transparency
          </h2>
          <p className="mt-3 max-w-2xl text-muted-foreground">
            Comprehensive tools to track, verify, and report your
            organization&apos;s financial activities.
          </p>

          <div className="mt-12 grid gap-4 sm:grid-cols-2">
            {features.map((feature) => {
              const Icon = feature.icon
              return (
                <div
                  key={feature.title}
                  className={`group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:-translate-y-0.5 hover:shadow-md ${feature.span}`}
                >
                  <div
                    className={`absolute left-0 top-0 h-full w-1 ${feature.accent}`}
                  />
                  <div className="relative pl-2">
                    <Icon
                      className="size-6 text-muted-foreground"
                      weight="duotone"
                    />
                    <h3 className="mt-3 font-display font-semibold">
                      {feature.title}
                    </h3>
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
      <section className="border-t border-border py-16">
        <div className="mx-auto max-w-6xl px-6 text-center">
          <h2 className="font-display text-3xl font-bold">
            Ready to build trust?
          </h2>
          <p className="mx-auto mt-4 max-w-xl text-lg text-muted-foreground">
            Join hundreds of organizations already using FlowTrace to
            demonstrate financial accountability and transparency.
          </p>
          <div className="mt-8 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Link to="/dashboard">
              <Button size="lg" className="group">
                {isAuthenticated ? 'Go to Dashboard' : 'Get Started Free'}
                <ArrowRight
                  className="size-4 transition-transform group-hover:translate-x-0.5"
                  data-icon="inline-end"
                />
              </Button>
            </Link>
            <Button variant="outline" size="lg">
              Schedule a Demo
            </Button>
          </div>
        </div>
      </section>

      {/* Footer - Single-line minimal */}
      <footer className="border-t border-border py-6">
        <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-6 sm:flex-row">
          <div className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary">
              <FlowArrow
                className="size-3.5 text-primary-foreground"
                weight="bold"
              />
            </div>
            <span className="font-display text-sm font-bold">FlowTrace</span>
          </div>
          <p className="text-xs text-muted-foreground">
            &copy; {new Date().getFullYear()} FlowTrace. All rights reserved.
          </p>
          <div className="flex gap-6 text-xs text-muted-foreground">
            <a href="#" className="transition-colors hover:text-foreground">
              Privacy
            </a>
            <a href="#" className="transition-colors hover:text-foreground">
              Terms
            </a>
            <a href="#" className="transition-colors hover:text-foreground">
              Contact
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}
