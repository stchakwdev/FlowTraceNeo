---
title: Glossary
description: Plain definitions of the words you meet in FlowTrace, from access group to visibility level, with links to the pages that use them.
order: 15
section: Reference
---

<!-- Verified against: DESIGN.md, flowtrace/src/lib/types.ts, flowtrace/src/lib/ledger/types.ts, flowtrace/src/lib/ledger/engine.ts, flowtrace/src/lib/ledger/reconcile.ts, flowtrace/src/lib/ledger/reports.ts, flowtrace/src/lib/data.ts, flowtrace/src/lib/offline-claims.ts, flowtrace/src/components/report/status-line.tsx, flowtrace/src/components/report/report.tsx, flowtrace/src/components/verification-gallery.tsx, flowtrace/src/components/verification-viewer.tsx, flowtrace/src/components/campaign-embed-widget.tsx, flowtrace/src/components/embed-code-generator.tsx, flowtrace/src/components/chain-request-modal.tsx, flowtrace/src/components/public-donation-form.tsx, flowtrace/src/components/ledger/reports/statements.tsx, flowtrace/src/routes/_authenticated/trust-score.tsx, flowtrace/src/routes/_authenticated/settings/access-groups.tsx, flowtrace/src/routes/_authenticated/settings/index.tsx, flowtrace/src/routes/_authenticated/ledger/index.tsx, flowtrace/src/routes/_authenticated/ledger/accounts.tsx, flowtrace/src/routes/_authenticated/ledger/general.tsx, flowtrace/src/routes/_authenticated/ledger/periods.tsx, flowtrace/src/routes/_authenticated/spend/claims.index.tsx, flowtrace/src/routes/_authenticated/spend/bills.index.tsx, flowtrace/src/routes/_authenticated/grants/index.tsx, flowtrace/src/routes/_authenticated/grants/budgets.tsx, flowtrace/src/routes/_authenticated/reconcile/index.tsx, flowtrace/src/routes/_authenticated/reconcile/$bankId.tsx, flowtrace/src/routes/_authenticated/reports/index.tsx, flowtrace/src/routes/badge/trust.$orgId.tsx -->

These are the words you meet in FlowTrace, in alphabetical order. Each one links to the guide where it matters most.

## A to C

**Access group.** A group of people who see the same amount of your flows. Each group has its own settings: whether it sees nodes, amounts and proofs, and whether it sees the original files. A default group can be changed but not deleted. You set up groups in [Settings](/docs/settings).

**Account.** One line in your chart of accounts, such as a bank account, a supplier you owe, a grant income line or an expense. Every account is one of five types: asset, liability, equity, income or expense. Header accounts group other accounts and cannot be posted to. See [Ledger](/docs/ledger).

**Accrual.** The way FlowTrace keeps your books. Spend counts when a bill is posted or a claim is approved, not when the money leaves. Reports and budgets can also show a cash view, which counts spend only once it is paid. See [Reports](/docs/reports).

**Auto-match.** The part of reconciliation that pairs statement lines with your books for you. A statement line matches a posted journal line on the same cash account when the amount is the same and the dates are no more than a few days apart (three by default). A matching reference wins. When there is more than one possible match, FlowTrace leaves it for you to choose. See [Reconcile](/docs/reconcile).

**Awaiting proof.** A movement of money that has no accepted proof yet. It is drawn as a grey dashed line. See [Transactions](/docs/transactions).

**Bill.** An invoice from a supplier. A bill is approved, posted to the books as money you owe, then paid. Someone other than the person who entered it must approve it. See [Spend](/docs/spend).

**Budget.** The amounts you plan to spend in a year, line by line. FlowTrace sets each line against what has been spent. A line is flagged when 80% is used, and again when it goes over. See [Grants](/docs/grants).

**Campaign.** A fundraising appeal with a target amount. It shows what has been raised, and can have a public page and an embed widget for your website. See [Campaigns](/docs/campaigns).

**Cash basis.** A view of your reports that leaves out unpaid bills and claims. Each payment counts against the expenses it settled. See accrual.

**Chain.** A link between your organisation and a partner. You ask another organisation for a view of their flows. They choose how much you see, and either side can end the link later. A chain can be pending, active, rejected or revoked. See [Chains](/docs/chains).

**Chart of accounts.** The full list of your accounts, grouped under header accounts. Every journal entry posts to accounts in this list. See [Ledger](/docs/ledger).

**Claim (spend claim).** Money a staff member spent from their own pocket and wants back. A claim is submitted, approved by someone else, posted to the books, then paid back. You cannot approve your own claim. In the web app, a claim made with no connection waits on your phone and is sent when you are back online. See [Spend](/docs/spend).

**Credit.** The right-hand side of a journal line. A credit makes income, liability and equity accounts go up, and makes asset and expense accounts go down. See debit and [Ledger](/docs/ledger).

## D to F

**Debit.** The left-hand side of a journal line. A debit makes asset and expense accounts go up, and makes income, liability and equity accounts go down. See [Ledger](/docs/ledger).

**Demo data.** The made-up books of Demo NGO Kenya, which come with FlowTrace so you can try it. Figures from the demo carry a "Demo data" tag. They are not real figures of any real organisation. See [Getting started](/docs/getting-started).

**Double-entry.** The rule that every journal entry has at least two lines, and its debits equal its credits. Each fund must balance inside the entry too. FlowTrace will not post an entry that breaks this rule. See [Ledger](/docs/ledger).

**Draft.** A journal entry that is saved but not yet in the books. Drafts are not checked for balance until you post them. See [Ledger](/docs/ledger).

**Embed widget.** A small card you can place on your own website. For a campaign, it shows what has been raised against the target, how much was spent with proof, and a link to give. You copy its code from the Embed page in [Settings](/docs/settings).

**Evidenced.** A movement of money whose proof has been checked and accepted. It is drawn as a solid line. See [Transactions](/docs/transactions).

**Flagged.** A movement of money that needs a closer look, for example cash taken out with no matching receipt. It is drawn as an orange long-dash line with a small cross. Resolve it with a matching receipt, or record why the money moved. See [Transactions](/docs/transactions).

**Flow.** The path money takes from where it came from to where it went. The flow map draws each movement as a line between nodes. The thicker the line, the larger the amount. The line's pattern shows its proof state. See [Flows](/docs/flows).

**FlowTrace Books.** The desktop app for Linux and macOS. It keeps the same books as the web app. See [FlowTrace Books](/docs/flowtrace-books).

**Fund.** A pot of money kept apart in the books. A fund is unrestricted, temporarily restricted or permanently restricted. See restricted funds.

## G to M

**General ledger.** Every posted line, account by account, with the balance after each one. A header account includes all of its sub-accounts. See [Ledger](/docs/ledger).

**Grant.** An award of money from a donor, with a start date, an end date and an amount. FlowTrace shows what has been spent, what is committed and what is left. It blocks spend outside the grant's dates or over its award, unless you give a reason. See [Grants](/docs/grants).

**Journal entry.** A record of one event in the books, numbered like JE-0001. It has a date, a memo and two or more lines. An entry is a draft, posted or reversed. A posted entry is never edited. To correct it, you reverse it and post a new one. See [Ledger](/docs/ledger).

**M-Pesa.** Safaricom's mobile money service. In Reconcile you can import an M-Pesa statement, as well as MTN MoMo and Airtel Money statements and bank CSV files. The public donation form also offers M-Pesa as a way to give. See [Reconcile](/docs/reconcile).

**Movement.** One transaction: an amount of money that moved from one node to another. Each movement carries its proof state. See [Transactions](/docs/transactions).

## N to R

**Node.** A place money comes from or goes to on the flow map. A node can be a donor, a department, a vendor, a beneficiary, an account or a project. See [Flows](/docs/flows).

**Period (open, closed, locked).** One month of your financial year. You can post into an open period only. Close a month once its entries are complete, so nothing is backdated into figures already reported. Months close in order, and a month with drafts cannot close. You can reopen a closed month with a reason. Lock it once its accounts are signed off. A locked month can never be reopened. See [Ledger](/docs/ledger).

**Proof (verification).** A file or record that shows a movement really happened. It can be a receipt, photo, document, bank statement, signature, GPS reading, invoice, video, contract, government document or third-party audit. See [Transactions](/docs/transactions).

**Reconciliation.** Checking each bank and mobile money account against its statement. A statement line is reconciled when it is matched to a posted journal line. Anything left over is money the books have not explained yet. See [Reconcile](/docs/reconcile).

**Rejected.** A movement of money whose proof was not accepted. It is drawn as a red dotted line, and it is left out of campaign reports. See [Transactions](/docs/transactions).

**Restricted funds.** Money a donor has given for a set purpose. Spend from a restricted fund that has grants must name the grant. At year end, restricted funds close to restricted net assets. See [Grants](/docs/grants).

**Reversal.** A journal entry that cancels a posted entry by swapping every debit and credit. You give a reason, and it is the only way to undo a posted entry. See [Ledger](/docs/ledger).

## S to Z

**Statement of activities.** A report of income and expenses over a span of dates. It splits each line into unrestricted and restricted money. See [Reports](/docs/reports).

**Statement of financial position.** A report of what your organisation owns and owes on one date: assets, liabilities and net assets. Assets always equal liabilities plus net assets. See [Reports](/docs/reports).

**Trial balance.** A list of every account with a balance on one date, in a debit column and a credit column. The two totals must agree. See [Reports](/docs/reports).

**Trust badge.** A small image of your Trust Score that you can show on your website. It has a public web address, and you copy its code from [Settings](/docs/settings).

**Trust Score.** A score from 0 to 100 that shows how well your records are backed by proof. It also comes with a grade, from A+ to F. It is made of parts: verification (movements with accepted proof), documentation (movements with any proof attached) and consistency (how well amounts, dates and parties agree). On live books there is a fourth part, independent review. Live books weight the parts 32, 24, 24 and 20. The demo uses three parts weighted 40, 30 and 30. See [Trust Score](/docs/trust-score).

**Vendor.** A supplier who sends you bills. See [Spend](/docs/spend).

**Visibility level.** How much of a proof a reader may see. There are four levels: full file, thumbnail only, summary only (that proof is attached, not the file), and hidden. Each proof has a level, and a reader's access group decides how much of it they see. See [Settings](/docs/settings).
