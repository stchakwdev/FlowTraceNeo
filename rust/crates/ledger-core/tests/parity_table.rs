//! Gate G2: `rust/docs/parity.md` has exactly one row per export of the TS ledger modules and
//! per public method of the TS `Ledger` class, found by the grep that wrote the table: a line
//! `export function|const|let|class <name>` in each file, and in the body of
//! `export class Ledger` (`flowtrace/src/lib/ledger/engine.ts:103`) a method at two spaces of
//! indent that is neither `private` nor the constructor. The TS files are read, never written.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The TS modules the table covers.
const FILES: [&str; 11] = [
    "engine",
    "commands",
    "reports",
    "reconcile",
    "importers",
    "csv",
    "coa-template",
    "setup",
    "bridge",
    "money",
    "errors",
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// The identifier at the start of `s`.
fn ident(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

/// `(file, name)` for every export and public `Ledger` method (`Ledger.<method>`).
fn ts_exports() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in FILES {
        let file = format!("{f}.ts");
        let text =
            std::fs::read_to_string(repo().join("flowtrace/src/lib/ledger").join(&file)).unwrap();
        let mut in_ledger = false;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("export ") {
                let rest = rest.strip_prefix("async ").unwrap_or(rest);
                for kind in ["function ", "const ", "let ", "class "] {
                    if let Some(name) = rest.strip_prefix(kind) {
                        out.push((file.clone(), ident(name).to_owned()));
                    }
                }
                in_ledger = rest.starts_with("class Ledger ");
                continue;
            }
            if !in_ledger {
                continue;
            }
            if line.starts_with('}') {
                in_ledger = false;
            } else if let Some(rest) = line.strip_prefix("  ") {
                let name = ident(rest);
                let after = &rest[name.len()..];
                let method = !name.is_empty()
                    && name.starts_with(|c: char| c.is_ascii_alphabetic())
                    && (after.starts_with('(') || after.starts_with('<'));
                if method && name != "constructor" && name != "private" {
                    out.push((file.clone(), format!("Ledger.{name}")));
                }
            }
        }
    }
    out
}

/// `(file, name)` of every table row: the first two cells, `engine.ts:46` and `` `defaultEnv` ``.
fn table_rows() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(repo().join("rust/docs/parity.md")).unwrap();
    text.lines()
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let file = cells.get(1)?.split(':').next()?;
            let ts = std::path::Path::new(file)
                .extension()
                .is_some_and(|x| x == "ts");
            ts.then(|| (file.to_owned(), cells[2].trim_matches('`').to_owned()))
        })
        .collect()
}

#[test]
fn the_parity_table_lists_every_ts_export_once() {
    let exports = ts_exports();
    let rows = table_rows();
    let want: BTreeSet<_> = exports.iter().cloned().collect();
    let have: BTreeSet<_> = rows.iter().cloned().collect();
    assert_eq!(want.len(), exports.len(), "an export is found twice");
    assert_eq!(have.len(), rows.len(), "a row appears twice");
    let missing: Vec<_> = want.difference(&have).collect();
    let unknown: Vec<_> = have.difference(&want).collect();
    assert!(
        missing.is_empty() && unknown.is_empty(),
        "rows missing: {missing:?}; rows naming no TS export: {unknown:?}"
    );
    assert_eq!(rows.len(), 94, "the row count in parity.md and the report");
}
