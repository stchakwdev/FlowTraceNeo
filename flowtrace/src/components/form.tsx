// Form building blocks shared by every data-entry screen: a labelled field
// wired to its error and hint through aria-describedby, and the themed
// select bound to the same props.
import { useId } from 'react'
import { cn } from '@/lib/utils'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

export type ControlProps = {
  id: string
  'aria-invalid'?: true
  'aria-describedby'?: string
  'aria-required'?: true
}

export function Field({
  id,
  label,
  required,
  error,
  hint,
  srOnlyLabel,
  fixed,
  children,
}: {
  id: string
  label: string
  required?: boolean
  error?: string
  hint?: string
  srOnlyLabel?: boolean
  /** Always has a value, so it is neither required nor optional. */
  fixed?: boolean
  children: (props: ControlProps) => React.ReactNode
}) {
  const hintId = useId()
  const describedBy = [error && `${id}-error`, hint && hintId]
    .filter(Boolean)
    .join(' ')
  return (
    <div className="min-w-0">
      <label
        htmlFor={id}
        className={cn(
          'mb-1.5 block text-sm font-medium',
          srOnlyLabel && 'sr-only',
        )}
      >
        {label}
        {required ? (
          <span className="sr-only"> (required)</span>
        ) : fixed ? null : (
          <span className="ml-1.5 text-xs font-normal text-muted-foreground">
            optional
          </span>
        )}
      </label>
      {children({
        id,
        'aria-invalid': error ? true : undefined,
        'aria-describedby': describedBy || undefined,
        'aria-required': required ? true : undefined,
      })}
      {error && (
        <p id={`${id}-error`} className="mt-1.5 text-sm text-destructive">
          {error}
        </p>
      )}
      {hint && (
        <p
          id={hintId}
          className={cn(
            'mt-1 text-xs text-muted-foreground',
            id === 'amount' && 'tabular-nums',
          )}
        >
          {hint}
        </p>
      )}
    </div>
  )
}

export type Option = { value: string; label: string }

// The themed select, wired to the Field's id and aria props. An empty string
// stands for "no choice" and shows the placeholder (or a '' option's label).
export function FormSelect({
  value,
  onValueChange,
  options = [],
  groups = [],
  placeholder,
  disabled,
  className,
  ...control
}: ControlProps & {
  value: string
  onValueChange: (value: string) => void
  options?: Array<Option>
  groups?: Array<{ label: string; options: Array<Option> }>
  placeholder?: string
  disabled?: boolean
  className?: string
}) {
  const all = [...options, ...groups.flatMap((g) => g.options)]
  const labelOf = (v: string) => all.find((o) => o.value === v)?.label
  const item = (o: Option) => (
    <SelectItem key={o.value || '__none'} value={o.value || null}>
      {o.label}
    </SelectItem>
  )
  return (
    <Select
      value={value || null}
      disabled={disabled}
      onValueChange={(v) => onValueChange(v ?? '')}
    >
      <SelectTrigger {...control} className={cn('h-10 w-full', className)}>
        <SelectValue>
          {(v: string | null) => labelOf(v ?? '') ?? placeholder ?? ''}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        {options.map(item)}
        {groups.map((g) => (
          <SelectGroup key={g.label}>
            <SelectLabel>{g.label}</SelectLabel>
            {g.options.map(item)}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  )
}
