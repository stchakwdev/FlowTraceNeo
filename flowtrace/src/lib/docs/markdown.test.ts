import { describe, expect, it } from 'vitest'
import {
  parseFrontMatter,
  renderMarkdown,
  sanitizeDocHtml,
  slugify,
  toFrontMatter,
} from './markdown'

describe('parseFrontMatter', () => {
  it('splits flat key: value front matter from the body', () => {
    const { data, body } = parseFrontMatter(
      '---\ntitle: Trust Score\ndescription: "How it works: the parts."\norder: 12\nsection: Transparency\n---\n\nBody text.\n',
    )
    expect(data).toEqual({
      title: 'Trust Score',
      description: 'How it works: the parts.',
      order: '12',
      section: 'Transparency',
    })
    expect(body.trim()).toBe('Body text.')
  })

  it('returns the whole text as body when there is no front matter', () => {
    expect(parseFrontMatter('# Hi')).toEqual({ data: {}, body: '# Hi' })
  })

  it('types the four keys and names the file when one is missing', () => {
    expect(
      toFrontMatter(
        { title: 'A', description: 'B', order: '3', section: 'C' },
        'a.md',
      ),
    ).toEqual({ title: 'A', description: 'B', order: 3, section: 'C' })
    expect(() =>
      toFrontMatter({ title: 'A', description: 'B', order: '3' }, 'a.md'),
    ).toThrow('a.md: front matter needs "section"')
    expect(() =>
      toFrontMatter(
        { title: 'A', description: 'B', order: 'two', section: 'C' },
        'a.md',
      ),
    ).toThrow('"order" must be a whole number')
  })
})

describe('renderMarkdown', () => {
  it('gives headings unique ids and lists h2 and h3 for the contents', () => {
    const { html, headings } = renderMarkdown(
      '## Proofs & points\n\ntext\n\n### Visibility\n\n## Proofs & points\n\n#### Small\n',
    )
    expect(html).toContain('<h2 id="proofs-points">Proofs &amp; points</h2>')
    expect(html).toContain('<h2 id="proofs-points-2">')
    expect(html).toContain('<h4 id="small">')
    expect(headings).toEqual([
      { id: 'proofs-points', text: 'Proofs & points', depth: 2 },
      { id: 'visibility', text: 'Visibility', depth: 3 },
      { id: 'proofs-points-2', text: 'Proofs & points', depth: 2 },
    ])
  })

  it('demotes an h1 in the body, since the title is the page h1', () => {
    expect(renderMarkdown('# Top').html).toContain('<h2 id="top">Top</h2>')
  })

  it('keeps external links in the same tab with rel="noopener"', () => {
    const { html } = renderMarkdown(
      '[Releases](https://github.com/x/y/releases) and [Ledger](/docs/ledger)',
    )
    expect(html).toContain(
      '<a href="https://github.com/x/y/releases" rel="noopener">Releases</a>',
    )
    expect(html).toContain('<a href="/docs/ledger">Ledger</a>')
    expect(html).not.toContain('target=')
  })

  it('sizes and lazy-loads an image whose title is WIDTHxHEIGHT', () => {
    const { html } = renderMarkdown(
      '![The dashboard](/docs/img/dashboard.webp "1280x800")',
    )
    expect(html).toContain(
      '<img src="/docs/img/dashboard.webp" alt="The dashboard" width="1280" height="800" loading="lazy" decoding="async" />',
    )
  })

  it('renders GFM tables and drops HTML comments', () => {
    const { html } = renderMarkdown(
      '<!-- Verified against: a.tsx -->\n\n| Kind | Points |\n| --- | --- |\n| Receipt | 10 |\n',
    )
    expect(html).toContain('<th>Kind</th>')
    expect(html).toContain('<td>10</td>')
    expect(html).not.toContain('Verified against')
  })
})

describe('sanitizeDocHtml', () => {
  it('strips scripts, event handlers and javascript: links', () => {
    const { html } = renderMarkdown(
      '<script>alert(1)</script>\n\n<p onclick="x()">hi</p>\n\n[bad](javascript:alert(1))\n\n<img src="x" onerror="alert(1)">\n\n<iframe src="https://evil.test"></iframe>',
    )
    expect(html).not.toMatch(/<script|onclick|onerror|javascript:|<iframe/i)
    expect(html).toContain('<p>hi</p>')
  })

  it('drops style and class attributes', () => {
    expect(sanitizeDocHtml('<p style="color:red" class="x">a</p>')).toBe(
      '<p>a</p>',
    )
  })
})

describe('slugify', () => {
  it('lowercases, drops accents and joins words with hyphens', () => {
    expect(slugify('M-Pesa statements (CSV)')).toBe('m-pesa-statements-csv')
    expect(slugify('Café réglé')).toBe('cafe-regle')
    expect(slugify('!!!')).toBe('section')
  })
})
