---
title: Ledger
description: How to read the journal, write and reverse entries by hand, manage the chart of accounts, close periods and import journals from QuickBooks or Xero.
order: 7
section: Books
---

<!-- Verified against: flowtrace/src/routes/_authenticated/ledger/index.tsx, flowtrace/src/routes/_authenticated/ledger/-shared.tsx, flowtrace/src/routes/_authenticated/ledger/new.tsx, flowtrace/src/routes/_authenticated/ledger/entries.$id.tsx, flowtrace/src/routes/_authenticated/ledger/accounts.tsx, flowtrace/src/routes/_authenticated/ledger/periods.tsx, flowtrace/src/routes/_authenticated/ledger/general.tsx, flowtrace/src/routes/_authenticated/ledger/import.tsx, flowtrace/src/components/ledger/ledger-nav.ts, flowtrace/src/lib/ledger/engine.ts, flowtrace/src/lib/ledger/importers.ts, flowtrace/src/lib/ledger/csv.ts -->

The ledger is the full record of your books. Open [Ledger](/ledger) and use the tabs: **Journal**, **General ledger**, **Chart of accounts**, **Periods** and **Import**.

![The Journal: tabs for the general ledger, chart of accounts, periods and import, counts of posted and draft entries, and the entry list.](/docs/img/ledger.webp "1280x800")

## The journal

A journal entry records one event in the books. It has two or more lines, each moving an amount into or out of an account. A debit is the left side and a credit the right. Debits must equal credits.

The **Journal** lists every entry, newest first. Each entry has one of three states:

- **Draft**: saved but not in the books. Drafts do not count in any balance or report.
- **Posted**: in the books. A posted entry is never edited.
- **Reversed**: cancelled by a later, mirror-image entry.

Filter by state, **Source** or **Period**, or use **Search** to find a number, memo or override reason.

## Write an entry by hand

1. Press **New journal entry**.
2. Fill in **Date** and **Memo**.
3. For each line, choose an **Account** and a **Fund**. Add a **Grant**, **Tag** or **Line memo** if needed.
4. Enter a **Debit** or a **Credit** on each line, not both.
5. Press **Add a line** for more lines.

The bottom shows **Debits**, **Credits** and **Difference**. When they match it reads **Balanced**. Debits must also equal credits within each fund.

Press **Save draft** to finish later, or **Post** to put it in the books. If only grant rules stop the entry, an **Override** box appears. Write an **Override reason** and press **Post with override**. The reason is kept on the entry and in the audit trail.

## Change, delete or reverse an entry

Open any entry to see its lines and its **Audit trail**. A draft has three buttons: **Edit**, **Delete** and **Post**. A posted entry has one: **Reverse**. Choose a **Reversal date** in an open period and give a **Reason**. Then press **Post reversal**. A mirror-image entry is posted on that date. The original stays in the books, marked reversed. To fix a mistake, reverse the entry and post a correct one.

## Chart of accounts

The chart of accounts is the list of every account the books use, shown as a tree with balances. Header accounts group other accounts and cannot take entries directly.

To add one, press **Add an account**. Fill in **Code**, **Name** and **Type**. Types are Asset, Liability, Net assets, Income and Expense. You can also set:

- **Parent**: the header account it sits under.
- **Header account**: tick this if it groups other accounts.
- **Cash account**: for a bank, mobile money or petty cash account.
- **Expense function**: Programme, Management and general, or Fundraising.

Press **Deactivate** to stop an account taking new entries. Press **Reactivate** to undo it. System accounts cannot be turned off.

## Periods

Each month is a period. A period can be open, closed or locked:

| State | Meaning |
| --- | --- |
| Open | Takes entries. |
| Closed | Takes no entries. Can be reopened with a reason. |
| Locked | Sealed for good. Cannot be reopened. |

Press **Close** once a month's entries are complete. Months close in order, oldest first. A month with drafts will not close until you post or delete them.

Press **Reopen** on a closed month and give a **Reason**. Months reopen latest first. Press **Lock** on a closed month once its accounts are signed off.

**Close the year** posts one entry on the year's last day. It brings every income and expense account to zero and moves the result into net assets, fund by fund. Then it closes every open month of that year. The last month must still be open, and a year closes only once.

The **Open fiscal year** button names the next year and adds its twelve months.

## General ledger

The **General ledger** lists every posted line, account by account. It shows the opening balance, each line, the running balance and the closing balance. Filter by **Account**, **Fund**, **Grant**, **Tag**, **From** and **To**. Press **Export CSV** to download it.

## Import journals from QuickBooks or Xero

**Import** reads a CSV file of up to 5 MB, uploaded or pasted into **Or paste the CSV**. It knows the file type from the header row.

| File | Columns it reads |
| --- | --- |
| QuickBooks journal export | Account, plus Debit or Credit (needed). Also Date, Num, and Memo/Description. |
| Xero manual journal | AccountCode, Amount and Narration (needed), plus Date. Debits are positive and credits negative. |

```
Date,Num,Account,Memo/Description,Debit,Credit
```

Accounts are found by code first, then by name. Accounts not in your chart are listed, and their lines are left out. Every imported line goes to the **Fund for every line** you pick.

Check the **Preview**. Then press the create button, which names the count, such as **Create 3 drafts**. Each journal becomes a draft. Nothing reaches the books until someone reviews and posts it.
