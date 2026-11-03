# Marketing site: how the frame works (LP1)

For LP2 to LP6. The marketing pages share one layout, one host value, one
head helper and one analytics helper. Add your route; do not fork these.

## Layout
- `src/routes/_marketing.tsx` is a pathless layout. A page at
  `src/routes/_marketing/pricing.tsx` uses `createFileRoute('/_marketing/pricing')`
  and serves `/pricing`. The landing page is `_marketing/index.tsx` (URL `/`).
- The layout renders the skip link, `MarketingMasthead`
  (`components/marketing/marketing-masthead.tsx`), `<main id="main">` and the
  footer (`components/kokonutui-pro/sitemap-footer.tsx`, Kokonut, restyled).
  Pages render sections only, no `<main>`, masthead or footer of their own.
- Set `staticData: { demoFigures: true }` on a page that shows demo figures;
  the footer then adds "Demo figures as of <date>".
- Links live in `src/lib/marketing-nav.ts` (`MASTHEAD_NAV`, `FOOTER_COLUMNS`,
  `DEMO_PATH`, `SIGN_IN_PATH`). Future pages are plain anchors until they land.
  "Sign in" goes to `/dashboard`: `AUTH_ROUTES.signIn` (`/sign-in`) has no route.

## Head and SEO (`src/lib/site.ts`)
- `SITE_URL` comes from `VITE_SITE_URL` (default
  `https://flow-trace-neo.vercel.app`). Never hardcode the host elsewhere.
- Each route: `head: () => pageHead({ title, description, path })`. It returns
  the title "<title> · FlowTrace", description, canonical, Open Graph (type,
  site_name, locale, title, description, url, image, image:alt) and Twitter
  tags. Pass `image` for a page-specific card; `absoluteTitle` to skip the suffix.
- The root route sets site-wide defaults (no canonical: links are not
  deduplicated across routes, meta tags are, deepest route wins).
- Organization JSON-LD is on `/` only. No FAQPage or SoftwareApplication.
- Icons and `public/og/flowtrace.png` come from `bun scripts/brand-assets.ts`
  (needs rsvg-convert, ImageMagick, Chromium). Rerun it if the mark or
  headline changes.

## Prerender and sitemap
- Prerender is OFF. With `@tanstack/react-start` 1.142 and the Nitro vercel
  preset, `tanstackStart({ prerender })` fails the build ("Failed to start the
  Vite preview server for prerendering": Start prerenders through
  `vite preview`, which needs `dist/server/server.js`; Nitro writes
  `.vercel/output` instead), and Nitro skips its own prerender under the Vite
  builder. Marketing pages are server-rendered per request.
- `server/routes/sitemap.xml.ts` and `robots.txt.ts` serve both files from
  `SITEMAP_PATHS` and `SITE_URL`. When your route lands, add its path to
  `SITEMAP_PATHS`; `site.test.ts` fails if a listed path is not a route.
- SSR check: `curl -s localhost:3000/ | grep -c "traced to its proof"`.
  `vite preview` does not run with the Nitro preset; use the dev server.

## Analytics (`src/lib/analytics.ts`)
- `track(event, props?)` calls `window.umami.track` when Umami is loaded and
  does nothing otherwise. The script loads only when `VITE_UMAMI_WEBSITE_ID` is set.
- Events: `demo_started`, `cta_click` `{ location }`, `pricing_viewed`,
  `security_viewed`, `docs_viewed`, `file_download` `{ name }`, `outbound_click`.
  Demo CTAs call `demoStarted(location)` from the masthead module.

## Parity rows and tests
- Every new route needs a row in `rust/docs/screen-parity.md` marked not
  applicable (marketing); run `cargo test -p desktop --test screen_parity --locked`.
- Render tests use `renderWithRouter` from `src/test/render-with-router.tsx`;
  test the page's headings, primary CTA and `Route.options.head()`.

## Docs (`/docs`, LP4)
- Pages are Markdown in `content/docs/<slug>.md` with front matter `title`,
  `description` (160 characters at most), `order` (unique) and `section`. The
  first body line is `<!-- Verified against: <repo paths> -->`; re-check those
  files when you edit a page. No MDX, no other HTML.
- `src/lib/docs/markdown.ts` parses front matter, renders with `marked` (GFM),
  gives headings ids and sanitizes with `sanitize-html`. `content.ts` bundles
  the files with `import.meta.glob(?raw)`; the routes load them through the
  server functions in `docs-server.ts`, so the browser gets HTML only.
- Images: WebP under `public/docs/img/`, written as
  `![alt](/docs/img/x.webp "1280x800")` (the title is the size), two per page
  at most. `content.test.ts` checks links, images, front matter and sources.
- The sitemap route adds `DOC_PATHS`; a new page needs no other registration.
