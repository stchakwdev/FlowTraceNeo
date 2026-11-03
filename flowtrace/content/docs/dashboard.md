---
title: Dashboard
description: What the dashboard's key figures, donor funding bars and recent movements tell you, and how to read their proof marks.
order: 2
section: Overview
---

<!-- Verified against: flowtrace/src/routes/_authenticated/dashboard.tsx, flowtrace/src/lib/dashboard-summary.ts, flowtrace/src/components/report/report.tsx, flowtrace/src/components/report/status-line.tsx, flowtrace/src/lib/types.ts, flowtrace/src/lib/ledger-client.ts -->

The dashboard at `/dashboard` is a short report on your organisation's money. It shows how much came in, how much has proof, and what moved most recently. It is the first screen you see after you sign in.

![The Dashboard for Demo NGO Kenya: four key figures, funding received by donor, and recent movements with their proof state.](/docs/img/dashboard.webp "1280x800")

## The top of the page

The heading reads **Dashboard**. Under it you see your organisation's name and an "as of" date. That date is the date of the latest movement recorded, not today's date. In the demo you also see a **Demo data** tag.

Two buttons sit on the right:

- **Record a transaction** opens the form at `/transactions/new`. See [Transactions and proofs](/docs/transactions).
- **Open the flow map** opens `/flows`. See [Flows](/docs/flows).

## Key figures

The **Key figures** row has four numbers.

| Figure | What it counts |
| --- | --- |
| **KES received with proof** | Money from donors whose movements are evidenced. This is the largest figure. |
| **movements evidenced** | Evidenced movements out of all movements, written as two numbers with a slash between them. |
| **awaiting proof** | Movements that still need proof. If any are flagged, a line below says how many. |
| **Trust Score** | Your organisation's score with its grade, from A+ down to F. |

The Trust Score number is a link. Select it to open `/trust-score`. The [Trust Score](/docs/trust-score) page explains how the score is worked out.

The first figure is always labelled in Kenyan shillings (KES).

## Funding received, by donor

This list has one row per donor. Each row shows the donor's name, a bar and an amount in KES. The amount is everything that donor has sent, with or without proof. Rejected movements are left out.

The bars are drawn to the same scale, so you can compare donors at a glance.

- The darker part of a bar is the money with proof.
- The lighter part is money received that does not have proof yet.
- A dashed line, with no bar, means everything from that donor is still awaiting proof.

Donors with evidenced money come first, largest first. Donors with nothing evidenced come last.

Under the list, a key shows the three line patterns. A line of text then gives the total evidenced against the total committed.

### Focus on one donor

Point at a donor's row, or move to it with the Tab key. The other donors fade. In **Recent movements**, any movement that does not involve that donor fades too. Move away and everything returns.

## Recent movements

This table lists the eight most recent movements. Each row shows:

- **Movement**: where the money came from and where it went, with the description under it.
- **Date**: the day the money moved.
- **Amount**: the currency and amount.
- **Proof**: the proof mark for that movement.

Select a movement to open its detail page. Select **All transactions** to see the full list at `/transactions`.

On a phone, the table becomes a stacked list with the same details.

## Read the proof marks

FlowTrace shows proof with the pattern of a short line, not with colour alone.

| Line | Label | Meaning |
| --- | --- | --- |
| Solid | **Evidenced** | The movement has proof. |
| Dashed | **Awaiting proof** | The movement still needs proof. |
| Long dashes with a cross | **Flagged** | The movement needs a closer look. |

## When there is nothing to show

If no movements are recorded yet, you see "No money movements recorded yet". Use **Record a transaction** to add the first one.

If the page cannot load, you see "We couldn't load your transactions". Check your connection and select **Try again**. Nothing you recorded is lost.
