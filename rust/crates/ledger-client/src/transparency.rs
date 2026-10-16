//! The client's cache of the transparency views (MOD1) and the typed records rebuilt from it.
//!
//! The same data path as the books (pre-decision P7, ADR 0002): row callbacks only send
//! [`crate::cache::RowChange`]s, the connection thread applies a whole message's worth here and
//! then announces one [`crate::ClientEvent::TransparencyChanged`] naming the organisations it
//! touched. Rows are kept as the module's own row structs, and the records come back through the
//! module's own codec ([`ledger_module::transparency::rows::from_rows`]), so the client reads
//! rows exactly as the module wrote them.

use std::collections::{BTreeMap, BTreeSet};

use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::rows::{TransparencyRows, from_rows};
use ledger_module::transparency::tables::{
    AccessGroupMemberRow, AccessGroupRow, CampaignRow, CampaignStatRow, DonationRow,
    DonorUpdateRow, OrganizationProfileRow, VerificationAccessRow, VerificationRow,
};
use ledger_module::transparency::types::Profile;

use crate::cache::DocColumns;
use crate::error::ClientError;

/// The eight organisation views that are doc rows (`ledger_module::transparency::tables`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransparencyTable {
    /// `my_access_group`.
    AccessGroup,
    /// `my_access_group_member`.
    AccessGroupMember,
    /// `my_verification`, already cut to what the caller may see.
    Verification,
    /// `my_verification_access_log`, the caller's own looks.
    AccessLog,
    /// `my_campaign`.
    Campaign,
    /// `my_campaign_stat`.
    CampaignStat,
    /// `my_donation`.
    Donation,
    /// `my_donor_update`.
    DonorUpdate,
}

/// One row of a transparency view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransparencyRow {
    /// A doc row of an organisation view.
    Doc(TransparencyTable, DocColumns),
    /// `my_organization_profile`.
    OrgProfile {
        /// The organisation.
        org_id: String,
        /// `OrganizationProfile` as JSON.
        data: String,
    },
    /// `my_profile`.
    Profile {
        /// The person's identity, 64 hex digits.
        identity: String,
        /// `Profile` as JSON.
        data: String,
    },
}

/// What a row touched: an organisation's records, or the profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touched {
    /// This organisation's records.
    Org(String),
    /// The person profiles.
    Profiles,
}

/// One organisation's rows.
#[derive(Debug, Default)]
struct OrgCache {
    profile: Option<String>,
    docs: BTreeMap<TransparencyTable, BTreeMap<String, DocColumns>>,
}

impl OrgCache {
    fn is_empty(&self) -> bool {
        self.profile.is_none() && self.docs.values().all(BTreeMap::is_empty)
    }

    fn rows<R: ledger_module::tables::DocRow>(&self, table: TransparencyTable) -> Vec<R> {
        self.docs
            .get(&table)
            .map(|rows| {
                rows.values()
                    .map(|c| {
                        R::from_columns(
                            c.key.clone(),
                            c.org_id.clone(),
                            c.id.clone(),
                            c.seq,
                            c.data.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Every transparency row the caller can read.
#[derive(Debug, Default)]
pub struct TransparencyCache {
    orgs: BTreeMap<String, OrgCache>,
    profiles: BTreeMap<String, String>,
}

/// Puts `value` under `key`, or removes the value under `key` if it equals `value`; a delete
/// and an insert of one key in one transaction leave the new row whatever their order.
fn put<T: PartialEq>(map: &mut BTreeMap<String, T>, key: String, value: T, upsert: bool) {
    if upsert {
        map.insert(key, value);
    } else if map.get(&key) == Some(&value) {
        map.remove(&key);
    }
}

impl TransparencyCache {
    /// Applies one row change (`upsert` false for a delete) and says what it touched.
    pub fn apply(&mut self, row: TransparencyRow, upsert: bool) -> Touched {
        match row {
            TransparencyRow::Profile { identity, data } => {
                put(&mut self.profiles, identity, data, upsert);
                Touched::Profiles
            }
            TransparencyRow::OrgProfile { org_id, data } => {
                let org = self.orgs.entry(org_id.clone()).or_default();
                if upsert {
                    org.profile = Some(data);
                } else if org.profile.as_ref() == Some(&data) {
                    org.profile = None;
                }
                self.forget_if_empty(&org_id);
                Touched::Org(org_id)
            }
            TransparencyRow::Doc(table, columns) => {
                let org_id = columns.org_id.clone();
                let org = self.orgs.entry(org_id.clone()).or_default();
                put(
                    org.docs.entry(table).or_default(),
                    columns.key.clone(),
                    columns,
                    upsert,
                );
                self.forget_if_empty(&org_id);
                Touched::Org(org_id)
            }
        }
    }

    fn forget_if_empty(&mut self, org_id: &str) {
        if self.orgs.get(org_id).is_some_and(OrgCache::is_empty) {
            self.orgs.remove(org_id);
        }
    }

    /// Applies a batch: the organisations it touched, and whether profiles changed.
    pub fn apply_all(
        &mut self,
        changes: impl IntoIterator<Item = (TransparencyRow, bool)>,
    ) -> (BTreeSet<String>, bool) {
        let mut orgs = BTreeSet::new();
        let mut profiles = false;
        for (row, upsert) in changes {
            match self.apply(row, upsert) {
                Touched::Org(org) => {
                    orgs.insert(org);
                }
                Touched::Profiles => profiles = true,
            }
        }
        (orgs, profiles)
    }

    /// The organisations with any transparency row, in id order.
    #[must_use]
    pub fn organisations(&self) -> Vec<String> {
        self.orgs.keys().cloned().collect()
    }

    /// One organisation's records as the caller may read them; empty collections for an
    /// organisation with no rows yet.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read as its record.
    pub fn transparency(&self, org_id: &str) -> Result<OrgTransparency, ClientError> {
        let Some(org) = self.orgs.get(org_id) else {
            return Ok(OrgTransparency::default());
        };
        let rows = TransparencyRows {
            profile: org.profile.as_ref().map(|data| OrganizationProfileRow {
                org_id: org_id.to_owned(),
                data: data.clone(),
            }),
            access_groups: org.rows::<AccessGroupRow>(TransparencyTable::AccessGroup),
            group_members: org.rows::<AccessGroupMemberRow>(TransparencyTable::AccessGroupMember),
            verifications: org.rows::<VerificationRow>(TransparencyTable::Verification),
            access_log: org.rows::<VerificationAccessRow>(TransparencyTable::AccessLog),
            campaigns: org.rows::<CampaignRow>(TransparencyTable::Campaign),
            campaign_stats: org.rows::<CampaignStatRow>(TransparencyTable::CampaignStat),
            donations: org.rows::<DonationRow>(TransparencyTable::Donation),
            donor_updates: org.rows::<DonorUpdateRow>(TransparencyTable::DonorUpdate),
        };
        Ok(from_rows(&rows)?)
    }

    /// The profiles the caller can read (their own and their co-members'), in identity order.
    ///
    /// # Errors
    ///
    /// [`ClientError::Json`] when a profile does not read.
    pub fn profiles(&self) -> Result<Vec<Profile>, ClientError> {
        self.profiles
            .values()
            .map(|data| serde_json::from_str(data).map_err(Into::into))
            .collect()
    }
}

/// Every row of an organisation's records as upserts: what a fresh subscription delivers. Used
/// by the tests.
#[must_use]
pub fn org_rows_as_changes(org_id: &str, rows: TransparencyRows) -> Vec<(TransparencyRow, bool)> {
    fn docs<R: ledger_module::tables::DocRow>(
        table: TransparencyTable,
        rows: Vec<R>,
        org_id: &str,
    ) -> impl Iterator<Item = (TransparencyRow, bool)> {
        rows.into_iter().map(move |r| {
            let id = r
                .key()
                .strip_prefix(org_id)
                .and_then(|k| k.strip_prefix(':'))
                .unwrap_or_default()
                .to_owned();
            (
                TransparencyRow::Doc(table, DocColumns::of(&r, org_id, &id)),
                true,
            )
        })
    }
    let TransparencyRows {
        profile,
        access_groups,
        group_members,
        verifications,
        access_log,
        campaigns,
        campaign_stats,
        donations,
        donor_updates,
    } = rows;
    let mut out: Vec<(TransparencyRow, bool)> = profile
        .into_iter()
        .map(|p| {
            (
                TransparencyRow::OrgProfile {
                    org_id: p.org_id,
                    data: p.data,
                },
                true,
            )
        })
        .collect();
    out.extend(docs(TransparencyTable::AccessGroup, access_groups, org_id));
    out.extend(docs(
        TransparencyTable::AccessGroupMember,
        group_members,
        org_id,
    ));
    out.extend(docs(TransparencyTable::Verification, verifications, org_id));
    out.extend(docs(TransparencyTable::AccessLog, access_log, org_id));
    out.extend(docs(TransparencyTable::Campaign, campaigns, org_id));
    out.extend(docs(
        TransparencyTable::CampaignStat,
        campaign_stats,
        org_id,
    ));
    out.extend(docs(TransparencyTable::Donation, donations, org_id));
    out.extend(docs(TransparencyTable::DonorUpdate, donor_updates, org_id));
    out
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_module::transparency::rows::to_rows;
    use ledger_module::transparency::ts;
    use ledger_module::transparency::types::Campaign;

    /// `rust/fixtures/transparency-dataset.json`: the demo organisation's records as the module
    /// would hold them after `import_transparency`.
    fn demo() -> (String, OrgTransparency) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/transparency-dataset.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let org = fixture["demo_org_id"].as_str().unwrap().to_owned();
        let t = ts::dataset(&org, &fixture["store"]).unwrap();
        (org, t)
    }

    fn demo_changes() -> (String, OrgTransparency, Vec<(TransparencyRow, bool)>) {
        let (org, t) = demo();
        let changes = org_rows_as_changes(&org, to_rows(&org, &t).unwrap());
        (org, t, changes)
    }

    #[test]
    fn the_demo_records_rebuild_equal_from_their_rows() {
        let (org, t, changes) = demo_changes();
        let mut cache = TransparencyCache::default();
        let (touched, profiles) = cache.apply_all(changes);
        assert_eq!(
            (touched, profiles, cache.transparency(&org).unwrap()),
            (BTreeSet::from([org.clone()]), false, t)
        );
    }

    #[test]
    fn the_rebuild_does_not_depend_on_arrival_order() {
        let (org, t, mut changes) = demo_changes();
        changes.reverse();
        let mut cache = TransparencyCache::default();
        cache.apply_all(changes);
        assert_eq!(cache.transparency(&org).unwrap(), t);
    }

    #[test]
    fn an_update_replaces_the_campaign_under_its_key() {
        let (org, t, changes) = demo_changes();
        let mut cache = TransparencyCache::default();
        cache.apply_all(changes.clone());
        let Some((TransparencyRow::Doc(_, row), _)) = changes
            .iter()
            .find(|(r, _)| matches!(r, TransparencyRow::Doc(TransparencyTable::Campaign, _)))
            .cloned()
        else {
            panic!("no campaign row");
        };
        let mut renamed: Campaign = serde_json::from_str(&row.data).unwrap();
        renamed.name = "Renamed campaign".to_owned();
        cache.apply(
            TransparencyRow::Doc(
                TransparencyTable::Campaign,
                DocColumns {
                    data: serde_json::to_string(&renamed).unwrap(),
                    ..row
                },
            ),
            true,
        );
        let back = cache.transparency(&org).unwrap();
        assert_eq!(
            (back.campaigns.len(), back.campaigns.contains(&renamed)),
            (t.campaigns.len(), true)
        );
    }

    #[test]
    fn deleting_every_row_forgets_the_organisation() {
        let (org, _, changes) = demo_changes();
        let mut cache = TransparencyCache::default();
        cache.apply_all(changes.clone());
        cache.apply_all(changes.into_iter().map(|(r, _)| (r, false)));
        assert_eq!(
            (cache.organisations(), cache.transparency(&org).unwrap()),
            (Vec::<String>::new(), OrgTransparency::default())
        );
    }

    #[test]
    fn a_stale_delete_leaves_the_new_row() {
        let (org, _, changes) = demo_changes();
        let mut cache = TransparencyCache::default();
        let (TransparencyRow::Doc(table, old), _) = changes[1].clone() else {
            panic!("not a doc row");
        };
        let new = DocColumns {
            data: old.data.replace("Board of Directors", "Board"),
            ..old.clone()
        };
        cache.apply_all([
            (TransparencyRow::Doc(table, new.clone()), true),
            (TransparencyRow::Doc(table, old), false),
        ]);
        assert_eq!(
            cache.transparency(&org).unwrap().access_groups[0].name,
            "Board"
        );
    }

    #[test]
    fn profiles_are_their_own_batch() {
        let mut cache = TransparencyCache::default();
        let p = Profile {
            identity: "ab".repeat(32),
            user_id: "user_ama".to_owned(),
            display_name: Some("Ama".to_owned()),
            email_hash: ts::email_hash("ama@example.org"),
            created_at: "2026-02-04T09:00:00.000Z".to_owned(),
            updated_at: "2026-02-04T09:00:00.000Z".to_owned(),
        };
        let (orgs, profiles) = cache.apply_all([(
            TransparencyRow::Profile {
                identity: p.identity.clone(),
                data: serde_json::to_string(&p).unwrap(),
            },
            true,
        )]);
        assert_eq!(
            (orgs.len(), profiles, cache.profiles().unwrap()),
            (0, true, vec![p])
        );
    }

    #[test]
    fn a_broken_row_is_a_codec_error() {
        let (org, _, mut changes) = demo_changes();
        if let Some((TransparencyRow::Doc(_, c), _)) = changes.get_mut(2) {
            c.data = "{".to_owned();
        }
        let mut cache = TransparencyCache::default();
        cache.apply_all(changes);
        assert!(matches!(
            cache.transparency(&org),
            Err(ClientError::Codec(_))
        ));
    }
}
