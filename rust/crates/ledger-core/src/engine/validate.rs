//! `validateEntry`: every reason an entry could not post, without changing anything
//! (`engine.ts:291-468`).

use serde::Deserialize;

use super::{Error, has_text, js_trim};
use crate::errors::{Issue, IssueCode};
use crate::money::{AMOUNT, Minor, assert_minor, convert, format_major};
use crate::types::{
    AccountType, EntryId, EntryInput, Fund, FundId, FundRestriction, GrantId, IsoDate, LedgerState,
    LineInput, Nullish, PeriodStatus,
};

/// The options of `validateEntry` and `post` (`engine.ts:303`, `engine.ts:527`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct EntryOptions {
    /// A system posting (a reversal or the year-end close) only moves what is already booked:
    /// the spend rules (grant dates, restricted funds, over-award) and the inactive-account rule
    /// do not apply to it (`engine.ts:380-383`, `engine.ts:362`).
    #[serde(default)]
    pub system: bool,
}

/// A grant id TS treats as present: `if (l.grant_id)` is false for `undefined`, `null` and `''`.
pub(super) fn grant_of(l: &LineInput) -> Option<&GrantId> {
    l.grant_id.value().filter(|g| !g.as_str().is_empty())
}

/// The override reason that lets overridable issues through: `override?.trim()` truthy.
pub(super) fn override_of(reason: &Nullish<String>) -> Option<&str> {
    reason.value().map(|r| js_trim(r)).filter(|r| !r.is_empty())
}

/// `blocking(issues, override)` (`engine.ts:466-468`): the issues an override reason does not
/// clear.
pub(super) fn blocking(issues: Vec<Issue>, override_reason: &Nullish<String>) -> Vec<Issue> {
    let overridden = override_of(override_reason).is_some();
    issues
        .into_iter()
        .filter(|i| !(i.overridable == Some(true) && overridden))
        .collect()
}

/// What one line contributed: its issues and, once its amount converted, its base amount.
struct LineCheck<'a> {
    issues: Vec<(IssueCode, String, bool)>,
    base: Option<(&'a FundId, Minor)>,
}

impl LineCheck<'_> {
    fn new() -> Self {
        Self {
            issues: Vec::new(),
            base: None,
        }
    }

    fn at(&mut self, code: IssueCode, message: String) {
        self.issues.push((code, message, false));
    }

    fn overridable(&mut self, code: IssueCode, message: String) {
        self.issues.push((code, message, true));
    }

    /// `return at(...)`: the issue that ends the line's checks.
    fn stop(mut self, code: IssueCode, message: &str) -> Self {
        self.at(code, message.to_owned());
        self
    }
}

impl LedgerState {
    /// `validateEntry(input, ignoreEntryId, opts)` (`engine.ts:295-445`): every reason `input`
    /// could not post today, without changing anything. Overridable issues are reported even
    /// when an override reason is given, flagged `overridable`, so callers decide.
    ///
    /// In order: the date shape and memo (`INVALID_INPUT`); the period (`NO_PERIOD`,
    /// `PERIOD_CLOSED`, `PERIOD_LOCKED`); at least two lines (`TOO_FEW_LINES`); then each line,
    /// tagged with its index and `Line n:` (amounts, account, fund, dimensions, rate, grant);
    /// then, only when no line has an issue, the balance in base currency (`UNBALANCED`) or, if
    /// that holds, per fund (`FUND_UNBALANCED`), and for non-system entries the award of each
    /// grant the entry spends from (`GRANT_OVER_BUDGET`, overridable), counting posted spend
    /// except `ignore`.
    ///
    /// # Errors
    ///
    /// [`Error::Money`] when a converted amount is not a safe integer, as `convert` throws in
    /// TS (`money.ts:49`).
    pub fn validate_entry(
        &self,
        input: &EntryInput,
        ignore: Option<&EntryId>,
        opts: EntryOptions,
    ) -> Result<Vec<Issue>, Error> {
        let mut issues = Vec::new();
        let base = self.settings.base_currency.as_str();

        // `/^\d{4}-\d{2}-\d{2}$/` (engine.ts:308-309), the shape IsoDate accepts.
        if IsoDate::parse(&input.date).is_err() {
            issues.push(Issue::new(IssueCode::InvalidInput, "Enter the entry date."));
        }
        if !has_text(&input.memo) {
            issues.push(Issue::new(IssueCode::InvalidInput, "Describe the entry."));
        }
        // engine.ts:313-328
        match self.period_for(&input.date) {
            None => issues.push(Issue::new(
                IssueCode::NoPeriod,
                format!("No fiscal period covers {}.", input.date),
            )),
            Some(p) if p.status == PeriodStatus::Closed => issues.push(Issue::new(
                IssueCode::PeriodClosed,
                format!(
                    "{} is closed. Reopen it or date the entry in an open period.",
                    p.name
                ),
            )),
            Some(p) if p.status == PeriodStatus::Locked => issues.push(Issue::new(
                IssueCode::PeriodLocked,
                format!("{} is locked and can no longer take entries.", p.name),
            )),
            Some(_) => {}
        }
        if input.lines.len() < 2 {
            issues.push(Issue::new(
                IssueCode::TooFewLines,
                "An entry needs at least two lines.",
            ));
        }

        // engine.ts:336-402: totals in base currency, overall and per fund in first-use order.
        let mut by_fund: Vec<(&FundId, Minor)> = Vec::new();
        let mut total: Minor = 0;
        let mut line_issues = false;
        for (i, l) in input.lines.iter().enumerate() {
            let check = self.check_line(l, &input.date, opts)?;
            for (code, message, overridable) in check.issues {
                line_issues = true;
                issues.push(
                    Issue::new(code, format!("Line {}: {message}", i + 1)).on_line(i, overridable),
                );
            }
            if let Some((fund_id, amount)) = check.base {
                total += amount;
                match by_fund.iter_mut().find(|(f, _)| *f == fund_id) {
                    Some((_, t)) => *t += amount,
                    None => by_fund.push((fund_id, amount)),
                }
            }
        }

        // engine.ts:404-417
        if !line_issues && input.lines.len() >= 2 {
            if total != 0 {
                issues.push(Issue::new(
                    IssueCode::Unbalanced,
                    format!(
                        "Debits and credits differ by {base} {}.",
                        format_major(total.abs())
                    ),
                ));
            } else {
                for (fund_id, t) in &by_fund {
                    if *t != 0 {
                        let name = self.fund(fund_id).map_or("A fund", |f| f.name.as_str());
                        issues.push(Issue::new(
                            IssueCode::FundUnbalanced,
                            format!(
                                "{name} does not balance within this entry. Move money between \
                                 funds with a transfer that debits and credits each fund."
                            ),
                        ));
                    }
                }
            }
        }

        if !line_issues && !opts.system {
            self.check_awards(input, ignore, &mut issues)?;
        }
        Ok(issues)
    }

    /// The checks of one line (`engine.ts:342-402`), stopping where TS `return`s.
    fn check_line<'a>(
        &'a self,
        l: &'a LineInput,
        date: &str,
        opts: EntryOptions,
    ) -> Result<LineCheck<'a>, Error> {
        let check = LineCheck::new();
        let debit = l.debit.unwrap_or(0.0);
        let credit = l.credit.unwrap_or(0.0);
        // Number.isSafeInteger on both amounts.
        let (Ok(debit_minor), Ok(credit_minor)) =
            (assert_minor(debit, AMOUNT), assert_minor(credit, AMOUNT))
        else {
            return Ok(check.stop(IssueCode::NotMinorUnits, "amounts must be whole cents."));
        };
        if debit_minor < 0 || credit_minor < 0 {
            return Ok(check.stop(IssueCode::NegativeAmount, "amounts cannot be negative."));
        }
        if debit_minor > 0 && credit_minor > 0 {
            return Ok(check.stop(
                IssueCode::LineBothSides,
                "use either a debit or a credit, not both.",
            ));
        }
        if debit_minor == 0 && credit_minor == 0 {
            return Ok(check.stop(IssueCode::LineZero, "enter a debit or a credit."));
        }

        let mut check = check;
        let Some(account) = self.account(&l.account_id) else {
            return Ok(check.stop(IssueCode::UnknownAccount, "choose an account."));
        };
        if account.is_group {
            check.at(
                IssueCode::GroupAccount,
                format!(
                    "{} {} is a header account; post to one of its sub-accounts.",
                    account.code, account.name
                ),
            );
        }
        if !account.is_active && !opts.system {
            check.at(
                IssueCode::InactiveAccount,
                format!("{} {} is inactive.", account.code, account.name),
            );
        }

        let Some(fund) = self.fund(&l.fund_id) else {
            return Ok(check.stop(IssueCode::UnknownFund, "choose a fund."));
        };
        for d in l.dimension_ids.iter().flatten() {
            if !self.dimensions.iter().any(|x| &x.id == d) {
                check.at(
                    IssueCode::UnknownDimension,
                    "a tag on this line no longer exists.".to_owned(),
                );
            }
        }

        // engine.ts:372-378
        let base = self.settings.base_currency.as_str();
        let currency = l.currency.as_deref().unwrap_or(base);
        let rate = if currency == base {
            Some(1.0)
        } else {
            l.fx_rate
        };
        let Some(rate) = rate.filter(|r| *r > 0.0 && r.is_finite()) else {
            let message = format!("enter the {currency} to {base} rate.");
            return Ok(check.stop(IssueCode::FxRateRequired, &message));
        };
        check.base = Some((&fund.id, convert(debit_minor - credit_minor, rate)?));

        // Restricted-fund and grant rules apply to spend: expense debits (engine.ts:380-401).
        let is_spend =
            account.account_type == AccountType::Expense && debit_minor > 0 && !opts.system;
        self.check_grant(&mut check, l, fund, is_spend, date);
        Ok(check)
    }

    /// The grant and restricted-fund rules of one line (`engine.ts:380-401`): the grant must
    /// exist and belong to the line's fund; spend must fall within the grant's dates
    /// (overridable); spend from a restricted fund that has grants must name one.
    fn check_grant(
        &self,
        check: &mut LineCheck<'_>,
        l: &LineInput,
        fund: &Fund,
        is_spend: bool,
        date: &str,
    ) {
        if let Some(grant_id) = grant_of(l) {
            let Some(grant) = self.grant(grant_id) else {
                check.at(
                    IssueCode::UnknownGrant,
                    "the grant no longer exists.".to_owned(),
                );
                return;
            };
            if grant.fund_id != fund.id {
                let owner = self
                    .fund(&grant.fund_id)
                    .map_or("another fund", |f| f.name.as_str());
                check.at(
                    IssueCode::GrantFundMismatch,
                    format!("{} belongs to {owner}.", grant.name),
                );
            }
            if is_spend {
                if date < grant.start_date.as_str() {
                    check.overridable(
                        IssueCode::GrantNotStarted,
                        format!("{} starts on {}.", grant.name, grant.start_date),
                    );
                }
                if date > grant.end_date.as_str() {
                    check.overridable(
                        IssueCode::GrantEnded,
                        format!("{} ended on {}.", grant.name, grant.end_date),
                    );
                }
            }
        } else if is_spend
            && fund.restriction != FundRestriction::Unrestricted
            && self.grants.iter().any(|g| g.fund_id == fund.id)
        {
            check.at(
                IssueCode::RestrictedNeedsGrant,
                format!(
                    "spend from {} must name the grant it is charged to.",
                    fund.name
                ),
            );
        }
    }

    /// Over-award (`engine.ts:419-442`): posted spend on each grant plus this entry's net
    /// expense on it, in base currency, against the award when the grant is in base currency.
    fn check_awards(
        &self,
        input: &EntryInput,
        ignore: Option<&EntryId>,
        issues: &mut Vec<Issue>,
    ) -> Result<(), Error> {
        let base = self.settings.base_currency.as_str();
        let mut spend_by_grant: Vec<(&GrantId, Minor)> = Vec::new();
        for l in &input.lines {
            let Some(grant_id) = grant_of(l) else {
                continue;
            };
            let is_expense = self
                .account(&l.account_id)
                .is_some_and(|a| a.account_type == AccountType::Expense);
            if !is_expense {
                continue;
            }
            // Every line passed its checks, so amounts are safe and the rate is valid.
            let rate = if l.currency.as_deref().unwrap_or(base) == base {
                1.0
            } else {
                l.fx_rate.unwrap_or(f64::NAN)
            };
            let net = assert_minor(l.debit.unwrap_or(0.0) - l.credit.unwrap_or(0.0), AMOUNT)?;
            let amount = convert(net, rate)?;
            match spend_by_grant.iter_mut().find(|(g, _)| *g == grant_id) {
                Some((_, t)) => *t += amount,
                None => spend_by_grant.push((grant_id, amount)),
            }
        }
        for (grant_id, amount) in spend_by_grant {
            if amount <= 0 {
                continue;
            }
            let Some(grant) = self.grant(grant_id) else {
                continue;
            };
            let spent = self.grant_spend(grant_id, ignore);
            if grant.currency == base && spent + amount > grant.award {
                issues.push(
                    Issue::new(
                        IssueCode::GrantOverBudget,
                        format!(
                            "This takes {} over its award by {base} {}.",
                            grant.name,
                            format_major(spent + amount - grant.award)
                        ),
                    )
                    .overridable(true),
                );
            }
        }
        Ok(())
    }
}
