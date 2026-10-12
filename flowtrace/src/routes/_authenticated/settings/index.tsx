import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  Globe,
  Palette,
  ShieldCheck,
  Users,
  Bell,
  Code,
} from '@phosphor-icons/react'
import type { ReactNode } from 'react'

export const Route = createFileRoute('/_authenticated/settings/')({
  component: SettingsPage,
})

const accentColors = [
  'bg-primary',
  'bg-chart-4',
  'bg-success',
  'bg-accent',
  'bg-chart-3',
  'bg-muted-foreground',
]

const settingsItems = [
  {
    icon: Users,
    title: 'Access Groups',
    description: 'Manage who can view and interact with your financial data',
    href: '/settings/access-groups',
  },
  {
    icon: Globe,
    title: 'Public Embed',
    description: 'Configure your embeddable transparency widget',
    href: '/settings/embed',
    comingSoon: true,
  },
  {
    icon: ShieldCheck,
    title: 'Trust Score',
    description: 'Configure how your trust score is calculated',
    href: '/settings/trust-score',
    comingSoon: true,
  },
  {
    icon: Bell,
    title: 'Notifications',
    description: 'Manage email and in-app notification preferences',
    href: '/settings/notifications',
    comingSoon: true,
  },
  {
    icon: Palette,
    title: 'Branding',
    description: 'Customize your organization logo and colors',
    href: '/settings/branding',
    comingSoon: true,
  },
  {
    icon: Code,
    title: 'API & Integrations',
    description: 'Manage API keys and third-party integrations',
    href: '/settings/api',
    comingSoon: true,
  },
]

function SettingsPage() {
  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="mb-8">
        <p className="section-label">CONFIGURATION</p>
        <h1 className="mt-1 font-display text-2xl font-bold">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Configure your organization and platform preferences
        </p>
      </div>

      {/* Settings Grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {settingsItems.map((item, idx) => (
          <SettingsCard
            key={item.title}
            icon={<item.icon className="h-5 w-5" weight="duotone" />}
            accentColor={accentColors[idx % accentColors.length]}
            title={item.title}
            description={item.description}
            href={item.href}
            comingSoon={item.comingSoon}
          />
        ))}
      </div>
    </div>
  )
}

function SettingsCard({
  icon,
  accentColor,
  title,
  description,
  href,
  comingSoon,
}: {
  icon: ReactNode
  accentColor: string
  title: string
  description: string
  href: string
  comingSoon?: boolean
}) {
  const content = (
    <div className="relative flex items-start gap-4 pl-2">
      <span className="shrink-0 text-muted-foreground">{icon}</span>
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <h3 className="font-display font-semibold">{title}</h3>
          {comingSoon && (
            <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
              Coming Soon
            </span>
          )}
        </div>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
      <ArrowRight className="h-5 w-5 shrink-0 text-muted-foreground/50 transition-all group-hover:translate-x-1 group-hover:text-primary" />
    </div>
  )

  if (comingSoon) {
    return (
      <div className="group relative cursor-not-allowed overflow-hidden rounded-lg border border-border bg-card/50 p-5 opacity-60">
        <div className={`absolute left-0 top-0 h-full w-1 ${accentColor}`} />
        {content}
      </div>
    )
  }

  return (
    <Link
      to={href}
      className="group relative overflow-hidden rounded-lg border border-border bg-card p-5 transition-all hover:-translate-y-0.5 hover:shadow-md"
    >
      <div className={`absolute left-0 top-0 h-full w-1 ${accentColor}`} />
      {content}
    </Link>
  )
}
