---
title: Trust Score
description: How the Trust Score is worked out, what each part measures and weighs, how grades are set, and what you can do to raise it.
order: 12
section: Transparency
---

<!-- Verified against: flowtrace/src/routes/_authenticated/trust-score.tsx, flowtrace/src/lib/types.ts, flowtrace/src/lib/trust-score.test.ts, flowtrace/src/lib/queries.ts, flowtrace/src/lib/data.ts, flowtrace/src/lib/demo/api.ts, flowtrace/src/lib/ledger/transparency-model.ts, flowtrace/src/lib/ledger/public-sql.ts, flowtrace/src/lib/badge-utils.ts, flowtrace/src/lib/landing-copy.ts, rust/crates/ledger-module/src/transparency/trust.rs, rust/crates/ledger-module/src/transparency/flows.rs, rust/crates/ledger-module/src/transparency/public.rs -->

The Trust Score is a number from 0 to 100. It shows how well your organisation's money is backed by proof. It is worked out by fixed rules from your own records. The weights are the same for every organisation. You can see your score, how it is made and how to raise it at `/trust-score`.

![The Trust Score screen for the demo: a score of 87 out of 100, grade A, and the parts it is made of with their weights.](/docs/img/trust-score.webp "1280x800")

## The four parts

The score has four parts. Each part is a number from 0 to 100. Each one counts for a fixed share of the total.

| Part | Weight | What it measures |
| --- | --- | --- |
| Verification | 32% | The share of your movements whose proof has been checked and accepted. |
| Documentation | 24% | The share of your movements with at least one proof attached, checked or not. |
| Consistency | 24% | Meant to show how well amounts, dates and parties agree across the records. See the note below. |
| Independent review | 20% | A review of your records against a fixed rubric, less 5 points for each confirmed flag. |

FlowTrace multiplies each part by its weight and adds them up. It then rounds the total to a whole number.

**About Consistency.** FlowTrace does not yet check your records to work this part out. It takes the consistency value from your last saved calculation. If there is none, it uses 80.

**About Independent review.** A separate review service scores your records against a set of fixed questions. The part is the average of those scores. It can also raise flags on a transaction, an expense claim or a bill. A member who can approve spend then dismisses each flag, with a reason, or confirms it. Each confirmed flag takes 5 points off this part, down to 0. Open flags change nothing.

## The demo uses three parts

The demo organisation is fictional. Its score uses the older three-part rule:

| Part | Weight |
| --- | --- |
| Verification | 40% |
| Documentation | 30% |
| Consistency | 30% |

In the demo, the **How it is made** list shows three rows, not four.

## Grades

Every score gets a grade. The same bands are used in the app, on public pages and on the badge.

| Score | Grade |
| --- | --- |
| 90 to 100 | A+ |
| 80 to 89 | A |
| 70 to 79 | B |
| 60 to 69 | C |
| 50 to 59 | D |
| 0 to 49 | F |

## "Not yet scored" and "Not yet calculated"

You may see two different notices.

**Not yet scored** appears on the **Independent review** row only. It shows until the review service has stored its first scores for your organisation. Until then, this part takes the same value as the other three parts combined (40% verification, 30% documentation, 30% consistency). So it does not move your total. With no review and no confirmed flags, your score equals the three-part score.

**Not yet calculated** appears in place of the whole **How it is made** list. It shows when there is no score for your organisation yet. The score is worked out when the first movement is recorded.

## Read the screen

The **Score** section shows your score out of 100 and your grade. Under it you see how much the score has moved since the last calculation and since the first one. This needs at least two calculations.

**How it is made** lists each part with its weight, its value and the points it adds.

The score updates by itself each time a movement or a proof is recorded or changed. **Recalculate score** adds today's score to the **History** chart. The chart shows up to the last 12 calculations.

**Ledger signals** sits at the bottom when your books are connected. It shows **Cash movement reconciled**, **Spend within budget** and **Spend coded to a grant**. These are read from the books, next to the score. They do not change the score.

## How to raise it

The **How to raise it** section lists what is holding your score back. It can show four kinds of item:

- **Movements awaiting proof.** Attach a receipt, photo or statement to each. The screen says how many points this could add. Use **Show movements awaiting proof** to find them.
- **Flagged movements.** Resolve the flag with a matching receipt, or record why the money moved. Use **Review flagged movements** to find them. If only one is flagged, the link reads **Review the flagged movement**.
- **Movements with no proof attached.** This shows your documentation coverage as a percentage.
- **Proofs waiting for review.** A reviewer can accept them from each transaction's page.

When nothing is outstanding, you see **Nothing outstanding**. Keep attaching proof as you record new transactions to hold the score.

For more on proof, see [Transactions](/docs/transactions).

## Where the score appears

Your score is shown in several places outside this screen:

- On your donate pages and the campaign widget. See [Campaigns and public pages](/docs/campaigns).
- On the Trust Score badge.
- On chain links, where FlowTrace shows the average of both organisations' scores. See [Chains](/docs/chains).

The badge shows your score only while **Show on public pages** is on in [Settings](/docs/settings). Otherwise it shows 0.
