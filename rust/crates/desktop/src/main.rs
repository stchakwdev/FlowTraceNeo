//! FlowTrace Books desktop app: opens the shell over the in-process backend.
//!
//! ```text
//! desktop [--theme light|dark] [--screen books|connect] [--entry 1-7] [--report <id>]
//!         [--large-books LINES] [--gl-account CODE] [--gl-row N] [--gl-scroll-bench on]
//!         [--grants list|budgets|<grant id>] [--server <url>] [--database <name>]
//!         [--timings on] [--config <path>] [--sign-in on]
//! ```
//!
//! Without `--theme` the app follows the system appearance. `--screen` and `--entry` choose
//! what shows first, for screenshots (README.md); `--report` opens Reports on one report:
//! `trial-balance`, `position`, `activities`, `functional`, `cash-flow` or `grant`.
//! `--large-books` adds synthetic books with that many posted lines, selected first; the
//! `--gl-*` flags filter the General ledger on an account code, open a row, or scroll the whole
//! table once and log the frame times. `--grants` opens the Grants entry on its list, its
//! budget tab or one grant (with `--entry 5`). `--server` and `--database` open the books on a
//! SpacetimeDB server instead of the demo, over `flowtrace-books/config.toml` (README.md,
//! Configuration); `--timings on` writes the time to the first books and each write's round
//! trip to stderr. `--config` reads another config file (token files then default to its
//! folder); `--sign-in on` starts the WorkOS sign-in at launch, as the Connect screen's button
//! does (README.md, Signing in with WorkOS).

use anyhow::{Context as _, bail};
use desktop::app::{Launch, run};
use desktop::model::{NavEntry, Screen};
use desktop::statements::ReportKind;

fn parse(mut args: impl Iterator<Item = String>) -> anyhow::Result<Launch> {
    let mut launch = Launch::default();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .with_context(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--theme" => {
                launch.dark = Some(match value.as_str() {
                    "light" => false,
                    "dark" => true,
                    other => bail!("--theme is light or dark, not {other}"),
                });
            }
            "--screen" => {
                launch.nav.screen = match value.as_str() {
                    "books" => Screen::Books,
                    "connect" => Screen::Connect,
                    other => bail!("--screen is books or connect, not {other}"),
                };
            }
            "--entry" => {
                launch.nav.entry = NavEntry::from_key(&value)
                    .with_context(|| format!("--entry is 1 to 7, not {value}"))?;
            }
            "--report" => {
                launch.report = Some(
                    ReportKind::from_id(&value)
                        .with_context(|| format!("--report is a report id, not {value}"))?,
                );
            }
            "--large-books" => {
                launch.large_books = Some(
                    value
                        .parse()
                        .with_context(|| format!("--large-books is a line count, not {value}"))?,
                );
            }
            "--gl-account" => launch.general_ledger.account_code = Some(value),
            "--gl-row" => {
                launch.general_ledger.row = Some(
                    value
                        .parse()
                        .with_context(|| format!("--gl-row is a row number, not {value}"))?,
                );
            }
            "--grants" => launch.grants = Some(value),
            "--gl-scroll-bench" => launch.general_ledger.scroll_bench = value == "on",
            "--server" => launch.server = Some(value),
            "--database" => launch.database = Some(value),
            "--timings" => launch.timings = value == "on",
            "--config" => launch.config = Some(value.into()),
            "--sign-in" => launch.sign_in = value == "on",
            other => bail!("unknown option {other}"),
        }
    }
    Ok(launch)
}

fn main() -> anyhow::Result<()> {
    let launch = parse(std::env::args().skip(1))?;
    run(launch)
}
