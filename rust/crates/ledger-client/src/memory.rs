//! [`InMemoryBackend`]: one organisation's books in memory behind [`LedgerBackend`], the Rust
//! form of `demoLedgerBackend` (`flowtrace/src/lib/demo/ledger.ts:395-430`), for the desktop's
//! screens before U10 plugs in [`crate::SpacetimeClient`].
//!
//! A command takes the module's own path (`ledger_module::command`, `index.ts:663-690`): the
//! typed command goes to JSON, back through `sanitize_command`, then `apply_command` on a copy
//! of the books that replaces them only when the engine accepts. A refusal comes back as the text
//! the reducer would return and is decoded as the live client decodes it, so a screen sees the
//! same [`ClientError`] from either backend.

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use ledger_core::env::fixed;
use ledger_core::types::{Actor, LedgerState};
use ledger_core::{CommandResult, LedgerCommand, LedgerEnv};
use ledger_module::command;

use crate::backend::{
    BackendEvent, CommandTicket, ConnectionStatus, LedgerBackend, MemberInfo, Organisation,
};
use crate::error::{ClientError, decode_reducer_error};

/// The identity the in-memory backend reports: 64 zeros, since nobody connected.
pub const IN_MEMORY_IDENTITY: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

struct Inner {
    state: LedgerState,
    env: Box<dyn LedgerEnv + Send>,
    actor: Actor,
    subscribers: Vec<async_channel::Sender<BackendEvent>>,
}

/// One set of books in memory. Every command runs as [`InMemoryBackend::actor`], who is the only
/// member the backend reports.
pub struct InMemoryBackend {
    inner: Mutex<Inner>,
}

impl InMemoryBackend {
    /// Books `state`, commands run as `actor`, ids and clock from `env`.
    #[must_use]
    pub fn new(state: LedgerState, actor: Actor, env: impl LedgerEnv + Send + 'static) -> Self {
        Self {
            inner: Mutex::new(Inner {
                state,
                env: Box::new(env),
                actor,
                subscribers: Vec::new(),
            }),
        }
    }

    /// The demo organisation's books (`ledger_core::demo::build_demo_ledger`, equal to
    /// `rust/fixtures/demo-books.json#/state`) as the demo user, with the commands env
    /// [`fixed::fixture`] (`fx-0001`, ..., clock `DEMO_NOW`). The demo env [`fixed::demo`] would
    /// hand out `demo-0001` again, which the demo books already use; the commands env is the one
    /// `commands-accepted.json` was recorded with on these books.
    ///
    /// # Errors
    ///
    /// The demo build failing, which its own tests rule out.
    pub fn demo() -> Result<Self, ledger_core::engine::Error> {
        Ok(Self::new(
            ledger_core::demo::build_demo_ledger()?,
            ledger_core::demo::demo_actor(),
            fixed::fixture(),
        ))
    }

    /// Who commands run as.
    #[must_use]
    pub fn actor(&self) -> Actor {
        self.lock().actor.clone()
    }

    /// Runs the next commands as `actor`, as another member would on the live module.
    pub fn set_actor(&self, actor: Actor) {
        self.lock().actor = actor;
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // The books are replaced whole or not at all, so a panic elsewhere leaves them sound.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn run(inner: &mut Inner, command: &LedgerCommand) -> Result<CommandResult, ClientError> {
        let json = serde_json::to_string(command)?;
        let cmd = command::parse_command(&json).map_err(|t| decode_reducer_error(&t))?;
        let mut after = inner.state.clone();
        let actor = inner.actor.clone();
        let result = command::apply(&mut after, inner.env.as_mut(), &actor, cmd)
            .map_err(|t| decode_reducer_error(&t))?;
        if after != inner.state {
            inner.state = after;
            let org_id = inner.state.settings.organization_id.to_string();
            let event = BackendEvent::Changed {
                orgs: BTreeSet::from([org_id]),
            };
            inner
                .subscribers
                .retain(|tx| tx.try_send(event.clone()).is_ok());
        }
        Ok(result)
    }
}

impl LedgerBackend for InMemoryBackend {
    fn status(&self) -> ConnectionStatus {
        ConnectionStatus::Ready {
            identity: IN_MEMORY_IDENTITY.to_owned(),
        }
    }

    fn organisations(&self) -> Vec<Organisation> {
        let inner = self.lock();
        vec![Organisation {
            org_id: inner.state.settings.organization_id.to_string(),
            settings: inner.state.settings.clone(),
            members: vec![MemberInfo {
                identity: IN_MEMORY_IDENTITY.to_owned(),
                user_id: inner.actor.id.to_string(),
                name: inner.actor.name.clone(),
                can_post: inner.actor.can_post,
                can_approve: inner.actor.can_approve,
                can_close: inner.actor.can_close,
            }],
        }]
    }

    fn state(&self, org_id: &str) -> Result<LedgerState, ClientError> {
        let inner = self.lock();
        if inner.state.settings.organization_id.as_str() == org_id {
            Ok(inner.state.clone())
        } else {
            Err(ClientError::NoLedger(org_id.to_owned()))
        }
    }

    fn subscribe(&self) -> async_channel::Receiver<BackendEvent> {
        let (tx, rx) = async_channel::unbounded();
        // An unbounded channel whose receiver is in hand takes the first event.
        let _ = tx.try_send(BackendEvent::Status(self.status()));
        self.lock().subscribers.push(tx);
        rx
    }

    fn submit(&self, org_id: &str, command: LedgerCommand) -> CommandTicket {
        let mut inner = self.lock();
        if inner.state.settings.organization_id.as_str() != org_id {
            return CommandTicket::ready(Err(ClientError::NoLedger(org_id.to_owned())));
        }
        CommandTicket::ready(Self::run(&mut inner, &command))
    }
}
