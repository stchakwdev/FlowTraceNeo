import { describe, expect, it } from 'vitest'
import { renderToString } from 'react-dom/server'
import { ThemeProvider } from './theme'

describe('ThemeProvider', () => {
  it('renders its children on the server', () => {
    // Returning null before mount sent an empty <body> to crawlers and curl.
    const html = renderToString(
      <ThemeProvider>
        <h1>Every payment, traced to its proof.</h1>
      </ThemeProvider>,
    )
    expect(html).toContain('traced to its proof')
  })
})
