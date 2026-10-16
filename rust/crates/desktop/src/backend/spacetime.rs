//! [`SpacetimeBackend`]: the books on a SpacetimeDB server, through `ledger-client`'s
//! [`SpacetimeClient`], behind the same [`LedgerBackend`] trait as the in-process books.
//!
//! The shape is the dated S3 section of `rust/docs/adr/0002-gpui-spacetime.md`: one forwarder
//! thread per connection owns `client.subscribe()`. For each [`ClientEvent`] it re-reads what
//! changed (`client.organisations()`, `client.state(org)` for each org a `Changed` names), keeps
//! the snapshots as `Arc`s and sends the matching [`BackendEvent`]s to every subscriber, so the
//! store entity only ever drains plain events and the UI thread never rebuilds books from rows.
//! The snapshots outlive the connection: while disconnected the screens keep the last books seen
//! and [`LedgerBackend::send`] refuses writes with [`BackendError::Offline`].
//!
//! Writes go through [`SpacetimeClient::send`], whose ticket resolves after the changed rows are
//! in the client's cache; the store awaits it in a task ([`LedgerBackend::send`]). The blocking
//! [`LedgerBackend::submit`] is for worker threads and tests only.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use async_channel::{Receiver, Sender};
use ledger_client::{
    ClientConfig, ClientError, ClientEvent, ConnectionStatus as ClientStatus,
    Organisation as ClientOrganisation, SpacetimeClient, TokenFile,
};
use ledger_core::engine;
use ledger_core::types::{Actor, ActorId, OrganizationId};
use ledger_core::{CommandError, CommandResult, LedgerCommand, LedgerState};

use super::{
    BackendError, BackendEvent, ConnectSettings, ConnectionStatus, LedgerBackend, Organisation,
    Submission,
};

/// What [`SpacetimeBackend`] needs of a client: [`SpacetimeClient`] in the app, a fake in tests.
pub trait LiveClient: Send + Sync {
    /// The connection status now ([`SpacetimeClient::status`]).
    fn status(&self) -> ClientStatus;
    /// The organisations the identity can read ([`SpacetimeClient::organisations`]).
    fn organisations(&self) -> Vec<ClientOrganisation>;
    /// One organisation's books rebuilt from the cache ([`SpacetimeClient::state`]).
    ///
    /// # Errors
    ///
    /// As [`SpacetimeClient::state`].
    fn state(&self, org: &str) -> Result<LedgerState, ClientError>;
    /// A new receiver of change events ([`SpacetimeClient::subscribe`]).
    fn subscribe(&self) -> Receiver<ClientEvent>;
    /// Sends a command without waiting ([`SpacetimeClient::send`]).
    fn send(&self, org: &str, command: &LedgerCommand) -> Submission;
    /// Sends a command and blocks until its result ([`SpacetimeClient::submit`]).
    ///
    /// # Errors
    ///
    /// As [`SpacetimeClient::submit`].
    fn submit(&self, org: &str, command: &LedgerCommand) -> Result<CommandResult, ClientError>;
    /// Closes the connection ([`SpacetimeClient::disconnect`]).
    fn disconnect(&self);
}

impl LiveClient for SpacetimeClient {
    fn status(&self) -> ClientStatus {
        Self::status(self)
    }

    fn organisations(&self) -> Vec<ClientOrganisation> {
        Self::organisations(self)
    }

    fn state(&self, org: &str) -> Result<LedgerState, ClientError> {
        Self::state(self, org)
    }

    fn subscribe(&self) -> Receiver<ClientEvent> {
        Self::subscribe(self)
    }

    fn send(&self, org: &str, command: &LedgerCommand) -> Submission {
        let ticket = Self::send(self, org, command);
        Submission::pending(async move { ticket.result().await.map_err(backend_error) })
    }

    fn submit(&self, org: &str, command: &LedgerCommand) -> Result<CommandResult, ClientError> {
        Self::submit(self, org, command)
    }

    fn disconnect(&self) {
        Self::disconnect(self);
    }
}

/// A client error as the screens show it. A broken ledger rule becomes the same
/// [`BackendError::Command`] the in-process books raise, so a screen's override box and issue
/// list work alike on both; a lost connection reads as [`BackendError::Offline`].
#[must_use]
pub fn backend_error(error: ClientError) -> BackendError {
    match error {
        ClientError::Ledger(e) => {
            BackendError::Command(CommandError::Engine(engine::Error::Ledger(e)))
        }
        ClientError::Disconnected => BackendError::Offline,
        other => BackendError::Connection(other.to_string()),
    }
}

/// The desktop's status for the client's. Books are only readable once the client is `Ready`
/// (`ledger_client::ConnectionStatus`), so `Connected` still reads as connecting; a disconnect
/// with a reason is an error.
#[must_use]
pub fn status_of(status: &ClientStatus) -> ConnectionStatus {
    match status {
        ClientStatus::Connecting | ClientStatus::Connected { .. } => ConnectionStatus::Connecting,
        ClientStatus::Ready { identity } => ConnectionStatus::Connected {
            identity: identity.clone(),
        },
        ClientStatus::Disconnected { error: None } => ConnectionStatus::Disconnected,
        ClientStatus::Disconnected {
            error: Some(message),
        } => ConnectionStatus::Error {
            message: message.clone(),
        },
    }
}

/// An organisation in the picker. The module keeps no organisation name (`ledger_org` holds the
/// id and the settings, `crates/ledger-module/README.md`), so the id is the name.
fn organisation_of(org: &ClientOrganisation) -> Organisation {
    Organisation {
        id: org.settings.organization_id.clone(),
        name: org.org_id.clone(),
        base_currency: org.settings.base_currency.clone(),
    }
}

/// Identities as the SDK writes them (`to_hex`) or as `spacetime call` takes them (`0x...`).
fn same_identity(a: &str, b: &str) -> bool {
    let bare = |s: &str| s.trim_start_matches("0x").to_ascii_lowercase();
    bare(a) == bare(b)
}

/// Who `identity` acts as in `org`: its member row, mapped as the module's `actor_of`
/// (`crates/ledger-module/src/rules.rs`, `index.ts:507-511`) maps it for `run_command`.
#[must_use]
pub fn actor_for(org: &ClientOrganisation, identity: &str) -> Option<Actor> {
    org.members
        .iter()
        .find(|m| same_identity(&m.identity, identity))
        .map(|m| Actor {
            id: ActorId::from(m.user_id.as_str()),
            name: m.name.clone(),
            can_post: m.can_post,
            can_approve: m.can_approve,
            can_close: m.can_close,
        })
}

/// What the backend has seen, shared with its forwarder thread.
#[derive(Default)]
struct Live {
    /// Bumped on every connect and disconnect; a forwarder of an older one stops.
    generation: u64,
    status: ConnectionStatus,
    /// The last identity the server named; kept while offline for the actor.
    identity: Option<String>,
    orgs: Vec<ClientOrganisation>,
    books: HashMap<String, Arc<LedgerState>>,
    subscribers: Vec<Sender<BackendEvent>>,
}

impl Live {
    /// Takes in one client event and returns what the screens must hear, in order.
    fn apply(&mut self, event: ClientEvent, client: &dyn LiveClient) -> Vec<BackendEvent> {
        let mut out = Vec::new();
        match event {
            ClientEvent::Status(status) => {
                if let Some(identity) = status.identity() {
                    self.identity = Some(identity.to_owned());
                }
                // The first rows are readable from `Ready` on (ledger-client `status.rs`).
                if matches!(status, ClientStatus::Ready { .. }) {
                    self.refresh(client, None, &mut out);
                }
                self.set_status(status_of(&status), &mut out);
            }
            ClientEvent::Changed { orgs } => self.refresh(client, Some(orgs), &mut out),
            ClientEvent::Error(message) => {
                self.set_status(ConnectionStatus::Error { message }, &mut out);
            }
        }
        out
    }

    fn set_status(&mut self, status: ConnectionStatus, out: &mut Vec<BackendEvent>) {
        if self.status != status {
            self.status = status.clone();
            out.push(BackendEvent::Status(status));
        }
    }

    /// Re-reads the organisation list, then the books of `changed` (every organisation when
    /// `None`). The list goes first so a screen's selection exists before its books arrive.
    /// Books that fail to rebuild keep their last snapshot.
    fn refresh(
        &mut self,
        client: &dyn LiveClient,
        changed: Option<BTreeSet<String>>,
        out: &mut Vec<BackendEvent>,
    ) {
        let orgs = client.organisations();
        if orgs != self.orgs {
            self.orgs = orgs;
            out.push(BackendEvent::Organisations(
                self.orgs.iter().map(organisation_of).collect(),
            ));
        }
        let ids = changed.unwrap_or_else(|| self.orgs.iter().map(|o| o.org_id.clone()).collect());
        for id in ids {
            if let Ok(state) = client.state(&id) {
                let state = Arc::new(state);
                self.books.insert(id.clone(), Arc::clone(&state));
                out.push(BackendEvent::Ledger {
                    org: OrganizationId::from(id),
                    state,
                });
            }
        }
    }

    fn broadcast(&mut self, events: &[BackendEvent]) {
        for event in events {
            self.subscribers
                .retain(|tx| tx.try_send(event.clone()).is_ok());
        }
    }
}

/// Locks `m`; the values behind it are replaced whole, so a poisoned lock still holds a sound one.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The forwarder loop: client events in, backend events out, until the client is gone, its
/// channel closes or a newer connection takes over.
fn forward(
    events: &Receiver<ClientEvent>,
    client: &Weak<dyn LiveClient>,
    live: &Mutex<Live>,
    generation: u64,
) {
    while let Ok(event) = events.recv_blocking() {
        let Some(client) = client.upgrade() else {
            return;
        };
        let mut live = lock(live);
        if live.generation != generation {
            return;
        }
        let out = live.apply(event, client.as_ref());
        live.broadcast(&out);
    }
}

/// The ledger on a SpacetimeDB server. Starts disconnected; [`LedgerBackend::connect`] opens
/// the connection.
pub struct SpacetimeBackend {
    live: Arc<Mutex<Live>>,
    client: Option<Arc<dyn LiveClient>>,
    settings: Option<ConnectSettings>,
}

impl Default for SpacetimeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SpacetimeBackend {
    /// A backend with no connection yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            live: Arc::new(Mutex::new(Live::default())),
            client: None,
            settings: None,
        }
    }

    /// A backend connecting to the server `settings` names.
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the connection cannot be started.
    pub fn connected(settings: ConnectSettings) -> Result<Self, BackendError> {
        let mut backend = Self::new();
        backend.connect(settings)?;
        Ok(backend)
    }

    /// The client config for `settings`: the token file is the settings' path, so a reconnect
    /// with the same path is the same identity (`ledger_client::TokenFile`); an empty path
    /// connects anonymously.
    #[must_use]
    pub fn client_config(settings: &ConnectSettings) -> ClientConfig {
        let config = ClientConfig::new(settings.host.clone(), settings.database.clone());
        if settings.token_path.is_empty() {
            config
        } else {
            config.with_token_file(TokenFile::new(&settings.token_path))
        }
    }

    /// The settings of the last [`LedgerBackend::connect`].
    #[must_use]
    pub fn settings(&self) -> Option<&ConnectSettings> {
        self.settings.as_ref()
    }

    /// Takes over `client`: from now on its events reach this backend's subscribers through a
    /// new forwarder thread. A client already attached is closed.
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the forwarder thread cannot start.
    pub fn attach(&mut self, client: Arc<dyn LiveClient>) -> Result<(), BackendError> {
        self.close();
        let generation = {
            let mut live = lock(&self.live);
            live.generation += 1;
            live.generation
        };
        let events = client.subscribe();
        let weak = Arc::downgrade(&client);
        let live = Arc::clone(&self.live);
        std::thread::Builder::new()
            .name("desktop-ledger-events".to_owned())
            .spawn(move || forward(&events, &weak, &live, generation))
            .map_err(|e| BackendError::Connection(e.to_string()))?;
        self.client = Some(client);
        Ok(())
    }

    /// Drops the client on a thread of its own: closing joins the client's thread, which must
    /// never hold up the UI.
    fn close(&mut self) {
        if let Some(client) = self.client.take() {
            let closed = std::thread::Builder::new()
                .name("desktop-ledger-close".to_owned())
                .spawn(move || client.disconnect());
            // Without a thread the client still closes when its last reference goes.
            drop(closed);
        }
    }
}

impl Drop for SpacetimeBackend {
    fn drop(&mut self) {
        self.close();
    }
}

impl LedgerBackend for SpacetimeBackend {
    fn organisations(&self) -> Vec<Organisation> {
        lock(&self.live).orgs.iter().map(organisation_of).collect()
    }

    fn state(&self, org: &OrganizationId) -> Option<Arc<LedgerState>> {
        lock(&self.live).books.get(org.as_str()).cloned()
    }

    fn subscribe(&mut self) -> Receiver<BackendEvent> {
        let (tx, rx) = async_channel::unbounded();
        lock(&self.live).subscribers.push(tx);
        rx
    }

    /// Blocks until the server answers; for worker threads and tests. The store calls
    /// [`LedgerBackend::send`].
    fn submit(
        &mut self,
        org: &OrganizationId,
        command: LedgerCommand,
    ) -> Result<CommandResult, BackendError> {
        let client = self.writable(org)?;
        client.submit(org.as_str(), &command).map_err(backend_error)
    }

    fn send(&mut self, org: &OrganizationId, command: LedgerCommand) -> Submission {
        match self.writable(org) {
            Ok(client) => client.send(org.as_str(), &command),
            Err(e) => Submission::ready(Err(e)),
        }
    }

    fn actor(&self, org: &OrganizationId) -> Option<Actor> {
        let live = lock(&self.live);
        let identity = live.identity.as_deref()?;
        live.orgs
            .iter()
            .find(|o| o.org_id == org.as_str())
            .and_then(|o| actor_for(o, identity))
    }

    fn status(&self) -> ConnectionStatus {
        lock(&self.live).status.clone()
    }

    fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError> {
        let client = SpacetimeClient::connect(Self::client_config(&settings))
            .map_err(|e| BackendError::Connection(e.to_string()))?;
        self.settings = Some(settings);
        self.attach(Arc::new(client))
    }

    fn disconnect(&mut self) {
        lock(&self.live).generation += 1;
        self.close();
        let mut live = lock(&self.live);
        let mut out = Vec::new();
        live.set_status(ConnectionStatus::Disconnected, &mut out);
        live.broadcast(&out);
    }

    fn is_remote(&self) -> bool {
        true
    }
}

impl SpacetimeBackend {
    /// The client, when writes may go out: connected and `org` one of the identity's.
    fn writable(&self, org: &OrganizationId) -> Result<Arc<dyn LiveClient>, BackendError> {
        let live = lock(&self.live);
        let client = match (&live.status, &self.client) {
            (ConnectionStatus::Connected { .. }, Some(client)) => Arc::clone(client),
            _ => return Err(BackendError::Offline),
        };
        if !live.orgs.iter().any(|o| o.org_id == org.as_str()) {
            return Err(BackendError::UnknownOrganisation(org.as_str().to_owned()));
        }
        Ok(client)
    }
}

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;
