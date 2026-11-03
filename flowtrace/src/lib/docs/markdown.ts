/**
 * The docs pipeline's pure parts: front matter, Markdown to HTML with heading
 * ids and a table of contents, and the sanitizer every page goes through. The
 * content is ours, but the sanitizer stays so a future page cannot carry a
 * script. Runs on the server only (see docs-server.ts).
 */
import { Marked } from 'marked'
import sanitizeHtml from 'sanitize-html'

export interface DocFrontMatter {
  title: string
  description: string
  order: number
  section: string
}

export interface DocHeading {
  id: string
  text: string
  depth: 2 | 3
}

export const REQUIRED_FRONT_MATTER = [
  'title',
  'description',
  'order',
  'section',
] as const

/**
 * Splits `---` front matter from the body. Only flat `key: value` lines are
 * allowed, which is all the docs need; quotes around a value are dropped.
 */
export function parseFrontMatter(raw: string): {
  data: Record<string, string>
  body: string
} {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw)
  if (!match) return { data: {}, body: raw }
  const data: Record<string, string> = {}
  for (const line of match[1].split(/\r?\n/)) {
    if (!line.trim() || line.trimStart().startsWith('#')) continue
    const colon = line.indexOf(':')
    if (colon < 1) throw new Error(`Bad front matter line: ${line}`)
    const key = line.slice(0, colon).trim()
    const value = line
      .slice(colon + 1)
      .trim()
      .replace(/^(['"])(.*)\1$/, '$2')
    data[key] = value
  }
  return { data, body: raw.slice(match[0].length) }
}

/** Checks the four required keys and types `order`; throws naming the file. */
export function toFrontMatter(
  data: Record<string, string>,
  file: string,
): DocFrontMatter {
  for (const key of REQUIRED_FRONT_MATTER) {
    if (!data[key]) throw new Error(`${file}: front matter needs "${key}"`)
  }
  const order = Number(data.order)
  if (!Number.isInteger(order))
    throw new Error(`${file}: "order" must be a whole number`)
  return {
    title: data.title,
    description: data.description,
    order,
    section: data.section,
  }
}

const ENTITIES: Record<string, string> = {
  '&amp;': '&',
  '&lt;': '<',
  '&gt;': '>',
  '&quot;': '"',
  '&#39;': "'",
}

function plainText(html: string): string {
  return html
    .replace(/<[^>]*>/g, '')
    .replace(/&(amp|lt|gt|quot|#39);/g, (e) => ENTITIES[e])
    .trim()
}

/** "Proofs & trust points" -> "proofs-trust-points". */
export function slugify(text: string): string {
  return (
    text
      .toLowerCase()
      .normalize('NFKD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'section'
  )
}

function escapeAttr(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;')
}

const SANITIZE: sanitizeHtml.IOptions = {
  allowedTags: [
    'h2',
    'h3',
    'h4',
    'p',
    'a',
    'ul',
    'ol',
    'li',
    'strong',
    'em',
    'del',
    'code',
    'pre',
    'blockquote',
    'hr',
    'br',
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
    'img',
    'figure',
    'figcaption',
  ],
  allowedAttributes: {
    h2: ['id'],
    h3: ['id'],
    h4: ['id'],
    a: ['href', 'rel', 'title'],
    img: ['src', 'alt', 'width', 'height', 'loading', 'decoding'],
    th: ['align'],
    td: ['align'],
    ol: ['start'],
  },
  allowedSchemes: ['https', 'http', 'mailto'],
  allowedSchemesAppliedToAttributes: ['href', 'src'],
  allowProtocolRelative: false,
  // HTML comments (the "Verified against" notes) are dropped here.
}

/** Strips anything outside the docs' tag and attribute allowlist. */
export function sanitizeDocHtml(html: string): string {
  return sanitizeHtml(html, SANITIZE)
}

/**
 * Renders a page body. Headings get unique ids and h2/h3 feed the table of
 * contents; links leaving the site keep the same tab with rel="noopener";
 * an image whose title is "WIDTHxHEIGHT" is sized and lazy-loaded.
 */
export function renderMarkdown(body: string): {
  html: string
  headings: Array<DocHeading>
} {
  const headings: Array<DocHeading> = []
  const used = new Map<string, number>()
  const marked = new Marked({ gfm: true })
  marked.use({
    renderer: {
      heading({ tokens, depth }) {
        const inner = this.parser.parseInline(tokens)
        const level = Math.min(Math.max(depth, 2), 4)
        const text = plainText(inner)
        const base = slugify(text)
        const seen = used.get(base) ?? 0
        used.set(base, seen + 1)
        const id = seen ? `${base}-${seen + 1}` : base
        if (level === 2 || level === 3)
          headings.push({ id, text, depth: level })
        return `<h${level} id="${id}">${inner}</h${level}>\n`
      },
      link({ href, title, tokens }) {
        const inner = this.parser.parseInline(tokens)
        const rel = /^https?:\/\//.test(href) ? ' rel="noopener"' : ''
        const t = title ? ` title="${escapeAttr(title)}"` : ''
        return `<a href="${escapeAttr(href)}"${t}${rel}>${inner}</a>`
      },
      image({ href, title, text }) {
        const size = title ? /^(\d+)x(\d+)$/.exec(title) : null
        const dims = size ? ` width="${size[1]}" height="${size[2]}"` : ''
        return `<img src="${escapeAttr(href)}" alt="${escapeAttr(text)}"${dims} loading="lazy" decoding="async">`
      },
    },
  })
  const html = sanitizeDocHtml(marked.parse(body, { async: false }))
  return { html, headings }
}
