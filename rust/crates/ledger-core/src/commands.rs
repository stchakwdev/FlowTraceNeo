//! Every ledger write as a serialisable command, mirroring `flowtrace/src/lib/ledger/commands.ts`.
//!
//! The SpacetimeDB module's `run_command` reducer and the desktop apply the same command, so one
//! dispatcher means one set of rules wherever the books live (`commands.ts:1-3`). A command
//! arrives as untrusted JSON: [`sanitize_command`] turns it into a typed [`LedgerCommand`] or
//! rejects it, and [`apply_command`] runs it through the [`Ledger`] and returns the
//! [`CommandResult`].
//!
//! The JSON shape is the TS union's: a `type` field with the TS literal (`entry.post`), payload
//! fields in camelCase (`entryId`), inputs in the engine's snake_case (`commands.ts:25-79`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::{
    AccountPatch, EntryOptions, Error, Ledger, NewAccount, NewBill, NewBudgetLine, NewClaim,
    NewGrant, NewVendor, PaymentInput, ReimburseInput,
};
use crate::errors::{IssueCode, LedgerError};
use crate::types::{
    AccountId, Actor, BankAccountId, BankAccountKind, BillId, ClaimId, EntryId, EntryInput, FundId,
    GrantId, JournalEntry, JournalLine, LineId, Nullish, PeriodId, SourceType, StatementLineId,
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
    /// at its own rate (`commands.ts:180-188`).
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
    /// `bank.add`: add a bank, mobile money or cash account (engine method in task C6).
    #[serde(rename = "bank.add")]
    BankAdd {
        /// The account.
        input: NewBankAccount,
    },
    /// `statement.import`: import statement rows (engine method in task C6).
    #[serde(rename = "statement.import", rename_all = "camelCase")]
    StatementImport {
        /// The account the statement belongs to.
        bank_account_id: BankAccountId,
        /// The rows.
        rows: Vec<StatementRow>,
    },
    /// `statement.autoMatch`: match open statement lines to journal lines (task C6).
    #[serde(rename = "statement.autoMatch", rename_all = "camelCase")]
    StatementAutoMatch {
        /// The account.
        bank_account_id: BankAccountId,
    },
    /// `statement.match`: match one statement line by hand (task C6).
    #[serde(rename = "statement.match", rename_all = "camelCase")]
    StatementMatch {
        /// The statement line.
        statement_line_id: StatementLineId,
        /// The journal line.
        journal_line_id: LineId,
    },
    /// `statement.unmatch`: undo a match (task C6).
    #[serde(rename = "statement.unmatch", rename_all = "camelCase")]
    StatementUnmatch {
        /// The statement line.
        statement_line_id: StatementLineId,
    },
    /// `statement.post`: post an entry for an unmatched statement line (task C6).
    #[serde(rename = "statement.post", rename_all = "camelCase")]
    StatementPost {
        /// The statement line.
        statement_line_id: StatementLineId,
        /// The other side of the entry.
        counter: StatementCounter,
    },
}

/// The input of `bank.add`: `Omit<BankAccount, 'id'>` (`commands.ts:66`, `types.ts:238-246`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewBankAccount {
    /// Display name.
    pub name: String,
    /// Bank, mobile money or cash.
    pub kind: BankAccountKind,
    /// Provider, for example `Equity Bank`.
    pub provider: String,
    /// The GL cash account.
    pub account_id: AccountId,
    /// Account currency.
    pub currency: String,
    /// Masked number.
    pub number_masked: String,
}

/// One row of `statement.import`: `Pick<StatementLine, 'date' | 'description' | 'reference' |
/// 'amount'>` (`commands.ts:67-71`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementRow {
    /// Value date.
    pub date: String,
    /// Statement text.
    pub description: String,
    /// Statement reference.
    pub reference: Option<String>,
    /// Signed amount in the account's currency, as the raw JS number sent.
    #[serde(serialize_with = "crate::js_number::serialize")]
    pub amount: f64,
}

/// The other side of `statement.post` (`commands.ts:75-79`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementCounter {
    /// The account to post against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<AccountId>,
    /// The fund.
    pub fund_id: FundId,
    /// The grant, if any.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub grant_id: Nullish<GrantId>,
    /// Entry memo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
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

/// `AutoMatchResult` (`reconcile.ts:101-105`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoMatchResult {
    /// Lines matched, one candidate each.
    pub matched: Vec<AutoMatched>,
    /// Lines with more than one equally good candidate.
    pub ambiguous: Vec<AmbiguousMatch>,
    /// Statement lines with no candidate.
    pub unmatched: Vec<StatementLineId>,
}

/// One automatic match (`reconcile.ts:102`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoMatched {
    /// The statement line.
    pub statement_line_id: StatementLineId,
    /// The journal line.
    pub journal_line_id: LineId,
}

/// A statement line with several candidates (`reconcile.ts:103`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguousMatch {
    /// The statement line.
    pub statement_line_id: StatementLineId,
    /// Its candidates.
    pub candidates: Vec<MatchCandidate>,
}

/// `MatchCandidate` (`reconcile.ts:94-99`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    /// The journal line.
    pub line: JournalLine,
    /// Its entry.
    pub entry: JournalEntry,
    /// Days between the statement and entry dates.
    #[serde(with = "crate::js_number")]
    pub day_gap: f64,
    /// The references agree.
    pub reference_match: bool,
}

/// Why [`apply_command`] did not apply a command.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    /// The engine refused it; the books are unchanged.
    #[error(transparent)]
    Engine(#[from] Error),
    /// A bank or statement command, whose engine method is not ported yet (task C6). Nothing
    /// was changed and no id was drawn.
    #[error("The {0} command is not implemented yet.")]
    NotImplemented(&'static str),
}

/// `sanitizeCommand(raw)` (`commands.ts:94-131`).
///
/// # Errors
///
/// `INVALID_INPUT` when the command is unknown or malformed.
pub fn sanitize_command(raw: &Value) -> Result<LedgerCommand, LedgerError> {
    let _ = raw;
    Err(LedgerError::fail(IssueCode::InvalidInput, "Not yet."))
}

/// `applyCommand(ledger, actor, cmd)` (`commands.ts:133-222`).
///
/// # Errors
///
/// [`CommandError::Engine`] when a rule blocks the command, [`CommandError::NotImplemented`] for
/// the bank and statement commands until task C6.
pub fn apply_command(
    ledger: &mut Ledger<'_>,
    actor: &Actor,
    cmd: LedgerCommand,
) -> Result<CommandResult, CommandError> {
    let _ = (ledger, actor, cmd, EntryOptions::default());
    let _: Option<(PaymentInput, ReimburseInput, SourceType)> = None;
    Err(CommandError::NotImplemented("any"))
}
