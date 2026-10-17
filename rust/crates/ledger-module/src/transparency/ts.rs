//! TS-shaped JSON (`flowtrace/src/lib/supabase.ts` types, the demo dataset and the demo API's
//! arguments) into the module's records and inputs ([`super::types`]).
//!
//! The mapping is the list of differences in [`super::types`]: major units become minor units
//! through `fromMajor` (`flowtrace/src/lib/ledger/money.ts:26-31`), file URLs become object keys,
//! the organisation's `trust_score` is left to MOD2, and fields the module takes from the caller
//! (`created_by`, `granted_by`, `verified_by`) or from the reducer (`organization_id`) are
//! dropped from inputs. `import_transparency` loads a dataset through it, and the fixture test
//! compares the demo API's results with the module's through it.

use std::fmt::Write as _;

use ledger_core::money::from_major;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::OrgTransparency;
use super::chains::{ChainActivity, Chains, OrganizationChain};
use super::flows::{
    EmbedConfigPatch, FlowNode, NewFlowNode, NewTransaction, NodeVisibility,
    NodeVisibilitySettings, PublicEmbedConfig, Transaction, TransactionPatch,
};
use super::stats::campaign_stat;
use super::trust::TrustHistory;
use super::types::{
    AccessGroup, Campaign, CampaignPatch, Donation, DonorUpdate, GroupMember, NewAccessGroup,
    NewCampaign, NewDonation, NewDonorUpdate, NewGroupMember, NewVerification, OrganizationPatch,
    OrganizationProfile, Profile, Verification,
};

/// The TS `JSON` did not map onto a module record.
#[derive(Debug, thiserror::Error)]
pub enum TsError {
    /// Not an object.
    #[error("{0} is not an object.")]
    NotObject(&'static str),
    /// An amount that is not a whole number of minor units once scaled.
    #[error("{what}: {source}")]
    Money {
        /// Which field.
        what: &'static str,
        /// Why.
        source: ledger_core::money::MoneyError,
    },
    /// The shape did not deserialise.
    #[error("{what}: {source}")]
    Json {
        /// Which record.
        what: &'static str,
        /// The serde error.
        source: serde_json::Error,
    },
}

/// One field edit on the way from the TS shape.
enum Edit {
    /// Major units to minor units, `null` kept.
    Major(&'static str),
    /// Renamed.
    Rename(&'static str, &'static str),
    /// Dropped.
    Drop(&'static str),
}

fn convert<T: DeserializeOwned>(
    what: &'static str,
    v: &Value,
    edits: &[Edit],
) -> Result<T, TsError> {
    let mut m: Map<String, Value> = v.as_object().cloned().ok_or(TsError::NotObject(what))?;
    for edit in edits {
        match *edit {
            Edit::Major(key) => {
                if let Some(major) = m.get(key).and_then(Value::as_f64) {
                    let minor =
                        from_major(major).map_err(|source| TsError::Money { what: key, source })?;
                    m.insert(key.to_owned(), Value::from(minor));
                }
            }
            Edit::Rename(from, to) => {
                if let Some(value) = m.remove(from) {
                    m.insert(to.to_owned(), value);
                }
            }
            Edit::Drop(key) => {
                m.remove(key);
            }
        }
    }
    serde_json::from_value(Value::Object(m)).map_err(|source| TsError::Json { what, source })
}

/// `Organization` (`supabase.ts:28-40`): `logo_url` is the logo key, `trust_score` is MOD2's,
/// and `is_public` starts as "verified and has a public slug" (`002_rls_policies.sql:40-42`).
///
/// # Errors
///
/// When the JSON is not an `Organization`.
pub fn organization(v: &Value) -> Result<OrganizationProfile, TsError> {
    let mut p: OrganizationProfile = convert(
        "Organization",
        v,
        &[
            Edit::Rename("logo_url", "logo_key"),
            Edit::Drop("trust_score"),
        ],
    )?;
    p.is_public = p.verified && p.public_slug.is_some();
    Ok(p)
}

/// SHA-256 of the trimmed, lowercased address, lowercase hex; `None` for a blank one.
#[must_use]
pub fn email_hash(email: &str) -> Option<String> {
    let normal = crate::rules::js_trim(email).to_lowercase();
    if normal.is_empty() {
        return None;
    }
    Some(
        Sha256::digest(normal.as_bytes())
            .iter()
            .fold(String::with_capacity(64), |mut hex, b| {
                // Writing to a String cannot fail.
                let _ = write!(hex, "{b:02x}");
                hex
            }),
    )
}

/// `Profile` (`supabase.ts:42-50`) for `identity`: the email becomes its hash, the phone, role
/// and organisation are not kept (the member rows hold the last two).
///
/// # Errors
///
/// When the JSON is not a `Profile`.
pub fn profile(v: &Value, identity: &str) -> Result<Profile, TsError> {
    let field = |key: &str| v.get(key).and_then(Value::as_str);
    let created = field("created_at").unwrap_or_default().to_owned();
    Ok(Profile {
        identity: identity.to_owned(),
        user_id: field("id").ok_or(TsError::NotObject("Profile"))?.to_owned(),
        display_name: field("full_name").map(str::to_owned),
        email_hash: field("email").and_then(email_hash),
        created_at: created.clone(),
        updated_at: created,
    })
}

/// `AccessGroup` (`supabase.ts:191-206`), unchanged.
///
/// # Errors
///
/// When the JSON is not an `AccessGroup`.
pub fn access_group(v: &Value) -> Result<AccessGroup, TsError> {
    convert("AccessGroup", v, &[])
}

/// `GroupMember` (`supabase.ts:208-218`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `GroupMember`.
pub fn group_member(v: &Value) -> Result<GroupMember, TsError> {
    convert("GroupMember", v, &[])
}

/// `Verification` (`supabase.ts:128-151`): file URLs become object keys (P13).
///
/// # Errors
///
/// When the JSON is not a `Verification`.
pub fn verification(v: &Value) -> Result<Verification, TsError> {
    convert(
        "Verification",
        v,
        &[
            Edit::Rename("file_url", "object_key"),
            Edit::Rename("thumbnail_url", "thumbnail_key"),
        ],
    )
}

/// `Campaign` (`supabase.ts:1390-1415`): amounts to minor units.
///
/// # Errors
///
/// When the JSON is not a `Campaign` or an amount is not money.
pub fn campaign(v: &Value) -> Result<Campaign, TsError> {
    convert(
        "Campaign",
        v,
        &[Edit::Major("target_amount"), Edit::Major("current_amount")],
    )
}

/// `Donation` (`supabase.ts:1442-1469`): amounts to minor units.
///
/// # Errors
///
/// When the JSON is not a `Donation` or an amount is not money.
pub fn donation(v: &Value) -> Result<Donation, TsError> {
    convert(
        "Donation",
        v,
        &[Edit::Major("amount"), Edit::Major("amount_usd")],
    )
}

/// `DonorUpdate` (`supabase.ts:1476-1490`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `DonorUpdate`.
pub fn donor_update(v: &Value) -> Result<DonorUpdate, TsError> {
    convert("DonorUpdate", v, &[])
}

/// `updateOrganization`'s update (`api.ts:158-180`).
///
/// # Errors
///
/// When the JSON is not an update the module takes.
pub fn organization_patch(v: &Value) -> Result<OrganizationPatch, TsError> {
    convert("OrganizationProfileUpdate", v, &[])
}

/// `createAccessGroup`'s input (`api.ts:585-596`) less the organisation.
///
/// # Errors
///
/// When the JSON is not an access group input.
pub fn new_access_group(v: &Value) -> Result<NewAccessGroup, TsError> {
    convert("AccessGroup input", v, &[Edit::Drop("organization_id")])
}

/// `addGroupMember`'s input (`api.ts:654-658`) less `granted_by`, which is the caller.
///
/// # Errors
///
/// When the JSON is not a group member input.
pub fn new_group_member(v: &Value) -> Result<NewGroupMember, TsError> {
    convert("GroupMember input", v, &[Edit::Drop("granted_by")])
}

/// `createVerification`'s input (`api.ts:309-317`) less `created_by` and `verified_by`, which are
/// the caller; file URLs become object keys.
///
/// # Errors
///
/// When the JSON is not a verification input.
pub fn new_verification(v: &Value) -> Result<NewVerification, TsError> {
    convert(
        "Verification input",
        v,
        &[
            Edit::Drop("created_by"),
            Edit::Drop("verified_by"),
            Edit::Rename("file_url", "object_key"),
            Edit::Rename("thumbnail_url", "thumbnail_key"),
        ],
    )
}

/// `createCampaign`'s input (`api.ts:1080-1098`) less the organisation and `created_by`; amounts
/// to minor units.
///
/// # Errors
///
/// When the JSON is not a campaign input or an amount is not money.
pub fn new_campaign(v: &Value) -> Result<NewCampaign, TsError> {
    convert(
        "Campaign input",
        v,
        &[
            Edit::Drop("organization_id"),
            Edit::Drop("created_by"),
            Edit::Major("target_amount"),
        ],
    )
}

/// `updateCampaign`'s patch (`api.ts:1100-1114`); amounts to minor units.
///
/// # Errors
///
/// When the JSON is not a patch the module takes (`current_amount` is refused).
pub fn campaign_patch(v: &Value) -> Result<CampaignPatch, TsError> {
    convert("Campaign update", v, &[Edit::Major("target_amount")])
}

/// `createDonation`'s input (`api.ts:1256-1272`); amounts to minor units.
///
/// # Errors
///
/// When the JSON is not a donation input or an amount is not money.
pub fn new_donation(v: &Value) -> Result<NewDonation, TsError> {
    convert(
        "Donation input",
        v,
        &[Edit::Major("amount"), Edit::Major("amount_usd")],
    )
}

/// `createDonorUpdate`'s input (`api.ts:1329-1341`).
///
/// # Errors
///
/// When the JSON is not a donor update input.
pub fn new_donor_update(v: &Value) -> Result<NewDonorUpdate, TsError> {
    convert("DonorUpdate input", v, &[])
}

/// `FlowNode` (`supabase.ts:52-66`): the balance to minor units.
///
/// # Errors
///
/// When the JSON is not a `FlowNode` or the balance is not money.
pub fn flow_node(v: &Value) -> Result<FlowNode, TsError> {
    convert("FlowNode", v, &[Edit::Major("balance")])
}

/// `Transaction` (`supabase.ts:68-87`): amounts to minor units; no journal entries linked.
///
/// # Errors
///
/// When the JSON is not a `Transaction` or an amount is not money.
pub fn transaction(v: &Value) -> Result<Transaction, TsError> {
    convert(
        "Transaction",
        v,
        &[Edit::Major("amount"), Edit::Major("amount_usd")],
    )
}

/// `NodeVisibility` (`supabase.ts:225-235`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `NodeVisibility`.
pub fn node_visibility(v: &Value) -> Result<NodeVisibility, TsError> {
    convert("NodeVisibility", v, &[])
}

/// `TrustScoreHistory` (`supabase.ts:164-173`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `TrustScoreHistory`.
pub fn trust_history(v: &Value) -> Result<TrustHistory, TsError> {
    convert("TrustScoreHistory", v, &[])
}

/// `PublicEmbedConfig` (`supabase.ts:248-271`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `PublicEmbedConfig`.
pub fn embed_config(v: &Value) -> Result<PublicEmbedConfig, TsError> {
    convert("PublicEmbedConfig", v, &[])
}

/// `OrganizationChain` (`supabase.ts:1088-1105`), unchanged.
///
/// # Errors
///
/// When the JSON is not an `OrganizationChain`.
pub fn chain(v: &Value) -> Result<OrganizationChain, TsError> {
    convert("OrganizationChain", v, &[])
}

/// `ChainActivityLog` (`supabase.ts:1122-1130`), unchanged.
///
/// # Errors
///
/// When the JSON is not a `ChainActivityLog`.
pub fn chain_activity(v: &Value) -> Result<ChainActivity, TsError> {
    convert("ChainActivityLog", v, &[])
}

/// `createNode`'s input (`api.ts:264-274`) less the organisation.
///
/// # Errors
///
/// When the JSON is not a node input.
pub fn new_flow_node(v: &Value) -> Result<NewFlowNode, TsError> {
    convert("FlowNode input", v, &[Edit::Drop("organization_id")])
}

/// `createTransaction`'s input (`api.ts:220-238`) less the organisation and `created_by`;
/// amounts to minor units.
///
/// # Errors
///
/// When the JSON is not a transaction input or an amount is not money.
pub fn new_transaction(v: &Value) -> Result<NewTransaction, TsError> {
    convert(
        "Transaction input",
        v,
        &[
            Edit::Drop("organization_id"),
            Edit::Drop("created_by"),
            Edit::Major("amount"),
            Edit::Major("amount_usd"),
        ],
    )
}

/// `updateTransaction`'s update (`api.ts:240-254`); amounts to minor units.
///
/// # Errors
///
/// When the JSON is not an update the module takes.
pub fn transaction_patch(v: &Value) -> Result<TransactionPatch, TsError> {
    convert(
        "Transaction update",
        v,
        &[Edit::Major("amount"), Edit::Major("amount_usd")],
    )
}

/// `setNodeVisibility`'s settings (`api.ts:708-736`).
///
/// # Errors
///
/// When the JSON is not node visibility settings.
pub fn node_visibility_settings(v: &Value) -> Result<NodeVisibilitySettings, TsError> {
    convert("NodeVisibility settings", v, &[])
}

/// `updatePublicEmbedConfig`'s update (`api.ts:752-770`).
///
/// # Errors
///
/// When the JSON is not an embed config update.
pub fn embed_patch(v: &Value) -> Result<EmbedConfigPatch, TsError> {
    convert("PublicEmbedConfig update", v, &[])
}

/// Every chain of a demo-store-shaped dataset that the organisation is on either side of, with
/// their log entries (`organizationChains`, `chainActivityLog`).
///
/// # Errors
///
/// The first record that does not map.
pub fn chains(org_id: &str, store: &Value) -> Result<Chains, TsError> {
    let chains = list(store, "organizationChains")
        .map(chain)
        .filter(|c| {
            c.as_ref().map_or(true, |c| {
                c.source_org_id == org_id || c.target_org_id == org_id
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let log = list(store, "chainActivityLog")
        .map(chain_activity)
        .filter(|l| {
            l.as_ref()
                .map_or(true, |l| chains.iter().any(|c| c.id == l.chain_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Chains { chains, log })
}

fn list<'a>(store: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    store
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// One organisation's records from a demo-store-shaped dataset (`transparency-dataset.json#/store`,
/// the collections of `flowtrace/src/lib/demo/api.ts:49-68`): its organisation, its access groups
/// and their members, its campaigns, their donations and donor updates, and every verification
/// in the store (a TS verification names a transaction, not an organisation, so the dataset given
/// must be the organisation's own); and for MOD2 its flow nodes, transactions, their node
/// visibility, its trust history and embed config. Profiles are per identity and not part of it;
/// chains belong to two organisations ([`chains`]). The campaign figures are computed; the trust
/// row is left for the first write to compute.
///
/// # Errors
///
/// The first record that does not map.
pub fn dataset(org_id: &str, store: &Value) -> Result<OrgTransparency, TsError> {
    let in_org = |v: &&Value| v.get("organization_id").and_then(Value::as_str) == Some(org_id);
    let profile = list(store, "organizations")
        .find(|v| v.get("id").and_then(Value::as_str) == Some(org_id))
        .map(organization)
        .transpose()?;
    let access_groups = list(store, "accessGroups")
        .filter(in_org)
        .map(access_group)
        .collect::<Result<Vec<_>, _>>()?;
    let group_members = list(store, "groupMembers")
        .map(group_member)
        .filter(|m| {
            m.as_ref()
                .map_or(true, |m| access_groups.iter().any(|g| g.id == m.group_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let campaigns = list(store, "campaigns")
        .filter(in_org)
        .map(campaign)
        .collect::<Result<Vec<_>, _>>()?;
    let donations = list(store, "donations")
        .map(donation)
        .filter(|d| {
            d.as_ref()
                .map_or(true, |d| campaigns.iter().any(|c| c.id == d.campaign_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let donor_updates = list(store, "donorUpdates")
        .map(donor_update)
        .filter(|u| {
            u.as_ref().map_or(true, |u| {
                donations.iter().any(|d| d.id == u.donation_id)
                    || u.campaign_id
                        .as_ref()
                        .is_some_and(|c| campaigns.iter().any(|x| &x.id == c))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verifications = list(store, "verifications")
        .map(verification)
        .collect::<Result<Vec<_>, _>>()?;
    let campaign_stats = campaigns
        .iter()
        .map(|c| campaign_stat(c, &donations))
        .collect();
    let flow_nodes = list(store, "flowNodes")
        .filter(in_org)
        .map(flow_node)
        .collect::<Result<Vec<_>, _>>()?;
    let transactions = list(store, "transactions")
        .filter(in_org)
        .map(transaction)
        .collect::<Result<Vec<_>, _>>()?;
    let node_visibility = list(store, "nodeVisibility")
        .map(node_visibility)
        .filter(|v| {
            v.as_ref()
                .map_or(true, |v| flow_nodes.iter().any(|n| n.id == v.node_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let trust_history = list(store, "trustScoreHistory")
        .filter(in_org)
        .map(trust_history)
        .collect::<Result<Vec<_>, _>>()?;
    let embed = list(store, "publicEmbedConfigs")
        .find(in_org)
        .map(embed_config)
        .transpose()?;
    Ok(OrgTransparency {
        profile,
        access_groups,
        group_members,
        verifications,
        access_log: Vec::new(),
        campaigns,
        campaign_stats,
        donations,
        donor_updates,
        flow_nodes,
        transactions,
        node_visibility,
        transaction_visibility: Vec::new(),
        trust: None,
        trust_history,
        flags: Vec::new(),
        embed,
    })
}
