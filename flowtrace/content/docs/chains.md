---
title: Chains
description: How to ask another organisation for a view of its flows, how to approve or refuse such a request, and how to end a link.
order: 11
section: Transparency
---

<!-- Verified against: flowtrace/src/routes/_authenticated/chains.tsx, flowtrace/src/components/chain-request-modal.tsx, flowtrace/src/components/chain-approval-modal.tsx, flowtrace/src/components/chain-status-badge.tsx, flowtrace/src/lib/navigation.ts, flowtrace/src/lib/queries.ts, flowtrace/src/lib/data.ts, flowtrace/src/lib/demo/api.ts, flowtrace/src/lib/ledger/public-sql.ts, flowtrace/src/lib/ledger/spacetime-transparency.ts, rust/crates/ledger-module/src/transparency/chains.rs -->

Money often passes through more than one organisation. A funder gives a grant to a partner, and the partner spends it. A chain link lets the funder follow that money into the partner's flows. The partner decides how much the funder sees. You manage links on the **Chains** screen at `/chains`. The screen's title is **Chain links**.

![The Chain links screen: counts of active and pending links, and incoming requests to see your flows with Approve and Reject buttons.](/docs/img/chains.webp "1280x800")

## How a link works

A link always has two sides:

- The organisation that **asks**. This is usually the funder. It wants to see how the money it passed on is spent.
- The organisation that **shares**. This is usually the partner. It approves the request and picks the depth.

When the partner approves, it puts the funder in one of its [access groups](/docs/settings). The funder then sees the partner's flows at that group's depth, and no more. The partner can end the link at any time.

A link moves through four states:

| State | What it means |
| --- | --- |
| Pending | Asked, but not yet answered. |
| Active | Approved. The asking side can see the flows at the granted depth. |
| Rejected | The sharing side said no. |
| Revoked | The sharing side ended an active link. |

## Read the screen

The top of the screen shows four counts: **Active links**, **Awaiting your review**, **Your requests pending** and **Ended**. Ended counts both rejected and revoked links.

Below that are two lists:

- **Incoming: who can see your flows**. Requests from others, and links you have approved.
- **Outgoing: flows you follow**. Requests you have sent, and links others have approved for you.

Pending links come first in each list. Each row shows both organisations, the state, and a date. It also shows two fields:

- **Access granted**. The name of the access group, and a short line such as "Sees the flow and amounts." While a link is pending, this reads **You choose on approval** or **They choose on approval**.
- **Chain Trust Score**. The average of the two organisations' Trust Scores, rounded. The row shows both scores it came from. See [Trust Score](/docs/trust-score).

## Ask for a link

1. Choose **Request a link**.
2. In **Organisation**, type at least two letters of the name, then pick it from the list. You can only find organisations that publish their profile. An organisation you already have a pending or active link with shows **Already linked**.
3. Add a **Message** if you like. Say which programme or grant you want to follow, and why. They see it when they review your request.
4. Choose **Send request**.

The request then shows under **Outgoing** as pending. To withdraw it before they answer, choose **Cancel request**.

## Answer a request

Requests to you appear under **Incoming**. Only admins and managers of your organisation can answer them.

To approve, choose **Approve**. Then:

1. Pick an **Access group**. This is required. The line under it says what that group can see.
2. Optionally pick **Join at node**. This is the point in your flow where their flow joins yours, so a funder can follow the money across both.
3. Choose **Approve and grant access**.

To refuse, choose **Reject**. You can add a **Reason**, which they will see. Then choose **Reject request**. They can send a new request later.

Either dialog has **Keep pending** if you want to decide later.

## End a link

On an active incoming link, choose **Revoke access**, then confirm with **Revoke access**. They stop seeing your flows straight away. The link stays on record as revoked. To link again, they must send a new request.

Only the sharing side can revoke an active link. The asking side has no button to end it.

If you delete an access group, every link that granted that group also stops showing your flows.

## What this screen does not do

The Chains screen sets up and manages links. It does not show the other organisation's flows itself. TODO-USER: where in the app a funder views a partner's flows once a link is active.
