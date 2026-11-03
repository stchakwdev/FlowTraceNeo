---
title: Flows
description: How to read the flow map, pick a time period, isolate one donor or payment, and use the map with a keyboard.
order: 4
section: Overview
---

<!-- Verified against: flowtrace/src/routes/_authenticated/flows.tsx, flowtrace/src/components/flows/flow-map.tsx, flowtrace/src/components/flows/flow-graph.ts, flowtrace/src/components/flows/flow-details.tsx, flowtrace/src/components/report/status-line.tsx, flowtrace/src/components/report/report.tsx -->

The **Money flow** screen at `/flows` draws your money as a map. It shows where money came from, where it went, and the proof behind each movement. Open it from **Flows** in the rail, or from **Open the flow map** on the [dashboard](/docs/dashboard).

![The Money flow map: donors on the left, then accounts, departments and recipients, joined by lines whose weight shows the amount.](/docs/img/flows.webp "1280x800")

## Pick a period

Use the buttons at the top right to choose **Week**, **Month**, **Quarter** or **Year**. The map opens on **Month**.

Each period counts back from the latest recorded transaction, not from today. So a quiet month still shows your most recent work. The line under the heading gives the exact dates covered.

If nothing was recorded in the period, you can choose **Show the last year** to widen it.

Above the map, four key figures sum up the period: **Total moved**, **Nodes**, **Links** and **Share evidenced**. **Share evidenced** is the part of the money that has proof.

## Read the map

Each box is a node. A node is a place money passes through, such as a donor, an account or a supplier. Each line is money moving between two nodes. Money always flows from left to right.

The map has four columns:

1. **Donors**
2. **Accounts**
3. **Departments, projects**
4. **Recipients**, which holds vendors and beneficiaries

Each box shows the node's name, its type, and the larger of the money it received or sent. A box with a dashed border is unassigned: a movement was recorded without that node.

One line can stand for many movements between the same two nodes.

## Line pattern and line weight

The **line pattern** shows the proof state. The key above the map shows the first three patterns.

| Line | Label | Meaning |
| --- | --- | --- |
| Solid | **Evidenced** | The money has proof. |
| Dashed | **Awaiting proof** | Proof is still needed. |
| Long dashes with a cross | **Flagged** | The movement needs a closer look. |
| Dotted | **Rejected** | The movement was rejected. |

When a line carries several movements, it shows the weakest state among them. One flagged payment makes the whole line flagged.

The **line weight** shows the amount. A thicker line carries more money. Small amounts still get a thin line you can see beside large grants.

## Select to isolate

Select a box to isolate it. Everything upstream and downstream of it stays clear: where its money came from, and where that money went next. The rest of the map fades.

A panel opens with the node's details. It shows **Received** and **Sent**, then lists **Money in** and **Money out**. Select **Ledger lines for** and the node's name to see its entries in the [ledger](/docs/ledger).

Select a line instead to see the movements it carries. The panel shows the total **Moved in period**, its proof state, and a **Movements** list. Select a movement to open its page. See [Transactions and proofs](/docs/transactions).

To clear the selection, press Esc, choose the close button in the panel, or select an empty part of the map.

On a computer the panel sits beside the map. On a phone it slides up from the bottom of the screen.

## Use the map with a keyboard

You can do everything on the map without a mouse.

- **Tab** moves through the boxes and lines in money order, from left to right.
- **Enter** or the **Space bar** selects the box or line you are on.
- **Esc** clears the selection.

## Zoom and fit

Use the magnifying glass buttons above the map to zoom out and in. The current zoom level shows between them as a percentage. Choose **Fit** to see the whole map at once.

The map opens at a size where the text stays readable. On a narrow screen, scroll sideways to see the whole map.

## When the map is empty

If you have not recorded any movements, you see "No movements recorded yet". Choose **Record a transaction** to start.

If the map cannot load, check your connection and choose **Try again**.
