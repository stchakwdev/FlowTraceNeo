//! Journal imports from the books small NGOs already keep, mirroring
//! `flowtrace/src/lib/ledger/importers.ts` (`Docs/flowtrace-ledger-plan.md`, Phase 6).
//! QuickBooks Online "Journal" report exports and Xero manual-journal CSVs become draft entry
//! inputs; nothing posts until a person reviews them (`importers.ts:1-3`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::csv::{CsvRecords, parse_csv_records, parse_statement_date};
use crate::money::{is_js_whitespace, parse_money};
use crate::types::{
    Account, EntryInput, FundId, IsoDate, LedgerState, LineInput, Minor, Nullish, SourceType,
};

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

/// `detectJournalSource(headers)` (`importers.ts:19-24`), on lower-cased header names: Xero by
/// its account code, amount and narration columns, QuickBooks by an account column with a debit
/// or credit column.
#[must_use]
pub fn detect_journal_source(headers: &[String]) -> Option<JournalSource> {
    let has = |h: &str| headers.iter().any(|x| x == h);
    if has("accountcode") && has("amount") && has("narration") {
        Some(JournalSource::Xero)
    } else if has("account") && (has("debit") || has("credit")) {
        Some(JournalSource::Quickbooks)
    } else {
        None
    }
}

/// One journal being gathered (`importers.ts:47`).
struct Group {
    date: String,
    memo: String,
    lines: Vec<LineInput>,
}

/// A field of a record, empty when absent (`r[k] ?? ''`, and `r[k] || ...` reads empty alike).
fn field<'a>(r: &'a BTreeMap<String, String>, k: &str) -> &'a str {
    r.get(k).map_or("", String::as_str)
}

/// The first non-empty of several fields (`a || b || c`).
fn first<'a>(r: &'a BTreeMap<String, String>, keys: &[&str]) -> &'a str {
    keys.iter()
        .map(|k| field(r, k))
        .find(|v| !v.is_empty())
        .unwrap_or("")
}

/// `parseJournalCsv(text, state, fundId)` (`importers.ts:26-102`): reads a journal CSV into
/// draft entry inputs, every line in `fund_id`.
///
/// Rows group into journals: Xero by date and narration, its amount signed (positive debit);
/// QuickBooks by date and journal number, which it writes only on a journal's first row, so a
/// row with neither belongs to the journal above (`importers.ts:76-79`). Accounts resolve by
/// code, then by the code before the first space ("1110 Bank: Main operating account"), then by
/// name, then by name without a leading code, all case-insensitive; an account that does not
/// resolve is listed in `unmapped_accounts` (non-empty, in order of first sight) and its line
/// is dropped. A row with no account and no amount is skipped. A journal with no date becomes a
/// problem, not an entry; an unreadable amount is a problem naming the row (rows numbered from
/// 2). A file that is neither format gives one problem and no entries.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the rows, as parseJournalCsv (importers.ts:26-102)"
)]
pub fn parse_journal_csv(text: &str, state: &LedgerState, fund_id: &FundId) -> JournalImport {
    let CsvRecords { headers, records } = parse_csv_records(text);
    let source = detect_journal_source(&headers);
    let mut result = JournalImport {
        source: source.unwrap_or(JournalSource::Quickbooks),
        entries: Vec::new(),
        unmapped_accounts: Vec::new(),
        problems: Vec::new(),
    };
    let Some(source) = source else {
        result.problems.push(
            "This file is not a QuickBooks journal export or a Xero manual journal import."
                .to_owned(),
        );
        return result;
    };
    // `new Map(...)` keeps the last account for a repeated key.
    let lookup = |key: fn(&Account) -> &str, k: &str| -> Option<&Account> {
        state
            .accounts
            .iter()
            .rev()
            .find(|a| key(a).to_lowercase() == k)
    };
    let by_code = |k: &str| lookup(|a| &a.code, k);
    let by_name = |k: &str| lookup(|a| &a.name, k);
    let resolve = |raw: &str| -> Option<&Account> {
        let k = raw.trim_matches(is_js_whitespace).to_lowercase();
        let code = k.split(is_js_whitespace).next().unwrap_or("");
        by_code(&k)
            .or_else(|| by_code(code))
            .or_else(|| by_name(&k))
            .or_else(|| by_name(strip_leading_code(&k)))
    };
    let mut unmapped: Vec<String> = Vec::new();
    let mut groups: Vec<(String, Group)> = Vec::new();
    let mut last_key = String::new();

    for (i, r) in records.iter().enumerate() {
        let row_no = i + 2;
        let key: String;
        let date: Option<IsoDate>;
        let mut memo: String;
        let account_raw: &str;
        let (debit, credit): (Minor, Minor);
        if source == JournalSource::Xero {
            date = parse_statement_date(field(r, "date"));
            memo = first(r, &["narration"]).to_owned();
            if memo.is_empty() {
                memo = "Imported journal".to_owned();
            }
            key = format!("{}|{memo}", field(r, "date"));
            account_raw = field(r, "accountcode");
            let Some(amt) = parse_money(field(r, "amount")) else {
                result.problems.push(format!("Row {row_no}: no amount."));
                continue;
            };
            (debit, credit) = if amt > 0 { (amt, 0) } else { (0, -amt) };
        } else {
            let num = first(r, &["num", "no.", "journal no."]);
            date = parse_statement_date(field(r, "date"));
            key = if date.is_some() || !num.is_empty() {
                format!("{}|{num}", field(r, "date"))
            } else {
                last_key.clone()
            };
            memo = first(r, &["memo/description", "memo", "description", "name"]).to_owned();
            account_raw = field(r, "account");
            let amount = |k: &str| {
                let v = field(r, k);
                if v.is_empty() {
                    Some(0)
                } else {
                    parse_money(v)
                }
            };
            let (Some(d), Some(c)) = (amount("debit"), amount("credit")) else {
                result
                    .problems
                    .push(format!("Row {row_no}: unreadable amount."));
                continue;
            };
            (debit, credit) = (d, c);
        }
        if account_raw.is_empty() && debit == 0 && credit == 0 {
            continue;
        }
        last_key.clone_from(&key);
        let gi = if let Some(gi) = groups.iter().position(|(k, _)| *k == key) {
            gi
        } else {
            {
                groups.push((
                    key,
                    Group {
                        date: date
                            .as_ref()
                            .map(|d| d.as_str().to_owned())
                            .unwrap_or_default(),
                        memo: if memo.is_empty() {
                            "Imported journal".to_owned()
                        } else {
                            memo.clone()
                        },
                        lines: Vec::new(),
                    },
                ));
                groups.len() - 1
            }
        };
        let g = &mut groups[gi].1;
        if let Some(d) = date.as_ref().filter(|_| g.date.is_empty()) {
            d.as_str().clone_into(&mut g.date);
        }
        if memo.is_empty() {
            memo.clone_from(&g.memo);
        }
        let Some(account) = resolve(account_raw) else {
            if !unmapped.iter().any(|u| u == account_raw) {
                unmapped.push(account_raw.to_owned());
            }
            continue;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "safe integers, as the TS numbers"
        )]
        g.lines.push(LineInput {
            account_id: account.id.clone(),
            debit: Some(debit as f64),
            credit: Some(credit as f64),
            currency: None,
            fx_rate: None,
            fund_id: fund_id.clone(),
            grant_id: Nullish::Undefined,
            dimension_ids: None,
            flow_node_id: Nullish::Undefined,
            memo: Nullish::Value(memo),
        });
    }

    for (_, g) in groups {
        if g.date.is_empty() {
            result
                .problems
                .push(format!("\u{201c}{}\u{201d} has no date.", g.memo));
            continue;
        }
        result.entries.push(EntryInput {
            date: g.date,
            memo: g.memo,
            lines: g.lines,
            source_type: Some(SourceType::Import),
            source_id: Nullish::Undefined,
            override_reason: Nullish::Undefined,
        });
    }
    result.unmapped_accounts = unmapped.into_iter().filter(|u| !u.is_empty()).collect();
    result
}

/// `k.replace(/^\d+\s+/, '')` (`importers.ts:43`): drops a leading code and the spaces after it.
fn strip_leading_code(k: &str) -> &str {
    let digits = k.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &k[digits..];
    let trimmed = rest.trim_start_matches(is_js_whitespace);
    if digits > 0 && trimmed.len() < rest.len() {
        trimmed
    } else {
        k
    }
}
