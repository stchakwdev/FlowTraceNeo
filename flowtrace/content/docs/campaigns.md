---
title: Campaigns and public pages
description: How to run a fundraising campaign, and what visitors see on the donate page, the campaign widget and the Trust Score badge.
order: 10
section: Transparency
---

<!-- Verified against: flowtrace/src/routes/_authenticated/campaigns/index.tsx, flowtrace/src/routes/_authenticated/campaigns/$id.tsx, flowtrace/src/components/campaign-create-modal.tsx, flowtrace/src/components/campaign-card.tsx, flowtrace/src/components/donation-modal.tsx, flowtrace/src/components/public-donation-form.tsx, flowtrace/src/components/public-campaign-header.tsx, flowtrace/src/routes/donate/$slug.tsx, flowtrace/src/routes/embed/campaign.$id.tsx, flowtrace/src/components/campaign-embed-widget.tsx, flowtrace/src/routes/_authenticated/settings/embed.tsx, flowtrace/src/components/embed-code-generator.tsx, flowtrace/src/routes/badge/trust.$orgId.tsx, flowtrace/src/components/trust-badge-preview.tsx, flowtrace/src/lib/badge-utils.ts, flowtrace/src/lib/demo/api.ts, flowtrace/src/lib/ledger/public-sql.ts, rust/crates/ledger-module/src/transparency/public.rs, rust/crates/ledger-module/src/transparency/flows.rs -->

A campaign raises money for one goal. You set it up in the app at `/campaigns`. Each public campaign also gets a donate page that anyone can open. You can put a small campaign widget and a Trust Score badge on your own website. This page covers all of them.

![The Campaigns screen: total raised, open and completed campaigns, and a list of campaigns with their status and progress to target.](/docs/img/campaigns.webp "1280x800")

## Create a campaign

Open **Campaigns** and choose **New campaign**. Fill in the form:

- **Campaign name** (required). The name donors will see.
- **One-line summary**. Shown on the donate page and in widgets. Up to 150 characters.
- **The story**. What the money will do, where, and for whom.
- **Target** (required) and **Currency**. The target must be more than zero.
- **Project it funds**. Pick a project or department, or **Not tied to one project**. Spending from that project then shows as proof on the campaign.
- **Category** and **End date**. Leave the end date empty for no end date.
- **Public donate page**. When ticked, anyone with the link can see the campaign and give.
- **Keep accepting gifts after the target is reached**.

Then choose one of two buttons. **Save as draft** keeps it private while you work on it. **Create and open to donors** makes it active straight away. FlowTrace makes the page link from the campaign name.

## Manage a campaign

The list shows four figures at the top: **Raised across campaigns**, **Open to donors**, **Confirmed donations** and **Completed**. You can search by name or category, and filter by status.

Open a campaign to see its page in the app. Here you find:

- **Funding coverage**: raised against target, and how much is spent with proof.
- **Donations**: each gift with donor, date, payment and amount.
- **Recent programme spending**: payments out of the campaign's project, with their proof state.
- **Details**, including the **Public link** when the campaign is public.

Use **Record donation** to add a gift you received another way. The **More campaign actions** menu lets you **Open to donors**, **Pause campaign** or **Resume campaign**. You can only delete a draft, with **Delete draft**. A completed or cancelled campaign has no actions.

No payment provider confirms gifts yet. Every new gift starts as pending. When the money arrives, choose **Confirm received** on that donation.

## The donate page

A visitor opens the donate page at `/donate/<campaign>`. It only works when the campaign is public, active and has a link. Otherwise the visitor sees "This campaign is not taking donations".

The visitor sees:

- Your organisation's name, a mark if it is verified, and your Trust Score.
- The campaign name, summary, start date and timing.
- **Raised so far**, with the number of **Gifts** and **Donors**, and days left.
- **The story**, if you wrote one.
- **Where the money goes**: each project, what it received, and how much has verified proof.
- **Recent spending, with proof**: each payment with its date, amount and proof state.
- A form to give, and a **Share** section.

In the form, the donor picks an amount and **Where it goes**, and chooses how to pay. An email is required. They can tick **Give anonymously**. Their name is then hidden from the public, but your organisation still sees it.

The donate page does not show the list of donors or their gifts, who created the campaign, or its internal notes. After the end date, the form is replaced by a closing note. The spending and proof stay on the page.

## The campaign widget

The widget shows one campaign on another website. To get its code, go to **Settings**, then **Embed & badges** at `/settings/embed`. Only active, public campaigns can be embedded.

Under **Campaign widget**, choose a **Campaign** and a **Theme**. Under **Show**, you can untick:

- **Amount raised and goal**. Hides the amount, the target and the "spent with verified proof" line.
- **Number of donors**. Hides the box with your Trust Score and the campaign timing.
- **Compact, one line**. Shows just the name, amount, percentage and a **Give** button.

Check the **Live preview**, then choose **Copy code** and paste it into your site's HTML. The widget sits at `/embed/campaign/<id>`. Its **Give** button opens the donate page while the campaign is open.

## The Trust Score badge

The badge is a small label. Your chosen words sit on the left and your current score out of 100 on the right. Make it on the same **Embed & badges** screen, under **Trust Score badge**. Pick a **Style** and a **Label** of up to 24 characters. Then copy **HTML, for websites** or **Markdown, for READMEs and docs**.

The badge page sits at `/badge/trust/<organisation>`. It shows the badge and your organisation's name. It shows nothing else.

The badge reads only what your organisation publishes. Turn on **Show on public pages** in [Settings](/docs/settings) first. If nothing is published, the badge shows 0. To learn what the number means, see [Trust Score](/docs/trust-score).
