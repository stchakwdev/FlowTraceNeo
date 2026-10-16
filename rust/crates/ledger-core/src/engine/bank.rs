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
        require(actor, Permission::Post)?;
        if !self
            .state
            .account(&input.account_id)
            .is_some_and(|a| a.is_cash)
        {
            return fail(
                IssueCode::InvalidInput,
                "Link the bank account to a cash account in the chart.",
            );
        }
        let id = match input.id {
            Some(id) => id,
            None => BankAccountId::from(self.env.new_id()),
        };
        let b = BankAccount {
            id,
            name: input.name,
            kind: input.kind,
            provider: input.provider,
            account_id: input.account_id,
            currency: input.currency,
            number_masked: input.number_masked,
        };
        self.state.bank_accounts.push(b.clone());
        Ok(b)
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
        require(actor, Permission::Post)?;
        if !self
            .state
            .bank_accounts
            .iter()
            .any(|b| &b.id == bank_account_id)
        {
            return fail(IssueCode::NotFound, "Bank account not found.");
        }
        let amounts = rows
            .iter()
            .map(|r| assert_minor(r.amount, "Statement amount"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut earlier: Vec<(String, usize)> = Vec::new();
        for s in &self.state.statement_lines {
            if &s.bank_account_id == bank_account_id {
                let k = statement_key(&s.date, s.amount, s.reference.as_deref(), &s.description);
                match earlier.iter_mut().find(|(key, _)| *key == k) {
                    Some((_, n)) => *n += 1,
                    None => earlier.push((k, 1)),
                }
            }
        }
        let batch = ImportBatchId::from(self.env.new_id());
        let mut added = Vec::new();
        for (r, amount) in rows.iter().zip(amounts) {
            let k = statement_key(&r.date, amount, r.reference.as_deref(), &r.description);
            if let Some((_, left)) = earlier.iter_mut().find(|(key, n)| *key == k && *n > 0) {
                *left -= 1;
                continue;
            }
            let line = StatementLine {
                id: StatementLineId::from(self.env.new_id()),
                bank_account_id: bank_account_id.clone(),
                date: r.date.clone(),
                description: r.description.clone(),
                reference: r.reference.clone(),
                amount,
                import_batch: batch.clone(),
                matched_line_id: None,
                match_kind: None,
            };
            self.state.statement_lines.push(line.clone());
            added.push(line);
        }
        self.audit(
            actor,
            "statement.import",
            bank_account_id.as_str(),
            &format!("{} lines", added.len()),
        );
        Ok(added)
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
        require(actor, Permission::Post)?;
        let Some(si) = self.statement_line_index(statement_line_id) else {
            return fail(IssueCode::NotFound, "Statement line not found.");
        };
        let Some(j) = self.state.lines.iter().find(|x| &x.id == journal_line_id) else {
            return fail(IssueCode::NotFound, "Journal line not found.");
        };
        let s = &self.state.statement_lines[si];
        let bank = self.bank_of(s)?;
        if j.account_id != bank.account_id {
            return fail(
                IssueCode::InvalidInput,
                "That journal line is on a different account.",
            );
        }
        if self
            .state
            .entry(&j.entry_id)
            .is_some_and(|e| e.status == crate::types::EntryStatus::Draft)
        {
            return fail(
                IssueCode::InvalidState,
                "Post the entry before matching it.",
            );
        }
        if j.debit - j.credit != s.amount {
            return fail(IssueCode::InvalidInput, "The amounts differ.");
        }
        if self
            .state
            .statement_lines
            .iter()
            .any(|x| x.matched_line_id.as_ref() == Some(journal_line_id) && x.id != s.id)
        {
            return fail(
                IssueCode::Duplicate,
                "That journal line is already matched.",
            );
        }
        let s = &mut self.state.statement_lines[si];
        s.matched_line_id = Some(journal_line_id.clone());
        s.match_kind = Some(kind);
        let s = s.clone();
        let kind = match kind {
            MatchKind::Auto => "auto",
            MatchKind::Manual => "manual",
        };
        self.audit(
            actor,
            "statement.match",
            s.id.as_str(),
            &format!("{kind} {journal_line_id}"),
        );
        Ok(s)
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
        require(actor, Permission::Post)?;
        let Some(si) = self.statement_line_index(statement_line_id) else {
            return fail(IssueCode::NotFound, "Statement line not found.");
        };
        let s = &mut self.state.statement_lines[si];
        s.matched_line_id = None;
        s.match_kind = None;
        let s = s.clone();
        self.audit(actor, "statement.unmatch", s.id.as_str(), "");
        Ok(s)
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
        let Some(si) = self.statement_line_index(statement_line_id) else {
            return fail(IssueCode::NotFound, "Statement line not found.");
        };
        let s = &self.state.statement_lines[si];
        if s.matched_line_id.is_some() {
            return fail(IssueCode::InvalidState, "This line is already matched.");
        }
        let cash_account = self.bank_of(s)?.account_id.clone();
        let counter_id = counter
            .account_id
            .clone()
            .unwrap_or_else(|| self.state.settings.system_accounts.suspense.clone());
        #[expect(
            clippy::cast_precision_loss,
            reason = "a safe integer, as the TS number"
        )]
        let amt = s.amount.unsigned_abs() as f64;
        let (inflow, outflow) = (s.amount > 0, s.amount < 0);
        let side = |yes: bool| Some(if yes { amt } else { 0.0 });
        let line = |account_id: AccountId, debit: bool, credit: bool| LineInput {
            account_id,
            debit: side(debit),
            credit: side(credit),
            currency: None,
            fx_rate: None,
            fund_id: counter.fund_id.clone(),
            grant_id: Nullish::Undefined,
            dimension_ids: None,
            flow_node_id: Nullish::Undefined,
            memo: Nullish::Undefined,
        };
        let cash_side = line(cash_account.clone(), inflow, outflow);
        let other = LineInput {
            grant_id: Nullish::from(counter.grant_id.value().cloned()),
            ..line(counter_id, outflow, inflow)
        };
        let memo = counter
            .memo
            .as_deref()
            .map(js_trim)
            .filter(|m| !m.is_empty())
            .unwrap_or(&s.description)
            .to_owned();
        let input = EntryInput {
            date: s.date.as_str().to_owned(),
            memo,
            lines: vec![cash_side, other],
            source_type: Some(SourceType::Statement),
            source_id: Nullish::Value(s.id.as_str().to_owned()),
            override_reason: Nullish::Undefined,
        };
        let statement_line_id = s.id.clone();
        let entry = self.post(actor, &input, EntryOptions::default())?;
        let Some(cash_line) = self
            .state
            .lines_of(&entry.id)
            .into_iter()
            .find(|l| l.account_id == cash_account)
            .map(|l| l.id.clone())
        else {
            return fail(IssueCode::NotFound, "Journal line not found.");
        };
        self.match_statement_line(actor, &statement_line_id, &cash_line, MatchKind::Manual)?;
        Ok(entry)
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
