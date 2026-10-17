// IT1's web side: one browser context per person, driving the Books screens with Playwright
// (playwright-core, a cached Chromium), and every command the browser sends with its result read
// off the WebSocket (it1-frames.ts) for the oracle.
//
// Dummy auth: the person's device token goes into local storage before the app loads
// (`flowtrace.spacetimedb.token`, flowtrace/src/lib/ledger/spacetime-backend.ts:67-75), so the
// browser connects as that member. Selectors are the screens' accessible names (labels, button
// text, roles); no test ids were needed.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { chromium } from 'playwright-core'
import type { Browser, BrowserContext, Locator, Page } from 'playwright-core'
import { receivedResults, sentCommands } from './it1-frames'
import type { Command } from './it1-frames'
import type { DomReport, DomRow } from './it1-report'

export const TOKEN_KEY = 'flowtrace.spacetimedb.token'

/**
 * The Chromium to drive: `IT1_CHROMIUM`, else the newest one Playwright cached
 * (~/.cache/ms-playwright/chromium-*), else Playwright's own default.
 */
export function chromiumPath(): string | undefined {
  if (process.env.IT1_CHROMIUM) return process.env.IT1_CHROMIUM
  const cache = path.join(os.homedir(), '.cache/ms-playwright')
  const dirs = fs.existsSync(cache) ? fs.readdirSync(cache).filter((d) => /^chromium-\d+$/.test(d)) : []
  dirs.sort((a, b) => Number(b.split('-')[1]) - Number(a.split('-')[1]))
  for (const d of dirs) {
    const exe = path.join(cache, d, 'chrome-linux64/chrome')
    if (fs.existsSync(exe)) return exe
  }
  return undefined
}

export const launchBrowser = (headed: boolean): Promise<Browser> => chromium.launch({ executablePath: chromiumPath(), headless: !headed })

/** One command the browser sent, and what came back. */
export interface WebCommand {
  person: string
  requestId: string
  command: Command
  sentAt: number
  result?: Record<string, unknown>
}

export interface WebPerson {
  key: string
  page: Page
  context: BrowserContext
  commands: Array<WebCommand>
  /** Console errors, for the transcript. */
  errors: Array<string>
  shot: (name: string) => Promise<string>
  close: () => Promise<void>
}

/** A browser context signed in as `key` (dummy mode: its device token in local storage). */
export async function webPerson(browser: Browser, key: string, token: string, shotsDir: string): Promise<WebPerson> {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } })
  await context.addInitScript(([k, t]) => localStorage.setItem(k, t), [TOKEN_KEY, token] as const)
  const page = await context.newPage()
  const commands: Array<WebCommand> = []
  const errors: Array<string> = []
  const bytes = (p: string | Buffer) => new Uint8Array(typeof p === 'string' ? Buffer.from(p, 'utf8') : p)
  page.on('websocket', (ws) => {
    ws.on('framesent', (f) => {
      for (const c of sentCommands(bytes(f.payload))) commands.push({ person: key, requestId: c.requestId, command: c.command, sentAt: Date.now() })
    })
    ws.on('framereceived', (f) => {
      for (const r of receivedResults(bytes(f.payload))) {
        const c = commands.find((x) => x.requestId === r.requestId)
        if (c) c.result = r.result
      }
    })
  })
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text().slice(0, 300))
  })
  fs.mkdirSync(shotsDir, { recursive: true })
  let n = 0
  return {
    key,
    page,
    context,
    commands,
    errors,
    shot: async (name) => {
      const file = path.join(shotsDir, `${String(++n).padStart(2, '0')}-${key}-${name}.png`)
      await page.screenshot({ path: file, fullPage: true })
      return file
    },
    close: () => context.close(),
  }
}

/** Waits until every command sent since `from` has its result, or `ms` pass. */
export async function settled(p: WebPerson, from: number, ms = 15000): Promise<Array<WebCommand>> {
  const end = Date.now() + ms
  for (;;) {
    const mine = p.commands.slice(from)
    if (mine.every((c) => c.result) || Date.now() > end) return mine
    await p.page.waitForTimeout(100)
  }
}

/** Picks the option whose text starts with `prefix` from a Base UI select (`FormSelect`). */
async function choose(select: Locator, prefix: string | RegExp) {
  await select.click()
  const page = select.page()
  const name = typeof prefix === 'string' ? new RegExp(`^${prefix.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`) : prefix
  const option = page.getByRole('option', { name }).first()
  await option.waitFor()
  // An option under the popup's scroll arrow cannot take a pointer click: highlight it from the
  // keyboard's side instead (focus, then Enter), as a keyboard user would.
  await option.click({ timeout: 3000 }).catch(async () => {
    await option.focus()
    await page.keyboard.press('Enter')
  })
  await option.waitFor({ state: 'hidden', timeout: 5000 })
}

const openBooks = async (page: Page, base: string, route: string, ready: Locator) => {
  await page.goto(`${base}${route}`)
  await ready.waitFor({ timeout: 30000 })
}

/** Ledger > Accounts > Add an account (`account.add`). */
export async function addAccount(p: WebPerson, base: string, a: { code: string; name: string }) {
  const { page } = p
  await openBooks(page, base, '/ledger/accounts', page.getByRole('button', { name: 'Add an account' }))
  await page.getByRole('button', { name: 'Add an account' }).click()
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('textbox', { name: /^Code/ }).fill(a.code)
  await dialog.getByRole('textbox', { name: /^Name/ }).fill(a.name)
  await choose(dialog.getByRole('combobox', { name: /^Type/ }), 'Expense')
  await dialog.getByRole('button', { name: 'Add account' }).click()
  await dialog.waitFor({ state: 'hidden', timeout: 15000 })
  await page.getByText(a.name).first().waitFor({ timeout: 15000 })
}

/** Ledger > New entry, two lines, Post (`entry.post`); returns the entry's page URL. */
export async function postEntry(p: WebPerson, base: string, e: { memo: string; debit: string; credit: string; amount: string }) {
  const { page } = p
  await openBooks(page, base, '/ledger/new', page.getByRole('textbox', { name: /^Memo/ }))
  await page.getByRole('textbox', { name: /^Memo/ }).fill(e.memo)
  const line = (n: number) => page.getByRole('group', { name: `Line ${n}` })
  await choose(line(1).getByRole('combobox', { name: /^Account/ }), e.debit)
  await line(1).getByRole('textbox', { name: /^Debit/ }).fill(e.amount)
  await choose(line(2).getByRole('combobox', { name: /^Account/ }), e.credit)
  await line(2).getByRole('textbox', { name: /^Credit/ }).fill(e.amount)
  await page.getByRole('button', { name: 'Post', exact: true }).click()
}

/** Spend > New expense claim, Submit for approval (`claim.create` with submit). */
export async function fillClaim(p: WebPerson, base: string, c: { purpose: string; description: string; amount: string; account: string }) {
  const { page } = p
  await openBooks(page, base, '/spend/claims/new', page.getByRole('textbox', { name: /^Purpose/ }))
  await page.getByRole('textbox', { name: /^Purpose/ }).fill(c.purpose)
  await page.getByRole('textbox', { name: /^What it was for/ }).fill(c.description)
  await page.getByRole('textbox', { name: /^Amount, KES/ }).fill(c.amount)
  await choose(page.getByRole('combobox', { name: /^Kind of cost/ }), c.account)
  await page.getByRole('button', { name: 'Submit for approval' }).click()
}

/** Spend > New bill from an existing supplier, Save bill (`bill.create`). */
export async function enterBill(p: WebPerson, base: string, b: { supplier: string; number: string; description: string; amount: string; account: string }) {
  const { page } = p
  await openBooks(page, base, '/spend/bills/new', page.getByRole('textbox', { name: /^Bill number/ }))
  await choose(page.getByRole('combobox', { name: /^Supplier/ }), b.supplier)
  await page.getByRole('textbox', { name: /^Bill number/ }).fill(b.number)
  await page.getByRole('textbox', { name: /^What it was for/ }).fill(b.description)
  await page.getByRole('textbox', { name: /^Amount, KES/ }).fill(b.amount)
  await choose(page.getByRole('combobox', { name: /^Kind of cost/ }), b.account)
  await page.getByRole('button', { name: 'Save bill' }).click()
  await page.waitForURL(/\/spend\/bills\/(?!new)[^/]+$/, { timeout: 15000 })
}

/** Reconcile > an account > Import statement, from a CSV file (`statement.import`). */
export async function importStatement(p: WebPerson, base: string, bankId: string, csv: string) {
  const { page } = p
  await openBooks(page, base, `/reconcile/${bankId}`, page.getByRole('button', { name: 'Import statement' }).first())
  await page.getByRole('button', { name: 'Import statement' }).first().click()
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('Statement file (CSV)').setInputFiles(csv)
  const go = dialog.getByRole('button', { name: /^Import \d+ lines?$/ })
  await go.waitFor({ timeout: 15000 })
  const label = (await go.textContent()) ?? ''
  await go.click()
  await dialog.waitFor({ state: 'hidden', timeout: 15000 })
  return label
}

/** A claim's page; its main text (the status line and "What happens next"). */
export async function claimPage(p: WebPerson, base: string, claimId: string) {
  await openBooks(p.page, base, `/spend/claims/${claimId}`, p.page.getByRole('heading', { name: 'What happens next' }))
  return mainText(p.page)
}

/** A bill's page > Record a payment from a cash account for everything outstanding (`bill.pay`). */
export async function payBill(p: WebPerson, base: string, billId: string, pay: { account: string; reference: string }) {
  const { page } = p
  await openBooks(page, base, `/spend/bills/${billId}`, page.getByRole('button', { name: 'Record a payment' }))
  await page.getByRole('button', { name: 'Record a payment' }).click()
  const dialog = page.getByRole('dialog')
  await choose(dialog.getByRole('combobox', { name: /^Paid from/ }), pay.account)
  await dialog.getByRole('textbox', { name: /^Reference/ }).fill(pay.reference)
  await dialog.getByRole('button', { name: 'Record payment' }).click()
  await dialog.waitFor({ state: 'hidden', timeout: 15000 })
}

/** The text of the page's main region, for checks on status words. */
export const mainText = async (page: Page) => ((await page.locator('main').innerText()) ?? '').replace(/\s+/g, ' ')

/** Reads one report off the Reports screen: every statement table and footing check. */
export async function readReport(p: WebPerson, base: string, id: string, from: string, to: string): Promise<DomReport> {
  const { page } = p
  await page.goto(`${base}/reports?report=${id}&from=${from}&to=${to}&basis=accrual`)
  await page.locator('main table').first().waitFor({ timeout: 30000 })
  // One more frame so every table of the statement is in.
  await page.waitForTimeout(300)
  return page.locator('main').evaluate((main): DomReport => {
    const text = (el: Element | null) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim()
    const tables = [...main.querySelectorAll('table')].map((table) => {
      const columns = [...table.querySelectorAll('thead th')].map((th) => text(th))
      const rows: Array<DomRow> = []
      for (const tr of table.querySelectorAll('tbody tr, :scope > tr')) {
        if (tr.getAttribute('aria-hidden') === 'true') continue
        const group = tr.querySelector('th[scope="colgroup"]')
        if (group) {
          rows.push({ kind: 'heading', code: null, label: text(group), detail: null, cells: [] })
          continue
        }
        const th = tr.querySelector('th[scope="row"]')
        if (!th) continue
        const code = th.querySelector('span.font-mono')
        const detail = th.querySelector('span.block')
        const clone = th.cloneNode(true) as Element
        clone.querySelector('span.font-mono')?.remove()
        clone.querySelector('span.block')?.remove()
        const cls = tr.className
        const kind = cls.includes('border-double') ? 'total' : cls.includes('border-t') ? 'subtotal' : 'line'
        rows.push({ kind, code: code ? text(code) : null, label: text(clone), detail: detail ? text(detail) : null, cells: [...tr.querySelectorAll('td')].map((td) => text(td)) })
      }
      return { columns, rows }
    })
    const footings = [...main.querySelectorAll('p')]
      .filter((p) => p.firstElementChild?.textContent?.trim() === 'Difference')
      .map((p) => text(p.lastElementChild))
    return { tables, footings }
  })
}
