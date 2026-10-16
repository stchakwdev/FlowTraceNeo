//! The one store entity every screen reads (`rust/docs/adr/0002-gpui-spacetime.md`, "Pattern
//! for U1 to U10"): it owns the backend and the [`LedgerView`], and a task drains the backend's
//! change channel in batches, applying each batch in one update with one `cx.notify()`.

use gpui_kit::{Context, Task};
use ledger_core::types::OrganizationId;
use ledger_core::{CommandResult, LedgerCommand};

use crate::backend::{BackendError, ConnectSettings, LedgerBackend};
use crate::model::LedgerView;

/// The backend and what the shell knows of it. Screens `cx.observe` this entity and re-derive
/// their rows from [`LedgerStore::view`] on each notification.
pub struct LedgerStore {
    backend: Box<dyn LedgerBackend>,
    view: LedgerView,
    _pump: Task<()>,
}

impl LedgerStore {
    /// Opens the store on `backend` with its first organisation selected.
    pub fn new(mut backend: Box<dyn LedgerBackend>, cx: &mut Context<Self>) -> Self {
        let rx = backend.subscribe();
        let orgs = backend.organisations();
        let snapshot = orgs.first().and_then(|org| backend.state(&org.id));
        let view = LedgerView::new(orgs, backend.status(), snapshot);

        // Wake on the first event, take whatever else is queued, apply it as one batch.
        let pump = cx.spawn(async move |this, cx| {
            while let Ok(first) = rx.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = rx.try_recv() {
                    batch.push(more);
                }
                let applied = this.update(cx, |store, cx| {
                    if store.apply(batch) {
                        cx.notify();
                    }
                });
                if applied.is_err() {
                    break;
                }
            }
        });

        Self {
            backend,
            view,
            _pump: pump,
        }
    }

    /// What the shell and the screens render from.
    #[must_use]
    pub fn view(&self) -> &LedgerView {
        &self.view
    }

    /// Who the app acts as in the selected organisation ([`LedgerBackend::actor`]).
    #[must_use]
    pub fn actor(&self) -> Option<ledger_core::types::Actor> {
        None
    }

    /// Applies a drained batch; when the selected organisation changed under it, loads that
    /// organisation's books.
    fn apply(&mut self, batch: Vec<crate::backend::BackendEvent>) -> bool {
        let mut changed = self.view.apply(batch);
        if self.view.snapshot().is_none()
            && let Some(org) = self.view.selected().map(|org| org.id.clone())
            && let Some(state) = self.backend.state(&org)
        {
            changed |= self.view.select(&org, Some(state));
        }
        changed
    }

    /// Selects `org` and its books.
    pub fn select_org(&mut self, org: &OrganizationId, cx: &mut Context<Self>) {
        let snapshot = self.backend.state(org);
        if self.view.select(org, snapshot) {
            cx.notify();
        }
    }

    /// Submits `command` to the selected organisation's books. The new books come back through
    /// the change channel, never by patching rows here (ADR 0002).
    ///
    /// # Errors
    ///
    /// [`BackendError`] from the backend; [`BackendError::UnknownOrganisation`] when none is
    /// selected.
    pub fn submit(&mut self, command: LedgerCommand) -> Result<CommandResult, BackendError> {
        let org = self
            .view
            .selected()
            .map(|org| org.id.clone())
            .ok_or_else(|| BackendError::UnknownOrganisation(String::new()))?;
        self.backend.submit(&org, command)
    }

    /// Asks the backend to connect; status changes come back through the change channel.
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the backend cannot start connecting.
    pub fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError> {
        self.backend.connect(settings)
    }
}
