//! Where the engine gets ids and time: the Rust form of `LedgerEnv`
//! (`flowtrace/src/lib/ledger/engine.ts:41-44`).
//!
//! The engine never reads a clock or a random source itself. Every audit event, journal line,
//! statement line, import batch and coded line draws one id in call order, so replaying a fixture
//! with the same env reproduces its ids (`rust/fixtures/README.md`, "How ids and time are
//! injected").

use std::fmt;

use serde::{Deserialize, Serialize};

/// An instant as TS writes it with `Date.prototype.toISOString`: `2026-02-04T09:00:00.000Z`, UTC
/// with milliseconds.
///
/// A validated newtype over the string rather than a `chrono` or `time` value: the engine only
/// stores timestamps and compares them as strings (`types.ts:130-132`, `types.ts:214`,
/// `types.ts:264`), never does arithmetic on them, and the fixtures need the exact TS bytes back.
/// Keeping it a string also keeps ledger-core free of date dependencies on the module's wasm
/// target (pre-decision P5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Timestamp(String);

/// A string that is not `YYYY-MM-DDTHH:MM:SS.mmmZ`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not an ISO-8601 UTC timestamp with milliseconds: {0:?}")]
pub struct TimestampError(String);

impl Timestamp {
    /// Accepts exactly the `toISOString` shape `YYYY-MM-DDTHH:MM:SS.mmmZ` (digits checked by
    /// position; calendar ranges are not checked, as the engine never reads the parts).
    ///
    /// # Errors
    ///
    /// [`TimestampError`] for any other shape.
    pub fn parse(s: &str) -> Result<Self, TimestampError> {
        const SHAPE: &[u8; 24] = b"dddd-dd-ddTdd:dd:dd.dddZ";
        let ok = s.len() == SHAPE.len()
            && s.bytes().zip(SHAPE).all(|(b, &want)| {
                if want == b'd' {
                    b.is_ascii_digit()
                } else {
                    b == want
                }
            });
        if ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(TimestampError(s.to_owned()))
        }
    }

    /// The timestamp as TS writes it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Timestamp {
    type Error = TimestampError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<Timestamp> for String {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Ids and time for the engine, as `LedgerEnv { now, newId }` (`engine.ts:41-44`).
pub trait LedgerEnv {
    /// The current instant, as `env.now()`.
    fn now(&self) -> Timestamp;

    /// A fresh id, as `env.newId()`. Callers wrap it in the id type they need.
    fn new_id(&mut self) -> String;
}

/// The frozen envs the fixtures were written with. Every counter starts at 1 for each new env,
/// as in TS, so make one env per replayed ledger.
pub mod fixed {
    use super::{LedgerEnv, Timestamp};

    /// `now` of the demo and fixture envs (`flowtrace/src/lib/demo/ledger.ts:118-124`).
    pub const DEMO_NOW: &str = "2026-02-04T09:00:00.000Z";
    /// `now` of the test env (`flowtrace/src/lib/ledger/engine.test.ts:16-19`).
    pub const TEST_NOW: &str = "2026-03-01T00:00:00.000Z";

    /// How a fixed env spells the n-th id.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum IdForm {
        /// `demo-` and n in base 36, zero-padded to 4 (`demo-0001`, `demo-000a`).
        Demo,
        /// `fx-` and n in decimal, zero-padded to 4 (`fx-0001`).
        Fixture,
        /// `id-` and n in decimal (`id-1`).
        Test,
    }

    /// A [`LedgerEnv`] with a frozen clock and a counting id source.
    #[derive(Debug, Clone)]
    pub struct FixedEnv {
        now: Timestamp,
        form: IdForm,
        drawn: u64,
    }

    impl FixedEnv {
        fn new(now: &str, form: IdForm) -> Self {
            Self {
                now: Timestamp(now.to_owned()),
                form,
                drawn: 0,
            }
        }

        /// How many ids this env has drawn.
        #[must_use]
        pub fn drawn(&self) -> u64 {
            self.drawn
        }
    }

    /// `n` in lower-case base 36, as JS `n.toString(36)`.
    fn base36(mut n: u64) -> String {
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut out = Vec::new();
        loop {
            out.push(DIGITS[usize::try_from(n % 36).unwrap_or(0)]);
            n /= 36;
            if n == 0 {
                break;
            }
        }
        out.reverse();
        String::from_utf8(out).unwrap_or_default()
    }

    impl LedgerEnv for FixedEnv {
        fn now(&self) -> Timestamp {
            self.now.clone()
        }

        fn new_id(&mut self) -> String {
            self.drawn += 1;
            let n = self.drawn;
            match self.form {
                IdForm::Demo => format!("demo-{:0>4}", base36(n)),
                IdForm::Fixture => format!("fx-{n:04}"),
                IdForm::Test => format!("id-{n}"),
            }
        }
    }

    /// The demo env `seedEnv` (`flowtrace/src/lib/demo/ledger.ts:118-124`): now
    /// [`DEMO_NOW`], ids `demo-0001`, ..., `demo-0009`, `demo-000a`, ... Used by `demo-books.json`
    /// and `demo-backfill.json`.
    #[must_use]
    pub fn demo() -> FixedEnv {
        FixedEnv::new(DEMO_NOW, IdForm::Demo)
    }

    /// The commands env (`rust/tools/src/commands.ts#commandsEnv`): now [`DEMO_NOW`], ids
    /// `fx-0001`, ... Used by `commands-accepted.json`.
    #[must_use]
    pub fn fixture() -> FixedEnv {
        FixedEnv::new(DEMO_NOW, IdForm::Fixture)
    }

    /// The test env `testEnv` (`flowtrace/src/lib/ledger/engine.test.ts:16-19`): now
    /// [`TEST_NOW`], ids `id-1`, `id-2`, ... Used by `rejected-cases.json`,
    /// `reconcile-auto-match.json` and `setup.json`.
    #[must_use]
    pub fn test() -> FixedEnv {
        FixedEnv::new(TEST_NOW, IdForm::Test)
    }

    #[cfg(test)]
    mod tests {
        use super::base36;

        #[test]
        fn base36_matches_js_to_string_36() {
            assert_eq!(base36(0), "0");
            assert_eq!(base36(10), "a");
            assert_eq!(base36(35), "z");
            assert_eq!(base36(36), "10");
            assert_eq!(base36(207), "5r");
        }
    }
}
