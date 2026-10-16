//! What bills and expense claims share: coded lines, their accrual posting and payment
//! (`engine.ts:1002-1140`).

use serde::Deserialize;

use crate::types::{AccountId, DimensionId, FundId, GrantId};

/// A coded line of a new bill or claim (`Omit<CodedLine, 'id'>`, `types.ts:164-172`), with the
/// amount as the raw JS number the caller sent. The line id is drawn when the document is
/// stored; an `id` in the input is ignored, as TS overwrites it (`engine.ts:836`,
/// `engine.ts:927`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewCodedLine {
    /// What was bought.
    pub description: String,
    /// The account charged; it must exist.
    pub account_id: AccountId,
    /// Amount in the document's currency: whole minor units, more than zero.
    pub amount: f64,
    /// The fund charged; it must exist.
    pub fund_id: FundId,
    /// Grant charged, if any.
    pub grant_id: Option<GrantId>,
    /// Dimension values tagged.
    pub dimension_ids: Vec<DimensionId>,
}
