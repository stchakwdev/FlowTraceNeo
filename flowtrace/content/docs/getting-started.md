---
title: Getting started
description: How to open the demo, sign in, see which organisation you are in, and move around the app on a computer or a phone.
order: 1
section: Get started
---

<!-- Verified against: flowtrace/src/lib/auth.ts, flowtrace/src/lib/auth-context.tsx, flowtrace/src/routes/_authenticated.tsx, flowtrace/src/routes/sign-out.tsx, flowtrace/src/lib/marketing-nav.ts, flowtrace/src/components/marketing/marketing-masthead.tsx, flowtrace/src/components/layout/app-shell.tsx, flowtrace/src/components/layout/mobile-nav.tsx, flowtrace/src/components/layout/theme-toggle.tsx, flowtrace/src/lib/navigation.ts, flowtrace/src/lib/org-context.tsx, flowtrace/src/lib/spacetime-session.ts, flowtrace/src/lib/ledger-client.ts, flowtrace/src/lib/demo/dataset.ts, flowtrace/src/components/ledger/page.tsx, flowtrace/src/components/report/report.tsx -->

FlowTrace shows where your organisation's money came from, where it went, and the proof behind each payment. This page helps you open the app, sign in, and find your way around.

![The demo open on the Dashboard: the demo banner under the top bar, and the menu on the left with its Books and Transparency groups.](/docs/img/dashboard.webp "1280x800")

## Try the demo

You can look around without an account. On the public site, choose **Explore the demo**. It opens the [dashboard](/docs/dashboard) at `/dashboard`.

The demo uses a made-up organisation called Demo NGO Kenya. Its figures are in Kenyan shillings (KES). None of it is real money.

When you are in the demo, a coloured banner sits under the top bar. It reads "Demo: you're viewing Demo NGO Kenya. Changes reset when you reload." You can try things out, such as recording a transaction. Reload the page and the demo goes back to how it started.

The banner ends with a **Back to the overview** link, which takes you back to the public site. Screens that use the demo books also show a small **Demo data** tag.

Every figure is dated. The dashboard shows "as of" and the date of the latest recorded movement. The demo figures are dated 4 February 2026.

## Sign in

On the public site, choose **Sign in**. If you are already signed in, the same button reads **Open app**.

How sign-in works depends on how your FlowTrace is set up:

- **With sign-in turned on.** FlowTrace sends you to its sign-in page, run by WorkOS. After you sign in, you come back to the screen you asked for.
- **With sign-in turned off.** There is no sign-in step. You go straight in as a demo user called Demo User.

To leave, open the account menu at the top right. It shows your initials and, on wide screens, your name. Choose **Sign out**. In the demo, the same item reads **Leave the demo**, and the menu notes that this is a **Demo account**.

## Your organisation

The top bar shows the name of the organisation you are working in. Every screen shows that organisation's records.

There is no organisation switcher in the app. FlowTrace opens the organisation your sign-in belongs to. If your account has no membership yet, you see "The books are not connected yet". This means an administrator needs to add you as a member. The notice may show an identity code. Give that code to your administrator so they can add you.

## Find your way with the rail

On a computer, the rail runs down the left side of the screen. The screen you are on is highlighted. The rail has three groups:

| Group | Screens |
| --- | --- |
| (no heading) | [Dashboard](/docs/dashboard), [Transactions](/docs/transactions), [Flows](/docs/flows) |
| Books | [Spend](/docs/spend), [Grants](/docs/grants), [Ledger](/docs/ledger), [Reconcile](/docs/reconcile), [Reports](/docs/reports) |
| Transparency | [Campaigns](/docs/campaigns), [Chains](/docs/chains), [Trust Score](/docs/trust-score), [Settings](/docs/settings) |

To make more room, choose **Collapse** at the foot of the rail. It then shows icons only. Rest on an icon to see its name. Choose the arrow to open the rail again. FlowTrace remembers your choice in this browser.

The FlowTrace name at the top left always takes you back to the dashboard.

## Use FlowTrace on a phone

On small screens the rail is hidden. Choose the menu button at the top right, labelled **Open menu**. A drawer slides in from the right with the same three groups.

Choose a screen and the drawer closes by itself. To close it without moving, choose **Close menu** or tap the shaded area beside it.

## Light and dark

The theme button sits in the top bar. Choose **Light**, **Dark** or **System**. **System** follows your device's own setting.

Next, read about the [dashboard](/docs/dashboard), or look up a word in the [glossary](/docs/glossary).
