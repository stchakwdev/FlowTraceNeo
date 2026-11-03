---
title: Transactions and proofs
description: How to find, record and open transactions, attach proof, and choose who can see each proof file.
order: 3
section: Overview
---

<!-- Verified against: flowtrace/src/routes/_authenticated/transactions/index.tsx, flowtrace/src/routes/_authenticated/transactions/new.tsx, flowtrace/src/routes/_authenticated/transactions/$id.tsx, flowtrace/src/lib/types.ts, flowtrace/src/lib/proof-labels.ts, flowtrace/src/lib/proofs.ts, flowtrace/src/lib/landing-figures.ts, flowtrace/src/components/file-upload.tsx, flowtrace/src/components/verification-gallery.tsx, flowtrace/src/components/verification-viewer.tsx, flowtrace/src/components/report/status-line.tsx -->

A transaction is one movement of money between two nodes, such as a donor and an account. The **Transactions** screen at `/transactions` lists every movement with the proof attached to it. Proof is what lets others trust the figures.

![The Transactions list: filters for Evidenced, Awaiting proof and Flagged above a table of movements, each with its proof count and status.](/docs/img/transactions.webp "1280x800")

## The transactions list

The top of the page counts the movements recorded, how many are awaiting proof and how many are flagged.

Use the filter buttons to narrow the list: **All**, **Evidenced**, **Awaiting proof** and **Flagged**. Each button shows how many movements it holds.

The search box looks through the description, reference, category and node names.

Each row shows the **Date**, **Description**, **From → To**, **Amount**, the number of **Proofs** and the **Status**. Select a row to open it. When every row shown uses the same currency, a total appears at the foot of the list.

If nothing matches, choose **Clear filters**.

## Record a transaction

Choose **Record a transaction** to open `/transactions/new`. A new transaction is marked as awaiting proof until proof is attached.

Fill in the form:

- **What moved**: a **Description** and an optional **Category**.
- **Between which nodes**: **From** and **To**. These must be two different nodes. Nodes are the boxes on the [flow map](/docs/flows).
- **Amount and date**: the **Amount**, the **Currency** and the **Date**. The date cannot be in the future. You can add a **Reference**.
- **Notes**: anything else worth keeping.
- **Proof**: optional. You can attach it later.

Choose **Save transaction**. If something is missing, a list at the top tells you what to fix.

The **Proof** part of this form offers six kinds: receipt, invoice, bank statement, photo, contract and document. You can add up to five files, each up to 10 MB. They upload when you save. For other kinds, save first and then attach proof from the detail page.

## A transaction's detail page

Each transaction has its own page. It shows the amount, a line from the **From** node to the **To** node, and the proof status. The pattern of that line shows the status: solid for **Evidenced**, dashed for **Awaiting proof**, long dashes with a cross for **Flagged**, and dotted for **Rejected**.

The side panel holds the **Record** and any **Notes**.

The **Proofs** section lists each proof with its kind, its trust points, its review state and who added it. The heading also gives the total trust points for this transaction.

## Attach proof

On the detail page, choose **Attach proof**. A panel opens.

1. Pick the **Kind of proof**. Each kind shows the trust points it adds.
2. Add one or more files. Files upload as soon as you choose them. To change the kind, remove the files first.
3. Fill in **What it shows** and **Captured by**, the name of the person in the field.
4. Choose **Who can see the file**.
5. If you are on site, choose **Add my current location** to add a GPS reading.
6. Choose **Attach proof**.

For a **GPS reading**, your location alone is enough. Every other kind needs at least one file.

### Kinds of proof and their trust points

Each proof adds trust points to the payment it backs. The attach panel offers 11 kinds:

| Kind of proof | Trust points |
| --- | --- |
| Third-party audit | +15 |
| Bank statement | +10 |
| Video | +10 |
| Photo | +8 |
| Government document | +8 |
| Signature | +7 |
| Contract | +6 |
| Receipt | +5 |
| Invoice | +4 |
| Document | +4 |
| GPS reading | +3 |

FlowTrace also knows a twelfth kind, **Biometric**, worth +7. You cannot attach it from these screens.

See [Trust Score](/docs/trust-score) for how points feed your score.

## Who can see a proof

When you attach proof, you choose one of four levels in **Who can see the file**:

| Level | What it means |
| --- | --- |
| **Full file** | Board and staff see the original file. |
| **Thumbnail only** | Donors see a low-resolution version. |
| **Summary only** | Partners see that proof is attached, not the file. |
| **Hidden** | Not shown outside the organisation. |

**Full file** is chosen unless you change it. In the proofs list, any proof below **Full file** shows its level next to it.

## Open and publish a proof

Select a proof to open the viewer. It shows the file and, beside it, the details: trust points, **Captured by**, **Location**, **Added**, and **Who can see it**. Use **Download original** to save the file.

Use the left and right arrow keys to move between proofs. Press Esc to close.

Each proof has its own review state: **Verified**, **Awaiting review** or **Rejected**.

For a proof with a file, admins and managers also see **Publish this proof**. Once published, anyone with the proof's public link can open the file. Choose **Stop showing publicly** to undo this.
