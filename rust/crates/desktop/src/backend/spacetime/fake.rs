//! A [`LiveClient`] whose organisations, books and events a test sets by hand.

#![expect(clippy::unwrap_used, reason = "test support may unwrap")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_channel::{Receiver, Sender};
use ledger_client::{
    ClientError, ClientEvent, ConnectionStatus as ClientStatus, MemberInfo,
    Organisation as ClientOrganisation,
};
use ledger_core::demo::build_demo_ledger;
use ledger_core::types::OrganizationId;
use ledger_core::{CommandResult, LedgerCommand, LedgerState};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::types::{NewVerification, Verification};

use super::{LiveClient, lock};
use crate::backend::{CampaignWrite, Submission};

pub const ME: &str = "c200aa00000000000000000000000000000000000000000000000000000000ff";

/// A client with set organisations and books, whose events the test sends by hand.
pub struct Fake {
    pub orgs: Mutex<Vec<ClientOrganisation>>,
    pub books: Mutex<HashMap<String, LedgerState>>,
    pub events: (Sender<ClientEvent>, Receiver<ClientEvent>),
    pub sent: Mutex<Vec<(String, LedgerCommand)>>,
    pub proofs_sent: Mutex<Vec<(String, NewVerification)>>,
    pub proofs: Mutex<HashMap<String, Vec<Verification>>>,
    pub transparency: Mutex<HashMap<String, OrgTransparency>>,
    pub campaign_writes: Mutex<Vec<(String, CampaignWrite)>>,
}

impl Fake {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            orgs: Mutex::new(Vec::new()),
            books: Mutex::new(HashMap::new()),
            events: async_channel::unbounded(),
            sent: Mutex::new(Vec::new()),
            proofs_sent: Mutex::new(Vec::new()),
            proofs: Mutex::new(HashMap::new()),
            transparency: Mutex::new(HashMap::new()),
            campaign_writes: Mutex::new(Vec::new()),
        })
    }

    /// Adds `org` with the demo books renamed to it and `members`.
    pub fn add_org(&self, org: &str, members: Vec<MemberInfo>) {
        let mut state = build_demo_ledger().unwrap();
        state.settings.organization_id = OrganizationId::from(org);
        lock(&self.orgs).push(ClientOrganisation {
            org_id: org.to_owned(),
            settings: state.settings.clone(),
            members,
        });
        lock(&self.books).insert(org.to_owned(), state);
    }

    pub fn emit(&self, event: ClientEvent) {
        self.events.0.try_send(event).unwrap();
    }
}

impl LiveClient for Fake {
    fn status(&self) -> ClientStatus {
        ClientStatus::Connecting
    }

    fn organisations(&self) -> Vec<ClientOrganisation> {
        lock(&self.orgs).clone()
    }

    fn state(&self, org: &str) -> Result<LedgerState, ClientError> {
        lock(&self.books)
            .get(org)
            .cloned()
            .ok_or_else(|| ClientError::NoLedger(org.to_owned()))
    }

    fn subscribe(&self) -> Receiver<ClientEvent> {
        self.events.1.clone()
    }

    fn send(&self, org: &str, command: &LedgerCommand) -> Submission {
        lock(&self.sent).push((org.to_owned(), command.clone()));
        Submission::ready(Ok(CommandResult::None))
    }

    fn submit(&self, org: &str, command: &LedgerCommand) -> Result<CommandResult, ClientError> {
        lock(&self.sent).push((org.to_owned(), command.clone()));
        Ok(CommandResult::None)
    }

    fn disconnect(&self) {}

    fn create_verification(&self, org: &str, proof: &NewVerification) -> Submission {
        lock(&self.proofs_sent).push((org.to_owned(), proof.clone()));
        Submission::ready(Ok(CommandResult::None))
    }

    fn verifications(&self, org: &str) -> Vec<Verification> {
        lock(&self.proofs).get(org).cloned().unwrap_or_default()
    }

    fn transparency(&self, org: &str) -> Option<OrgTransparency> {
        lock(&self.transparency).get(org).cloned()
    }

    fn write_campaign(&self, org: &str, write: &CampaignWrite) -> Submission {
        lock(&self.campaign_writes).push((org.to_owned(), write.clone()));
        Submission::ready(Ok(CommandResult::None))
    }
}

pub fn member(identity: &str, user_id: &str, can_post: bool) -> MemberInfo {
    MemberInfo {
        identity: identity.to_owned(),
        user_id: user_id.to_owned(),
        name: format!("Name of {user_id}"),
        can_post,
        can_approve: false,
        can_close: true,
    }
}

/// The fake's identity reaching `Ready`.
pub fn ready() -> ClientEvent {
    ClientEvent::Status(ClientStatus::Ready {
        identity: ME.to_owned(),
    })
}
