//! Organisation chains (MOD2): one organisation (the source, a funder) asks to follow another's
//! money (the target, a partner); the target approves with one of its access groups as the level
//! of access, rejects, or later revokes; every step goes into the chain's activity log
//! (`OrganizationChain`, `ChainActivityLog`, `flowtrace/src/lib/supabase.ts:1086-1166`; behaviour
//! `flowtrace/src/lib/demo/api.ts:812-1033`).
//!
//! A chain belongs to two organisations, so it is not part of one organisation's records: the
//! rules work on [`Chains`], the chains and log entries a call touches.
//!
//! Who may: the policies let any member of the source ask and cancel, and any member of the
//! target change the chain (`flowtrace/supabase/migrations/010_chains_rls.sql:17-39`). Approving,
//! rejecting, revoking and changing the granted group hand another organisation an access group,
//! and managing access is for admins and managers (`006_access_groups_rls.sql:26-65`), so the
//! module asks a member of the target who can close periods for those (the same mapping as
//! [`super::plan`]).

use ledger_core::env::LedgerEnv;
use ledger_core::money::js_round;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::OrgTransparency;
use super::flows::NodeRef;
use super::types::{AccessGroupPermissions, Metadata, OrgType};
use crate::tables::Member;

/// `ChainStatus` (`supabase.ts:1086`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    /// Asked, not answered.
    Pending,
    /// Approved.
    Active,
    /// Turned down.
    Rejected,
    /// Ended by the target.
    Revoked,
}

impl ChainStatus {
    const fn word(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Rejected => "rejected",
            Self::Revoked => "revoked",
        }
    }
}

/// A link from a source organisation to a target (`OrganizationChain`, `supabase.ts:1088-1105`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationChain {
    /// Chain id.
    pub id: String,
    /// The organisation that asked.
    pub source_org_id: String,
    /// The organisation asked.
    pub target_org_id: String,
    /// A node of the target's flow the chain links to.
    pub link_node_id: Option<String>,
    /// Where it stands.
    pub status: ChainStatus,
    /// The target's access group the source gets.
    pub granted_group_id: Option<String>,
    /// Why the source asked.
    pub request_message: Option<String>,
    /// Why the target said no.
    pub rejection_reason: Option<String>,
    /// Who asked (user id).
    pub requested_by: String,
    /// When.
    pub requested_at: String,
    /// Who answered (approved or rejected).
    pub approved_by: Option<String>,
    /// When.
    pub approved_at: Option<String>,
    /// Who revoked it.
    pub revoked_by: Option<String>,
    /// When.
    pub revoked_at: Option<String>,
    /// When it was made.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// `ChainActivityAction` (`supabase.ts:1114-1120`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainAction {
    /// Asked.
    Requested,
    /// Approved.
    Approved,
    /// Rejected.
    Rejected,
    /// Revoked.
    Revoked,
    /// Granted group or link node changed.
    PermissionsUpdated,
    /// A node linked.
    NodeLinked,
}

/// One step of a chain (`ChainActivityLog`, `supabase.ts:1122-1130`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainActivity {
    /// Entry id.
    pub id: String,
    /// The chain.
    pub chain_id: String,
    /// What happened.
    pub action: ChainAction,
    /// Who did it (user id).
    pub actor_id: String,
    /// For which side.
    pub actor_org_id: String,
    /// The fields that changed.
    pub details: Metadata,
    /// When.
    pub created_at: String,
}

/// The chains and log entries one call works on.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chains {
    /// Chains, in insertion order.
    pub chains: Vec<OrganizationChain>,
    /// Log entries, in insertion order.
    pub log: Vec<ChainActivity>,
}

/// `"This chain is no longer ${requiredStatus}, so it can't be changed."` (`api.ts:906-910`).
#[must_use]
pub fn no_longer(required: ChainStatus) -> String {
    format!(
        "This chain is no longer {}, so it can't be changed.",
        required.word()
    )
}

fn manager_of<'a>(member: Option<&'a Member>, org_id: &str) -> Result<&'a Member, String> {
    let m = member.ok_or_else(|| format!("You are not a member of organisation {org_id}."))?;
    if m.can_close {
        Ok(m)
    } else {
        Err(
            "Only members who can close periods can answer for their organisation's chains."
                .to_owned(),
        )
    }
}

fn details(v: Value) -> Metadata {
    match v {
        Value::Object(m) => m,
        _ => Metadata::new(),
    }
}

/// `logChain` (`api.ts:824-839`): one log entry by the acting member for their side.
fn log(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    chain_id: &str,
    action: ChainAction,
    actor: &Member,
    actor_org_id: &str,
    details: Metadata,
) {
    let id = env.new_id();
    c.log.push(ChainActivity {
        id,
        chain_id: chain_id.to_owned(),
        action,
        actor_id: actor.user_id.clone(),
        actor_org_id: actor_org_id.to_owned(),
        details,
        created_at: env.now().to_string(),
    });
}

/// `requestChain` (`api.ts:865-892`): any member of the source (policy `chains_insert_policy`,
/// `010_chains_rls.sql:17-21`) asks the target; an empty message is none. The module adds what the
/// table enforces (`009_organization_chains.sql:46-50`): the target is another organisation of
/// the module, and there is no other chain from the source to it that is pending or active (the
/// table's unique pair would refuse any; the module lets a rejected or revoked pair ask again).
///
/// # Errors
///
/// Not a member of the source, the source itself as target, no such target, or a live chain.
pub fn request_chain(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    caller: Option<&Member>,
    source_org_id: &str,
    target_org_id: &str,
    target_exists: bool,
    message: Option<String>,
) -> Result<OrganizationChain, String> {
    let actor = caller
        .ok_or_else(|| format!("You are not a member of organisation {source_org_id}."))?
        .clone();
    if source_org_id == target_org_id {
        return Err("An organisation cannot chain to itself.".to_owned());
    }
    if !target_exists {
        return Err(format!("No ledger for organisation {target_org_id}."));
    }
    if c.chains.iter().any(|x| {
        x.source_org_id == source_org_id
            && x.target_org_id == target_org_id
            && matches!(x.status, ChainStatus::Pending | ChainStatus::Active)
    }) {
        return Err("There is already a pending or active chain to that organisation.".to_owned());
    }
    let id = env.new_id();
    let now = env.now().to_string();
    let row = OrganizationChain {
        id,
        source_org_id: source_org_id.to_owned(),
        target_org_id: target_org_id.to_owned(),
        link_node_id: None,
        status: ChainStatus::Pending,
        granted_group_id: None,
        request_message: message.filter(|m| !m.is_empty()),
        rejection_reason: None,
        requested_by: actor.user_id.clone(),
        requested_at: now.clone(),
        approved_by: None,
        approved_at: None,
        revoked_by: None,
        revoked_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    c.chains.push(row.clone());
    let d = details(json!({ "message": row.request_message }));
    log(
        c,
        env,
        &row.id,
        ChainAction::Requested,
        &actor,
        source_org_id,
        d,
    );
    Ok(row)
}

/// What a target's answer may grant: one of its access groups and one of its flow nodes.
fn check_grant(
    target: &OrgTransparency,
    group_id: &str,
    link_node_id: Option<&str>,
) -> Result<(), String> {
    if !target.access_groups.iter().any(|g| g.id == group_id) {
        return Err(format!("Access group {group_id} not found."));
    }
    if let Some(node) = link_node_id
        && !target.flow_nodes.iter().any(|n| n.id == node)
    {
        return Err(format!("Flow node {node} not found."));
    }
    Ok(())
}

/// `changeChain` (`api.ts:894-919`): the chain must stand at `required`; the update is applied
/// with `updated_at`, and logged for the target with the update as its details.
fn change(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    actor: &Member,
    chain_id: &str,
    required: ChainStatus,
    action: ChainAction,
    update: impl FnOnce(&mut OrganizationChain, &str) -> Value,
) -> Result<OrganizationChain, String> {
    let at = c
        .chains
        .iter()
        .position(|x| x.id == chain_id && x.status == required)
        .ok_or_else(|| no_longer(required))?;
    let now = env.now().to_string();
    let x = &mut c.chains[at];
    let d = update(x, &now);
    x.updated_at = now;
    let row = x.clone();
    log(
        c,
        env,
        chain_id,
        action,
        actor,
        &row.target_org_id,
        details(d),
    );
    Ok(row)
}

/// `approveChain` (`api.ts:921-940`): a pending chain becomes active with one of the target's
/// access groups and, when given, one of its nodes; `caller` is the caller's member row in the
/// target, `target` the target's records.
///
/// # Errors
///
/// Not an admin or manager of the target, not pending, or a group or node not the target's.
pub fn approve_chain(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    caller: Option<&Member>,
    target: &OrgTransparency,
    chain_id: &str,
    granted_group_id: &str,
    link_node_id: Option<String>,
) -> Result<OrganizationChain, String> {
    let target_org = target_of(c, chain_id, ChainStatus::Pending)?;
    let actor = manager_of(caller, &target_org)?.clone();
    let link = link_node_id.filter(|n| !n.is_empty());
    check_grant(target, granted_group_id, link.as_deref())?;
    let user = actor.user_id.clone();
    change(
        c,
        env,
        &actor,
        chain_id,
        ChainStatus::Pending,
        ChainAction::Approved,
        |x, now| {
            x.status = ChainStatus::Active;
            x.approved_by = Some(user.clone());
            x.approved_at = Some(now.to_owned());
            x.granted_group_id = Some(granted_group_id.to_owned());
            x.link_node_id.clone_from(&link);
            json!({
                "status": "active",
                "approved_by": user,
                "approved_at": now,
                "granted_group_id": granted_group_id,
                "link_node_id": link,
            })
        },
    )
}

/// `rejectChain` (`api.ts:942-959`): a pending chain becomes rejected; an empty reason is none.
///
/// # Errors
///
/// Not an admin or manager of the target, or not pending.
pub fn reject_chain(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    caller: Option<&Member>,
    chain_id: &str,
    reason: Option<String>,
) -> Result<OrganizationChain, String> {
    let target_org = target_of(c, chain_id, ChainStatus::Pending)?;
    let actor = manager_of(caller, &target_org)?.clone();
    let reason = reason.filter(|r| !r.is_empty());
    let user = actor.user_id.clone();
    change(
        c,
        env,
        &actor,
        chain_id,
        ChainStatus::Pending,
        ChainAction::Rejected,
        |x, now| {
            x.status = ChainStatus::Rejected;
            x.approved_by = Some(user.clone());
            x.approved_at = Some(now.to_owned());
            x.rejection_reason.clone_from(&reason);
            json!({
                "status": "rejected",
                "approved_by": user,
                "approved_at": now,
                "rejection_reason": reason,
            })
        },
    )
}

/// `revokeChain` (`api.ts:961-969`): an active chain becomes revoked.
///
/// # Errors
///
/// Not an admin or manager of the target, or not active.
pub fn revoke_chain(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    caller: Option<&Member>,
    chain_id: &str,
) -> Result<OrganizationChain, String> {
    let target_org = target_of(c, chain_id, ChainStatus::Active)?;
    let actor = manager_of(caller, &target_org)?.clone();
    let user = actor.user_id.clone();
    change(
        c,
        env,
        &actor,
        chain_id,
        ChainStatus::Active,
        ChainAction::Revoked,
        |x, now| {
            x.status = ChainStatus::Revoked;
            x.revoked_by = Some(user.clone());
            x.revoked_at = Some(now.to_owned());
            json!({ "status": "revoked", "revoked_by": user, "revoked_at": now })
        },
    )
}

/// `updateChainPermissions` (`api.ts:978-990`): an active chain gets another of the target's
/// groups and node (none when not given).
///
/// # Errors
///
/// Not an admin or manager of the target, not active, or a group or node not the target's.
pub fn update_chain_permissions(
    c: &mut Chains,
    env: &mut dyn LedgerEnv,
    caller: Option<&Member>,
    target: &OrgTransparency,
    chain_id: &str,
    granted_group_id: &str,
    link_node_id: Option<String>,
) -> Result<OrganizationChain, String> {
    let target_org = target_of(c, chain_id, ChainStatus::Active)?;
    let actor = manager_of(caller, &target_org)?.clone();
    let link = link_node_id.filter(|n| !n.is_empty());
    check_grant(target, granted_group_id, link.as_deref())?;
    change(
        c,
        env,
        &actor,
        chain_id,
        ChainStatus::Active,
        ChainAction::PermissionsUpdated,
        |x, _| {
            x.granted_group_id = Some(granted_group_id.to_owned());
            x.link_node_id.clone_from(&link);
            json!({ "granted_group_id": granted_group_id, "link_node_id": link })
        },
    )
}

/// The target of a chain at `required`, or the demo's refusal.
fn target_of(c: &Chains, chain_id: &str, required: ChainStatus) -> Result<String, String> {
    c.chains
        .iter()
        .find(|x| x.id == chain_id && x.status == required)
        .map(|x| x.target_org_id.clone())
        .ok_or_else(|| no_longer(required))
}

/// `cancelChainRequest` (`api.ts:971-976`): a pending chain goes, with its log (the table's
/// `ON DELETE CASCADE`, `009_organization_chains.sql:56`; the demo leaves the entries behind,
/// unreachable). Anything else, or an unknown id, changes nothing, as in the demo. Any member of
/// the source (policy `chains_delete_policy`, `010_chains_rls.sql:35-39`); `source_member` is the
/// caller's member row in the chain's source.
///
/// # Errors
///
/// Not a member of the source of a pending chain.
pub fn cancel_chain_request(
    c: &mut Chains,
    source_member: Option<&Member>,
    chain_id: &str,
) -> Result<(), String> {
    let Some(x) = c
        .chains
        .iter()
        .find(|x| x.id == chain_id && x.status == ChainStatus::Pending)
    else {
        return Ok(());
    };
    if source_member.is_none() {
        return Err(format!(
            "You are not a member of organisation {}.",
            x.source_org_id
        ));
    }
    c.chains.retain(|x| x.id != chain_id);
    c.log.retain(|l| l.chain_id != chain_id);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------------

/// `ChainCounts` (`supabase.ts:1132-1137`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainCounts {
    /// Asked by this organisation, pending.
    pub outgoing_pending: u64,
    /// Asked by it, active.
    pub outgoing_active: u64,
    /// Asked of it, pending.
    pub incoming_pending: u64,
    /// Asked of it, active.
    pub incoming_active: u64,
}

/// `getChainCounts` (`api.ts:1000-1016`).
#[must_use]
pub fn chain_counts<'a>(
    chains: impl IntoIterator<Item = &'a OrganizationChain>,
    org_id: &str,
) -> ChainCounts {
    let mut n = ChainCounts {
        outgoing_pending: 0,
        outgoing_active: 0,
        incoming_pending: 0,
        incoming_active: 0,
    };
    for c in chains {
        let out = c.source_org_id == org_id;
        let inc = c.target_org_id == org_id;
        match c.status {
            ChainStatus::Pending if out => n.outgoing_pending += 1,
            ChainStatus::Active if out => n.outgoing_active += 1,
            _ => {}
        }
        match c.status {
            ChainStatus::Pending if inc => n.incoming_pending += 1,
            ChainStatus::Active if inc => n.incoming_active += 1,
            _ => {}
        }
    }
    n
}

/// `ChainTrustScore.scores` (`supabase.ts:1142-1145`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainScores {
    /// The source's score.
    pub source_org: i64,
    /// The target's score.
    pub target_org: i64,
}

/// `ChainTrustScore` (`supabase.ts:1139-1147`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainTrustScore {
    /// The rounded mean.
    pub aggregated_score: i64,
    /// Always 2.
    pub org_count: u64,
    /// The two scores.
    pub scores: ChainScores,
}

/// `getChainTrustScore` (`api.ts:1018-1031`): the rounded mean of the two organisations' scores.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "scores are 0 to 100"
)]
pub fn chain_trust_score(source_org: i64, target_org: i64) -> ChainTrustScore {
    ChainTrustScore {
        aggregated_score: js_round((source_org + target_org) as f64 / 2.0) as i64,
        org_count: 2,
        scores: ChainScores {
            source_org,
            target_org,
        },
    }
}

/// `getChainActivityLog` (`api.ts:992-998`): one chain's entries, newest first (equal times keep
/// their order).
#[must_use]
pub fn activity_of(log: &[ChainActivity], chain_id: &str) -> Vec<ChainActivity> {
    let mut rows: Vec<ChainActivity> = log
        .iter()
        .filter(|l| l.chain_id == chain_id)
        .cloned()
        .collect();
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    rows
}

/// `OrganizationChainWithDetails.source_org` and `target_org` (`orgRef`, `api.ts:94-105`), with
/// the logo as its object key and the score from the trust row (0 without one).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgRef {
    /// Organisation id.
    pub id: String,
    /// Name (the id until it has a profile).
    pub name: String,
    /// Logo object key.
    pub logo_key: Option<String>,
    /// Trust score.
    pub trust_score: i64,
    /// Kind of organisation.
    #[serde(rename = "type")]
    pub org_type: Option<OrgType>,
    /// Country.
    pub country: String,
}

/// An organisation as the other side of a chain sees it.
#[must_use]
pub fn org_ref(org_id: &str, t: &OrgTransparency) -> OrgRef {
    let p = t.profile.as_ref();
    OrgRef {
        id: org_id.to_owned(),
        name: p.map_or_else(|| org_id.to_owned(), |p| p.name.clone()),
        logo_key: p.and_then(|p| p.logo_key.clone()),
        trust_score: t.trust.as_ref().map_or(0, |s| s.score),
        org_type: p.and_then(|p| p.org_type),
        country: p.map_or_else(String::new, |p| p.country.clone()),
    }
}

/// `OrganizationChainWithDetails.granted_group` (`groupRef`, `api.ts:107-112`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRef {
    /// Group id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Short id.
    pub slug: String,
    /// What it grants.
    pub permissions: AccessGroupPermissions,
}

/// `OrganizationChainWithDetails` (`supabase.ts:1107-1112`, `chainDetails`, `api.ts:816-822`):
/// what `my_organization_chain` gives each side, since neither can read the other's records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainDetails {
    /// The chain.
    #[serde(flatten)]
    pub chain: OrganizationChain,
    /// The source.
    pub source_org: OrgRef,
    /// The target.
    pub target_org: OrgRef,
    /// The target's linked node.
    pub link_node: Option<NodeRef>,
    /// The target's granted group.
    pub granted_group: Option<GroupRef>,
}

/// A chain with both organisations, its node and its group, from each side's records.
#[must_use]
pub fn details_of(
    chain: &OrganizationChain,
    source: &OrgTransparency,
    target: &OrgTransparency,
) -> ChainDetails {
    let node = chain.link_node_id.as_deref();
    let group = chain.granted_group_id.as_deref();
    ChainDetails {
        source_org: org_ref(&chain.source_org_id, source),
        target_org: org_ref(&chain.target_org_id, target),
        link_node: super::flows::node_ref(target, node),
        granted_group: group.and_then(|g| {
            target
                .access_groups
                .iter()
                .find(|x| x.id == g)
                .map(|x| GroupRef {
                    id: x.id.clone(),
                    name: x.name.clone(),
                    slug: x.slug.clone(),
                    permissions: x.permissions,
                })
        }),
        chain: chain.clone(),
    }
}
