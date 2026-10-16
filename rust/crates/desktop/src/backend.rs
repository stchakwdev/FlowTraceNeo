//! Where the desktop gets its books: the [`LedgerBackend`] trait every screen reads through,
//! and [`InProcessBackend`], which runs the ledger core in the app itself.
//!
//! The trait follows the store-entity pattern of `rust/docs/adr/0002-gpui-spacetime.md`
//! ("Pattern for U1 to U10"): a backend hands out a channel of [`BackendEvent`]s, one store
//! entity drains it in batches, and screens re-derive what they show from the store. Writes are
//! [`LedgerCommand`]s, the same commands the SpacetimeDB module's `run_command` reducer takes
//! (`spacetimedb/src/index.ts`, `run_command`), so the live client of U10 slots in behind the
//! same trait.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_channel::{Receiver, Sender};
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::types::{Actor, ActorId, OrganizationId};
use ledger_core::{CommandError, CommandResult, LedgerCommand, LedgerEnv, LedgerState, Timestamp};

/// The display name of the demo organisation (`flowtrace/src/lib/demo/dataset.ts:61`).
pub const DEMO_ORG_NAME: &str = "Demo NGO Kenya";

/// The display name of the empty organisation the in-process backend opens next to the demo.
pub const EMPTY_ORG_NAME: &str = "New organisation";

/// The base currency of the empty organisation, the demo's currency
/// (`ledger_core::demo::build_demo_ledger`).
pub const EMPTY_ORG_CURRENCY: &str = "KES";

/// What the identity of the in-process backend reads as once "connected".
pub const IN_PROCESS_IDENTITY: &str = "in-process";

/// One organisation whose books the backend holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organisation {
    /// Its id, as in `OrgLedgerSettings.organization_id`.
    pub id: OrganizationId,
    /// Display name.
    pub name: String,
    /// Base currency, for example `KES`.
    pub base_currency: String,
}

/// Where the backend stands with its server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    /// Not connected to a server.
    #[default]
    Disconnected,
    /// A connection is being opened.
    Connecting,
    /// Connected, as this identity.
    Connected {
        /// The identity the server knows the app by.
        identity: String,
    },
    /// The connection failed or dropped.
    Error {
        /// What went wrong, for the status bar.
        message: String,
    },
}

/// What the connect screen collects: the server, the database and where the auth token lives.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectSettings {
    /// Server URL, for example `http://127.0.0.1:3100`.
    pub host: String,
    /// Database name on that server.
    pub database: String,
    /// Path of the file holding the auth token; empty for none.
    pub token_path: String,
}

/// One change a backend reports. A store drains these in batches (ADR 0002, decision 4).
#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    /// An organisation's books changed; `state` is the whole new snapshot.
    Ledger {
        /// The organisation.
        org: OrganizationId,
        /// Its books after the change.
        state: Arc<LedgerState>,
    },
    /// The list of organisations changed.
    Organisations(Vec<Organisation>),
    /// The connection status changed.
    Status(ConnectionStatus),
}

/// Why a backend call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendError {
    /// No books for this organisation.
    #[error("No organisation with id {0}.")]
    UnknownOrganisation(String),
    /// The ledger refused the command; the books are unchanged.
    #[error(transparent)]
    Command(#[from] CommandError),
    /// The books could not be built.
    #[error("The books could not be opened: {0}")]
    Setup(#[from] ledger_core::engine::Error),
    /// The server could not be reached or refused the connection.
    #[error("{0}")]
    Connection(String),
}

/// The books behind the desktop: read a snapshot, watch for changes, submit commands.
///
/// Implementations: [`InProcessBackend`] (U1) and the SpacetimeDB client (U10).
pub trait LedgerBackend {
    /// Every organisation the backend holds books for, in a stable order.
    fn organisations(&self) -> Vec<Organisation>;

    /// The current books of `org`, or `None` when the backend holds none for it.
    fn state(&self, org: &OrganizationId) -> Option<Arc<LedgerState>>;

    /// A new receiver of every change from now on. Each call makes its own channel.
    fn subscribe(&mut self) -> Receiver<BackendEvent>;

    /// Applies `command` to the books of `org`. On success subscribers get a
    /// [`BackendEvent::Ledger`] with the new books.
    ///
    /// # Errors
    ///
    /// [`BackendError::UnknownOrganisation`] for an org the backend does not hold;
    /// [`BackendError::Command`] when a ledger rule refuses the command.
    fn submit(
        &mut self,
        org: &OrganizationId,
        command: LedgerCommand,
    ) -> Result<CommandResult, BackendError>;

    /// The connection status now.
    fn status(&self) -> ConnectionStatus;

    /// Connects to the server `settings` names; status changes arrive as
    /// [`BackendEvent::Status`].
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the connection cannot be started.
    fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError>;
}

/// One organisation's books in the in-process backend.
struct Books {
    org: Organisation,
    state: Arc<LedgerState>,
    env: Box<dyn LedgerEnv>,
    actor: Actor,
}

/// The ledger core in the app's own process: the demo books and one empty organisation.
///
/// Commands go through `ledger_core::apply_command`. The demo org draws ids and time from the
/// fixed demo env, continued past the ids the demo already holds; the empty org draws from
/// [`SystemEnv`]. [`LedgerBackend::connect`] only records the settings and reports itself
/// connected: the live client arrives in U10.
pub struct InProcessBackend {
    books: Vec<Books>,
    status: ConnectionStatus,
    settings: Option<ConnectSettings>,
    subscribers: Vec<Sender<BackendEvent>>,
}

impl InProcessBackend {
    /// The demo books (`ledger_core::demo::build_demo_ledger`) and an empty organisation
    /// (`ledger_core::setup::create_org_ledger`) for this year, in that order.
    ///
    /// # Errors
    ///
    /// [`BackendError::Setup`] when either set of books cannot be built.
    pub fn with_demo_and_empty() -> Result<Self, BackendError> {
        todo!()
    }

    /// The settings the last [`LedgerBackend::connect`] recorded.
    #[must_use]
    pub fn settings(&self) -> Option<&ConnectSettings> {
        self.settings.as_ref()
    }
}

impl LedgerBackend for InProcessBackend {
    fn organisations(&self) -> Vec<Organisation> {
        todo!()
    }

    fn state(&self, _org: &OrganizationId) -> Option<Arc<LedgerState>> {
        todo!()
    }

    fn subscribe(&mut self) -> Receiver<BackendEvent> {
        todo!()
    }

    fn submit(
        &mut self,
        _org: &OrganizationId,
        _command: LedgerCommand,
    ) -> Result<CommandResult, BackendError> {
        todo!()
    }

    fn status(&self) -> ConnectionStatus {
        self.status.clone()
    }

    fn connect(&mut self, _settings: ConnectSettings) -> Result<(), BackendError> {
        todo!()
    }
}

/// A [`LedgerEnv`] on the wall clock with random UUID ids, as the TS module's `envFor`
/// (`spacetimedb/src/index.ts`: `new Date().toISOString()` and `crypto.randomUUID()`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEnv;

impl LedgerEnv for SystemEnv {
    fn now(&self) -> Timestamp {
        todo!()
    }

    fn new_id(&mut self) -> String {
        todo!()
    }
}

/// `ms` milliseconds after the Unix epoch as `Date.prototype.toISOString` writes it,
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
#[must_use]
pub fn iso_millis(_ms: u64) -> String {
    todo!()
}

/// The UTC calendar year now.
#[must_use]
pub fn current_year() -> i32 {
    todo!()
}

/// A demo env that continues after the ids `state` already holds, so the next id is fresh.
#[must_use]
pub fn demo_env_after(_state: &LedgerState) -> FixedEnv {
    todo!()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::demo::{DEMO_ORG_ID, build_demo_ledger};
    use ledger_core::sanitize_command;
    use serde_json::json;

    fn backend() -> InProcessBackend {
        InProcessBackend::with_demo_and_empty().unwrap()
    }

    fn demo() -> OrganizationId {
        OrganizationId::from(DEMO_ORG_ID)
    }

    fn add_account(code: &str, name: &str) -> LedgerCommand {
        sanitize_command(&json!({
            "type": "account.add",
            "input": {
                "code": code,
                "name": name,
                "type": "expense",
                "is_group": false,
                "is_cash": false
            }
        }))
        .unwrap()
    }

    fn all_strings(state: &LedgerState) -> String {
        serde_json::to_string(state).unwrap()
    }

    #[test]
    fn opens_the_demo_books_and_an_empty_organisation() {
        let backend = backend();
        let orgs = backend.organisations();
        assert_eq!(orgs.len(), 2);
        assert_eq!(orgs[0].id, demo());
        assert_eq!(orgs[0].name, DEMO_ORG_NAME);
        assert_eq!(orgs[0].base_currency, "KES");
        assert_eq!(orgs[1].name, EMPTY_ORG_NAME);
        assert_eq!(orgs[1].base_currency, EMPTY_ORG_CURRENCY);
        assert!(uuid::Uuid::parse_str(orgs[1].id.as_str()).is_ok());

        let demo_state = backend.state(&demo()).unwrap();
        assert_eq!(*demo_state, build_demo_ledger().unwrap());

        let empty = backend.state(&orgs[1].id).unwrap();
        assert_eq!(empty.settings.organization_id, orgs[1].id);
        assert!(empty.entries.is_empty());
        assert!(!empty.accounts.is_empty(), "the NGO chart of accounts");
        assert_eq!(backend.status(), ConnectionStatus::Disconnected);
        assert_eq!(
            backend.state(&OrganizationId::from("nobody")),
            None,
            "no books for an unknown org"
        );
    }

    #[test]
    fn a_command_changes_the_books_and_reaches_subscribers() {
        let mut backend = backend();
        let rx = backend.subscribe();
        let before = backend.state(&demo()).unwrap();

        let result = backend
            .submit(&demo(), add_account("5990", "Field radios"))
            .unwrap();
        let CommandResult::Id { id } = result else {
            panic!("account.add returns an id, got {result:?}");
        };
        assert!(id.starts_with("demo-"), "drawn from the demo env: {id}");
        assert!(
            !all_strings(&before).contains(&format!("\"{id}\"")),
            "{id} was already in the demo books"
        );

        let after = backend.state(&demo()).unwrap();
        let added = after.accounts.iter().find(|a| a.code == "5990").unwrap();
        assert_eq!(added.name, "Field radios");
        assert_eq!(added.id.as_str(), id);
        assert_eq!(*before, build_demo_ledger().unwrap(), "old snapshot untouched");

        let event = rx.try_recv().unwrap();
        assert_eq!(
            event,
            BackendEvent::Ledger {
                org: demo(),
                state: Arc::clone(&after)
            }
        );
        assert!(rx.try_recv().is_err(), "one event per command");
    }

    #[test]
    fn ids_keep_counting_across_commands() {
        let mut backend = backend();
        let first = backend
            .submit(&demo(), add_account("5991", "One"))
            .unwrap();
        let second = backend
            .submit(&demo(), add_account("5992", "Two"))
            .unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn the_empty_org_draws_uuids_and_wall_clock_time() {
        let mut backend = backend();
        let org = backend.organisations()[1].id.clone();
        let result = backend
            .submit(&org, add_account("5990", "Field radios"))
            .unwrap();
        let CommandResult::Id { id } = result else {
            panic!("account.add returns an id");
        };
        assert!(uuid::Uuid::parse_str(&id).is_ok(), "{id} is a UUID");

        let state = backend.state(&org).unwrap();
        let event = state.audit.last().unwrap();
        let year = current_year().to_string();
        assert!(
            event.at.as_str().starts_with(&year),
            "audit time {} is this year",
            event.at
        );
    }

    #[test]
    fn a_refused_command_changes_nothing_and_sends_nothing() {
        let mut backend = backend();
        let rx = backend.subscribe();
        let before = backend.state(&demo()).unwrap();
        let taken = before.accounts[0].code.clone();

        let err = backend
            .submit(&demo(), add_account(&taken, "Duplicate"))
            .unwrap_err();
        assert!(matches!(err, BackendError::Command(_)), "{err:?}");
        assert_eq!(backend.state(&demo()).unwrap(), before);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn an_unknown_org_is_refused() {
        let mut backend = backend();
        let err = backend
            .submit(&OrganizationId::from("nobody"), add_account("5990", "X"))
            .unwrap_err();
        assert_eq!(err, BackendError::UnknownOrganisation("nobody".to_owned()));
    }

    #[test]
    fn every_subscriber_gets_every_change_and_closed_ones_are_dropped() {
        let mut backend = backend();
        let a = backend.subscribe();
        let b = backend.subscribe();
        drop(b);
        backend
            .submit(&demo(), add_account("5990", "Field radios"))
            .unwrap();
        assert!(matches!(a.try_recv(), Ok(BackendEvent::Ledger { .. })));
        assert_eq!(backend.subscribers.len(), 1);
    }

    #[test]
    fn connect_records_the_settings_and_reports_connected() {
        let mut backend = backend();
        let rx = backend.subscribe();
        let settings = ConnectSettings {
            host: "http://127.0.0.1:3100".to_owned(),
            database: "ft-u1-test".to_owned(),
            token_path: "/home/me/.config/flowtrace/token".to_owned(),
        };
        backend.connect(settings.clone()).unwrap();
        assert_eq!(backend.settings(), Some(&settings));
        let connected = ConnectionStatus::Connected {
            identity: IN_PROCESS_IDENTITY.to_owned(),
        };
        assert_eq!(backend.status(), connected);
        assert_eq!(
            rx.try_recv().unwrap(),
            BackendEvent::Status(ConnectionStatus::Connecting)
        );
        assert_eq!(rx.try_recv().unwrap(), BackendEvent::Status(connected));
    }

    #[test]
    fn iso_millis_writes_to_iso_string() {
        assert_eq!(iso_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_millis(1_770_195_600_000), fixed::DEMO_NOW);
        assert_eq!(iso_millis(951_868_799_999), "2000-02-29T23:59:59.999Z");
        assert!(Timestamp::parse(&iso_millis(253_402_300_799_999)).is_ok());
    }

    #[test]
    fn the_system_env_gives_fresh_uuids_and_now() {
        let mut env = SystemEnv;
        let a = env.new_id();
        let b = env.new_id();
        assert_ne!(a, b);
        assert!(uuid::Uuid::parse_str(&a).is_ok());
        assert!(env.now().as_str().starts_with(&current_year().to_string()));
        assert!(current_year() >= 2026);
    }

    #[test]
    fn the_demo_env_continues_after_the_demo_ids() {
        let state = build_demo_ledger().unwrap();
        let mut env = demo_env_after(&state);
        let next = env.new_id();
        assert!(next.starts_with("demo-"));
        assert!(!all_strings(&state).contains(&format!("\"{next}\"")));
        assert!(env.drawn() > 100, "the demo drew many ids: {}", env.drawn());
        assert_eq!(env.now().as_str(), fixed::DEMO_NOW);
    }
}
