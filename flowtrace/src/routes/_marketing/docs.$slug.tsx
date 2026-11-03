import { createFileRoute, notFound } from '@tanstack/react-router'
import { DocArticlePage } from '@/components/docs/docs-page'
import { getDocPage } from '@/lib/docs/docs-server'
import { pageHead } from '@/lib/site'

export const Route = createFileRoute('/_marketing/docs/$slug')({
  loader: async ({ params }) => {
    const { shell, view } = await getDocPage({ data: params.slug })
    if (!view) throw notFound()
    return { shell, view }
  },
  staleTime: Infinity,
  // The screenshots show the bundled demo data; the footer dates it.
  staticData: { demoFigures: true },
  head: ({ loaderData, params }) =>
    loaderData
      ? pageHead({
          title: loaderData.view.page.title,
          description: loaderData.view.page.description,
          path: `/docs/${params.slug}`,
        })
      : {},
  component: DocRoute,
})

function DocRoute() {
  const { shell, view } = Route.useLoaderData()
  return <DocArticlePage shell={shell} view={view} />
}
