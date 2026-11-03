import { Fragment } from 'react'
import { ChainLine } from '@/components/chain-status-badge'
import { cn } from '@/lib/utils'

// The landing page's chain graphic (`routes/_marketing/index.tsx`, FundersSection), drawn
// from the same data; the landing page can switch to this component once LP2 has landed.
const CHAIN: Array<{ name: string; note: string }> = [
  { name: 'Funder', note: 'Follows its grant' },
  { name: 'Your organisation', note: 'Records and proves' },
  { name: 'Implementing partner', note: 'Delivers on the ground' },
]
const CHAIN_LINKS: Array<{ status: 'active' | 'pending'; label: string }> = [
  { status: 'active', label: 'Approved' },
  { status: 'pending', label: 'Requested' },
]

/** Funder to organisation (approved) to partner (requested), with the legend under it. */
export function ChainGraphic() {
  return (
    <div>
      <ol
        className="flex flex-col border-t-[3px] border-foreground pt-5 md:grid md:grid-cols-[minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)] md:items-start md:gap-x-3"
        aria-label="A chain of organisations: funder to your organisation approved, your organisation to implementing partner requested"
      >
        {CHAIN.map((org, i) => (
          <Fragment key={org.name}>
            <li className="min-w-0">
              <p className="leading-tight font-semibold">{org.name}</p>
              <p className="mt-1 text-sm text-muted-foreground">{org.note}</p>
            </li>
            {i < CHAIN_LINKS.length && (
              <li
                aria-hidden="true"
                className="flex items-center gap-3 py-3 pl-1.5 md:flex-col md:items-stretch md:gap-1 md:py-0 md:pt-0.5 md:pl-0"
              >
                <svg
                  width="3"
                  height="40"
                  className="block shrink-0 overflow-visible md:hidden"
                >
                  <line
                    x1="1.5"
                    y1="0"
                    x2="1.5"
                    y2="40"
                    strokeWidth="3"
                    strokeDasharray={
                      CHAIN_LINKS[i].status === 'pending' ? '7 5' : undefined
                    }
                    className={
                      CHAIN_LINKS[i].status === 'pending'
                        ? 'stroke-muted-foreground'
                        : 'stroke-foreground'
                    }
                  />
                </svg>
                <span
                  className={cn(
                    'text-[0.6875rem] font-semibold tracking-[0.08em] uppercase md:text-center',
                    CHAIN_LINKS[i].status === 'active'
                      ? 'text-foreground'
                      : 'text-muted-foreground',
                  )}
                >
                  {CHAIN_LINKS[i].label}
                </span>
                <ChainLine
                  status={CHAIN_LINKS[i].status}
                  className="hidden md:block"
                />
              </li>
            )}
          </Fragment>
        ))}
      </ol>
      <p className="mt-6 text-sm text-muted-foreground">
        A solid line is an approved link; a dashed line is a request awaiting
        the other side. Either side can revoke a link at any time.
      </p>
    </div>
  )
}
