// The dashboard's figures from the transaction list (routes/_authenticated/dashboard.tsx): the
// same for every backend, so the module's list and the demo's give the same dashboard.
import type { TransactionWithNodes } from './types'

export interface DonorLine {
  id: string
  name: string
  received: number
  evidenced: number
  awaiting: number
}

export function summarise(transactions: Array<TransactionWithNodes>) {
  const donors = new Map<string, DonorLine>()
  let latest: string | null = null
  let verified = 0
  let pending = 0
  let flagged = 0

  for (const t of transactions) {
    if (!latest || t.transaction_date > latest) latest = t.transaction_date
    if (t.verification_status === 'verified') verified++
    if (t.verification_status === 'pending') pending++
    if (t.verification_status === 'flagged') flagged++

    const from = t.from_node
    if (from?.type !== 'donor') continue
    const line = donors.get(from.id) ?? {
      id: from.id,
      name: from.name,
      received: 0,
      evidenced: 0,
      awaiting: 0,
    }
    if (t.verification_status === 'pending') line.awaiting += t.amount
    else if (t.verification_status !== 'rejected') line.received += t.amount
    if (t.verification_status === 'verified') line.evidenced += t.amount
    donors.set(from.id, line)
  }

  // Evidenced donors first, largest evidenced amount first; donors with
  // nothing evidenced yet (awaiting proof) go last, largest first.
  const donorLines = [...donors.values()].sort(
    (a, b) =>
      Number(b.evidenced > 0) - Number(a.evidenced > 0) ||
      b.evidenced - a.evidenced ||
      b.received + b.awaiting - (a.received + a.awaiting),
  )
  const evidencedTotal = donorLines.reduce((s, d) => s + d.evidenced, 0)
  const committedTotal = donorLines.reduce(
    (s, d) => s + d.received + d.awaiting,
    0,
  )
  const largest = donorLines.reduce(
    (m, d) => Math.max(m, d.received + d.awaiting),
    0,
  )

  return {
    latest,
    total: transactions.length,
    verified,
    pending,
    flagged,
    donorLines,
    evidencedTotal,
    committedTotal,
    largest,
  }
}
