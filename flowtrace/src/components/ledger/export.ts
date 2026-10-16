import { toCsv } from '@/lib/ledger'

/** Saves rows as a CSV file. Amounts should already be in major units. */
export function downloadCsv(filename: string, rows: Array<Array<unknown>>) {
  const blob = new Blob([toCsv(rows)], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename.endsWith('.csv') ? filename : `${filename}.csv`
  document.body.appendChild(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

/** Minor units as a plain decimal string for spreadsheets: -1250.50 */
export const csvAmount = (minor: number) => (minor / 100).toFixed(2)
