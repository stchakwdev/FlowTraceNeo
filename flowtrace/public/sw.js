// FlowTrace service worker: lets a field officer open the expense-claim form
// with no connection, so claims can be captured offline and queued on the
// phone (src/lib/offline-claims.ts sends them when the connection returns).
//
// - Pages: network first, falling back to the last copy; a page never seen
//   falls back to the claim form, which is precached at install.
// - Built assets (/assets/*, fonts): cache first; their names are hashed.
const VERSION = 'flowtrace-v1'
const OFFLINE_PAGE = '/spend/claims/new'

async function precache() {
  const cache = await caches.open(VERSION)
  const res = await fetch(OFFLINE_PAGE, { credentials: 'same-origin' })
  if (!res.ok) return
  const html = await res.clone().text()
  await cache.put(OFFLINE_PAGE, res)
  // Also cache the scripts and styles the form page loads.
  const urls = new Set()
  for (const m of html.matchAll(/(?:src|href)="(\/[^"]+\.(?:js|css|woff2?))"/g)) urls.add(m[1])
  await Promise.all(
    [...urls].map((u) => cache.add(u).catch(() => undefined)),
  )
}

self.addEventListener('install', (event) => {
  event.waitUntil(precache().catch(() => undefined).then(() => self.skipWaiting()))
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener('fetch', (event) => {
  const req = event.request
  if (req.method !== 'GET') return
  const url = new URL(req.url)
  if (url.origin !== self.location.origin || url.pathname.startsWith('/api/')) return

  if (req.mode === 'navigate') {
    event.respondWith(
      fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone()
            caches.open(VERSION).then((c) => c.put(req, copy))
          }
          return res
        })
        .catch(async () => (await caches.match(req)) ?? (await caches.match(OFFLINE_PAGE)) ?? Response.error()),
    )
    return
  }

  if (url.pathname.startsWith('/assets/') || /\.(?:woff2?|css|js)$/.test(url.pathname)) {
    event.respondWith(
      caches.match(req).then(
        (hit) =>
          hit ??
          fetch(req).then((res) => {
            if (res.ok) {
              const copy = res.clone()
              caches.open(VERSION).then((c) => c.put(req, copy))
            }
            return res
          }),
      ),
    )
  }
})
