//! Replaying fixture steps through the engine.
//!
//! A step whose method has an arm in [`call`] (or [`query`]) runs live through `ledger_core`, and
//! its return value and state delta must equal the fixture's. Any other step belongs to a later
//! task: its recorded delta is applied to the books directly and the env skips the ids it drew,
//! so the books stay in step with the TS run. Flipping a method from delta to live is one arm in
//! [`call`]; the replay tests count both kinds.

use ledger_core::engine::{EntryOptions, Error, Ledger};
use ledger_core::env::LedgerEnv;
use ledger_core::env::fixed::FixedEnv;
use ledger_core::types::{EntryId, EntryInput, LedgerState};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

/// The id-keyed collections of a `LedgerState`, in `diffState` order
/// (`rust/tools/src/trace.ts:23-26`).
pub const COLLECTIONS: [&str; 16] = [
    "accounts",
    "funds",
    "grants",
    "dimensions",
    "periods",
    "entries",
    "lines",
    "budgets",
    "budgetLines",
    "vendors",
    "bills",
    "payments",
    "claims",
    "bankAccounts",
    "statementLines",
    "audit",
];

/// Argument `i` of a fixture call; a missing argument reads as JSON `null`.
pub fn arg<T: DeserializeOwned>(args: &Value, i: usize) -> T {
    let v = args.get(i).cloned().unwrap_or(Value::Null);
    serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("argument {i} {v}: {e}"))
}

/// An optional argument: `null` or missing gives the default.
pub fn opt<T: DeserializeOwned + Default>(args: &Value, i: usize) -> T {
    match args.get(i) {
        None | Some(Value::Null) => T::default(),
        Some(_) => arg(args, i),
    }
}

/// A return value as JSON.
pub fn to_json<T: Serialize>(x: &T) -> Value {
    serde_json::to_value(x).unwrap()
}

fn out<T: Serialize>(r: Result<T, Error>) -> Result<Value, Error> {
    r.map(|x| to_json(&x))
}

/// The error as the exporter writes it (`rust/fixtures/README.md`, "Shapes").
pub fn error_json(e: &Error) -> Value {
    match e {
        Error::Ledger(e) => json!({
            "name": "LedgerError",
            "code": e.code(),
            "message": e.to_string(),
            "issues": e.issues(),
        }),
        Error::Money(e) => json!({ "name": "RangeError", "message": e.to_string() }),
    }
}

/// Calls the `Ledger` write method `method` with the fixture's `args` (actor first). `None` when
/// the method is not ported yet: the caller applies the recorded delta instead.
pub fn call(l: &mut Ledger<'_>, method: &str, a: &Value) -> Option<Result<Value, Error>> {
    Some(match method {
        "addAccount" => out(l.add_account(&arg(a, 0), arg(a, 1))),
        "updateAccount" => out(l.update_account(&arg(a, 0), &arg(a, 1), &arg(a, 2))),
        "addFund" => out(l.add_fund(&arg(a, 0), arg(a, 1))),
        "addDimension" => out(l.add_dimension(&arg(a, 0), arg(a, 1))),
        "openFiscalYear" => out(l
            .open_fiscal_year(&arg(a, 0), arg(a, 1))
            .map(|()| Value::Null)),
        "ensureFiscalYear" => Ok(Value::Null).inspect(|_| l.ensure_fiscal_year(arg(a, 0))),
        "saveDraft" => out(l.save_draft(&arg(a, 0), &arg(a, 1))),
        "post" => out(l.post(&arg(a, 0), &arg(a, 1), opt(a, 2))),
        "updateDraft" => out(l.update_draft(&arg(a, 0), &arg(a, 1), &arg(a, 2))),
        "deleteDraft" => out(l.delete_draft(&arg(a, 0), &arg(a, 1)).map(|()| Value::Null)),
        "postDraft" => out(l.post_draft(
            &arg(a, 0),
            &arg(a, 1),
            opt::<Option<String>>(a, 2).as_deref(),
        )),
        "reverse" => out(l.reverse(
            &arg(a, 0),
            &arg(a, 1),
            &arg::<String>(a, 2),
            &arg::<String>(a, 3),
        )),
        "closePeriod" => out(l.close_period(&arg(a, 0), &arg(a, 1))),
        "reopenPeriod" => out(l.reopen_period(&arg(a, 0), &arg(a, 1), &arg::<String>(a, 2))),
        "lockPeriod" => out(l.lock_period(&arg(a, 0), &arg(a, 1))),
        "closeYear" => out(l.close_year(&arg(a, 0), arg(a, 1))),
        _ => return None,
    })
}

/// Calls the read method `method` of the books. `None` when it is not ported yet.
pub fn query(s: &LedgerState, method: &str, a: &Value) -> Option<Result<Value, Error>> {
    Some(match method {
        "account" => Ok(to_json(&s.account(&arg(a, 0)))),
        "accountByCode" => Ok(to_json(&s.account_by_code(&arg::<String>(a, 0)))),
        "fund" => Ok(to_json(&s.fund(&arg(a, 0)))),
        "grant" => Ok(to_json(&s.grant(&arg(a, 0)))),
        "entry" => Ok(to_json(&s.entry(&arg(a, 0)))),
        "linesOf" => Ok(to_json(&s.lines_of(&arg(a, 0)))),
        "periodFor" => Ok(to_json(&s.period_for(&arg::<String>(a, 0)))),
        "inputOf" => out(s.input_of(&arg(a, 0)).map_err(Error::from)),
        "grantSpend" => Ok(json!(
            s.grant_spend(&arg(a, 0), opt::<Option<EntryId>>(a, 1).as_ref())
        )),
        "validateEntry" => out(s.validate_entry(
            &arg(a, 0),
            opt::<Option<EntryId>>(a, 1).as_ref(),
            opt::<EntryOptions>(a, 2),
        )),
        "claim" => out(s.claim(&arg(a, 0)).cloned().map_err(Error::from)),
        _ => return None,
    })
}

/// Applies a sanitised `LedgerCommand` as `applyCommand` does (`commands.ts:139-221`) for the
/// commands whose engine methods are ported, returning the `CommandResult`. `None` otherwise.
pub fn command(l: &mut Ledger<'_>, actor: &Value, cmd: &Value) -> Option<Result<Value, Error>> {
    let id = |r: Result<Value, Error>| {
        r.map(|v| match v.get("id") {
            Some(id) => json!({ "type": "id", "id": id }),
            None => json!({ "type": "none" }),
        })
    };
    // A sanitised input that does not fit the typed `EntryInput` (a string `fx_rate`, which TS
    // answers with FX_RATE_REQUIRED) is the command layer's to map (C4), not the engine's.
    let typed = |v: &Value| serde_json::from_value::<EntryInput>(v.clone()).is_ok();
    if !cmd["input"].is_null()
        && cmd["type"].as_str()?.starts_with("entry.")
        && !typed(&cmd["input"])
    {
        return None;
    }
    let (method, args) = match cmd["type"].as_str()? {
        "entry.post" => ("post", json!([actor, cmd["input"]])),
        "entry.saveDraft" => ("saveDraft", json!([actor, cmd["input"]])),
        "entry.updateDraft" => ("updateDraft", json!([actor, cmd["entryId"], cmd["input"]])),
        "entry.postDraft" => (
            "postDraft",
            json!([actor, cmd["entryId"], cmd["overrideReason"]]),
        ),
        "entry.deleteDraft" => ("deleteDraft", json!([actor, cmd["entryId"]])),
        "entry.reverse" => (
            "reverse",
            json!([actor, cmd["entryId"], cmd["date"], cmd["reason"]]),
        ),
        "account.add" => ("addAccount", json!([actor, cmd["input"]])),
        "account.update" => (
            "updateAccount",
            json!([actor, cmd["accountId"], cmd["patch"]]),
        ),
        "period.close" => ("closePeriod", json!([actor, cmd["periodId"]])),
        "period.reopen" => (
            "reopenPeriod",
            json!([actor, cmd["periodId"], cmd["reason"]]),
        ),
        "period.lock" => ("lockPeriod", json!([actor, cmd["periodId"]])),
        "year.close" => ("closeYear", json!([actor, cmd["year"]])),
        "year.open" => ("openFiscalYear", json!([actor, cmd["year"]])),
        "entry.import" => {
            // `{ type: 'ids', ids: inputs.map((i) => saveDraft(actor, i).id) }` (commands.ts:155-156).
            let mut ids = Vec::new();
            for input in cmd["inputs"].as_array()? {
                match call(l, "saveDraft", &json!([actor, input]))? {
                    Ok(v) => ids.push(v["id"].clone()),
                    Err(e) => return Some(Err(e)),
                }
            }
            return Some(Ok(json!({ "type": "ids", "ids": ids })));
        }
        _ => return None,
    };
    call(l, method, &args).map(id)
}

/// `diffState` (`rust/tools/src/trace.ts:30-62`) over two JSON snapshots of the books.
pub fn diff_state(before: &Value, after: &Value) -> Value {
    let mut delta = Map::new();
    for name in COLLECTIONS {
        let b = before[name].as_array().unwrap();
        let a = after[name].as_array().unwrap();
        let ids = |xs: &[Value]| xs.iter().map(|x| x["id"].clone()).collect::<Vec<_>>();
        let unique = |xs: &[Value]| {
            let ids = ids(xs);
            ids.iter().enumerate().all(|(i, id)| !ids[..i].contains(id))
        };
        if !unique(a) || !unique(b) {
            if a != b {
                delta.insert(name.to_owned(), json!({ "replaced": a }));
            }
            continue;
        }
        let find = |xs: &[Value], id: &Value| xs.iter().find(|x| &x["id"] == id).cloned();
        let added: Vec<Value> = a
            .iter()
            .filter(|x| find(b, &x["id"]).is_none())
            .cloned()
            .collect();
        let changed: Vec<Value> = a
            .iter()
            .filter(|x| find(b, &x["id"]).is_some_and(|old| &old != *x))
            .cloned()
            .collect();
        let a_ids = ids(a);
        let removed: Vec<Value> = ids(b)
            .into_iter()
            .filter(|id| !a_ids.contains(id))
            .collect();
        let mut d = Map::new();
        if !added.is_empty() {
            d.insert("added".into(), Value::Array(added.clone()));
        }
        if !changed.is_empty() {
            d.insert("changed".into(), Value::Array(changed));
        }
        if !removed.is_empty() {
            d.insert("removed".into(), Value::Array(removed));
        }
        let mut expected: Vec<Value> = ids(b).into_iter().filter(|id| a_ids.contains(id)).collect();
        expected.extend(ids(&added));
        if expected != a_ids {
            d.insert("order".into(), Value::Array(a_ids));
        }
        if !d.is_empty() {
            delta.insert(name.to_owned(), Value::Object(d));
        }
    }
    if before["settings"] != after["settings"] {
        delta.insert("settings".into(), after["settings"].clone());
    }
    let mut counters = Map::new();
    for (k, v) in after["counters"].as_object().unwrap() {
        if before["counters"].get(k) != Some(v) {
            counters.insert(k.clone(), v.clone());
        }
    }
    if !counters.is_empty() {
        delta.insert("counters".into(), Value::Object(counters));
    }
    Value::Object(delta)
}

/// Applies a recorded delta to a JSON snapshot of the books: the inverse of [`diff_state`].
pub fn apply_delta(state: &mut Value, delta: &Value) {
    for (name, d) in delta.as_object().unwrap() {
        match name.as_str() {
            "settings" => state["settings"] = d.clone(),
            "counters" => {
                for (k, v) in d.as_object().unwrap() {
                    state["counters"][k] = v.clone();
                }
            }
            _ => {
                assert!(
                    COLLECTIONS.contains(&name.as_str()),
                    "unknown delta key {name}"
                );
                if let Some(all) = d.get("replaced") {
                    state[name] = all.clone();
                    continue;
                }
                let list = state[name].as_array_mut().unwrap();
                if let Some(removed) = d.get("removed") {
                    let removed = removed.as_array().unwrap();
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
                if let Some(order) = d.get("order") {
                    let order = order.as_array().unwrap();
                    let mut sorted = Vec::with_capacity(order.len());
                    for id in order {
                        sorted.push(list.iter().find(|x| &x["id"] == id).unwrap().clone());
                    }
                    *list = sorted;
                }
            }
        }
    }
}

/// The counter behind an id the env would draw, if `id` has the env's form.
fn id_number(next: &str, id: &str) -> Option<u64> {
    // demo-000a (base 36, padded to 4), fx-0001 (padded to 4), id-1 (`rust/fixtures/README.md`).
    let (prefix, radix, width) = if next.starts_with("demo-") {
        ("demo-", 36, Some(4))
    } else if next.starts_with("fx-") {
        ("fx-", 10, Some(4))
    } else {
        ("id-", 10, None)
    };
    let digits = id.strip_prefix(prefix)?;
    let lowercase = digits
        .bytes()
        .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase());
    if digits.is_empty() || !lowercase || width.is_some_and(|w| digits.len() != w) {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

/// Moves `env` past every id of its form in `value`, so a step applied as a delta leaves the env
/// where the TS run left it.
pub fn sync_env(env: &mut FixedEnv, value: &Value) {
    let next = env.clone().new_id();
    let mut strings = Vec::new();
    super::find_strings(value, &|_| true, &mut strings);
    let highest = strings
        .iter()
        .filter_map(|s| id_number(&next, s))
        .max()
        .unwrap_or(0);
    while env.drawn() < highest {
        env.new_id();
    }
}

/// Pretty JSON for assertion messages.
pub fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap()
}

/// Asserts two JSON values are equal, printing both in full when they are not.
pub fn assert_json_eq(actual: &Value, expected: &Value, what: &str) {
    assert!(
        actual == expected,
        "{what}\n--- actual ---\n{}\n--- expected ---\n{}",
        pretty(actual),
        pretty(expected)
    );
}

/// How a fixture step went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ran {
    /// Through the engine, result and delta checked.
    Live,
    /// Rejected by the engine with the recorded error, books unchanged.
    Rejected,
    /// The recorded delta was applied; the method is not ported yet.
    Delta,
}

/// Runs one `op: "method"` step (`method`, `args`, `outcome`, `result`/`delta` or `error`).
pub fn run_method_step(
    state: &mut LedgerState,
    env: &mut FixedEnv,
    step: &Value,
    what: &str,
) -> Ran {
    let method = step["method"].as_str().unwrap();
    let before = to_json(state);
    let outcome = call(&mut Ledger::new(state, env), method, &step["args"]);
    check_outcome(
        state,
        env,
        step,
        before,
        outcome,
        &format!("{what} {method}"),
    )
}

/// Runs one `op: "command"` step through [`command`] with its sanitised command.
pub fn run_command_step(
    state: &mut LedgerState,
    env: &mut FixedEnv,
    step: &Value,
    what: &str,
) -> Ran {
    let kind = step["sanitized"]["type"].as_str().unwrap_or("?");
    let before = to_json(state);
    let outcome = if step["sanitized"].is_null() {
        None
    } else {
        command(
            &mut Ledger::new(state, env),
            &step["actor"],
            &step["sanitized"],
        )
    };
    check_outcome(state, env, step, before, outcome, &format!("{what} {kind}"))
}

fn check_outcome(
    state: &mut LedgerState,
    env: &mut FixedEnv,
    step: &Value,
    before: Value,
    outcome: Option<Result<Value, Error>>,
    what: &str,
) -> Ran {
    let ok = step["outcome"] == "ok";
    match outcome {
        Some(Ok(result)) => {
            assert!(
                ok,
                "{what}: accepted, but the TS engine rejected it with {}",
                pretty(&step["error"])
            );
            let expected = step.get("result").cloned().unwrap_or(Value::Null);
            assert_json_eq(&result, &expected, &format!("{what}: result"));
            assert_json_eq(
                &diff_state(&before, &to_json(state)),
                &step["delta"],
                &format!("{what}: delta"),
            );
            Ran::Live
        }
        Some(Err(e)) => {
            assert!(
                !ok,
                "{what}: rejected with {e}, but the TS engine accepted it"
            );
            assert_json_eq(&error_json(&e), &step["error"], &format!("{what}: error"));
            assert_json_eq(
                &to_json(state),
                &before,
                &format!("{what}: books changed by a rejected call"),
            );
            Ran::Rejected
        }
        None => {
            if ok {
                let mut books = before;
                apply_delta(&mut books, &step["delta"]);
                *state = serde_json::from_value(books).unwrap();
                sync_env(env, &step["delta"]);
                sync_env(env, step.get("result").unwrap_or(&Value::Null));
            }
            Ran::Delta
        }
    }
}

/// The `Ledger` write methods ported in task C2: every fixture step that calls one must run live.
pub const C2_METHODS: [&str; 16] = [
    "addAccount",
    "updateAccount",
    "addFund",
    "addDimension",
    "openFiscalYear",
    "ensureFiscalYear",
    "saveDraft",
    "post",
    "updateDraft",
    "deleteDraft",
    "postDraft",
    "reverse",
    "closePeriod",
    "reopenPeriod",
    "lockPeriod",
    "closeYear",
];

/// The `Ledger` read methods ported in task C2 (`outstanding` belongs to C3).
pub const C2_QUERIES: [&str; 11] = [
    "account",
    "accountByCode",
    "fund",
    "grant",
    "entry",
    "linesOf",
    "periodFor",
    "inputOf",
    "grantSpend",
    "validateEntry",
    "claim",
];
