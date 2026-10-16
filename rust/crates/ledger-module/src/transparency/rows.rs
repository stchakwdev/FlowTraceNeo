//! One organisation's transparency records to table rows and back, and the row writes that take
//! the tables from one set of records to another.
//!
//! The organisation tables are doc rows, so this reuses the ledger's doc codec
//! (`crate::codec::{doc_to_row, row_to_doc}`). Rows come back in `seq` order; a new record gets
//! the next `seq` after the table's highest for the organisation, a changed one keeps its `seq`.

use std::collections::HashMap;

use super::OrgTransparency;
use super::tables::{
    AccessGroupMemberRow, AccessGroupRow, CampaignRow, CampaignStatRow, DonationRow,
    DonorUpdateRow, OrganizationProfileRow, VerificationAccessRow, VerificationRow,
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
