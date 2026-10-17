//! The desktop side of the IT1 integration test (`rust/tools/it1.ts`,
//! `rust/docs/integration-test.md`): what the desktop's backend does, without a window, so a
//! script can drive it where the UI has no scriptable path. Prints one JSON document on stdout.
//!
//! ```sh
//! it1_probe send    --server U --database D --token-file F --org O --command '<json>'
//! it1_probe reports --server U --database D --token-file F --org O --from YYYY-MM-DD --to YYYY-MM-DD
//! it1_probe oracle  --log <path> --from YYYY-MM-DD --to YYYY-MM-DD
//! ```
//!
//! - `send`: connects as the token file's identity through `ledger_client` (the desktop's U10
//!   backend), runs one command with `SpacetimeClient::submit` and prints `{ok, result}` or
//!   `{ok: false, error, issues}`.
//! - `reports`: the five statements the Reports screen shows (trial balance, financial position,
//!   activities, functional expenses, cash flow; accrual basis), laid out by
//!   `desktop::statements::statement` from the books `ledger_client` rebuilds from the module rows.
//! - `oracle`: replays a run's command log through `ledger_core` from
//!   `rust/fixtures/demo-books.json#/state` (the parity suite's starting books,
//!   `crates/ledger-core/tests/replay.rs`), with the module's own `parse_command` and `apply`
//!   (`ledger_module::command`), checks every step's outcome against the module's, and prints
//!   the same five statements.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use desktop::statements::{Block, Cell, ReportKind, ReportParams, RowKind, Statement, statement};
use ledger_client::{ClientConfig, ClientError, ConnectionStatus, SpacetimeClient, TokenFile};
use ledger_core::reports::Basis;
use ledger_core::types::{Actor, IsoDate, LedgerState};
use ledger_core::{LedgerCommand, LedgerEnv, Timestamp};
use ledger_module::command::LEDGER_ERROR_PREFIX;
use serde_json::{Value, json};

/// The five reports IT1 compares, in the web picker's order (`REPORTS` in `statements.tsx`).
const REPORTS: [ReportKind; 5] = [
    ReportKind::FinancialPosition,
    ReportKind::Activities,
    ReportKind::FunctionalExpenses,
    ReportKind::CashFlow,
    ReportKind::TrialBalance,
];

/// How long `send` and `reports` wait for the books to load.
const READY_TIMEOUT: Duration = Duration::from_secs(60);

fn main() {
    match run(std::env::args().skip(1).collect()) {
        Ok(out) => println!("{out}"),
        Err(e) => {
            eprintln!("it1_probe: {e:#}");
            std::process::exit(2);
        }
    }
}

fn run(argv: Vec<String>) -> Result<Value> {
    let (sub, flags) = parse(argv)?;
    match sub.as_str() {
        "send" => send(&flags),
        "reports" => live_reports(&flags),
        "oracle" => oracle(&flags),
        other => bail!("unknown subcommand {other}: send, reports or oracle"),
    }
}

/// `sub --name value ...`.
fn parse(argv: Vec<String>) -> Result<(String, BTreeMap<String, String>)> {
    let mut it = argv.into_iter();
    let sub = it
        .next()
        .context("missing subcommand: send, reports or oracle")?;
    let mut flags = BTreeMap::new();
    while let Some(name) = it.next() {
        let key = name
            .strip_prefix("--")
            .with_context(|| format!("expected --flag, got {name}"))?;
        let value = it
            .next()
            .with_context(|| format!("--{key} needs a value"))?;
        flags.insert(key.to_owned(), value);
    }
    Ok((sub, flags))
}

fn flag<'a>(flags: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str> {
    flags
        .get(name)
        .map(String::as_str)
        .with_context(|| format!("--{name} is required"))
}

/// Connects as the token file's identity and waits until the organisation's books load.
fn connect(flags: &BTreeMap<String, String>) -> Result<(SpacetimeClient, String)> {
    let org = flag(flags, "org")?.to_owned();
    let config = ClientConfig::new(flag(flags, "server")?, flag(flags, "database")?)
        .with_token_file(TokenFile::new(PathBuf::from(flag(flags, "token-file")?)));
    let client = SpacetimeClient::connect(config)?;
    let started = Instant::now();
    loop {
        if let ConnectionStatus::Disconnected { error } = client.status() {
            bail!("disconnected: {}", error.unwrap_or_default());
        }
        if matches!(client.status(), ConnectionStatus::Ready { .. }) && client.state(&org).is_ok() {
            return Ok((client, org));
        }
        if started.elapsed() > READY_TIMEOUT {
            bail!("the books of {org} did not load in {READY_TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn send(flags: &BTreeMap<String, String>) -> Result<Value> {
    let raw: Value =
        serde_json::from_str(flag(flags, "command")?).context("--command is not JSON")?;
    let command: LedgerCommand =
        ledger_core::sanitize_command(&raw).map_err(|e| anyhow!("--command: {e:?}"))?;
    let (client, org) = connect(flags)?;
    let started = Instant::now();
    let out = match client.submit(&org, &command) {
        Ok(result) => json!({ "ok": true, "result": result }),
        Err(e) => {
            let issues = match &e {
                ClientError::Ledger(l) => serde_json::to_value(l.issues())?,
                _ => Value::Null,
            };
            json!({ "ok": false, "error": e.to_string(), "issues": issues })
        }
    };
    let ms = started.elapsed().as_millis();
    let identity = client.identity();
    client.disconnect();
    Ok(json!({ "identity": identity, "ms": ms, "outcome": out }))
}

fn live_reports(flags: &BTreeMap<String, String>) -> Result<Value> {
    let (client, org) = connect(flags)?;
    let state = client.state(&org)?;
    client.disconnect();
    reports(&state, flags)
}

/// The five statements as JSON: tables with their rows and cells (money in minor units), and
/// the footing checks.
fn reports(state: &LedgerState, flags: &BTreeMap<String, String>) -> Result<Value> {
    let from = IsoDate::parse(flag(flags, "from")?).map_err(|e| anyhow!("--from: {e}"))?;
    let to = IsoDate::parse(flag(flags, "to")?).map_err(|e| anyhow!("--to: {e}"))?;
    let mut out = serde_json::Map::new();
    for kind in REPORTS {
        let params = ReportParams {
            kind,
            from: from.clone(),
            to: to.clone(),
            basis: Basis::Accrual,
            grant: None,
        };
        let s = statement(state, &params).map_err(|e| anyhow!("{}: {e}", kind.id()))?;
        out.insert(kind.id().to_owned(), statement_json(&s));
    }
    Ok(Value::Object(out))
}

fn statement_json(s: &Statement) -> Value {
    let tables: Vec<Value> = s
        .tables()
        .map(|t| {
            let rows: Vec<Value> = t
                .rows
                .iter()
                .map(|r| {
                    let cells: Vec<Value> = r
                        .cells
                        .iter()
                        .map(|c| match c {
                            Cell::Money(m) => json!(m),
                            Cell::Text(text) => json!(text),
                        })
                        .collect();
                    json!({
                        "kind": match r.kind {
                            RowKind::Heading => "heading",
                            RowKind::Line => "line",
                            RowKind::Subtotal => "subtotal",
                            RowKind::Total => "total",
                            RowKind::Gap => "gap",
                        },
                        "code": r.code,
                        "label": r.label,
                        "detail": r.detail,
                        "cells": cells,
                    })
                })
                .collect();
            json!({
                "title": t.title,
                "first_column": t.first_column,
                "columns": t.columns,
                "rows": rows,
                "empty": t.empty,
            })
        })
        .collect();
    let footings: Vec<Value> = s
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Footing(f) => {
                Some(json!({ "statement": f.statement, "difference": f.difference }))
            }
            _ => None,
        })
        .collect();
    json!({ "title": s.title, "period": s.period, "currency": s.currency, "tables": tables, "footings": footings })
}

/// The oracle's ids (`it1-oracle-0001`, ...) and one frozen instant, the run's.
struct OracleEnv {
    now: Timestamp,
    drawn: u64,
}

impl LedgerEnv for OracleEnv {
    fn now(&self) -> Timestamp {
        self.now.clone()
    }

    fn new_id(&mut self) -> String {
        self.drawn += 1;
        format!("it1-oracle-{:04}", self.drawn)
    }
}

/// Module id -> oracle id, learned from each step's result.
#[derive(Default)]
struct IdMap(BTreeMap<String, String>);

impl IdMap {
    /// Every string in `v` that is a known module id becomes the oracle's.
    fn translate(&self, v: &Value) -> Value {
        match v {
            Value::String(s) => Value::String(self.0.get(s).cloned().unwrap_or_else(|| s.clone())),
            Value::Array(xs) => Value::Array(xs.iter().map(|x| self.translate(x)).collect()),
            Value::Object(m) => Value::Object(
                m.iter()
                    .map(|(k, x)| (k.clone(), self.translate(x)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Walks the module's result and the oracle's in step; differing strings at the same place
    /// are the same record under two ids.
    fn learn(&mut self, module: &Value, oracle: &Value) {
        match (module, oracle) {
            (Value::String(m), Value::String(o)) if m != o => {
                self.0.insert(m.clone(), o.clone());
            }
            (Value::Array(ms), Value::Array(os)) => {
                for (m, o) in ms.iter().zip(os) {
                    self.learn(m, o);
                }
            }
            (Value::Object(ms), Value::Object(os)) => {
                for (k, m) in ms {
                    if let Some(o) = os.get(k) {
                        self.learn(m, o);
                    }
                }
            }
            _ => {}
        }
    }
}

/// `demo-books.json#/state` with the demo organisation's id replaced by `org`, as the run
/// loaded it with `import_books` (`rust/tools/src/demo-replay.ts`, `demoFixtures`).
fn starting_books(org: &str) -> Result<LedgerState> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo-books.json");
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let books: Value = serde_json::from_str(&text)?;
    let demo = books["state"]["settings"]["organization_id"]
        .as_str()
        .context("demo-books.json has no organization_id")?;
    let replaced = text.replace(&json!(demo).to_string(), &json!(org).to_string());
    let books: Value = serde_json::from_str(&replaced)?;
    Ok(serde_json::from_value(books["state"].clone())?)
}

fn oracle(flags: &BTreeMap<String, String>) -> Result<Value> {
    let path = flag(flags, "log")?;
    let log: Value = serde_json::from_str(&std::fs::read_to_string(path).context("--log")?)?;
    let org = log["org"].as_str().context("log.org")?;
    let mut state = starting_books(org)?;
    let mut env = OracleEnv {
        now: Timestamp::parse(log["now"].as_str().context("log.now")?)
            .map_err(|e| anyhow!("log.now: {e}"))?,
        drawn: 0,
    };
    let actors: BTreeMap<String, Actor> = serde_json::from_value(log["actors"].clone())?;
    let mut ids = IdMap::default();
    let mut steps = Vec::new();
    for (i, step) in log["steps"]
        .as_array()
        .context("log.steps")?
        .iter()
        .enumerate()
    {
        let who = step["actor"].as_str().context("step.actor")?;
        let actor = actors
            .get(who)
            .with_context(|| format!("step {i}: no actor {who}"))?;
        let command = ids.translate(&step["command"]);
        let outcome = ledger_module::command::parse_command(&command.to_string())
            .and_then(|cmd| ledger_module::command::apply(&mut state, &mut env, actor, cmd));
        let module_ok = step["ok"].as_bool().unwrap_or(false);
        let (agrees, detail) = match (&outcome, module_ok) {
            (Ok(result), true) => {
                let result = serde_json::to_value(result)?;
                ids.learn(&step["result"], &result);
                (true, format!("ok {result}"))
            }
            // The module's text when the log has it, else its issues (a client decodes the text
            // into issues, `ledger_client::decode_reducer_error`), else any refusal agrees.
            (Err(e), false) => {
                let issues = e
                    .strip_prefix(LEDGER_ERROR_PREFIX)
                    .and_then(|j| serde_json::from_str::<Value>(j).ok());
                let same = match (step["error"].as_str(), step.get("issues")) {
                    (Some(text), _) => text == e,
                    (None, Some(want)) if !want.is_null() => issues.as_ref() == Some(want),
                    _ => true,
                };
                (same, format!("refused {e}"))
            }
            (Ok(result), false) => (
                false,
                format!("the oracle accepted it: {}", serde_json::to_value(result)?),
            ),
            (Err(e), true) => (false, format!("the oracle refused it: {e}")),
        };
        steps
            .push(json!({ "step": i, "label": step["label"], "agrees": agrees, "detail": detail }));
    }
    Ok(json!({ "steps": steps, "reports": reports(&state, flags)? }))
}
