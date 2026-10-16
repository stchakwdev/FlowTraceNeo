//! Journal imports from the books small NGOs already keep, mirroring
//! `flowtrace/src/lib/ledger/importers.ts` (`Docs/flowtrace-ledger-plan.md`, Phase 6).
//! QuickBooks Online "Journal" report exports and Xero manual-journal CSVs become draft entry
//! inputs; nothing posts until a person reviews them (`importers.ts:1-3`).

use serde::{Deserialize, Serialize};

use crate::types::{EntryInput, FundId, LedgerState};

/// `JournalSource` (`importers.ts:9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalSource {
    /// A QuickBooks Online journal report export.
    Quickbooks,
    /// A Xero manual journal import file.
    Xero,
}

/// `JournalImport` (`importers.ts:11-17`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalImport {
    /// The detected source; `quickbooks` when the file is neither.
    pub source: JournalSource,
    /// Draft entry inputs, `source_type` `import`.
    pub entries: Vec<EntryInput>,
    /// Account names or codes in the file with no account in the chart.
    pub unmapped_accounts: Vec<String>,
    /// What could not be read.
    pub problems: Vec<String>,
}

/// `detectJournalSource(headers)` (`importers.ts:19-24`), on lower-cased header names.
#[must_use]
pub fn detect_journal_source(headers: &[String]) -> Option<JournalSource> {
    let _ = headers;
    None
}

/// `parseJournalCsv(text, state, fundId)` (`importers.ts:26-102`).
#[must_use]
pub fn parse_journal_csv(text: &str, state: &LedgerState, fund_id: &FundId) -> JournalImport {
    let _ = (text, state, fund_id);
    JournalImport {
        source: JournalSource::Quickbooks,
        entries: Vec::new(),
        unmapped_accounts: Vec::new(),
        problems: Vec::new(),
    }
}
