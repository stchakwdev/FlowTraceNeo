//! Statement import and matching, mirroring `flowtrace/src/lib/ledger/reconcile.ts`. Bank CSVs
//! and mobile-money statements (M-Pesa, MTN `MoMo`, Airtel Money) become signed statement lines;
//! auto-match pairs each with a posted journal line on the same cash account by amount, a date
//! window and the reference (`reconcile.ts:1-4`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::csv::{CsvRecords, parse_csv_records, parse_statement_date};
use crate::dates::day_number;
use crate::engine::{Error, Ledger, StatementRow};
use crate::money::parse_money;
use crate::types::{
    Actor, BankAccountId, EntryStatus, IsoDate, JournalEntry, JournalLine, LedgerState, LineId,
    MatchKind, Minor, StatementLine, StatementLineId,
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

/// `pick(r, names)` (`reconcile.ts:25-31`): the first named field that is not empty.
fn pick<'a>(r: &'a BTreeMap<String, String>, names: &[&str]) -> &'a str {
    names
        .iter()
        .filter_map(|n| r.get(*n))
        .find(|v| !v.is_empty())
        .map_or("", String::as_str)
}

/// `detectFormat(headers)` (`reconcile.ts:33-39`), on lower-cased header names: M-Pesa by its
/// receipt number or completion time and paid-in columns, MTN by its transaction ids, Airtel by
/// a transaction id with a transaction or service type, otherwise a bank.
#[must_use]
pub fn detect_format(headers: &[String]) -> StatementFormat {
    let has = |h: &str| headers.iter().any(|x| x == h);
    if has("receipt no.") || has("receipt no") || (has("completion time") && has("paid in")) {
        StatementFormat::Mpesa
    } else if has("external transaction id") || has("financial transaction id") {
        StatementFormat::Mtn
    } else if has("transaction id") && (has("transaction type") || has("service type")) {
        StatementFormat::Airtel
    } else {
        StatementFormat::Bank
    }
}

/// `parseStatement(text)` (`reconcile.ts:41-92`): reads a statement CSV. A row whose status is
/// not completed or successful is skipped, as is one with no readable date or no non-zero
/// amount. The amount comes from a pair of columns (paid in and withdrawn, credit and debit,
/// money in and money out: in minus out, both taken as absolute values) or else from one signed
/// column, made negative when a direction column says debit or out. Lines are numbered from 2,
/// the header being line 1.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the rows, as parseStatement (reconcile.ts:41-92)"
)]
pub fn parse_statement(text: &str) -> ParseResult {
    let CsvRecords { headers, records } = parse_csv_records(text);
    let format = detect_format(&headers);
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for (i, r) in records.iter().enumerate() {
        let line = i + 2;
        let mut skip = |reason: String| skipped.push(SkippedRow { line, reason });
        let status = pick(r, &["transaction status", "status"]).to_lowercase();
        if !status.is_empty() && !["completed", "success", "successful"].contains(&status.as_str())
        {
            skip(format!("status is {status}"));
            continue;
        }
        let Some(date) = parse_statement_date(pick(
            r,
            &[
                "completion time",
                "date",
                "transaction date",
                "value date",
                "posting date",
                "date time",
                "initiation time",
            ],
        )) else {
            skip("no readable date".to_owned());
            continue;
        };
        let signed = pick(r, &["amount", "transaction amount"]);
        let in_raw = pick(
            r,
            &["paid in", "credit", "money in", "deposits", "credit amount"],
        );
        let out_raw = pick(
            r,
            &[
                "withdrawn",
                "withdrawen",
                "debit",
                "money out",
                "withdrawals",
                "debit amount",
            ],
        );
        let money = |raw: &str| {
            if raw.is_empty() {
                Some(0)
            } else {
                parse_money(raw)
            }
        };
        let amount = if !in_raw.is_empty() || !out_raw.is_empty() {
            match (money(in_raw), money(out_raw)) {
                (Some(inn), Some(out)) => Some(inn.abs() - out.abs()),
                _ => None,
            }
        } else if signed.is_empty() {
            None
        } else {
            let dir = pick(r, &["direction", "dr/cr", "type"]).to_lowercase();
            parse_money(signed).map(|a| {
                if matches!(dir.as_str(), "dr" | "debit" | "out") {
                    -a.abs()
                } else {
                    a
                }
            })
        };
        let Some(amount) = amount.filter(|&a| a != 0) else {
            skip("no amount".to_owned());
            continue;
        };
        let description = pick(
            r,
            &[
                "details",
                "description",
                "narration",
                "narrative",
                "particulars",
                "transaction type",
                "memo",
            ],
        );
        let reference = pick(
            r,
            &[
                "receipt no.",
                "receipt no",
                "reference",
                "ref",
                "transaction id",
                "external transaction id",
                "financial transaction id",
                "cheque no",
                "reference number",
            ],
        );
        rows.push(ParsedStatementRow {
            date,
            description: if description.is_empty() {
                "Statement line".to_owned()
            } else {
                description.to_owned()
            },
            reference: (!reference.is_empty()).then(|| reference.to_owned()),
            amount,
        });
    }
    ParseResult {
        format,
        rows,
        skipped,
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

/// `normRef(s)` (`reconcile.ts:108`): upper-cased, letters and digits only.
fn norm_ref(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        .collect()
}

/// `candidatesFor(ledger, s, windowDays, taken)` (`reconcile.ts:110-127`): the posted journal
/// lines on the statement's GL account, not matched to any statement line nor in `taken`, with
/// the statement's signed amount, dated within `window_days` of it. A candidate's reference
/// matches when the statement reference, normalised, has at least four characters and appears
/// in the entry memo, line memo and entry number. Reference matches first, then by day gap;
/// ties keep the order of the lines. A statement line on an unknown bank account has none.
#[must_use]
pub fn candidates_for(
    state: &LedgerState,
    s: &StatementLine,
    window_days: u32,
    taken: &[LineId],
) -> Vec<MatchCandidate> {
    let Some(bank) = state
        .bank_accounts
        .iter()
        .find(|b| b.id == s.bank_account_id)
    else {
        return Vec::new();
    };
    let used = |id: &LineId| {
        taken.contains(id)
            || state
                .statement_lines
                .iter()
                .any(|x| x.matched_line_id.as_ref() == Some(id))
    };
    let reference = s.reference.as_deref().map(norm_ref).unwrap_or_default();
    let statement_day = day_number(&s.date);
    let mut out = Vec::new();
    for l in &state.lines {
        if l.account_id != bank.account_id || used(&l.id) || l.debit - l.credit != s.amount {
            continue;
        }
        let Some(entry) = state.entry(&l.entry_id) else {
            continue;
        };
        if entry.status == EntryStatus::Draft {
            continue;
        }
        let gap = (day_number(&entry.date) - statement_day).abs();
        if gap > i64::from(window_days) {
            continue;
        }
        let text = norm_ref(&format!(
            "{} {} {}",
            entry.memo,
            l.memo.as_deref().unwrap_or(""),
            entry.number
        ));
        #[expect(clippy::cast_precision_loss, reason = "a day count, as the TS number")]
        out.push(MatchCandidate {
            line: l.clone(),
            entry: entry.clone(),
            day_gap: gap as f64,
            reference_match: reference.len() >= 4 && text.contains(&reference),
        });
    }
    out.sort_by(|a, b| {
        b.reference_match
            .cmp(&a.reference_match)
            .then(a.day_gap.total_cmp(&b.day_gap))
    });
    out
}

/// `autoMatch(ledger, actor, bankAccountId, windowDays)` (`reconcile.ts:129-166`): matches
/// every unmatched statement line of the bank account it can, in two passes. First, a line
/// with exactly one reference-matching candidate is matched to it, so a same-amount line that
/// only matches on date cannot steal it; then a remaining line with exactly one candidate is
/// matched, one with several is ambiguous, one with none unmatched. Each match is
/// [`Ledger::match_statement_line`] with kind `auto` (one audit id each, in that order).
///
/// # Errors
///
/// Whatever [`Ledger::match_statement_line`] refuses, `PERMISSION` among them.
pub fn auto_match(
    ledger: &mut Ledger<'_>,
    actor: &Actor,
    bank_account_id: &BankAccountId,
    window_days: u32,
) -> Result<AutoMatchResult, Error> {
    let mut result = AutoMatchResult {
        matched: Vec::new(),
        ambiguous: Vec::new(),
        unmatched: Vec::new(),
    };
    let open: Vec<StatementLine> = ledger
        .state()
        .statement_lines
        .iter()
        .filter(|s| &s.bank_account_id == bank_account_id && s.matched_line_id.is_none())
        .cloned()
        .collect();
    let mut taken: Vec<LineId> = Vec::new();
    let mut pending = Vec::new();
    for s in open {
        let c = candidates_for(ledger.state(), &s, window_days, &taken);
        let mut by_ref = c.into_iter().filter(|x| x.reference_match);
        match (by_ref.next(), by_ref.next()) {
            (Some(only), None) => {
                let line = only.line.id;
                ledger.match_statement_line(actor, &s.id, &line, MatchKind::Auto)?;
                taken.push(line.clone());
                result.matched.push(AutoMatched {
                    statement_line_id: s.id,
                    journal_line_id: line,
                });
            }
            _ => pending.push(s),
        }
    }
    for s in pending {
        let mut c = candidates_for(ledger.state(), &s, window_days, &taken);
        if c.len() == 1 {
            let line = c.remove(0).line.id;
            ledger.match_statement_line(actor, &s.id, &line, MatchKind::Auto)?;
            taken.push(line.clone());
            result.matched.push(AutoMatched {
                statement_line_id: s.id,
                journal_line_id: line,
            });
        } else if c.is_empty() {
            result.unmatched.push(s.id);
        } else {
            result.ambiguous.push(AmbiguousMatch {
                statement_line_id: s.id,
                candidates: c,
            });
        }
    }
    Ok(result)
}
