import { Link, createFileRoute } from '@tanstack/react-router'
import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import {
  ArrowsInLineHorizontal,
  MagnifyingGlassMinus,
  MagnifyingGlassPlus,
} from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import type { PeriodId, Selection } from '@/components/flows/flow-graph'
import {
  KeyFigure,
  KeyFigures,
  ProofLegend,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { Button, buttonVariants } from '@/components/ui/button'
import { FlowDetails } from '@/components/flows/flow-details'
import { FlowMap } from '@/components/flows/flow-map'
import {
  PERIODS,
  WIDEST_PERIOD,
  buildFlowGraph,
  connectedSet,
  filterByPeriod,
  periodWindow,
} from '@/components/flows/flow-graph'
import { formatCompact, formatDate, formatMoney } from '@/lib/format'
import { useTransactions } from '@/lib/queries'
import { cn } from '@/lib/utils'

export const Route = createFileRoute('/_authenticated/flows')({
  component: FlowsPage,
})

const ZOOM_MIN = 0.4
const ZOOM_MAX = 1.6
const ZOOM_STEP = 0.1
// The opening view never shrinks the map below this, which keeps node text at
// 12px or more; narrower screens scroll sideways instead. "Fit" may go smaller
// when asked for.
const LEGIBLE_MIN = 0.9

function FlowsPage() {
  const { data, isLoading, error, refetch, isRefetching } = useTransactions()
  const [period, setPeriod] = useState<PeriodId>('month')
  const [selection, setSelection] = useState<Selection | null>(null)
  // null = legible fit (the opening view); 'fit' = the whole map, however small.
  const [zoom, setZoom] = useState<number | 'fit' | null>(null)
  const [mapWidth, setMapWidth] = useState(0)
  const detailsHeadingId = useId()

  const all = data ?? []
  const range = useMemo(() => periodWindow(all, period), [all, period])
  const inPeriod = useMemo(() => filterByPeriod(all, period), [all, period])
  const graph = useMemo(() => buildFlowGraph(inPeriod), [inPeriod])

  // Drop a selection that the new period no longer contains.
  const current =
    selection &&
    (selection.kind === 'node'
      ? graph.nodes.some((n) => n.id === selection.id)
      : graph.edges.some((e) => e.id === selection.id))
      ? selection
      : null
  const active = useMemo(() => connectedSet(graph, current), [graph, current])

  useEffect(() => {
    if (!current) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setSelection(null)
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [current])

  // Measure the map's container (a callback ref, since it mounts only once
  // data has arrived) so "Fit" can size the map to it.
  const observer = useRef<ResizeObserver | null>(null)
  const mapRef = useCallback((el: HTMLDivElement | null) => {
    observer.current?.disconnect()
    observer.current = null
    if (!el) return
    observer.current = new ResizeObserver(([entry]) =>
      setMapWidth(entry.contentRect.width),
    )
    observer.current.observe(el)
  }, [])

  // Leave a pixel for the container border so a fitted map never scrolls.
  const ratio = mapWidth > 0 ? (mapWidth - 2) / graph.width : 1
  const scale =
    zoom === 'fit'
      ? Math.min(1, Math.max(ZOOM_MIN, ratio))
      : (zoom ?? Math.min(1, Math.max(LEGIBLE_MIN, ratio)))
  const stepZoom = (dir: 1 | -1) =>
    setZoom(
      Math.min(
        ZOOM_MAX,
        Math.max(ZOOM_MIN, Math.round((scale + dir * ZOOM_STEP) * 10) / 10),
      ),
    )

  const periodMeta = PERIODS.find((p) => p.id === period)!
  const widest = PERIODS.find((p) => p.id === WIDEST_PERIOD)!
  const evidencedShare =
    graph.totals.moved > 0
      ? Math.round((graph.totals.evidenced / graph.totals.moved) * 100)
      : 0

  const periodControl = (
    <div
      role="group"
      aria-label="Period"
      className="inline-flex border border-foreground/70"
    >
      {PERIODS.map((p) => (
        <button
          key={p.id}
          type="button"
          aria-pressed={period === p.id}
          onClick={() => setPeriod(p.id)}
          className={cn(
            'h-10 min-w-16 px-3 text-sm font-semibold transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset [&+&]:border-l [&+&]:border-foreground/70',
            period === p.id
              ? 'bg-primary text-primary-foreground'
              : 'text-foreground hover:bg-muted',
          )}
        >
          {p.label}
        </button>
      ))}
    </div>
  )

  let body: ReactNode
  if (isLoading) {
    body = (
      <ReportNotice title="Loading movements">
        Building the flow map from your recorded transactions.
      </ReportNotice>
    )
  } else if (error) {
    body = (
      <ReportNotice
        tone="error"
        title="The flow map could not load"
        action={
          <Button
            variant="outline"
            onClick={() => refetch()}
            disabled={isRefetching}
          >
            {isRefetching ? 'Trying again' : 'Try again'}
          </Button>
        }
      >
        {error instanceof Error ? error.message : String(error)} Check your
        connection, then try again.
      </ReportNotice>
    )
  } else if (all.length === 0) {
    body = (
      <ReportNotice
        title="No movements recorded yet"
        action={
          <Link to="/transactions/new" className={buttonVariants()}>
            Record a transaction
          </Link>
        }
      >
        The map draws each recorded movement from donor to recipient. Record
        your first transaction to start it.
      </ReportNotice>
    )
  } else if (graph.edges.length === 0) {
    body = (
      <ReportNotice
        title={`No movements in the ${periodMeta.phrase}`}
        action={
          period !== WIDEST_PERIOD ? (
            <Button onClick={() => setPeriod(WIDEST_PERIOD)}>
              Show the {widest.phrase}
            </Button>
          ) : undefined
        }
      >
        {range &&
          `The ${periodMeta.label.toLowerCase()} view covers ${formatDate(range.start)} to ${formatDate(range.end)}, and nothing was recorded in it.`}
      </ReportNotice>
    )
  } else {
    body = (
      <>
        <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 border-b border-border py-3">
          <ProofLegend />
          <p className="text-xs text-muted-foreground">
            Line weight carries the amount.
          </p>
        </div>

        {/* The map takes the full width until a selection opens the panel. */}
        <div
          className={cn(
            'grid gap-6 pt-4',
            current && 'lg:grid-cols-[minmax(0,1fr)_19rem]',
          )}
        >
          <div className="min-w-0">
            <div
              ref={mapRef}
              className="max-h-[70vh] overflow-auto overscroll-contain border border-border bg-background lg:max-h-none"
            >
              <FlowMap
                graph={graph}
                zoom={scale}
                selection={current}
                active={active}
                onSelect={setSelection}
                onClear={() => setSelection(null)}
              />
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              Select a donor, account or recipient to isolate everything
              upstream and downstream of it; select a line to see the movements
              it carries. Tab moves through the map in money order; Esc clears
              the selection.
              <span className="lg:hidden"> Scroll sideways to see the whole map.</span>
            </p>
          </div>

          {current ? (
            <aside
              aria-labelledby={detailsHeadingId}
              className="fixed inset-x-0 bottom-0 z-30 max-h-[45vh] overflow-y-auto border-t-[3px] border-foreground bg-background px-4 pt-3 pb-6 shadow-[0_-8px_24px_-12px_rgb(0_0_0/0.35)] lg:sticky lg:top-6 lg:z-auto lg:max-h-[calc(100vh-3rem)] lg:self-start lg:border-t-[3px] lg:px-0 lg:pt-3 lg:shadow-none"
            >
              <FlowDetails
                graph={graph}
                selection={current}
                onClear={() => setSelection(null)}
                headingId={detailsHeadingId}
              />
            </aside>
          ) : null}
        </div>
      </>
    )
  }

  const showMap = !isLoading && !error && graph.edges.length > 0

  return (
    <div className={cn('mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8', current && 'pb-[47vh] lg:pb-8')}>
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">Money flow</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {range
              ? `Movements from ${formatDate(range.start)} to ${formatDate(range.end)}, counted back from the latest recorded transaction.`
              : 'Where money came from, where it went, and the proof behind each movement.'}
          </p>
        </div>
        {all.length > 0 && periodControl}
      </div>

      {showMap && (
        <KeyFigures className="mt-6">
          <KeyFigure
            value={`KES ${formatCompact(graph.totals.moved)}`}
            label="Total moved"
            detail={
              <span className="text-muted-foreground tabular-nums">
                {formatMoney(graph.totals.moved)}
              </span>
            }
          />
          <KeyFigure value={graph.nodes.length} label="Nodes" />
          <KeyFigure
            value={graph.edges.length}
            label="Links"
            detail={
              <span className="text-muted-foreground">
                {graph.totals.movements} movements
              </span>
            }
          />
          <KeyFigure
            value={`${evidencedShare}%`}
            label="Share evidenced"
            detail={
              <span className="text-muted-foreground tabular-nums">
                {formatMoney(graph.totals.evidenced)} has proof
              </span>
            }
          />
        </KeyFigures>
      )}

      <ReportSection
        title="Flow map"
        className="mt-8"
        actions={
          showMap ? (
            <div
              role="group"
              aria-label="Zoom"
              className="flex items-center gap-1"
            >
              <Button
                variant="ghost"
                size="icon"
                className="size-10"
                onClick={() => stepZoom(-1)}
                disabled={scale <= ZOOM_MIN}
                aria-label="Zoom out"
              >
                <MagnifyingGlassMinus />
              </Button>
              <output
                aria-live="polite"
                className="w-11 text-center text-xs font-semibold text-foreground tabular-nums"
              >
                {Math.round(scale * 100)}%
              </output>
              <Button
                variant="ghost"
                size="icon"
                className="size-10"
                onClick={() => stepZoom(1)}
                disabled={scale >= ZOOM_MAX}
                aria-label="Zoom in"
              >
                <MagnifyingGlassPlus />
              </Button>
              <Button
                variant="ghost"
                className="h-10"
                onClick={() => setZoom('fit')}
                disabled={zoom === 'fit'}
              >
                <ArrowsInLineHorizontal />
                Fit
              </Button>
            </div>
          ) : undefined
        }
      >
        {body}
      </ReportSection>
    </div>
  )
}
