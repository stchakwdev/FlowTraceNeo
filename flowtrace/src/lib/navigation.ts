import {
  CurrencyCircleDollar,
  FlowArrow,
  Gear,
  House,
  LinkSimple,
  Megaphone,
  ShieldCheck,
} from '@phosphor-icons/react'

export const navItems = [
  { label: 'Dashboard', href: '/dashboard', icon: House },
  { label: 'Transactions', href: '/transactions', icon: CurrencyCircleDollar },
  { label: 'Flows', href: '/flows', icon: FlowArrow },
  { label: 'Campaigns', href: '/campaigns', icon: Megaphone },
  { label: 'Chains', href: '/chains', icon: LinkSimple },
  { label: 'Trust Score', href: '/trust-score', icon: ShieldCheck },
  { label: 'Settings', href: '/settings', icon: Gear },
]

export function isNavItemActive(href: string, pathname: string) {
  return (
    pathname === href || (href !== '/dashboard' && pathname.startsWith(href))
  )
}
