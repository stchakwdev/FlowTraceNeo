import { createServerFn } from '@tanstack/react-start'
import { createReleaseLoader } from './books-release'

// One cache per server instance: the /books loader reads GitHub at most once every ten
// minutes, on the server only, whether the page is rendered there or navigated to.
const loadLatestRelease = createReleaseLoader()

export const getBooksRelease = createServerFn({ method: 'GET' }).handler(() =>
  loadLatestRelease(),
)
