//! One `run_command` call up to the write-back, as pure functions: the reducer's engine env, the
//! JSON and sanitising step, the engine call, and the error texts and result JSON the caller sees
//! (`spacetimedb/src/index.ts:655-691`).
//!
//! The reducer in [`crate::reducers`] only adds the table reads and writes, so the host tests
//! cover everything a caller can observe except the transaction itself.

use ledger_core::types::{Actor, LedgerState, Timestamp};
use ledger_core::{CommandError, CommandResult, Ledger, LedgerCommand, LedgerEnv, LedgerError};

/// What a rejected command's error text starts with, followed by the issues as JSON
/// (`LEDGER_ERROR_PREFIX`, `flowtrace/src/lib/ledger/spacetime-codec.ts:245-246`). The TS client
/// turns such a text back into a `LedgerError` (`flowtrace/src/lib/ledger/spacetime-backend.ts:160-168`).
pub const LEDGER_ERROR_PREFIX: &str = "LEDGER_ERROR ";

/// The engine env of one reducer call, `envFor` (`index.ts:361-365`): `now` is the reducer
/// timestamp and every id is a fresh reducer-seeded UUID v4 (`ctx.newUuidV4().toString()`).
///
/// [`LedgerEnv::new_id`] cannot fail, but `ReducerContext::new_uuid_v4` returns a `Result`
/// (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/struct.ReducerContext.html#method.new_uuid_v4>).
/// So the env keeps the first failure, hands out an empty id from then on, and [`ModuleEnv::finish`]
/// turns the failure into the reducer's error, which rolls the whole call back.
pub struct ModuleEnv<F> {
    now: Timestamp,
    draw: F,
    failure: Option<String>,
}

impl<F: FnMut() -> Result<String, String>> ModuleEnv<F> {
    /// An env whose clock reads `now` and whose ids come from `draw`, in call order.
    pub fn new(now: Timestamp, draw: F) -> Self {
        Self {
            now,
            draw,
            failure: None,
        }
    }

    /// Ends the call.
    ///
    /// # Errors
    ///
    /// The first error `draw` returned, if any.
    pub fn finish(self) -> Result<(), String> {
        self.failure.map_or(Ok(()), Err)
    }
}

impl<F: FnMut() -> Result<String, String>> LedgerEnv for ModuleEnv<F> {
    fn now(&self) -> Timestamp {
        self.now.clone()
    }

    fn new_id(&mut self) -> String {
        if self.failure.is_some() {
            return String::new();
        }
        (self.draw)().unwrap_or_else(|e| {
            self.failure = Some(e);
            String::new()
        })
    }
}

/// `rethrow` (`index.ts:512-518`): a ledger error as [`LEDGER_ERROR_PREFIX`] and its issues as
/// JSON.
#[must_use]
pub fn ledger_error_text(error: &LedgerError) -> String {
    // Issues are plain strings, numbers and booleans, so they always serialise.
    let issues = serde_json::to_string(error.issues()).unwrap_or_default();
    format!("{LEDGER_ERROR_PREFIX}{issues}")
}

/// `JSON.parse` then `sanitizeCommand` (`index.ts:665-675`).
///
/// # Errors
///
/// `The command is not valid JSON.`, or the sanitiser's ledger error as
/// [`ledger_error_text`].
pub fn parse_command(command_json: &str) -> Result<LedgerCommand, String> {
    let raw: serde_json::Value = serde_json::from_str(command_json)
        .map_err(|_| "The command is not valid JSON.".to_owned())?;
    ledger_core::sanitize_command(&raw).map_err(|e| ledger_error_text(&e))
}

/// `applyCommand` on the loaded books with the member as actor (`index.ts:677-684`). The engine
/// changes `state` in place, and leaves it unchanged when it refuses the command.
///
/// # Errors
///
/// A ledger rule as [`ledger_error_text`]; a `money.ts` range error by its message, as TS rethrows
/// it unchanged; a command ledger-core cannot run yet by its message.
pub fn apply(
    state: &mut LedgerState,
    env: &mut dyn LedgerEnv,
    actor: &Actor,
    cmd: LedgerCommand,
) -> Result<CommandResult, String> {
    let mut ledger = Ledger::new(state, env);
    ledger_core::apply_command(&mut ledger, actor, cmd).map_err(|e| match e {
        CommandError::Engine(ledger_core::engine::Error::Ledger(e)) => ledger_error_text(&e),
        other => other.to_string(),
    })
}

/// `resultJson`, `JSON.stringify(result)` (`index.ts:687`).
///
/// # Errors
///
/// Never in practice: a result that does not serialise.
pub fn result_json(result: &CommandResult) -> Result<String, String> {
    serde_json::to_string(result).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_core::env::fixed;
    use ledger_core::types::ActorId;

    fn now() -> Timestamp {
        Timestamp::parse(fixed::DEMO_NOW).unwrap()
    }

    fn counting() -> impl FnMut() -> Result<String, String> {
        let mut n = 0;
        move || {
            n += 1;
            Ok(format!("uuid-{n}"))
        }
    }

    fn demo_state() -> LedgerState {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/demo-books.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        serde_json::from_value(fixture["state"].clone()).unwrap()
    }

    fn actor(can_post: bool) -> Actor {
        Actor {
            id: ActorId::from("user_dummy"),
            name: "Demo User".to_owned(),
            can_post,
            can_approve: true,
            can_close: true,
        }
    }

    const PAPER: &str = r#"{"type":"entry.post","input":{"date":"2026-02-04","memo":"Paper","lines":[
        {"account_id":"acc-7100","debit":250000,"fund_id":"fund-unrestricted"},
        {"account_id":"acc-1110","credit":250000,"fund_id":"fund-unrestricted"}]}}"#;

    #[test]
    fn the_env_reads_the_reducer_time() {
        let env = ModuleEnv::new(now(), counting());
        assert_eq!(env.now().as_str(), "2026-02-04T09:00:00.000Z");
    }

    #[test]
    fn the_env_draws_ids_from_its_source_in_call_order() {
        let mut env = ModuleEnv::new(now(), counting());
        assert_eq!(
            [env.new_id(), env.new_id(), env.new_id()],
            ["uuid-1", "uuid-2", "uuid-3"]
        );
        assert_eq!(env.finish(), Ok(()));
    }

    #[test]
    fn the_env_keeps_the_first_failed_draw_for_finish() {
        let mut calls = 0;
        let mut env = ModuleEnv::new(now(), move || {
            calls += 1;
            if calls == 1 {
                Ok("uuid-1".to_owned())
            } else {
                Err(format!("no randomness on draw {calls}"))
            }
        });
        let drawn = [env.new_id(), env.new_id(), env.new_id()];
        assert_eq!(drawn, ["uuid-1", "", ""]);
        assert_eq!(env.finish(), Err("no randomness on draw 2".to_owned()));
    }

    #[test]
    fn a_command_that_is_not_json_is_refused_with_the_ts_text() {
        assert_eq!(
            parse_command("{type:").unwrap_err(),
            "The command is not valid JSON."
        );
    }

    #[test]
    fn an_unknown_command_is_a_ledger_error_with_its_issues_as_json() {
        assert_eq!(
            parse_command(r#"{"type":"entry.burn"}"#).unwrap_err(),
            r#"LEDGER_ERROR [{"code":"INVALID_INPUT","message":"Unknown command."}]"#
        );
    }

    #[test]
    fn a_rule_the_engine_enforces_comes_back_as_a_ledger_error() {
        let mut state = demo_state();
        let before = state.clone();
        let mut env = ModuleEnv::new(now(), counting());
        let cmd = parse_command(PAPER).unwrap();
        let text = apply(&mut state, &mut env, &actor(false), cmd).unwrap_err();
        assert_eq!(
            text,
            r#"LEDGER_ERROR [{"code":"PERMISSION","message":"Demo User does not have permission to post to the ledger."}]"#
        );
        assert_eq!(state, before);
    }

    #[test]
    fn an_accepted_command_changes_the_state_with_the_env_ids_and_clock() {
        let mut state = demo_state();
        let mut env = ModuleEnv::new(now(), counting());
        let cmd = parse_command(PAPER).unwrap();
        let result = apply(&mut state, &mut env, &actor(true), cmd).unwrap();
        assert_eq!(
            result,
            CommandResult::Id {
                id: "uuid-1".to_owned()
            }
        );
        let entry = state.entries.last().unwrap();
        assert_eq!(
            (entry.id.as_str(), entry.created_at.as_str()),
            ("uuid-1", "2026-02-04T09:00:00.000Z")
        );
    }

    #[test]
    fn every_fixture_result_serialises_as_the_ts_json() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/commands-accepted.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for step in fixture["steps"].as_array().unwrap() {
            let result: CommandResult = serde_json::from_value(step["result"].clone()).unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&result_json(&result).unwrap()).unwrap();
            assert_eq!(json, step["result"]);
        }
    }

    #[test]
    fn a_result_json_starts_with_its_type_as_the_ts_literal_does() {
        let json = result_json(&CommandResult::Id {
            id: "fx-0001".to_owned(),
        });
        assert_eq!(json.unwrap(), r#"{"type":"id","id":"fx-0001"}"#);
    }
}
