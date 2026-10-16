//! What bills and expense claims share: coded lines, their accrual posting and payment
//! (`engine.ts:1002-1140`).

use serde::Deserialize;

use super::queries::js_number;
use super::{EntryOptions, Error, Ledger, fail};
use crate::errors::IssueCode;
use crate::money::{Minor, assert_minor, convert, format_major, js_round};
use crate::types::{
    AccountId, Actor, AppliesTo, CodedLine, CodedLineId, DimensionId, EntryInput, EntryStatus,
    FlowNodeId, FundId, GrantId, LedgerState, LineInput, Nullish, Payment, PaymentId, SourceType,
};

/// A coded line of a new bill or claim (`Omit<CodedLine, 'id'>`, `types.ts:164-172`), with the
/// amount as the raw JS number the caller sent. The line id is drawn when the document is
/// stored; an `id` in the input is ignored, as TS overwrites it (`engine.ts:836`,
/// `engine.ts:927`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewCodedLine {
    /// What was bought.
    pub description: String,
    /// The account charged; it must exist.
    pub account_id: AccountId,
    /// Amount in the document's currency: whole minor units, more than zero.
    pub amount: f64,
    /// The fund charged; it must exist.
    pub fund_id: FundId,
    /// Grant charged, if any.
    pub grant_id: Option<GrantId>,
    /// Dimension values tagged.
    pub dimension_ids: Vec<DimensionId>,
}

/// What a payment pays, as `pay` takes it (`engine.ts:1071`): `payBill`'s input, or
/// `reimburseClaim`'s with the amount owed (`engine.ts:995`).
pub(super) struct PayInput<'a> {
    /// Payment date, the entry date.
    pub(super) date: &'a str,
    /// The cash account money left from.
    pub(super) cash_account_id: &'a AccountId,
    /// The raw amount, checked by `assertMinor`.
    pub(super) amount: f64,
    /// Bank or mobile-money reference.
    pub(super) reference: Option<&'a str>,
    /// Today's rate, if given.
    pub(super) fx_rate: Option<f64>,
}

/// The document a payment settles, with what `pay` reads from it (`engine.ts:1066-1070`).
pub(super) struct Settled<'a> {
    /// The bill or claim.
    pub(super) doc: AppliesTo,
    /// Its coded lines.
    pub(super) coded: &'a [CodedLine],
    /// Its currency.
    pub(super) currency: &'a str,
    /// The rate it was booked at.
    pub(super) rate: f64,
    /// The payable account it was booked to.
    pub(super) payable: &'a AccountId,
    /// The memo of the payment entry, before the reference.
    pub(super) memo: String,
}

/// `byFund(coded)` (`engine.ts:1054-1058`): the document's total per fund, funds in the order
/// they first appear, as a JS `Map` keeps them.
fn by_fund(coded: &[CodedLine]) -> Vec<(FundId, Minor)> {
    let mut totals: Vec<(FundId, Minor)> = Vec::new();
    for l in coded {
        add_to(&mut totals, &l.fund_id, l.amount);
    }
    totals
}

/// `m.set(fund, (m.get(fund) ?? 0) + amount)` on an insertion-ordered map.
fn add_to(totals: &mut Vec<(FundId, Minor)>, fund_id: &FundId, amount: Minor) {
    match totals.iter_mut().find(|(f, _)| f == fund_id) {
        Some((_, total)) => *total += amount,
        None => totals.push((fund_id.clone(), amount)),
    }
}

/// A line with only an account and a fund; the rest is filled in by the caller.
fn line(account_id: &AccountId, fund_id: &FundId) -> LineInput {
    LineInput {
        account_id: account_id.clone(),
        debit: None,
        credit: None,
        currency: None,
        fx_rate: None,
        fund_id: fund_id.clone(),
        grant_id: Nullish::Undefined,
        dimension_ids: None,
        flow_node_id: Nullish::Undefined,
        memo: Nullish::Undefined,
    }
}

/// The base-currency line that takes a rounding or exchange difference `diff` (debit when
/// negative, credit when positive), as `accrualLines` and `pay` write it (`engine.ts:1043-1050`,
/// `engine.ts:1112-1119`).
fn difference_line(account_id: &AccountId, fund_id: &FundId, diff: Minor) -> LineInput {
    LineInput {
        debit: Some(js_number(if diff < 0 { -diff } else { 0 })),
        credit: Some(js_number(if diff > 0 { diff } else { 0 })),
        ..line(account_id, fund_id)
    }
}

impl LedgerState {
    /// `checkCodedLines(lines)` (`engine.ts:1006-1016`): at least one line (`TOO_FEW_LINES`),
    /// then line by line: whole minor units ([`Error::Money`], "Line 1 amount ..."), more than
    /// zero (`NEGATIVE_AMOUNT`), an existing account (`UNKNOWN_ACCOUNT`) and fund
    /// (`UNKNOWN_FUND`). Returns the amounts as minor units.
    pub(super) fn check_coded_lines(&self, lines: &[NewCodedLine]) -> Result<Vec<Minor>, Error> {
        if lines.is_empty() {
            return fail(IssueCode::TooFewLines, "Add at least one line.");
        }
        let check = |n: usize, l: &NewCodedLine| -> Result<Minor, Error> {
            let amount = assert_minor(l.amount, &format!("Line {n} amount"))?;
            if amount <= 0 {
                return fail(
                    IssueCode::NegativeAmount,
                    format!("Line {n}: the amount must be more than zero."),
                );
            }
            if self.account(&l.account_id).is_none() {
                return fail(
                    IssueCode::UnknownAccount,
                    format!("Line {n}: choose an account."),
                );
            }
            if self.fund(&l.fund_id).is_none() {
                return fail(IssueCode::UnknownFund, format!("Line {n}: choose a fund."));
            }
            Ok(amount)
        };
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| check(i + 1, l))
            .collect()
    }

    /// `accrualLines(coded, currency, fx_rate, payable, nodeId)` (`engine.ts:1018-1052`): Dr each
    /// coded line (memo its description, tagged with the flow node), then per fund Cr the
    /// payable account with the fund's total. Lines convert one by one and the payable as a
    /// total, so any cent between them goes to the exchange-differences account in base
    /// currency, memo `Currency rounding`.
    ///
    /// The rounding uses the document's rate even for a base-currency document, as TS does;
    /// an invalid rate is the `RangeError` of `convert` ([`Error::Money`]).
    pub(super) fn accrual_lines(
        &self,
        coded: &[CodedLine],
        currency: &str,
        fx_rate: f64,
        payable: &AccountId,
        node_id: Option<&FlowNodeId>,
    ) -> Result<Vec<LineInput>, Error> {
        let node = || Nullish::from(node_id.cloned());
        let mut lines: Vec<LineInput> = coded
            .iter()
            .map(|l| LineInput {
                debit: Some(js_number(l.amount)),
                currency: Some(currency.to_owned()),
                fx_rate: Some(fx_rate),
                grant_id: l.grant_id.clone().into(),
                dimension_ids: Some(l.dimension_ids.clone()),
                flow_node_id: node(),
                memo: Nullish::Value(l.description.clone()),
                ..line(&l.account_id, &l.fund_id)
            })
            .collect();
        for (fund_id, amount) in by_fund(coded) {
            lines.push(LineInput {
                credit: Some(js_number(amount)),
                currency: Some(currency.to_owned()),
                fx_rate: Some(fx_rate),
                flow_node_id: node(),
                ..line(payable, &fund_id)
            });
            let debits: Minor = coded
                .iter()
                .filter(|l| l.fund_id == fund_id)
                .map(|l| convert(l.amount, fx_rate))
                .sum::<Result<Minor, _>>()?;
            let diff = debits - convert(amount, fx_rate)?;
            if diff != 0 {
                lines.push(LineInput {
                    memo: Nullish::Value("Currency rounding".to_owned()),
                    ..difference_line(&self.settings.system_accounts.fx_gain_loss, &fund_id, diff)
                });
            }
        }
        Ok(lines)
    }

    /// The lines of a payment of `amount` on `doc` from `cash_id` at `pay_rate`, split across
    /// the document's funds by what each still owes (`engine.ts:1083-1121`).
    fn payment_lines(
        &self,
        doc: &Settled<'_>,
        amount: Minor,
        pay_rate: f64,
        cash_id: &AccountId,
    ) -> Result<Vec<LineInput>, Error> {
        let paid = self.paid_by_fund(&doc.doc, doc.payable);
        let shares: Vec<(FundId, Minor)> = by_fund(doc.coded)
            .into_iter()
            .map(|(fund_id, total)| {
                let done = paid.iter().find(|(f, _)| f == &fund_id).map_or(0, |p| p.1);
                (fund_id, total - done)
            })
            .filter(|(_, remaining)| *remaining > 0)
            .collect();
        let remaining_total: Minor = shares.iter().map(|(_, r)| r).sum();
        let fx_gain_loss = &self.settings.system_accounts.fx_gain_loss;
        let mut allocated = 0;
        let mut lines = Vec::new();
        for (i, (fund_id, remaining)) in shares.iter().enumerate() {
            let part = if i == shares.len() - 1 {
                amount - allocated
            } else {
                // engine.ts:1101: a JS double product and quotient, rounded half up.
                let share = js_round(
                    js_number(amount) * js_number(*remaining) / js_number(remaining_total),
                );
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the share is a whole number no larger than the amount"
                )]
                let share = share as Minor;
                (*remaining).min(share)
            };
            allocated += part;
            if part == 0 {
                continue;
            }
            lines.push(LineInput {
                debit: Some(js_number(part)),
                currency: Some(doc.currency.to_owned()),
                fx_rate: Some(doc.rate),
                ..line(doc.payable, fund_id)
            });
            lines.push(LineInput {
                credit: Some(js_number(part)),
                currency: Some(doc.currency.to_owned()),
                fx_rate: Some(pay_rate),
                ..line(cash_id, fund_id)
            });
            let diff = convert(part, doc.rate)? - convert(part, pay_rate)?;
            if diff != 0 {
                lines.push(difference_line(fx_gain_loss, fund_id, diff));
            }
        }
        Ok(lines)
    }

    /// What each fund has paid of the document so far: the debits to `payable` on the entries of
    /// its posted payments (`engine.ts:1085-1091`).
    fn paid_by_fund(&self, doc: &AppliesTo, payable: &AccountId) -> Vec<(FundId, Minor)> {
        let mut paid = Vec::new();
        for p in &self.payments {
            if &p.applies_to != doc
                || !self
                    .entry(&p.entry_id)
                    .is_some_and(|e| e.status == EntryStatus::Posted)
            {
                continue;
            }
            for l in self.lines_of(&p.entry_id) {
                if &l.account_id == payable {
                    add_to(&mut paid, &l.fund_id, l.debit);
                }
            }
        }
        paid
    }
}

impl Ledger<'_> {
    /// Stores coded lines, drawing one id each in order (`engine.ts:836`, `engine.ts:927`).
    pub(super) fn coded_lines(
        &mut self,
        lines: Vec<NewCodedLine>,
        amounts: Vec<Minor>,
    ) -> Vec<CodedLine> {
        lines
            .into_iter()
            .zip(amounts)
            .map(|(l, amount)| CodedLine {
                id: CodedLineId::from(self.env.new_id()),
                description: l.description,
                account_id: l.account_id,
                amount,
                fund_id: l.fund_id,
                grant_id: l.grant_id,
                dimension_ids: l.dimension_ids,
            })
            .collect()
    }

    /// `pay(actor, appliesTo, coded, currency, docRate, input, payable, memo)`
    /// (`engine.ts:1060-1140`): Dr payable / Cr cash, split across funds in proportion to what
    /// each fund still owes, so each fund's payable clears with its own cash.
    ///
    /// The cash account must be a cash account (`UNKNOWN_ACCOUNT`); the amount whole minor units
    /// ([`Error::Money`], "Payment ..."), more than zero (`NEGATIVE_AMOUNT`) and at most what is
    /// owed (`INVALID_INPUT`, "Only 12.50 is outstanding."). A foreign document pays at the
    /// given rate or its own; a base one at 1. Each fund but the last gets
    /// `min(remaining, Math.round(amount * remaining / remainingTotal))` (JS rounding,
    /// [`js_round`]), the last the rest; a fund with nothing to pay gets no lines. Per fund the
    /// payable clears at the document's rate, cash leaves at the payment's, and the difference
    /// is a realised exchange gain or loss in base currency. The entry is dated the payment
    /// date, memo `<memo>` or `<memo> · <reference>`, source `payment` for a bill or
    /// `reimbursement` for a claim pointing at the document. Draws the ids of
    /// [`Ledger::post`], then the payment id. No audit event of its own.
    pub(super) fn pay(
        &mut self,
        actor: &Actor,
        doc: &Settled<'_>,
        input: &PayInput<'_>,
    ) -> Result<Payment, Error> {
        let Some(cash) = self
            .state
            .account(input.cash_account_id)
            .filter(|a| a.is_cash)
        else {
            return fail(
                IssueCode::UnknownAccount,
                "Choose the bank, mobile money or cash account the money left.",
            );
        };
        let cash_id = cash.id.clone();
        let amount = assert_minor(input.amount, "Payment")?;
        let owed = self.state.outstanding(&doc.doc)?;
        if amount <= 0 {
            return fail(
                IssueCode::NegativeAmount,
                "The payment must be more than zero.",
            );
        }
        if amount > owed {
            return fail(
                IssueCode::InvalidInput,
                format!("Only {} is outstanding.", format_major(owed)),
            );
        }
        let pay_rate = if doc.currency == self.state.settings.base_currency {
            1.0
        } else {
            input.fx_rate.unwrap_or(doc.rate)
        };
        let lines = self.state.payment_lines(doc, amount, pay_rate, &cash_id)?;
        let (source_type, source_id) = match &doc.doc {
            AppliesTo::Bill(id) => (SourceType::Payment, id.as_str()),
            AppliesTo::ExpenseClaim(id) => (SourceType::Reimbursement, id.as_str()),
        };
        let entry_input = EntryInput {
            date: input.date.to_owned(),
            memo: match input.reference.filter(|r| !r.is_empty()) {
                Some(r) => format!("{} \u{b7} {r}", doc.memo),
                None => doc.memo.clone(),
            },
            source_type: Some(source_type),
            source_id: Nullish::Value(source_id.to_owned()),
            override_reason: Nullish::Undefined,
            lines,
        };
        let prepared = self.check_post(actor, &entry_input, EntryOptions::default())?;
        let date = prepared.date.clone();
        let entry = self.store_posted(actor, &entry_input, prepared);
        let payment = Payment {
            id: PaymentId::from(self.env.new_id()),
            date,
            cash_account_id: cash_id,
            amount,
            currency: doc.currency.to_owned(),
            fx_rate: pay_rate,
            reference: input.reference.map(str::to_owned),
            applies_to: doc.doc.clone(),
            entry_id: entry.id,
            created_by: actor.id.clone(),
        };
        self.state.payments.push(payment.clone());
        Ok(payment)
    }
}
