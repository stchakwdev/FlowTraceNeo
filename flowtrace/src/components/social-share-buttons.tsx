import { useState } from 'react'
import {
  Check,
  Copy,
  FacebookLogo,
  LinkedinLogo,
  WhatsappLogo,
  XLogo,
} from '@phosphor-icons/react'
import { Button, buttonVariants } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface SocialShareButtonsProps {
  url: string
  title: string
  text?: string
  className?: string
}

export function SocialShareButtons({ url, title, text, className }: SocialShareButtonsProps) {
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle')
  const encodedUrl = encodeURIComponent(url)
  const encodedText = encodeURIComponent(text || title)

  const links = [
    {
      name: 'WhatsApp',
      icon: WhatsappLogo,
      href: `https://wa.me/?text=${encodedText}%20${encodedUrl}`,
    },
    {
      name: 'X',
      icon: XLogo,
      href: `https://twitter.com/intent/tweet?url=${encodedUrl}&text=${encodedText}`,
    },
    {
      name: 'Facebook',
      icon: FacebookLogo,
      href: `https://www.facebook.com/sharer/sharer.php?u=${encodedUrl}`,
    },
    {
      name: 'LinkedIn',
      icon: LinkedinLogo,
      href: `https://www.linkedin.com/sharing/share-offsite/?url=${encodedUrl}`,
    },
  ]

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url)
      setCopyState('copied')
    } catch {
      setCopyState('failed')
    }
    setTimeout(() => setCopyState('idle'), 2500)
  }

  return (
    <div className={cn('flex flex-wrap items-center gap-2', className)}>
      {links.map(({ name, icon: Icon, href }) => (
        <a
          key={name}
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Share on ${name}`}
          className={buttonVariants({ variant: 'outline', size: 'icon', className: 'size-10' })}
        >
          <Icon className="size-5" />
        </a>
      ))}
      <Button variant="outline" className="h-10" onClick={() => void copy()}>
        {copyState === 'copied' ? <Check /> : <Copy />}
        {copyState === 'copied'
          ? 'Link copied'
          : copyState === 'failed'
            ? 'Copy failed, select the address bar'
            : 'Copy link'}
      </Button>
      <span role="status" className="sr-only">
        {copyState === 'copied' ? 'Link copied to clipboard' : ''}
      </span>
    </div>
  )
}
