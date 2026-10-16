//! [`SpacetimeClient`]: the ledger module over SpacetimeDB behind [`LedgerBackend`], the Rust
//! form of `spacetimeLedgerBackend` (`flowtrace/src/lib/ledger/spacetime-backend.ts`).
//!
//! One thread per client owns the connection. It drives the SDK with its own loop over
//! `advance_one_message_blocking` instead of `run_threaded`, which panics its thread on a
//! disconnect that is not clean (SDK 2.12 `db_connection.rs`, `run_threaded`; ADR 0002). The
//! SDK runs every callback inside that call on this thread; callbacks only send on an internal
//! channel. After each message the loop drains that channel: it applies the row changes to the
//! [`RowCache`] as one batch, sends one [`BackendEvent::Changed`] naming the organisations they
//! touched, then resolves the tickets whose result arrived, so a ticket never resolves before
//! the books it changed are readable. Status changes and errors go out on the same receivers.
//!
//! Connection and subscription API: <https://spacetimedb.com/docs/clients/rust> (connection
//! builder, `with_token`, `on_connect`, `subscription_builder`, `advance_one_message_blocking`).

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use ledger_core::types::LedgerState;
use ledger_core::{CommandResult, LedgerCommand};
use spacetimedb_sdk::__codegen::InternalError;
use spacetimedb_sdk::{DbContext as _, Table, TableWithPrimaryKey};

use crate::backend::{
    BackendEvent, CommandTicket, ConnectionStatus, LedgerBackend, Organisation, RESULT_TIMEOUT,
    Ticket, TicketSender,
};
use crate::cache::{Row, RowCache, RowChange};
use crate::error::{ClientError, decode_reducer_error, is_valid_org_id};
use crate::module_bindings::{
    self as b, DbConnection, MyAccountTableAccess as _, MyAuditEventTableAccess as _,
    MyBankAccountTableAccess as _, MyBillTableAccess as _, MyBudgetLineTableAccess as _,
    MyBudgetTableAccess as _, MyCommandResultTableAccess as _, MyDimensionTableAccess as _,
    MyExpenseClaimTableAccess as _, MyFiscalPeriodTableAccess as _, MyFundTableAccess as _,
    MyGrantTableAccess as _, MyJournalEntryTableAccess as _, MyJournalLineTableAccess as _,
    MyLedgerOrgTableAccess as _, MyMemberTableAccess as _, MyPaymentTableAccess as _,
    MyStatementLineTableAccess as _, MyVendorTableAccess as _, create_org_ledger as _,
    import_books as _, run_command as _,
};
use crate::token::TokenFile;

/// The views the client subscribes to: every `my_*` view, which hold the rows of every
/// organisation the caller is a member of (`spacetime-backend.ts:105-108`, `LEDGER_TABLES`, plus
/// `my_member` for the organisation list). The TS client narrows each query to one organisation;
/// the desktop shows all of them, so these do not.
pub const VIEWS: [&str; 19] = [
    "my_ledger_org",
    "my_member",
    "my_journal_entry",
    "my_journal_line",
    "my_account",
    "my_fund",
    "my_grant",
    "my_dimension",
    "my_fiscal_period",
    "my_budget",
    "my_budget_line",
    "my_vendor",
    "my_bill",
    "my_payment",
    "my_expense_claim",
    "my_bank_account",
    "my_statement_line",
    "my_audit_event",
    "my_command_result",
];

/// Where to connect.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// The host, `http://127.0.0.1:3100` or `wss://maincloud.spacetimedb.com`
    /// (`VITE_SPACETIMEDB_URI`, `spacetime-backend.ts:11`).
    pub uri: String,
    /// The database name (`VITE_SPACETIMEDB_MODULE`).
    pub database: String,
    /// Where the token is kept; `None` connects anonymously with a new identity every time.
    pub token_file: Option<TokenFile>,
    /// How long a call waits for its result ([`RESULT_TIMEOUT`] by default).
    pub result_timeout: Duration,
}

impl ClientConfig {
    /// A config for `database` on `uri` with no token file and the default timeout.
    #[must_use]
    pub fn new(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            token_file: None,
            result_timeout: RESULT_TIMEOUT,
        }
    }

    /// The same config, keeping the token in `file`.
    #[must_use]
    pub fn with_token_file(mut self, file: TokenFile) -> Self {
        self.token_file = Some(file);
        self
    }
}

/// What the SDK callbacks send to the loop.
#[expect(
    clippy::large_enum_variant,
    reason = "nearly every message is a row change; boxing it would allocate once more per row"
)]
enum Internal {
    Change(RowChange),
    Connected {
        identity: String,
        token: String,
    },
    Applied,
    SubscriptionError(String),
    Ended(Option<String>),
    CommandFailed {
        request_id: String,
        error: ClientError,
    },
    CallDone(TicketSender<()>, Result<(), ClientError>),
}

type InternalTx = mpsc::Sender<Internal>;

/// Status and receivers under one lock, so a new receiver's first status is never stale.
struct Hub {
    status: ConnectionStatus,
    subscribers: Vec<async_channel::Sender<BackendEvent>>,
}

impl Hub {
    fn broadcast(&mut self, event: &BackendEvent) {
        self.subscribers
            .retain(|tx| tx.try_send(event.clone()).is_ok());
    }

    fn set_status(&mut self, status: ConnectionStatus) {
        if self.status != status {
            self.status = status.clone();
            self.broadcast(&BackendEvent::Status(status));
        }
    }
}

struct Shared {
    hub: Mutex<Hub>,
    cache: Mutex<RowCache>,
    conn: Mutex<Option<Arc<DbConnection>>>,
    pending: Mutex<HashMap<String, TicketSender<CommandResult>>>,
    request_prefix: Mutex<String>,
    next_request: AtomicU64,
    timeout: Duration,
}

/// Locks `m`; every value behind these locks is replaced whole, so a poisoned lock still holds a
/// sound value.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The ledger module over a SpacetimeDB connection.
pub struct SpacetimeClient {
    shared: Arc<Shared>,
    internal: InternalTx,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl SpacetimeClient {
    /// Starts connecting on a new thread and returns at once; the status starts at
    /// [`ConnectionStatus::Connecting`] and moves on as [`BackendEvent::Status`] events.
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the token file exists but cannot be read, or
    /// [`ClientError::Sdk`] when the thread cannot start.
    pub fn connect(config: ClientConfig) -> Result<Self, ClientError> {
        let token = config
            .token_file
            .as_ref()
            .map(TokenFile::load)
            .transpose()?
            .flatten();
        let shared = Arc::new(Shared {
            hub: Mutex::new(Hub {
                status: ConnectionStatus::Connecting,
                subscribers: Vec::new(),
            }),
            cache: Mutex::new(RowCache::default()),
            conn: Mutex::new(None),
            pending: Mutex::new(HashMap::new()),
            request_prefix: Mutex::new(String::new()),
            next_request: AtomicU64::new(1),
            timeout: config.result_timeout,
        });
        let (tx, rx) = mpsc::channel();
        let thread = {
            let shared = Arc::clone(&shared);
            let tx = tx.clone();
            std::thread::Builder::new()
                .name("ledger-client".to_owned())
                .spawn(move || run(&shared, &config, token, &tx, &rx))
                .map_err(|e| ClientError::Sdk(e.to_string()))?
        };
        Ok(Self {
            shared,
            internal: tx,
            thread: Mutex::new(Some(thread)),
        })
    }

    /// The caller's identity (hex), once the host has sent it.
    #[must_use]
    pub fn identity(&self) -> Option<String> {
        lock(&self.shared.hub).status.identity().map(str::to_owned)
    }

    /// Closes the connection and waits for the client thread to end. Calls still pending fail
    /// with [`ClientError::Disconnected`].
    pub fn disconnect(&self) {
        if let Some(conn) = lock(&self.shared.conn).clone() {
            // Already closed is fine: the loop is ending anyway.
            let _ = conn.disconnect();
        }
        if let Some(thread) = lock(&self.thread).take() {
            // A panic on the client thread already ended it; nothing more to clean up.
            let _ = thread.join();
        }
    }

    fn connection(&self) -> Result<Arc<DbConnection>, ClientError> {
        lock(&self.shared.conn)
            .clone()
            .ok_or(ClientError::Disconnected)
    }

    /// Opens books for a new organisation; the caller must be an operator and becomes its owner
    /// (`createSpacetimeOrgLedger`, `spacetime-backend.ts:268-277`; `create_org_ledger`,
    /// `crates/ledger-module/README.md`).
    pub fn create_org_ledger(
        &self,
        org_id: &str,
        base_currency: &str,
        fiscal_year: u32,
        display_name: &str,
    ) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.create_org_ledger_then(
                org_id.to_owned(),
                base_currency.to_owned(),
                fiscal_year,
                display_name.to_owned(),
                done,
            )
        })
    }

    /// Replaces an organisation's books with `state`; operator only, the books must exist and
    /// name `org_id` (`import_books`, `crates/ledger-module/README.md`).
    pub fn import_books(&self, org_id: &str, state: &LedgerState) -> Ticket<()> {
        let json = match serde_json::to_string(state) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .import_books_then(org_id.to_owned(), json, done)
        })
    }

    /// Sends one reducer call whose ticket resolves with the reducer's outcome.
    fn call(
        &self,
        send: impl FnOnce(&DbConnection, ReducerCallback) -> spacetimedb_sdk::Result<()>,
    ) -> Ticket<()> {
        let conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => return Ticket::ready(Err(e)),
        };
        let (done, ticket) = Ticket::pending();
        let internal = self.internal.clone();
        let reply = done.clone();
        let callback: ReducerCallback = Box::new(move |_, outcome| {
            // The loop is gone only when the client is; the ticket then reads as disconnected.
            let _ = internal.send(Internal::CallDone(reply, outcome_of(outcome)));
        });
        if let Err(e) = send(&conn, callback) {
            return Ticket::ready(Err(ClientError::Sdk(e.to_string())));
        }
        expire(self.shared.timeout, move || {
            let _ = done.try_send(Err(ClientError::Timeout));
        });
        ticket
    }
}

/// What a reducer call reports back (the generated `*_then` callbacks).
type ReducerCallback = Box<
    dyn FnOnce(&b::ReducerEventContext, Result<Result<(), String>, InternalError>) + Send + 'static,
>;

/// A reducer outcome as the caller sees it.
fn outcome_of(outcome: Result<Result<(), String>, InternalError>) -> Result<(), ClientError> {
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(text)) => Err(decode_reducer_error(&text)),
        Err(e) => Err(ClientError::Sdk(e.to_string())),
    }
}

/// Runs `fire` after `after` on a short-lived thread: the result timeout of one call
/// (`spacetime-backend.ts:180-183`). Calls are human-paced, so a sleeping thread each is cheap.
fn expire(after: Duration, fire: impl FnOnce() + Send + 'static) {
    // A thread that cannot start leaves the call without a timeout, which only matters if the
    // host never answers.
    let _ = std::thread::Builder::new()
        .name("ledger-client-timeout".to_owned())
        .spawn(move || {
            std::thread::sleep(after);
            fire();
        });
}

impl LedgerBackend for SpacetimeClient {
    fn status(&self) -> ConnectionStatus {
        lock(&self.shared.hub).status.clone()
    }

    fn organisations(&self) -> Vec<Organisation> {
        lock(&self.shared.cache).organisations()
    }

    fn state(&self, org_id: &str) -> Result<LedgerState, ClientError> {
        if !is_valid_org_id(org_id) {
            return Err(ClientError::InvalidOrgId(org_id.to_owned()));
        }
        lock(&self.shared.cache).state(org_id)
    }

    fn subscribe(&self) -> async_channel::Receiver<BackendEvent> {
        let (tx, rx) = async_channel::unbounded();
        let mut hub = lock(&self.shared.hub);
        // An unbounded channel whose receiver is in hand takes the first event.
        let _ = tx.try_send(BackendEvent::Status(hub.status.clone()));
        hub.subscribers.push(tx);
        rx
    }

    /// `run` (`spacetime-backend.ts:199-217`): the command as JSON through `run_command` under a
    /// fresh request id; resolves with the `command_result` row of that id, or the reducer's
    /// error decoded, or [`ClientError::Timeout`].
    fn submit(&self, org_id: &str, command: LedgerCommand) -> CommandTicket {
        if !is_valid_org_id(org_id) {
            return Ticket::ready(Err(ClientError::InvalidOrgId(org_id.to_owned())));
        }
        let json = match serde_json::to_string(&command) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        let conn = match self.connection() {
            Ok(conn) => conn,
            Err(e) => return Ticket::ready(Err(e)),
        };
        let request_id = format!(
            "{}-{}",
            lock(&self.shared.request_prefix),
            self.shared.next_request.fetch_add(1, Ordering::Relaxed)
        );
        let (tx, ticket) = Ticket::pending();
        lock(&self.shared.pending).insert(request_id.clone(), tx);
        let internal = self.internal.clone();
        let id = request_id.clone();
        let sent = conn.reducers.run_command_then(
            org_id.to_owned(),
            request_id.clone(),
            json,
            move |_, outcome| {
                // Success resolves through the result row; only a failure needs the loop here.
                if let Err(error) = outcome_of(outcome) {
                    let _ = internal.send(Internal::CommandFailed {
                        request_id: id,
                        error,
                    });
                }
            },
        );
        if let Err(e) = sent {
            lock(&self.shared.pending).remove(&request_id);
            return Ticket::ready(Err(ClientError::Sdk(e.to_string())));
        }
        let shared = Arc::clone(&self.shared);
        expire(self.shared.timeout, move || {
            if let Some(tx) = lock(&shared.pending).remove(&request_id) {
                let _ = tx.try_send(Err(ClientError::Timeout));
            }
        });
        ticket
    }
}

impl Drop for SpacetimeClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Registers insert, update and delete callbacks that send the row as a [`RowChange`].
fn watch<H>(handle: &H, tx: &InternalTx)
where
    H: TableWithPrimaryKey<EventContext = b::EventContext>,
    H::Row: Clone + Into<Row> + Send + 'static,
{
    watch_without_key(handle, tx);
    let t = tx.clone();
    handle.on_update(move |_, _, new| {
        let _ = t.send(Internal::Change(RowChange::Upsert(new.clone().into())));
    });
}

/// Insert and delete callbacks only, for a view without a primary key (`my_member`, a
/// procedural view): an update arrives as a delete and an insert.
fn watch_without_key<H>(handle: &H, tx: &InternalTx)
where
    H: Table<EventContext = b::EventContext>,
    H::Row: Clone + Into<Row> + Send + 'static,
{
    let t = tx.clone();
    handle.on_insert(move |_, row| {
        let _ = t.send(Internal::Change(RowChange::Upsert(row.clone().into())));
    });
    let t = tx.clone();
    handle.on_delete(move |_, row| {
        let _ = t.send(Internal::Change(RowChange::Delete(row.clone().into())));
    });
}

fn watch_all(conn: &DbConnection, tx: &InternalTx) {
    let db = &conn.db;
    watch(&db.my_ledger_org(), tx);
    watch_without_key(&db.my_member(), tx);
    watch(&db.my_journal_entry(), tx);
    watch(&db.my_journal_line(), tx);
    watch(&db.my_account(), tx);
    watch(&db.my_fund(), tx);
    watch(&db.my_grant(), tx);
    watch(&db.my_dimension(), tx);
    watch(&db.my_fiscal_period(), tx);
    watch(&db.my_budget(), tx);
    watch(&db.my_budget_line(), tx);
    watch(&db.my_vendor(), tx);
    watch(&db.my_bill(), tx);
    watch(&db.my_payment(), tx);
    watch(&db.my_expense_claim(), tx);
    watch(&db.my_bank_account(), tx);
    watch(&db.my_statement_line(), tx);
    watch(&db.my_audit_event(), tx);
    watch(&db.my_command_result(), tx);
}

fn subscribe_all(conn: &DbConnection, tx: &InternalTx) {
    let applied = tx.clone();
    let failed = tx.clone();
    let queries: Vec<String> = VIEWS.iter().map(|v| format!("SELECT * FROM {v}")).collect();
    conn.subscription_builder()
        .on_applied(move |_| {
            let _ = applied.send(Internal::Applied);
        })
        .on_error(move |_, e| {
            let _ = failed.send(Internal::SubscriptionError(e.to_string()));
        })
        .subscribe(queries);
}

/// The client thread: connect, subscribe, then one message at a time until the connection ends.
fn run(
    shared: &Arc<Shared>,
    config: &ClientConfig,
    token: Option<String>,
    tx: &InternalTx,
    rx: &mpsc::Receiver<Internal>,
) {
    let on_connect = tx.clone();
    let on_connect_error = tx.clone();
    let on_disconnect = tx.clone();
    let built = DbConnection::builder()
        .with_uri(config.uri.as_str())
        .with_database_name(config.database.as_str())
        .with_token(token)
        .on_connect(move |conn, identity, token| {
            subscribe_all(conn, &on_connect);
            let _ = on_connect.send(Internal::Connected {
                identity: identity.to_hex().to_string(),
                token: token.to_owned(),
            });
        })
        .on_connect_error(move |_, e| {
            let _ = on_connect_error.send(Internal::Ended(Some(e.to_string())));
        })
        .on_disconnect(move |_, e| {
            let _ = on_disconnect.send(Internal::Ended(e.map(|e| e.to_string())));
        })
        .build();
    let conn = match built {
        Ok(conn) => Arc::new(conn),
        Err(e) => {
            end(shared, Some(e.to_string()));
            return;
        }
    };
    watch_all(&conn, tx);
    *lock(&shared.conn) = Some(Arc::clone(&conn));

    let ended = loop {
        let step = conn.advance_one_message_blocking();
        let ended = drain(shared, config, &conn, rx);
        match (ended, step) {
            (Some(Ended(error)), _) => break error,
            (None, Ok(())) => {}
            (None, Err(spacetimedb_sdk::Error::Disconnected)) => break None,
            (None, Err(e)) => break Some(e.to_string()),
        }
    };
    end(shared, ended);
}

/// The connection ended: `None` for a clean close, else what went wrong.
struct Ended(Option<String>);

/// Applies everything the callbacks sent for the last message. Returns `Some` when the
/// connection ended.
fn drain(
    shared: &Shared,
    config: &ClientConfig,
    conn: &DbConnection,
    rx: &mpsc::Receiver<Internal>,
) -> Option<Ended> {
    let mut changes = Vec::new();
    let mut rest = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            Internal::Change(c) => changes.push(c),
            other => rest.push(other),
        }
    }

    // Rows first, so every ticket and status below sees the books they belong to.
    let mut results = Vec::new();
    let touched: BTreeSet<String> = {
        let mut cache = lock(&shared.cache);
        changes
            .into_iter()
            .filter_map(|c| {
                if let RowChange::Upsert(Row::Result(r)) = &c {
                    results.push(r.clone());
                }
                cache.apply(c)
            })
            .collect()
    };
    if !touched.is_empty() {
        lock(&shared.hub).broadcast(&BackendEvent::Changed { orgs: touched });
    }
    for r in results {
        if let Some(tx) = lock(&shared.pending).remove(&r.request_id) {
            // `JSON.parse(row.resultJson)` (`spacetime-backend.ts:173`).
            let result = serde_json::from_str::<CommandResult>(&r.result_json).map_err(Into::into);
            let _ = tx.try_send(result);
        }
    }

    let mut ended = None;
    for msg in rest {
        match msg {
            Internal::Change(_) => {}
            Internal::Connected { identity, token } => {
                *lock(&shared.request_prefix) = conn
                    .try_connection_id()
                    .map_or_else(|| identity.clone(), |c| c.to_hex().to_string());
                if let Some(file) = &config.token_file
                    && let Err(e) = file.save(&token)
                {
                    lock(&shared.hub).broadcast(&BackendEvent::Error(e.to_string()));
                }
                lock(&shared.hub).set_status(ConnectionStatus::Connected { identity });
            }
            Internal::Applied => {
                let mut hub = lock(&shared.hub);
                if let Some(identity) = hub.status.identity().map(str::to_owned) {
                    hub.set_status(ConnectionStatus::Ready { identity });
                }
            }
            Internal::SubscriptionError(e) => {
                lock(&shared.hub).broadcast(&BackendEvent::Error(e));
            }
            Internal::CommandFailed { request_id, error } => {
                if let Some(tx) = lock(&shared.pending).remove(&request_id) {
                    let _ = tx.try_send(Err(error));
                }
            }
            Internal::CallDone(tx, outcome) => {
                let _ = tx.try_send(outcome);
            }
            Internal::Ended(error) => ended = Some(Ended(error)),
        }
    }
    ended
}

/// The connection is over: no more sends, every pending command fails, status says why.
fn end(shared: &Shared, error: Option<String>) {
    *lock(&shared.conn) = None;
    for (_, tx) in lock(&shared.pending).drain() {
        let _ = tx.try_send(Err(ClientError::Disconnected));
    }
    let mut hub = lock(&shared.hub);
    if let Some(e) = &error {
        hub.broadcast(&BackendEvent::Error(e.clone()));
    }
    hub.set_status(ConnectionStatus::Disconnected { error });
}
