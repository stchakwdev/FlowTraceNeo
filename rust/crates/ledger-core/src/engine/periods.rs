//! Periods: close, reopen, lock and the year-end close (`engine.ts:664-774`).

use std::collections::HashSet;

use super::queries::js_number;
use super::{EntryOptions, Error, Ledger, Permission, fail, has_text, require};
use crate::errors::IssueCode;
use crate::money::Minor;
use crate::types::{
    AccountId, AccountType, Actor, EntryInput, EntryStatus, FiscalPeriod, FundId, FundRestriction,
    GrantId, JournalEntry, LedgerState, LineInput, Nullish, PeriodId, PeriodStatus, SourceType,
};

/// The TS literal of a period status, for messages.
fn status_str(s: PeriodStatus) -> &'static str {
    match s {
        PeriodStatus::Open => "open",
        PeriodStatus::Closed => "closed",
        PeriodStatus::Locked => "locked",
    }
}

impl LedgerState {
    /// The private `period(id)` (`engine.ts:668-670`), as an index.
    fn period_index(&self, id: &PeriodId) -> Result<usize, Error> {
        match self.periods.iter().position(|p| &p.id == id) {
            Some(i) => Ok(i),
            None => fail(IssueCode::NotFound, "Period not found."),
        }
    }
}

impl Ledger<'_> {
    /// `closePeriod` (`engine.ts:672-687`): closes an open period. Needs `can_close`. Periods
    /// of a fiscal year close in order (no earlier open period of the same year), and a period
    /// with drafts dated in it cannot close. One audit id (`period.close`, detail the name).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` when not open, out of order or with drafts.
    pub fn close_period(&mut self, actor: &Actor, id: &PeriodId) -> Result<FiscalPeriod, Error> {
        require(actor, Permission::Close)?;
        let index = self.state.period_index(id)?;
        let p = &self.state.periods[index];
        if p.status != PeriodStatus::Open {
            return fail(
                IssueCode::InvalidState,
                format!("{} is already {}.", p.name, status_str(p.status)),
            );
        }
        let earlier_open = self.state.periods.iter().find(|x| {
            x.start_date < p.start_date
                && x.status == PeriodStatus::Open
                && x.fiscal_year == p.fiscal_year
        });
        if let Some(earlier) = earlier_open {
            return fail(
                IssueCode::InvalidState,
                format!("Close {} first; periods close in order.", earlier.name),
            );
        }
        let drafts = self
            .state
            .entries
            .iter()
            .filter(|e| e.period_id == p.id && e.status == EntryStatus::Draft)
            .count();
        if drafts > 0 {
            let (noun, pronoun) = if drafts == 1 {
                ("entry", "it")
            } else {
                ("entries", "them")
            };
            return fail(
                IssueCode::InvalidState,
                format!(
                    "{} has {drafts} draft {noun}. Post or delete {pronoun} before closing.",
                    p.name
                ),
            );
        }
        Ok(self.set_status(actor, index, PeriodStatus::Closed, "period.close", None))
    }

    /// `reopenPeriod` (`engine.ts:689-703`): reopens a closed period with a reason. Needs
    /// `can_close`. Locked periods never reopen, and periods reopen in reverse order (no later
    /// period, of any year, may be closed or locked). One audit id (`period.reopen`, detail the
    /// reason as given).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `PERIOD_LOCKED`; `INVALID_STATE` when already open or out of
    /// order; `INVALID_INPUT` for a blank reason.
    pub fn reopen_period(
        &mut self,
        actor: &Actor,
        id: &PeriodId,
        reason: &str,
    ) -> Result<FiscalPeriod, Error> {
        require(actor, Permission::Close)?;
        let index = self.state.period_index(id)?;
        let p = &self.state.periods[index];
        if p.status == PeriodStatus::Locked {
            return fail(
                IssueCode::PeriodLocked,
                format!("{} is locked and cannot be reopened.", p.name),
            );
        }
        if p.status == PeriodStatus::Open {
            return fail(
                IssueCode::InvalidState,
                format!("{} is already open.", p.name),
            );
        }
        if !has_text(reason) {
            return fail(IssueCode::InvalidInput, "Give a reason for reopening.");
        }
        let later_closed = self
            .state
            .periods
            .iter()
            .find(|x| x.start_date > p.start_date && x.status != PeriodStatus::Open);
        if let Some(later) = later_closed {
            return fail(
                IssueCode::InvalidState,
                format!(
                    "Reopen {} first; periods reopen in reverse order.",
                    later.name
                ),
            );
        }
        Ok(self.set_status(
            actor,
            index,
            PeriodStatus::Open,
            "period.reopen",
            Some(reason),
        ))
    }

    /// `lockPeriod` (`engine.ts:705-712`): locks a closed period for good. Needs `can_close`.
    /// One audit id (`period.lock`, detail the name).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` unless the period is closed.
    pub fn lock_period(&mut self, actor: &Actor, id: &PeriodId) -> Result<FiscalPeriod, Error> {
        require(actor, Permission::Close)?;
        let index = self.state.period_index(id)?;
        let p = &self.state.periods[index];
        if p.status != PeriodStatus::Closed {
            return fail(
                IssueCode::InvalidState,
                format!("Close {} before locking it.", p.name),
            );
        }
        Ok(self.set_status(actor, index, PeriodStatus::Locked, "period.lock", None))
    }

    /// Sets a period's status and audits it, with the name as detail unless one is given.
    fn set_status(
        &mut self,
        actor: &Actor,
        index: usize,
        status: PeriodStatus,
        action: &str,
        detail: Option<&str>,
    ) -> FiscalPeriod {
        let p = &mut self.state.periods[index];
        p.status = status;
        let period = p.clone();
        let detail = detail.unwrap_or(&period.name);
        self.audit(actor, action, period.id.as_str(), detail);
        period
    }

    /// `closeYear` (`engine.ts:714-774`): the year-end close. Zeroes every income and expense
    /// balance of the year's posted (and reversed) entries into net assets, fund by fund:
    /// one line per account, fund and grant with a non-zero balance (first-seen order), then
    /// one net-assets line per fund with a non-zero net, to restricted net assets unless the
    /// fund is unrestricted. The entry is dated the last day of the year's last period, posted
    /// as a system entry (source `year_end_close`, override reason `Year-end close`) while that
    /// period is open, and only when it has at least two lines; then every open period of the
    /// year closes. Needs `can_close`. Draws the ids of [`Ledger::post`] when an entry posts,
    /// then one audit id (`year.close`, subject the year, detail the entry number or
    /// `nothing to close`).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND` when the year has no periods; `INVALID_STATE` when already
    /// closed; `PERIOD_CLOSED` when the last period is not open; the validation issues of the
    /// closing entry.
    pub fn close_year(&mut self, actor: &Actor, year: i32) -> Result<Option<JournalEntry>, Error> {
        require(actor, Permission::Close)?;
        let periods: Vec<&FiscalPeriod> = self
            .state
            .periods
            .iter()
            .filter(|p| p.fiscal_year == year)
            .collect();
        let Some(last) = periods.last() else {
            return fail(
                IssueCode::NotFound,
                format!("There are no periods for {year}."),
            );
        };
        let closed_before = self.state.entries.iter().any(|e| {
            e.source_type == SourceType::YearEndClose
                && e.status == EntryStatus::Posted
                && e.date == last.end_date
        });
        if closed_before {
            return fail(
                IssueCode::InvalidState,
                format!("{year} has already been closed."),
            );
        }
        let lines = self.state.closing_lines(&periods);
        // Earlier periods may already be closed; the closing entry lands in the last period,
        // which must still be open (engine.ts:758-760).
        if last.status != PeriodStatus::Open {
            return fail(
                IssueCode::PeriodClosed,
                format!("{} must be open to post the year-end close.", last.name),
            );
        }
        let input = EntryInput {
            date: last.end_date.as_str().to_owned(),
            memo: format!("Year-end close {year}: income and expense to net assets"),
            source_type: Some(SourceType::YearEndClose),
            source_id: Nullish::Undefined,
            override_reason: Nullish::Value("Year-end close".to_owned()),
            lines,
        };
        let entry = if input.lines.len() >= 2 {
            Some(self.post(actor, &input, EntryOptions { system: true })?)
        } else {
            None
        };
        for p in &mut self.state.periods {
            if p.fiscal_year == year && p.status == PeriodStatus::Open {
                p.status = PeriodStatus::Closed;
            }
        }
        let detail = entry
            .as_ref()
            .map_or("nothing to close", |e| e.number.as_str());
        self.audit(actor, "year.close", &year.to_string(), detail);
        Ok(entry)
    }
}

impl LedgerState {
    /// The lines of the year-end close (`engine.ts:727-757`).
    fn closing_lines(&self, periods: &[&FiscalPeriod]) -> Vec<LineInput> {
        let year_ids: HashSet<&PeriodId> = periods.iter().map(|p| &p.id).collect();
        let posted: HashSet<_> = self
            .entries
            .iter()
            .filter(|e| e.status != EntryStatus::Draft && year_ids.contains(&e.period_id))
            .map(|e| &e.id)
            .collect();
        // Balances by account, fund and grant (`''` for none), in first-seen order like a JS Map.
        let mut balances: Vec<((&AccountId, &FundId, &str), Minor)> = Vec::new();
        for l in &self.lines {
            if !posted.contains(&l.entry_id) {
                continue;
            }
            let income_or_expense = self.account(&l.account_id).is_some_and(|a| {
                matches!(a.account_type, AccountType::Income | AccountType::Expense)
            });
            if !income_or_expense {
                continue;
            }
            let key = (
                &l.account_id,
                &l.fund_id,
                l.grant_id.as_ref().map_or("", GrantId::as_str),
            );
            match balances.iter_mut().find(|(k, _)| *k == key) {
                Some((_, b)) => *b += l.base_amount,
                None => balances.push((key, l.base_amount)),
            }
        }
        let sys = &self.settings.system_accounts;
        let mut lines = Vec::new();
        let mut net_by_fund: Vec<(&FundId, Minor)> = Vec::new();
        for ((account_id, fund_id, grant), bal) in balances {
            if bal == 0 {
                continue;
            }
            lines.push(LineInput {
                account_id: account_id.clone(),
                debit: Some(js_number(if bal < 0 { -bal } else { 0 })),
                credit: Some(js_number(if bal > 0 { bal } else { 0 })),
                currency: None,
                fx_rate: None,
                fund_id: fund_id.clone(),
                grant_id: if grant.is_empty() {
                    Nullish::Null
                } else {
                    Nullish::Value(GrantId::from(grant))
                },
                dimension_ids: None,
                flow_node_id: Nullish::Undefined,
                memo: Nullish::Undefined,
            });
            match net_by_fund.iter_mut().find(|(f, _)| *f == fund_id) {
                Some((_, n)) => *n += bal,
                None => net_by_fund.push((fund_id, bal)),
            }
        }
        for (fund_id, net) in net_by_fund {
            if net == 0 {
                continue;
            }
            let restricted = self
                .fund(fund_id)
                .is_none_or(|f| f.restriction != FundRestriction::Unrestricted);
            let account_id = if restricted {
                &sys.net_assets_restricted
            } else {
                &sys.net_assets_unrestricted
            };
            lines.push(LineInput {
                account_id: account_id.clone(),
                debit: Some(js_number(if net > 0 { net } else { 0 })),
                credit: Some(js_number(if net < 0 { -net } else { 0 })),
                currency: None,
                fx_rate: None,
                fund_id: fund_id.clone(),
                grant_id: Nullish::Undefined,
                dimension_ids: None,
                flow_node_id: Nullish::Undefined,
                memo: Nullish::Undefined,
            });
        }
        lines
    }
}
