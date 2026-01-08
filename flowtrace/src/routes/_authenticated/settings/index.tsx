import { Link, createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  Gear,
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

const settingsItems = [
  {
    icon: Users,
    iconBg: 'bg-blue-100 dark:bg-blue-900/30',
    iconColor: 'text-blue-600 dark:text-blue-400',
    title: 'Access Groups',
    description: 'Manage who can view and interact with your financial data',
    href: '/settings/access-groups',
  },
  {
    icon: Globe,
    iconBg: 'bg-violet-100 dark:bg-violet-900/30',
    iconColor: 'text-violet-600 dark:text-violet-400',
    title: 'Public Embed',
    description: 'Configure your embeddable transparency widget',
    href: '/settings/embed',
    comingSoon: true,
  },
  {
    icon: ShieldCheck,
    iconBg: 'bg-emerald-100 dark:bg-emerald-900/30',
    iconColor: 'text-emerald-600 dark:text-emerald-400',
    title: 'Trust Score',
    description: 'Configure how your trust score is calculated',
    href: '/settings/trust-score',
    comingSoon: true,
  },
  {
    icon: Bell,
    iconBg: 'bg-amber-100 dark:bg-amber-900/30',
    iconColor: 'text-amber-600 dark:text-amber-400',
    title: 'Notifications',
    description: 'Manage email and in-app notification preferences',
    href: '/settings/notifications',
    comingSoon: true,
  },
  {
    icon: Palette,
    iconBg: 'bg-pink-100 dark:bg-pink-900/30',
    iconColor: 'text-pink-600 dark:text-pink-400',
    title: 'Branding',
    description: 'Customize your organization logo and colors',
    href: '/settings/branding',
    comingSoon: true,
  },
  {
    icon: Code,
    iconBg: 'bg-slate-100 dark:bg-slate-900/30',
    iconColor: 'text-slate-600 dark:text-slate-400',
    title: 'API & Integrations',
    description: 'Manage API keys and third-party integrations',
    href: '/settings/api',
    comingSoon: true,
  },
]

function SettingsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-primary/70 shadow-lg shadow-primary/20">
            <Gear className="h-6 w-6 text-primary-foreground" weight="bold" />
          </div>
          <div>
            <h1 className="text-3xl font-bold">Settings</h1>
            <p className="text-muted-foreground">
              Configure your organization and platform preferences
            </p>
          </div>
        </div>
      </div>

      {/* Settings Grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {settingsItems.map((item) => (
          <SettingsCard
            key={item.title}
            icon={<item.icon className="h-5 w-5" weight="duotone" />}
            iconBg={item.iconBg}
            iconColor={item.iconColor}
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
  iconBg,
  iconColor,
  title,
  description,
  href,
  comingSoon,
}: {
  icon: ReactNode
  iconBg: string
  iconColor: string
  title: string
  description: string
  href: string
  comingSoon?: boolean
}) {
  const content = (
    <div className="relative flex items-start gap-4">
      <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ${iconBg} transition-transform group-hover:scale-110`}>
        <span className={iconColor}>{icon}</span>
      </div>
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <h3 className="font-semibold">{title}</h3>
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
      <div className="group relative cursor-not-allowed overflow-hidden rounded-xl border border-border bg-card/50 p-6 opacity-60">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0" />
        {content}
      </div>
    )
  }

  return (
    <Link
      to={href}
      className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
    >
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
      {content}
    </Link>
  )
}
