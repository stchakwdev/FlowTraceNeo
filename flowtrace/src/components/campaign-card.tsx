import { Link } from '@tanstack/react-router'
import { ArrowSquareOut, DotsThree, Eye, Trash } from '@phosphor-icons/react'
import { CampaignStatusMark, campaignTiming } from './campaign-status-badge'
import type { CampaignWithDetails } from '@/lib/types'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { CoverageBar } from '@/components/report'
import { formatMoney } from '@/lib/format'

interface CampaignTableProps {
  campaigns: Array<CampaignWithDetails>
  onDelete?: (campaign: CampaignWithDetails) => void
}

const pctOf = (c: CampaignWithDetails) =>
  c.target_amount > 0 ? Math.round((c.current_amount / c.target_amount) * 100) : 0

/**
 * Campaigns as a report table: one row per campaign with its coverage,
 * status and timing. Collapses to stacked rows on narrow screens. Hovering a
 * row isolates it and dims the rest.
 */
export function CampaignTable({ campaigns, onDelete }: CampaignTableProps) {
  return (
    <>
      <table className="hidden w-full text-sm md:table">
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2 pr-4 font-semibold">
              Campaign
            </th>
            <th scope="col" className="w-28 py-2 pr-4 font-semibold">
              Status
            </th>
            <th scope="col" className="w-[34%] py-2 pr-4 font-semibold">
              Raised of target
            </th>
            <th scope="col" className="w-40 py-2 pr-4 font-semibold">
              Timing
            </th>
            <th scope="col" className="w-12 py-2">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody className="[&:hover>tr]:opacity-50 [&>tr]:transition-opacity [&>tr]:duration-150 [&>tr:focus-within]:opacity-100 [&>tr:hover]:opacity-100">
          {campaigns.map((c) => (
            <tr key={c.id} className="border-b border-border align-top">
              <td className="py-3 pr-4">
                <Link
                  to="/campaigns/$id"
                  params={{ id: c.id }}
                  className="font-semibold text-foreground underline-offset-4 hover:text-primary hover:underline"
                >
                  {c.name}
                </Link>
                <p className="mt-0.5 line-clamp-1 text-xs text-muted-foreground">
                  {c.target_node ? `Funds ${c.target_node.name}` : 'Not tied to one project'}
                  {c.is_public ? ' · public page' : ' · private'}
                </p>
              </td>
              <td className="py-3 pr-4">
                <CampaignStatusMark status={c.status} />
              </td>
              <td className="py-3 pr-4">
                <div className="flex items-baseline justify-between gap-2 tabular-nums">
                  <span className="font-semibold">
                    {formatMoney(c.current_amount, c.currency)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    of {formatMoney(c.target_amount, c.currency)} · {pctOf(c)}%
                  </span>
                </div>
                <CoverageBar
                  total={Math.max(c.target_amount, c.current_amount)}
                  filled={c.current_amount}
                  className="mt-1.5 h-2"
                  label={`${pctOf(c)}% of target raised`}
                />
              </td>
              <td className="py-3 pr-4 text-xs text-muted-foreground">
                {campaignTiming(c).label}
              </td>
              <td className="py-2 text-right">
                <RowMenu campaign={c} onDelete={onDelete} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <ul className="divide-y divide-border border-b border-border md:hidden">
        {campaigns.map((c) => (
          <li key={c.id} className="py-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <Link
                  to="/campaigns/$id"
                  params={{ id: c.id }}
                  className="font-semibold text-foreground underline-offset-4 hover:underline"
                >
                  {c.name}
                </Link>
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
                  <CampaignStatusMark status={c.status} />
                  <span className="text-xs text-muted-foreground">
                    {campaignTiming(c).label}
                  </span>
                </div>
              </div>
              <RowMenu campaign={c} onDelete={onDelete} />
            </div>
            <div className="mt-2 flex items-baseline justify-between gap-2 text-sm tabular-nums">
              <span className="font-semibold">
                {formatMoney(c.current_amount, c.currency)}
              </span>
              <span className="text-xs text-muted-foreground">
                of {formatMoney(c.target_amount, c.currency)} · {pctOf(c)}%
              </span>
            </div>
            <CoverageBar
              total={Math.max(c.target_amount, c.current_amount)}
              filled={c.current_amount}
              className="mt-1.5 h-2"
              label={`${pctOf(c)}% of target raised`}
            />
          </li>
        ))}
      </ul>
    </>
  )
}

function RowMenu({
  campaign,
  onDelete,
}: {
  campaign: CampaignWithDetails
  onDelete?: (campaign: CampaignWithDetails) => void
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            className="size-10 shrink-0"
            aria-label={`Actions for ${campaign.name}`}
          />
        }
      >
        <DotsThree className="size-5" weight="bold" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          render={<Link to="/campaigns/$id" params={{ id: campaign.id }} />}
        >
          <Eye />
          Open campaign
        </DropdownMenuItem>
        {campaign.is_public && campaign.public_slug && campaign.status === 'active' && (
          <DropdownMenuItem
            render={
              <a
                href={`/donate/${campaign.public_slug}`}
                target="_blank"
                rel="noopener noreferrer"
              />
            }
          >
            <ArrowSquareOut />
            View donate page
          </DropdownMenuItem>
        )}
        {onDelete && campaign.status === 'draft' && (
          <DropdownMenuItem
            onClick={() => onDelete(campaign)}
            className="text-destructive"
          >
            <Trash />
            Delete draft
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
