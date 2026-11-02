import { createFileRoute } from '@tanstack/react-router'
import { BooksPage } from '@/components/books/books-page'
import { HERO_SCREEN } from '@/components/books/books-screens'
import { RELEASE_CACHE_MS } from '@/lib/books-release'
import { getBooksRelease } from '@/lib/books-release-server'
import { pageHead } from '@/lib/site'

export const BOOKS_TITLE = 'FlowTrace Books · the books, offline, native'
export const BOOKS_DESCRIPTION =
  'FlowTrace Books is a desktop app for double-entry books, for organisations that work where the connection drops. Grants, claims, bills, M-Pesa reconciliation and statements, for Linux and macOS.'

export const Route = createFileRoute('/_marketing/books')({
  loader: () => getBooksRelease(),
  staleTime: RELEASE_CACHE_MS,
  head: () =>
    pageHead({
      title: BOOKS_TITLE,
      description: BOOKS_DESCRIPTION,
      path: '/books',
      absoluteTitle: true,
      image: {
        path: HERO_SCREEN.src,
        alt: 'The FlowTrace Books general ledger: posted lines of the main bank account with their running balance.',
        width: HERO_SCREEN.width,
        height: HERO_SCREEN.height,
      },
    }),
  staticData: { demoFigures: true },
  component: BooksRoute,
})

function BooksRoute() {
  return <BooksPage release={Route.useLoaderData()} />
}
