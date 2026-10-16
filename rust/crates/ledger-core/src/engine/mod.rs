//! The ledger engine: every write goes through here, mirroring the `Ledger` class of
//! `flowtrace/src/lib/ledger/engine.ts`.
//!
//! The invariants of `Docs/flowtrace-ledger-plan.md` section 5 are checked here rather than in a
//! UI (`engine.ts:1-11`): a posted entry balances in base currency and within each fund; posted
//! entries are immutable and a correction is a reversal plus a new entry; nothing posts into a
//! closed or locked period; restricted spend names its grant, and grant dates and awards block
//! unless an override reason is given; amounts are integer minor units and base amounts are
//! frozen at posting.
//!
//! Shape: [`Ledger`] borrows the books and a [`LedgerEnv`] for writes, as
//! `new Ledger(state, env)` (`engine.ts:103-107`). Reads are methods on [`LedgerState`] and never
//! call the env (`engine-queries.json`). Every id and timestamp comes from the env, drawn in the
//! order the TS engine draws them, so a replay with a fixed env reproduces the fixtures.
//!
//! A rejected write returns before its first change, so the books and the env are left as they
//! were. TS mutates in place and has its callers discard the books on a throw
//! (`commands.ts:85-89`); the outcome is the same, and Rust checks every rule first.

mod budgets;
mod claims;
mod documents;
mod entries;
mod payables;
mod periods;
mod queries;
mod setup;
mod validate;

pub use budgets::{NewBudget, NewBudgetLine};
pub use claims::{NewClaim, ReimburseInput};
pub use documents::NewCodedLine;
pub use payables::{NewBill, NewVendor, PaymentInput};
pub use setup::{AccountPatch, NewAccount, NewDimension, NewFund, NewGrant};
pub use validate::EntryOptions;

use std::collections::BTreeMap;

use crate::env::LedgerEnv;
use crate::errors::{IssueCode, LedgerError};
use crate::money::MoneyError;
use crate::types::{
    AccountType, Actor, AuditEvent, AuditEventId, LedgerState, NormalBalance, OrgLedgerSettings,
};

/// Why an engine call failed: a ledger rule ([`LedgerError`]), or a `RangeError` from
/// `money.ts` ([`MoneyError`]) that TS lets escape the engine, such as a draft line in a foreign
/// currency with no rate (`money.ts:36-39`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A broken ledger rule, with its code and message.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    /// An amount or rate `money.ts` refuses.
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// `fail(code, message)` (`errors.ts:56-58`) as an `Err`.
fn fail<T>(code: IssueCode, message: impl Into<String>) -> Result<T, Error> {
    Err(LedgerError::fail(code, message).into())
}

/// The permissions `require` checks (`engine.ts:154-163`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    /// `can_post`
    Post,
    /// `can_approve`
    Approve,
    /// `can_close`
    Close,
}

/// `require(actor, perm)` (`engine.ts:154-163`): `PERMISSION` naming the actor and the act.
fn require(actor: &Actor, perm: Permission) -> Result<(), Error> {
    let (allowed, what) = match perm {
        Permission::Post => (actor.can_post, "post to the ledger"),
        Permission::Approve => (actor.can_approve, "approve spend"),
        Permission::Close => (actor.can_close, "close periods"),
    };
    if allowed {
        Ok(())
    } else {
        fail(
            IssueCode::Permission,
            format!("{} does not have permission to {what}.", actor.name),
        )
    }
}

/// `String.prototype.trim`: strips JS whitespace and line terminators (ECMA-262 `TrimString`),
/// which include U+FEFF and leave out U+0085, unlike [`str::trim`].
fn js_trim(s: &str) -> &str {
    s.trim_matches(crate::money::is_js_whitespace)
}

/// `s.trim()` is non-empty: the TS test `!s.trim()` negated.
fn has_text(s: &str) -> bool {
    !js_trim(s).is_empty()
}

/// `JSON.stringify` of a string (ECMA-262 `QuoteJSONString`): quotes, backslash, the short
/// escapes and `\u00xx` for the other control characters; everything else as is.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let n = u32::from(c);
                out.push_str("\\u00");
                out.extend([n >> 4, n & 0xf].map(|d| char::from_digit(d, 16).unwrap_or('0')));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `normalBalanceOf(type)` (`engine.ts:95-97`): assets and expenses are debit-normal, the rest
/// credit-normal.
#[must_use]
pub fn normal_balance_of(account_type: AccountType) -> NormalBalance {
    match account_type {
        AccountType::Asset | AccountType::Expense => NormalBalance::Debit,
        AccountType::Liability | AccountType::Equity | AccountType::Income => NormalBalance::Credit,
    }
}

/// `emptyLedger(settings)` (`engine.ts:51-72`): books with the settings and nothing else.
#[must_use]
pub fn empty_ledger(settings: OrgLedgerSettings) -> LedgerState {
    LedgerState {
        settings,
        accounts: Vec::new(),
        funds: Vec::new(),
        grants: Vec::new(),
        dimensions: Vec::new(),
        periods: Vec::new(),
        entries: Vec::new(),
        lines: Vec::new(),
        budgets: Vec::new(),
        budget_lines: Vec::new(),
        vendors: Vec::new(),
        bills: Vec::new(),
        payments: Vec::new(),
        claims: Vec::new(),
        bank_accounts: Vec::new(),
        statement_lines: Vec::new(),
        audit: Vec::new(),
        counters: BTreeMap::new(),
    }
}

/// A read-and-write view over one organisation's books (`engine.ts:99-107`): reads through
/// [`Ledger::state`], writes through the methods, ids and time from the env.
pub struct Ledger<'a> {
    state: &'a mut LedgerState,
    env: &'a mut dyn LedgerEnv,
}

impl<'a> Ledger<'a> {
    /// `new Ledger(state, env)` (`engine.ts:104-107`).
    pub fn new(state: &'a mut LedgerState, env: &'a mut dyn LedgerEnv) -> Self {
        Self { state, env }
    }

    /// The books, for reads.
    #[must_use]
    pub fn state(&self) -> &LedgerState {
        self.state
    }

    /// `nextNumber(prefix)` (`engine.ts:137-141`): bumps the counter and formats `JE-0001`.
    fn next_number(&mut self, prefix: &str) -> String {
        let n = self.state.counters.get(prefix).copied().unwrap_or(0) + 1;
        self.state.counters.insert(prefix.to_owned(), n);
        format!("{prefix}-{n:04}")
    }

    /// `audit(by, action, subject, detail)` (`engine.ts:143-152`): appends one event, drawing
    /// one id.
    fn audit(&mut self, by: &Actor, action: &str, subject: &str, detail: &str) {
        let id = AuditEventId::from(self.env.new_id());
        let at = self.env.now();
        self.state.audit.push(AuditEvent {
            id,
            at,
            by: by.id.clone(),
            action: action.to_owned(),
            subject: subject.to_owned(),
            detail: detail.to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_string_escapes_as_json_stringify() {
        assert_eq!(json_string("Internet, data"), r#""Internet, data""#);
        assert_eq!(json_string("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(json_string("\n\t\u{1}"), r#""\n\t\u0001""#);
        assert_eq!(json_string("caf\u{e9} \u{2019}"), "\"caf\u{e9} \u{2019}\"");
    }

    #[test]
    fn js_trim_follows_ecmascript_whitespace() {
        assert_eq!(js_trim("\u{FEFF} x \u{3000}"), "x");
        assert_eq!(js_trim("\u{85}x"), "\u{85}x");
        assert!(!has_text(" \t\n"));
    }
}
