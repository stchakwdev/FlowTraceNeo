//! One organisation's transparency records to table rows and back, and the row writes that take
//! the tables from one set of records to another.
//!
//! The organisation tables are doc rows, so this reuses the ledger's doc codec
//! (`crate::codec::{doc_to_row, row_to_doc}`). Rows come back in `seq` order; a new record gets
//! the next `seq` after the table's highest for the organisation, a changed one keeps its `seq`.

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::OrgTransparency;
use super::chains::{ChainActivity, Chains, OrganizationChain};
use super::public::Projection;
use super::tables::{
    AccessGroupMemberRow, AccessGroupRow, CampaignRow, CampaignStatRow, ChainActivityRow, ChainRow,
    DonationRow, DonorUpdateRow, EmbedConfigRow, FlagRow, FlowNodeRow, NodeVisibilityRow,
    OrganizationProfileRow, PublicBadgeRow, PublicCampaignRow, PublicOrganizationRow,
    TransactionRow, TransactionVisibilityRow, TrustHistoryRow, TrustScoreRow,
    VerificationAccessRow, VerificationRow,
};
use super::types::OrganizationProfile;
use crate::codec::{CodecError, doc_to_row, row_key, row_to_doc};
use crate::tables::{Doc, DocRow};
use crate::write_back::TableOps;

/// One organisation's transparency rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransparencyRows {
    /// `organization_profile`.
    pub profile: Option<OrganizationProfileRow>,
    /// `access_group`.
    pub access_groups: Vec<AccessGroupRow>,
    /// `access_group_member`.
    pub group_members: Vec<AccessGroupMemberRow>,
    /// `verification`.
    pub verifications: Vec<VerificationRow>,
    /// `verification_access_log`.
    pub access_log: Vec<VerificationAccessRow>,
    /// `campaign`.
    pub campaigns: Vec<CampaignRow>,
    /// `campaign_stat`.
    pub campaign_stats: Vec<CampaignStatRow>,
    /// `donation`.
    pub donations: Vec<DonationRow>,
    /// `donor_update`.
    pub donor_updates: Vec<DonorUpdateRow>,
    /// `flow_node`.
    pub flow_nodes: Vec<FlowNodeRow>,
    /// `transaction`.
    pub transactions: Vec<TransactionRow>,
    /// `node_visibility`.
    pub node_visibility: Vec<NodeVisibilityRow>,
    /// `transaction_visibility`.
    pub transaction_visibility: Vec<TransactionVisibilityRow>,
    /// `trust_score`.
    pub trust: Option<TrustScoreRow>,
    /// `trust_score_history`.
    pub trust_history: Vec<TrustHistoryRow>,
    /// `flag`.
    pub flags: Vec<FlagRow>,
    /// `public_embed_config`.
    pub embed: Option<EmbedConfigRow>,
}

/// The writes that bring the rows to a new set of records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransparencyWrite {
    /// The profile row to insert or replace, when it changed.
    pub profile: Option<OrganizationProfileRow>,
    /// `access_group`.
    pub access_groups: TableOps<AccessGroupRow>,
    /// `access_group_member`.
    pub group_members: TableOps<AccessGroupMemberRow>,
    /// `verification`.
    pub verifications: TableOps<VerificationRow>,
    /// `verification_access_log`.
    pub access_log: TableOps<VerificationAccessRow>,
    /// `campaign`.
    pub campaigns: TableOps<CampaignRow>,
    /// `campaign_stat`.
    pub campaign_stats: TableOps<CampaignStatRow>,
    /// `donation`.
    pub donations: TableOps<DonationRow>,
    /// `donor_update`.
    pub donor_updates: TableOps<DonorUpdateRow>,
    /// `flow_node`.
    pub flow_nodes: TableOps<FlowNodeRow>,
    /// `transaction`.
    pub transactions: TableOps<TransactionRow>,
    /// `node_visibility`.
    pub node_visibility: TableOps<NodeVisibilityRow>,
    /// `transaction_visibility`.
    pub transaction_visibility: TableOps<TransactionVisibilityRow>,
    /// The trust row to insert or replace, when it changed.
    pub trust: Option<TrustScoreRow>,
    /// `trust_score_history`.
    pub trust_history: TableOps<TrustHistoryRow>,
    /// `flag`.
    pub flags: TableOps<FlagRow>,
    /// The embed config row to insert or replace, when it changed.
    pub embed: Option<EmbedConfigRow>,
}

/// The organisation profile as a row.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn profile_row(
    org_id: &str,
    profile: &OrganizationProfile,
) -> Result<OrganizationProfileRow, CodecError> {
    Ok(OrganizationProfileRow {
        org_id: org_id.to_owned(),
        data: serde_json::to_string(profile).map_err(|source| CodecError::Json {
            what: "organization_profile",
            source,
        })?,
    })
}

/// The organisation profile of a row.
///
/// # Errors
///
/// When `data` is not an `OrganizationProfile`.
pub fn profile_of(row: &OrganizationProfileRow) -> Result<OrganizationProfile, CodecError> {
    serde_json::from_str(&row.data).map_err(|source| CodecError::Json {
        what: "organization_profile",
        source,
    })
}

fn json_of<T: Serialize>(what: &'static str, record: &T) -> Result<String, CodecError> {
    serde_json::to_string(record).map_err(|source| CodecError::Json { what, source })
}

/// A record from its JSON.
///
/// # Errors
///
/// When `data` is not the record.
pub fn record_of<T: DeserializeOwned>(what: &'static str, data: &str) -> Result<T, CodecError> {
    serde_json::from_str(data).map_err(|source| CodecError::Json { what, source })
}

/// The trust score as a row.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn trust_row(
    org_id: &str,
    score: &super::trust::TrustScore,
) -> Result<TrustScoreRow, CodecError> {
    Ok(TrustScoreRow {
        org_id: org_id.to_owned(),
        data: json_of("trust_score", score)?,
    })
}

/// The embed config as a row.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn embed_row(
    org_id: &str,
    config: &super::flows::PublicEmbedConfig,
) -> Result<EmbedConfigRow, CodecError> {
    Ok(EmbedConfigRow {
        org_id: org_id.to_owned(),
        data: json_of("public_embed_config", config)?,
    })
}

fn docs<R: DocRow>(rows: &[R]) -> Result<Vec<R::Doc>, CodecError> {
    let mut sorted: Vec<&R> = rows.iter().collect();
    sorted.sort_by_key(|r| r.seq());
    sorted.into_iter().map(row_to_doc).collect()
}

fn to_doc_rows<R: DocRow>(org_id: &str, records: &[R::Doc]) -> Result<Vec<R>, CodecError> {
    (0_u64..)
        .zip(records)
        .map(|(seq, r)| doc_to_row(org_id, r, seq))
        .collect()
}

/// The records of an organisation's rows, each collection in `seq` order.
///
/// # Errors
///
/// The first row whose JSON is not its record.
pub fn from_rows(rows: &TransparencyRows) -> Result<OrgTransparency, CodecError> {
    Ok(OrgTransparency {
        profile: rows.profile.as_ref().map(profile_of).transpose()?,
        access_groups: docs(&rows.access_groups)?,
        group_members: docs(&rows.group_members)?,
        verifications: docs(&rows.verifications)?,
        access_log: docs(&rows.access_log)?,
        campaigns: docs(&rows.campaigns)?,
        campaign_stats: docs(&rows.campaign_stats)?,
        donations: docs(&rows.donations)?,
        donor_updates: docs(&rows.donor_updates)?,
        flow_nodes: docs(&rows.flow_nodes)?,
        transactions: docs(&rows.transactions)?,
        node_visibility: docs(&rows.node_visibility)?,
        transaction_visibility: docs(&rows.transaction_visibility)?,
        trust: rows
            .trust
            .as_ref()
            .map(|r| record_of("trust_score", &r.data))
            .transpose()?,
        trust_history: docs(&rows.trust_history)?,
        flags: docs(&rows.flags)?,
        embed: rows
            .embed
            .as_ref()
            .map(|r| record_of("public_embed_config", &r.data))
            .transpose()?,
    })
}

/// An organisation's records as rows numbered from 0, as a fresh import writes them.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn to_rows(org_id: &str, t: &OrgTransparency) -> Result<TransparencyRows, CodecError> {
    Ok(TransparencyRows {
        profile: t
            .profile
            .as_ref()
            .map(|p| profile_row(org_id, p))
            .transpose()?,
        access_groups: to_doc_rows(org_id, &t.access_groups)?,
        group_members: to_doc_rows(org_id, &t.group_members)?,
        verifications: to_doc_rows(org_id, &t.verifications)?,
        access_log: to_doc_rows(org_id, &t.access_log)?,
        campaigns: to_doc_rows(org_id, &t.campaigns)?,
        campaign_stats: to_doc_rows(org_id, &t.campaign_stats)?,
        donations: to_doc_rows(org_id, &t.donations)?,
        donor_updates: to_doc_rows(org_id, &t.donor_updates)?,
        flow_nodes: to_doc_rows(org_id, &t.flow_nodes)?,
        transactions: to_doc_rows(org_id, &t.transactions)?,
        node_visibility: to_doc_rows(org_id, &t.node_visibility)?,
        transaction_visibility: to_doc_rows(org_id, &t.transaction_visibility)?,
        trust: t.trust.as_ref().map(|s| trust_row(org_id, s)).transpose()?,
        trust_history: to_doc_rows(org_id, &t.trust_history)?,
        flags: to_doc_rows(org_id, &t.flags)?,
        embed: t.embed.as_ref().map(|c| embed_row(org_id, c)).transpose()?,
    })
}

/// The writes that take one table's rows to `after`: changed records keep their `seq`, new ones
/// are numbered on from the highest, and rows whose record is gone are deleted.
fn table_ops<R>(org_id: &str, rows: &[R], after: &[R::Doc]) -> Result<TableOps<R>, CodecError>
where
    R: DocRow,
    R::Doc: PartialEq,
{
    let before: HashMap<&str, &R> = rows.iter().map(|r| (r.key(), r)).collect();
    let mut next_seq = rows.iter().map(|r| r.seq() + 1).max().unwrap_or(0);
    let mut ops = TableOps::default();
    let mut kept = Vec::new();
    for record in after {
        let key = row_key(org_id, record.doc_id());
        if let Some(row) = before.get(key.as_str()) {
            if row_to_doc(*row).ok().as_ref() != Some(record) {
                ops.update.push(doc_to_row(org_id, record, row.seq())?);
            }
            kept.push(key);
        } else {
            ops.insert.push(doc_to_row(org_id, record, next_seq)?);
            next_seq += 1;
        }
    }
    ops.delete = rows
        .iter()
        .map(|r| r.key().to_owned())
        .filter(|k| !kept.contains(k))
        .collect();
    Ok(ops)
}

/// The writes that take an organisation's rows to `after`.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn plan_write(
    org_id: &str,
    rows: &TransparencyRows,
    after: &OrgTransparency,
) -> Result<TransparencyWrite, CodecError> {
    let before_profile = rows
        .profile
        .as_ref()
        .map(profile_of)
        .transpose()
        .ok()
        .flatten();
    let profile = match &after.profile {
        Some(p) if before_profile.as_ref() != Some(p) => Some(profile_row(org_id, p)?),
        _ => None,
    };
    let before_trust: Option<super::trust::TrustScore> = rows
        .trust
        .as_ref()
        .and_then(|r| record_of("trust_score", &r.data).ok());
    let trust = match &after.trust {
        Some(s) if before_trust.as_ref() != Some(s) => Some(trust_row(org_id, s)?),
        _ => None,
    };
    let before_embed: Option<super::flows::PublicEmbedConfig> = rows
        .embed
        .as_ref()
        .and_then(|r| record_of("public_embed_config", &r.data).ok());
    let embed = match &after.embed {
        Some(c) if before_embed.as_ref() != Some(c) => Some(embed_row(org_id, c)?),
        _ => None,
    };
    Ok(TransparencyWrite {
        profile,
        access_groups: table_ops(org_id, &rows.access_groups, &after.access_groups)?,
        group_members: table_ops(org_id, &rows.group_members, &after.group_members)?,
        verifications: table_ops(org_id, &rows.verifications, &after.verifications)?,
        access_log: table_ops(org_id, &rows.access_log, &after.access_log)?,
        campaigns: table_ops(org_id, &rows.campaigns, &after.campaigns)?,
        campaign_stats: table_ops(org_id, &rows.campaign_stats, &after.campaign_stats)?,
        donations: table_ops(org_id, &rows.donations, &after.donations)?,
        donor_updates: table_ops(org_id, &rows.donor_updates, &after.donor_updates)?,
        flow_nodes: table_ops(org_id, &rows.flow_nodes, &after.flow_nodes)?,
        transactions: table_ops(org_id, &rows.transactions, &after.transactions)?,
        node_visibility: table_ops(org_id, &rows.node_visibility, &after.node_visibility)?,
        transaction_visibility: table_ops(
            org_id,
            &rows.transaction_visibility,
            &after.transaction_visibility,
        )?,
        trust,
        trust_history: table_ops(org_id, &rows.trust_history, &after.trust_history)?,
        flags: table_ops(org_id, &rows.flags, &after.flags)?,
        embed,
    })
}

/// One table's writes applied to its rows, as the reducer applies them to the table.
fn apply_ops<R: DocRow + Clone>(rows: &mut Vec<R>, ops: TableOps<R>) {
    rows.retain(|r| !ops.delete.iter().any(|k| k == r.key()));
    for new in ops.update {
        if let Some(r) = rows.iter_mut().find(|r| r.key() == new.key()) {
            *r = new;
        }
    }
    rows.extend(ops.insert);
}

/// The writes applied to a copy of the rows, as the reducers apply them to the tables: what a
/// subscriber sees after the call.
pub fn apply_write(rows: &mut TransparencyRows, w: TransparencyWrite) {
    if let Some(p) = w.profile {
        rows.profile = Some(p);
    }
    apply_ops(&mut rows.access_groups, w.access_groups);
    apply_ops(&mut rows.group_members, w.group_members);
    apply_ops(&mut rows.verifications, w.verifications);
    apply_ops(&mut rows.access_log, w.access_log);
    apply_ops(&mut rows.campaigns, w.campaigns);
    apply_ops(&mut rows.campaign_stats, w.campaign_stats);
    apply_ops(&mut rows.donations, w.donations);
    apply_ops(&mut rows.donor_updates, w.donor_updates);
    apply_ops(&mut rows.flow_nodes, w.flow_nodes);
    apply_ops(&mut rows.transactions, w.transactions);
    apply_ops(&mut rows.node_visibility, w.node_visibility);
    apply_ops(&mut rows.transaction_visibility, w.transaction_visibility);
    if let Some(r) = w.trust {
        rows.trust = Some(r);
    }
    apply_ops(&mut rows.trust_history, w.trust_history);
    apply_ops(&mut rows.flags, w.flags);
    if let Some(r) = w.embed {
        rows.embed = Some(r);
    }
}

/// One single-row table's change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OneRow<R> {
    /// No change.
    Keep,
    /// Insert or replace this row.
    Put(R),
    /// Delete the row.
    Delete,
}

/// An organisation's published rows (`public_organization`, `public_badge`, `public_campaign`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PublicRows {
    /// `public_organization`.
    pub organization: Option<PublicOrganizationRow>,
    /// `public_badge`.
    pub badge: Option<PublicBadgeRow>,
    /// `public_campaign`.
    pub campaigns: Vec<PublicCampaignRow>,
}

/// The writes that bring an organisation's published rows to its [`Projection`].
#[derive(Debug, Clone, PartialEq)]
pub struct PublicWrite {
    /// `public_organization`.
    pub organization: OneRow<PublicOrganizationRow>,
    /// `public_badge`.
    pub badge: OneRow<PublicBadgeRow>,
    /// `public_campaign`, keyed by campaign id.
    pub campaigns: TableOps<PublicCampaignRow>,
}

impl PublicWrite {
    /// Nothing to write.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.organization == OneRow::Keep
            && self.badge == OneRow::Keep
            && self.campaigns.insert.is_empty()
            && self.campaigns.update.is_empty()
            && self.campaigns.delete.is_empty()
    }
}

fn one<R: PartialEq>(before: Option<&R>, after: Option<R>) -> OneRow<R> {
    match (before, after) {
        (None, None) => OneRow::Keep,
        (Some(_), None) => OneRow::Delete,
        (Some(b), Some(a)) if *b == a => OneRow::Keep,
        (_, Some(a)) => OneRow::Put(a),
    }
}

/// The published rows of a projection.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn public_rows(org_id: &str, p: &Projection) -> Result<PublicRows, CodecError> {
    Ok(PublicRows {
        organization: p
            .organization
            .as_ref()
            .map(|o| -> Result<_, CodecError> {
                Ok(PublicOrganizationRow {
                    org_id: org_id.to_owned(),
                    slug: o.public_slug.clone().unwrap_or_default(),
                    data: json_of("public_organization", o)?,
                })
            })
            .transpose()?,
        badge: p
            .badge
            .as_ref()
            .map(|b| -> Result<_, CodecError> {
                Ok(PublicBadgeRow {
                    org_id: org_id.to_owned(),
                    data: json_of("public_badge", b)?,
                })
            })
            .transpose()?,
        campaigns: p
            .campaigns
            .iter()
            .map(|c| {
                Ok(PublicCampaignRow {
                    campaign_id: c.campaign.id.clone(),
                    org_id: org_id.to_owned(),
                    slug: c.campaign.public_slug.clone().unwrap_or_default(),
                    data: json_of("public_campaign", c)?,
                })
            })
            .collect::<Result<_, CodecError>>()?,
    })
}

/// The writes that take an organisation's published rows to `after`.
#[must_use]
pub fn plan_public_write(before: &PublicRows, after: PublicRows) -> PublicWrite {
    let mut campaigns = TableOps::default();
    for row in &after.campaigns {
        match before
            .campaigns
            .iter()
            .find(|b| b.campaign_id == row.campaign_id)
        {
            Some(b) if b == row => {}
            Some(_) => campaigns.update.push(row.clone()),
            None => campaigns.insert.push(row.clone()),
        }
    }
    campaigns.delete = before
        .campaigns
        .iter()
        .filter(|b| {
            !after
                .campaigns
                .iter()
                .any(|a| a.campaign_id == b.campaign_id)
        })
        .map(|b| b.campaign_id.clone())
        .collect();
    PublicWrite {
        organization: one(before.organization.as_ref(), after.organization),
        badge: one(before.badge.as_ref(), after.badge),
        campaigns,
    }
}

/// The chain rows one call works on (`organization_chain`, `chain_activity_log`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChainRows {
    /// Chains.
    pub chains: Vec<ChainRow>,
    /// Their log entries.
    pub log: Vec<ChainActivityRow>,
}

/// The writes that take [`ChainRows`] to a new [`Chains`], keyed by id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChainWrite {
    /// `organization_chain`.
    pub chains: TableOps<ChainRow>,
    /// `chain_activity_log`.
    pub log: TableOps<ChainActivityRow>,
}

/// The chains and log entries of rows, each in `seq` order.
///
/// # Errors
///
/// The first row whose JSON is not its record.
pub fn chains_from_rows(rows: &ChainRows) -> Result<Chains, CodecError> {
    let mut chains: Vec<&ChainRow> = rows.chains.iter().collect();
    chains.sort_by_key(|r| r.seq);
    let mut log: Vec<&ChainActivityRow> = rows.log.iter().collect();
    log.sort_by_key(|r| r.seq);
    Ok(Chains {
        chains: chains
            .into_iter()
            .map(|r| record_of("organization_chain", &r.data))
            .collect::<Result<_, _>>()?,
        log: log
            .into_iter()
            .map(|r| record_of("chain_activity_log", &r.data))
            .collect::<Result<_, _>>()?,
    })
}

/// A chain as its row at `seq`.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn chain_row(c: &OrganizationChain, seq: u64) -> Result<ChainRow, CodecError> {
    Ok(ChainRow {
        id: c.id.clone(),
        source_org_id: c.source_org_id.clone(),
        target_org_id: c.target_org_id.clone(),
        seq,
        data: json_of("organization_chain", c)?,
    })
}

/// A log entry as its row at `seq`.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn chain_activity_row(a: &ChainActivity, seq: u64) -> Result<ChainActivityRow, CodecError> {
    Ok(ChainActivityRow {
        id: a.id.clone(),
        chain_id: a.chain_id.clone(),
        seq,
        data: json_of("chain_activity_log", a)?,
    })
}

/// The writes from `before` to `after`: changed records keep their `seq`, new ones are numbered
/// on from `next_chain_seq` and `next_log_seq` (one past the tables' highest), gone ones are
/// deleted.
///
/// # Errors
///
/// Never in practice: a record that does not serialise.
pub fn plan_chain_write(
    before: &ChainRows,
    after: &Chains,
    mut next_chain_seq: u64,
    mut next_log_seq: u64,
) -> Result<ChainWrite, CodecError> {
    let mut w = ChainWrite::default();
    for c in &after.chains {
        if let Some(r) = before.chains.iter().find(|r| r.id == c.id) {
            let row = chain_row(c, r.seq)?;
            if row != *r {
                w.chains.update.push(row);
            }
        } else {
            w.chains.insert.push(chain_row(c, next_chain_seq)?);
            next_chain_seq += 1;
        }
    }
    w.chains.delete = before
        .chains
        .iter()
        .filter(|r| !after.chains.iter().any(|c| c.id == r.id))
        .map(|r| r.id.clone())
        .collect();
    for a in &after.log {
        if let Some(r) = before.log.iter().find(|r| r.id == a.id) {
            let row = chain_activity_row(a, r.seq)?;
            if row != *r {
                w.log.update.push(row);
            }
        } else {
            w.log.insert.push(chain_activity_row(a, next_log_seq)?);
            next_log_seq += 1;
        }
    }
    w.log.delete = before
        .log
        .iter()
        .filter(|r| !after.log.iter().any(|a| a.id == r.id))
        .map(|r| r.id.clone())
        .collect();
    Ok(w)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::transparency::types::AccessGroup;
    use crate::transparency::types::{AccessGroupPermissions, NodeScope};

    fn group(id: &str, name: &str) -> AccessGroup {
        AccessGroup {
            id: id.to_owned(),
            organization_id: "org-1".to_owned(),
            name: name.to_owned(),
            slug: id.to_owned(),
            description: None,
            icon: "users".to_owned(),
            color: "#1C5FA8".to_owned(),
            permissions: AccessGroupPermissions::default(),
            node_scope: NodeScope::All,
            is_default: false,
            is_public: false,
            priority: 50,
            created_at: "2026-02-04T09:00:00.000Z".to_owned(),
            updated_at: "2026-02-04T09:00:00.000Z".to_owned(),
        }
    }

    fn org(groups: Vec<AccessGroup>) -> OrgTransparency {
        OrgTransparency {
            access_groups: groups,
            ..OrgTransparency::default()
        }
    }

    #[test]
    fn records_round_trip_through_rows_in_seq_order() {
        let t = org(vec![group("g-b", "B"), group("g-a", "A")]);
        let mut rows = to_rows("org-1", &t).unwrap();
        rows.access_groups.reverse();
        assert_eq!(from_rows(&rows).unwrap(), t);
    }

    #[test]
    fn an_unchanged_organisation_needs_no_write() {
        let t = org(vec![group("g-a", "A")]);
        let rows = to_rows("org-1", &t).unwrap();
        assert_eq!(
            plan_write("org-1", &rows, &t).unwrap(),
            TransparencyWrite::default()
        );
    }

    #[test]
    fn a_change_keeps_its_seq_a_new_record_takes_the_next_and_a_gone_one_is_deleted() {
        let rows = to_rows("org-1", &org(vec![group("g-a", "A"), group("g-b", "B")])).unwrap();
        let after = org(vec![group("g-a", "A2"), group("g-c", "C")]);
        let ops = plan_write("org-1", &rows, &after).unwrap().access_groups;
        assert_eq!(
            (
                ops.update
                    .iter()
                    .map(|r| (r.id.as_str(), r.seq))
                    .collect::<Vec<_>>(),
                ops.insert
                    .iter()
                    .map(|r| (r.id.as_str(), r.seq))
                    .collect::<Vec<_>>(),
                ops.delete
            ),
            (
                vec![("g-a", 0)],
                vec![("g-c", 2)],
                vec!["org-1:g-b".to_owned()]
            )
        );
    }
}
