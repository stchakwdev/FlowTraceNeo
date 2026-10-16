//! `run_command` end to end on the host, with an in-memory row store in place of the tables:
//! every step of `rust/fixtures/commands-accepted.json` is loaded from rows, run through
//! [`crate::command`], diffed by [`crate::write_back::plan`] and written back, and the rows must
//! hold exactly the fixture's books after each step and at the end. The fixture env (ids
//! `fx-0001`, ..., frozen clock) stands in for the reducer's UUIDs and timestamp, so ids and
//! times compare exactly here; the local-server replay (`rust/tools/module-replay.ts`) covers the
//! real env.
//!
//! A command ledger-core cannot run yet (task C6) is replaced by its recorded delta, applied to
//! the books before the write-back, so the diff is still exercised and later steps stay in step
//! with the TS run.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::HashMap;

use ledger_core::env::LedgerEnv;
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::types::{Actor, LedgerState};
use serde_json::Value;

use crate::codec::{OrgRows, assemble_state, counters_json, settings_json, state_to_rows};
use crate::command::{apply, parse_command, result_json};
use crate::tables::{
    AccountRow, AuditEventRow, BankAccountRow, BillRow, BudgetLineRow, BudgetRow, DimensionRow,
    DocRow, ExpenseClaimRow, FiscalPeriodRow, FundRow, GrantRow, JournalEntryRow, JournalLineRow,
    PaymentRow, StatementLineRow, VendorRow,
};
use crate::write_back::{TableOps, WriteBack, plan};

/// The demo books' organisation (`demo-books.json#/state/settings/organization_id`).
const ORG: &str = "550e8400-e29b-41d4-a716-446655440000";

/// The commands ledger-core leaves to task C6 (`ledger_core::commands::apply_command`).
const C6_COMMANDS: [&str; 6] = [
    "bank.add",
    "statement.import",
    "statement.autoMatch",
    "statement.match",
    "statement.unmatch",
    "statement.post",
];

fn fixture(name: &str) -> Value {
    let path = format!("{}/../../fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A row with a primary key.
trait Keyed {
    fn row_key(&self) -> &str;
}

macro_rules! keyed {
    ($($row:ty),+) => {$(
        impl Keyed for $row {
            fn row_key(&self) -> &str {
                &self.key
            }
        }
    )+};
}

keyed!(
    JournalEntryRow,
    JournalLineRow,
    AccountRow,
    FundRow,
    GrantRow,
    DimensionRow,
    FiscalPeriodRow,
    BudgetRow,
    BudgetLineRow,
    VendorRow,
    BillRow,
    PaymentRow,
    ExpenseClaimRow,
    BankAccountRow,
    StatementLineRow,
    AuditEventRow
);

/// What the reducer does to one table: delete, update in place, insert, each by primary key.
fn write<R: Keyed>(table: &mut Vec<R>, ops: TableOps<R>) {
    table.retain(|r| !ops.delete.iter().any(|k| k == r.row_key()));
    for row in ops.update {
        let at = table
            .iter()
            .position(|r| r.row_key() == row.row_key())
            .unwrap();
        table[at] = row;
    }
    for row in ops.insert {
        assert!(
            table.iter().all(|r| r.row_key() != row.row_key()),
            "insert of a present key {}",
            row.row_key()
        );
        table.push(row);
    }
}

/// The organisation as the tables hold it.
struct Org {
    rows: OrgRows,
    settings_json: String,
    counters_json: String,
    next_seq: u64,
}

impl Org {
    fn import(state: &LedgerState) -> Self {
        let mut next_seq = 0;
        let rows = state_to_rows(ORG, state, &mut next_seq).unwrap();
        Self {
            rows,
            settings_json: settings_json(&state.settings).unwrap(),
            counters_json: counters_json(&state.counters).unwrap(),
            next_seq,
        }
    }

    fn books(&self) -> LedgerState {
        assemble_state(&self.settings_json, &self.counters_json, &self.rows).unwrap()
    }

    /// Applies a write-back and returns how many rows each collection wrote, as
    /// `(inserted, updated, deleted)` by `LedgerState` collection name.
    fn write(&mut self, wb: WriteBack) -> HashMap<&'static str, (usize, usize, usize)> {
        let mut counts = HashMap::new();
        macro_rules! tables {
            ($($name:literal => $field:ident),+) => {$(
                let ops = wb.$field;
                counts.insert($name, (ops.insert.len(), ops.update.len(), ops.delete.len()));
                write(&mut self.rows.$field, ops);
            )+};
        }
        tables!(
            "entries" => entries,
            "lines" => lines,
            "accounts" => accounts,
            "funds" => funds,
            "grants" => grants,
            "dimensions" => dimensions,
            "periods" => periods,
            "budgets" => budgets,
            "budgetLines" => budget_lines,
            "vendors" => vendors,
            "bills" => bills,
            "payments" => payments,
            "claims" => claims,
            "bankAccounts" => bank_accounts,
            "statementLines" => statement_lines,
            "audit" => audit
        );
        self.settings_json = wb.settings_json;
        self.counters_json = wb.counters_json;
        self.next_seq = wb.next_seq;
        counts
    }
}

/// `(added, changed, removed)` of one collection in a fixture delta.
fn delta_counts(delta: &Value, name: &str) -> (usize, usize, usize) {
    let len = |k: &str| {
        delta[name]
            .get(k)
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    };
    (len("added"), len("changed"), len("removed"))
}

/// A recorded delta applied to the books (`apply_delta`,
/// `rust/crates/ledger-core/tests/common/replay.rs`), for the shapes this fixture uses.
fn apply_delta(state: &LedgerState, delta: &Value) -> LedgerState {
    let mut books = serde_json::to_value(state).unwrap();
    for (name, d) in delta.as_object().unwrap() {
        if name == "counters" {
            for (k, v) in d.as_object().unwrap() {
                books["counters"][k] = v.clone();
            }
            continue;
        }
        assert!(
            d.get("order").is_none() && d.get("replaced").is_none(),
            "{name}"
        );
        let list = books[name].as_array_mut().unwrap();
        if let Some(removed) = d.get("removed").and_then(Value::as_array) {
            list.retain(|x| !removed.contains(&x["id"]));
        }
        for new in d
            .get("changed")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let old = list.iter_mut().find(|x| x["id"] == new["id"]).unwrap();
            *old = new.clone();
        }
        list.extend(
            d.get("added")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        );
    }
    serde_json::from_value(books).unwrap()
}

/// Moves the fixture env past every `fx-` id in `delta`, as the TS run drew them.
fn sync_env(env: &mut FixedEnv, delta: &Value) {
    fn highest(v: &Value) -> u64 {
        match v {
            Value::String(s) => s
                .strip_prefix("fx-")
                .filter(|d| d.len() == 4)
                .and_then(|d| d.parse().ok())
                .unwrap_or(0),
            Value::Array(xs) => xs.iter().map(highest).max().unwrap_or(0),
            Value::Object(m) => m.values().map(highest).max().unwrap_or(0),
            _ => 0,
        }
    }
    let top = highest(delta);
    while env.drawn() < top {
        env.new_id();
    }
}

/// The step's command text, as the client sends it (`JSON.stringify(cmd)`,
/// `flowtrace/src/lib/ledger/spacetime-backend.ts:212`).
fn command_text(step: &Value) -> String {
    serde_json::to_string(&step["command"]).unwrap()
}

/// What one step did.
struct Ran {
    /// The command's result as JSON, or `None` when the step was its recorded delta.
    result: Option<Value>,
    /// The books the engine (or the delta) left.
    after: LedgerState,
    /// Rows written per collection, as `(inserted, updated, deleted)`.
    written: HashMap<&'static str, (usize, usize, usize)>,
}

/// One fixture step the way `run_command` runs it: load the books from the rows, sanitise and
/// apply the command, plan the write-back and write it to the rows.
fn run_step(org: &mut Org, env: &mut FixedEnv, i: usize, step: &Value) -> Ran {
    let kind = step["command"]["type"].as_str().unwrap();
    let actor: Actor = serde_json::from_value(step["actor"].clone()).unwrap();
    let before = org.books();
    let mut after = before.clone();
    let cmd = parse_command(&command_text(step)).unwrap();
    let result = match apply(&mut after, env, &actor, cmd) {
        Ok(result) => Some(serde_json::from_str(&result_json(&result).unwrap()).unwrap()),
        Err(text) if C6_COMMANDS.contains(&kind) && text.ends_with("not implemented yet.") => {
            after = apply_delta(&before, &step["delta"]);
            sync_env(env, &step["delta"]);
            None
        }
        Err(text) => panic!("step {i} {kind}: {text}"),
    };
    let wb = plan(ORG, &org.rows, org.next_seq, &before, &after).unwrap();
    let written = org.write(wb);
    Ran {
        result,
        after,
        written,
    }
}

fn demo_org() -> Org {
    let demo: LedgerState = serde_json::from_value(fixture("demo-books")["state"].clone()).unwrap();
    Org::import(&demo)
}

#[test]
fn every_accepted_command_writes_exactly_the_fixture_books_back_to_the_rows() {
    let fx = fixture("commands-accepted");
    let mut org = demo_org();
    let mut env = fixed::fixture();
    let (mut live, mut delta_steps) = (Vec::new(), Vec::new());

    for (i, step) in fx["steps"].as_array().unwrap().iter().enumerate() {
        let kind = step["command"]["type"].as_str().unwrap();
        let ran = run_step(&mut org, &mut env, i, step);
        if let Some(result) = &ran.result {
            assert_eq!(result, &step["result"], "step {i} {kind}: result");
            live.push(kind);
        } else {
            delta_steps.push(kind);
        }
        for (name, written) in &ran.written {
            assert_eq!(
                *written,
                delta_counts(&step["delta"], name),
                "step {i} {kind}: rows written to {name}"
            );
        }
        assert_eq!(org.books(), ran.after, "step {i} {kind}: books read back");
    }

    let books = serde_json::to_value(org.books()).unwrap();
    assert_eq!(books, fx["final_state"], "final books");
    assert_eq!(live.len() + delta_steps.len(), 37);
    assert!(delta_steps.iter().all(|k| C6_COMMANDS.contains(k)));
}

#[test]
fn every_row_keeps_a_seq_of_its_own_below_next_seq() {
    let fx = fixture("commands-accepted");
    let mut org = demo_org();
    let mut env = fixed::fixture();
    let first = org.next_seq;
    let mut inserted = 0;
    for (i, step) in fx["steps"].as_array().unwrap().iter().enumerate() {
        inserted += run_step(&mut org, &mut env, i, step)
            .written
            .values()
            .map(|(ins, _, _)| ins)
            .sum::<usize>();
    }
    let r = &org.rows;
    let mut seqs: Vec<u64> = r.entries.iter().map(|x| x.seq).collect();
    seqs.extend(r.lines.iter().map(|x| x.seq));
    macro_rules! doc_seqs {
        ($($table:ident),+) => {$(
            seqs.extend(r.$table.iter().map(DocRow::seq));
        )+};
    }
    doc_seqs!(
        accounts,
        funds,
        grants,
        dimensions,
        periods,
        budgets,
        budget_lines,
        vendors,
        bills,
        payments,
        claims,
        bank_accounts,
        statement_lines,
        audit
    );
    let count = seqs.len();
    seqs.sort_unstable();
    seqs.dedup();
    assert_eq!(seqs.len(), count, "a seq is used twice");
    assert!(seqs.iter().all(|&s| s < org.next_seq));
    assert_eq!(org.next_seq, first + u64::try_from(inserted).unwrap());
}
