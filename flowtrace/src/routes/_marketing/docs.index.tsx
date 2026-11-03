import { createFileRoute } from '@tanstack/react-router'
import { DocsIndexPage } from '@/components/docs/docs-page'
import { getDocsShell } from '@/lib/docs/docs-server'
import { pageHead } from '@/lib/site'

export const DOCS_TITLE = 'Documentation'
export const DOCS_DESCRIPTION =
  'How to use FlowTrace, screen by screen: the demo, transactions and proofs, the books, reconciling, reports, public pages and the Trust Score.'

export const Route = createFileRoute('/_marketing/docs/')({
  loader: () => getDocsShell(),
  staleTime: Infinity,
  head: () =>
    pageHead({
      title: DOCS_TITLE,
      description: DOCS_DESCRIPTION,
      path: '/docs',
    }),
  component: DocsIndexRoute,
})

function DocsIndexRoute() {
  return <DocsIndexPage shell={Route.useLoaderData()} />
}
