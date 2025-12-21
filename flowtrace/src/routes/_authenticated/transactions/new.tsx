import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { useCreateTransaction, useNodes } from '@/lib/queries'
import { useOrg } from '@/lib/org-context'

export const Route = createFileRoute('/_authenticated/transactions/new')({
  component: NewTransaction,
})

const categories = [
  'Operations',
  'Programs',
  'Salaries',
  'Transport',
  'Equipment',
  'Training',
  'Supplies',
  'Utilities',
  'Funding',
  'Budget',
  'Other',
]

function NewTransaction() {
  const navigate = useNavigate()
  const { organization } = useOrg()
  const { data: nodes, isLoading: nodesLoading } = useNodes()
  const createTransaction = useCreateTransaction()

  const [formData, setFormData] = useState({
    description: '',
    amount: '',
    currency: organization?.currency ?? 'KES',
    fromNodeId: '',
    toNodeId: '',
    category: '',
    date: new Date().toISOString().split('T')[0],
    notes: '',
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    try {
      await createTransaction.mutateAsync({
        description: formData.description,
        amount: parseFloat(formData.amount),
        currency: formData.currency,
        from_node_id: formData.fromNodeId || null,
        to_node_id: formData.toNodeId || null,
        category: formData.category || null,
        transaction_date: formData.date,
        notes: formData.notes || null,
        reference: null,
        exchange_rate: null,
        amount_usd: null,
        verification_status: 'pending',
        created_by: null,
      })

      // Navigate back to transaction list
      navigate({ to: '/transactions' })
    } catch (error) {
      console.error('Failed to create transaction:', error)
    }
  }

  const handleChange = (
    e: React.ChangeEvent<
      HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
    >,
  ) => {
    const { name, value } = e.target
    setFormData((prev) => ({ ...prev, [name]: value }))
  }

  return (
    <div className="mx-auto max-w-2xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <button
          onClick={() => navigate({ to: '/transactions' })}
          className="mb-4 flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeftIcon className="h-4 w-4" />
          Back to Transactions
        </button>
        <h1 className="text-3xl font-bold">New Transaction</h1>
        <p className="text-muted-foreground">
          Record a new financial transaction
        </p>
      </div>

      {/* Form */}
      <form onSubmit={handleSubmit} className="space-y-6">
        <div className="rounded-lg border border-border bg-card p-6">
          <h2 className="mb-4 text-lg font-semibold">Transaction Details</h2>

          <div className="space-y-4">
            {/* Description */}
            <div>
              <label
                htmlFor="description"
                className="mb-1.5 block text-sm font-medium"
              >
                Description *
              </label>
              <input
                type="text"
                id="description"
                name="description"
                required
                value={formData.description}
                onChange={handleChange}
                placeholder="e.g., Office supplies purchase"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>

            {/* Amount and Currency */}
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label
                  htmlFor="amount"
                  className="mb-1.5 block text-sm font-medium"
                >
                  Amount *
                </label>
                <input
                  type="number"
                  id="amount"
                  name="amount"
                  required
                  min="0"
                  step="0.01"
                  value={formData.amount}
                  onChange={handleChange}
                  placeholder="0.00"
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
              <div>
                <label
                  htmlFor="currency"
                  className="mb-1.5 block text-sm font-medium"
                >
                  Currency
                </label>
                <select
                  id="currency"
                  name="currency"
                  value={formData.currency}
                  onChange={handleChange}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  <option value="KES">KES - Kenyan Shilling</option>
                  <option value="USD">USD - US Dollar</option>
                  <option value="UGX">UGX - Ugandan Shilling</option>
                  <option value="TZS">TZS - Tanzanian Shilling</option>
                  <option value="NGN">NGN - Nigerian Naira</option>
                </select>
              </div>
            </div>

            {/* From and To */}
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label
                  htmlFor="fromNodeId"
                  className="mb-1.5 block text-sm font-medium"
                >
                  From *
                </label>
                <select
                  id="fromNodeId"
                  name="fromNodeId"
                  required
                  disabled={nodesLoading}
                  value={formData.fromNodeId}
                  onChange={handleChange}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
                >
                  <option value="">
                    {nodesLoading ? 'Loading...' : 'Select source...'}
                  </option>
                  {nodes?.map((node) => (
                    <option key={node.id} value={node.id}>
                      {node.name} ({node.type})
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label
                  htmlFor="toNodeId"
                  className="mb-1.5 block text-sm font-medium"
                >
                  To *
                </label>
                <select
                  id="toNodeId"
                  name="toNodeId"
                  required
                  disabled={nodesLoading}
                  value={formData.toNodeId}
                  onChange={handleChange}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
                >
                  <option value="">
                    {nodesLoading ? 'Loading...' : 'Select destination...'}
                  </option>
                  {nodes?.map((node) => (
                    <option key={node.id} value={node.id}>
                      {node.name} ({node.type})
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {/* Category and Date */}
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label
                  htmlFor="category"
                  className="mb-1.5 block text-sm font-medium"
                >
                  Category *
                </label>
                <select
                  id="category"
                  name="category"
                  required
                  value={formData.category}
                  onChange={handleChange}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  <option value="">Select category...</option>
                  {categories.map((cat) => (
                    <option key={cat} value={cat}>
                      {cat}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label
                  htmlFor="date"
                  className="mb-1.5 block text-sm font-medium"
                >
                  Date *
                </label>
                <input
                  type="date"
                  id="date"
                  name="date"
                  required
                  value={formData.date}
                  onChange={handleChange}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            </div>

            {/* Notes */}
            <div>
              <label
                htmlFor="notes"
                className="mb-1.5 block text-sm font-medium"
              >
                Notes
              </label>
              <textarea
                id="notes"
                name="notes"
                rows={3}
                value={formData.notes}
                onChange={handleChange}
                placeholder="Additional details or context..."
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
          </div>
        </div>

        {/* Verification Upload */}
        <div className="rounded-lg border border-border bg-card p-6">
          <h2 className="mb-4 text-lg font-semibold">Verification Documents</h2>
          <div className="rounded-lg border-2 border-dashed border-border p-8 text-center">
            <UploadIcon className="mx-auto h-8 w-8 text-muted-foreground" />
            <p className="mt-2 text-sm text-muted-foreground">
              Drag and drop receipts, invoices, or photos
            </p>
            <p className="text-xs text-muted-foreground">or</p>
            <button
              type="button"
              className="mt-2 rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent"
            >
              Browse Files
            </button>
          </div>
        </div>

        {/* Error Message */}
        {createTransaction.isError && (
          <div className="rounded-lg border border-destructive bg-destructive/10 p-4 text-sm text-destructive">
            Failed to create transaction. Please try again.
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => navigate({ to: '/transactions' })}
            className="rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={createTransaction.isPending || nodesLoading}
            className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {createTransaction.isPending ? 'Saving...' : 'Save Transaction'}
          </button>
        </div>
      </form>
    </div>
  )
}

function ArrowLeftIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M10 19l-7-7m0 0l7-7m-7 7h18"
      />
    </svg>
  )
}

function UploadIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
      />
    </svg>
  )
}
