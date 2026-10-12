import * as React from 'react'
import { CalendarBlank } from '@phosphor-icons/react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

// A date field that reads and writes day/month/year, the order used in Kenya
// and most of FlowTrace's markets, whatever the browser's locale. The text the
// person types is the field's value; parseDayMonthYear turns it into an ISO
// date (yyyy-mm-dd) for storage. A calendar button opens the browser's own
// picker from a hidden native date input, for people who prefer to click.

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
]

const pad = (n: number) => String(n).padStart(2, '0')

/** Today as yyyy-mm-dd in the viewer's own timezone. */
export function localTodayISO() {
  const d = new Date()
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** '2026-01-18' → '18/01/2026'; anything else → ''. */
export function isoToDayMonthYear(iso: string | null | undefined) {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso ?? '')
  return m ? `${m[3]}/${m[2]}/${m[1]}` : ''
}

export type DateParse =
  | { iso: string; error?: undefined }
  | { iso?: undefined; error: string }

/**
 * Reads dd/mm/yyyy (also d/m/yyyy, or with . or - between the parts).
 * Returns null for an empty field, otherwise an ISO date or a plain-words
 * problem.
 */
export function parseDayMonthYear(text: string): DateParse | null {
  const t = text.trim()
  if (!t) return null
  const m = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d+)$/.exec(t)
  if (!m) {
    return {
      error: 'Enter the date as day/month/year, for example 18/01/2026.',
    }
  }
  const day = Number(m[1])
  const month = Number(m[2])
  if (m[3].length !== 4) {
    return { error: 'Write the year in full, for example 2026.' }
  }
  const year = Number(m[3])
  if (month < 1 || month > 12) {
    return {
      error: `There is no month ${month}. Put the day first, then the month: dd/mm/yyyy.`,
    }
  }
  const days = new Date(Date.UTC(year, month, 0)).getUTCDate()
  if (day < 1 || day > days) {
    return {
      error:
        day < 1
          ? 'There is no day 0. Enter a day from 1.'
          : `${MONTHS[month - 1]} ${year} has only ${days} days.`,
    }
  }
  return { iso: `${year}-${pad(month)}-${pad(day)}` }
}

// Keep only digits and separators, normalise separators to '/', and add the
// slash after the day and the month as the person types forward.
function maskDateInput(next: string, prev: string) {
  let t = next
    .replace(/[^\d/.-]/g, '')
    .replace(/[.-]/g, '/')
    .replace(/\/{2,}/g, '/')
  if (t.length > prev.length && /^(\d{2}|\d{1,2}\/\d{2})$/.test(t)) t += '/'
  return t.slice(0, 10)
}

type DateFieldProps = Omit<
  React.ComponentProps<'input'>,
  'value' | 'onChange' | 'type' | 'min' | 'max' | 'defaultValue'
> & {
  /** The text as typed, dd/mm/yyyy. */
  value: string
  onValueChange: (text: string) => void
  /** Bounds for the calendar, as yyyy-mm-dd. */
  min?: string
  max?: string
  /** Accessible name for the calendar button. */
  pickerLabel?: string
  wrapperClassName?: string
}

function DateField({
  value,
  onValueChange,
  min,
  max,
  pickerLabel = 'Choose from a calendar',
  className,
  wrapperClassName,
  disabled,
  onBlur,
  ...props
}: DateFieldProps) {
  const pickerRef = React.useRef<HTMLInputElement>(null)
  const parsed = parseDayMonthYear(value)

  const openPicker = () => {
    const el = pickerRef.current
    if (!el) return
    try {
      el.showPicker()
    } catch {
      el.focus()
    }
  }

  return (
    <div className={cn('relative', wrapperClassName)}>
      <Input
        {...props}
        type="text"
        inputMode="numeric"
        autoComplete="off"
        placeholder="dd/mm/yyyy"
        maxLength={10}
        disabled={disabled}
        value={value}
        onChange={(e) => onValueChange(maskDateInput(e.target.value, value))}
        onBlur={(e) => {
          // Tidy 1/2/2026 into 01/02/2026 once the person moves on.
          if (parsed?.iso) onValueChange(isoToDayMonthYear(parsed.iso))
          onBlur?.(e)
        }}
        className={cn('pr-11 tabular-nums', className)}
      />
      <input
        ref={pickerRef}
        type="date"
        tabIndex={-1}
        aria-hidden="true"
        min={min}
        max={max}
        value={parsed?.iso ?? ''}
        onChange={(e) => onValueChange(isoToDayMonthYear(e.target.value))}
        className="pointer-events-none absolute right-0 bottom-0 h-px w-px opacity-0"
      />
      <button
        type="button"
        onClick={openPicker}
        disabled={disabled}
        aria-label={pickerLabel}
        className="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:text-foreground focus-visible:ring-1 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
      >
        <CalendarBlank aria-hidden="true" className="size-4" />
      </button>
    </div>
  )
}

export { DateField }
