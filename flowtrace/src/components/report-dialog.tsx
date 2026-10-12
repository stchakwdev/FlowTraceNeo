import { Dialog } from '@base-ui/react/dialog'
import { Checkbox } from '@base-ui/react/checkbox'
import { Check, X } from '@phosphor-icons/react'
import { useId } from 'react'
import type { ReactNode, RefObject } from 'react'
import { cn } from '@/lib/utils'

/**
 * A modal dialog in the report's own frame: a heavy ink rule, a condensed
 * title and a footer of actions. Built on Base UI's Dialog, so it carries
 * role="dialog", aria-modal, labelling from the title, a focus trap, Escape
 * to close, and focus returned to the trigger on close.
 */
export function ReportDialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  initialFocus,
  className,
  bodyClassName,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: ReactNode
  description?: ReactNode
  children: ReactNode
  footer?: ReactNode
  initialFocus?: RefObject<HTMLElement | null>
  className?: string
  /** Padding for the scrolling body; pass '' when the content pads itself. */
  bodyClassName?: string
}) {
  return (
    <Dialog.Root open={open} onOpenChange={(next) => onOpenChange(next)}>
      <Dialog.Portal>
        <Dialog.Backdrop className="fixed inset-0 z-50 bg-foreground/40 transition-opacity duration-150 data-[ending-style]:opacity-0 data-[starting-style]:opacity-0" />
        <Dialog.Popup
          aria-modal="true"
          initialFocus={initialFocus}
          className={cn(
            'fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100dvh-2rem)] w-[calc(100vw-2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col border-t-[3px] border-foreground bg-card text-card-foreground shadow-[0_12px_40px_-12px_oklch(0.24_0.035_250/0.35)] outline-none transition-[opacity,transform] duration-150 ease-out data-[ending-style]:opacity-0 data-[starting-style]:translate-y-[calc(-50%+8px)] data-[starting-style]:opacity-0',
            className,
          )}
        >
          <div className="flex items-start justify-between gap-4 border-b border-border px-5 pt-4 pb-3">
            <div className="min-w-0">
              <Dialog.Title className="text-2xl leading-tight">
                {title}
              </Dialog.Title>
              {description && (
                <Dialog.Description className="mt-1 text-sm text-muted-foreground">
                  {description}
                </Dialog.Description>
              )}
            </div>
            <Dialog.Close
              aria-label="Close"
              className="-mr-2 inline-flex size-10 shrink-0 items-center justify-center text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
            >
              <X className="size-5" />
            </Dialog.Close>
          </div>
          <div
            className={cn(
              'min-h-0 flex-1 overflow-y-auto',
              bodyClassName ?? 'px-5 py-4',
            )}
          >
            {children}
          </div>
          {footer && (
            <div className="flex flex-col-reverse gap-2 border-t border-border px-5 py-3 sm:flex-row sm:justify-end">
              {footer}
            </div>
          )}
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

/**
 * A square checkbox drawn in the report palette (Base UI Checkbox, so it is
 * a real role="checkbox" with keyboard support). Wrap it in a <label>.
 */
export function ReportCheckbox({
  checked,
  onCheckedChange,
  disabled,
  className,
  id,
  'aria-describedby': describedBy,
}: {
  checked: boolean
  onCheckedChange: (checked: boolean) => void
  disabled?: boolean
  className?: string
  id?: string
  'aria-describedby'?: string
}) {
  return (
    <Checkbox.Root
      id={id}
      checked={checked}
      disabled={disabled}
      aria-describedby={describedBy}
      onCheckedChange={(next) => onCheckedChange(next)}
      className={cn(
        'inline-flex size-5 shrink-0 items-center justify-center border-[1.5px] border-foreground/70 bg-card outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background data-[checked]:border-primary data-[checked]:bg-primary data-[disabled]:opacity-50',
        className,
      )}
    >
      <Checkbox.Indicator className="text-primary-foreground data-[unchecked]:hidden">
        <Check weight="bold" className="size-3.5" />
      </Checkbox.Indicator>
    </Checkbox.Root>
  )
}

/** A square checkbox with its label and an optional hint line. */
export function CheckboxField({
  checked,
  onCheckedChange,
  label,
  hint,
  className,
}: {
  checked: boolean
  onCheckedChange: (checked: boolean) => void
  label: ReactNode
  hint?: ReactNode
  className?: string
}) {
  const hintId = useId()
  return (
    <label
      className={cn(
        'flex min-h-10 cursor-pointer items-start gap-3 py-2 text-sm',
        className,
      )}
    >
      <Checkbox.Root
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value)}
        aria-describedby={hint ? hintId : undefined}
        className="mt-px flex size-5 shrink-0 items-center justify-center border-2 border-foreground/70 bg-card outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-offset-2 focus-visible:ring-offset-background data-[checked]:border-primary data-[checked]:bg-primary"
      >
        <Checkbox.Indicator>
          <Check weight="bold" className="size-3.5 text-primary-foreground" />
        </Checkbox.Indicator>
      </Checkbox.Root>
      <span className="min-w-0">
        <span className="font-medium text-foreground">{label}</span>
        {hint && (
          <span id={hintId} className="mt-0.5 block text-xs text-muted-foreground">
            {hint}
          </span>
        )}
      </span>
    </label>
  )
}

/** Label, control and a visible error or hint underneath. */
export function FormField({
  id,
  label,
  required,
  error,
  hint,
  children,
  className,
}: {
  id: string
  label: ReactNode
  required?: boolean
  error?: string
  hint?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <label htmlFor={id} className="block text-sm font-semibold text-foreground">
        {label}
        {required ? (
          <span className="font-normal text-muted-foreground"> (required)</span>
        ) : null}
      </label>
      {children}
      {error ? (
        <p id={`${id}-error`} className="text-xs font-semibold text-destructive">
          {error}
        </p>
      ) : hint ? (
        <p id={`${id}-hint`} className="text-xs text-muted-foreground">
          {hint}
        </p>
      ) : null}
    </div>
  )
}

/** aria props for a control inside FormField. */
export function fieldAria(id: string, error?: string, hint?: boolean) {
  return {
    'aria-invalid': error ? true : undefined,
    'aria-describedby': error ? `${id}-error` : hint ? `${id}-hint` : undefined,
  }
}
