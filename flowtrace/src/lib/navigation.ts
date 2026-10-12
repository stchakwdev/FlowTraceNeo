import {
  Bank,
  BookOpen,
  ChartBar,
  CurrencyCircleDollar,
  FlowArrow,
  Gear,
  HandCoins,
  House,
  LinkSimple,
  Megaphone,
  Receipt,
  ShieldCheck,
} from '@phosphor-icons/react'

export const navSections = [
  {
    title: null,
    items: [
      { label: 'Dashboard', href: '/dashboard', icon: House },
      { label: 'Transactions', href: '/transactions', icon: CurrencyCircleDollar },
      { label: 'Flows', href: '/flows', icon: FlowArrow },
    ],
  },
  {
    title: 'Books',
    items: [
      { label: 'Spend', href: '/spend', icon: Receipt },
      { label: 'Grants', href: '/grants', icon: HandCoins },
      { label: 'Ledger', href: '/ledger', icon: BookOpen },
      { label: 'Reconcile', href: '/reconcile', icon: Bank },
      { label: 'Reports', href: '/reports', icon: ChartBar },
    ],
  },
  {
    title: 'Transparency',
    items: [
      { label: 'Campaigns', href: '/campaigns', icon: Megaphone },
      { label: 'Chains', href: '/chains', icon: LinkSimple },
      { label: 'Trust Score', href: '/trust-score', icon: ShieldCheck },
      { label: 'Settings', href: '/settings', icon: Gear },
    ],
  },
] as const

export type NavItem = (typeof navSections)[number]['items'][number]

export const navItems: Array<NavItem> = navSections.flatMap(
  (s): ReadonlyArray<NavItem> => s.items,
)

export function isNavItemActive(href: string, pathname: string) {
  return (
    pathname === href || (href !== '/dashboard' && pathname.startsWith(href))
  )
}
