/**
 * Draws FlowTrace's icons and Open Graph image from the FlowTraceMark
 * (src/components/layout/wordmark.tsx) and the landing headline.
 *
 *   cd flowtrace && bun scripts/brand-assets.ts
 *
 * Needs rsvg-convert, ImageMagick (`magick`) and Chromium on PATH. Writes
 * public/favicon.svg, favicon.ico, apple-touch-icon.png, logo192.png,
 * logo512.png and og/flowtrace.png; scratch files go under
 * node_modules/.cache/brand-assets.
 */
import { execFileSync } from 'node:child_process'
import { mkdirSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { resolveSiteUrl } from '../src/lib/site'

const ROOT = path.resolve(import.meta.dirname, '..')
const PUBLIC = path.join(ROOT, 'public')
const CACHE = path.join(ROOT, 'node_modules/.cache/brand-assets')
mkdirSync(CACHE, { recursive: true })
mkdirSync(path.join(PUBLIC, 'og'), { recursive: true })

// Light-theme tokens from src/styles.css (--masthead/--foreground, --background).
const INK = '#1d2431'
const PAPER = '#f5f6f3'
const HEADLINE = 'Every payment, traced to its proof.'
const HOST = new URL(resolveSiteUrl(process.env.VITE_SITE_URL)).host

/** The mark's three stations on one line, drawn in a 28 x 12 box. */
function mark(fg: string, hollow: string): string {
  return [
    `<line x1="3" y1="6" x2="14" y2="6" stroke="${fg}" stroke-width="2.5"/>`,
    `<line x1="14" y1="6" x2="25" y2="6" stroke="${fg}" stroke-width="2.5" stroke-dasharray="3 2.5"/>`,
    `<rect x="0.5" y="3.5" width="5" height="5" fill="${fg}"/>`,
    `<rect x="11.5" y="3.5" width="5" height="5" fill="${hollow}" stroke="${fg}" stroke-width="2"/>`,
    `<rect x="22.5" y="3.5" width="5" height="5" fill="${hollow}" stroke="${fg}" stroke-width="2"/>`,
  ].join('')
}

/** A square ink tile with the mark in paper, centred. */
function iconSvg(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" fill="${INK}"/><g transform="translate(2 10)">${mark(PAPER, INK)}</g></svg>\n`
}

function run(cmd: string, args: Array<string>) {
  execFileSync(cmd, args, { stdio: 'inherit' })
}

const icon = iconSvg()
writeFileSync(path.join(PUBLIC, 'favicon.svg'), icon)
const iconFile = path.join(CACHE, 'icon.svg')
writeFileSync(iconFile, icon)
for (const [name, size] of [
  ['apple-touch-icon.png', 180],
  ['logo192.png', 192],
  ['logo512.png', 512],
] as const) {
  run('rsvg-convert', [
    '-w',
    String(size),
    '-h',
    String(size),
    '-o',
    path.join(PUBLIC, name),
    iconFile,
  ])
}
const ico = [16, 32, 48].map((size) => {
  const file = path.join(CACHE, `icon-${size}.png`)
  run('rsvg-convert', [
    '-w',
    String(size),
    '-h',
    String(size),
    '-o',
    file,
    iconFile,
  ])
  return file
})
run('magick', [...ico, path.join(PUBLIC, 'favicon.ico')])

// The Open Graph image: report layout, rendered by Chromium so the
// headline is set in the app's own Archivo.
const font = pathToFileURL(
  path.join(
    ROOT,
    'node_modules/@fontsource-variable/archivo/files/archivo-latin-wdth-normal.woff2',
  ),
).href
const html = `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face { font-family: Archivo; src: url('${font}') format('woff2-variations'); font-weight: 100 900; font-stretch: 62% 125%; }
* { margin: 0; box-sizing: border-box; }
html, body { width: 1200px; height: 630px; overflow: hidden; }
body { background: ${PAPER}; color: ${INK}; font-family: Archivo, sans-serif; padding: 64px 80px; display: flex; flex-direction: column; }
.top { display: flex; align-items: center; justify-content: space-between; padding-bottom: 28px; border-bottom: 6px solid ${INK}; }
.word { display: flex; align-items: center; gap: 18px; font-size: 44px; font-weight: 800; font-stretch: 75%; letter-spacing: -0.01em; }
.label { font-size: 20px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
h1 { margin-top: 64px; font-size: 118px; line-height: 0.95; font-weight: 800; font-stretch: 75%; letter-spacing: -0.01em; max-width: 960px; }
.foot { margin-top: auto; display: flex; justify-content: space-between; align-items: end; font-size: 24px; }
</style></head><body>
<div class="top"><div class="word"><svg width="84" height="36" viewBox="0 0 28 12">${mark(INK, PAPER)}</svg>FlowTrace</div><div class="label">The situation report for your money</div></div>
<h1>${HEADLINE}</h1>
<div class="foot"><span>Grants, transfers and payments, each with its proof attached.</span><span>${HOST}</span></div>
</body></html>`
const htmlFile = path.join(CACHE, 'og.html')
writeFileSync(htmlFile, html)
const shot = path.join(CACHE, 'og-raw.png')
run('chromium', [
  '--headless=new',
  '--disable-gpu',
  '--hide-scrollbars',
  '--force-device-scale-factor=1',
  '--window-size=1200,630',
  '--virtual-time-budget=3000',
  `--screenshot=${shot}`,
  pathToFileURL(htmlFile).href,
])
const og = path.join(PUBLIC, 'og/flowtrace.png')
run('magick', [
  shot,
  '-crop',
  '1200x630+0+0',
  '+repage',
  '-strip',
  '-colors',
  '64',
  `PNG8:${og}`,
])

const kb = Math.round(statSync(og).size / 1024)
console.log(`og/flowtrace.png: ${kb} KB`)
if (kb >= 300) throw new Error('og/flowtrace.png must stay under 300 KB')
