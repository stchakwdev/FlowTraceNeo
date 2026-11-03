---
title: FlowTrace Books
description: How to install the FlowTrace Books desktop app, connect it to your books, and what it does when the connection drops.
order: 14
section: Desktop app
---

<!-- Verified against: flowtrace/src/components/books/books-page.tsx, flowtrace/src/lib/books-release.ts, .github/workflows/rust.yml, rust/tools/release.sh, rust/docs/module-release.md, rust/crates/desktop/release/config.toml, rust/README.md, rust/crates/desktop/README.md, rust/crates/desktop/src/main.rs, rust/crates/desktop/src/model.rs, rust/crates/desktop/src/shell.rs, rust/crates/desktop/src/backend.rs, rust/crates/desktop/src/backend/spacetime.rs, rust/docs/screen-parity.md -->

FlowTrace Books is the desktop app for your books. It runs on your own computer, not in a browser. It uses the same ledger engine as the web app, so both give the same answers. When the connection drops, it keeps showing the books it last loaded.

![FlowTrace Books on Linux, on the Connect screen, with the organisation this computer's identity is a member of listed.](/docs/img/books-connect.webp "1280x1096")

![FlowTrace Books with the connection lost: a notice says the app is offline and nothing can be saved until you reconnect.](/docs/img/books-offline.webp "1280x1096")

## Which computers it runs on

Each release is built for two systems:

| System | Processor | Archive |
|---|---|---|
| Linux | 64-bit Intel or AMD (x86_64) | `flowtrace-books-<version>-linux-x86_64.tar.gz` |
| macOS | Apple silicon (arm64) | `flowtrace-books-<version>-macos-arm64.tar.gz` |

There is no Windows build and no build for Intel Macs. Linux needs a Wayland or X11 desktop and a Vulkan graphics driver.

## Install the app

Go to the [FlowTrace Books page](/books). When a release is published, it lists one archive per system, with checksums and release notes. Until then, it says the app is not released yet and lets you ask for early access. Releases are also on [GitHub](https://github.com/stchakwdev/FlowTraceNeo/releases).

Each archive holds the app (`flowtrace-books`), `INSTALL.txt` and `config.toml`.

### Check the download

Put the release's `SHA256SUMS` file next to the archive and run:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

On macOS, use `shasum -a 256 -c SHA256SUMS --ignore-missing`. Your archive should show `OK`.

### Unpack and start it

```sh
tar xzf flowtrace-books-<version>-linux-x86_64.tar.gz
./flowtrace-books-<version>-linux-x86_64/flowtrace-books
```

On Linux, `INSTALL.txt` lists one `apt-get` line for the system libraries on Debian and Ubuntu. On macOS, the app is not signed or notarised yet. The first time, right-click it and choose Open.

## Demo books or your own books

Started as it is, the app opens demo books for a made-up charity, Demo NGO Kenya. These live only in memory, so nothing you post is kept when you close the window.

To open your real books, copy `config.toml` from the archive into the app's settings folder, then restart:

- Linux: `~/.config/flowtrace-books/`
- macOS: `~/Library/Application Support/flowtrace-books/`

The file names the FlowTrace server and database. It holds no password or key. Remove it to go back to the demo.

## Connect to your books

Press Connection at the foot of the left-hand rail. The Connect screen shows your status, the Server, Database and "Token file (optional)" fields, and the Connect and Disconnect buttons. Below them is the list of organisations you belong to. Click one to open its books. Connect remembers the server, so the next start connects by itself.

### Your token file is your login

On your first connect, the server gives the app an identity. The app keeps it in a token file next to `config.toml`. Keep this file private and never send it. If you delete it, you become a new identity.

### Getting added to your organisation

A new identity belongs to no organisation, so at first you see no books. Someone must add you: a FlowTrace operator, or a member of your organisation who can close periods. Give them the eight characters of your identity shown on the Connect screen. Your organisation then appears in the list.

A "Sign in with WorkOS" button appears only if your `config.toml` holds a WorkOS client id. The release file has none.

## Working offline

If the server cannot be reached, the app still opens at once. A notice at the top of each screen says what is happening. While connecting or offline, you see the last books loaded. If no books have loaded yet, the notice asks you to check the server.

The app keeps those books only while it is open, not on disk. Start it with no connection and there is nothing to show.

Offline, every change is refused straight away with this message: "You are offline, so nothing was saved. Reconnect on the Connection screen, then try again." Changes are not queued for later, so nothing is half-saved.

## Syncing when you reconnect

The app does not reconnect by itself after the line drops. Open the Connect screen and press Reconnect. The app loads the books from the server again and every screen updates. Then make any changes you could not save.

While connected, the books stay live. When someone else changes them, your screens update. When you save, the app waits until the server has taken the change before it shows it.

## The screens

The rail has 16 screens. Keys `0` to `9` open ten of them while no field is selected.

| Screen | Key | What it holds |
|---|---|---|
| Dashboard | 0 | Key figures, funding by donor, recent movements |
| Transactions | none | Every movement of money, with its proof |
| Flows | 9 | Where money came from and where it went |
| Accounts | 1 | Chart of accounts and fiscal periods |
| Journal | 2 | Journal entries, manual entry and import |
| General ledger | 3 | Posted lines by account, with balances |
| Spend | 4 | Expense claims, bills, payments, vendors |
| Grants | 5 | Grants and budgets, with budget versus actual |
| Reconcile | 6 | Bank and mobile money statements |
| Reports | 7 | Financial statements, with CSV export |
| Campaigns | 8 | Campaigns, donations and embed code |
| Chains | none | Links to partner organisations |
| Trust Score | none | Your score and how to raise it |
| Settings | none | Organisation profile and sharing |
| Access groups | none | Who sees what |
| Embed & badges | none | The Trust Score badge and its code |

Public pages, such as donate pages and the trust badge, stay on the web. The app lets you manage them and copy their links. Each screen works like its web page, so the web guides apply, for example [Ledger](/docs/ledger) and [Reconcile](/docs/reconcile). Terms are explained in the [Glossary](/docs/glossary).
