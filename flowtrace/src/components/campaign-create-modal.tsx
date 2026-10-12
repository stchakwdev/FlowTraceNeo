import { useState } from 'react'
import { CircleNotch, X } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useCreateCampaign, useNodes } from '@/lib/queries'
import type { CampaignStatus } from '@/lib/supabase'

interface CampaignCreateModalProps {
  isOpen: boolean
  onClose: () => void
}

const CATEGORIES = [
  { value: 'education', label: 'Education' },
  { value: 'health', label: 'Health' },
  { value: 'infrastructure', label: 'Infrastructure' },
  { value: 'emergency', label: 'Emergency Relief' },
  { value: 'environment', label: 'Environment' },
  { value: 'community', label: 'Community Development' },
  { value: 'other', label: 'Other' },
]

const CURRENCIES = [
  { value: 'KES', label: 'KES - Kenyan Shilling' },
  { value: 'USD', label: 'USD - US Dollar' },
  { value: 'EUR', label: 'EUR - Euro' },
  { value: 'GBP', label: 'GBP - British Pound' },
]

export function CampaignCreateModal({ isOpen, onClose }: CampaignCreateModalProps) {
  const createCampaign = useCreateCampaign()
  const { data: nodes } = useNodes()

  const [formData, setFormData] = useState({
    name: '',
    description: '',
    short_description: '',
    target_amount: '',
    currency: 'KES',
    target_node_id: '',
    category: '',
    is_public: true,
    allow_overflow: false,
    ends_at: '',
    status: 'draft' as CampaignStatus,
  })

  const handleSubmit = async (e: React.FormEvent, saveAsDraft = false) => {
    e.preventDefault()

    try {
      await createCampaign.mutateAsync({
        name: formData.name,
        description: formData.description || null,
        short_description: formData.short_description || null,
        image_url: null,
        target_amount: parseFloat(formData.target_amount),
        currency: formData.currency,
        target_node_id: formData.target_node_id || null,
        status: saveAsDraft ? 'draft' : 'active',
        allow_overflow: formData.allow_overflow,
        overflow_campaign_id: null,
        overflow_consent_message: null,
        starts_at: new Date().toISOString(),
        ends_at: formData.ends_at ? new Date(formData.ends_at).toISOString() : null,
        is_public: formData.is_public,
        public_slug: null, // Will be auto-generated
        category: formData.category || null,
        tags: null,
        metadata: {},
        created_by: null,
      })
      onClose()
      resetForm()
    } catch (error) {
      console.error('Failed to create campaign:', error)
    }
  }

  const resetForm = () => {
    setFormData({
      name: '',
      description: '',
      short_description: '',
      target_amount: '',
      currency: 'KES',
      target_node_id: '',
      category: '',
      is_public: true,
      allow_overflow: false,
      ends_at: '',
      status: 'draft',
    })
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative z-10 w-full max-w-lg max-h-[90vh] overflow-auto rounded-lg border bg-card shadow-lg">
        {/* Header */}
        <div className="flex items-center justify-between border-b px-6 py-4">
          <h2 className="text-lg font-semibold">Create Campaign</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-5 w-5" />
          </Button>
        </div>

        {/* Form */}
        <form onSubmit={(e) => handleSubmit(e, false)} className="p-6 space-y-4">
          {/* Name */}
          <div className="space-y-2">
            <Label htmlFor="name">Campaign Name *</Label>
            <Input
              id="name"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="e.g., Clean Water for Schools 2024"
              required
            />
          </div>

          {/* Short Description */}
          <div className="space-y-2">
            <Label htmlFor="short_description">Short Description</Label>
            <Input
              id="short_description"
              value={formData.short_description}
              onChange={(e) =>
                setFormData({ ...formData, short_description: e.target.value })
              }
              placeholder="Brief summary for cards (max 150 chars)"
              maxLength={150}
            />
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">Full Description</Label>
            <Textarea
              id="description"
              value={formData.description}
              onChange={(e) =>
                setFormData({ ...formData, description: e.target.value })
              }
              placeholder="Detailed description of the campaign..."
              rows={3}
            />
          </div>

          {/* Amount and Currency */}
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="target_amount">Target Amount *</Label>
              <Input
                id="target_amount"
                type="number"
                min="1"
                step="0.01"
                value={formData.target_amount}
                onChange={(e) =>
                  setFormData({ ...formData, target_amount: e.target.value })
                }
                placeholder="100000"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="currency">Currency</Label>
              <Select
                value={formData.currency}
                items={CURRENCIES}
                onValueChange={(value) =>
                  setFormData({ ...formData, currency: value ?? '' })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CURRENCIES.map((c) => (
                    <SelectItem key={c.value} value={c.value}>
                      {c.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Target Node */}
          <div className="space-y-2">
            <Label htmlFor="target_node">Target Project/Department</Label>
            <Select
              value={formData.target_node_id}
              items={{
                '': 'Select target...',
                ...Object.fromEntries(
                  (nodes ?? []).map((node) => [
                    node.id,
                    `${node.name} (${node.type})`,
                  ]),
                ),
              }}
              onValueChange={(value) =>
                setFormData({ ...formData, target_node_id: value ?? '' })
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {nodes?.map((node) => (
                  <SelectItem key={node.id} value={node.id}>
                    {node.name} ({node.type})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Category */}
          <div className="space-y-2">
            <Label htmlFor="category">Category</Label>
            <Select
              value={formData.category}
              items={[{ value: '', label: 'Select category...' }, ...CATEGORIES]}
              onValueChange={(value) =>
                setFormData({ ...formData, category: value ?? '' })
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CATEGORIES.map((c) => (
                  <SelectItem key={c.value} value={c.value}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* End Date */}
          <div className="space-y-2">
            <Label htmlFor="ends_at">End Date (Optional)</Label>
            <Input
              id="ends_at"
              type="date"
              value={formData.ends_at}
              onChange={(e) => setFormData({ ...formData, ends_at: e.target.value })}
              min={new Date().toISOString().split('T')[0]}
            />
          </div>

          {/* Options */}
          <div className="space-y-3 pt-2">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.is_public}
                onChange={(e) =>
                  setFormData({ ...formData, is_public: e.target.checked })
                }
                className="rounded border-input"
              />
              <span className="text-sm">Make campaign public</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.allow_overflow}
                onChange={(e) =>
                  setFormData({ ...formData, allow_overflow: e.target.checked })
                }
                className="rounded border-input"
              />
              <span className="text-sm">Allow donations beyond target</span>
            </label>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={(e) => handleSubmit(e, true)}
              disabled={createCampaign.isPending}
            >
              {createCampaign.isPending && (
                <CircleNotch className="h-4 w-4 mr-2 animate-spin" />
              )}
              Save as Draft
            </Button>
            <Button type="submit" disabled={createCampaign.isPending}>
              {createCampaign.isPending && (
                <CircleNotch className="h-4 w-4 mr-2 animate-spin" />
              )}
              Create & Activate
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
