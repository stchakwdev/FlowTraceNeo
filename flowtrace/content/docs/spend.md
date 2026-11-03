---
title: Spend
description: How to make, approve and pay back expense claims, how claims wait on a phone with no signal, and how to enter, post and pay supplier bills.
order: 5
section: Books
---

<!-- Verified against: flowtrace/src/routes/_authenticated/spend/index.tsx, flowtrace/src/routes/_authenticated/spend/-shared.tsx, flowtrace/src/routes/_authenticated/spend/claims.index.tsx, flowtrace/src/routes/_authenticated/spend/claims.new.tsx, flowtrace/src/routes/_authenticated/spend/claims.$id.tsx, flowtrace/src/routes/_authenticated/spend/bills.index.tsx, flowtrace/src/routes/_authenticated/spend/bills.new.tsx, flowtrace/src/routes/_authenticated/spend/bills.$id.tsx, flowtrace/src/components/ledger/spend-nav.ts, flowtrace/src/lib/offline-claims.ts, flowtrace/src/lib/ledger/engine.ts -->

Spend is where money goes out. An expense claim is money a staff member paid from their own pocket and wants back. A bill is an invoice from a supplier. Open [Spend](/spend) to see what waits for a decision and what is owed.

![The Spend overview: what is owed and waiting, and claims and bills waiting for approval, each with its Approve or Post button.](/docs/img/spend.webp "1280x800")

## The Spend overview

The overview has three tabs at the top: **Overview**, **Claims** and **Bills**. It shows four figures:

- **To pay**: posted claims and bills that are not yet paid.
- **Waiting for a decision**: claims and bills to approve or post.
- **Overdue**: bills past their due date.
- **Due in the next 14 days**.

**Waiting for approval** lists submitted claims and unposted bills, with an **Approve** or **Post** button. **To pay** lists what is owed, with a **Reimburse** or **Pay** button.

## Make an expense claim

1. Press **New expense claim**.
2. Fill in **Date spent** and **Purpose**. The date cannot be after today.
3. Under **What you spent**, add one line per cost. Each line needs **What it was for**, an amount, a **Kind of cost** and a **Fund**. **Grant**, **Project** and **Place** are extra details.
4. Press **Add another line** for more costs.
5. Under **Receipts**, press **Take a photo of a receipt** or **Choose saved photos or files**. You can add up to 8.
6. Press **Save draft** to finish later, or **Submit for approval**.

Money from a restricted fund must name its grant. Receipts are recorded by file name only, for now. Keep the paper receipts until the claim is paid.

## Claims on a phone with no signal

A claim is kept on your phone when the phone is offline, or when sending fails on the network. It is saved in the browser's storage on that phone. You see **Saved on this phone; it will be sent when you are back online.**

Waiting claims are sent:

- when the page opens,
- when the phone comes back online,
- when you return to the page,
- and every 20 seconds while any claim is still waiting.

They go oldest first, once you are signed in.

On the **Claims** tab, **Waiting on this phone** lists each one. Press **Send now** to try at once. If a send fails, the claim stays and shows **Last try:** with the reason. Press **Discard** to delete a claim from the phone.

A claim that already reached the books is not made twice. Sent claims show **Captured offline**. Waiting claims live only in that browser on that phone. Clearing the browser's data before they send deletes them.

## Approve, reject and pay back a claim

Only the person who made a claim can submit it. Nobody can approve their own claim.

A manager with approval rights opens a submitted claim and sees two buttons:

- **Approve and post** puts the claim in the books straight away. The costs land on their grants and budget lines. The money is now owed to the claimant. You can add a **Note to the claimant**.
- **Reject** needs a **Reason**. The claimant can see it in the **Approval trail**.

Once posted, press **Record the reimbursement**. Fill in **Date paid**, **Paid from** (bank, M-Pesa or petty cash) and an optional **Reference**. This pays back the full amount still owed. You cannot pay out your own claim.

## Enter and pay a bill

You need permission to post to the books to enter bills.

1. Press **New bill**.
2. Under **Supplier**, choose **Existing supplier** or **Add a new supplier**. A new supplier needs a **Supplier name**. **Payment details** is optional.
3. Fill in **Bill number**, **Bill date** and **Due date**. The due date starts 30 days ahead.
4. Pick a **Currency**. For a foreign currency, enter the exchange rate on the bill date.
5. Add lines under **Coding**, and attach files under **Proof** (up to 5, 10 MB each).
6. Press **Save bill**. It is saved as a draft.

A bill then moves through these steps:

| Status | What happens next |
| --- | --- |
| Draft | Someone other than the person who entered it presses **Approve bill**. |
| Approved | Press **Post to the books**. The bill posts to the month of its bill date. |
| Posted, to pay | Press **Record a payment**. The amount starts at the full balance, but you can pay part of it. |
| Paid | Nothing more to do. |

An approved bill counts as committed on its grant. See [Grants and budgets](/docs/grants).

## When the books flag a problem

Some rules can be set aside with a reason. This happens when spend falls outside a grant's dates or takes a grant over its award. The button then reads **Override and approve and post** or **Override and post bill**. You must fill in **Reason for overriding**. The reason is kept on the journal entry for auditors.

Each claim and bill links to its journal entries under **In the books**. See [Ledger](/docs/ledger).
