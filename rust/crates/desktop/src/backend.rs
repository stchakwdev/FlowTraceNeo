//! Where the desktop gets its books: the [`LedgerBackend`] trait every screen reads through,
//! and [`InProcessBackend`], which runs the ledger core in the app itself.
//!
//! The trait follows the store-entity pattern of `rust/docs/adr/0002-gpui-spacetime.md`
//! ("Pattern for U1 to U10"): a backend hands out a channel of [`BackendEvent`]s, one store
//! entity drains it in batches, and screens re-derive what they show from the store. Writes are
//! [`LedgerCommand`]s, the same commands the SpacetimeDB module's `run_command` reducer takes
//! (`spacetimedb/src/index.ts`, `run_command`), so the live client of U10,
//! [`SpacetimeBackend`], slots in behind the same trait.

mod spacetime;

#[cfg(test)]
pub(crate) use spacetime::fake as spacetime_fake;
pub use spacetime::{LiveClient, SpacetimeBackend, actor_for, backend_error, status_of};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_channel::{Receiver, Sender};
use ledger_client::AccessToken;
use ledger_core::demo::{build_demo_ledger, demo_actor};
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::setup::create_org_ledger;
use ledger_core::types::{Actor, ActorId, OrganizationId};
use ledger_core::{
    CommandError, CommandResult, Ledger, LedgerCommand, LedgerEnv, LedgerState, Timestamp,
    apply_command,
};
use ledger_module::tables::Member;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::plan::{self as transparency_plan, Ctx, LedgerLinks};
use ledger_module::transparency::types::{
    CampaignPatch, NewCampaign, NewDonation, NewVerification, PaymentStatus, Verification,
};
use ledger_module::transparency::{trust, ts};
use serde_json::Value;
use spacetimedb::Identity;
use uuid::Uuid;

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
    /// A WorkOS sign-in's access token, presented instead of the token file's device token
    /// (`crate::signin`); memory only, and its `Debug` hides it.
    pub access_token: Option<AccessToken>,
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
    /// An organisation's transparency records changed (campaigns, donations, flow nodes,
    /// transactions, trust score); re-read [`LedgerBackend::transparency`].
    Transparency {
        /// The organisation.
        org: OrganizationId,
    },
}

/// A write to an organisation's campaigns and donations: the module's reducers
/// `create_campaign`, `update_campaign`, `delete_campaign`, `record_donation` and
/// `update_donation_status` (`crates/ledger-module/src/transparency/reducers.rs`), amounts in
/// minor units. The new records come back as a [`BackendEvent::Transparency`].
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignWrite {
    /// `createCampaign` (`api.ts:1080-1098`).
    Create(NewCampaign),
    /// `updateCampaign` (`api.ts:1100-1114`): the fields `patch` sets.
    Update {
        /// The campaign.
        id: String,
        /// What changes.
        patch: CampaignPatch,
    },
    /// `deleteCampaign` (`api.ts:1116-1126`): drafts only; the module keeps any other silently.
    Delete {
        /// The campaign.
        id: String,
    },
    /// `createDonation` (`api.ts:1256-1272`): a record, no payment is processed (P19).
    RecordDonation(NewDonation),
    /// `updateDonationStatus` (`api.ts:1274-1289`).
    DonationStatus {
        /// The donation.
        id: String,
        /// Its new payment status.
        status: PaymentStatus,
        /// The payment's reference; blank or `None` stores none.
        reference: Option<String>,
    },
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
    /// The books are only the last ones seen: the app is not connected, so it refuses writes.
    #[error(
        "You are offline, so nothing was saved. Reconnect on the Connection screen, then try again."
    )]
    Offline,
}

/// What a submitted command comes to.
pub type SubmitResult = Result<CommandResult, BackendError>;

/// A submitted command's result: known at once ([`InProcessBackend`]) or once the server answers
/// ([`SpacetimeBackend`]). The store awaits a pending one in a task, never on the UI thread.
pub struct Submission(Outcome);

enum Outcome {
    Ready(SubmitResult),
    Pending(Pin<Box<dyn Future<Output = SubmitResult> + Send>>),
}

impl Submission {
    /// A result known now.
    #[must_use]
    pub fn ready(result: SubmitResult) -> Self {
        Self(Outcome::Ready(result))
    }

    /// A result `future` resolves to.
    pub fn pending(future: impl Future<Output = SubmitResult> + Send + 'static) -> Self {
        Self(Outcome::Pending(Box::pin(future)))
    }

    /// The result when it is known now; else the submission back, to await.
    ///
    /// # Errors
    ///
    /// The submission itself while the result is still pending.
    pub fn now(self) -> Result<SubmitResult, Self> {
        match self.0 {
            Outcome::Ready(result) => Ok(result),
            pending @ Outcome::Pending(_) => Err(Self(pending)),
        }
    }

    /// The result, once it is known.
    ///
    /// # Errors
    ///
    /// The refusal the backend reports.
    pub async fn result(self) -> SubmitResult {
        match self.0 {
            Outcome::Ready(result) => result,
            Outcome::Pending(future) => future.await,
        }
    }
}

impl std::fmt::Debug for Submission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Outcome::Ready(result) => f.debug_tuple("Ready").field(result).finish(),
            Outcome::Pending(_) => f.write_str("Pending"),
        }
    }
}

/// The books behind the desktop: read a snapshot, watch for changes, submit commands.
///
/// Implementations: [`InProcessBackend`] (U1) and [`SpacetimeBackend`] (U10). Screens never
/// ask which one they have: both hand out the same events, snapshots and errors.
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

    /// [`LedgerBackend::submit`] without waiting: what the store calls. The default answers at
    /// once through `submit`, which suits a backend whose `submit` never blocks.
    fn send(&mut self, org: &OrganizationId, command: LedgerCommand) -> Submission {
        Submission::ready(self.submit(org, command))
    }

    /// Who the backend acts as in `org`: the claimant of a new claim and the approver of
    /// others' (`claims.$id.tsx:96`, `useLedgerActor`). `None` when it holds no books there.
    fn actor(&self, org: &OrganizationId) -> Option<Actor>;

    /// The connection status now.
    fn status(&self) -> ConnectionStatus;

    /// Connects to the server `settings` names; status changes arrive as
    /// [`BackendEvent::Status`].
    ///
    /// # Errors
    ///
    /// [`BackendError::Connection`] when the connection cannot be started.
    fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError>;

    /// Closes the connection, keeping the last books seen. Does nothing for a backend with no
    /// server.
    fn disconnect(&mut self) {}

    /// Whether the books live on a server, so that being disconnected means being offline.
    fn is_remote(&self) -> bool {
        false
    }

    /// Records a proof file on a claim or bill of `org`: the object key and SHA-256 the proofs
    /// Worker returned, through the module's `create_verification` (ADR 0004; P13). The new
    /// proof shows in [`LedgerBackend::proofs`] once it is recorded.
    fn attach_proof(&mut self, org: &OrganizationId, proof: NewVerification) -> Submission {
        let _ = (org, proof);
        Submission::ready(Err(BackendError::Connection(NO_PROOFS.to_owned())))
    }

    /// The proofs of `org` the caller may see (`my_verification`), oldest first.
    fn proofs(&self, org: &OrganizationId) -> Vec<Verification> {
        let _ = org;
        Vec::new()
    }

    /// The token the proofs Worker checks the caller by: the connection's SpacetimeDB token.
    /// `None` when the backend has none to give (the in-process books).
    fn proofs_token(&self) -> Option<String> {
        None
    }

    /// The transparency records of `org` as the caller may read them (`my_*` views, ADR 0004
    /// and 0005): campaigns with their figures, donations, flow nodes, transactions, the trust
    /// score. `None` while the backend holds none for it.
    fn transparency(&self, org: &OrganizationId) -> Option<Arc<OrgTransparency>> {
        let _ = org;
        None
    }

    /// Writes to the campaigns and donations of `org` ([`CampaignWrite`]). On success
    /// subscribers get a [`BackendEvent::Transparency`] with the change readable.
    fn write_campaign(&mut self, org: &OrganizationId, write: CampaignWrite) -> Submission {
        let _ = (org, write);
        Submission::ready(Err(BackendError::Connection(NO_TRANSPARENCY.to_owned())))
    }
}

/// Why a backend keeps no campaigns.
pub const NO_TRANSPARENCY: &str =
    "Campaigns are kept with books on a server. Connect on the Connection screen first.";

/// Why a backend without a server keeps no proof files.
pub const NO_PROOFS: &str =
    "Proof files are kept with books on a server. Connect on the Connection screen first.";

/// One organisation's books in the in-process backend.
struct Books {
    org: Organisation,
    state: Arc<LedgerState>,
    env: Box<dyn LedgerEnv>,
    actor: Actor,
    /// The transparency records, changed by the module's own rules
    /// (`ledger_module::transparency::plan`): the demo's from `transparency-dataset.json`.
    transparency: Arc<OrgTransparency>,
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
        let demo_state = build_demo_ledger()?;
        let demo = Books {
            org: Organisation {
                id: demo_state.settings.organization_id.clone(),
                name: DEMO_ORG_NAME.to_owned(),
                base_currency: demo_state.settings.base_currency.clone(),
            },
            env: Box::new(demo_env_after(&demo_state)),
            state: Arc::new(demo_state),
            actor: demo_actor(),
            transparency: Arc::new(demo_transparency()?),
        };

        let mut env = SystemEnv;
        let org_id = env.new_id();
        let empty_state = create_org_ledger(&org_id, EMPTY_ORG_CURRENCY, current_year(), &mut env)?;
        let empty = Books {
            org: Organisation {
                id: OrganizationId::from(org_id),
                name: EMPTY_ORG_NAME.to_owned(),
                base_currency: EMPTY_ORG_CURRENCY.to_owned(),
            },
            state: Arc::new(empty_state),
            env: Box::new(env),
            actor: local_actor(),
            transparency: Arc::default(),
        };

        Ok(Self {
            books: vec![demo, empty],
            status: ConnectionStatus::Disconnected,
            settings: None,
            subscribers: Vec::new(),
        })
    }

    /// Puts [`crate::large_books`] with `lines` posted lines first in the list, so it opens
    /// selected (`--large-books`, for the General ledger's 50,000-line check). Its commands draw
    /// UUIDs and wall-clock time from [`SystemEnv`].
    ///
    /// # Errors
    ///
    /// [`BackendError::Setup`] when the books cannot be built.
    pub fn with_large_books(mut self, lines: usize) -> Result<Self, BackendError> {
        let state = crate::large_books::build(lines)?;
        let large = Books {
            org: Organisation {
                id: state.settings.organization_id.clone(),
                name: crate::large_books::LARGE_ORG_NAME.to_owned(),
                base_currency: state.settings.base_currency.clone(),
            },
            state: Arc::new(state),
            env: Box::new(SystemEnv),
            actor: local_actor(),
            transparency: Arc::default(),
        };
        self.books.insert(0, large);
        Ok(self)
    }

    /// Runs one rule of `ledger_module::transparency::plan` on a copy of `org`'s records as the
    /// backend's actor, then computes the trust score again, as the module's `change` does
    /// (`crates/ledger-module/src/transparency/reducers.rs`), and tells subscribers. A refusal
    /// leaves the records as they were.
    fn change_transparency(
        &mut self,
        org: &OrganizationId,
        rule: impl FnOnce(&mut OrgTransparency, &mut Ctx<'_>, &LedgerState) -> Result<(), String>,
    ) -> SubmitResult {
        let books = self
            .books
            .iter_mut()
            .find(|b| &b.org.id == org)
            .ok_or_else(|| BackendError::UnknownOrganisation(org.as_str().to_owned()))?;
        let member = member_of(org, &books.actor);
        let mut next = OrgTransparency::clone(&books.transparency);
        {
            let mut ctx = Ctx {
                org_id: org.as_str(),
                identity: Identity::ZERO,
                member: Some(&member),
                env: books.env.as_mut(),
            };
            rule(&mut next, &mut ctx, &books.state).map_err(BackendError::Connection)?;
        }
        let now = books.env.now().to_string();
        trust::refresh(&mut next, org.as_str(), &now, books.actor.id.as_str());
        books.transparency = Arc::new(next);
        self.broadcast(&BackendEvent::Transparency { org: org.clone() });
        Ok(CommandResult::None)
    }

    /// Sends `event` to every live subscriber and forgets the closed ones.
    fn broadcast(&mut self, event: &BackendEvent) {
        self.subscribers
            .retain(|tx| tx.try_send(event.clone()).is_ok());
    }

    /// The settings the last [`LedgerBackend::connect`] recorded.
    #[must_use]
    pub fn settings(&self) -> Option<&ConnectSettings> {
        self.settings.as_ref()
    }
}

impl LedgerBackend for InProcessBackend {
    fn organisations(&self) -> Vec<Organisation> {
        self.books.iter().map(|b| b.org.clone()).collect()
    }

    fn state(&self, org: &OrganizationId) -> Option<Arc<LedgerState>> {
        self.books
            .iter()
            .find(|b| &b.org.id == org)
            .map(|b| Arc::clone(&b.state))
    }

    fn subscribe(&mut self) -> Receiver<BackendEvent> {
        let (tx, rx) = async_channel::unbounded();
        self.subscribers.push(tx);
        rx
    }

    /// Applies the command to a copy of the books, so a snapshot a screen still holds never
    /// changes under it. The engine checks every rule before its first write
    /// (`ledger_core::engine`), so a refusal leaves the copy as it was and it is dropped.
    fn submit(
        &mut self,
        org: &OrganizationId,
        command: LedgerCommand,
    ) -> Result<CommandResult, BackendError> {
        let books = self
            .books
            .iter_mut()
            .find(|b| &b.org.id == org)
            .ok_or_else(|| BackendError::UnknownOrganisation(org.as_str().to_owned()))?;
        let mut next = LedgerState::clone(&books.state);
        let result = {
            let mut ledger = Ledger::new(&mut next, books.env.as_mut());
            apply_command(&mut ledger, &books.actor, command)?
        };
        books.state = Arc::new(next);
        let event = BackendEvent::Ledger {
            org: org.clone(),
            state: Arc::clone(&books.state),
        };
        self.broadcast(&event);
        Ok(result)
    }

    fn actor(&self, org: &OrganizationId) -> Option<Actor> {
        self.books
            .iter()
            .find(|b| &b.org.id == org)
            .map(|b| b.actor.clone())
    }

    fn status(&self) -> ConnectionStatus {
        self.status.clone()
    }

    fn connect(&mut self, settings: ConnectSettings) -> Result<(), BackendError> {
        self.settings = Some(settings);
        self.status = ConnectionStatus::Connecting;
        self.broadcast(&BackendEvent::Status(ConnectionStatus::Connecting));
        self.status = ConnectionStatus::Connected {
            identity: IN_PROCESS_IDENTITY.to_owned(),
        };
        self.broadcast(&BackendEvent::Status(self.status.clone()));
        Ok(())
    }

    /// Runs the module's own `create_verification` rules (`ledger_module::transparency::plan`)
    /// as the backend's actor, so the in-process books refuse what the server would. The
    /// in-process books have no Worker token ([`LedgerBackend::proofs_token`]), so screens only
    /// get here in tests.
    fn attach_proof(&mut self, org: &OrganizationId, proof: NewVerification) -> Submission {
        Submission::ready(self.change_transparency(org, |t, ctx, state| {
            transparency_plan::create_verification(t, ctx, proof, &StateLinks(state)).map(drop)
        }))
    }

    fn proofs(&self, org: &OrganizationId) -> Vec<Verification> {
        self.books
            .iter()
            .find(|b| &b.org.id == org)
            .map(|b| b.transparency.verifications.clone())
            .unwrap_or_default()
    }

    fn transparency(&self, org: &OrganizationId) -> Option<Arc<OrgTransparency>> {
        self.books
            .iter()
            .find(|b| &b.org.id == org)
            .map(|b| Arc::clone(&b.transparency))
    }

    /// Runs the module's own rules for each write (`plan::create_campaign`, `update_campaign`,
    /// `delete_campaign`, `record_donation`, `update_donation_status`). A public link is unique
    /// across the organisations the backend holds, as across the module's
    /// (`campaign_slug_elsewhere`, `reducers.rs`).
    fn write_campaign(&mut self, org: &OrganizationId, write: CampaignWrite) -> Submission {
        let elsewhere: Vec<String> = self
            .books
            .iter()
            .filter(|b| &b.org.id != org)
            .flat_map(|b| b.transparency.campaigns.iter())
            .filter_map(|c| c.public_slug.clone())
            .collect();
        let taken = move |s: &str| elsewhere.iter().any(|x| x == s);
        Submission::ready(self.change_transparency(org, |t, ctx, _| match write {
            CampaignWrite::Create(input) => {
                transparency_plan::create_campaign(t, ctx, input, &taken).map(drop)
            }
            CampaignWrite::Update { id, patch } => {
                transparency_plan::update_campaign(t, ctx, &id, patch, &taken).map(drop)
            }
            CampaignWrite::Delete { id } => transparency_plan::delete_campaign(t, ctx, &id),
            CampaignWrite::RecordDonation(input) => {
                transparency_plan::record_donation(t, ctx, input).map(drop)
            }
            CampaignWrite::DonationStatus {
                id,
                status,
                reference,
            } => {
                transparency_plan::update_donation_status(t, ctx, &id, status, reference).map(drop)
            }
        }))
    }
}

/// The demo organisation's transparency records: `rust/fixtures/transparency-dataset.json`
/// read as the module's `import_transparency` reads it (`ts::dataset`), with the trust row the
/// first write computes, stamped at the demo's frozen time (as ledger-client's tests do,
/// `crates/ledger-client/src/transparency.rs`, `demo`).
///
/// # Errors
///
/// [`BackendError::Connection`] when the bundled dataset does not read.
pub fn demo_transparency() -> Result<OrgTransparency, BackendError> {
    let fixture: Value = serde_json::from_str(DEMO_TRANSPARENCY)
        .map_err(|e| BackendError::Connection(e.to_string()))?;
    let org = fixture["demo_org_id"].as_str().unwrap_or_default();
    let mut t =
        ts::dataset(org, &fixture["store"]).map_err(|e| BackendError::Connection(e.to_string()))?;
    trust::refresh(&mut t, org, fixed::DEMO_NOW, "module");
    Ok(t)
}

/// The demo's transparency dataset, built into the app.
const DEMO_TRANSPARENCY: &str = include_str!("../../../fixtures/transparency-dataset.json");

/// The in-process actor as the module's member row (`ledger-module`, `tables::Member`).
fn member_of(org: &OrganizationId, actor: &Actor) -> Member {
    Member {
        key: format!("{}:{}", org.as_str(), actor.id.as_str()),
        identity: Identity::ZERO,
        org_id: org.as_str().to_owned(),
        user_id: actor.id.as_str().to_owned(),
        name: actor.name.clone(),
        can_post: actor.can_post,
        can_approve: actor.can_approve,
        can_close: actor.can_close,
    }
}

/// The claims and bills a proof may link to, from a snapshot of the books.
struct StateLinks<'a>(&'a LedgerState);

impl LedgerLinks for StateLinks<'_> {
    fn has_claim(&self, id: &str) -> bool {
        self.0.claims.iter().any(|c| c.id.as_str() == id)
    }

    fn has_bill(&self, id: &str) -> bool {
        self.0.bills.iter().any(|b| b.id.as_str() == id)
    }
}

/// A [`LedgerEnv`] on the wall clock with random UUID ids, as the TS module's `envFor`
/// (`spacetimedb/src/index.ts`: `new Date().toISOString()` and `crypto.randomUUID()`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEnv;

impl LedgerEnv for SystemEnv {
    fn now(&self) -> Timestamp {
        // iso_millis always writes the toISOString shape for years 0 to 9999; the fallback
        // is unreachable before the year 10000.
        Timestamp::parse(&iso_millis(now_millis())).unwrap_or_else(|_| fixed::demo().now())
    }

    fn new_id(&mut self) -> String {
        Uuid::new_v4().to_string()
    }
}

/// `ms` milliseconds after the Unix epoch as `Date.prototype.toISOString` writes it,
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
#[must_use]
pub fn iso_millis(ms: u64) -> String {
    let millis = ms % 1000;
    let secs = ms / 1000;
    let (days, secs_of_day) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        secs_of_day / 3600,
        secs_of_day / 60 % 60,
        secs_of_day % 60
    )
}

/// The proleptic Gregorian date `days` after 1970-01-01, by Howard Hinnant's
/// `civil_from_days` (<https://howardhinnant.github.io/date_algorithms.html#civil_from_days>),
/// for days on or after the epoch.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// Milliseconds since the Unix epoch now; 0 if the clock is before it.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The UTC calendar year now.
#[must_use]
pub fn current_year() -> i32 {
    let (year, _, _) = civil_from_days(now_millis() / 1000 / 86_400);
    i32::try_from(year).unwrap_or(i32::MAX)
}

/// A demo env that continues after the ids `state` already holds, so the next id is fresh.
#[must_use]
pub fn demo_env_after(state: &LedgerState) -> FixedEnv {
    let mut highest = 0;
    if let Ok(value) = serde_json::to_value(state) {
        highest_demo_id(&value, &mut highest);
    }
    let mut env = fixed::demo();
    while env.drawn() < highest {
        env.new_id();
    }
    env
}

/// The highest `n` of any `demo-<n in base 36>` string in `value`.
fn highest_demo_id(value: &Value, highest: &mut u64) {
    match value {
        Value::String(s) => {
            if let Some(n) = s
                .strip_prefix("demo-")
                .and_then(|digits| u64::from_str_radix(digits, 36).ok())
            {
                *highest = (*highest).max(n);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| highest_demo_id(v, highest)),
        Value::Object(map) => map.values().for_each(|v| highest_demo_id(v, highest)),
        _ => {}
    }
}

/// The actor of the empty organisation: a local user with every permission, as the demo user
/// (`ledger_core::demo::demo_actor`).
fn local_actor() -> Actor {
    Actor {
        id: ActorId::from("local-user"),
        name: "Local user".to_owned(),
        can_post: true,
        can_approve: true,
        can_close: true,
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::demo::DEMO_ORG_ID;
    use ledger_core::sanitize_command;
    use serde_json::json;

    #[test]
    fn large_books_open_first_with_their_lines() {
        let backend = InProcessBackend::with_demo_and_empty()
            .unwrap()
            .with_large_books(100)
            .unwrap();
        let orgs = backend.organisations();
        assert_eq!(orgs.len(), 3);
        assert_eq!(orgs[0].name, crate::large_books::LARGE_ORG_NAME);
        assert_eq!(orgs[1].name, DEMO_ORG_NAME);
        assert_eq!(backend.state(&orgs[0].id).unwrap().lines.len(), 100);
    }

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
    fn each_organisation_has_its_actor() {
        let backend = backend();
        let orgs = backend.organisations();
        assert_eq!(backend.actor(&demo()), Some(demo_actor()));
        assert_eq!(backend.actor(&orgs[1].id), Some(local_actor()));
        assert_eq!(backend.actor(&OrganizationId::from("nobody")), None);
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
        assert_eq!(empty.entries, vec![]);
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
        assert_eq!(
            *before,
            build_demo_ledger().unwrap(),
            "old snapshot untouched"
        );

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
        let first = backend.submit(&demo(), add_account("5991", "One")).unwrap();
        let second = backend.submit(&demo(), add_account("5992", "Two")).unwrap();
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
            access_token: None,
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

    fn new_campaign(name: &str) -> NewCampaign {
        serde_json::from_value(json!({
            "name": name,
            "target_amount": 50_000_000,
            "currency": "KES",
            "starts_at": "2026-02-04T09:00:00.000Z",
            "is_public": true,
        }))
        .unwrap()
    }

    #[test]
    fn the_demo_holds_the_transparency_dataset_and_the_empty_org_none() {
        let backend = backend();
        let t = backend.transparency(&demo()).unwrap();
        assert_eq!(
            (
                t.campaigns.len(),
                t.donations.len(),
                t.flow_nodes.len(),
                t.transactions.len()
            ),
            (4, 25, 22, 16)
        );
        assert_eq!(t.campaign_stats.len(), 4, "one figure row per campaign");
        assert!(t.trust.is_some(), "the trust row is computed at start");
        let empty = backend.organisations()[1].id.clone();
        assert_eq!(
            *backend.transparency(&empty).unwrap(),
            OrgTransparency::default()
        );
        assert!(
            backend
                .transparency(&OrganizationId::from("nobody"))
                .is_none()
        );
    }

    #[test]
    fn a_campaign_write_runs_the_module_rules_and_reaches_subscribers() {
        let mut backend = backend();
        let rx = backend.subscribe();
        let before = backend.transparency(&demo()).unwrap();
        let result = backend
            .write_campaign(&demo(), CampaignWrite::Create(new_campaign("Boreholes")))
            .now()
            .unwrap();
        assert_eq!(result, Ok(CommandResult::None));
        assert_eq!(
            rx.try_recv().unwrap(),
            BackendEvent::Transparency { org: demo() }
        );
        let after = backend.transparency(&demo()).unwrap();
        let made = after.campaigns.last().unwrap();
        assert_eq!(made.name, "Boreholes");
        assert_eq!(
            made.public_slug.as_deref(),
            Some("boreholes"),
            "slug from the name"
        );
        assert_eq!(made.created_by.as_deref(), Some(demo_actor().id.as_str()));
        assert_eq!(made.current_amount, 0);
        assert_eq!(before.campaigns.len(), 4, "the old snapshot is untouched");

        let id = made.id.clone();
        let donation: NewDonation = serde_json::from_value(json!({
            "campaign_id": id, "amount": 250_000, "currency": "KES",
            "payment_status": "confirmed", "payment_method": "mpesa",
        }))
        .unwrap();
        backend
            .write_campaign(&demo(), CampaignWrite::RecordDonation(donation))
            .now()
            .unwrap()
            .unwrap();
        let t = backend.transparency(&demo()).unwrap();
        let c = t.campaigns.iter().find(|c| c.id == id).unwrap();
        assert_eq!(c.current_amount, 250_000, "a confirmed gift is raised");
        let d = t.donations.last().unwrap().id.clone();
        backend
            .write_campaign(
                &demo(),
                CampaignWrite::DonationStatus {
                    id: d,
                    status: PaymentStatus::Refunded,
                    reference: Some("RF1".to_owned()),
                },
            )
            .now()
            .unwrap()
            .unwrap();
        let t = backend.transparency(&demo()).unwrap();
        assert_eq!(
            t.campaigns
                .iter()
                .find(|c| c.id == id)
                .unwrap()
                .current_amount,
            0
        );
    }

    #[test]
    fn a_refused_campaign_write_changes_nothing_and_sends_nothing() {
        let mut backend = backend();
        let rx = backend.subscribe();
        let before = backend.transparency(&demo()).unwrap();
        let mut zero = new_campaign("Nothing");
        zero.target_amount = 0;
        let err = backend
            .write_campaign(&demo(), CampaignWrite::Create(zero))
            .now()
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, BackendError::Connection(_)), "{err:?}");
        assert_eq!(backend.transparency(&demo()).unwrap(), before);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn only_a_draft_is_deleted_and_a_link_is_unique_across_organisations() {
        let mut backend = backend();
        let t = backend.transparency(&demo()).unwrap();
        let active = t.campaigns[0].id.clone();
        let draft = t
            .campaigns
            .iter()
            .find(|c| c.status == ledger_module::transparency::types::CampaignStatus::Draft)
            .unwrap()
            .id
            .clone();
        for id in [&active, &draft] {
            backend
                .write_campaign(&demo(), CampaignWrite::Delete { id: id.clone() })
                .now()
                .unwrap()
                .unwrap();
        }
        let t = backend.transparency(&demo()).unwrap();
        assert!(t.campaigns.iter().any(|c| c.id == active), "kept silently");
        assert!(!t.campaigns.iter().any(|c| c.id == draft));

        let empty = backend.organisations()[1].id.clone();
        let mut taken = new_campaign("Elsewhere");
        taken.public_slug = Some("clean-water-turkana".to_owned());
        let err = backend
            .write_campaign(&empty, CampaignWrite::Create(taken))
            .now()
            .unwrap()
            .unwrap_err();
        assert_eq!(
            err,
            BackendError::Connection(transparency_plan::SLUG_TAKEN.to_owned())
        );
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
