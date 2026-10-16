//! Every ledger write as a serialisable command, mirroring `flowtrace/src/lib/ledger/commands.ts`.
//!
//! The SpacetimeDB module's `run_command` reducer and the desktop apply the same command, so one
//! dispatcher means one set of rules wherever the books live (`commands.ts:1-3`). A command
//! arrives as untrusted JSON: [`sanitize_command`] turns it into a typed [`LedgerCommand`] or
//! rejects it, and [`apply_command`] runs it through the [`Ledger`] and returns the
//! [`CommandResult`].
//!
//! The JSON shape is the TS union's: a `type` field with the TS literal (`entry.post`), payload
//! fields in camelCase (`entryId`), inputs in the engine's `snake_case` (`commands.ts:25-79`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::{
    AccountPatch, EntryOptions, Error, Ledger, NewAccount, NewBill, NewBudgetLine, NewClaim,
    NewGrant, NewVendor, PaymentInput, ReimburseInput,
};
pub use crate::engine::{NewBankAccount, StatementCounter, StatementRow};
use crate::errors::{IssueCode, LedgerError};
pub use crate::reconcile::{AmbiguousMatch, AutoMatchResult, AutoMatched, MatchCandidate};
use crate::reconcile::{DEFAULT_WINDOW_DAYS, auto_match};
use crate::types::{
    AccountId, Actor, BankAccountId, BillId, ClaimId, EntryId, EntryInput, LineId, MatchKind,
    Nullish, PeriodId, StatementLineId,
};

/// A ledger write (`LedgerCommand`, `commands.ts:25-79`), tagged by `type` as in TS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LedgerCommand {
    /// `entry.post`: post a new entry.
    #[serde(rename = "entry.post")]
    EntryPost {
        /// The entry.
        input: EntryInput,
    },
    /// `entry.saveDraft`: save a new draft.
    #[serde(rename = "entry.saveDraft")]
    EntrySaveDraft {
        /// The entry.
        input: EntryInput,
    },
    /// `entry.updateDraft`: replace a draft.
    #[serde(rename = "entry.updateDraft", rename_all = "camelCase")]
    EntryUpdateDraft {
        /// The draft.
        entry_id: EntryId,
        /// Its new content.
        input: EntryInput,
    },
    /// `entry.postDraft`: post a draft.
    #[serde(rename = "entry.postDraft", rename_all = "camelCase")]
    EntryPostDraft {
        /// The draft.
        entry_id: EntryId,
        /// Overrides a restricted-fund rule, with a reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        override_reason: Option<String>,
    },
    /// `entry.deleteDraft`: delete a draft.
    #[serde(rename = "entry.deleteDraft", rename_all = "camelCase")]
    EntryDeleteDraft {
        /// The draft.
        entry_id: EntryId,
    },
    /// `entry.reverse`: reverse a posted entry.
    #[serde(rename = "entry.reverse", rename_all = "camelCase")]
    EntryReverse {
        /// The posted entry.
        entry_id: EntryId,
        /// Date of the reversal, `YYYY-MM-DD`; the posting checks it.
        date: String,
        /// Why.
        reason: String,
    },
    /// `entry.import`: save each input as an import draft.
    #[serde(rename = "entry.import")]
    EntryImport {
        /// The entries.
        inputs: Vec<EntryInput>,
    },
    /// `account.add`: add an account to the chart.
    #[serde(rename = "account.add")]
    AccountAdd {
        /// The account.
        input: NewAccount,
    },
    /// `account.update`: rename, (de)activate or re-classify an account.
    #[serde(rename = "account.update", rename_all = "camelCase")]
    AccountUpdate {
        /// The account.
        account_id: AccountId,
        /// The change.
        patch: AccountPatch,
    },
    /// `grant.add`: add a grant.
    #[serde(rename = "grant.add")]
    GrantAdd {
        /// The grant.
        input: NewGrant,
    },
    /// `budgetLine.set`: add or replace a budget line.
    #[serde(rename = "budgetLine.set")]
    BudgetLineSet {
        /// The line.
        input: NewBudgetLine,
    },
    /// `period.close`: close a period.
    #[serde(rename = "period.close", rename_all = "camelCase")]
    PeriodClose {
        /// The period.
        period_id: PeriodId,
    },
    /// `period.reopen`: reopen a closed period.
    #[serde(rename = "period.reopen", rename_all = "camelCase")]
    PeriodReopen {
        /// The period.
        period_id: PeriodId,
        /// Why.
        reason: String,
    },
    /// `period.lock`: lock a closed period for good.
    #[serde(rename = "period.lock", rename_all = "camelCase")]
    PeriodLock {
        /// The period.
        period_id: PeriodId,
    },
    /// `year.close`: post the year-end close.
    #[serde(rename = "year.close")]
    YearClose {
        /// The fiscal year.
        year: i32,
    },
    /// `year.open`: open a fiscal year's periods.
    #[serde(rename = "year.open")]
    YearOpen {
        /// The fiscal year.
        year: i32,
    },
    /// `vendor.add`: add a vendor.
    #[serde(rename = "vendor.add")]
    VendorAdd {
        /// The vendor.
        input: NewVendor,
    },
    /// `bill.create`: record a vendor bill.
    #[serde(rename = "bill.create")]
    BillCreate {
        /// The bill.
        input: NewBill,
    },
    /// `bill.approve`: approve a bill.
    #[serde(rename = "bill.approve", rename_all = "camelCase")]
    BillApprove {
        /// The bill.
        bill_id: BillId,
    },
    /// `bill.post`: post an approved bill.
    #[serde(rename = "bill.post", rename_all = "camelCase")]
    BillPost {
        /// The bill.
        bill_id: BillId,
        /// Overrides a restricted-fund rule, with a reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        override_reason: Option<String>,
    },
    /// `bill.pay`: pay a posted bill, wholly or in part. It takes no rate: a foreign bill is paid
    /// at its own rate (`commands.ts:184-191`).
    #[serde(rename = "bill.pay", rename_all = "camelCase")]
    BillPay {
        /// The bill.
        bill_id: BillId,
        /// Payment date, `YYYY-MM-DD`; the posting checks it.
        date: String,
        /// The cash account the money left.
        cash_account_id: AccountId,
        /// Amount in the bill's currency, as the raw JS number sent.
        #[serde(serialize_with = "crate::js_number::serialize")]
        amount: f64,
        /// Bank or mobile-money reference.
        #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
        reference: Nullish<String>,
    },
    /// `claim.create`: record an expense claim, and submit it when `submit` is set.
    #[serde(rename = "claim.create")]
    ClaimCreate {
        /// The claim.
        input: NewClaim,
    },
    /// `claim.submit`: submit a draft claim.
    #[serde(rename = "claim.submit", rename_all = "camelCase")]
    ClaimSubmit {
        /// The claim.
        claim_id: ClaimId,
    },
    /// `claim.approve`: approve and post a submitted claim.
    #[serde(rename = "claim.approve", rename_all = "camelCase")]
    ClaimApprove {
        /// The claim.
        claim_id: ClaimId,
        /// Approver's note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        /// Overrides a restricted-fund rule, with a reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        override_reason: Option<String>,
    },
    /// `claim.reject`: send a submitted claim back.
    #[serde(rename = "claim.reject", rename_all = "camelCase")]
    ClaimReject {
        /// The claim.
        claim_id: ClaimId,
        /// Why.
        note: String,
    },
    /// `claim.reimburse`: pay what is owed on an approved claim.
    #[serde(rename = "claim.reimburse", rename_all = "camelCase")]
    ClaimReimburse {
        /// The claim.
        claim_id: ClaimId,
        /// Payment date, `YYYY-MM-DD`; the posting checks it.
        date: String,
        /// The cash account the money left.
        cash_account_id: AccountId,
        /// Bank or mobile-money reference.
        #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
        reference: Nullish<String>,
    },
    /// `bank.add`: add a bank, mobile money or cash account.
    #[serde(rename = "bank.add")]
    BankAdd {
        /// The account.
        input: NewBankAccount,
    },
    /// `statement.import`: import statement rows.
    #[serde(rename = "statement.import", rename_all = "camelCase")]
    StatementImport {
        /// The account the statement belongs to.
        bank_account_id: BankAccountId,
        /// The rows.
        rows: Vec<StatementRow>,
    },
    /// `statement.autoMatch`: match open statement lines to journal lines.
    #[serde(rename = "statement.autoMatch", rename_all = "camelCase")]
    StatementAutoMatch {
        /// The account.
        bank_account_id: BankAccountId,
    },
    /// `statement.match`: match one statement line by hand.
    #[serde(rename = "statement.match", rename_all = "camelCase")]
    StatementMatch {
        /// The statement line.
        statement_line_id: StatementLineId,
        /// The journal line.
        journal_line_id: LineId,
    },
    /// `statement.unmatch`: undo a match.
    #[serde(rename = "statement.unmatch", rename_all = "camelCase")]
    StatementUnmatch {
        /// The statement line.
        statement_line_id: StatementLineId,
    },
    /// `statement.post`: post an entry for an unmatched statement line.
    #[serde(rename = "statement.post", rename_all = "camelCase")]
    StatementPost {
        /// The statement line.
        statement_line_id: StatementLineId,
        /// The other side of the entry.
        counter: StatementCounter,
    },
}

/// What a command returns (`CommandResult`, `commands.ts:81-85`), tagged by `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandResult {
    /// `{ type: 'id', id }`: the record written.
    #[serde(rename = "id")]
    Id {
        /// Its id.
        id: String,
    },
    /// `{ type: 'ids', ids }`: the records written.
    #[serde(rename = "ids")]
    Ids {
        /// Their ids, in order.
        ids: Vec<String>,
    },
    /// `{ type: 'autoMatch', result }`: what `statement.autoMatch` did.
    #[serde(rename = "autoMatch")]
    AutoMatch {
        /// The matches.
        result: AutoMatchResult,
    },
    /// `{ type: 'none' }`: nothing to name.
    #[serde(rename = "none")]
    None,
}

/// Why [`apply_command`] did not apply a command.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    /// The engine refused it; the books are unchanged.
    #[error(transparent)]
    Engine(#[from] Error),
}

/// The `type` of every command (`COMMAND_TYPES`, `commands.ts:92-98`).
pub const COMMAND_TYPES: [&str; 32] = [
    "entry.post",
    "entry.saveDraft",
    "entry.updateDraft",
    "entry.postDraft",
    "entry.deleteDraft",
    "entry.reverse",
    "entry.import",
    "account.add",
    "account.update",
    "grant.add",
    "budgetLine.set",
    "period.close",
    "period.reopen",
    "period.lock",
    "year.close",
    "year.open",
    "vendor.add",
    "bill.create",
    "bill.approve",
    "bill.post",
    "bill.pay",
    "claim.create",
    "claim.submit",
    "claim.approve",
    "claim.reject",
    "claim.reimburse",
    "bank.add",
    "statement.import",
    "statement.autoMatch",
    "statement.match",
    "statement.unmatch",
    "statement.post",
];

/// `INVALID_INPUT` with `message`.
fn invalid(message: impl Into<String>) -> LedgerError {
    LedgerError::fail(IssueCode::InvalidInput, message)
}

/// `asManual(value, source)` (`commands.ts:112-117`): an entry needs a date, a memo and lines,
/// and whatever a client says, it is `source` (`manual` or `import`) with no source record.
///
/// A line's `fx_rate` that is not a JSON number is dropped. TS keeps it and the posting refuses
/// it with `FX_RATE_REQUIRED` when the line is in a foreign currency (`typeof rate !== 'number'`,
/// `engine.ts:373-375`) and uses 1 when it is not, which is what the engine does with no rate,
/// so a post gives the same outcome; the typed [`EntryInput`] cannot hold a string.
fn as_manual(value: Option<&mut Value>, source: &str) -> Result<(), LedgerError> {
    let is = |o: &serde_json::Map<String, Value>, key: &str, test: fn(&Value) -> bool| {
        o.get(key).is_some_and(test)
    };
    let Some(input) = value.and_then(Value::as_object_mut).filter(|o| {
        is(o, "lines", Value::is_array)
            && is(o, "memo", Value::is_string)
            && is(o, "date", Value::is_string)
    }) else {
        return Err(invalid("An entry needs a date, a memo and lines."));
    };
    input.insert("source_type".to_owned(), Value::from(source));
    input.insert("source_id".to_owned(), Value::Null);
    for line in input
        .get_mut("lines")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        if let Some(line) = line.as_object_mut()
            && line.get("fx_rate").is_some_and(|r| !r.is_number())
        {
            line.remove("fx_rate");
        }
    }
    Ok(())
}

/// `sanitizeCommand(raw)` (`commands.ts:100-138`). Commands arrive as JSON from a client, so
/// nothing in them is trusted: an entry a person writes is always `manual` (or `import`), never a
/// system source such as a bill or the year-end close, and new records never choose their own
/// ids, except claims, whose client id makes an offline resend idempotent.
///
/// Where TS goes on with JSON that does not fit the command's type and lets the engine stumble
/// on it, Rust stops here: a missing or wrong-typed field is `INVALID_INPUT` naming it.
///
/// # Errors
///
/// `INVALID_INPUT`: `Unknown command.` for anything but an object with a known `type`
/// (`commands.ts:108-109`); `An entry needs a date, a memo and lines.` (`commands.ts:113-115`);
/// `Nothing to import.` when `inputs` is not a list (`commands.ts:125`); and a malformed
/// command.
pub fn sanitize_command(raw: &Value) -> Result<LedgerCommand, LedgerError> {
    let kind = raw
        .as_object()
        .and_then(|o| o.get("type"))
        .and_then(Value::as_str)
        .filter(|k| COMMAND_TYPES.contains(k))
        .ok_or_else(|| invalid("Unknown command."))?;
    let mut cmd = raw.clone();
    match kind {
        // commands.ts:119-123
        "entry.post" | "entry.saveDraft" | "entry.updateDraft" => {
            as_manual(cmd.get_mut("input"), "manual")?;
        }
        // commands.ts:124-127
        "entry.import" => {
            let inputs = cmd
                .get_mut("inputs")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| invalid("Nothing to import."))?;
            for input in inputs {
                as_manual(Some(input), "import")?;
            }
        }
        // commands.ts:128-134
        "account.add" | "grant.add" | "vendor.add" | "bank.add" | "bill.create" => {
            if let Some(input) = cmd.get_mut("input").and_then(Value::as_object_mut) {
                input.remove("id");
            }
        }
        _ => {}
    }
    serde_json::from_value(cmd)
        .map_err(|e| invalid(format!("The {kind} command is malformed: {e}.")))
}

/// `id(x)` of `applyCommand` (`commands.ts:140`): the id of what was written.
fn id(x: impl AsRef<str>) -> CommandResult {
    CommandResult::Id {
        id: x.as_ref().to_owned(),
    }
}

/// `applyCommand(ledger, actor, cmd)` (`commands.ts:139-222`): runs the command through the
/// engine method it names.
///
/// A rejected command leaves the books as they were. TS has the caller discard them
/// (`commands.ts:88-91`); the engine checks every rule before its first change, and
/// `entry.import`, the one command of several writes, puts the books back itself. Ids drawn
/// before a failure stay drawn, as in TS.
///
/// # Errors
///
/// [`CommandError::Engine`] when a rule blocks the command.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per command, as the applyCommand switch (commands.ts:141-221)"
)]
pub fn apply_command(
    ledger: &mut Ledger<'_>,
    actor: &Actor,
    cmd: LedgerCommand,
) -> Result<CommandResult, CommandError> {
    use LedgerCommand as C;
    Ok(match cmd {
        C::EntryPost { input } => id(ledger.post(actor, &input, EntryOptions::default())?.id),
        C::EntrySaveDraft { input } => id(ledger.save_draft(actor, &input)?.id),
        C::EntryUpdateDraft { entry_id, input } => {
            id(ledger.update_draft(actor, &entry_id, &input)?.id)
        }
        C::EntryPostDraft {
            entry_id,
            override_reason,
        } => id(ledger
            .post_draft(actor, &entry_id, override_reason.as_deref())?
            .id),
        C::EntryDeleteDraft { entry_id } => {
            ledger.delete_draft(actor, &entry_id)?;
            CommandResult::None
        }
        C::EntryReverse {
            entry_id,
            date,
            reason,
        } => id(ledger.reverse(actor, &entry_id, &date, &reason)?.id),
        // commands.ts:155-156
        C::EntryImport { inputs } => {
            let ids = ledger.atomically(|l| {
                inputs
                    .iter()
                    .map(|input| Ok(l.save_draft(actor, input)?.id.as_str().to_owned()))
                    .collect()
            })?;
            CommandResult::Ids { ids }
        }
        C::AccountAdd { input } => id(ledger.add_account(actor, input)?.id),
        C::AccountUpdate { account_id, patch } => {
            id(ledger.update_account(actor, &account_id, &patch)?.id)
        }
        C::GrantAdd { input } => id(ledger.add_grant(actor, input)?.id),
        C::BudgetLineSet { input } => id(ledger.set_budget_line(actor, input)?.id),
        C::PeriodClose { period_id } => id(ledger.close_period(actor, &period_id)?.id),
        C::PeriodReopen { period_id, reason } => {
            id(ledger.reopen_period(actor, &period_id, &reason)?.id)
        }
        C::PeriodLock { period_id } => id(ledger.lock_period(actor, &period_id)?.id),
        // commands.ts:171-172: no closing entry when there is nothing to close.
        C::YearClose { year } => match ledger.close_year(actor, year)? {
            Some(entry) => id(entry.id),
            None => CommandResult::None,
        },
        C::YearOpen { year } => {
            ledger.open_fiscal_year(actor, year)?;
            CommandResult::None
        }
        C::VendorAdd { input } => id(ledger.add_vendor(actor, input)?.id),
        C::BillCreate { input } => id(ledger.create_bill(actor, input)?.id),
        C::BillApprove { bill_id } => id(ledger.approve_bill(actor, &bill_id)?.id),
        C::BillPost {
            bill_id,
            override_reason,
        } => id(ledger
            .post_bill(actor, &bill_id, override_reason.as_deref())?
            .id),
        // commands.ts:184-191: no rate is passed.
        C::BillPay {
            bill_id,
            date,
            cash_account_id,
            amount,
            reference,
        } => {
            let input = PaymentInput {
                date,
                cash_account_id,
                amount,
                reference,
                fx_rate: None,
            };
            id(ledger.pay_bill(actor, &bill_id, &input)?.id)
        }
        C::ClaimCreate { input } => id(ledger.create_claim(actor, input)?.id),
        C::ClaimSubmit { claim_id } => id(ledger.submit_claim(actor, &claim_id)?.id),
        C::ClaimApprove {
            claim_id,
            note,
            override_reason,
        } => id(ledger
            .approve_claim(
                actor,
                &claim_id,
                note.as_deref(),
                override_reason.as_deref(),
            )?
            .id),
        C::ClaimReject { claim_id, note } => id(ledger.reject_claim(actor, &claim_id, &note)?.id),
        C::ClaimReimburse {
            claim_id,
            date,
            cash_account_id,
            reference,
        } => {
            let input = ReimburseInput {
                date,
                cash_account_id,
                reference,
            };
            id(ledger.reimburse_claim(actor, &claim_id, &input)?.id)
        }
        // commands.ts:209-220
        C::BankAdd { input } => id(ledger.add_bank_account(actor, input)?.id),
        C::StatementImport {
            bank_account_id,
            rows,
        } => CommandResult::Ids {
            ids: ledger
                .import_statement(actor, &bank_account_id, &rows)?
                .into_iter()
                .map(|l| l.id.into())
                .collect(),
        },
        C::StatementAutoMatch { bank_account_id } => CommandResult::AutoMatch {
            result: auto_match(ledger, actor, &bank_account_id, DEFAULT_WINDOW_DAYS)?,
        },
        C::StatementMatch {
            statement_line_id,
            journal_line_id,
        } => id(ledger
            .match_statement_line(
                actor,
                &statement_line_id,
                &journal_line_id,
                MatchKind::Manual,
            )?
            .id),
        C::StatementUnmatch { statement_line_id } => {
            id(ledger.unmatch_statement_line(actor, &statement_line_id)?.id)
        }
        C::StatementPost {
            statement_line_id,
            counter,
        } => id(ledger
            .post_from_statement_line(actor, &statement_line_id, &counter)?
            .id),
    })
}
