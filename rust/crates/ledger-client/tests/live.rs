//! [`SpacetimeClient`] against the ledger module: publish, open and import the demo books
//! through the client, rebuild them from the subscription, check the trial balance and financial
//! position against the fixtures, run the client checks of `tests/common`, reconnect with the
//! saved token, and see a stranger refused.
//!
//! Ignored by default so the workspace checks stay offline. To run it on the local server (the
//! database is published fresh and deleted at the end):
//!
//! ```sh
//! cd rust
//! cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//! LEDGER_CLIENT_LIVE=1 cargo test -p ledger-client --test live -- --ignored --nocapture
//! ```
//!
//! `LEDGER_CLIENT_LIVE_DB` names the database (default `ft-s3-test`), `SPACETIME` the CLI
//! (default `~/.local/bin/spacetime`).
//!
//! Remote mode, for a database that is already published (maincloud dev for gate G3):
//!
//! ```sh
//! LEDGER_CLIENT_LIVE=1 LEDGER_CLIENT_LIVE_EXISTING=1 \
//!   LEDGER_CLIENT_LIVE_URL=https://maincloud.spacetimedb.com LEDGER_CLIENT_LIVE_DB=flowtrace-ledger-dev \
//!   cargo test -p ledger-client --test live -- --ignored --nocapture
//! ```
//!
//! It never publishes or deletes: it opens a new organisation with a random id, makes the
//! client an operator through `spacetime call -s <server>` (the CLI login must be an operator of
//! that database, as its publisher is) and withdraws it with `remove_operator` at the end.
//! WARNING: it writes a demo organisation into whatever database it is pointed at and leaves it
//! there.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::{client_contract, demo_org_id, demo_state_for, new_vendor, wait_for};
use ledger_client::{
    ClientConfig, ClientError, ClientEvent, ConnectionStatus, SpacetimeClient, TokenFile,
};

/// The local server, the default and the only one the test publishes to
/// (`crates/ledger-module/README.md`).
const LOCAL: &str = "http://127.0.0.1:3100";

/// Maincloud, which the CLI knows as `maincloud` (`spacetime server list`).
const MAINCLOUD: &str = "https://maincloud.spacetimedb.com";

/// The server and database the test talks to, from the `LEDGER_CLIENT_LIVE_*` variables.
#[derive(Debug, PartialEq, Eq)]
struct Target {
    url: String,
    /// The `spacetime -s` value for `url`.
    cli_server: String,
    db: String,
    /// The database is already published: never publish or delete it.
    existing: bool,
}

impl Target {
    fn from_env() -> Self {
        let var = |name| std::env::var(name).ok();
        Self::from_vars(
            var("LEDGER_CLIENT_LIVE_URL"),
            var("LEDGER_CLIENT_LIVE_EXISTING"),
            var("LEDGER_CLIENT_LIVE_DB"),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    /// Without `existing`: the local server and an `ft-` name, published and deleted by the test.
    /// With `existing` (any value but empty or `0`): both the URL and the name must be given.
    fn from_vars(
        url: Option<String>,
        existing: Option<String>,
        db: Option<String>,
    ) -> Result<Self, String> {
        let existing = existing.is_some_and(|v| !v.is_empty() && v != "0");
        let url = url.map(|u| u.trim_end_matches('/').to_owned());
        if existing {
            let url = url.ok_or("LEDGER_CLIENT_LIVE_EXISTING needs LEDGER_CLIENT_LIVE_URL")?;
            let db = db.ok_or("LEDGER_CLIENT_LIVE_EXISTING needs LEDGER_CLIENT_LIVE_DB")?;
            let cli_server = match url.as_str() {
                MAINCLOUD => "maincloud".to_owned(),
                LOCAL => "local".to_owned(),
                other => other.to_owned(),
            };
            return Ok(Self {
                url,
                cli_server,
                db,
                existing,
            });
        }
        let url = url.unwrap_or_else(|| LOCAL.to_owned());
        if url != LOCAL {
            return Err(format!(
                "{url}: the test publishes only on {LOCAL}; set LEDGER_CLIENT_LIVE_EXISTING=1 for a published database"
            ));
        }
        let db = db.unwrap_or_else(|| "ft-s3-test".to_owned());
        if !db.starts_with("ft-") {
            return Err(format!(
                "{db}: a database the test publishes is named ft-<task>"
            ));
        }
        Ok(Self {
            url,
            cli_server: "local".to_owned(),
            db,
            existing,
        })
    }
}

/// A new organisation id for a run on an existing database, shaped like a UUID from the clock,
/// the process id and a counter (no uuid dependency in this crate).
fn run_org_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the low 64 bits of the clock"
    )]
    let hex = format!("{:016x}{:08x}{n:08x}", nanos as u64, std::process::id());
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn cli() -> PathBuf {
    std::env::var_os("SPACETIME").map_or_else(
        || PathBuf::from(std::env::var("HOME").unwrap()).join(".local/bin/spacetime"),
        PathBuf::from,
    )
}

/// Runs the CLI and returns its output; panics on failure unless `may_fail`.
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

/// A database the test published on the local server; deleted when the test ends, pass or fail.
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

/// Publishes the module as `name` on the local server, replacing a leftover from an earlier run.
fn publish_locally(name: &str) -> Database {
    let wasm = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-unknown-unknown/release/ledger_module.wasm");
    assert!(
        wasm.exists(),
        "build the module first: cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module"
    );
    spacetime(&["delete", "-s", "local", "-y", name], true);
    let published = spacetime(
        &[
            "publish",
            "-s",
            "local",
            "-b",
            wasm.to_str().unwrap(),
            "-y",
            name,
        ],
        false,
    );
    eprintln!(
        "published {name}: {}",
        published.trim().lines().last().unwrap_or_default()
    );
    Database(name.to_owned())
}

fn wait_ready(client: &SpacetimeClient) -> String {
    let rx = client.subscribe();
    wait_for(&rx, Duration::from_secs(30), |e| match e {
        ClientEvent::Status(ConnectionStatus::Ready { identity }) => Some(identity.clone()),
        ClientEvent::Status(ConnectionStatus::Disconnected { error }) => {
            panic!("disconnected: {error:?}")
        }
        _ => None,
    })
}

/// Connects and waits for `Ready` (the initial subscription applied); returns that wait.
fn connect(t: &Target, token: Option<&TokenFile>) -> (SpacetimeClient, String, Duration) {
    let mut config = ClientConfig::new(&t.url, &t.db);
    if let Some(file) = token {
        config = config.with_token_file(file.clone());
    }
    let started = Instant::now();
    let client = SpacetimeClient::connect(config).unwrap();
    let identity = wait_ready(&client);
    (client, identity, started.elapsed())
}

/// Calls `reducer` as the CLI login (the database's publisher, an operator).
fn cli_call(t: &Target, reducer: &str, identity: &str) {
    spacetime(
        &[
            "call",
            "-s",
            &t.cli_server,
            &t.db,
            reducer,
            &format!("\"0x{identity}\""),
        ],
        false,
    );
}

#[test]
#[ignore = "needs a SpacetimeDB server; set LEDGER_CLIENT_LIVE=1 and pass --ignored"]
fn the_client_rebuilds_the_demo_books_from_the_module_and_runs_a_command() {
    if std::env::var_os("LEDGER_CLIENT_LIVE").is_none() {
        eprintln!("skipped: set LEDGER_CLIENT_LIVE=1 to run against a server");
        return;
    }
    let t = Target::from_env();
    // Only a database the test published is deleted (on drop).
    let _published = (!t.existing).then(|| publish_locally(&t.db));
    let org = if t.existing {
        run_org_id()
    } else {
        demo_org_id()
    };
    eprintln!(
        "{} {} on {}, organisation {org}",
        if t.existing {
            "existing database"
        } else {
            "database"
        },
        t.db,
        t.url
    );

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ledger-client-live");
    let _ = std::fs::remove_dir_all(&dir);
    let token = TokenFile::new(dir.join("token"));

    // 1. Connect with no saved token: a new identity, saved to the token file.
    let (client, identity, took) = connect(&t, Some(&token));
    assert_eq!(client.organisations(), []);
    assert!(token.load().unwrap().is_some(), "the token was saved");
    eprintln!(
        "connected as {}... and ready (initial subscription, no organisations) in {took:?}; token saved",
        &identity[..16]
    );

    // 2. The CLI identity that published the module makes the client an operator.
    cli_call(&t, "add_operator", &identity);

    // 3. Open the organisation's books and load the demo books.
    open_and_import(&client, &org);

    // 4. The rebuilt books equal the demo books, the reports equal the fixtures, a command and
    //    a refusal come back through the subscription.
    let (tb, fp) = common::reports_match_the_demo_fixtures(&client.state(&org).unwrap());
    let started = Instant::now();
    let run = client_contract(&client, &org);
    eprintln!(
        "rebuilt books equal demo-books.json#/state; trial balance {tb} and financial position {fp} \
         demo-books cases equal; contract kept in {:?}: vendor.add -> {} in {:?} (sent to change \
         readable), events {:?}, unbalanced entry -> {:?}",
        started.elapsed(),
        run.vendor_id,
        run.accepted_took,
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
    let (again, same, took) = connect(&t, Some(&token));
    assert_eq!(same, identity);
    assert_eq!(again.state(&org).unwrap(), books);
    let members = &again.organisations()[0].members;
    assert_eq!(members.len(), 1);
    assert!(members[0].can_close);
    eprintln!(
        "reconnected with the saved token as the same identity, initial subscription with the \
         books ({} entries, {} lines) in {took:?}; books equal",
        books.entries.len(),
        books.lines.len()
    );
    drop(again);

    // 6. A stranger sees no books and is refused.
    stranger_is_refused(&t, &org, &identity);

    // 7. On an existing database, leave the operator table as it was.
    if t.existing {
        cli_call(&t, "remove_operator", &identity);
        eprintln!(
            "withdrew operator 0x{}...; left organisation {org} in {}",
            &identity[..12],
            t.db
        );
    }
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
    client
        .import_books(org, &demo_state_for(org))
        .wait()
        .unwrap();
    eprintln!("import_books in {:?}", started.elapsed());
}

/// Step 6: someone who is no member sees no books and is refused with the module's text.
fn stranger_is_refused(t: &Target, org: &str, identity: &str) {
    let (stranger, other, _) = connect(t, None);
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

#[test]
fn without_the_remote_variables_the_test_publishes_on_the_local_server() {
    let t = Target::from_vars(None, None, None).unwrap();
    assert_eq!(
        t,
        Target {
            url: LOCAL.to_owned(),
            cli_server: "local".to_owned(),
            db: "ft-s3-test".to_owned(),
            existing: false,
        }
    );
    let named = Target::from_vars(None, None, Some("ft-x".to_owned())).unwrap();
    assert_eq!(named.db, "ft-x");
}

#[test]
fn existing_mode_targets_the_named_database_on_maincloud_by_its_cli_nickname() {
    let t = Target::from_vars(
        Some("https://maincloud.spacetimedb.com".to_owned()),
        Some("1".to_owned()),
        Some("flowtrace-ledger-dev".to_owned()),
    )
    .unwrap();
    assert_eq!(
        t,
        Target {
            url: "https://maincloud.spacetimedb.com".to_owned(),
            cli_server: "maincloud".to_owned(),
            db: "flowtrace-ledger-dev".to_owned(),
            existing: true,
        }
    );
}

#[test]
fn existing_mode_needs_a_url_and_a_database_name() {
    let no_db = Target::from_vars(
        Some("https://maincloud.spacetimedb.com".to_owned()),
        Some("1".to_owned()),
        None,
    );
    assert!(no_db.unwrap_err().contains("LEDGER_CLIENT_LIVE_DB"));
    let no_url = Target::from_vars(None, Some("1".to_owned()), Some("db".to_owned()));
    assert!(no_url.unwrap_err().contains("LEDGER_CLIENT_LIVE_URL"));
}

#[test]
fn local_mode_refuses_a_remote_url_and_a_name_without_ft() {
    let remote = Target::from_vars(
        Some("https://maincloud.spacetimedb.com".to_owned()),
        None,
        None,
    );
    assert!(
        remote
            .unwrap_err()
            .contains("LEDGER_CLIENT_LIVE_EXISTING=1")
    );
    let name = Target::from_vars(None, None, Some("flowtrace-ledger-dev".to_owned()));
    assert!(name.unwrap_err().contains("ft-"));
}

#[test]
fn a_run_org_id_is_new_each_time_and_shaped_like_a_uuid() {
    let a = run_org_id();
    let b = run_org_id();
    assert_ne!(a, b);
    let parts: Vec<usize> = a.split('-').map(str::len).collect();
    assert_eq!(parts, [8, 4, 4, 4, 12], "{a}");
}
