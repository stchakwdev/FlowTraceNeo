//! FlowTrace Books desktop app: opens the shell over the in-process backend.
//!
//! ```text
//! desktop [--theme light|dark] [--screen books|connect] [--entry 1-7] [--report <id>]
//! ```
//!
//! Without `--theme` the app follows the system appearance. `--screen` and `--entry` choose
//! what shows first, for screenshots (README.md); `--report` opens Reports on one report:
//! `trial-balance`, `position`, `activities`, `functional`, `cash-flow` or `grant`.

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
            other => bail!("unknown option {other}"),
        }
    }
    Ok(launch)
}

fn main() -> anyhow::Result<()> {
    let launch = parse(std::env::args().skip(1))?;
    run(launch)
}
