// Screenshots of FlowTrace Books on the /books page. Each file in public/books is the
// screenshot of the same name in rust/docs/screens, resized to 960 px wide as WebP (quality
// 82, ImageMagick); the eight together are about 190 KB.

export interface BooksScreen {
  /** The source shot is rust/docs/screens/<shot>.png. */
  shot: string
  src: string
  width: number
  height: number
  alt: string
  /** The shot shows the bundled demo books (Demo NGO Kenya, as of 4 Feb 2026). */
  demo: boolean
}

export interface BooksFeature {
  title: string
  body: string
  screen: BooksScreen | null
}

function screen(
  shot: string,
  height: number,
  alt: string,
  demo = true,
): BooksScreen {
  return { shot, src: `/books/${shot}.webp`, width: 960, height, alt, demo }
}

export const BOOKS_FEATURES: Array<BooksFeature> = [
  {
    title: 'Chart of accounts and periods',
    body: 'Start from a chart built for organisations that hold grants: cash, M-Pesa, receivables and restricted funds. Close each month when it is done, and lock it so nobody posts into it later.',
    screen: screen(
      'u2-accounts',
      543,
      'The Chart of accounts screen: accounts grouped under headers such as Assets and Cash and cash equivalents, each with its code, type, normal side and balance in KES.',
    ),
  },
  {
    title: 'Journal and general ledger',
    body: 'Every entry has lines that balance, a fund, a grant and an audit trail. The general ledger shows each posted line with the running balance of its account.',
    screen: screen(
      'u3-journal',
      543,
      'The Journal screen: 22 posted entries, none in draft or reversed, and a list of entries with number, date, memo, source, fund, status and total.',
    ),
  },
  {
    title: 'Spend: claims and bills',
    body: 'Staff submit what they paid from their own pocket, someone else approves it, and the books record it. Supplier bills are posted when they arrive and paid when they are due.',
    screen: screen(
      'u5-claims',
      543,
      'The Expense claims screen: two claims with date, claimant, fund and grant, amount in KES and a status of Submitted or Reimbursed.',
    ),
  },
  {
    title: 'Grants and budget against actual',
    body: 'See each grant and what is left of it. Every budget line is set against what has been spent, and lines are flagged at 80% used and again when they go over.',
    screen: screen(
      'u7-budgets',
      543,
      'The Budget vs actual screen: 43.9 million KES budgeted, 8.5 million spent, one line over budget, and a bar for each budget line with its actual spend beside it.',
    ),
  },
  {
    title: 'Bank and M-Pesa reconciliation',
    body: 'Import a bank or M-Pesa statement. Lines that match the books within three days match on their own; the rest wait for you to match them or post them as new entries.',
    screen: screen(
      'u8-reconcile-matched',
      543,
      'The Reconcile screen: the statement set against the books, two of three lines matched automatically, and one unmatched M-Pesa charge with an option to post it as a new entry.',
    ),
  },
  {
    title: 'Financial statements, ready to export',
    body: 'Trial balance, financial position and activities, each split by donor restriction. Export any statement as a CSV file for your auditor or your board.',
    screen: screen(
      'u9-financial-position',
      543,
      'The statement of financial position: assets, liabilities and net assets with and without donor restrictions, a line confirming assets equal liabilities plus net assets, and an Export CSV button.',
    ),
  },
  {
    title: 'Campaigns, chains and the Trust Score',
    body: 'The same transparency screens as the web app: campaigns, funding chains between organisations, and the Trust Score with how it is made and how to raise it.',
    // TODO-USER: no desktop shot of these screens exists yet (DT1 to DT4 shots are owed).
    screen: null,
  },
  {
    title: 'When the connection drops',
    body: 'Every screen keeps the last books it loaded and says that you are offline. A change is refused, not half-saved, until you reconnect.',
    screen: screen(
      'u10-offline',
      822,
      'The Chart of accounts screen with an orange notice across the top: You are offline and no books have loaded. Check the server on the Connection screen.',
      false,
    ),
  },
]

/** Above the fold on /books, loaded eagerly. */
export const HERO_SCREEN = screen(
  'u4-general-ledger',
  543,
  'The General ledger screen filtered to the main bank account: total debits and credits, nine posted lines with date, entry, memo, fund and running balance, and one entry open on the right.',
)

export const BOOKS_SCREENS: Array<BooksScreen> = [
  HERO_SCREEN,
  ...BOOKS_FEATURES.flatMap((f) => (f.screen ? [f.screen] : [])),
]
