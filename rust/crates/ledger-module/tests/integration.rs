//! The module end to end on the local SpacetimeDB server: runs `bun run integration` in
//! `rust/tools` (`module-integration.ts`: build, publish `ft-s2-<timestamp>`, load the demo books
//! and the command log, SQL row, balance and trial-balance checks, refusals, views, delete).
//!
//! Ignored by default so the workspace checks stay offline. Run it with the local server up:
//!
//! ```sh
//! FT_MODULE_INTEGRATION=1 cargo test -p ledger-module --test integration -- --ignored
//! ```
//!
//! Without `FT_MODULE_INTEGRATION` the test returns at once, so `--include-ignored` elsewhere
//! never reaches for a server. `BUN` overrides the bun binary (default `~/.bun/bin/bun`).
#![expect(clippy::expect_used, reason = "tests may expect")]

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// The variable that turns the run on.
const SWITCH: &str = "FT_MODULE_INTEGRATION";

fn bun() -> PathBuf {
    env::var_os("BUN").map_or_else(
        || {
            let home = env::var_os("HOME").expect("HOME is set");
            PathBuf::from(home).join(".bun/bin/bun")
        },
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs the local SpacetimeDB server and bun; set FT_MODULE_INTEGRATION=1 and pass --ignored"]
fn module_integration_script_passes_on_the_local_server() {
    if env::var_os(SWITCH).is_none() {
        eprintln!("{SWITCH} is not set; skipping the local-server integration run");
        return;
    }
    let tools = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let status = Command::new(bun())
        .args(["run", "integration"])
        .current_dir(&tools)
        .status()
        .expect("bun starts");
    assert!(
        status.success(),
        "bun run integration failed ({status}); its pass and FAIL lines are above"
    );
}
