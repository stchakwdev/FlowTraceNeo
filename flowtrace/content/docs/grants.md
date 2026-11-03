---
title: Grants and budgets
description: How to read each grant's spend, commitments and money left, check whether it will last, and set and compare budget lines.
order: 6
section: Books
---

<!-- Verified against: flowtrace/src/routes/_authenticated/grants/index.tsx, flowtrace/src/routes/_authenticated/grants/$id.tsx, flowtrace/src/routes/_authenticated/grants/budgets.tsx, flowtrace/src/lib/ledger/reports.ts, flowtrace/src/lib/ledger/engine.ts, flowtrace/src/lib/ledger/commands.ts -->

A grant is money a donor gives for a set purpose, between a start date and an end date. [Grants](/grants) shows each grant with what has been spent, what is promised and what is left. Spend reaches a grant when a claim is approved or a bill is posted.

![The Grants screen: totals awarded, spent, committed and remaining across all grants, and each award with a bar of how much is used.](/docs/img/grants.webp "1280x800")

## The grants list

The page has two tabs: **Grants** and **Budget vs actual**.

At the top, **Across all grants** adds up every grant:

- **Awarded**: the total the donors agreed to give.
- **Spent**: costs posted to the books against the grants.
- **Committed**: approved bills and submitted claims. These are promised but not yet in the books.
- **Remaining**: the award less what is spent and committed.

Under **Awards**, each grant shows its donor, fund and dates. A bar shows spent, committed and remaining. The grant with the largest share used comes first.

Each grant also has a status:

| Status | Meaning |
| --- | --- |
| Within award | Less than 80% spent or committed |
| 80% used | 80% to 100% spent or committed |
| Over award | More than the award spent or committed |

You cannot add a new grant from this screen. TODO-USER

## One grant in detail

Select a grant to open it. **Award** shows the same four figures, plus:

- **Received from the donor**: income posted to the grant.
- **Spent per month, on average**: spend since the start date, divided by the months gone.
- **Money lasts, at that rate**: how many months of award are left at that pace.
- **Until the grant ends**: months to the end date.

Under the bar, a line says if the money will last. It is flagged when the award will run out more than a month before the end date. It is also flagged when a lot will be left unspent at the end. If nothing is spent yet, it says there is no rate to project from.

**Terms** shows the grant's conditions, if any are recorded.

**Budget vs actual** lists this grant's budget lines. Press **Whole budget** to see every line.

**Charged to this grant** lists every posted journal line on the grant. A journal line is one row of an entry in the books. It shows date, entry, account, memo, debit and credit. Press **CSV** to download the list.

## How the books guard a grant

The books check every cost charged to a grant:

- Spend from a restricted fund must name its grant.
- The grant must belong to the fund on the line.
- Spend dated before the grant starts or after it ends is flagged.
- Spend that takes a grant over its award is flagged.

The last two can go through with a written reason. The reason is kept on the journal entry. See [Spend](/docs/spend) and [Ledger](/docs/ledger).

## Budget vs actual

Open [Budget vs actual](/grants/budgets) to compare each budget line for the year with what was spent on it. Lines are grouped as **Grant budgets**, **Unrestricted running costs** and **Department allocations**.

The top figures show **Budgeted**, **Spent**, **Lines 80% used** and **Lines over budget**. Each line shows **Budget**, **Actual**, and **Left** or **Over by**.

You can narrow the list:

- **Budget**: shown only when there is more than one year's budget.
- **Grant**: one grant, or **All lines**.
- **Department, project or place**: one tag, or **All lines**.
- **Basis**: **Accrual** or **Cash**.

Accrual counts spend when a bill is posted or a claim is approved. Cash counts spend only when the money has left the bank or M-Pesa.

Press **CSV** to download the lines you see. The file holds group, grant, line, budget, actual, variance, percent used and status.

## Set or change a budget line

You need permission to post to the books to change a budget.

To change an amount, press **Change** on a line. Enter the **New budget** and press **Save**.

To add a line, use **Add a budget line** at the bottom:

1. Pick an **Expense account**. Leave it as **All expense accounts** for a department allocation.
2. Pick a **Fund**, **Grant** and **Department, project or place** if they apply.
3. Enter the **Amount for the year**.
4. Press **Add line**.

If no budget exists for the year, the page says **No budget set up**. A budget must be created before lines can be added. TODO-USER
