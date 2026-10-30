//! The one store entity every screen reads (`rust/docs/adr/0002-gpui-spacetime.md`, "Pattern
//! for U1 to U10"): it owns the backend and the [`LedgerView`], and a task drains the backend's
//! change channel in batches, applying each batch in one update with one `cx.notify()`.
//!
//! The state transitions live in [`StoreCore`], plain Rust with no GPUI types, so they are
//! tested without a window; [`LedgerStore`] only adds the draining task.

use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::{Context, Task};
use ledger_core::LedgerCommand;
use ledger_core::types::{Actor, OrganizationId};

use ledger_module::transparency::chains::ChainActivity;
use ledger_module::transparency::public::PublicOrganization;
use ledger_module::transparency::types::{AccessType, NewVerification, Verification};

use crate::backend::{
    BackendError, BackendEvent, CampaignWrite, ChainWrite, ConnectSettings, FlowWrite,
    LedgerBackend, Submission,
};
use crate::model::LedgerView;
use crate::proofs::{LinkKind, ProofsClient, ProofsError};
use crate::signin::state::{Card, SignInState};

/// Makes the backend for a connect from the Connect screen: in the app, a
/// [`crate::backend::SpacetimeBackend`] on those settings.
pub type Connector =
    Box<dyn FnMut(ConnectSettings) -> Result<Box<dyn LedgerBackend>, BackendError>>;

/// The store's state and transitions: the backend, what the screens see of it, and how to
/// swap in a backend for a new connection.
pub struct StoreCore {
    backend: Box<dyn LedgerBackend>,
    view: LedgerView,
    connector: Connector,
    timings: Option<Timings>,
    /// The proofs Worker from `config.toml` (`proofs_url`).
    proofs_url: Option<String>,
    signin: SignInState,
}

/// Start-up and round-trip times written to stderr with `--timings on`.
struct Timings {
    started: Instant,
    books_seen: bool,
    submitted: Option<Instant>,
    /// The last sign-in line written, so each change is written once.
    signin: String,
}

impl StoreCore {
    /// The store over `backend` with its first organisation selected, and the receiver of its
    /// changes. `connector` makes the backend of a later [`StoreCore::connect`].
    #[must_use]
    pub fn new(
        backend: Box<dyn LedgerBackend>,
        connector: Connector,
    ) -> (Self, Receiver<BackendEvent>) {
        let mut core = Self {
            backend,
            view: LedgerView::default(),
            connector,
            timings: None,
            proofs_url: None,
            signin: SignInState::new(false),
        };
        let rx = core.open();
        (core, rx)
    }

    /// Re-reads everything from the backend and subscribes to it.
    fn open(&mut self) -> Receiver<BackendEvent> {
        let rx = self.backend.subscribe();
        let orgs = self.backend.organisations();
        let snapshot = orgs.first().and_then(|org| self.backend.state(&org.id));
        self.view = LedgerView::new(orgs, self.backend.status(), snapshot);
        self.view.set_remote(self.backend.is_remote());
        self.reload_transparency();
        self.view.set_chains(self.backend.chains());
        rx
    }

    /// Reads the selected organisation's transparency records again. Returns whether they
    /// changed.
    fn reload_transparency(&mut self) -> bool {
        let records = self
            .view
            .selected()
            .and_then(|org| self.backend.transparency(&org.id));
        self.view.set_transparency(records)
    }

    /// Writes start-up and round-trip times to stderr from now on.
    pub fn log_timings(&mut self, started: Instant) {
        self.timings = Some(Timings {
            started,
            books_seen: false,
            submitted: None,
            signin: String::new(),
        });
        eprintln!("desktop: status {:?}", self.view.status());
        self.note_books();
        self.note_signin();
    }

    /// What the shell and the screens render from.
    #[must_use]
    pub fn view(&self) -> &LedgerView {
        &self.view
    }

    /// Who the app acts as in the selected organisation ([`LedgerBackend::actor`]).
    #[must_use]
    pub fn actor(&self) -> Option<Actor> {
        self.view
            .selected()
            .and_then(|org| self.backend.actor(&org.id))
    }

    /// The WorkOS sign-in as the Connect screen shows it.
    #[must_use]
    pub fn signin(&self) -> &SignInState {
        &self.signin
    }

    /// The sign-in, to change; the store's sign-in steps drive it.
    pub fn signin_mut(&mut self) -> &mut SignInState {
        &mut self.signin
    }

    /// Applies a drained batch; when the selected organisation changed under it, loads that
    /// organisation's books. Returns whether anything shown changed.
    pub fn apply(&mut self, batch: Vec<BackendEvent>) -> bool {
        let selected = self.view.selected().map(|org| org.id.clone());
        let touched_selected = batch.iter().any(|event| {
            matches!(event, BackendEvent::Ledger { org, .. } if Some(org) == selected.as_ref())
        });
        if self.timings.is_some() {
            // The identity is public (`spacetime sql` shows it in `member`); the token never is.
            for event in &batch {
                match event {
                    BackendEvent::Status(status) => eprintln!("desktop: status {status:?}"),
                    BackendEvent::Organisations(orgs) => {
                        let names: Vec<&str> = orgs.iter().map(|o| o.name.as_str()).collect();
                        eprintln!("desktop: organisations {names:?}");
                    }
                    BackendEvent::Ledger { .. }
                    | BackendEvent::Transparency { .. }
                    | BackendEvent::Chains => {}
                }
            }
        }
        for event in &batch {
            if let BackendEvent::Status(status) = event {
                self.signin.on_status(status);
            }
        }
        self.note_signin();
        let touched_records = batch.iter().any(|event| {
            matches!(event, BackendEvent::Transparency { org } if Some(org) == selected.as_ref())
        });
        let touched_chains = batch
            .iter()
            .any(|event| matches!(event, BackendEvent::Chains));
        let mut changed = self.view.apply(batch);
        if self.view.snapshot().is_none()
            && let Some(org) = self.view.selected().map(|org| org.id.clone())
            && let Some(state) = self.backend.state(&org)
        {
            changed |= self.view.select(&org, Some(state));
        }
        let now_selected = self.view.selected().map(|org| org.id.clone());
        if touched_records || now_selected != selected {
            changed |= self.reload_transparency();
        }
        if touched_chains {
            changed |= self.view.set_chains(self.backend.chains());
        }
        if let Some(t) = &mut self.timings
            && touched_selected
            && let Some(at) = t.submitted.take()
        {
            eprintln!(
                "desktop: change visible {} ms after submit",
                at.elapsed().as_millis()
            );
        }
        self.note_books();
        changed
    }

    /// With `--timings on`, writes the sign-in section's text to stderr when it changes: the
    /// phase, the person, the identity, a failure or the module's refusal. Never a token.
    pub fn note_signin(&mut self) {
        let Some(t) = &mut self.timings else {
            return;
        };
        let card = Card::of(&self.signin);
        let mut parts = vec![format!(
            "sign-in {:?}: {}",
            self.signin.phase(),
            card.status
        )];
        parts.extend(
            card.details
                .iter()
                .map(|(label, value)| format!("{label}: {value}")),
        );
        parts.extend(card.refusal.iter().map(|r| format!("refused: {r}")));
        parts.extend(card.error.iter().map(|e| format!("error: {e}")));
        let line = parts.join(" | ");
        if line != t.signin {
            eprintln!("desktop: {line}");
            t.signin = line;
        }
    }

    fn note_books(&mut self) {
        if let Some(t) = &mut self.timings
            && !t.books_seen
            && self.view.snapshot().is_some()
        {
            t.books_seen = true;
            eprintln!(
                "desktop: books visible {} ms after launch",
                t.started.elapsed().as_millis()
            );
        }
    }

    /// Sets the proofs Worker (`config.toml`, `proofs_url`); blank means none.
    pub fn set_proofs_url(&mut self, url: Option<String>) {
        self.proofs_url = url.map(|u| u.trim().to_owned()).filter(|u| !u.is_empty());
    }

    /// A client of the proofs Worker for the selected organisation, with the organisation id to
    /// upload under. Signed in with WorkOS, it sends the session's access token, the refreshed
    /// one once the session renews (`signin::state::SignInState::refreshed`), so the Worker sees
    /// the same caller the module does; otherwise the connection's token
    /// ([`LedgerBackend::proofs_token`]).
    ///
    /// # Errors
    ///
    /// [`ProofsError::Unavailable`] with what to do: set `proofs_url`, or connect to a server.
    pub fn proofs_client(&self) -> Result<(ProofsClient, OrganizationId), ProofsError> {
        let url = self.proofs_url.as_deref().ok_or_else(|| {
            ProofsError::Unavailable(
                "No proof store is set up. Add proofs_url to config.toml (see the README)."
                    .to_owned(),
            )
        })?;
        let session = (self.signin.phase() == crate::signin::state::Phase::SignedIn)
            .then(|| self.signin.session())
            .flatten()
            .filter(|_| self.backend.is_remote())
            .map(|s| s.access_token.expose().to_owned());
        let token = session
            .or_else(|| self.backend.proofs_token())
            .ok_or_else(|| ProofsError::Unavailable(crate::backend::NO_PROOFS.to_owned()))?;
        let org = self
            .view
            .selected()
            .map(|o| o.id.clone())
            .ok_or_else(|| ProofsError::Unavailable("No organisation is open.".to_owned()))?;
        Ok((ProofsClient::new(url, token), org))
    }

    /// Records an uploaded proof on the selected organisation's books
    /// ([`LedgerBackend::attach_proof`]).
    pub fn attach_proof(&mut self, proof: NewVerification) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        self.backend.attach_proof(&org, proof)
    }

    /// Publishes a proof of the selected organisation at the public link, or takes it back
    /// ([`LedgerBackend::mark_published`]).
    pub fn mark_published(&mut self, verification_id: &str, published: bool) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        self.backend
            .mark_published(&org, verification_id, published)
    }

    /// Logs that the caller opened or downloaded a proof's file
    /// ([`LedgerBackend::log_verification_access`]).
    pub fn log_verification_access(
        &mut self,
        verification_id: &str,
        access: AccessType,
    ) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        self.backend
            .log_verification_access(&org, verification_id, access)
    }

    /// The selected organisation's proofs of one claim, bill or transaction that the caller may
    /// see, in the backend's order.
    #[must_use]
    pub fn proofs_of(&self, kind: LinkKind, id: &str) -> Vec<Verification> {
        let Some(org) = self.view.selected() else {
            return Vec::new();
        };
        self.backend
            .proofs(&org.id)
            .into_iter()
            .filter(|v| match kind {
                LinkKind::Claim => v.claim_id.as_deref() == Some(id),
                LinkKind::Bill => v.bill_id.as_deref() == Some(id),
                LinkKind::Transaction => v.transaction_id.as_deref() == Some(id),
            })
            .collect()
    }

    /// Selects `org` and its books. Returns whether the selection changed.
    pub fn select_org(&mut self, org: &OrganizationId) -> bool {
        let snapshot = self.backend.state(org);
        let selected = self.view.select(org, snapshot);
        if selected {
            self.reload_transparency();
        }
        selected
    }

    /// Sends a campaign or donation write to the selected organisation
    /// ([`LedgerBackend::write_campaign`]); the new records come back through the change
    /// channel. With no organisation selected the submission is a ready
    /// [`BackendError::UnknownOrganisation`].
    pub fn write_campaign(&mut self, write: CampaignWrite) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        self.backend.write_campaign(&org, write)
    }

    /// Sends a flow map write to the selected organisation ([`LedgerBackend::write_flow`]); the
    /// change comes back through the change channel.
    pub fn write_flow(&mut self, write: FlowWrite) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        self.backend.write_flow(&org, write)
    }

    /// Sends a chain write ([`LedgerBackend::write_chain`]); the change comes back through the
    /// change channel.
    pub fn write_chain(&mut self, write: ChainWrite) -> Submission {
        self.backend.write_chain(write)
    }

    /// The organisations a chain can be requested of ([`LedgerBackend::public_organizations`]).
    #[must_use]
    pub fn public_organizations(&self) -> Vec<PublicOrganization> {
        self.backend.public_organizations()
    }

    /// One chain's log, newest first ([`LedgerBackend::chain_activity`]).
    #[must_use]
    pub fn chain_activity(&self, chain_id: &str) -> Vec<ChainActivity> {
        self.backend.chain_activity(chain_id)
    }

    /// Sends `command` to the selected organisation's books. The new books come back through
    /// the change channel, never by patching rows here (ADR 0002). With no organisation
    /// selected the submission is a ready [`BackendError::UnknownOrganisation`].
    pub fn submit(&mut self, command: LedgerCommand) -> Submission {
        let Some(org) = self.view.selected().map(|org| org.id.clone()) else {
            return Submission::ready(Err(BackendError::UnknownOrganisation(String::new())));
        };
        if let Some(t) = &mut self.timings {
            t.submitted = Some(Instant::now());
        }
        self.backend.send(&org, command)
    }

    /// Swaps in a backend connected with `settings` from the connector; the old one is dropped
    /// (a live one closes its connection off the UI thread). Returns the new change receiver.
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the connection cannot be started; the old backend stays.
    pub fn connect(
        &mut self,
        settings: ConnectSettings,
    ) -> Result<Receiver<BackendEvent>, BackendError> {
        self.backend = (self.connector)(settings)?;
        self.signin.connecting();
        Ok(self.open())
    }

    /// Closes the backend's connection; its status change comes back through the channel and
    /// the last books stay.
    pub fn disconnect(&mut self) {
        self.backend.disconnect();
    }
}

/// The backend and what the shell knows of it. Screens `cx.observe` this entity and re-derive
/// their rows from [`LedgerStore::view`] on each notification.
pub struct LedgerStore {
    core: StoreCore,
    pump_task: Task<()>,
    session: session::Sessions,
}

impl LedgerStore {
    /// Opens the store on `backend` with its first organisation selected. With `signin` the
    /// Connect screen offers the WorkOS sign-in, and when `settings` name a server with a saved
    /// sign-in ([`SignInSetup::has_saved`]) the store resumes it at once; the app then passes an
    /// unconnected backend, so no device connection opens first.
    pub fn new(
        backend: Box<dyn LedgerBackend>,
        connector: Connector,
        settings: Option<ConnectSettings>,
        signin: Option<SignInSetup>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (mut core, rx) = StoreCore::new(backend, connector);
        *core.signin_mut() = SignInState::new(signin.is_some());
        let resume = settings
            .clone()
            .filter(|s| signin.as_ref().is_some_and(|setup| setup.has_saved(s)));
        let mut store = Self {
            core,
            pump_task: pump(rx, cx),
            session: session::Sessions::new(signin, settings),
        };
        if let Some(settings) = resume {
            store.resume(settings, cx);
        }
        store
    }

    /// Writes start-up and round-trip times to stderr (`--timings on`).
    pub fn log_timings(&mut self, started: Instant) {
        self.core.log_timings(started);
    }

    /// What the shell and the screens render from.
    #[must_use]
    pub fn view(&self) -> &LedgerView {
        self.core.view()
    }

    /// Who the app acts as in the selected organisation ([`LedgerBackend::actor`]).
    #[must_use]
    pub fn actor(&self) -> Option<Actor> {
        self.core.actor()
    }

    /// Selects `org` and its books.
    pub fn select_org(&mut self, org: &OrganizationId, cx: &mut Context<Self>) {
        if self.core.select_org(org) {
            cx.notify();
        }
    }

    /// [`StoreCore::submit`]. Screens go through `panels::submit`, which awaits a pending
    /// result in a task.
    pub fn submit(&mut self, command: LedgerCommand) -> Submission {
        self.core.submit(command)
    }

    /// Connects to the server `settings` names, replacing the backend ([`StoreCore::connect`]).
    /// Signed in, the connection presents the session's access token, refreshed first when it
    /// is about to expire; with a sign-in saved for that server the store resumes it; otherwise
    /// it connects with the device identity.
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the backend cannot start connecting.
    pub fn connect(
        &mut self,
        settings: ConnectSettings,
        cx: &mut Context<Self>,
    ) -> Result<(), BackendError> {
        self.connect_signed_in_or_device(settings, cx)
    }

    /// Connects with exactly `settings` (their access token, if any).
    fn open_backend(
        &mut self,
        settings: ConnectSettings,
        cx: &mut Context<Self>,
    ) -> Result<(), BackendError> {
        let rx = self.core.connect(settings)?;
        self.pump_task = pump(rx, cx);
        cx.notify();
        Ok(())
    }

    /// The WorkOS sign-in as the Connect screen shows it.
    #[must_use]
    pub fn signin(&self) -> &SignInState {
        self.core.signin()
    }

    /// Closes the connection, keeping the last books.
    pub fn disconnect(&mut self) {
        self.core.disconnect();
    }

    /// [`StoreCore::set_proofs_url`].
    pub fn set_proofs_url(&mut self, url: Option<String>) {
        self.core.set_proofs_url(url);
    }

    /// [`StoreCore::proofs_client`].
    ///
    /// # Errors
    ///
    /// As [`StoreCore::proofs_client`].
    pub fn proofs_client(&self) -> Result<(ProofsClient, OrganizationId), ProofsError> {
        self.core.proofs_client()
    }

    /// [`StoreCore::attach_proof`].
    pub fn attach_proof(&mut self, proof: NewVerification) -> Submission {
        self.core.attach_proof(proof)
    }

    /// [`StoreCore::mark_published`].
    pub fn mark_published(&mut self, verification_id: &str, published: bool) -> Submission {
        self.core.mark_published(verification_id, published)
    }

    /// [`StoreCore::log_verification_access`].
    pub fn log_verification_access(
        &mut self,
        verification_id: &str,
        access: AccessType,
    ) -> Submission {
        self.core.log_verification_access(verification_id, access)
    }

    /// [`StoreCore::write_campaign`]. Screens go through `panels::write_campaign`, which
    /// awaits a pending result in a task.
    pub fn write_campaign(&mut self, write: CampaignWrite) -> Submission {
        self.core.write_campaign(write)
    }

    /// [`StoreCore::proofs_of`].
    #[must_use]
    pub fn proofs_of(&self, kind: LinkKind, id: &str) -> Vec<Verification> {
        self.core.proofs_of(kind, id)
    }

    /// [`StoreCore::write_flow`]. Screens go through `panels::write_flow`.
    pub fn write_flow(&mut self, write: FlowWrite) -> Submission {
        self.core.write_flow(write)
    }

    /// [`StoreCore::write_chain`]. Screens go through `panels::write_chain`.
    pub fn write_chain(&mut self, write: ChainWrite) -> Submission {
        self.core.write_chain(write)
    }

    /// [`StoreCore::public_organizations`].
    #[must_use]
    pub fn public_organizations(&self) -> Vec<PublicOrganization> {
        self.core.public_organizations()
    }

    /// [`StoreCore::chain_activity`].
    #[must_use]
    pub fn chain_activity(&self, chain_id: &str) -> Vec<ChainActivity> {
        self.core.chain_activity(chain_id)
    }
}

/// Wakes on the first event, takes whatever else is queued and applies it as one batch.
fn pump(rx: Receiver<BackendEvent>, cx: &mut Context<LedgerStore>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        while let Ok(first) = rx.recv().await {
            let mut batch = vec![first];
            while let Ok(more) = rx.try_recv() {
                batch.push(more);
            }
            let applied = this.update(cx, |store, cx| {
                if store.core.apply(batch) {
                    cx.notify();
                }
            });
            if applied.is_err() {
                break;
            }
        }
    })
}

mod session;
pub use session::SignInSetup;

#[cfg(test)]
mod tests;
