import { createServerFn } from '@tanstack/react-start'
import { docsShell, getDoc } from './content'
import type { DocView, DocsShell } from './content'

// The docs routes load through these, so the Markdown, marked and the
// sanitizer stay on the server; the browser receives rendered HTML only.

export const getDocsShell = createServerFn({ method: 'GET' }).handler(
  (): DocsShell => docsShell(),
)

export const getDocPage = createServerFn({ method: 'GET' })
  .inputValidator((slug: unknown) => {
    if (typeof slug !== 'string') throw new Error('slug must be a string')
    return slug
  })
  .handler(({ data }): { shell: DocsShell; view: DocView | null } => ({
    shell: docsShell(),
    view: getDoc(data),
  }))
