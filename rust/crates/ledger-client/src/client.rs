//! [`SpacetimeClient`]: the ledger module over SpacetimeDB, the Rust form of
//! `spacetimeLedgerBackend` (`flowtrace/src/lib/ledger/spacetime-backend.ts`).
//!
//! One thread per client owns the connection. It drives the SDK with its own loop over
//! `advance_one_message_blocking` instead of `run_threaded`, which panics its thread on a
//! disconnect that is not clean (SDK 2.12 `db_connection.rs`, `run_threaded`; ADR 0002). The
//! SDK runs every callback inside that call on this thread; callbacks only send on an internal
//! channel. After each message the loop drains that channel: it applies the row changes to the
//! [`RowCache`] as one batch, sends one [`ClientEvent::Changed`] naming the organisations they
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
use ledger_module::transparency::flows::{
    EmbedConfigPatch, FlowNodePatch, NewFlowNode, NewTransaction, NodeVisibilitySettings,
    TransactionPatch, TransactionVisibilitySettings,
};
use ledger_module::transparency::types::{
    AccessGroupPatch, AccessType, CampaignPatch, NewAccessGroup, NewCampaign, NewDonation,
    NewGroupMember, NewVerification, OrganizationPatch, PaymentStatus,
};
use spacetimedb_sdk::__codegen::InternalError;
use spacetimedb_sdk::{DbContext as _, Table, TableWithPrimaryKey};

use crate::auth::AccessToken;
use crate::cache::{Row, RowCache, RowChange};
use crate::error::{ClientError, decode_reducer_error, is_valid_org_id};
use crate::module_bindings::{
    self as b, DbConnection, MyAccessGroupMemberTableAccess as _, MyAccessGroupTableAccess as _,
    MyAccountTableAccess as _, MyAuditEventTableAccess as _, MyBankAccountTableAccess as _,
    MyBillTableAccess as _, MyBudgetLineTableAccess as _, MyBudgetTableAccess as _,
    MyCampaignStatTableAccess as _, MyCampaignTableAccess as _, MyChainActivityLogTableAccess as _,
    MyCommandResultTableAccess as _, MyDimensionTableAccess as _, MyDonationTableAccess as _,
    MyDonorUpdateTableAccess as _, MyExpenseClaimTableAccess as _, MyFiscalPeriodTableAccess as _,
    MyFlagTableAccess as _, MyFlowNodeTableAccess as _, MyFundTableAccess as _,
    MyGrantTableAccess as _, MyJournalEntryTableAccess as _, MyJournalLineTableAccess as _,
    MyLedgerOrgTableAccess as _, MyMemberTableAccess as _, MyNodeVisibilityTableAccess as _,
    MyOrganizationChainTableAccess as _, MyOrganizationProfileTableAccess as _,
    MyPaymentTableAccess as _, MyProfileTableAccess as _, MyPublicEmbedConfigTableAccess as _,
    MyServiceTableAccess as _, MyStatementLineTableAccess as _, MyTransactionTableAccess as _,
    MyTransactionVisibilityTableAccess as _, MyTrustScoreHistoryTableAccess as _,
    MyTrustScoreTableAccess as _, MyVendorTableAccess as _,
    MyVerificationAccessLogTableAccess as _, MyVerificationTableAccess as _,
    PublicBadgeTableAccess as _, PublicCampaignTableAccess as _,
    PublicOrganizationTableAccess as _, add_access_group_member as _, approve_chain as _,
    cancel_chain_request as _, create_access_group as _, create_campaign as _,
    create_default_access_groups as _, create_flow_node as _, create_org_ledger as _,
    create_transaction as _, create_verification as _, delete_access_group as _,
    delete_campaign as _, import_books as _, invite_access_group_member as _,
    link_transaction_entries as _, log_verification_access as _, mark_published as _,
    recompute_trust as _, record_donation as _, regenerate_embed_token as _, reject_chain as _,
    remove_access_group_member as _, remove_node_visibility as _,
    remove_transaction_visibility as _, request_chain as _, revoke_chain as _, run_command as _,
    set_node_visibility as _, set_transaction_visibility as _, update_access_group as _,
    update_campaign as _, update_chain_permissions as _, update_donation_status as _,
    update_flow_node as _, update_organization_profile as _, update_public_embed_config as _,
    update_transaction as _,
};
use crate::status::{
    ClientEvent, CommandTicket, ConnectionStatus, Organisation, RESULT_TIMEOUT, Ticket,
    TicketSender,
};
use crate::token::TokenFile;
use crate::transparency::{ServiceInfo, TransparencyCache};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::chains::{ChainActivity, ChainDetails};
use ledger_module::transparency::public::{PublicBadge, PublicCampaign, PublicOrganization};
use ledger_module::transparency::types::Profile;

/// The views the client subscribes to: every `my_*` view, which hold the rows of every
/// organisation the caller is a member of (`spacetime-backend.ts:105-108`, `LEDGER_TABLES`, plus
/// `my_member` for the organisation list), then the ten transparency views of MOD1 and the eleven
/// of MOD2 (`crates/ledger-module/src/transparency/views.rs`), then the three public tables, which
/// every connection may read, anonymous included. The TS client narrows each query to one
/// organisation; the desktop shows all of them, so these do not.
pub const VIEWS: [&str; 43] = [
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
    "my_organization_profile",
    "my_profile",
    "my_access_group",
    "my_access_group_member",
    "my_verification",
    "my_verification_access_log",
    "my_campaign",
    "my_campaign_stat",
    "my_donation",
    "my_donor_update",
    "my_flow_node",
    "my_transaction",
    "my_node_visibility",
    "my_transaction_visibility",
    "my_trust_score",
    "my_trust_score_history",
    "my_flag",
    "my_public_embed_config",
    "my_organization_chain",
    "my_chain_activity_log",
    "my_service",
    "public_organization",
    "public_campaign",
    "public_badge",
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
    /// A sign-in's access token. When set it is presented instead of the token file's, and the
    /// token the host returns is not saved: the file keeps the device identity for later.
    pub access_token: Option<AccessToken>,
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
            access_token: None,
            result_timeout: RESULT_TIMEOUT,
        }
    }

    /// The same config, presenting `token` from a sign-in ([`ClientConfig::access_token`]).
    #[must_use]
    pub fn with_access_token(mut self, token: AccessToken) -> Self {
        self.access_token = Some(token);
        self
    }

    /// The token to present and whether to save the one the host returns: a sign-in's access
    /// token, never saved, else the token file's device token (none on a first run), saved
    /// (`flowtrace/src/lib/ledger/spacetime-auth.ts`, `connectionToken`: `saveReturned` only for
    /// device identities).
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the token file exists but cannot be read.
    pub fn presented_token(&self) -> Result<(Option<String>, bool), ClientError> {
        if let Some(token) = &self.access_token {
            return Ok((Some(token.expose().to_owned()), false));
        }
        let saved = self
            .token_file
            .as_ref()
            .map(TokenFile::load)
            .transpose()?
            .flatten();
        Ok((saved, self.token_file.is_some()))
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
    subscribers: Vec<async_channel::Sender<ClientEvent>>,
}

impl Hub {
    fn broadcast(&mut self, event: &ClientEvent) {
        self.subscribers
            .retain(|tx| tx.try_send(event.clone()).is_ok());
    }

    fn set_status(&mut self, status: ConnectionStatus) {
        if self.status != status {
            self.status = status.clone();
            self.broadcast(&ClientEvent::Status(status));
        }
    }
}

struct Shared {
    hub: Mutex<Hub>,
    cache: Mutex<RowCache>,
    transparency: Mutex<TransparencyCache>,
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
    /// [`ConnectionStatus::Connecting`] and moves on as [`ClientEvent::Status`] events.
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the token file exists but cannot be read, or
    /// [`ClientError::Sdk`] when the thread cannot start.
    pub fn connect(config: ClientConfig) -> Result<Self, ClientError> {
        let (token, _) = config.presented_token()?;
        let shared = Arc::new(Shared {
            hub: Mutex::new(Hub {
                status: ConnectionStatus::Connecting,
                subscribers: Vec::new(),
            }),
            cache: Mutex::new(RowCache::default()),
            transparency: Mutex::new(TransparencyCache::default()),
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

    /// Records a proof on a claim, bill or transaction of the organisation: the R2 object key
    /// and SHA-256 the proofs Worker returned (`create_verification`, ADR 0004, Reducers; P13).
    /// The new row arrives through `my_verification` as any other change.
    pub fn create_verification(&self, org_id: &str, proof: &NewVerification) -> Ticket<()> {
        let json = match serde_json::to_string(proof) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .create_verification_then(org_id.to_owned(), json, done)
        })
    }

    /// Shows a proof at the proofs Worker's public path, or takes it back (`mark_published`,
    /// `crates/ledger-module/src/transparency/plan.rs`); admins and managers only.
    pub fn mark_published(
        &self,
        org_id: &str,
        verification_id: &str,
        published: bool,
    ) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.mark_published_then(
                org_id.to_owned(),
                verification_id.to_owned(),
                published,
                done,
            )
        })
    }

    /// Logs that the caller opened (`View`) or downloaded (`Download`) a proof's file
    /// (`log_verification_access`, `crates/ledger-module/src/transparency/plan.rs`;
    /// `logVerificationAccess`, `supabase.ts:554-571`). The module records the caller and the
    /// level they were given, never taken from the call; the row arrives through
    /// `my_verification_access_log`.
    pub fn log_verification_access(
        &self,
        org_id: &str,
        verification_id: &str,
        access: AccessType,
    ) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.log_verification_access_then(
                org_id.to_owned(),
                verification_id.to_owned(),
                access_arg(access),
                done,
            )
        })
    }

    /// Opens a campaign for the organisation (`create_campaign`, ADR 0004, Reducers;
    /// `createCampaign`, `api.ts:1080-1098`): any member; amounts in minor units. The new row
    /// arrives through `my_campaign` with its `my_campaign_stat` figures.
    pub fn create_campaign(&self, org_id: &str, campaign: &NewCampaign) -> Ticket<()> {
        let json = match serde_json::to_string(campaign) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .create_campaign_then(org_id.to_owned(), json, done)
        })
    }

    /// Changes a campaign field by field (`update_campaign`; `updateCampaign`,
    /// `api.ts:1100-1114`): any member; a field left out of `patch` is kept.
    pub fn update_campaign(
        &self,
        org_id: &str,
        campaign_id: &str,
        patch: &CampaignPatch,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers.update_campaign_then(
                org_id.to_owned(),
                campaign_id.to_owned(),
                json,
                done,
            )
        })
    }

    /// Deletes a draft campaign (`delete_campaign`; `deleteCampaign`, `api.ts:1116-1126`). The
    /// module silently keeps a campaign that is not a draft, so offer it for drafts only.
    pub fn delete_campaign(&self, org_id: &str, campaign_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers
                .delete_campaign_then(org_id.to_owned(), campaign_id.to_owned(), done)
        })
    }

    /// Records a donation (`record_donation`; `createDonation`, `api.ts:1256-1272`): a record
    /// only, no payment is processed (P19). A confirmed one adds to the campaign's raised amount.
    pub fn record_donation(&self, org_id: &str, donation: &NewDonation) -> Ticket<()> {
        let json = match serde_json::to_string(donation) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .record_donation_then(org_id.to_owned(), json, done)
        })
    }

    /// Sets a donation's payment status and reference (`update_donation_status`;
    /// `updateDonationStatus`, `api.ts:1274-1289`): any member; moving into or out of
    /// `confirmed` moves the campaign's raised amount. A blank reference is stored as none.
    pub fn update_donation_status(
        &self,
        org_id: &str,
        donation_id: &str,
        status: PaymentStatus,
        payment_reference: Option<&str>,
    ) -> Ticket<()> {
        let status = status_arg(status);
        let reference = payment_reference.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers.update_donation_status_then(
                org_id.to_owned(),
                donation_id.to_owned(),
                status,
                reference,
                done,
            )
        })
    }

    /// Adds a node to the organisation's flow map (`create_flow_node`, ADR 0005; `createNode`,
    /// `api.ts:264-274`): any member; the balance starts at zero.
    pub fn create_flow_node(&self, org_id: &str, node: &NewFlowNode) -> Ticket<()> {
        let json = match serde_json::to_string(node) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .create_flow_node_then(org_id.to_owned(), json, done)
        })
    }

    /// Changes a flow node field by field (`update_flow_node`, module addition): any member; a
    /// field left out of `patch` is kept.
    pub fn update_flow_node(
        &self,
        org_id: &str,
        node_id: &str,
        patch: &FlowNodePatch,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .update_flow_node_then(org_id.to_owned(), node_id.to_owned(), json, done)
        })
    }

    /// Records a movement between two flow nodes (`create_transaction`; `createTransaction`,
    /// `api.ts:220-238`): any member; amounts in minor units, `created_by` the caller. Posting it
    /// to the books stays a `run_command`; [`SpacetimeClient::link_transaction_entries`] ties the
    /// two.
    pub fn create_transaction(&self, org_id: &str, transaction: &NewTransaction) -> Ticket<()> {
        let json = match serde_json::to_string(transaction) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .create_transaction_then(org_id.to_owned(), json, done)
        })
    }

    /// Changes a transaction field by field (`update_transaction`; `updateTransaction`,
    /// `api.ts:240-254`): any member; a field left out of `patch` is kept.
    pub fn update_transaction(
        &self,
        org_id: &str,
        transaction_id: &str,
        patch: &TransactionPatch,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers.update_transaction_then(
                org_id.to_owned(),
                transaction_id.to_owned(),
                json,
                done,
            )
        })
    }

    /// Sets the journal entries that record a transaction (`link_transaction_entries`, module
    /// addition): each must be in the books and record no other transaction; an empty list
    /// unlinks.
    pub fn link_transaction_entries(
        &self,
        org_id: &str,
        transaction_id: &str,
        entry_ids: &[String],
    ) -> Ticket<()> {
        let ids = entry_ids.to_vec();
        self.call(|conn, done| {
            conn.reducers.link_transaction_entries_then(
                org_id.to_owned(),
                transaction_id.to_owned(),
                ids,
                done,
            )
        })
    }

    /// Sets what one access group sees of one node (`set_node_visibility`;
    /// `setNodeVisibility`, `api.ts:708-736`): admins and managers.
    pub fn set_node_visibility(
        &self,
        org_id: &str,
        node_id: &str,
        group_id: &str,
        settings: &NodeVisibilitySettings,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(settings) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers.set_node_visibility_then(
                org_id.to_owned(),
                node_id.to_owned(),
                group_id.to_owned(),
                json,
                done,
            )
        })
    }

    /// Drops a node's setting for one group, back to the group's own (`remove_node_visibility`).
    pub fn remove_node_visibility(
        &self,
        org_id: &str,
        node_id: &str,
        group_id: &str,
    ) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.remove_node_visibility_then(
                org_id.to_owned(),
                node_id.to_owned(),
                group_id.to_owned(),
                done,
            )
        })
    }

    /// Sets what one access group sees of one transaction (`set_transaction_visibility`, module
    /// addition): admins and managers.
    pub fn set_transaction_visibility(
        &self,
        org_id: &str,
        transaction_id: &str,
        group_id: &str,
        settings: &TransactionVisibilitySettings,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(settings) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers.set_transaction_visibility_then(
                org_id.to_owned(),
                transaction_id.to_owned(),
                group_id.to_owned(),
                json,
                done,
            )
        })
    }

    /// Drops a transaction's setting for one group (`remove_transaction_visibility`).
    pub fn remove_transaction_visibility(
        &self,
        org_id: &str,
        transaction_id: &str,
        group_id: &str,
    ) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.remove_transaction_visibility_then(
                org_id.to_owned(),
                transaction_id.to_owned(),
                group_id.to_owned(),
                done,
            )
        })
    }

    /// Asks `target_org_id` for a view of its flows on behalf of `source_org_id`
    /// (`request_chain`; `requestChain`, `api.ts:865-892`): any member of the source; the target
    /// must have books on the module. A blank message is none.
    pub fn request_chain(
        &self,
        source_org_id: &str,
        target_org_id: &str,
        message: Option<&str>,
    ) -> Ticket<()> {
        let message = message.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers.request_chain_then(
                source_org_id.to_owned(),
                target_org_id.to_owned(),
                message,
                done,
            )
        })
    }

    /// Approves a pending chain with one of the target's access groups and, when given, one of
    /// its nodes (`approve_chain`; `approveChain`, `api.ts:921-940`): a member of the target who
    /// can close periods.
    pub fn approve_chain(
        &self,
        chain_id: &str,
        granted_group_id: &str,
        link_node_id: Option<&str>,
    ) -> Ticket<()> {
        let node = link_node_id.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers.approve_chain_then(
                chain_id.to_owned(),
                granted_group_id.to_owned(),
                node,
                done,
            )
        })
    }

    /// Turns a pending chain down (`reject_chain`; `rejectChain`, `api.ts:942-959`): a member of
    /// the target who can close periods. A blank reason is none.
    pub fn reject_chain(&self, chain_id: &str, reason: Option<&str>) -> Ticket<()> {
        let reason = reason.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers
                .reject_chain_then(chain_id.to_owned(), reason, done)
        })
    }

    /// Ends an active chain (`revoke_chain`; `revokeChain`, `api.ts:961-969`): a member of the
    /// target who can close periods.
    pub fn revoke_chain(&self, chain_id: &str) -> Ticket<()> {
        self.call(|conn, done| conn.reducers.revoke_chain_then(chain_id.to_owned(), done))
    }

    /// Withdraws a pending request (`cancel_chain_request`; `cancelChainRequest`,
    /// `api.ts:971-976`): any member of the source; anything else changes nothing.
    pub fn cancel_chain_request(&self, chain_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers
                .cancel_chain_request_then(chain_id.to_owned(), done)
        })
    }

    /// Gives an active chain another of the target's groups and node
    /// (`update_chain_permissions`; `updateChainPermissions`, `api.ts:978-990`): a member of
    /// the target who can close periods.
    pub fn update_chain_permissions(
        &self,
        chain_id: &str,
        granted_group_id: &str,
        link_node_id: Option<&str>,
    ) -> Ticket<()> {
        let node = link_node_id.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers.update_chain_permissions_then(
                chain_id.to_owned(),
                granted_group_id.to_owned(),
                node,
                done,
            )
        })
    }

    /// Changes the organisation's profile field by field (`update_organization_profile`;
    /// `updateOrganization`, `api.ts:146-160`; `plan::update_organization`): a member who can
    /// close periods; a public link another organisation holds is refused as `SLUG_TAKEN`.
    pub fn update_organization_profile(
        &self,
        org_id: &str,
        patch: &OrganizationPatch,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .update_organization_profile_then(org_id.to_owned(), json, done)
        })
    }

    /// Adds the five default access groups (`create_default_access_groups`,
    /// `plan::create_default_access_groups`): a member who can close periods; refused when any
    /// default group exists.
    pub fn create_default_access_groups(&self, org_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers
                .create_default_access_groups_then(org_id.to_owned(), done)
        })
    }

    /// Adds an access group (`create_access_group`; `createAccessGroup`, `api.ts:585-596`): a
    /// member who can close periods.
    pub fn create_access_group(&self, org_id: &str, group: &NewAccessGroup) -> Ticket<()> {
        let json = match serde_json::to_string(group) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .create_access_group_then(org_id.to_owned(), json, done)
        })
    }

    /// Changes an access group field by field, permissions whole (`update_access_group`;
    /// `updateAccessGroup`, `api.ts:598-609`): a member who can close periods.
    pub fn update_access_group(
        &self,
        org_id: &str,
        group_id: &str,
        patch: &AccessGroupPatch,
    ) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers.update_access_group_then(
                org_id.to_owned(),
                group_id.to_owned(),
                json,
                done,
            )
        })
    }

    /// Deletes a custom access group and its members (`delete_access_group`;
    /// `deleteAccessGroup`, `api.ts:611-617`): a member who can close periods; default groups
    /// are refused.
    pub fn delete_access_group(&self, org_id: &str, group_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers
                .delete_access_group_then(org_id.to_owned(), group_id.to_owned(), done)
        })
    }

    /// Puts a person in an access group (`add_access_group_member`; `addGroupMember`,
    /// `api.ts:654-670`): a member who can close periods; `granted_by` is the caller.
    pub fn add_access_group_member(&self, org_id: &str, member: &NewGroupMember) -> Ticket<()> {
        let json = match serde_json::to_string(member) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .add_access_group_member_then(org_id.to_owned(), json, done)
        })
    }

    /// Invites an email address to an access group (`invite_access_group_member`;
    /// `inviteGroupMember`, `api.ts:672-690`): a member who can close periods. No email is sent;
    /// `valid_until` is a date or `None` for no end.
    pub fn invite_access_group_member(
        &self,
        org_id: &str,
        group_id: &str,
        email: &str,
        valid_until: Option<&str>,
    ) -> Ticket<()> {
        let until = valid_until.map(str::to_owned);
        self.call(|conn, done| {
            conn.reducers.invite_access_group_member_then(
                org_id.to_owned(),
                group_id.to_owned(),
                email.to_owned(),
                until,
                done,
            )
        })
    }

    /// Takes a person out of an access group (`remove_access_group_member`;
    /// `removeGroupMember`, `api.ts:692-698`): a member who can close periods.
    pub fn remove_access_group_member(&self, org_id: &str, member_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers.remove_access_group_member_then(
                org_id.to_owned(),
                member_id.to_owned(),
                done,
            )
        })
    }

    /// Changes the public embed's settings field by field (`update_public_embed_config`,
    /// `flows::update_embed_config`): a member who can close periods. No screen uses it yet; the
    /// web has no form for it.
    pub fn update_public_embed_config(&self, org_id: &str, patch: &EmbedConfigPatch) -> Ticket<()> {
        let json = match serde_json::to_string(patch) {
            Ok(json) => json,
            Err(e) => return Ticket::ready(Err(e.into())),
        };
        self.call(|conn, done| {
            conn.reducers
                .update_public_embed_config_then(org_id.to_owned(), json, done)
        })
    }

    /// Draws a new embed token, retiring the old one (`regenerate_embed_token`,
    /// `flows::regenerate_embed_token`): a member who can close periods.
    pub fn regenerate_embed_token(&self, org_id: &str) -> Ticket<()> {
        self.call(|conn, done| {
            conn.reducers
                .regenerate_embed_token_then(org_id.to_owned(), done)
        })
    }

    /// Recalculates the trust score and appends a history point (`recompute_trust`,
    /// `trust_reducers.rs`; the web's "Recalculate score"): any member.
    pub fn recompute_trust(&self, org_id: &str) -> Ticket<()> {
        self.call(|conn, done| conn.reducers.recompute_trust_then(org_id.to_owned(), done))
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

/// `update_donation_status`'s `status` argument: the status as its JSON string without the
/// quotes (`pending`, `confirmed`, `failed`, `refunded`), which the reducer parses back.
fn status_arg(status: PaymentStatus) -> String {
    match status {
        PaymentStatus::Pending => "pending",
        PaymentStatus::Confirmed => "confirmed",
        PaymentStatus::Failed => "failed",
        PaymentStatus::Refunded => "refunded",
    }
    .to_owned()
}

/// `log_verification_access`'s `access_type` argument: `view` or `download`, which the reducer
/// parses back as its JSON string.
fn access_arg(access: AccessType) -> String {
    match access {
        AccessType::View => "view",
        AccessType::Download => "download",
    }
    .to_owned()
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

/// Reads and writes.
impl SpacetimeClient {
    /// The connection's current status.
    #[must_use]
    pub fn status(&self) -> ConnectionStatus {
        lock(&self.shared.hub).status.clone()
    }

    /// The organisations whose books the caller can read, in id order.
    #[must_use]
    pub fn organisations(&self) -> Vec<Organisation> {
        lock(&self.shared.cache).organisations()
    }

    /// One organisation's books as the cache holds them now (`load`,
    /// `spacetime-backend.ts:193-197`), rebuilt from its rows.
    ///
    /// # Errors
    ///
    /// [`ClientError::InvalidOrgId`], [`ClientError::NoLedger`] when the caller cannot read the
    /// books (yet), or [`ClientError::Codec`] when the rows do not rebuild.
    pub fn state(&self, org_id: &str) -> Result<LedgerState, ClientError> {
        if !is_valid_org_id(org_id) {
            return Err(ClientError::InvalidOrgId(org_id.to_owned()));
        }
        lock(&self.shared.cache).state(org_id)
    }

    /// One organisation's transparency records as the caller may read them (proofs already cut
    /// to their access groups), rebuilt from the cached rows through the module's codec; empty
    /// when the organisation has none yet.
    ///
    /// # Errors
    ///
    /// [`ClientError::InvalidOrgId`], or [`ClientError::Codec`] when a row does not read.
    pub fn transparency(&self, org_id: &str) -> Result<OrgTransparency, ClientError> {
        if !is_valid_org_id(org_id) {
            return Err(ClientError::InvalidOrgId(org_id.to_owned()));
        }
        lock(&self.shared.transparency).transparency(org_id)
    }

    /// The person profiles the caller can read: their own and their co-members'.
    ///
    /// # Errors
    ///
    /// [`ClientError::Json`] when a profile does not read.
    pub fn profiles(&self) -> Result<Vec<Profile>, ClientError> {
        lock(&self.shared.transparency).profiles()
    }

    /// The chains of the caller's organisations, either side, with both organisations' names and
    /// scores and the target's node and group (`my_organization_chain`).
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn chains(&self) -> Result<Vec<ChainDetails>, ClientError> {
        lock(&self.shared.transparency).chains()
    }

    /// One chain's log, newest first (`my_chain_activity_log`).
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn chain_activity(&self, chain_id: &str) -> Result<Vec<ChainActivity>, ClientError> {
        lock(&self.shared.transparency).chain_activity(chain_id)
    }

    /// The caller's own service registration (`my_service`), when the caller is a service.
    #[must_use]
    pub fn service(&self) -> Option<ServiceInfo> {
        lock(&self.shared.transparency).service()
    }

    /// Every published organisation (`public_organization`), readable by anyone.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_organizations(&self) -> Result<Vec<PublicOrganization>, ClientError> {
        lock(&self.shared.transparency).public_organizations()
    }

    /// Every published campaign (`public_campaign`), readable by anyone.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_campaigns(&self) -> Result<Vec<PublicCampaign>, ClientError> {
        lock(&self.shared.transparency).public_campaigns()
    }

    /// Every published trust badge (`public_badge`), readable by anyone.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_badges(&self) -> Result<Vec<PublicBadge>, ClientError> {
        lock(&self.shared.transparency).public_badges()
    }

    /// A new receiver of change notifications. Its first event is the current
    /// [`ClientEvent::Status`]; after that it gets every event, one [`ClientEvent::Changed`] per
    /// applied message. Drop it to stop.
    #[must_use]
    pub fn subscribe(&self) -> async_channel::Receiver<ClientEvent> {
        let (tx, rx) = async_channel::unbounded();
        let mut hub = lock(&self.shared.hub);
        // An unbounded channel whose receiver is in hand takes the first event.
        let _ = tx.try_send(ClientEvent::Status(hub.status.clone()));
        hub.subscribers.push(tx);
        rx
    }

    /// Runs one command as the caller and blocks until its result: [`SpacetimeClient::send`],
    /// then [`Ticket::wait`]. Never call it on the UI thread.
    ///
    /// # Errors
    ///
    /// As [`SpacetimeClient::send`].
    pub fn submit(
        &self,
        org_id: &str,
        command: &LedgerCommand,
    ) -> Result<CommandResult, ClientError> {
        self.send(org_id, command).wait()
    }

    /// `run` (`spacetime-backend.ts:199-217`) without blocking: the command as JSON through
    /// `run_command` under a fresh request id. The ticket resolves with the `command_result` row
    /// of that id once the changed rows are in the cache, or with the reducer's error decoded
    /// ([`ClientError::Ledger`] for a broken rule), or [`ClientError::Timeout`]. The actor is
    /// the member the module finds for the caller's identity (`spacetime-backend.ts:6-8`).
    pub fn send(&self, org_id: &str, command: &LedgerCommand) -> CommandTicket {
        if !is_valid_org_id(org_id) {
            return Ticket::ready(Err(ClientError::InvalidOrgId(org_id.to_owned())));
        }
        let json = match serde_json::to_string(command) {
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
    watch(&db.my_organization_profile(), tx);
    watch_without_key(&db.my_profile(), tx);
    watch(&db.my_access_group(), tx);
    watch(&db.my_access_group_member(), tx);
    watch_without_key(&db.my_verification(), tx);
    watch_without_key(&db.my_verification_access_log(), tx);
    watch(&db.my_campaign(), tx);
    watch(&db.my_campaign_stat(), tx);
    watch(&db.my_donation(), tx);
    watch(&db.my_donor_update(), tx);
    watch(&db.my_flow_node(), tx);
    watch(&db.my_transaction(), tx);
    watch(&db.my_node_visibility(), tx);
    watch(&db.my_transaction_visibility(), tx);
    watch(&db.my_trust_score(), tx);
    watch(&db.my_trust_score_history(), tx);
    watch(&db.my_flag(), tx);
    watch(&db.my_public_embed_config(), tx);
    watch_without_key(&db.my_organization_chain(), tx);
    watch_without_key(&db.my_chain_activity_log(), tx);
    watch(&db.my_service(), tx);
    watch(&db.public_organization(), tx);
    watch(&db.public_campaign(), tx);
    watch(&db.public_badge(), tx);
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
    let mut transparency = Vec::new();
    let touched: BTreeSet<String> = {
        let mut cache = lock(&shared.cache);
        changes
            .into_iter()
            .filter_map(|c| match c {
                RowChange::Upsert(Row::Transparency(r)) => {
                    transparency.push((r, true));
                    None
                }
                RowChange::Delete(Row::Transparency(r)) => {
                    transparency.push((r, false));
                    None
                }
                c => {
                    if let RowChange::Upsert(Row::Result(r)) = &c {
                        results.push(r.clone());
                    }
                    cache.apply(c)
                }
            })
            .collect()
    };
    if !touched.is_empty() {
        lock(&shared.hub).broadcast(&ClientEvent::Changed { orgs: touched });
    }
    if !transparency.is_empty() {
        let b = lock(&shared.transparency).apply_all(transparency);
        lock(&shared.hub).broadcast(&ClientEvent::TransparencyChanged {
            orgs: b.orgs,
            profiles: b.profiles,
            chains: b.chains,
            public: b.public,
            service: b.service,
        });
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
                if let (Some(file), None) = (&config.token_file, &config.access_token)
                    && let Err(e) = file.save(&token)
                {
                    lock(&shared.hub).broadcast(&ClientEvent::Error(e.to_string()));
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
                lock(&shared.hub).broadcast(&ClientEvent::Error(e));
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
        hub.broadcast(&ClientEvent::Error(e.clone()));
    }
    hub.set_status(ConnectionStatus::Disconnected { error });
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::types::Nullish;
    use ledger_module::transparency::types::{AccessGroupPermissions, CampaignStatus};

    /// The reducer reads `status` as `serde_json::Value::String(status)` parsed into a
    /// `PaymentStatus` (`ledger_module::transparency::reducers::update_donation_status`).
    #[test]
    fn the_status_argument_parses_back_as_the_reducer_reads_it() {
        for status in [
            PaymentStatus::Pending,
            PaymentStatus::Confirmed,
            PaymentStatus::Failed,
            PaymentStatus::Refunded,
        ] {
            let text = serde_json::Value::String(status_arg(status)).to_string();
            let back: PaymentStatus = serde_json::from_str(&text).unwrap();
            assert_eq!(back, status);
        }
    }

    /// The reducer reads `access_type` the same way (`reducers::log_verification_access`).
    #[test]
    fn the_access_argument_parses_back_as_the_reducer_reads_it() {
        for access in [AccessType::View, AccessType::Download] {
            let text = serde_json::Value::String(access_arg(access)).to_string();
            let back: AccessType = serde_json::from_str(&text).unwrap();
            assert_eq!(back, access);
        }
        assert_eq!(access_arg(AccessType::View), "view");
    }

    /// DT3's proof writes take the same path as the others.
    #[test]
    fn proof_writes_before_a_connection_resolve_as_disconnected() {
        let client =
            SpacetimeClient::connect(ClientConfig::new("http://127.0.0.1:9", "none")).unwrap();
        let tickets = [
            client.mark_published("org-1", "v1", true),
            client.log_verification_access("org-1", "v1", AccessType::View),
        ];
        for ticket in tickets {
            assert!(matches!(ticket.wait(), Err(ClientError::Disconnected)));
        }
        client.disconnect();
    }

    fn campaign() -> NewCampaign {
        serde_json::from_value(serde_json::json!({
            "name": "Boreholes",
            "target_amount": 50_000_000,
            "currency": "KES",
            "starts_at": "2026-10-29T00:00:00.000Z",
        }))
        .unwrap()
    }

    /// Before the host has answered there is no connection, so every write resolves at once as
    /// disconnected instead of waiting for the timeout.
    #[test]
    fn writes_before_a_connection_resolve_as_disconnected() {
        let client =
            SpacetimeClient::connect(ClientConfig::new("http://127.0.0.1:9", "none")).unwrap();
        let donation: NewDonation = serde_json::from_value(serde_json::json!({
            "campaign_id": "c1", "amount": 100_000, "currency": "KES",
        }))
        .unwrap();
        let patch = CampaignPatch {
            status: Some(CampaignStatus::Active),
            ..CampaignPatch::default()
        };
        let tickets = [
            client.create_campaign("org-1", &campaign()),
            client.update_campaign("org-1", "c1", &patch),
            client.delete_campaign("org-1", "c1"),
            client.record_donation("org-1", &donation),
            client.update_donation_status("org-1", "d1", PaymentStatus::Confirmed, Some("REF")),
        ];
        for ticket in tickets {
            assert!(matches!(ticket.wait(), Err(ClientError::Disconnected)));
        }
        client.disconnect();
    }

    fn transaction() -> NewTransaction {
        serde_json::from_value(serde_json::json!({
            "from_node_id": "n1", "to_node_id": "n2", "amount": 150_000, "currency": "KES",
            "description": "School fees", "verification_status": "pending",
            "transaction_date": "2026-10-29",
        }))
        .unwrap()
    }

    /// DT2's flow, visibility and chain writes take the same path: with no connection each
    /// resolves at once as disconnected.
    #[test]
    fn flow_and_chain_writes_before_a_connection_resolve_as_disconnected() {
        let client =
            SpacetimeClient::connect(ClientConfig::new("http://127.0.0.1:9", "none")).unwrap();
        let node: NewFlowNode =
            serde_json::from_value(serde_json::json!({ "name": "Donor", "type": "donor" }))
                .unwrap();
        let ids = ["e1".to_owned()];
        let tickets = [
            client.create_flow_node("org-1", &node),
            client.update_flow_node("org-1", "n1", &FlowNodePatch::default()),
            client.create_transaction("org-1", &transaction()),
            client.update_transaction("org-1", "t1", &TransactionPatch::default()),
            client.link_transaction_entries("org-1", "t1", &ids),
            client.set_node_visibility("org-1", "n1", "g1", &NodeVisibilitySettings::default()),
            client.remove_node_visibility("org-1", "n1", "g1"),
            client.set_transaction_visibility(
                "org-1",
                "t1",
                "g1",
                &TransactionVisibilitySettings::default(),
            ),
            client.remove_transaction_visibility("org-1", "t1", "g1"),
            client.request_chain("org-1", "org-2", Some("Please")),
            client.approve_chain("c1", "g1", None),
            client.reject_chain("c1", Some("No")),
            client.revoke_chain("c1"),
            client.cancel_chain_request("c1"),
            client.update_chain_permissions("c1", "g1", Some("n1")),
        ];
        for ticket in tickets {
            assert!(matches!(ticket.wait(), Err(ClientError::Disconnected)));
        }
        client.disconnect();
    }

    /// DT4's settings, access-group, embed and trust writes take the same path: with no
    /// connection each resolves at once as disconnected.
    #[test]
    fn settings_writes_before_a_connection_resolve_as_disconnected() {
        let client =
            SpacetimeClient::connect(ClientConfig::new("http://127.0.0.1:9", "none")).unwrap();
        let tickets = [
            client.update_organization_profile("org-1", &OrganizationPatch::default()),
            client.create_default_access_groups("org-1"),
            client.create_access_group("org-1", &group()),
            client.update_access_group("org-1", "g1", &AccessGroupPatch::default()),
            client.delete_access_group("org-1", "g1"),
            client.add_access_group_member("org-1", &member()),
            client.invite_access_group_member("org-1", "g1", "a@example.org", Some("2027-01-01")),
            client.remove_access_group_member("org-1", "m1"),
            client.update_public_embed_config("org-1", &EmbedConfigPatch::default()),
            client.regenerate_embed_token("org-1"),
            client.recompute_trust("org-1"),
        ];
        for ticket in tickets {
            assert!(matches!(ticket.wait(), Err(ClientError::Disconnected)));
        }
        client.disconnect();
    }

    fn group() -> NewAccessGroup {
        serde_json::from_value(serde_json::json!({
            "name": "Auditors", "slug": "auditors", "description": null, "icon": "eye",
            "color": "#1C5FA8", "permissions": AccessGroupPermissions::default(),
            "node_scope": "all", "priority": 50,
        }))
        .unwrap()
    }

    fn member() -> NewGroupMember {
        serde_json::from_value(serde_json::json!({
            "group_id": "g1", "user_email": "a@example.org",
        }))
        .unwrap()
    }

    /// The settings arguments read back as the module parses them (`deny_unknown_fields`), and
    /// a patch sends only what it sets, with `null` kept apart from left out.
    #[test]
    fn settings_arguments_parse_back_as_the_module_reads_them() {
        let text = serde_json::to_string(&group()).unwrap();
        assert_eq!(
            serde_json::from_str::<NewAccessGroup>(&text).unwrap(),
            group()
        );
        let text = serde_json::to_string(&member()).unwrap();
        assert_eq!(
            serde_json::from_str::<NewGroupMember>(&text).unwrap(),
            member()
        );
        let patch = OrganizationPatch {
            name: Some("Demo NGO".to_owned()),
            public_slug: Nullish::Null,
            is_public: Some(true),
            ..OrganizationPatch::default()
        };
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"name":"Demo NGO","public_slug":null,"is_public":true}"#
        );
        let patch = AccessGroupPatch {
            priority: Some(10),
            ..AccessGroupPatch::default()
        };
        assert_eq!(serde_json::to_string(&patch).unwrap(), r#"{"priority":10}"#);
    }

    /// The JSON arguments read back as the module's reducers parse them (`parse` in
    /// `ledger_module::transparency::flow_reducers`, `deny_unknown_fields`), and a patch sends
    /// only what it sets.
    #[test]
    fn flow_arguments_parse_back_as_the_module_reads_them() {
        let text = serde_json::to_string(&transaction()).unwrap();
        let back: NewTransaction = serde_json::from_str(&text).unwrap();
        assert_eq!(back, transaction());
        let patch = TransactionPatch {
            amount: Some(200_000),
            ..TransactionPatch::default()
        };
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"amount":200000}"#
        );
        let node = FlowNodePatch {
            name: Some("Clinic".to_owned()),
            ..FlowNodePatch::default()
        };
        assert_eq!(
            serde_json::to_string(&node).unwrap(),
            r#"{"name":"Clinic"}"#
        );
    }

    /// A patch sends only the fields it sets, so the module keeps the rest
    /// (`CampaignPatch`, `skip_serializing_if`).
    #[test]
    fn a_status_patch_sends_the_status_alone() {
        let patch = CampaignPatch {
            status: Some(CampaignStatus::Paused),
            ..CampaignPatch::default()
        };
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"status":"paused"}"#
        );
    }
}
