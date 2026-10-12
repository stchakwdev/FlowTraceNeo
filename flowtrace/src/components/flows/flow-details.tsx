import { Link } from '@tanstack/react-router'
import { ArrowRight, X } from '@phosphor-icons/react'
import { NODE_TYPE_LABEL, UNASSIGNED_ID } from './flow-graph'
import type { TransactionWithNodes } from '@/lib/supabase'
import type { FlowGraph, Selection } from './flow-graph'
import { StatusMark } from '@/components/report'
import { Button } from '@/components/ui/button'
import { formatDate, formatMoney } from '@/lib/format'

function byDateDesc(a: TransactionWithNodes, b: TransactionWithNodes) {
  return b.transaction_date.localeCompare(a.transaction_date)
}

function plural(n: number, word: string) {
  return `${n} ${word}${n === 1 ? '' : 's'}`
}

export function FlowDetails({
  graph,
  selection,
  onClear,
  headingId,
}: {
  graph: FlowGraph
  selection: Selection
  onClear: () => void
  headingId: string
}) {
  const nodeName = (id: string) =>
    graph.nodes.find((n) => n.id === id)?.name ?? 'Unknown'

  const clear = (
    <Button
      variant="ghost"
      size="icon-lg"
      onClick={onClear}
      aria-label="Clear selection (Esc)"
      className="-mr-2 shrink-0"
    >
      <X />
    </Button>
  )

  if (selection.kind === 'edge') {
    const edge = graph.edges.find((e) => e.id === selection.id)
    if (!edge) return null
    const txns = [...edge.transactions].sort(byDateDesc)
    return (
      <div>
        <div className="flex items-start justify-between gap-3">
          <h2
            id={headingId}
            className="text-2xl leading-tight"
          >
            {nodeName(edge.source)}
            <ArrowRight
              aria-label="to"
              className="mx-1.5 inline size-5 align-[-0.1em]"
            />
            {nodeName(edge.target)}
          </h2>
          {clear}
        </div>
        <p className="mt-1 text-sm text-muted-foreground">
          {plural(txns.length, 'movement')} on this line
        </p>
        <div className="mt-4 flex flex-wrap items-end justify-between gap-3 border-y border-border py-3">
          <div>
            <p className="figure text-3xl">{formatMoney(edge.amount)}</p>
            <p className="mt-1 text-xs text-muted-foreground">Moved in period</p>
          </div>
          <StatusMark status={edge.status} />
        </div>
        <MovementList
          title="Movements"
          transactions={txns}
          counterparty={() => null}
        />
      </div>
    )
  }

  const node = graph.nodes.find((n) => n.id === selection.id)
  if (!node) return null
  const incoming = graph.edges
    .filter((e) => e.target === node.id)
    .flatMap((e) => e.transactions)
    .sort(byDateDesc)
  const outgoing = graph.edges
    .filter((e) => e.source === node.id)
    .flatMap((e) => e.transactions)
    .sort(byDateDesc)

  return (
    <div>
      <div className="flex items-start justify-between gap-3">
        <h2 id={headingId} className="text-2xl leading-tight">
          {node.name}
        </h2>
        {clear}
      </div>
      <p className="mt-1 text-sm text-muted-foreground">
        {NODE_TYPE_LABEL[node.type]}
        {node.id === UNASSIGNED_ID &&
          '. These movements left an account without a recipient on record.'}
      </p>
      <dl className="mt-4 grid grid-cols-2 border-y border-border">
        <div className="py-3 pr-3">
          <dt className="text-xs text-muted-foreground">Received</dt>
          <dd className="figure mt-1 text-2xl">{formatMoney(node.inflow)}</dd>
        </div>
        <div className="border-l border-border py-3 pl-3">
          <dt className="text-xs text-muted-foreground">Sent</dt>
          <dd className="figure mt-1 text-2xl">{formatMoney(node.outflow)}</dd>
        </div>
      </dl>
      {incoming.length > 0 && (
        <MovementList
          title={`Money in (${incoming.length})`}
          transactions={incoming}
          counterparty={(t) => t.from_node?.name ?? null}
          direction="from"
        />
      )}
      {outgoing.length > 0 && (
        <MovementList
          title={`Money out (${outgoing.length})`}
          transactions={outgoing}
          counterparty={(t) => t.to_node?.name ?? 'No recipient recorded'}
          direction="to"
        />
      )}
    </div>
  )
}

function MovementList({
  title,
  transactions,
  counterparty,
  direction,
}: {
  title: string
  transactions: Array<TransactionWithNodes>
  counterparty: (t: TransactionWithNodes) => string | null
  direction?: 'from' | 'to'
}) {
  return (
    <section className="mt-6">
      <h3 className="section-label">{title}</h3>
      <ul className="mt-2 border-t border-border">
        {transactions.map((t) => {
          const who = counterparty(t)
          return (
            <li key={t.id} className="border-b border-border">
              <Link
                to="/transactions/$id"
                params={{ id: t.id }}
                className="-mx-2 block px-2 py-3 transition-colors hover:bg-muted focus-visible:bg-muted focus-visible:outline-2 focus-visible:outline-ring"
              >
                <span className="flex items-baseline justify-between gap-3">
                  <span className="min-w-0 truncate text-sm font-semibold">
                    {who ? (
                      <>
                        <span className="font-normal text-muted-foreground">
                          {direction}{' '}
                        </span>
                        {who}
                      </>
                    ) : (
                      t.description
                    )}
                  </span>
                  <span className="shrink-0 text-sm font-semibold tabular-nums">
                    {formatMoney(Number(t.amount))}
                  </span>
                </span>
                {who && (
                  <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                    {t.description}
                  </span>
                )}
                <span className="mt-1.5 flex items-center justify-between gap-3">
                  <time
                    dateTime={t.transaction_date}
                    className="text-xs text-muted-foreground tabular-nums"
                  >
                    {formatDate(t.transaction_date)}
                  </time>
                  <StatusMark status={t.verification_status} />
                </span>
              </Link>
            </li>
          )
        })}
      </ul>
    </section>
  )
}
