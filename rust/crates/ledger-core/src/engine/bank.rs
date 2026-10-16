//! Bank, mobile money and cash accounts and their statement lines (`engine.ts:1141-1253`).

use serde::{Deserialize, Serialize};

use super::{EntryOptions, Error, Ledger, Permission, fail, js_trim, require};
use crate::errors::IssueCode;
use crate::money::assert_minor;
use crate::types::{
    AccountId, Actor, BankAccount, BankAccountId, BankAccountKind, EntryInput, FundId, GrantId,
    ImportBatchId, IsoDate, JournalEntry, LineId, LineInput, MatchKind, Minor, Nullish, SourceType,
    StatementLine, StatementLineId,
};

/// The input of `addBankAccount` (`engine.ts:1144`): `Omit<BankAccount, 'id'> & { id?: string }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewBankAccount {
    /// Id; drawn from the env when absent. A client never chooses it (`commands.ts:128-134`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<BankAccountId>,
    /// Display name.
    pub name: String,
    /// Bank, mobile money or cash.
    pub kind: BankAccountKind,
    /// Provider, for example `Equity Bank`.
    pub provider: String,
    /// The GL cash account; it must be a cash account of the chart.
    pub account_id: AccountId,
    /// Account currency.
    pub currency: String,
    /// Masked number.
    pub number_masked: String,
}

/// One row of `importStatement`: `Pick<StatementLine, 'date' | 'description' | 'reference' |
/// 'amount'>` (`engine.ts:1161`, `commands.ts:67-71`).
///
/// The amount is the raw JS number sent and is checked by `assertMinor` (`engine.ts:1177`). The
/// date is a typed [`IsoDate`]: a malformed date fails to deserialise, where TS stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementRow {
    /// Value date.
    pub date: IsoDate,
    /// Statement text.
    pub description: String,
    /// Statement reference.
    pub reference: Option<String>,
    /// Signed amount in the account's currency: money in > 0, money out < 0.
    #[serde(with = "crate::js_number")]
    pub amount: f64,
}

/// The other side of `postFromStatementLine` (`engine.ts:1233`, `commands.ts:75-79`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementCounter {
    /// The account to post against; the suspense account when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<AccountId>,
    /// The fund of both lines.
    pub fund_id: FundId,
    /// The grant of the counter line, if any.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub grant_id: Nullish<GrantId>,
    /// Entry memo; the statement description when absent or blank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// The duplicate key of a statement line (`engine.ts:1166-1167`): date, amount, and the
/// reference, or the description when there is no reference (`??`, so an empty reference
/// counts).
fn statement_key(
    date: &IsoDate,
    amount: Minor,
    reference: Option<&str>,
    description: &str,
) -> String {
    format!("{date}|{amount}|{}", reference.unwrap_or(description))
}

impl Ledger<'_> {
    /// `addBankAccount` (`engine.ts:1144-1151`): adds a bank, mobile money or cash account
    /// linked to a cash account of the chart. Needs `can_post`; the linked account must exist and
    /// be a cash account (`INVALID_INPUT`). No uniqueness check on the id and no audit event, as
    /// in TS. Draws the id only when none is given.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn add_bank_account(
        &mut self,
        actor: &Actor,
        input: NewBankAccount,
    ) -> Result<BankAccount, Error> {
        let _ = (&self, actor);
        fail(IssueCode::InvalidState, "not ported")
    }

    /// `importStatement` (`engine.ts:1153-1197`): adds statement lines to a bank account,
    /// skipping rows already imported. A row is a repeat while earlier imports on the account
    /// hold more lines with its key (date, amount, and the reference or else the description)
    /// than this file has used up, so two identical rows in one file both count
    /// (`engine.ts:1168-1170`). Needs `can_post`; the account must exist (`NOT_FOUND`); every
    /// amount must be whole minor units ([`Error::Money`]). Draws the import batch id, then one
    /// id per line added, then one audit id (`statement.import`, subject the bank account,
    /// detail `<n> lines`). Returns the lines added.
    ///
    /// TS checks each amount as it reaches the row, after drawing the batch id; Rust checks them
    /// all before drawing anything.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above, [`Error::Money`] for a fractional amount.
    pub fn import_statement(
        &mut self,
        actor: &Actor,
        bank_account_id: &BankAccountId,
        rows: &[StatementRow],
    ) -> Result<Vec<StatementLine>, Error> {
        let _ = (&self, actor);
        fail(IssueCode::InvalidState, "not ported")
    }

    /// `matchStatementLine` (`engine.ts:1199-1215`): links a statement line to a posted journal
    /// line on the same account, for the same signed amount. Needs `can_post`. The statement
    /// line and the journal line must exist (`NOT_FOUND`); the journal line must be on the bank
    /// account's GL account and carry the statement amount (`INVALID_INPUT`); its entry must be
    /// posted (`INVALID_STATE`); no other statement line may hold it (`DUPLICATE`). Draws one
    /// audit id (`statement.match`, detail `<kind> <journal line id>`).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn match_statement_line(
        &mut self,
        actor: &Actor,
        statement_line_id: &StatementLineId,
        journal_line_id: &LineId,
        kind: MatchKind,
    ) -> Result<StatementLine, Error> {
        let _ = (&self, actor);
        fail(IssueCode::InvalidState, "not ported")
    }

    /// `unmatchStatementLine` (`engine.ts:1217-1224`): clears a statement line's match, matched
    /// or not. Needs `can_post`; the line must exist (`NOT_FOUND`). Draws one audit id
    /// (`statement.unmatch`, empty detail).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn unmatch_statement_line(
        &mut self,
        actor: &Actor,
        statement_line_id: &StatementLineId,
    ) -> Result<StatementLine, Error> {
        let _ = (&self, actor);
        fail(IssueCode::InvalidState, "not ported")
    }

    /// `postFromStatementLine` (`engine.ts:1226-1252`): posts an entry for an unmatched
    /// statement line, the bank's GL account against `counter.account_id` (or the suspense
    /// account), both lines in the counter's fund, the grant on the counter line only, then
    /// matches the statement line to the cash line by hand. The entry is dated as the statement
    /// line, its memo is the trimmed counter memo or else the statement description, its source
    /// is the statement line. The line must exist (`NOT_FOUND`) and be unmatched
    /// (`INVALID_STATE`); the posting checks the rest, `can_post` among them. Draws the ids of
    /// [`Ledger::post`], then the `statement.match` audit id.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above and those of the posting.
    pub fn post_from_statement_line(
        &mut self,
        actor: &Actor,
        statement_line_id: &StatementLineId,
        counter: &StatementCounter,
    ) -> Result<JournalEntry, Error> {
        let _ = (&self, actor);
        fail(IssueCode::InvalidState, "not ported")
    }

    fn statement_line_index(&self, id: &StatementLineId) -> Option<usize> {
        self.state.statement_lines.iter().position(|x| &x.id == id)
    }

    /// The bank account of a statement line. TS asserts it exists (`engine.ts:1204`, `1237`);
    /// books with a statement line on an unknown account answer `NOT_FOUND`.
    fn bank_of(&self, s: &StatementLine) -> Result<&BankAccount, Error> {
        match self
            .state
            .bank_accounts
            .iter()
            .find(|b| b.id == s.bank_account_id)
        {
            Some(b) => Ok(b),
            None => fail(IssueCode::NotFound, "Bank account not found."),
        }
    }
}
