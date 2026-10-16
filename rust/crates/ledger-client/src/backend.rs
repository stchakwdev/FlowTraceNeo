//! The [`LedgerBackend`] trait the desktop builds against (`PRIMER.md` section 1, the
//! `LedgerBackend` of `flowtrace/src/lib/ledger/backend.ts:7-25`), and the values it hands out.
//!
//! Two implementations: [`crate::SpacetimeClient`] (the module over SpacetimeDB) and
//! [`crate::InMemoryBackend`] (one set of books in memory, for screens before U10). The desktop
//! drives either the same way, per `docs/adr/0002-gpui-spacetime.md` ("Pattern for U1 to U10"):
//! one store entity takes [`LedgerBackend::subscribe`]'s receiver, drains it in batches from a
//! `cx.spawn` task, re-reads [`LedgerBackend::state`] for the organisations a batch names, and
//! notifies once. Nothing here blocks on the network: [`LedgerBackend::submit`] returns a
//! [`Ticket`] to await.

use std::collections::BTreeSet;
use std::time::Duration;

use ledger_core::types::{LedgerState, OrgLedgerSettings};
use ledger_core::{CommandResult, LedgerCommand};

use crate::error::ClientError;

/// How long a submitted command waits for its result (`RESULT_TIMEOUT_MS`,
/// `flowtrace/src/lib/ledger/spacetime-backend.ts:29`).
pub const RESULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Where the backend's connection is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Opening the WebSocket and waiting for the host's identity message.
    Connecting,
    /// Connected as `identity` (hex); the first subscription rows have not arrived yet.
    Connected {
        /// The caller's identity, 64 hex digits.
        identity: String,
    },
    /// Connected and the cache holds every row of the caller's organisations: `state` answers.
    Ready {
        /// The caller's identity, 64 hex digits.
        identity: String,
    },
    /// Not connected: closed on purpose (`error` is `None`), or lost or refused.
    Disconnected {
        /// What went wrong, when something did.
        error: Option<String>,
    },
}

impl ConnectionStatus {
    /// The identity, once the host has told it.
    #[must_use]
    pub fn identity(&self) -> Option<&str> {
        match self {
            Self::Connected { identity } | Self::Ready { identity } => Some(identity),
            Self::Connecting | Self::Disconnected { .. } => None,
        }
    }
}

/// One change notification. Row changes are batched: one [`BackendEvent::Changed`] per applied
/// transaction or subscription update, naming every organisation it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    /// The connection moved to a new status.
    Status(ConnectionStatus),
    /// Rows of these organisations changed; re-read their [`LedgerBackend::state`].
    Changed {
        /// The organisation ids.
        orgs: BTreeSet<String>,
    },
    /// The subscription or the connection reported an error; the status event that follows
    /// says whether the connection survived.
    Error(String),
}

/// A member of an organisation, from `my_member` (`spacetimedb/src/index.ts:54-66`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    /// Their identity, 64 hex digits.
    pub identity: String,
    /// The person behind approvals and the audit trail.
    pub user_id: String,
    /// Display name.
    pub name: String,
    /// May post to the ledger.
    pub can_post: bool,
    /// May approve spend.
    pub can_approve: bool,
    /// May close periods and manage members.
    pub can_close: bool,
}

/// An organisation whose books the caller can read.
#[derive(Debug, Clone, PartialEq)]
pub struct Organisation {
    /// The organisation id.
    pub org_id: String,
    /// The books' settings (`ledger_org.settings_json`).
    pub settings: OrgLedgerSettings,
    /// Its members the caller can see, in key order.
    pub members: Vec<MemberInfo>,
}

/// The pending result of a call: await [`Ticket::result`] from an async task (a GPUI
/// `cx.spawn`), or block a worker thread on [`Ticket::wait`]. It resolves once, after the rows
/// the call changed are in the backend's cache, so a [`LedgerBackend::state`] read after it sees
/// the change.
#[derive(Debug)]
pub struct Ticket<T> {
    rx: async_channel::Receiver<Result<T, ClientError>>,
}

/// What [`LedgerBackend::submit`] returns.
pub type CommandTicket = Ticket<CommandResult>;

/// The sending half of a [`Ticket`]; the first send wins, later ones are dropped.
pub(crate) type TicketSender<T> = async_channel::Sender<Result<T, ClientError>>;

impl<T> Ticket<T> {
    /// A ticket and the sender that resolves it.
    pub(crate) fn pending() -> (TicketSender<T>, Self) {
        let (tx, rx) = async_channel::bounded(1);
        (tx, Self { rx })
    }

    /// A ticket that is already resolved.
    pub(crate) fn ready(result: Result<T, ClientError>) -> Self {
        let (tx, ticket) = Self::pending();
        // A fresh channel of capacity one always takes its first value.
        let _ = tx.try_send(result);
        ticket
    }

    /// The call's result.
    ///
    /// # Errors
    ///
    /// The refusal, decoded ([`ClientError::Ledger`] for a broken rule), a timeout or a lost
    /// connection.
    pub async fn result(self) -> Result<T, ClientError> {
        self.rx
            .recv()
            .await
            .unwrap_or(Err(ClientError::Disconnected))
    }

    /// [`Ticket::result`], blocking this thread. Never call it on the UI thread.
    ///
    /// # Errors
    ///
    /// As [`Ticket::result`].
    pub fn wait(self) -> Result<T, ClientError> {
        self.rx
            .recv_blocking()
            .unwrap_or(Err(ClientError::Disconnected))
    }
}

/// What every screen reads and writes through. Object safe and `Send + Sync`, so the desktop
/// holds an `Arc<dyn LedgerBackend>` chosen at start-up.
pub trait LedgerBackend: Send + Sync {
    /// The connection's current status.
    fn status(&self) -> ConnectionStatus;

    /// The organisations whose books the caller can read, in id order.
    fn organisations(&self) -> Vec<Organisation>;

    /// One organisation's books as the backend holds them now (`load`,
    /// `spacetime-backend.ts:193-197`).
    ///
    /// # Errors
    ///
    /// [`ClientError::NoLedger`] when the caller cannot read the books (yet), or
    /// [`ClientError::Codec`] when the rows do not rebuild.
    fn state(&self, org_id: &str) -> Result<LedgerState, ClientError>;

    /// A new receiver of change notifications. Its first event is the current
    /// [`BackendEvent::Status`], so a subscriber never misses the status it starts in; after
    /// that every receiver gets every event. Drop it to stop.
    fn subscribe(&self) -> async_channel::Receiver<BackendEvent>;

    /// Runs one command on an organisation's books as the caller (`run`,
    /// `spacetime-backend.ts:199-217`). The actor is whoever the backend says the caller is,
    /// never a parameter (`spacetime-backend.ts:6-8`).
    fn submit(&self, org_id: &str, command: LedgerCommand) -> CommandTicket;
}
