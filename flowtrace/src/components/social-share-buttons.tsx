import {
  FacebookLogo,
  LinkedinLogo,
  TwitterLogo,
  WhatsappLogo,
  Copy,
  Check,
} from '@phosphor-icons/react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface SocialShareButtonsProps {
  url: string
  title: string
  text?: string
  className?: string
}

export function SocialShareButtons({
  url,
  title,
  text,
  className,
}: SocialShareButtonsProps) {
  const [copied, setCopied] = useState(false)

  const encodedUrl = encodeURIComponent(url)
  const encodedTitle = encodeURIComponent(title)
  const encodedText = encodeURIComponent(text || title)

  const shareLinks = {
    twitter: `https://twitter.com/intent/tweet?url=${encodedUrl}&text=${encodedText}`,
    facebook: `https://www.facebook.com/sharer/sharer.php?u=${encodedUrl}`,
    linkedin: `https://www.linkedin.com/sharing/share-offsite/?url=${encodedUrl}`,
    whatsapp: `https://wa.me/?text=${encodedText}%20${encodedUrl}`,
  }

  const handleCopyLink = async () => {
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  const handleShare = (platform: keyof typeof shareLinks) => {
    window.open(shareLinks[platform], '_blank', 'width=600,height=400')
  }

  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      <Button
        variant="outline"
        size="icon"
        onClick={() => handleShare('twitter')}
        className="hover:bg-[#1DA1F2]/10 hover:border-[#1DA1F2] hover:text-[#1DA1F2]"
        aria-label="Share on Twitter"
      >
        <TwitterLogo className="h-5 w-5" weight="fill" />
      </Button>

      <Button
        variant="outline"
        size="icon"
        onClick={() => handleShare('facebook')}
        className="hover:bg-[#1877F2]/10 hover:border-[#1877F2] hover:text-[#1877F2]"
        aria-label="Share on Facebook"
      >
        <FacebookLogo className="h-5 w-5" weight="fill" />
      </Button>

      <Button
        variant="outline"
        size="icon"
        onClick={() => handleShare('linkedin')}
        className="hover:bg-[#0A66C2]/10 hover:border-[#0A66C2] hover:text-[#0A66C2]"
        aria-label="Share on LinkedIn"
      >
        <LinkedinLogo className="h-5 w-5" weight="fill" />
      </Button>

      <Button
        variant="outline"
        size="icon"
        onClick={() => handleShare('whatsapp')}
        className="hover:bg-[#25D366]/10 hover:border-[#25D366] hover:text-[#25D366]"
        aria-label="Share on WhatsApp"
      >
        <WhatsappLogo className="h-5 w-5" weight="fill" />
      </Button>

      <Button
        variant="outline"
        onClick={handleCopyLink}
        className={cn(
          'ml-2 gap-2',
          copied && 'text-emerald-600 border-emerald-600',
        )}
      >
        {copied ? (
          <>
            <Check className="h-4 w-4" />
            Copied!
          </>
        ) : (
          <>
            <Copy className="h-4 w-4" />
            Copy Link
          </>
        )}
      </Button>
    </div>
  )
}
