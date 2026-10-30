//! The state behind the shell, in plain structs with no GPUI types, so it is unit-tested:
//! which screen and nav entry are showing ([`Navigation`]), which organisation is selected and
//! what the backend last reported ([`LedgerView`]), and the connect form ([`ConnectForm`]).
//! The GPUI views in `shell` and `store` only forward input here and render what these hold.

use std::sync::Arc;

use ledger_core::LedgerState;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::chains::ChainDetails;

use crate::backend::{BackendEvent, ConnectSettings, ConnectionStatus, Organisation};

/// One entry of the left navigation, in display order. Each is one screen task (U2 to U9,
/// DT1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavEntry {
    /// Key figures, funding by donor and recent movements (DT1).
    Dashboard,
    /// Every movement of money with its proofs, and recording one (DT3).
    Transactions,
    /// The flow map of money between flow nodes (DT2).
    Flows,
    /// Chart of accounts and fiscal periods (U2).
    Accounts,
    /// Journal entries and manual entry (U3).
    Journal,
    /// General ledger by account (U4).
    GeneralLedger,
    /// Spend: expense claims, bills, payments and vendors (U5, U6).
    Spend,
    /// Grants and budgets with budget versus actual (U7).
    Grants,
    /// Bank reconciliation and statement import (U8).
    Reconcile,
    /// Financial statements and CSV export (U9).
    Reports,
    /// Fundraising campaigns, donations, the preview card and embed code (DT1).
    Campaigns,
    /// Chain links to partner organisations (DT2).
    Chains,
    /// The Trust Score with how it is made, its history and how to raise it (DT4).
    TrustScore,
    /// The organisation profile and the access and sharing links (DT4).
    Settings,
    /// Access groups and their members (DT4).
    AccessGroups,
    /// The Trust Score badge and its codes (DT4).
    EmbedBadges,
}

impl NavEntry {
    /// Every entry, in navigation order: the rail's sections of `flowtrace/src/lib/navigation.ts`
    /// (Dashboard, Transactions and Flows, the books, then Transparency with Campaigns and
    /// Chains).
    pub const ALL: [Self; 16] = [
        Self::Dashboard,
        Self::Transactions,
        Self::Flows,
        Self::Accounts,
        Self::Journal,
        Self::GeneralLedger,
        Self::Spend,
        Self::Grants,
        Self::Reconcile,
        Self::Reports,
        Self::Campaigns,
        Self::Chains,
        Self::TrustScore,
        Self::Settings,
        Self::AccessGroups,
        Self::EmbedBadges,
    ];

    /// The rail section heading the entry sits under (`navSections`, `navigation.ts:16-44`):
    /// none for Dashboard and Flows, `Books` for the ledger screens, `Transparency` for
    /// Campaigns, Chains, Trust Score and Settings, with the two settings pages the web reaches
    /// from Settings (Access groups, Embed & badges) under them.
    #[must_use]
    pub fn section(self) -> Option<&'static str> {
        match self {
            Self::Dashboard | Self::Transactions | Self::Flows => None,
            Self::Campaigns
            | Self::Chains
            | Self::TrustScore
            | Self::Settings
            | Self::AccessGroups
            | Self::EmbedBadges => Some("Transparency"),
            _ => Some("Books"),
        }
    }

    /// The label in the navigation and the panel title.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Transactions => "Transactions",
            Self::Flows => "Flows",
            Self::Campaigns => "Campaigns",
            Self::Chains => "Chains",
            Self::TrustScore => "Trust Score",
            Self::Settings => "Settings",
            Self::AccessGroups => "Access groups",
            Self::EmbedBadges => "Embed & badges",
            Self::Accounts => "Accounts",
            Self::Journal => "Journal",
            Self::GeneralLedger => "General ledger",
            Self::Spend => "Spend",
            Self::Grants => "Grants",
            Self::Reconcile => "Reconcile",
            Self::Reports => "Reports",
        }
    }

    /// One line on what the screen will hold, for its placeholder panel.
    #[must_use]
    pub fn summary(self) -> &'static str {
        match self {
            Self::Dashboard => "Key figures, funding received by donor and recent movements.",
            Self::Transactions => "Every movement of money, with the proof attached to it.",
            Self::Flows => {
                "Where money came from, where it went, and the proof behind each movement."
            }
            Self::Campaigns => "Fundraising campaigns, donations, the preview card and embed code.",
            Self::Chains => "Chain links to partners whose flows are linked to yours.",
            Self::TrustScore => "The Trust Score, how it is made and how to raise it.",
            Self::Settings => "The organisation profile, access and what you share publicly.",
            Self::AccessGroups => "Who sees what of your flows, and the people in each group.",
            Self::EmbedBadges => "The Trust Score badge and its code for your website.",
            Self::Accounts => {
                "The chart of accounts and the fiscal periods, with closing and locking."
            }
            Self::Journal => "Journal entries: drafts, posting, reversals and manual entry.",
            Self::GeneralLedger => "Every posted line by account, with running balances.",
            Self::Spend => "Expense claims, bills, payments and vendors.",
            Self::Grants => "Grants and budgets, with budget versus actual.",
            Self::Reconcile => "Bank and mobile money statements matched to the ledger.",
            Self::Reports => "Financial statements and trial balance, with CSV export.",
        }
    }

    /// The key that opens this entry: `0` for Dashboard, `1` to `7` for the books screens in
    /// order (the keys they had before DT1), `8` for Campaigns, `9` for Flows. Chains and
    /// Transactions have none (the digits are spent): they open from the rail or `--entry
    /// chains` and `--entry transactions`; so do DT4's four (`--entry trust-score`, `settings`,
    /// `access-groups`, `embed-badges`).
    #[must_use]
    pub fn key(self) -> Option<&'static str> {
        Some(match self {
            Self::Dashboard => "0",
            Self::Flows => "9",
            Self::Campaigns => "8",
            Self::Accounts => "1",
            Self::Journal => "2",
            Self::GeneralLedger => "3",
            Self::Spend => "4",
            Self::Grants => "5",
            Self::Reconcile => "6",
            Self::Reports => "7",
            Self::Chains
            | Self::Transactions
            | Self::TrustScore
            | Self::Settings
            | Self::AccessGroups
            | Self::EmbedBadges => return None,
        })
    }

    /// The entry a key opens, for the keys `0` to `9`.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|entry| entry.key() == Some(key))
    }

    /// The entry's name for `--entry`: its label in lower case with hyphens, an `&` dropped
    /// (`general-ledger`, `embed-badges`).
    #[must_use]
    pub fn name(self) -> String {
        self.label()
            .to_lowercase()
            .replace(" & ", " ")
            .replace(' ', "-")
    }

    /// The entry `--entry` names: a key digit or a [`NavEntry::name`], any case.
    #[must_use]
    pub fn from_name(value: &str) -> Option<Self> {
        Self::from_key(value).or_else(|| {
            let wanted = value.to_lowercase();
            Self::ALL.into_iter().find(|entry| entry.name() == wanted)
        })
    }
}

/// What the content area shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// The panel of the selected nav entry.
    #[default]
    Books,
    /// The connect screen.
    Connect,
}

/// The selected nav entry and screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Navigation {
    /// The selected entry; its panel shows while the screen is [`Screen::Books`].
    pub entry: NavEntry,
    /// What the content area shows.
    pub screen: Screen,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            entry: NavEntry::Accounts,
            screen: Screen::Books,
        }
    }
}

impl Navigation {
    /// Shows `entry`'s panel, leaving the connect screen if it was open.
    pub fn select(&mut self, entry: NavEntry) {
        self.entry = entry;
        self.screen = Screen::Books;
    }

    /// Handles a key pressed with no modifier. Keys `0` to `9` select their entry while the
    /// books show; on the connect screen they belong to the form's fields. Returns whether the
    /// key was taken.
    pub fn press_key(&mut self, key: &str) -> bool {
        if self.screen != Screen::Books {
            return false;
        }
        let Some(entry) = NavEntry::from_key(key) else {
            return false;
        };
        self.select(entry);
        true
    }

    /// Opens the connect screen.
    pub fn open_connect(&mut self) {
        self.screen = Screen::Connect;
    }

    /// Leaves the connect screen for the books.
    pub fn close_connect(&mut self) {
        self.screen = Screen::Books;
    }
}

/// What the shell knows of the backend: its organisations, the selected one, the connection
/// status, the selected books and how many times they have changed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LedgerView {
    orgs: Vec<Organisation>,
    selected: Option<OrganizationId>,
    status: ConnectionStatus,
    snapshot: Option<Arc<LedgerState>>,
    revision: u64,
    remote: bool,
    transparency: Option<Arc<OrgTransparency>>,
    transparency_revision: u64,
    chains: Arc<Vec<ChainDetails>>,
    chains_revision: u64,
}

impl LedgerView {
    /// A view over `orgs` with the first one selected and its books `snapshot`.
    #[must_use]
    pub fn new(
        orgs: Vec<Organisation>,
        status: ConnectionStatus,
        snapshot: Option<Arc<LedgerState>>,
    ) -> Self {
        let selected = orgs.first().map(|org| org.id.clone());
        Self {
            orgs,
            selected,
            status,
            snapshot,
            revision: 0,
            remote: false,
            transparency: None,
            transparency_revision: 0,
            chains: Arc::default(),
            chains_revision: 0,
        }
    }

    /// Every chain the caller's organisations are on, either side
    /// ([`crate::backend::LedgerBackend::chains`]), as [`crate::store::StoreCore`] last read
    /// them.
    #[must_use]
    pub fn chains(&self) -> &Arc<Vec<ChainDetails>> {
        &self.chains
    }

    /// Goes up by one every time the chains are replaced.
    #[must_use]
    pub fn chains_revision(&self) -> u64 {
        self.chains_revision
    }

    /// Replaces the chains. Returns whether they differ from the ones held.
    pub fn set_chains(&mut self, chains: Vec<ChainDetails>) -> bool {
        if *self.chains == chains {
            return false;
        }
        self.chains = Arc::new(chains);
        self.chains_revision += 1;
        true
    }

    /// The selected organisation's transparency records (campaigns, donations, flow nodes,
    /// transactions, trust score), as [`crate::store::StoreCore`] last read them.
    #[must_use]
    pub fn transparency(&self) -> Option<&Arc<OrgTransparency>> {
        self.transparency.as_ref()
    }

    /// Goes up by one every time the transparency records are replaced, apart from
    /// [`LedgerView::revision`], so Dashboard and Campaigns re-derive only when they changed.
    #[must_use]
    pub fn transparency_revision(&self) -> u64 {
        self.transparency_revision
    }

    /// Replaces the transparency records. Returns whether they differ from the ones held.
    pub fn set_transparency(&mut self, records: Option<Arc<OrgTransparency>>) -> bool {
        let same = match (&self.transparency, &records) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
            _ => false,
        };
        if same {
            return false;
        }
        self.transparency = records;
        self.transparency_revision += 1;
        true
    }

    /// Marks the books as living on a server ([`crate::backend::LedgerBackend::is_remote`]), so
    /// that not being connected means being offline.
    pub fn set_remote(&mut self, remote: bool) {
        self.remote = remote;
    }

    /// Whether the books live on a server.
    #[must_use]
    pub fn is_remote(&self) -> bool {
        self.remote
    }

    /// The notice every screen shows above its content while the books on a server are not
    /// live: the last books seen stay readable and writes are refused
    /// ([`crate::backend::BackendError::Offline`]). `None` while connected and for books that
    /// live in the app.
    #[must_use]
    pub fn offline_notice(&self) -> Option<String> {
        if !self.remote {
            return None;
        }
        let books = self.snapshot.is_some();
        let text = match (&self.status, books) {
            (ConnectionStatus::Connected { .. }, _) => return None,
            (ConnectionStatus::Connecting, true) => {
                "Connecting. These are the last books seen; changes wait until the connection is up."
                    .to_owned()
            }
            (ConnectionStatus::Connecting, false) => {
                "Connecting to the server. The books show once they have loaded.".to_owned()
            }
            (ConnectionStatus::Disconnected | ConnectionStatus::Error { .. }, true) => {
                "You are offline. These are the last books seen, and nothing can be saved until \
                 you reconnect on the Connection screen."
                    .to_owned()
            }
            (ConnectionStatus::Disconnected | ConnectionStatus::Error { .. }, false) => {
                "You are offline and no books have loaded. Check the server on the Connection \
                 screen."
                    .to_owned()
            }
        };
        Some(text)
    }

    /// Every organisation, in the backend's order.
    #[must_use]
    pub fn organisations(&self) -> &[Organisation] {
        &self.orgs
    }

    /// The selected organisation.
    #[must_use]
    pub fn selected(&self) -> Option<&Organisation> {
        let id = self.selected.as_ref()?;
        self.orgs.iter().find(|org| &org.id == id)
    }

    /// The connection status last reported.
    #[must_use]
    pub fn status(&self) -> &ConnectionStatus {
        &self.status
    }

    /// The selected organisation's books.
    #[must_use]
    pub fn snapshot(&self) -> Option<&Arc<LedgerState>> {
        self.snapshot.as_ref()
    }

    /// Goes up by one every time the snapshot is replaced, so a screen can tell whether it
    /// must re-derive its rows.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Selects `org` with its books `snapshot`. Returns false, changing nothing, when `org` is
    /// not one of the organisations.
    pub fn select(&mut self, org: &OrganizationId, snapshot: Option<Arc<LedgerState>>) -> bool {
        if !self.orgs.iter().any(|o| &o.id == org) {
            return false;
        }
        self.selected = Some(org.clone());
        self.replace_snapshot(snapshot);
        true
    }

    fn replace_snapshot(&mut self, snapshot: Option<Arc<LedgerState>>) {
        self.snapshot = snapshot;
        self.revision += 1;
    }

    /// Applies one drained batch of backend events in order. Ledger changes of other
    /// organisations are ignored; when the selected organisation leaves the list, the first one
    /// is selected and its snapshot cleared until the caller selects it again. Returns whether
    /// anything shown changed.
    pub fn apply(&mut self, batch: Vec<BackendEvent>) -> bool {
        let mut changed = false;
        for event in batch {
            match event {
                BackendEvent::Ledger { org, state } => {
                    if self.selected.as_ref() == Some(&org) {
                        self.replace_snapshot(Some(state));
                        changed = true;
                    }
                }
                BackendEvent::Organisations(orgs) => {
                    self.orgs = orgs;
                    let kept = self
                        .selected
                        .as_ref()
                        .is_some_and(|id| self.orgs.iter().any(|o| &o.id == id));
                    if !kept {
                        self.selected = self.orgs.first().map(|org| org.id.clone());
                        self.replace_snapshot(None);
                    }
                    changed = true;
                }
                BackendEvent::Status(status) => {
                    changed |= self.status != status;
                    self.status = status;
                }
                // The store re-reads the records from the backend
                // (`crate::store::StoreCore::apply`).
                BackendEvent::Transparency { .. } | BackendEvent::Chains => {}
            }
        }
        changed
    }

    /// The status bar text: the organisation, its currency and the connection.
    #[must_use]
    pub fn status_text(&self) -> String {
        let connection = match &self.status {
            ConnectionStatus::Disconnected if self.remote => "Offline".to_owned(),
            status => status_label(status),
        };
        match self.selected() {
            Some(org) => format!("{} · {} · {connection}", org.name, org.base_currency),
            None => format!("No organisation · {connection}"),
        }
    }
}

/// How the status bar names a connection status.
#[must_use]
pub fn status_label(status: &ConnectionStatus) -> String {
    match status {
        ConnectionStatus::Disconnected => "Not connected, local books".to_owned(),
        ConnectionStatus::Connecting => "Connecting".to_owned(),
        ConnectionStatus::Connected { identity } => {
            format!("Connected as {}", short_identity(identity))
        }
        ConnectionStatus::Error { message } => format!("Connection error: {message}"),
    }
}

/// The short form of an identity: its first eight hex digits, as `spacetime login show` and the
/// TS client's settings page shorten it. Shorter names (the in-process `in-process`) stay whole.
#[must_use]
pub fn short_identity(identity: &str) -> String {
    let hex = identity.trim_start_matches("0x");
    if hex.len() > 16 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        hex[..8].to_owned()
    } else {
        identity.to_owned()
    }
}

/// The Connection screen's sentence on the status: what the books on screen are and whether
/// changes are saved.
#[must_use]
pub fn connection_summary(status: &ConnectionStatus, remote: bool) -> String {
    match (status, remote) {
        (ConnectionStatus::Disconnected, false) => {
            "Not connected. These are the demo books in the app; nothing is saved.".to_owned()
        }
        (ConnectionStatus::Disconnected, true) => {
            "Disconnected. The last books seen stay on screen; nothing can be saved.".to_owned()
        }
        (ConnectionStatus::Connected { identity }, _) => format!(
            "Connected as {}. The identity is kept in the token file, so the next start \
             connects as the same identity.",
            short_identity(identity)
        ),
        (other, _) => status_label(other),
    }
}

/// The connect screen's fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectForm {
    /// Server URL.
    pub host: String,
    /// Database name.
    pub database: String,
    /// Auth token file; may be empty.
    pub token_path: String,
}

/// The local SpacetimeDB server (`PRIMER.md`, environment on Omarchy).
pub const DEFAULT_HOST: &str = "http://127.0.0.1:3100";

/// The maincloud dev database name of pre-decision P8.
pub const DEFAULT_DATABASE: &str = "flowtrace-ledger-dev";

impl Default for ConnectForm {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_owned(),
            database: DEFAULT_DATABASE.to_owned(),
            token_path: String::new(),
        }
    }
}

impl ConnectForm {
    /// The settings the form describes, trimmed.
    ///
    /// # Errors
    ///
    /// A message for the form when the host is not an `http`, `https`, `ws` or `wss` URL, or
    /// the database name is empty or holds anything but lower-case letters, digits and `-`.
    pub fn validate(&self) -> Result<ConnectSettings, String> {
        let host = self.host.trim();
        let has_address = ["http://", "https://", "ws://", "wss://"]
            .iter()
            .any(|scheme| {
                host.strip_prefix(scheme)
                    .is_some_and(|rest| !rest.is_empty())
            });
        if !has_address {
            return Err("Enter the server as a URL, for example http://127.0.0.1:3100.".to_owned());
        }
        let database = self.database.trim();
        let valid_name = !database.is_empty()
            && database
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_name {
            return Err(
                "Enter the database name in lower-case letters, digits and dashes.".to_owned(),
            );
        }
        Ok(ConnectSettings {
            host: host.to_owned(),
            database: database.to_owned(),
            token_path: self.token_path.trim().to_owned(),
            access_token: None,
        })
    }
}

/// Where a scrolling key takes the content area: `scrolled` is how far it is scrolled down,
/// `page` its visible height and `max` the furthest it scrolls, all in pixels. `pagedown` and
/// `pageup` move by a page less one 40px row of overlap, `home` and `end` go to the ends; any
/// other key is not a scrolling key.
#[must_use]
pub fn scroll_target(key: &str, scrolled: f32, page: f32, max: f32) -> Option<f32> {
    let step = (page - 40.0).max(40.0);
    let target = match key {
        "pagedown" => scrolled + step,
        "pageup" => scrolled - step,
        "home" => 0.0,
        "end" => max,
        _ => return None,
    };
    Some(target.clamp(0.0, max.max(0.0)))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::demo::build_demo_ledger;

    fn org(id: &str, name: &str) -> Organisation {
        Organisation {
            id: OrganizationId::from(id),
            name: name.to_owned(),
            base_currency: "KES".to_owned(),
        }
    }

    fn books() -> Arc<LedgerState> {
        Arc::new(build_demo_ledger().unwrap())
    }

    fn view() -> LedgerView {
        LedgerView::new(
            vec![org("a", "Alpha"), org("b", "Beta")],
            ConnectionStatus::Disconnected,
            Some(books()),
        )
    }

    #[test]
    fn nav_has_the_sixteen_entries_in_the_webs_rail_order() {
        let labels: Vec<_> = NavEntry::ALL.iter().map(|e| e.label()).collect();
        assert_eq!(
            labels,
            [
                "Dashboard",
                "Transactions",
                "Flows",
                "Accounts",
                "Journal",
                "General ledger",
                "Spend",
                "Grants",
                "Reconcile",
                "Reports",
                "Campaigns",
                "Chains",
                "Trust Score",
                "Settings",
                "Access groups",
                "Embed & badges"
            ]
        );
        let sections: Vec<_> = NavEntry::ALL.iter().map(|e| e.section()).collect();
        assert_eq!(sections[..3], [None, None, None]);
        assert!(sections[3..10].iter().all(|s| *s == Some("Books")));
        assert!(sections[10..].iter().all(|s| *s == Some("Transparency")));
        for entry in NavEntry::ALL {
            assert_ne!(entry.summary(), "");
            assert!(entry.summary().is_ascii(), "no emoji: {}", entry.summary());
        }
    }

    #[test]
    fn keys_zero_to_nine_keep_their_entries_and_chains_and_transactions_have_none() {
        let keys: Vec<(NavEntry, Option<&str>)> =
            NavEntry::ALL.iter().map(|e| (*e, e.key())).collect();
        assert_eq!(
            keys,
            [
                (NavEntry::Dashboard, Some("0")),
                (NavEntry::Transactions, None),
                (NavEntry::Flows, Some("9")),
                (NavEntry::Accounts, Some("1")),
                (NavEntry::Journal, Some("2")),
                (NavEntry::GeneralLedger, Some("3")),
                (NavEntry::Spend, Some("4")),
                (NavEntry::Grants, Some("5")),
                (NavEntry::Reconcile, Some("6")),
                (NavEntry::Reports, Some("7")),
                (NavEntry::Campaigns, Some("8")),
                (NavEntry::Chains, None),
                (NavEntry::TrustScore, None),
                (NavEntry::Settings, None),
                (NavEntry::AccessGroups, None),
                (NavEntry::EmbedBadges, None),
            ]
        );
        for entry in NavEntry::ALL {
            if let Some(key) = entry.key() {
                assert_eq!(NavEntry::from_key(key), Some(entry));
            }
        }
        for key in ["a", "", "10", "11", "F1"] {
            assert_eq!(NavEntry::from_key(key), None, "{key:?}");
        }
    }

    #[test]
    fn an_entry_is_named_by_its_digit_or_its_name() {
        for entry in NavEntry::ALL {
            assert_eq!(NavEntry::from_name(&entry.name()), Some(entry));
        }
        assert_eq!(NavEntry::from_name("chains"), Some(NavEntry::Chains));
        assert_eq!(
            NavEntry::from_name("transactions"),
            Some(NavEntry::Transactions)
        );
        assert_eq!(NavEntry::from_name("flows"), Some(NavEntry::Flows));
        assert_eq!(
            NavEntry::from_name("general-ledger"),
            Some(NavEntry::GeneralLedger)
        );
        assert_eq!(NavEntry::from_name("9"), Some(NavEntry::Flows));
        assert_eq!(NavEntry::from_name("3"), Some(NavEntry::GeneralLedger));
        assert_eq!(
            NavEntry::from_name("Chains"),
            Some(NavEntry::Chains),
            "any case"
        );
        assert_eq!(NavEntry::from_name("ledger"), None);
        for (name, entry) in [
            ("trust-score", NavEntry::TrustScore),
            ("settings", NavEntry::Settings),
            ("access-groups", NavEntry::AccessGroups),
            ("embed-badges", NavEntry::EmbedBadges),
        ] {
            assert_eq!(NavEntry::from_name(name), Some(entry), "{name}");
            assert_eq!(entry.section(), Some("Transparency"));
        }
    }

    #[test]
    fn a_number_key_selects_its_entry_on_the_books_screen_only() {
        let mut nav = Navigation::default();
        assert_eq!(nav.entry, NavEntry::Accounts);
        assert!(nav.press_key("4"));
        assert_eq!(nav.entry, NavEntry::Spend);
        assert!(!nav.press_key("a"));
        assert_eq!(nav.entry, NavEntry::Spend);
        assert!(nav.press_key("9"));
        assert_eq!(nav.entry, NavEntry::Flows);

        nav.open_connect();
        assert_eq!(nav.screen, Screen::Connect);
        assert!(!nav.press_key("2"), "the form's fields take digits");
        assert_eq!(nav.entry, NavEntry::Flows);

        nav.close_connect();
        assert_eq!(nav.screen, Screen::Books);
        assert!(nav.press_key("7"));
        assert_eq!(nav.entry, NavEntry::Reports);
    }

    #[test]
    fn selecting_an_entry_leaves_the_connect_screen() {
        let mut nav = Navigation::default();
        nav.open_connect();
        nav.select(NavEntry::Grants);
        assert_eq!(
            nav,
            Navigation {
                entry: NavEntry::Grants,
                screen: Screen::Books
            }
        );
    }

    #[test]
    fn the_first_org_starts_selected() {
        let view = view();
        assert_eq!(view.selected().unwrap().name, "Alpha");
        assert!(view.snapshot().is_some());
        assert_eq!(view.revision(), 0);
        assert_eq!(
            LedgerView::new(vec![], ConnectionStatus::Disconnected, None).selected(),
            None
        );
    }

    #[test]
    fn switching_orgs_replaces_the_snapshot() {
        let mut view = view();
        let other = Arc::new(LedgerState::clone(&books()));
        assert!(view.select(&OrganizationId::from("b"), Some(Arc::clone(&other))));
        assert_eq!(view.selected().unwrap().name, "Beta");
        assert!(Arc::ptr_eq(view.snapshot().unwrap(), &other));
        assert_eq!(view.revision(), 1);

        assert!(!view.select(&OrganizationId::from("zz"), None));
        assert_eq!(view.selected().unwrap().name, "Beta");
        assert_eq!(view.revision(), 1);
    }

    #[test]
    fn a_batch_applies_changes_to_the_selected_org_only() {
        let mut view = view();
        let newer = books();
        let changed = view.apply(vec![
            BackendEvent::Ledger {
                org: OrganizationId::from("b"),
                state: books(),
            },
            BackendEvent::Ledger {
                org: OrganizationId::from("a"),
                state: Arc::clone(&newer),
            },
            BackendEvent::Status(ConnectionStatus::Connecting),
        ]);
        assert!(changed);
        assert!(Arc::ptr_eq(view.snapshot().unwrap(), &newer));
        assert_eq!(view.revision(), 1);
        assert_eq!(view.status(), &ConnectionStatus::Connecting);

        assert!(!view.apply(vec![BackendEvent::Ledger {
            org: OrganizationId::from("b"),
            state: books(),
        }]));
        assert_eq!(view.revision(), 1);
        assert!(!view.apply(vec![]));
    }

    #[test]
    fn a_new_org_list_keeps_the_selection_or_falls_back_to_the_first() {
        let mut view = view();
        assert!(view.apply(vec![BackendEvent::Organisations(vec![
            org("c", "Gamma"),
            org("a", "Alpha renamed"),
        ])]));
        assert_eq!(view.selected().unwrap().name, "Alpha renamed");
        assert!(view.snapshot().is_some());

        assert!(view.apply(vec![BackendEvent::Organisations(vec![org("c", "Gamma")])]));
        assert_eq!(view.selected().unwrap().name, "Gamma");
        assert!(
            view.snapshot().is_none(),
            "Gamma's books are not loaded yet"
        );

        assert!(view.apply(vec![BackendEvent::Organisations(vec![])]));
        assert_eq!(view.selected(), None);
    }

    #[test]
    fn status_text_names_the_org_currency_and_connection() {
        let mut view = view();
        assert_eq!(
            view.status_text(),
            "Alpha · KES · Not connected, local books"
        );
        view.apply(vec![BackendEvent::Status(ConnectionStatus::Connected {
            identity: "c200abcd".to_owned(),
        })]);
        assert_eq!(view.status_text(), "Alpha · KES · Connected as c200abcd");
        let empty = LedgerView::new(vec![], ConnectionStatus::Connecting, None);
        assert_eq!(empty.status_text(), "No organisation · Connecting");
    }

    #[test]
    fn remote_books_show_an_offline_notice_unless_connected() {
        let mut local = view();
        assert_eq!(
            local.offline_notice(),
            None,
            "in-app books are never offline"
        );

        let mut remote = LedgerView::new(vec![], ConnectionStatus::Connecting, None);
        remote.set_remote(true);
        assert!(
            remote
                .offline_notice()
                .unwrap()
                .starts_with("Connecting to the server")
        );

        let id = "c2008f29".repeat(8);
        remote.apply(vec![
            BackendEvent::Organisations(vec![org("a", "Alpha")]),
            BackendEvent::Ledger {
                org: OrganizationId::from("a"),
                state: books(),
            },
            BackendEvent::Status(ConnectionStatus::Connected {
                identity: id.clone(),
            }),
        ]);
        assert_eq!(remote.offline_notice(), None);
        assert_eq!(remote.status_text(), "Alpha · KES · Connected as c2008f29");

        remote.apply(vec![BackendEvent::Status(ConnectionStatus::Error {
            message: "connection reset".to_owned(),
        })]);
        assert!(remote.snapshot().is_some(), "the last books stay");
        assert!(
            remote
                .offline_notice()
                .unwrap()
                .starts_with("You are offline. These are the last books seen")
        );
        remote.apply(vec![BackendEvent::Status(ConnectionStatus::Disconnected)]);
        assert_eq!(remote.status_text(), "Alpha · KES · Offline");

        let mut empty = LedgerView::new(vec![], ConnectionStatus::Disconnected, None);
        empty.set_remote(true);
        assert!(
            empty
                .offline_notice()
                .unwrap()
                .starts_with("You are offline and no books")
        );

        local.apply(vec![BackendEvent::Status(ConnectionStatus::Disconnected)]);
        assert_eq!(local.offline_notice(), None);
    }

    #[test]
    fn the_connection_screen_says_what_the_books_are() {
        assert!(connection_summary(&ConnectionStatus::Disconnected, false).contains("demo books"));
        assert!(
            connection_summary(&ConnectionStatus::Disconnected, true).starts_with("Disconnected.")
        );
        assert!(
            connection_summary(
                &ConnectionStatus::Connected {
                    identity: "c2008f29".repeat(8)
                },
                true
            )
            .starts_with("Connected as c2008f29.")
        );
        assert_eq!(
            connection_summary(
                &ConnectionStatus::Error {
                    message: "refused".to_owned()
                },
                true
            ),
            "Connection error: refused"
        );
    }

    #[test]
    fn identities_shorten_to_eight_hex_digits() {
        let id = "c2008f29".repeat(8);
        assert_eq!(short_identity(&id), "c2008f29");
        assert_eq!(short_identity(&format!("0x{id}")), "c2008f29");
        assert_eq!(short_identity("in-process"), "in-process");
    }

    #[test]
    fn status_labels() {
        assert_eq!(
            status_label(&ConnectionStatus::Disconnected),
            "Not connected, local books"
        );
        assert_eq!(status_label(&ConnectionStatus::Connecting), "Connecting");
        assert_eq!(
            status_label(&ConnectionStatus::Connected {
                identity: "in-process".to_owned()
            }),
            "Connected as in-process"
        );
        assert_eq!(
            status_label(&ConnectionStatus::Error {
                message: "refused".to_owned()
            }),
            "Connection error: refused"
        );
    }

    #[test]
    fn the_connect_form_validates_and_trims() {
        let form = ConnectForm::default();
        assert_eq!(
            form.validate().unwrap(),
            ConnectSettings {
                host: DEFAULT_HOST.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                token_path: String::new(),
                access_token: None,
            }
        );

        let form = ConnectForm {
            host: "  https://maincloud.spacetimedb.com ".to_owned(),
            database: " ft-u1-test ".to_owned(),
            token_path: " /tmp/token ".to_owned(),
        };
        let settings = form.validate().unwrap();
        assert_eq!(settings.host, "https://maincloud.spacetimedb.com");
        assert_eq!(settings.database, "ft-u1-test");
        assert_eq!(settings.token_path, "/tmp/token");

        for host in ["", "127.0.0.1:3100", "ftp://x", "http://"] {
            let form = ConnectForm {
                host: host.to_owned(),
                ..ConnectForm::default()
            };
            assert!(form.validate().unwrap_err().contains("server"), "{host:?}");
        }
        for database in ["", "  ", "Ledger", "ledger_dev", "a b"] {
            let form = ConnectForm {
                database: database.to_owned(),
                ..ConnectForm::default()
            };
            assert!(
                form.validate().unwrap_err().contains("database"),
                "{database:?}"
            );
        }
    }

    #[test]
    fn scrolling_keys_move_by_a_page_and_stop_at_the_ends() {
        assert_eq!(scroll_target("pagedown", 0.0, 1000.0, 2500.0), Some(960.0));
        assert_eq!(
            scroll_target("pagedown", 2000.0, 1000.0, 2500.0),
            Some(2500.0)
        );
        assert_eq!(scroll_target("pageup", 500.0, 1000.0, 2500.0), Some(0.0));
        assert_eq!(scroll_target("end", 0.0, 1000.0, 2500.0), Some(2500.0));
        assert_eq!(scroll_target("home", 900.0, 1000.0, 2500.0), Some(0.0));
        assert_eq!(scroll_target("end", 0.0, 1000.0, -5.0), Some(0.0));
        assert_eq!(scroll_target("pagedown", 0.0, 10.0, 100.0), Some(40.0));
        assert_eq!(scroll_target("a", 0.0, 1000.0, 2500.0), None);
    }
}
