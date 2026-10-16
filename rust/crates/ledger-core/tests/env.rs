//! The fixed envs draw the same ids and time as the TS envs that wrote the fixtures.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeSet;

use common::{find_strings, fixture};
use ledger_core::env::{LedgerEnv, Timestamp, fixed};

/// The first `n` ids an env draws.
fn draw(env: &mut impl LedgerEnv, n: usize) -> Vec<String> {
    (0..n).map(|_| env.new_id()).collect()
}

#[test]
fn demo_env_draws_exactly_the_ids_in_the_demo_books() {
    let books = fixture("demo-books");
    let env = fixed::demo();
    assert_eq!(env.now().as_str(), books["env"]["now"].as_str().unwrap());

    // The demo books keep every id the demo env drew (demo-0001 to demo-005r, contiguous).
    let mut found = Vec::new();
    find_strings(
        &books["state"],
        &|s| s.len() == 9 && s.starts_with("demo-"),
        &mut found,
    );
    let found: BTreeSet<&str> = found.into_iter().collect();
    let mut env = fixed::demo();
    let drawn = draw(&mut env, found.len());
    assert_eq!(drawn[0], "demo-0001");
    assert_eq!(drawn[9], "demo-000a");
    assert_eq!(
        drawn.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        found
    );
}

#[test]
fn fixture_env_draws_the_ids_in_the_commands_final_state() {
    let commands = fixture("commands-accepted");
    let env = fixed::fixture();
    assert_eq!(env.now().as_str(), commands["env"]["now"].as_str().unwrap());

    // Deleted drafts leave gaps, so every fx- id must be one the env draws, up to the largest.
    let mut found = Vec::new();
    find_strings(
        &commands["final_state"],
        &|s| s.starts_with("fx-"),
        &mut found,
    );
    let found: BTreeSet<&str> = found.into_iter().collect();
    let largest = found
        .iter()
        .map(|s| s[3..].parse::<usize>().unwrap())
        .max()
        .unwrap();
    let mut env = fixed::fixture();
    let drawn = draw(&mut env, largest);
    assert_eq!(drawn[0], "fx-0001");
    let drawn: BTreeSet<&str> = drawn.iter().map(String::as_str).collect();
    assert!(found.is_subset(&drawn));
    assert!(found.contains(format!("fx-{largest:04}").as_str()));
}

#[test]
fn test_env_draws_the_first_ids_of_a_rejected_scenario() {
    let rejected = fixture("rejected-cases");
    let env = fixed::test();
    assert_eq!(env.now().as_str(), rejected["env"]["now"].as_str().unwrap());

    // Scenario 1 starts addFund then addGrant on a fresh ledger: one audit event each.
    let steps = rejected["scenarios"][0]["steps"].as_array().unwrap();
    let mut env = fixed::test();
    for step in &steps[..2] {
        let event = &step["delta"]["audit"]["added"][0];
        assert_eq!(env.new_id(), event["id"].as_str().unwrap());
        assert_eq!(env.now().as_str(), event["at"].as_str().unwrap());
    }
}

#[test]
fn every_fixed_env_counter_starts_at_one_per_env() {
    let mut a = fixed::test();
    let mut b = fixed::test();
    assert_eq!(draw(&mut a, 2), ["id-1", "id-2"]);
    assert_eq!(draw(&mut b, 1), ["id-1"]);
}

#[test]
fn timestamp_accepts_only_iso_utc_with_milliseconds() {
    let ok = Timestamp::parse("2026-02-04T09:00:00.000Z").unwrap();
    assert_eq!(ok.as_str(), "2026-02-04T09:00:00.000Z");
    for bad in [
        "2026-02-04T09:00:00Z",
        "2026-02-04T09:00:00.000+00:00",
        "2026-02-04 09:00:00.000Z",
        "2026-2-04T09:00:00.000Z",
        "",
    ] {
        assert!(Timestamp::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn timestamp_serialises_as_the_ts_string() {
    let ts = Timestamp::parse("2026-03-01T00:00:00.000Z").unwrap();
    let json = serde_json::to_string(&ts).unwrap();
    assert_eq!(json, "\"2026-03-01T00:00:00.000Z\"");
    let back: Timestamp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ts);
    assert!(serde_json::from_str::<Timestamp>("\"yesterday\"").is_err());
}
