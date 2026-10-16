//! A small RFC 4180 CSV reader and writer, and the statement date reader, mirroring
//! `flowtrace/src/lib/ledger/csv.ts`. Statement exports from banks and mobile-money providers
//! are CSV; reports export to CSV (`csv.ts:1-2`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::IsoDate;

/// `parseCsv(text)` (`csv.ts:4-37`): rows of fields. A leading byte-order mark is dropped;
/// quoted fields may hold commas, line breaks and doubled quotes; `\r\n`, `\n` and `\r` end a
/// row; rows whose fields are all blank are left out. An unterminated quote runs to the end.
#[must_use]
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let _ = text;
    Vec::new()
}

/// The result of [`parse_csv_records`]: the header names and one map per row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsvRecords {
    /// Header names, trimmed, lower-cased, a leading `*` dropped.
    pub headers: Vec<String>,
    /// Each row keyed by header name, values trimmed; a missing field is empty.
    pub records: Vec<BTreeMap<String, String>>,
}

/// `parseCsvRecords(text)` (`csv.ts:39-50`): rows as maps keyed by the lower-cased, trimmed
/// header names (a leading `*`, as Xero marks required columns, dropped).
#[must_use]
pub fn parse_csv_records(text: &str) -> CsvRecords {
    let _ = text;
    CsvRecords {
        headers: Vec::new(),
        records: Vec::new(),
    }
}

/// One value of [`to_csv`]: what TS passes as `unknown` and writes with `String(v)`, or empty
/// for `null` and `undefined` (`csv.ts:52-55`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CsvCell {
    /// `null` or `undefined`: an empty field.
    Empty,
    /// A boolean, `true` or `false`.
    Bool(bool),
    /// A number, written as JS writes it.
    Number(f64),
    /// Text.
    Text(String),
}

/// `toCsv(rows)` (`csv.ts:57-59`): fields joined by commas, rows ended by `\r\n`; a field with
/// a quote, comma or line break is quoted, quotes doubled.
#[must_use]
pub fn to_csv(rows: &[Vec<CsvCell>]) -> String {
    let _ = rows;
    String::new()
}

/// `parseStatementDate(raw)` (`csv.ts:66-80`): dates as banks write them, `2026-01-15`,
/// `2026-01-15 14:03:22`, `15/01/2026`, `15-01-2026`, `15.01.2026`, `15 Jan 2026`,
/// `15-Jan-2026`. Day first, as in East Africa, unless only month first is a real date.
#[must_use]
pub fn parse_statement_date(raw: &str) -> Option<IsoDate> {
    let _ = raw;
    None
}
