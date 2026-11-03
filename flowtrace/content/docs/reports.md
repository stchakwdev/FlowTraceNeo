---
title: Reports
description: How to choose a financial statement, set its dates and basis, check that it adds up, and export it as CSV or print it.
order: 9
section: Books
---

<!-- Verified against: flowtrace/src/routes/_authenticated/reports/index.tsx, flowtrace/src/components/ledger/reports/statements.tsx, flowtrace/src/components/ledger/reports/statement.tsx, flowtrace/src/components/ledger/export.ts, flowtrace/src/lib/ledger/reports.ts -->

[Financial reports](/reports) holds the statements a board, auditor or donor expects. Each one is read from posted journal lines. Drafts are never included. Each statement also shows a check that proves it adds up.

![Financial reports open on the statement of financial position, with tabs for the other statements and buttons to export CSV or print.](/docs/img/reports.webp "1280x800")

## Choose a statement

Tabs at the top pick the statement. There are five:

| Tab | Statement | What it shows |
| --- | --- | --- |
| Trial balance | Trial balance | Every account's debit or credit balance on one date |
| Financial position | Statement of financial position | What you own, what you owe and your net assets on one date |
| Activities | Statement of activities | Income, expenses and the change in net assets over a period |
| Functional expenses | Statement of functional expenses | Spend split into programmes, management and fundraising |
| Cash flow | Statement of cash flows | Cash in and out over a period |

The page opens on **Financial position**.

## Set the dates and basis

**Period** offers quick choices:

- **This month**, named after the current month
- **Year to date**
- The last fiscal year, if your books have one

You can also type your own dates. Trial balance and Financial position use one date, **As at**. The others use **From** and **To**. If the start is after the end, the page asks you to fix it.

**Basis** has two choices:

- **Accrual** counts a cost when the bill is posted or the claim approved.
- **Cash** leaves out unpaid bills and claims. It counts each payment against the expenses it settled.

The cash flow statement has no basis choice. It uses the direct method, which lists actual cash received and paid.

## What each statement holds

**Trial balance** lists each account with a **Debit** or **Credit** amount, and **Totals**.

**Statement of financial position** has three parts. Assets are what you own. Liabilities are what you owe. Net assets are what is left. Net assets are split into **Without donor restrictions** and **With donor restrictions**. They include this year's surplus or deficit to date.

**Statement of activities** lists income and expenses in three columns: **Without restrictions**, **With restrictions** and **Total**. It ends with **Change in net assets**. Below is a **Schedule by fund**, with income, expenses and change for each fund.

**Statement of functional expenses** shows each expense account across **Programmes**, **Management**, **Fundraising** and **Total**. The **Programme ratio** is the share of spend on programmes. Each expense account carries one function. Accounts without one count as programmes.

**Statement of cash flows** groups cash into operating, investing and financing activities. It shows opening cash, the net change and closing cash. Transfers between your own bank and mobile-money accounts are left out.

## The check under each statement

Each statement ends with a check. It reads **Foots.** when the figures agree, and **Does not foot.** when they do not.

| Statement | The check |
| --- | --- |
| Trial balance | Total debits equal total credits. |
| Financial position | Assets equal liabilities plus net assets. |
| Activities | Change in net assets equals the movement in net assets on the statement of financial position. |
| Functional expenses | Total expenses equal total expenses on the statement of activities. |
| Cash flow | Opening cash plus the net change equals closing cash. |

If a statement does not foot, look for the cause in the [Ledger](/docs/ledger) before you share it.

## Export or print

Press **Export CSV** to download the statement you are viewing. The file opens in any spreadsheet. It starts with the statement title, the date or period, the currency and the basis. Amounts are in your base currency, with two decimal places. The trial balance, financial position and cash flow files end with a **Difference** row from the check.

Press **Print or save as PDF** to print the statement. Your browser can also save it as a PDF. Only the statement is printed, not the app's menus.

Other screens export CSV too. These are the [General ledger](/ledger/general), [Budget vs actual](/grants/budgets) and each grant's detail page. See [Grants and budgets](/docs/grants).
