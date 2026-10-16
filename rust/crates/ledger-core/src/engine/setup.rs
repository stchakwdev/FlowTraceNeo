//! Setup writes: the chart of accounts, funds, grants, dimensions and fiscal years
//! (`engine.ts:165-289`).

use serde::Deserialize;

use super::{
    Error, Ledger, Permission, fail, has_text, js_trim, json_string, normal_balance_of, require,
};
use crate::errors::IssueCode;
use crate::types::{
    Account, AccountId, AccountType, Actor, CashFlowClass, DimensionId, DimensionKind,
    DimensionValue, ExpenseFunction, FiscalPeriod, FlowNodeId, Fund, FundId, FundRestriction,
    Grant, GrantId, IsoDate, Nullish, PeriodId, PeriodStatus, SystemAccounts,
};

/// The input of `addAccount` (`engine.ts:169-175`): an [`Account`] without `normal_balance`,
/// which follows the type, and with `id` and `is_active` optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewAccount {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<AccountId>,
    /// Account code, unique; stored trimmed.
    pub code: String,
    /// Display name; stored trimmed.
    pub name: String,
    /// `type` in TS.
    #[serde(rename = "type")]
    pub account_type: AccountType,
    /// Header account above this one; it must be a header of the same type.
    #[serde(default)]
    pub parent_id: Option<AccountId>,
    /// Header accounts group others and cannot be posted to.
    pub is_group: bool,
    /// Defaults to active.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// Bank, mobile money or petty cash.
    pub is_cash: bool,
    /// Functional classification of an expense account.
    #[serde(default)]
    pub expense_function: Option<ExpenseFunction>,
    /// Cash-flow classification.
    #[serde(default)]
    pub cash_flow_class: Option<CashFlowClass>,
}

/// The patch of `updateAccount` (`engine.ts:203-207`). Only these fields may change: type, code
/// and normal balance would re-state every posted line on the account (`engine.ts:212-213`), so
/// other fields in a patch are ignored, as TS ignores them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct AccountPatch {
    /// New name, stored trimmed; it cannot be blank.
    #[serde(default)]
    pub name: Option<String>,
    /// Activate or deactivate.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// New function; `null` is refused, as TS refuses it (`engine.ts:219-220`).
    #[serde(default)]
    pub expense_function: Nullish<ExpenseFunction>,
}

/// The input of `addFund` (`engine.ts:230`): a [`Fund`] with `id` optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewFund {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<FundId>,
    /// Display name.
    pub name: String,
    /// Donor restriction.
    pub restriction: FundRestriction,
}

/// The input of `addGrant` (`engine.ts:239`): a [`Grant`] with `id` optional and the award as the
/// raw JS number the caller sent, checked by `assertMinor` (`engine.ts:243`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewGrant {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<GrantId>,
    /// Display name.
    pub name: String,
    /// The donor flow node, so the flow map and the ledger meet.
    pub donor_node_id: Option<FlowNodeId>,
    /// Donor display name.
    pub donor_name: String,
    /// The restricted fund the grant sits in; it must exist.
    pub fund_id: FundId,
    /// Award in minor units of `currency`; a positive whole number.
    pub award: f64,
    /// Award currency.
    pub currency: String,
    /// First day spend may be charged.
    pub start_date: IsoDate,
    /// Last day spend may be charged; not before `start_date`.
    pub end_date: IsoDate,
    /// Free-text terms.
    pub terms: String,
}

/// The input of `addDimension` (`engine.ts:253-256`): a [`DimensionValue`] with `id` optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewDimension {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<DimensionId>,
    /// Which dimension.
    pub kind: DimensionKind,
    /// Display name.
    pub name: String,
    /// The flow node this value stands for, if any.
    #[serde(default)]
    pub node_id: Option<FlowNodeId>,
}

/// Month names of period names (`engine.ts:74-87`).
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days in `month` (1 to 12) of `year`, Gregorian, as `lastDayOfMonth` reads them from
/// `Date.UTC` (`engine.ts:91-93`).
fn days_in_month(year: i32, month: i32) -> i32 {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A truthy id: TS tests `input.id &&` and `if (input.parent_id)`, so an empty string is false.
fn truthy<T: AsRef<str>>(id: Option<&T>) -> Option<&T> {
    id.filter(|id| !id.as_ref().is_empty())
}

/// `Object.values(system_accounts)` (`engine.ts:223`).
fn system_account_ids(s: &SystemAccounts) -> [&AccountId; 6] {
    [
        &s.accounts_payable,
        &s.claims_payable,
        &s.suspense,
        &s.net_assets_unrestricted,
        &s.net_assets_restricted,
        &s.fx_gain_loss,
    ]
}

/// The TS literal of an expense function, for the `account.update` audit detail.
fn expense_function_str(f: ExpenseFunction) -> &'static str {
    match f {
        ExpenseFunction::Program => "program",
        ExpenseFunction::Management => "management",
        ExpenseFunction::Fundraising => "fundraising",
    }
}

impl Ledger<'_> {
    /// `addAccount` (`engine.ts:169-201`): adds an account to the chart. Needs `can_post`. Code
    /// and name must not be blank (`INVALID_INPUT`); a given id and the code must be unused
    /// (`DUPLICATE`; the code is looked up untrimmed, as TS does); a parent must exist
    /// (`UNKNOWN_ACCOUNT`), be a header account and have the same type (`INVALID_INPUT`). The
    /// normal balance follows the type ([`normal_balance_of`]); the account starts active unless
    /// told otherwise. Draws the account id (when none is given), then one audit id
    /// (`account.create`, detail `code name`).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn add_account(&mut self, actor: &Actor, input: NewAccount) -> Result<Account, Error> {
        require(actor, Permission::Post)?;
        if !has_text(&input.code) || !has_text(&input.name) {
            return fail(
                IssueCode::InvalidInput,
                "An account needs a code and a name.",
            );
        }
        if truthy(input.id.as_ref()).is_some_and(|id| self.state.account(id).is_some()) {
            return fail(
                IssueCode::Duplicate,
                "An account with this id already exists.",
            );
        }
        if self.state.account_by_code(&input.code).is_some() {
            return fail(
                IssueCode::Duplicate,
                format!("Account code {} is already in use.", input.code),
            );
        }
        if let Some(parent_id) = truthy(input.parent_id.as_ref()) {
            let Some(parent) = self.state.account(parent_id) else {
                return fail(
                    IssueCode::UnknownAccount,
                    "The parent account does not exist.",
                );
            };
            if !parent.is_group {
                return fail(
                    IssueCode::InvalidInput,
                    "The parent must be a header account.",
                );
            }
            if parent.account_type != input.account_type {
                return fail(
                    IssueCode::InvalidInput,
                    "A sub-account must have its parent\u{2019}s type.",
                );
            }
        }
        let id = match input.id {
            Some(id) => id,
            None => AccountId::from(self.env.new_id()),
        };
        let account = Account {
            id,
            code: js_trim(&input.code).to_owned(),
            name: js_trim(&input.name).to_owned(),
            account_type: input.account_type,
            normal_balance: normal_balance_of(input.account_type),
            parent_id: input.parent_id,
            is_group: input.is_group,
            is_active: input.is_active.unwrap_or(true),
            is_cash: input.is_cash,
            expense_function: input.expense_function,
            cash_flow_class: input.cash_flow_class,
        };
        self.state.accounts.push(account.clone());
        let detail = format!("{} {}", account.code, account.name);
        self.audit(actor, "account.create", account.id.as_str(), &detail);
        Ok(account)
    }

    /// `updateAccount` (`engine.ts:203-228`): renames, activates or deactivates an account, or
    /// sets an expense account's function. Needs `can_post`. The account must exist
    /// (`NOT_FOUND`); a name must not be blank, only expense accounts take a function and it
    /// must be one of the three (`INVALID_INPUT`); a system account cannot be deactivated
    /// (`INVALID_INPUT`). One audit id (`account.update`, detail the JSON of the applied patch
    /// with keys in the order name, `is_active`, `expense_function`).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn update_account(
        &mut self,
        actor: &Actor,
        id: &AccountId,
        patch: &AccountPatch,
    ) -> Result<Account, Error> {
        require(actor, Permission::Post)?;
        let Some(index) = self.state.accounts.iter().position(|a| &a.id == id) else {
            return fail(IssueCode::NotFound, "Account not found.");
        };
        let a = &self.state.accounts[index];
        if patch.name.as_deref().is_some_and(|name| !has_text(name)) {
            return fail(IssueCode::InvalidInput, "An account needs a name.");
        }
        let name = patch.name.as_deref().map(|name| js_trim(name).to_owned());
        let expense_function = match &patch.expense_function {
            Nullish::Undefined => None,
            f => {
                if a.account_type != AccountType::Expense {
                    return fail(
                        IssueCode::InvalidInput,
                        "Only expense accounts have a function.",
                    );
                }
                let Nullish::Value(f) = f else {
                    return fail(
                        IssueCode::InvalidInput,
                        "Choose programme, management or fundraising.",
                    );
                };
                Some(*f)
            }
        };
        if patch.is_active == Some(false)
            && system_account_ids(&self.state.settings.system_accounts).contains(&&a.id)
        {
            return fail(
                IssueCode::InvalidInput,
                format!(
                    "{} {} is used by the system and cannot be deactivated.",
                    a.code, a.name
                ),
            );
        }
        let mut detail = Vec::new();
        if let Some(name) = &name {
            detail.push(format!("\"name\":{}", json_string(name)));
        }
        if let Some(active) = patch.is_active {
            detail.push(format!("\"is_active\":{active}"));
        }
        if let Some(f) = expense_function {
            detail.push(format!(
                "\"expense_function\":\"{}\"",
                expense_function_str(f)
            ));
        }
        let a = &mut self.state.accounts[index];
        if let Some(name) = name {
            a.name = name;
        }
        if let Some(active) = patch.is_active {
            a.is_active = active;
        }
        if let Some(f) = expense_function {
            a.expense_function = Some(f);
        }
        let account = a.clone();
        self.audit(
            actor,
            "account.update",
            id.as_str(),
            &format!("{{{}}}", detail.join(",")),
        );
        Ok(account)
    }

    /// `addFund` (`engine.ts:230-237`): adds a fund. Needs `can_post`; a given id must be unused
    /// (`DUPLICATE`). Draws the fund id (when none is given), then one audit id (`fund.create`,
    /// detail the name).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn add_fund(&mut self, actor: &Actor, input: NewFund) -> Result<Fund, Error> {
        require(actor, Permission::Post)?;
        if truthy(input.id.as_ref()).is_some_and(|id| self.state.fund(id).is_some()) {
            return fail(IssueCode::Duplicate, "A fund with this id already exists.");
        }
        let id = match input.id {
            Some(id) => id,
            None => FundId::from(self.env.new_id()),
        };
        let fund = Fund {
            id,
            name: input.name,
            restriction: input.restriction,
        };
        self.state.funds.push(fund.clone());
        self.audit(actor, "fund.create", fund.id.as_str(), &fund.name);
        Ok(fund)
    }

    /// `addGrant` (`engine.ts:239-251`): adds a grant to a fund. Needs `can_post`. The fund must
    /// exist (`UNKNOWN_FUND`); a given id must be unused (`DUPLICATE`); the award must be whole
    /// minor units ([`Error::Money`]) and positive, and the grant cannot end before it starts
    /// (`INVALID_INPUT`, dates compared as strings). Fields are stored as given. Draws the grant
    /// id (when none is given), then one audit id (`grant.create`, detail the name).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above, [`Error::Money`] for a fractional award.
    pub fn add_grant(&mut self, actor: &Actor, input: NewGrant) -> Result<Grant, Error> {
        let _ = (actor, input);
        todo!()
    }

    /// `addDimension` (`engine.ts:253-261`): adds a dimension value. Needs `can_post`. No
    /// uniqueness check and no audit event, as in TS; draws the id only when none is given.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] `PERMISSION`.
    pub fn add_dimension(
        &mut self,
        actor: &Actor,
        input: NewDimension,
    ) -> Result<DimensionValue, Error> {
        require(actor, Permission::Post)?;
        let id = match input.id {
            Some(id) => id,
            None => DimensionId::from(self.env.new_id()),
        };
        let d = DimensionValue {
            id,
            kind: input.kind,
            name: input.name,
            node_id: input.node_id,
        };
        self.state.dimensions.push(d.clone());
        Ok(d)
    }

    /// `openFiscalYear` (`engine.ts:263-269`): creates the periods of a fiscal year, as a
    /// command. Needs `can_close`; the year must be 2000 to 2100 (`INVALID_INPUT`). One audit id
    /// (`year.open`, subject the year).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn open_fiscal_year(&mut self, actor: &Actor, year: i32) -> Result<(), Error> {
        require(actor, Permission::Close)?;
        if !(2000..=2100).contains(&year) {
            return fail(
                IssueCode::InvalidInput,
                "Choose a year between 2000 and 2100.",
            );
        }
        self.ensure_fiscal_year(year)?;
        self.audit(actor, "year.open", &year.to_string(), "");
        Ok(())
    }

    /// `ensureFiscalYear` (`engine.ts:271-289`): adds the twelve monthly periods of fiscal year
    /// `year` that are missing (by start date), open, named `Jan 2026`, with ids
    /// `period-2026-01`; a year starting in month m > 1 begins in m of the previous calendar
    /// year. Then sorts every period by start date. No permission check, no ids, no audit.
    ///
    /// # Errors
    ///
    /// `INVALID_INPUT` when a period date of the year cannot be written as `YYYY-MM-DD` (a
    /// calendar year outside 1000 to 9999), which TS would store as a malformed date; nothing
    /// is added then.
    pub fn ensure_fiscal_year(&mut self, year: i32) -> Result<(), Error> {
        let start_month = i32::from(self.state.settings.fiscal_year_start_month);
        let mut missing = Vec::new();
        for i in 0..12 {
            let m0 = (start_month - 1 + i).rem_euclid(12);
            let y = year + (start_month - 1 + i).div_euclid(12) - i32::from(start_month > 1);
            let start = format!("{y}-{:02}-01", m0 + 1);
            let end = format!("{y}-{:02}-{:02}", m0 + 1, days_in_month(y, m0 + 1));
            if self
                .state
                .periods
                .iter()
                .any(|p| p.start_date.as_str() == start)
            {
                continue;
            }
            let (Ok(start_date), Ok(end_date)) = (IsoDate::parse(&start), IsoDate::parse(&end))
            else {
                return fail(
                    IssueCode::InvalidInput,
                    format!("Periods of {year} cannot be dated YYYY-MM-DD."),
                );
            };
            let month = usize::try_from(m0).unwrap_or_default();
            missing.push(FiscalPeriod {
                id: PeriodId::from(format!("period-{y}-{:02}", m0 + 1)),
                name: format!("{} {y}", MONTHS[month]),
                fiscal_year: year,
                start_date,
                end_date,
                status: PeriodStatus::Open,
            });
        }
        self.state.periods.extend(missing);
        self.state
            .periods
            .sort_by(|a, b| a.start_date.cmp(&b.start_date));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::days_in_month;

    #[test]
    fn days_in_month_follows_the_gregorian_calendar() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }
}
