//! The trust score and the flags that feed it (MOD2; pre-decisions P15, P16).
//!
//! **The web's formula** is `calculateTrustScore` (`flowtrace/src/lib/supabase.ts:603-612`):
//! `Math.round(0.4 v + 0.3 d + 0.3 c)` over the verification rate `v` (verified transactions over
//! all, percent), the documentation rate `d` (transactions with a proof over all, percent) and the
//! consistency score `c` (the latest history row's, else 80), each as `calculateTrustScoreAPI`
//! takes them (`flowtrace/src/lib/demo/api.ts:406-472`). [`calculate_trust_score`] is it, float
//! for float.
//!
//! **The module's formula** (P16) adds the model component `m` (0 to 100):
//! `score = round(0.32 v + 0.24 d + 0.24 c + 0.20 m)`, `m = max(0, base - 5 k)` for `k` confirmed
//! flags ([`PENALTY_PER_FLAG`]). `base` is the mean of the rubric scores the model service last
//! stored ([`record_rubric`]); before it has scored, `base` is the web's own three-component score
//! `0.4 v + 0.3 d + 0.3 c`, so an unscored model is neutral: then `0.32 v + 0.24 d + 0.24 c +
//! 0.2 (0.4 v + 0.3 d + 0.3 c)` is the web's score exactly, and with no confirmed flags the module
//! takes it from [`calculate_trust_score`] so the two agree to the float. With `m = 100` the score
//! would be `0.8 × web + 20`, not the web's (the demo's 77 would read 82). Grade: `getTrustGrade`
//! (`supabase.ts:614-621`).
//!
//! **Flags** are what the model service found on a transaction, expense claim or bill: open until
//! a member who can approve spend dismisses one (with a reason) or confirms it; confirmed flags
//! count against `m`. The service only adds and refreshes open flags ([`record_flags`]); a resolved
//! flag never changes again.

use std::collections::BTreeMap;

use ledger_core::env::LedgerEnv;
use ledger_core::money::js_round;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrgTransparency;
use super::flows::{TransactionStatus, rate};
use super::plan::{Ctx, LedgerLinks, not_found};

/// The weights, in percent (P16).
pub const WEIGHTS: TrustWeights = TrustWeights {
    verification: 32,
    documentation: 24,
    consistency: 24,
    model: 20,
};

/// Points `m` loses per confirmed flag (the brief's default; floor 0).
pub const PENALTY_PER_FLAG: f64 = 5.0;

/// The consistency score when there is no history (`api.ts:428`).
pub const DEFAULT_CONSISTENCY: i64 = 80;

/// `calculateTrustScore` (`supabase.ts:603-612`): `Math.round` of the weighted float sum, in the
/// TS order of operations.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "a rounded score of rates within 0 to 100 fits i64"
)]
pub fn calculate_trust_score(verification: f64, documentation: f64, consistency: f64) -> i64 {
    js_round(verification * 0.4 + documentation * 0.3 + consistency * 0.3) as i64
}

/// `getTrustGrade` (`supabase.ts:614-621`).
#[must_use]
pub const fn trust_grade(score: i64) -> &'static str {
    match score {
        90.. => "A+",
        80..=89 => "A",
        70..=79 => "B",
        60..=69 => "C",
        50..=59 => "D",
        _ => "F",
    }
}

// ---------------------------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------------------------

/// The four weights in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustWeights {
    /// Verification rate.
    pub verification: u8,
    /// Documentation rate.
    pub documentation: u8,
    /// Consistency.
    pub consistency: u8,
    /// Model component.
    pub model: u8,
}

/// The four component values, 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrustComponents {
    /// Verified transactions over all, percent, rounded.
    pub verification: i64,
    /// Transactions with a proof over all, percent, rounded.
    pub documentation: i64,
    /// The latest history row's consistency, else 80.
    pub consistency: i64,
    /// `m`, unrounded.
    pub model: f64,
}

/// Where `m`'s base came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSource {
    /// The mean of the stored rubric scores.
    Rubric,
    /// No rubric yet: the web's three-component score, so the model does not move the total.
    Neutral,
}

/// What `m` was computed from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInputs {
    /// Rubric or neutral.
    pub source: ModelSource,
    /// The model service's rubric scores by question key, 0 to 100.
    pub rubric: BTreeMap<String, f64>,
    /// Their mean, when there are any.
    pub rubric_mean: Option<f64>,
    /// The model that scored them.
    pub model_version: Option<String>,
    /// When they were stored.
    pub scored_at: Option<String>,
    /// Confirmed flags (each costs [`PENALTY_PER_FLAG`]).
    pub confirmed_flags: u64,
    /// Open flags (no effect until resolved).
    pub open_flags: u64,
    /// [`PENALTY_PER_FLAG`].
    pub penalty_per_flag: f64,
}

/// The transaction counts behind the rates (`TrustScoreResult.details`, `supabase.ts:925-933`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustCounts {
    /// The organisation's transactions.
    pub total_transactions: u64,
    /// Of which verified.
    pub verified_transactions: u64,
    /// Of which with at least one proof.
    pub documented_transactions: u64,
}

/// The organisation's trust score, one per organisation (P16), kept current by every write that
/// changes an input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustScore {
    /// The organisation.
    pub organization_id: String,
    /// The total, 0 to 100.
    pub score: i64,
    /// `getTrustGrade` of it.
    pub grade: String,
    /// The four values.
    pub components: TrustComponents,
    /// The four weights.
    pub weights: TrustWeights,
    /// The four values rounded, by name, as the TS history `breakdown` plus `model`.
    pub breakdown: BTreeMap<String, i64>,
    /// What `m` came from.
    pub model: ModelInputs,
    /// The counts behind the rates.
    pub inputs: TrustCounts,
    /// When it was computed.
    pub computed_at: String,
    /// Who or what caused it: a member's user id, `service:<label>`, or `module` for an import.
    pub computed_by: String,
}

/// A snapshot of the score (`TrustScoreHistory`, `supabase.ts:164-173`), plus what the module adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustHistory {
    /// Row id.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// The total.
    pub score: i64,
    /// The rounded components by name.
    pub breakdown: BTreeMap<String, i64>,
    /// Verification rate.
    pub verification_rate: Option<i64>,
    /// Documentation rate.
    pub documentation_rate: Option<i64>,
    /// Consistency.
    pub consistency_score: Option<i64>,
    /// When.
    pub calculated_at: String,
    /// `m`; module addition, `None` on rows from the web.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_score: Option<f64>,
    /// The grade; module addition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
}

/// `TrustScoreResult.details` (`supabase.ts:925-933`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustDetails {
    /// The new score.
    pub trust_score: i64,
    /// Verification rate.
    pub verification_rate: i64,
    /// Documentation rate.
    pub documentation_rate: i64,
    /// Consistency.
    pub consistency_score: i64,
    /// Transactions.
    pub total_transactions: u64,
    /// Verified ones.
    pub verified_transactions: u64,
    /// Documented ones.
    pub documented_transactions: u64,
}

/// `TrustScoreResult` (`supabase.ts:919-934`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustResult {
    /// Always true.
    pub success: bool,
    /// The new score.
    pub trust_score: i64,
    /// The score of the latest snapshot before this one (0 for none).
    pub previous_score: i64,
    /// The change.
    pub delta: i64,
    /// The inputs.
    pub details: TrustDetails,
}

/// What a flag is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagSubject {
    /// A transaction of the organisation.
    Transaction,
    /// An expense claim of the books.
    Claim,
    /// A bill of the books.
    Bill,
}

/// Where a flag stands (P15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagState {
    /// Waiting for an approver.
    Open,
    /// Looked at and set aside, with a reason.
    Dismissed,
    /// Looked at and upheld; counts against the score.
    Confirmed,
}

/// How an approver resolves a flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// Set aside; needs a reason.
    Dismissed,
    /// Upheld.
    Confirmed,
}

/// One finding of the model service on one subject and question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flag {
    /// Flag id.
    pub id: String,
    /// The organisation.
    pub organization_id: String,
    /// Transaction, claim or bill.
    pub subject_type: FlagSubject,
    /// Its id.
    pub subject_id: String,
    /// The question asked, for example `duplicate_invoice`.
    pub question_key: String,
    /// The model's answer, as it gave it.
    pub answer: Value,
    /// The answer's probability, 0 to 1.
    pub probability: Option<f64>,
    /// The model's confidence, 0 to 1.
    pub confidence: Option<f64>,
    /// The model that answered.
    pub model_version: String,
    /// Open, dismissed or confirmed.
    pub state: FlagState,
    /// Why it was resolved so.
    pub reason: Option<String>,
    /// The service that raised it.
    pub raised_by: String,
    /// The approver's user id.
    pub resolved_by: Option<String>,
    /// When.
    pub resolved_at: Option<String>,
    /// When it was raised.
    pub created_at: String,
    /// When it last changed.
    pub updated_at: String,
}

/// One finding in a [`FlagBatch`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewFlag {
    /// Transaction, claim or bill.
    pub subject_type: FlagSubject,
    /// Its id.
    pub subject_id: String,
    /// The question.
    pub question_key: String,
    /// The answer.
    pub answer: Value,
    /// Probability, 0 to 1.
    #[serde(default)]
    pub probability: Option<f64>,
    /// Confidence, 0 to 1.
    #[serde(default)]
    pub confidence: Option<f64>,
}

/// `record_flags`'s argument: one model run's findings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagBatch {
    /// The model that answered.
    pub model_version: String,
    /// The findings.
    pub flags: Vec<NewFlag>,
}

/// `record_rubric`'s argument: one model run's rubric scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubricInput {
    /// The model that scored.
    pub model_version: String,
    /// Scores by question key, 0 to 100.
    pub scores: BTreeMap<String, f64>,
}

// ---------------------------------------------------------------------------------------------
// The formula
// ---------------------------------------------------------------------------------------------

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The transaction counts (`calculateTrustScoreAPI`, `api.ts:413-424`): every transaction, the
/// verified ones, and those with a proof of any status.
#[must_use]
pub fn counts(t: &OrgTransparency) -> TrustCounts {
    TrustCounts {
        total_transactions: count(t.transactions.len()),
        verified_transactions: count(
            t.transactions
                .iter()
                .filter(|x| x.verification_status == TransactionStatus::Verified)
                .count(),
        ),
        documented_transactions: count(
            t.transactions
                .iter()
                .filter(|x| {
                    t.verifications
                        .iter()
                        .any(|v| v.transaction_id.as_deref() == Some(x.id.as_str()))
                })
                .count(),
        ),
    }
}

/// The latest history row (`api.ts:425-428`: the greatest `calculated_at`, the first of equals).
fn latest(t: &OrgTransparency) -> Option<&TrustHistory> {
    t.trust_history
        .iter()
        .fold(None, |best: Option<&TrustHistory>, h| match best {
            Some(b) if b.calculated_at >= h.calculated_at => Some(b),
            _ => Some(h),
        })
}

/// The consistency input: the latest history row's, else 80 (`api.ts:428`).
#[must_use]
pub fn consistency(t: &OrgTransparency) -> i64 {
    latest(t)
        .and_then(|h| h.consistency_score)
        .unwrap_or(DEFAULT_CONSISTENCY)
}

/// The model inputs now: the stored rubric (from the current row) and the flags.
fn model_inputs(t: &OrgTransparency) -> ModelInputs {
    let (rubric, model_version, scored_at) = t.trust.as_ref().map_or_else(
        || (BTreeMap::new(), None, None),
        |s| {
            (
                s.model.rubric.clone(),
                s.model.model_version.clone(),
                s.model.scored_at.clone(),
            )
        },
    );
    let flags = |state: FlagState| count(t.flags.iter().filter(|f| f.state == state).count());
    #[expect(clippy::cast_precision_loss, reason = "a handful of rubric questions")]
    let rubric_mean =
        (!rubric.is_empty()).then(|| rubric.values().sum::<f64>() / rubric.len() as f64);
    ModelInputs {
        source: if rubric.is_empty() {
            ModelSource::Neutral
        } else {
            ModelSource::Rubric
        },
        rubric,
        rubric_mean,
        model_version,
        scored_at,
        confirmed_flags: flags(FlagState::Confirmed),
        open_flags: flags(FlagState::Open),
        penalty_per_flag: PENALTY_PER_FLAG,
    }
}

/// The module's formula (module docs) on the three web inputs, the rubric mean (`None` before the
/// model has scored) and the confirmed flags: `m` and the total.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "rates and flag counts are small; the rounded score fits i64"
)]
pub fn formula(
    verification: i64,
    documentation: i64,
    consistency: i64,
    rubric_mean: Option<f64>,
    confirmed_flags: u64,
) -> (f64, i64) {
    let (vf, df, cf) = (
        verification as f64,
        documentation as f64,
        consistency as f64,
    );
    let base = rubric_mean.unwrap_or(vf * 0.4 + df * 0.3 + cf * 0.3);
    let m = (base - PENALTY_PER_FLAG * confirmed_flags as f64).max(0.0);
    let score = if rubric_mean.is_none() && confirmed_flags == 0 {
        calculate_trust_score(vf, df, cf)
    } else {
        js_round(vf * 0.32 + df * 0.24 + cf * 0.24 + m * 0.2).clamp(0.0, 100.0) as i64
    };
    (m, score)
}

/// The score of an organisation's records now, stamped `at` by `by` (module docs: the formula).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "a rounded model value of 0 to 100 fits i64"
)]
pub fn compute(t: &OrgTransparency, org_id: &str, at: &str, by: &str) -> TrustScore {
    let inputs = counts(t);
    let verification = rate(inputs.verified_transactions, inputs.total_transactions);
    let documentation = rate(inputs.documented_transactions, inputs.total_transactions);
    let consistent = consistency(t);
    let model = model_inputs(t);
    let (model_value, score) = formula(
        verification,
        documentation,
        consistent,
        model.rubric_mean,
        model.confirmed_flags,
    );
    let breakdown = BTreeMap::from([
        ("verification".to_owned(), verification),
        ("documentation".to_owned(), documentation),
        ("consistency".to_owned(), consistent),
        ("model".to_owned(), js_round(model_value) as i64),
    ]);
    TrustScore {
        organization_id: org_id.to_owned(),
        score,
        grade: trust_grade(score).to_owned(),
        components: TrustComponents {
            verification,
            documentation,
            consistency: consistent,
            model: model_value,
        },
        weights: WEIGHTS,
        breakdown,
        model,
        inputs,
        computed_at: at.to_owned(),
        computed_by: by.to_owned(),
    }
}

/// The two rows equal but for when and by whom.
fn same_score(a: &TrustScore, b: &TrustScore) -> bool {
    TrustScore {
        computed_at: String::new(),
        computed_by: String::new(),
        ..a.clone()
    } == TrustScore {
        computed_at: String::new(),
        computed_by: String::new(),
        ..b.clone()
    }
}

/// The automatic recompute after a write: when an input changed (or there is no row yet), the row
/// is computed again; no history row is written (history rows are [`recompute`]'s snapshots, as
/// `calculateTrustScoreAPI` writes them). Says whether the row changed.
pub fn refresh(t: &mut OrgTransparency, org_id: &str, at: &str, by: &str) -> bool {
    let next = compute(t, org_id, at, by);
    if t.trust.as_ref().is_some_and(|s| same_score(s, &next)) {
        return false;
    }
    t.trust = Some(next);
    true
}

/// `calculateTrustScoreAPI` (`api.ts:406-472`) with the module's formula: the row computed again
/// and a history snapshot appended (id, then time, as the TS draws them). `previous_score` is the
/// latest snapshot's score (the TS reads the organisation's stored score, which only this call
/// writes, so the two agree), 0 with none. Who may call it is the reducer's check (any member or
/// a service with the `trust` scope).
pub fn recompute(
    t: &mut OrgTransparency,
    org_id: &str,
    env: &mut dyn LedgerEnv,
    by: &str,
) -> TrustResult {
    let previous_score = latest(t).map_or(0, |h| h.score);
    let id = env.new_id();
    let now = env.now().to_string();
    let row = compute(t, org_id, &now, by);
    let entry = TrustHistory {
        id,
        organization_id: org_id.to_owned(),
        score: row.score,
        breakdown: row.breakdown.clone(),
        verification_rate: Some(row.components.verification),
        documentation_rate: Some(row.components.documentation),
        consistency_score: Some(row.components.consistency),
        calculated_at: now,
        model_score: Some(row.components.model),
        grade: Some(row.grade.clone()),
    };
    t.trust_history.push(entry);
    let result = TrustResult {
        success: true,
        trust_score: row.score,
        previous_score,
        delta: row.score - previous_score,
        details: TrustDetails {
            trust_score: row.score,
            verification_rate: row.components.verification,
            documentation_rate: row.components.documentation,
            consistency_score: row.components.consistency,
            total_transactions: row.inputs.total_transactions,
            verified_transactions: row.inputs.verified_transactions,
            documented_transactions: row.inputs.documented_transactions,
        },
    };
    t.trust = Some(row);
    result
}

/// `getTrustScoreHistory` (`api.ts:397-404`): the latest `limit` snapshots, oldest first.
#[must_use]
pub fn history(t: &OrgTransparency, limit: usize) -> Vec<TrustHistory> {
    let mut rows: Vec<&TrustHistory> = t.trust_history.iter().collect();
    rows.sort_by(|a, b| b.calculated_at.cmp(&a.calculated_at));
    rows.truncate(limit);
    rows.reverse();
    rows.into_iter().cloned().collect()
}

// ---------------------------------------------------------------------------------------------
// Model service writes and flag resolution
// ---------------------------------------------------------------------------------------------

fn unit(name: &str, v: Option<f64>) -> Result<(), String> {
    match v {
        Some(x) if !(0.0..=1.0).contains(&x) => Err(format!("A flag {name} must be from 0 to 1.")),
        _ => Ok(()),
    }
}

fn subject_exists(t: &OrgTransparency, links: &dyn LedgerLinks, f: &NewFlag) -> bool {
    match f.subject_type {
        FlagSubject::Transaction => t.transactions.iter().any(|x| x.id == f.subject_id),
        FlagSubject::Claim => links.has_claim(&f.subject_id),
        FlagSubject::Bill => links.has_bill(&f.subject_id),
    }
}

/// One model run's findings (P14, P17; the caller is a registered service with the `flags`
/// scope, checked by the reducer): each finding is upserted by subject and question. A new one
/// opens a flag; an open one takes the new answer, probability, confidence and model; a dismissed
/// or confirmed one is left exactly as it is. Every subject must exist; probabilities and
/// confidences are 0 to 1. Returns the flags added or changed.
///
/// # Errors
///
/// No model version, a blank question, an unknown subject or a value out of range.
pub fn record_flags(
    t: &mut OrgTransparency,
    org_id: &str,
    env: &mut dyn LedgerEnv,
    links: &dyn LedgerLinks,
    service: &str,
    batch: FlagBatch,
) -> Result<Vec<Flag>, String> {
    let model_version = crate::rules::js_trim(&batch.model_version).to_owned();
    if model_version.is_empty() {
        return Err("A flag batch needs its model version.".to_owned());
    }
    for f in &batch.flags {
        if crate::rules::js_trim(&f.question_key).is_empty() {
            return Err("A flag needs its question key.".to_owned());
        }
        unit("probability", f.probability)?;
        unit("confidence", f.confidence)?;
        if !subject_exists(t, links, f) {
            return Err(format!(
                "{} {} not found.",
                subject_name(f.subject_type),
                f.subject_id
            ));
        }
    }
    let now = env.now().to_string();
    let mut written = Vec::new();
    for f in batch.flags {
        let same = |x: &Flag| {
            x.subject_type == f.subject_type
                && x.subject_id == f.subject_id
                && x.question_key == f.question_key
        };
        if let Some(x) = t.flags.iter_mut().find(|x| same(x)) {
            if x.state != FlagState::Open {
                continue;
            }
            let before = x.clone();
            x.answer = f.answer;
            x.probability = f.probability;
            x.confidence = f.confidence;
            x.model_version.clone_from(&model_version);
            if *x != before {
                x.updated_at.clone_from(&now);
                written.push(x.clone());
            }
        } else {
            let flag = Flag {
                id: env.new_id(),
                organization_id: org_id.to_owned(),
                subject_type: f.subject_type,
                subject_id: f.subject_id,
                question_key: f.question_key,
                answer: f.answer,
                probability: f.probability,
                confidence: f.confidence,
                model_version: model_version.clone(),
                state: FlagState::Open,
                reason: None,
                raised_by: service.to_owned(),
                resolved_by: None,
                resolved_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            t.flags.push(flag.clone());
            written.push(flag);
        }
    }
    Ok(written)
}

const fn subject_name(s: FlagSubject) -> &'static str {
    match s {
        FlagSubject::Transaction => "Transaction",
        FlagSubject::Claim => "Expense claim",
        FlagSubject::Bill => "Bill",
    }
}

/// Resolves an open flag (P15): only a member who can approve spend; dismissed needs a reason,
/// confirmed may have one. The reducer writes the audit event.
///
/// # Errors
///
/// Not an approver, no such flag, a flag already resolved, or a dismissal without a reason.
pub fn resolve_flag(
    t: &mut OrgTransparency,
    ctx: &mut Ctx<'_>,
    id: &str,
    resolution: Resolution,
    reason: Option<&str>,
) -> Result<Flag, String> {
    let user_id = ctx.approver("resolve flags")?.user_id.clone();
    let at = t
        .flags
        .iter()
        .position(|f| f.id == id)
        .ok_or_else(|| not_found("Flag"))?;
    let state = t.flags[at].state;
    if state != FlagState::Open {
        let word = if state == FlagState::Dismissed {
            "dismissed"
        } else {
            "confirmed"
        };
        return Err(format!("This flag is already {word}."));
    }
    let reason = reason
        .map(crate::rules::js_trim)
        .filter(|r| !r.is_empty())
        .map(str::to_owned);
    if resolution == Resolution::Dismissed && reason.is_none() {
        return Err("Say why the flag is dismissed.".to_owned());
    }
    let now = ctx.now();
    let f = &mut t.flags[at];
    f.state = match resolution {
        Resolution::Dismissed => FlagState::Dismissed,
        Resolution::Confirmed => FlagState::Confirmed,
    };
    f.reason = reason;
    f.resolved_by = Some(user_id);
    f.resolved_at = Some(now.clone());
    f.updated_at = now;
    Ok(f.clone())
}

/// Stores one model run's rubric scores (P16; the caller is a registered service with the
/// `rubric` scope, checked by the reducer) and computes the row again: the scores replace the
/// previous run's whole.
///
/// # Errors
///
/// No model version, no scores, a blank key, or a score that is not 0 to 100.
pub fn record_rubric(
    t: &mut OrgTransparency,
    org_id: &str,
    now: &str,
    service: &str,
    input: RubricInput,
) -> Result<TrustScore, String> {
    let model_version = crate::rules::js_trim(&input.model_version).to_owned();
    if model_version.is_empty() {
        return Err("A rubric needs its model version.".to_owned());
    }
    if input.scores.is_empty() {
        return Err("A rubric needs at least one score.".to_owned());
    }
    for (key, score) in &input.scores {
        if crate::rules::js_trim(key).is_empty() {
            return Err("A rubric score needs its question key.".to_owned());
        }
        if !(0.0..=100.0).contains(score) {
            return Err(format!("Rubric score {key} must be from 0 to 100."));
        }
    }
    let mut row = compute(t, org_id, now, service);
    row.model.rubric = input.scores;
    row.model.model_version = Some(model_version);
    row.model.scored_at = Some(now.to_owned());
    t.trust = Some(row);
    let next = compute(t, org_id, now, service);
    t.trust = Some(next.clone());
    Ok(next)
}
