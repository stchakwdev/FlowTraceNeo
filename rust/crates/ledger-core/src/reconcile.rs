//! Statement import and matching, mirroring `flowtrace/src/lib/ledger/reconcile.ts`. Bank CSVs
//! and mobile-money statements (M-Pesa, MTN MoMo, Airtel Money) become signed statement lines;
//! auto-match pairs each with a posted journal line on the same cash account by amount, a date
//! window and the reference (`reconcile.ts:1-4`).

use serde::{Deserialize, Serialize};

use crate::engine::{Error, Ledger, StatementRow};
use crate::types::{
    Actor, BankAccountId, IsoDate, JournalEntry, JournalLine, LedgerState, LineId, Minor,
    StatementLine, StatementLineId,
};

/// The default date window of [`candidates_for`] and [`auto_match`], in days
/// (`reconcile.ts:111`, `130`).
pub const DEFAULT_WINDOW_DAYS: u32 = 3;

/// `StatementFormat` (`reconcile.ts:10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementFormat {
    /// Safaricom M-Pesa.
    Mpesa,
    /// MTN `MoMo`.
    Mtn,
    /// Airtel Money.
    Airtel,
    /// Any other bank CSV.
    Bank,
}

/// `ParsedStatementRow` (`reconcile.ts:12-17`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedStatementRow {
    /// Value date.
    pub date: IsoDate,
    /// Statement text.
    pub description: String,
    /// Statement reference.
    pub reference: Option<String>,
    /// Signed amount in minor units: money in > 0, money out < 0.
    pub amount: Minor,
}

impl From<ParsedStatementRow> for StatementRow {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a safe integer, as the TS number"
    )]
    fn from(r: ParsedStatementRow) -> Self {
        Self {
            date: r.date,
            description: r.description,
            reference: r.reference,
            amount: r.amount as f64,
        }
    }
}

/// A row [`parse_statement`] skipped (`reconcile.ts:22`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedRow {
    /// 1-based CSV line number (the header is line 1).
    pub line: usize,
    /// Why.
    pub reason: String,
}

/// `ParseResult` (`reconcile.ts:19-23`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseResult {
    /// The detected format.
    pub format: StatementFormat,
    /// The rows read.
    pub rows: Vec<ParsedStatementRow>,
    /// Rows skipped, with the reason.
    pub skipped: Vec<SkippedRow>,
}

/// `detectFormat(headers)` (`reconcile.ts:33-39`), on lower-cased header names.
#[must_use]
pub fn detect_format(headers: &[String]) -> StatementFormat {
    let _ = headers;
    StatementFormat::Bank
}

/// `parseStatement(text)` (`reconcile.ts:41-92`).
#[must_use]
pub fn parse_statement(text: &str) -> ParseResult {
    let _ = text;
    ParseResult {
        format: StatementFormat::Bank,
        rows: Vec::new(),
        skipped: Vec::new(),
    }
}

/// `MatchCandidate` (`reconcile.ts:94-99`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    /// The journal line.
    pub line: JournalLine,
    /// Its entry.
    pub entry: JournalEntry,
    /// Days between the entry and statement dates.
    #[serde(with = "crate::js_number")]
    pub day_gap: f64,
    /// The statement reference appears in the entry's text.
    pub reference_match: bool,
}

/// `AutoMatchResult` (`reconcile.ts:101-105`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoMatchResult {
    /// Lines matched, one candidate each.
    pub matched: Vec<AutoMatched>,
    /// Lines with more than one candidate, left for a person.
    pub ambiguous: Vec<AmbiguousMatch>,
    /// Statement lines with no candidate.
    pub unmatched: Vec<StatementLineId>,
}

/// One automatic match (`reconcile.ts:102`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoMatched {
    /// The statement line.
    pub statement_line_id: StatementLineId,
    /// The journal line.
    pub journal_line_id: LineId,
}

/// A statement line with several candidates (`reconcile.ts:103`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguousMatch {
    /// The statement line.
    pub statement_line_id: StatementLineId,
    /// Its candidates, best first.
    pub candidates: Vec<MatchCandidate>,
}

/// `candidatesFor(ledger, s, windowDays, taken)` (`reconcile.ts:110-127`).
#[must_use]
pub fn candidates_for(
    state: &LedgerState,
    s: &StatementLine,
    window_days: u32,
    taken: &[LineId],
) -> Vec<MatchCandidate> {
    let _ = (state, s, window_days, taken);
    Vec::new()
}

/// `autoMatch(ledger, actor, bankAccountId, windowDays)` (`reconcile.ts:129-166`).
///
/// # Errors
///
/// Whatever [`Ledger::match_statement_line`] refuses.
pub fn auto_match(
    ledger: &mut Ledger<'_>,
    actor: &Actor,
    bank_account_id: &BankAccountId,
    window_days: u32,
) -> Result<AutoMatchResult, Error> {
    let _ = (ledger, actor, bank_account_id, window_days);
    Ok(AutoMatchResult {
        matched: Vec::new(),
        ambiguous: Vec::new(),
        unmatched: Vec::new(),
    })
}
