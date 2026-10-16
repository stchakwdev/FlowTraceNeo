//! [`SpacetimeClient`] against the ledger module on the local SpacetimeDB server: publish, open
//! and import the demo books through the client, rebuild them from the subscription, check the
//! trial balance and financial position against the fixtures, run the client checks of
//! `tests/common`, reconnect with the saved token, and see a stranger refused.
//!
//! Ignored by default so the workspace checks stay offline. To run it (local server only, never
//! maincloud; the database is deleted at the end):
//!
//! ```sh
//! cd rust
//! cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//! LEDGER_CLIENT_LIVE=1 cargo test -p ledger-client --test live -- --ignored --nocapture
//! ```
//!
//! `LEDGER_CLIENT_LIVE_DB` names the database (default `ft-s3-test`), `SPACETIME` the CLI
//! (default `~/.local/bin/spacetime`).
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use common::{client_contract, demo_org_id, demo_state, new_vendor, wait_for};
use ledger_client::{
    ClientConfig, ClientError, ClientEvent, ConnectionStatus, SpacetimeClient, TokenFile,
};

/// The local server, the only one this test talks to (`crates/ledger-module/README.md`).
const LOCAL: &str = "http://127.0.0.1:3100";

fn cli() -> PathBuf {
    std::env::var_os("SPACETIME").map_or_else(
        || PathBuf::from(std::env::var("HOME").unwrap()).join(".local/bin/spacetime"),
        PathBuf::from,
    )
}

/// Runs the CLI against the local server and returns its output; panics on failure unless
/// `may_fail`.
fn spacetime(args: &[&str], may_fail: bool) -> String {
    let out = Command::new(cli()).args(args).output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success() || may_fail,
        "spacetime {args:?} failed:\n{text}"
    );
    text
}

/// The published database; deleted when the test ends, pass or fail.
struct Database(String);

impl Drop for Database {
    fn drop(&mut self) {
        spacetime(&["delete", "-s", "local", "-y", &self.0], true);
        eprintln!(
            "deleted {}; spacetime list -s local:\n{}",
            self.0,
            spacetime(&["list", "-s", "local"], true).trim()
        );
    }
}

fn wait_ready(client: &SpacetimeClient) -> String {
    let rx = client.subscribe();
    wait_for(&rx, Duration::from_secs(15), |e| match e {
        ClientEvent::Status(ConnectionStatus::Ready { identity }) => Some(identity.clone()),
        ClientEvent::Status(ConnectionStatus::Disconnected { error }) => {
            panic!("disconnected: {error:?}")
        }
        _ => None,
    })
}

fn connect(db: &str, token: Option<&TokenFile>) -> (SpacetimeClient, String, Duration) {
    let mut config = ClientConfig::new(LOCAL, db);
    if let Some(file) = token {
        config = config.with_token_file(file.clone());
    }
    let started = Instant::now();
    let client = SpacetimeClient::connect(config).unwrap();
    let identity = wait_ready(&client);
    (client, identity, started.elapsed())
}

#[test]
#[ignore = "needs the local SpacetimeDB server; set LEDGER_CLIENT_LIVE=1 and pass --ignored"]
fn the_client_rebuilds_the_demo_books_from_the_module_and_runs_a_command() {
    if std::env::var_os("LEDGER_CLIENT_LIVE").is_none() {
        eprintln!("skipped: set LEDGER_CLIENT_LIVE=1 to run against {LOCAL}");
        return;
    }
    let wasm = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-unknown-unknown/release/ledger_module.wasm");
    assert!(
        wasm.exists(),
        "build the module first: cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module"
    );
    let name = std::env::var("LEDGER_CLIENT_LIVE_DB").unwrap_or_else(|_| "ft-s3-test".to_owned());

    // A leftover from an earlier run would already hold the books.
    spacetime(&["delete", "-s", "local", "-y", &name], true);
    let published = spacetime(
        &[
            "publish",
            "-s",
            "local",
            "-b",
            wasm.to_str().unwrap(),
            "-y",
            &name,
        ],
        false,
    );
    let db = Database(name);
    eprintln!(
        "published {}: {}",
        db.0,
        published.trim().lines().last().unwrap_or_default()
    );

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ledger-client-live");
    let _ = std::fs::remove_dir_all(&dir);
    let token = TokenFile::new(dir.join("token"));

    // 1. Connect with no saved token: a new identity, saved to the token file.
    let (client, identity, took) = connect(&db.0, Some(&token));
    assert_eq!(client.organisations(), []);
    assert!(token.load().unwrap().is_some(), "the token was saved");
    eprintln!(
        "connected as {}... and ready in {took:?}; token saved",
        &identity[..16]
    );

    // 2. The CLI identity that published the module makes the client an operator.
    spacetime(
        &[
            "call",
            "-s",
            "local",
            &db.0,
            "add_operator",
            &format!("\"0x{identity}\""),
        ],
        false,
    );

    // 3. Open the demo organisation's books and load the demo books.
    let org = demo_org_id();
    open_and_import(&client, &org);

    // 4. The rebuilt books equal the demo books, the reports equal the fixtures, a command and
    //    a refusal come back through the subscription.
    let (tb, fp) = common::reports_match_the_demo_fixtures(&client.state(&org).unwrap());
    let started = Instant::now();
    let run = client_contract(&client, &org);
    eprintln!(
        "rebuilt books equal demo-books.json#/state; trial balance {tb} and financial position {fp} \
         demo-books cases equal; contract kept in {:?}: vendor.add -> {}, events {:?}, unbalanced \
         entry -> {:?}",
        started.elapsed(),
        run.vendor_id,
        run.events,
        run.refused
    );
    let books = client.state(&org).unwrap();
    client.disconnect();
    assert!(matches!(
        client.status(),
        ConnectionStatus::Disconnected { error: None }
    ));
    assert!(matches!(
        client.submit(&org, &new_vendor()),
        Err(ClientError::Disconnected)
    ));

    // 5. The saved token brings back the same identity, and a fresh subscription the same books.
    let (again, same, took) = connect(&db.0, Some(&token));
    assert_eq!(same, identity);
    assert_eq!(again.state(&org).unwrap(), books);
    let members = &again.organisations()[0].members;
    assert_eq!(members.len(), 1);
    assert!(members[0].can_close);
    eprintln!("reconnected with the saved token as the same identity in {took:?}; books equal");
    drop(again);

    // 6. A stranger sees no books and is refused.
    stranger_is_refused(&db.0, &org, &identity);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Step 3: open the demo organisation's books and load the demo books, through the client.
fn open_and_import(client: &SpacetimeClient, org: &str) {
    let started = Instant::now();
    client
        .create_org_ledger(org, "KES", 2026, "Live Test Owner")
        .wait()
        .unwrap();
    let seeded = client.state(org).unwrap();
    eprintln!(
        "create_org_ledger in {:?}: {} accounts, {} periods",
        started.elapsed(),
        seeded.accounts.len(),
        seeded.periods.len()
    );
    let started = Instant::now();
    client.import_books(org, &demo_state()).wait().unwrap();
    eprintln!("import_books in {:?}", started.elapsed());
}

/// Step 6: someone who is no member sees no books and is refused with the module's text.
fn stranger_is_refused(db: &str, org: &str, identity: &str) {
    let (stranger, other, _) = connect(db, None);
    assert_ne!(other, identity);
    assert_eq!(stranger.organisations(), []);
    assert!(matches!(stranger.state(org), Err(ClientError::NoLedger(_))));
    let refused = stranger.submit(org, &new_vendor());
    let Err(ClientError::Rejected(text)) = refused else {
        panic!("stranger came back {refused:?}");
    };
    assert_eq!(text, format!("You are not a member of organisation {org}."));
    eprintln!("a stranger sees no organisations and is refused: {text}");
    drop(stranger);
}
