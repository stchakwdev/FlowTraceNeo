---
title: Settings and access groups
description: How to edit your organisation profile, choose what the public sees, and set what each access group can see and do.
order: 13
section: Transparency
---

<!-- Verified against: flowtrace/src/routes/_authenticated/settings/index.tsx, flowtrace/src/routes/_authenticated/settings/access-groups.tsx, flowtrace/src/routes/_authenticated/settings/embed.tsx, flowtrace/src/components/public-pages-switch.tsx, flowtrace/src/components/embed-settings.tsx, flowtrace/src/components/chain-approval-modal.tsx, flowtrace/src/lib/types.ts, flowtrace/src/lib/queries.ts, flowtrace/src/lib/demo/dataset.ts, flowtrace/src/lib/ledger/public-sql.ts, rust/crates/ledger-module/src/transparency/plan.rs, rust/crates/ledger-module/src/transparency/public.rs -->

Settings is where you describe your organisation and decide who sees what. Open it at `/settings`. It has two parts: **Organisation profile**, and **Access and sharing**. Access groups have their own screen at `/settings/access-groups`.

![The Settings screen: the organisation profile form, and links to access groups, embed and badges, and chain links.](/docs/img/settings.webp "1280x800")

## Organisation profile

These fields describe your organisation:

- **Name**. As funders and the public will see it.
- **Type**. **NGO or nonprofit**, **Social enterprise**, **Business** or **Government body**.
- **Country**.
- **Reporting currency**. Amounts across your reports are shown in this currency.
- **Public link name**. A short name for your public pages. Use lowercase letters, numbers and single hyphens, like `demo-ngo`. Keep it under 48 characters.

Choose **Save profile** to keep your changes. The button only works once you have changed something. **Discard changes** puts the fields back as they were.

## Access and sharing

**Show on public pages** is a switch. While it is on, your name and Trust Score are published for anyone to read. The Trust Score badge reads this published profile. While it is off, the badge shows a score of 0.

Below the switch are three links:

- **Access groups**. Who sees which depth of your flows.
- **Embed & badges**. Campaign widgets and Trust Score badges for your website. See [Campaigns and public pages](/docs/campaigns).
- **Chain links**. Partners and funders whose flows are linked to yours. See [Chains](/docs/chains).

The **Embed & badges** screen also has **Embed settings**. These switches are **Allow embedding**, **Show amounts**, **Show Trust Score**, **Show node names** and **Show transaction count**. There is also an **Embed token**. **New token** makes a fresh one and stops every embed that uses the old one. Only an admin or manager can change these. The campaign widget and the badge do not read these settings.

## The standard access groups

An access group sets how deep someone can see into your flows. Every person or partner you share with goes into a group.

If you have no groups yet, choose **Create the standard groups**. This makes five default groups:

| Group | Which flows | Amounts | Proofs | Who handled it | Can change things |
| --- | --- | --- | --- | --- | --- |
| Board of Directors | All flows | Yes | Originals | Yes | Edit flows, edit transactions, manage members and groups |
| Staff | All flows | Yes | Summaries | Yes | Edit flows, edit transactions |
| Donors | What their money funds | Yes | Summaries | No | Nothing |
| Partners | Assigned nodes | Yes | None | No | Nothing |
| Public | All flows, names only | No | None | No | Nothing |

What the words mean:

- **All flows**: every node in your flows.
- **What their money funds**: the nodes downstream of the money they gave.
- **Assigned nodes**: only the nodes you assign to the group.
- **Names only**: the group sees the flow but not the amounts.
- **Originals**: the receipts, photos and documents themselves.
- **Summaries**: the type, date and status of each proof only.
- **Who handled it**: the staff member or vendor behind each payment.

You can edit a default group, but you cannot delete it or change its short ID.

## Create or edit a group

Choose **Create group**, or the edit button on a group's row. Fill in:

- **Name** and **Short ID**. The short ID is used in links and the API. When you create a group, it fills in from the name.
- **Description** (optional).
- Under **What they see**: **Which flows**, **Proofs**, and the boxes **The flow**, **Amounts** and **Who handled it**.
- Under **What they can do**: **Edit flows**, **Edit transactions**, **Manage members** and **Manage groups**.

Choose **Create group** or **Save group**. A new group starts with **Assigned nodes**, the flow and amounts, and no proofs.

To delete a custom group, use its delete button and confirm with **Delete group**. Everyone in it loses access straight away. Any chain link that granted this group stops showing your flows. This cannot be undone.

## Add people to a group

The table on the access groups screen is titled **What each group can see and do**. Select a group's name to see its members below.

There are two ways to add someone:

- **Add a member**. Pick a **Person** who is already in your organisation, then choose **Add to** and the group's name.
- **Invite someone**. Enter an **Email**. You can set **Access ends** for auditors and short reviews. Access stops at the end of that day. An invite is recorded here, but no email is sent.

Each member shows **No end date**, **Until** a date, or **Expired** with a date. To take someone out, use the remove button and confirm with **Remove member**.

When you approve a [chain link](/docs/chains), you pick one of these groups for the other organisation.
