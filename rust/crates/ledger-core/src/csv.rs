//! A small RFC 4180 CSV reader and writer, and the statement date reader, mirroring
//! `flowtrace/src/lib/ledger/csv.ts`. Statement exports from banks and mobile-money providers
//! are CSV; reports export to CSV (`csv.ts:1-2`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dates::days_in_month;
use crate::js_number::to_js_string;
use crate::money::is_js_whitespace;
use crate::types::IsoDate;

/// `String.prototype.trim`.
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// `parseCsv(text)` (`csv.ts:4-37`): rows of fields. A leading byte-order mark is dropped;
/// quoted fields may hold commas, line breaks and doubled quotes; `\r\n`, `\n` and `\r` end a
/// row; rows whose fields are all blank (after trimming) are left out. An unterminated quote
/// runs to the end of the text.
#[must_use]
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let src = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut chars = src.chars().peekable();
    let keep = |row: &[String]| row.iter().any(|f| !js_trim(f).is_empty());
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\n' | '\r' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                let done = std::mem::take(&mut row);
                if keep(&done) {
                    rows.push(done);
                }
            }
            c => field.push(c),
        }
    }
    row.push(field);
    if keep(&row) {
        rows.push(row);
    }
    rows
}

/// The result of [`parse_csv_records`]: the header names and one map per row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsvRecords {
    /// Header names, trimmed, lower-cased, a leading `*` dropped.
    pub headers: Vec<String>,
    /// Each row keyed by header name, values trimmed; a missing field is empty. With a repeated
    /// header the last column wins, as in TS.
    pub records: Vec<BTreeMap<String, String>>,
}

/// `parseCsvRecords(text)` (`csv.ts:39-50`): rows as maps keyed by the lower-cased, trimmed
/// header names (a leading `*`, as Xero marks required columns, dropped).
#[must_use]
pub fn parse_csv_records(text: &str) -> CsvRecords {
    let mut rows = parse_csv(text).into_iter();
    let Some(first) = rows.next() else {
        return CsvRecords {
            headers: Vec::new(),
            records: Vec::new(),
        };
    };
    let headers: Vec<String> = first
        .iter()
        .map(|h| {
            let h = js_trim(h).to_lowercase();
            h.strip_prefix('*').map(str::to_owned).unwrap_or(h)
        })
        .collect();
    let records = rows
        .map(|r| {
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    (
                        h.clone(),
                        js_trim(r.get(i).map_or("", String::as_str)).to_owned(),
                    )
                })
                .collect()
        })
        .collect();
    CsvRecords { headers, records }
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

impl From<&str> for CsvCell {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<String> for CsvCell {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<f64> for CsvCell {
    fn from(x: f64) -> Self {
        Self::Number(x)
    }
}

impl From<i64> for CsvCell {
    #[expect(clippy::cast_precision_loss, reason = "a JS number, as TS holds it")]
    fn from(x: i64) -> Self {
        Self::Number(x as f64)
    }
}

impl From<bool> for CsvCell {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl<T: Into<CsvCell>> From<Option<T>> for CsvCell {
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Empty, Into::into)
    }
}

/// `escape(v)` (`csv.ts:52-55`): `String(v)`, quoted with quotes doubled when it holds a quote,
/// a comma or a line break.
fn escape(v: &CsvCell) -> String {
    let s = match v {
        CsvCell::Empty => String::new(),
        CsvCell::Bool(b) => b.to_string(),
        CsvCell::Number(x) => to_js_string(*x),
        CsvCell::Text(s) => s.clone(),
    };
    if s.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// `toCsv(rows)` (`csv.ts:57-59`): fields joined by commas, rows joined by `\r\n`, and a final
/// `\r\n` (so no rows give `\r\n`).
#[must_use]
pub fn to_csv(rows: &[Vec<CsvCell>]) -> String {
    let lines: Vec<String> = rows
        .iter()
        .map(|r| r.iter().map(escape).collect::<Vec<_>>().join(","))
        .collect();
    lines.join("\r\n") + "\r\n"
}

/// `MONTHS` (`csv.ts:61-64`): month numbers by lower-case three-letter name.
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// A cursor over the ASCII patterns of `parseStatementDate`; `\d` in a JS regex is ASCII only.
struct Scan<'a> {
    rest: &'a str,
}

impl<'a> Scan<'a> {
    /// `\d{min,max}`, greedy.
    fn digits(&mut self, min: usize, max: usize) -> Option<i64> {
        let n = self
            .rest
            .bytes()
            .take(max)
            .take_while(u8::is_ascii_digit)
            .count();
        if n < min {
            return None;
        }
        let (d, rest) = self.rest.split_at(n);
        self.rest = rest;
        d.parse().ok()
    }

    /// One character satisfying `pred`.
    fn one(&mut self, pred: impl Fn(char) -> bool) -> Option<char> {
        let c = self.rest.chars().next().filter(|&c| pred(c))?;
        self.rest = &self.rest[c.len_utf8()..];
        Some(c)
    }

    /// `[A-Za-z]{3}[A-Za-z]*`: the first three letters.
    fn letters(&mut self) -> Option<&'a str> {
        let n = self
            .rest
            .bytes()
            .take_while(u8::is_ascii_alphabetic)
            .count();
        if n < 3 {
            return None;
        }
        let word = &self.rest[..3];
        self.rest = &self.rest[n..];
        Some(word)
    }
}

/// `parseStatementDate(raw)` (`csv.ts:66-80`): dates as banks write them, `2026-01-15`,
/// `2026-01-15 14:03:22`, `15/01/2026`, `15-01-2026`, `15.01.2026`, `15 Jan 2026`,
/// `15-Jan-2026`. Day first, as in East Africa, unless that is impossible and month first is
/// not (US-style exports such as QuickBooks write `01/15/2026`). Anything after the date is
/// ignored.
///
/// TS writes the year unpadded (`${y}`), so a year below 1000 gives a string that is not
/// `YYYY-MM-DD`; Rust returns `None` for it.
#[must_use]
pub fn parse_statement_date(raw: &str) -> Option<IsoDate> {
    let s = js_trim(raw);
    // ^(\d{4})-(\d{1,2})-(\d{1,2})
    let mut c = Scan { rest: s };
    if let Some(y) = c.digits(4, 4)
        && c.one(|x| x == '-').is_some()
        && let Some(m) = c.digits(1, 2)
        && c.one(|x| x == '-').is_some()
        && let Some(d) = c.digits(1, 2)
    {
        return iso(y, m, d);
    }
    // ^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})
    let sep = |x: char| matches!(x, '/' | '.' | '-');
    let mut c = Scan { rest: s };
    if let Some(a) = c.digits(1, 2)
        && c.one(sep).is_some()
        && let Some(b) = c.digits(1, 2)
        && c.one(sep).is_some()
        && let Some(y) = c.digits(4, 4)
    {
        return iso(y, b, a).or_else(|| if b > 12 { iso(y, a, b) } else { None });
    }
    // ^(\d{1,2})[\s-]([A-Za-z]{3})[A-Za-z]*[\s-](\d{4})
    let gap = |x: char| x == '-' || is_js_whitespace(x);
    let mut c = Scan { rest: s };
    if let Some(d) = c.digits(1, 2)
        && c.one(gap).is_some()
        && let Some(word) = c.letters()
        && c.one(gap).is_some()
        && let Some(y) = c.digits(4, 4)
    {
        let word = word.to_ascii_lowercase();
        let m = MONTHS.iter().position(|&m| m == word)?;
        return iso(y, i64::try_from(m).ok()? + 1, d);
    }
    None
}

/// `iso(y, mo, d)` (`csv.ts:82-87`): the date when it exists. `Date.UTC` reads a year from 0 to
/// 99 as 1900 to 1999, which only matters for leap days.
fn iso(y: i64, mo: i64, d: i64) -> Option<IsoDate> {
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let utc_year = if (0..100).contains(&y) { 1900 + y } else { y };
    if d > days_in_month(utc_year, mo) || y < 1000 {
        return None;
    }
    Some(IsoDate::from_parts(y, mo, d))
}
