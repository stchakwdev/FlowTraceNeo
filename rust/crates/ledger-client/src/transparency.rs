//! The client's cache of the transparency views (MOD1 and MOD2) and the public tables, and the
//! typed records rebuilt from it.
//!
//! The same data path as the books (pre-decision P7, ADR 0002): row callbacks only send
//! [`crate::cache::RowChange`]s, the connection thread applies a whole message's worth here and
//! then announces one [`crate::ClientEvent::TransparencyChanged`] naming the organisations it
//! touched. Rows are kept as the module's own row structs, and the records come back through the
//! module's own codec ([`ledger_module::transparency::rows::from_rows`]), so the client reads
//! rows exactly as the module wrote them.
//!
//! MOD2 adds six organisation views (flow nodes, transactions, node and transaction visibility,
//! trust history, flags), two one-per-organisation rows (trust score, embed config), the chains
//! with their log (both sides, as `ChainDetails`), the caller's own service row, and the three
//! public tables, which any connection reads, members or not, anonymous included.

use std::collections::{BTreeMap, BTreeSet};

use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::chains::{ChainActivity, ChainDetails, activity_of};
use ledger_module::transparency::public::{PublicBadge, PublicCampaign, PublicOrganization};
use ledger_module::transparency::rows::{TransparencyRows, from_rows, record_of};
use ledger_module::transparency::tables::{
    AccessGroupMemberRow, AccessGroupRow, CampaignRow, CampaignStatRow, DonationRow,
    DonorUpdateRow, EmbedConfigRow, FlagRow, FlowNodeRow, NodeVisibilityRow,
    OrganizationProfileRow, TransactionRow, TransactionVisibilityRow, TrustHistoryRow,
    TrustScoreRow, VerificationAccessRow, VerificationRow,
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
    /// `my_flow_node`.
    FlowNode,
    /// `my_transaction`.
    Transaction,
    /// `my_node_visibility`.
    NodeVisibility,
    /// `my_transaction_visibility`.
    TransactionVisibility,
    /// `my_trust_score_history`.
    TrustHistory,
    /// `my_flag`.
    Flag,
}

/// A chain row (`my_organization_chain`): `data` is its `ChainDetails`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainColumns {
    /// Chain id.
    pub id: String,
    /// The source organisation.
    pub source_org_id: String,
    /// The target organisation.
    pub target_org_id: String,
    /// Insertion order.
    pub seq: u64,
    /// `ChainDetails` as JSON.
    pub data: String,
}

/// A chain log row (`my_chain_activity_log`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainActivityColumns {
    /// Entry id.
    pub id: String,
    /// The chain.
    pub chain_id: String,
    /// Insertion order.
    pub seq: u64,
    /// `ChainActivity` as JSON.
    pub data: String,
}

/// The caller's own service registration (`my_service`, P17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    /// The service identity, 64 hex digits.
    pub identity: String,
    /// Its label.
    pub label: String,
    /// Its scopes (`flags`, `rubric`, `trust`).
    pub scopes: Vec<String>,
    /// When it was registered.
    pub registered_at: String,
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
    /// `my_trust_score`.
    TrustScore {
        /// The organisation.
        org_id: String,
        /// `TrustScore` as JSON.
        data: String,
    },
    /// `my_public_embed_config`.
    EmbedConfig {
        /// The organisation.
        org_id: String,
        /// `PublicEmbedConfig` as JSON.
        data: String,
    },
    /// `my_organization_chain`.
    Chain(ChainColumns),
    /// `my_chain_activity_log`.
    ChainActivity(ChainActivityColumns),
    /// `my_service`.
    Service(ServiceInfo),
    /// `public_organization`.
    PublicOrganization {
        /// The organisation.
        org_id: String,
        /// `PublicOrganization` as JSON.
        data: String,
    },
    /// `public_campaign`.
    PublicCampaign {
        /// The campaign.
        campaign_id: String,
        /// `PublicCampaign` as JSON.
        data: String,
    },
    /// `public_badge`.
    PublicBadge {
        /// The organisation.
        org_id: String,
        /// `PublicBadge` as JSON.
        data: String,
    },
}

/// What a row touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touched {
    /// This organisation's records.
    Org(String),
    /// The person profiles.
    Profiles,
    /// The chains or their log.
    Chains,
    /// The public tables.
    Public,
    /// The caller's service row.
    Service,
}

/// What one applied batch touched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per kind of read, as `ClientEvent::TransparencyChanged` names them"
)]
pub struct Batch {
    /// Organisations whose records changed.
    pub orgs: BTreeSet<String>,
    /// The person profiles changed.
    pub profiles: bool,
    /// The chains or their log changed.
    pub chains: bool,
    /// The public tables changed.
    pub public: bool,
    /// The caller's service row changed.
    pub service: bool,
}

/// One organisation's rows.
#[derive(Debug, Default)]
struct OrgCache {
    profile: Option<String>,
    trust: Option<String>,
    embed: Option<String>,
    docs: BTreeMap<TransparencyTable, BTreeMap<String, DocColumns>>,
}

/// Sets or clears a one-per-organisation row; a delete of a value no longer held is stale.
fn put_one(slot: &mut Option<String>, data: String, upsert: bool) {
    if upsert {
        *slot = Some(data);
    } else if slot.as_ref() == Some(&data) {
        *slot = None;
    }
}

impl OrgCache {
    fn is_empty(&self) -> bool {
        self.profile.is_none()
            && self.trust.is_none()
            && self.embed.is_none()
            && self.docs.values().all(BTreeMap::is_empty)
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
    chains: BTreeMap<String, ChainColumns>,
    chain_log: BTreeMap<String, ChainActivityColumns>,
    service: Option<ServiceInfo>,
    public_orgs: BTreeMap<String, String>,
    public_campaigns: BTreeMap<String, String>,
    public_badges: BTreeMap<String, String>,
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
                put_one(
                    &mut self.orgs.entry(org_id.clone()).or_default().profile,
                    data,
                    upsert,
                );
                self.forget_if_empty(&org_id);
                Touched::Org(org_id)
            }
            TransparencyRow::TrustScore { org_id, data } => {
                put_one(
                    &mut self.orgs.entry(org_id.clone()).or_default().trust,
                    data,
                    upsert,
                );
                self.forget_if_empty(&org_id);
                Touched::Org(org_id)
            }
            TransparencyRow::EmbedConfig { org_id, data } => {
                put_one(
                    &mut self.orgs.entry(org_id.clone()).or_default().embed,
                    data,
                    upsert,
                );
                self.forget_if_empty(&org_id);
                Touched::Org(org_id)
            }
            TransparencyRow::Chain(c) => {
                put(&mut self.chains, c.id.clone(), c, upsert);
                Touched::Chains
            }
            TransparencyRow::ChainActivity(a) => {
                put(&mut self.chain_log, a.id.clone(), a, upsert);
                Touched::Chains
            }
            TransparencyRow::Service(info) => {
                if upsert {
                    self.service = Some(info);
                } else if self.service.as_ref() == Some(&info) {
                    self.service = None;
                }
                Touched::Service
            }
            TransparencyRow::PublicOrganization { org_id, data } => {
                put(&mut self.public_orgs, org_id, data, upsert);
                Touched::Public
            }
            TransparencyRow::PublicCampaign { campaign_id, data } => {
                put(&mut self.public_campaigns, campaign_id, data, upsert);
                Touched::Public
            }
            TransparencyRow::PublicBadge { org_id, data } => {
                put(&mut self.public_badges, org_id, data, upsert);
                Touched::Public
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

    /// Applies a batch and says what it touched.
    pub fn apply_all(
        &mut self,
        changes: impl IntoIterator<Item = (TransparencyRow, bool)>,
    ) -> Batch {
        let mut b = Batch::default();
        for (row, upsert) in changes {
            match self.apply(row, upsert) {
                Touched::Org(org) => {
                    b.orgs.insert(org);
                }
                Touched::Profiles => b.profiles = true,
                Touched::Chains => b.chains = true,
                Touched::Public => b.public = true,
                Touched::Service => b.service = true,
            }
        }
        b
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
            flow_nodes: org.rows::<FlowNodeRow>(TransparencyTable::FlowNode),
            transactions: org.rows::<TransactionRow>(TransparencyTable::Transaction),
            node_visibility: org.rows::<NodeVisibilityRow>(TransparencyTable::NodeVisibility),
            transaction_visibility: org
                .rows::<TransactionVisibilityRow>(TransparencyTable::TransactionVisibility),
            trust: org.trust.as_ref().map(|data| TrustScoreRow {
                org_id: org_id.to_owned(),
                data: data.clone(),
            }),
            trust_history: org.rows::<TrustHistoryRow>(TransparencyTable::TrustHistory),
            flags: org.rows::<FlagRow>(TransparencyTable::Flag),
            embed: org.embed.as_ref().map(|data| EmbedConfigRow {
                org_id: org_id.to_owned(),
                data: data.clone(),
            }),
        };
        Ok(from_rows(&rows)?)
    }

    /// The chains of the caller's organisations, either side, in the order they were asked, each
    /// with both organisations' names and scores and the target's node and group.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn chains(&self) -> Result<Vec<ChainDetails>, ClientError> {
        let mut rows: Vec<&ChainColumns> = self.chains.values().collect();
        rows.sort_by_key(|c| c.seq);
        rows.into_iter()
            .map(|c| Ok(record_of("organization_chain", &c.data)?))
            .collect()
    }

    /// One chain's log, newest first (`getChainActivityLog`, `api.ts:992-998`).
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn chain_activity(&self, chain_id: &str) -> Result<Vec<ChainActivity>, ClientError> {
        let mut rows: Vec<&ChainActivityColumns> = self
            .chain_log
            .values()
            .filter(|a| a.chain_id == chain_id)
            .collect();
        rows.sort_by_key(|a| a.seq);
        let log = rows
            .into_iter()
            .map(|a| record_of("chain_activity_log", &a.data))
            .collect::<Result<Vec<ChainActivity>, _>>()?;
        Ok(activity_of(&log, chain_id))
    }

    /// The caller's own service registration, when the caller is a service.
    #[must_use]
    pub fn service(&self) -> Option<ServiceInfo> {
        self.service.clone()
    }

    /// Every published organisation, in id order.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_organizations(&self) -> Result<Vec<PublicOrganization>, ClientError> {
        public(&self.public_orgs, |d| record_of("public_organization", d))
    }

    /// Every published campaign, in campaign id order.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_campaigns(&self) -> Result<Vec<PublicCampaign>, ClientError> {
        public(&self.public_campaigns, |d| record_of("public_campaign", d))
    }

    /// Every published trust badge, in organisation id order.
    ///
    /// # Errors
    ///
    /// [`ClientError::Codec`] when a row does not read.
    pub fn public_badges(&self) -> Result<Vec<PublicBadge>, ClientError> {
        public(&self.public_badges, |d| record_of("public_badge", d))
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

fn public<T>(
    rows: &BTreeMap<String, String>,
    read: impl Fn(&str) -> Result<T, ledger_module::codec::CodecError>,
) -> Result<Vec<T>, ClientError> {
    rows.values().map(|data| Ok(read(data)?)).collect()
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
        flow_nodes,
        transactions,
        node_visibility,
        transaction_visibility,
        trust,
        trust_history,
        flags,
        embed,
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
    out.extend(docs(TransparencyTable::FlowNode, flow_nodes, org_id));
    out.extend(docs(TransparencyTable::Transaction, transactions, org_id));
    out.extend(docs(
        TransparencyTable::NodeVisibility,
        node_visibility,
        org_id,
    ));
    out.extend(docs(
        TransparencyTable::TransactionVisibility,
        transaction_visibility,
        org_id,
    ));
    out.extend(docs(TransparencyTable::TrustHistory, trust_history, org_id));
    out.extend(docs(TransparencyTable::Flag, flags, org_id));
    out.extend(trust.map(|r| {
        (
            TransparencyRow::TrustScore {
                org_id: r.org_id,
                data: r.data,
            },
            true,
        )
    }));
    out.extend(embed.map(|r| {
        (
            TransparencyRow::EmbedConfig {
                org_id: r.org_id,
                data: r.data,
            },
            true,
        )
    }));
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
    fn dataset() -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/transparency-dataset.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// The demo organisation's records, MOD2's included, with the trust row the first write
    /// computes.
    fn demo() -> (String, OrgTransparency) {
        let fixture = dataset();
        let org = fixture["demo_org_id"].as_str().unwrap().to_owned();
        let mut t = ts::dataset(&org, &fixture["store"]).unwrap();
        ledger_module::transparency::trust::refresh(
            &mut t,
            &org,
            "2026-02-04T09:00:00.000Z",
            "module",
        );
        (org, t)
    }

    /// The demo's chains as the module's view rows give them (`ChainDetails`), and their log.
    fn chain_changes() -> Vec<(TransparencyRow, bool)> {
        use ledger_module::transparency::chains::details_of;
        use ledger_module::transparency::rows::{ChainRows, plan_chain_write};
        let fixture = dataset();
        let (org, t) = demo();
        let c = ts::chains(&org, &fixture["store"]).unwrap();
        let w = plan_chain_write(&ChainRows::default(), &c, 0, 0).unwrap();
        let other = OrgTransparency::default();
        let side = |id: &str| if id == org { &t } else { &other };
        let mut out: Vec<(TransparencyRow, bool)> = w
            .chains
            .insert
            .into_iter()
            .zip(&c.chains)
            .map(|(r, chain)| {
                let details = details_of(
                    chain,
                    side(&chain.source_org_id),
                    side(&chain.target_org_id),
                );
                (
                    TransparencyRow::Chain(ChainColumns {
                        id: r.id,
                        source_org_id: r.source_org_id,
                        target_org_id: r.target_org_id,
                        seq: r.seq,
                        data: serde_json::to_string(&details).unwrap(),
                    }),
                    true,
                )
            })
            .collect();
        out.extend(w.log.insert.into_iter().map(|r| {
            (
                TransparencyRow::ChainActivity(ChainActivityColumns {
                    id: r.id,
                    chain_id: r.chain_id,
                    seq: r.seq,
                    data: r.data,
                }),
                true,
            )
        }));
        out
    }

    #[test]
    fn the_mod2_records_are_part_of_the_organisations_records() {
        let (org, t, changes) = demo_changes();
        let mut cache = TransparencyCache::default();
        cache.apply_all(changes);
        let back = cache.transparency(&org).unwrap();
        assert_eq!(
            (
                back.flow_nodes.len(),
                back.transactions.len(),
                back.node_visibility.len(),
                back.trust_history.len(),
                back.trust.as_ref().map(|s| s.score),
                back.embed.is_some()
            ),
            (22, 16, 1, 8, Some(77), true)
        );
        assert_eq!(back, t);
    }

    #[test]
    fn chains_read_in_asking_order_with_both_sides_and_their_log_newest_first() {
        let mut cache = TransparencyCache::default();
        let mut changes = chain_changes();
        changes.reverse();
        let b = cache.apply_all(changes);
        assert!(b.chains && b.orgs.is_empty());
        let chains = cache.chains().unwrap();
        assert_eq!(chains.len(), 4);
        assert!(
            chains
                .windows(2)
                .all(|w| w[0].chain.requested_at <= w[1].chain.requested_at)
        );
        assert_eq!(chains[0].source_org.name, "Demo NGO Kenya");
        assert_eq!(chains[0].source_org.trust_score, 77);
        let log = cache.chain_activity(&chains[0].chain.id).unwrap();
        assert_eq!(
            log.iter()
                .map(|l| format!("{:?}", l.action))
                .collect::<Vec<_>>(),
            ["Approved", "Requested"]
        );
    }

    #[test]
    fn public_rows_read_typed_and_say_so() {
        use ledger_module::transparency::public::project;
        use ledger_module::transparency::rows::public_rows;
        let (org, mut t) = demo();
        if let Some(p) = t.profile.as_mut() {
            p.is_public = true;
        }
        let rows = public_rows(&org, &project(&org, &t)).unwrap();
        let mut changes: Vec<(TransparencyRow, bool)> = Vec::new();
        if let Some(o) = rows.organization {
            changes.push((
                TransparencyRow::PublicOrganization {
                    org_id: o.org_id,
                    data: o.data,
                },
                true,
            ));
        }
        if let Some(b) = rows.badge {
            changes.push((
                TransparencyRow::PublicBadge {
                    org_id: b.org_id,
                    data: b.data,
                },
                true,
            ));
        }
        for c in rows.campaigns {
            changes.push((
                TransparencyRow::PublicCampaign {
                    campaign_id: c.campaign_id,
                    data: c.data,
                },
                true,
            ));
        }
        let mut cache = TransparencyCache::default();
        let b = cache.apply_all(changes.clone());
        assert!(b.public && b.orgs.is_empty());
        let expected = project(&org, &t);
        assert_eq!(
            (
                cache.public_organizations().unwrap(),
                cache.public_badges().unwrap(),
                cache.public_campaigns().unwrap().len()
            ),
            (
                vec![expected.organization.unwrap()],
                vec![expected.badge.unwrap()],
                expected.campaigns.len()
            )
        );
        cache.apply_all(changes.into_iter().map(|(r, _)| (r, false)));
        assert_eq!(cache.public_campaigns().unwrap(), []);
    }

    #[test]
    fn the_service_row_is_its_own_batch() {
        let mut cache = TransparencyCache::default();
        let info = ServiceInfo {
            identity: "cd".repeat(32),
            label: "jev-worker".to_owned(),
            scopes: vec!["flags".to_owned(), "trust".to_owned()],
            registered_at: "2026-02-04T09:00:00.000Z".to_owned(),
        };
        let b = cache.apply_all([(TransparencyRow::Service(info.clone()), true)]);
        assert!(b.service && !b.public && b.orgs.is_empty());
        assert_eq!(cache.service(), Some(info.clone()));
        cache.apply_all([(TransparencyRow::Service(info), false)]);
        assert_eq!(cache.service(), None);
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
        let b = cache.apply_all(changes);
        assert_eq!(
            (b.orgs, b.profiles, cache.transparency(&org).unwrap()),
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
        let b = cache.apply_all([(
            TransparencyRow::Profile {
                identity: p.identity.clone(),
                data: serde_json::to_string(&p).unwrap(),
            },
            true,
        )]);
        assert_eq!(
            (b.orgs.len(), b.profiles, cache.profiles().unwrap()),
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
