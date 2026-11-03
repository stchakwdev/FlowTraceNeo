/**
 * Umami analytics. The script loads only when VITE_UMAMI_WEBSITE_ID is set
 * (see `umamiScript`, used by __root.tsx); without it `track` does nothing.
 * Events: demo_started, cta_click { location }, pricing_viewed,
 * security_viewed, docs_viewed, file_download { name }, outbound_click.
 */

export type AnalyticsEvent =
  | 'demo_started'
  | 'cta_click'
  | 'pricing_viewed'
  | 'security_viewed'
  | 'docs_viewed'
  | 'file_download'
  | 'outbound_click'

type Props = Record<string, string | number | boolean>

declare global {
  interface Window {
    umami?: { track: (event: string, props?: Props) => void }
  }
}

export const UMAMI_SCRIPT_URL = 'https://cloud.umami.is/script.js'

export function track(event: AnalyticsEvent, props?: Props): void {
  if (typeof window === 'undefined') return
  try {
    window.umami?.track(event, props)
  } catch {
    // Analytics must never break a click.
  }
}

/** The `head().scripts` entry for Umami, or nothing when no website id is set. */
export function umamiScript(
  websiteId?: string,
): Array<{ src: string; defer: true; 'data-website-id': string }> {
  const id = (
    websiteId ?? (import.meta.env.VITE_UMAMI_WEBSITE_ID as string | undefined)
  )?.trim()
  return id
    ? [{ src: UMAMI_SCRIPT_URL, defer: true, 'data-website-id': id }]
    : []
}
