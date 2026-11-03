---
title: Reconcile
description: How to import bank and mobile-money statements, let auto-match pair lines with the books, and match or post the lines that are left.
order: 8
section: Books
---

<!-- Verified against: flowtrace/src/routes/_authenticated/reconcile/index.tsx, flowtrace/src/routes/_authenticated/reconcile/$bankId.tsx, flowtrace/src/components/ledger/reports/match-mark.tsx, flowtrace/src/lib/ledger/reconcile.ts, flowtrace/src/lib/ledger/csv.ts, flowtrace/src/lib/ledger/engine.ts -->

To reconcile means to check your books against your bank or mobile-money statement. Each statement line should match a line in the books. Anything left over is money the books have not explained yet. Open [Reconcile](/reconcile) to start.

![The Reconcile screen: the share of cash movement reconciled, and each bank and mobile-money account with its matched and unmatched lines.](/docs/img/reconcile.webp "1280x800")

## The accounts list

**Across all accounts** shows **Cash movement reconciled**, **Accounts**, **Statement lines** and **Unmatched lines**.

Under **Accounts**, each bank or mobile-money account shows its book balance and a bar of matched lines. It also shows **Matched**, **Unmatched** and **Last statement line**. Select an account to work on it.

There is no button here to add a bank or mobile-money account. TODO-USER

## Import a statement

On an account, press **Import statement**. Choose a **Statement file (CSV)**, or paste it into **Or paste the statement**. The first row must hold the column names.

The importer knows these formats from their column names:

| Format | How it is spotted |
| --- | --- |
| Safaricom M-Pesa statement | A Receipt No. column, or Completion Time with Paid In |
| MTN MoMo statement | External Transaction Id or Financial Transaction Id |
| Airtel Money statement | Transaction Id with Transaction Type or Service Type |
| Bank CSV | Anything else |

Each row needs a date and an amount:

- **Date**: read from Completion Time, Date, Transaction Date, Value Date, Posting Date and similar. Dates are read day first, such as 15/01/2026 or 15 Jan 2026.
- **Amount**: either a money-in and money-out pair (Paid In and Withdrawn, Credit and Debit, Money In and Money Out, Deposits and Withdrawals), or one signed Amount column.
- **Description** and **reference** are read if present, such as Details or Narration, and Receipt No. or Reference.

An M-Pesa example:

```
Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn
```

Rows whose status is not completed or successful are skipped. So are rows with no readable date or no amount. The preview lists each skipped row and why.

Press the import button, which names the count, such as **Import 12 lines**. Lines already on this statement are left out, so you can import the same file twice safely.

## Auto-match

Press **Auto-match**. It looks at each unmatched statement line. It finds posted journal lines that:

- are on the same cash account,
- have exactly the same amount,
- are dated within 3 days of the statement line,
- and are not matched yet.

Drafts are never matched. A line is matched when there is only one fit. It is also matched when only one fit holds the statement's reference in its memo or entry number. References shorter than 4 characters are not used. Reference matches are done first.

When it is done, you see how many lines were matched, how many need a decision and how many are unmatched.

## Match or post the rest by hand

Each statement line has a state:

| State | Meaning |
| --- | --- |
| Matched automatically | Auto-match paired it. |
| Matched by hand | A person paired it. |
| Needs a decision | More than one journal line fits. |
| Unmatched | No journal line fits yet. |

Use the filter at the top to see one state.

Press **Match** on a line. **Match to the books** lists posted lines on this account for the same amount within 7 days, closest first. Pick the right one.

If nothing fits, press **Post as new entry**. Choose what the money was for under **Charge it to** or **Record it as**. Leave it empty to hold the money in a suspense account until it is explained. Pick a **Fund**, and a **Grant** if needed. Then press **Post and match**. The entry is posted on the statement date and matched to the line.

To undo a match, press **Unmatch**. The line goes back to the unmatched lines.

## Check the position

**Position** shows the **Book balance**, the share of **Statement lines matched**, and how many lines **Need a decision** or are **Unmatched**.

**Statement against the books** covers the statement's dates. It shows what is only on the statement and what is only in the books. When the two agree, it says **The statement and the books agree over these dates.** If not, match or post the open lines to close the gap.

The share of cash movement matched to a statement line also appears as **Cash movement reconciled** on the [Trust Score](/docs/trust-score) page. It is shown beside the score and does not change it.
