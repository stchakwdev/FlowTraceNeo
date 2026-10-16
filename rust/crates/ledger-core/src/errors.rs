//! Ledger rule failures, mirroring `flowtrace/src/lib/ledger/errors.ts`.
//!
//! Every rule the ledger enforces has a code, so a UI can point at the field and the SpacetimeDB
//! module can return the same message the demo does (`errors.ts:1-2`). The message text is
//! written where the rule is checked (the engine, C2 onward), as in TS: one code carries many
//! messages (`INVALID_STATE` has seven in the fixtures), so the code enum does not own the text.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The code of a broken rule (`errors.ts:4-32`), written as the TS string literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueCode {
    /// The actor may not do this.
    Permission,
    /// A referenced record does not exist.
    NotFound,
    /// The record is in a state that does not allow this.
    InvalidState,
    /// An entry or document has too few lines.
    TooFewLines,
    /// A line has both a debit and a credit.
    LineBothSides,
    /// A line has neither a debit nor a credit.
    LineZero,
    /// A line amount is negative.
    NegativeAmount,
    /// A line amount is not a whole number of minor units.
    NotMinorUnits,
    /// A line names an account that does not exist.
    UnknownAccount,
    /// A line posts to a header account.
    GroupAccount,
    /// A line posts to an inactive account.
    InactiveAccount,
    /// A line names a fund that does not exist.
    UnknownFund,
    /// A line names a grant that does not exist.
    UnknownGrant,
    /// A line names a dimension value that does not exist.
    UnknownDimension,
    /// A foreign-currency line has no rate.
    FxRateRequired,
    /// No fiscal period covers the date.
    NoPeriod,
    /// The period is closed.
    PeriodClosed,
    /// The period is locked.
    PeriodLocked,
    /// Debits and credits differ in base currency.
    Unbalanced,
    /// A fund does not balance within the entry.
    FundUnbalanced,
    /// Spend from a restricted fund names no grant.
    RestrictedNeedsGrant,
    /// The grant belongs to another fund.
    GrantFundMismatch,
    /// The date is before the grant starts.
    GrantNotStarted,
    /// The date is after the grant ends.
    GrantEnded,
    /// The entry takes the grant over its award.
    GrantOverBudget,
    /// The actor approves their own document.
    SelfApproval,
    /// A record with this id already exists.
    Duplicate,
    /// The input is malformed.
    InvalidInput,
}

impl IssueCode {
    /// Every code, in the order of `errors.ts:4-32`.
    pub const ALL: [IssueCode; 28] = [
        Self::Permission,
        Self::NotFound,
        Self::InvalidState,
        Self::TooFewLines,
        Self::LineBothSides,
        Self::LineZero,
        Self::NegativeAmount,
        Self::NotMinorUnits,
        Self::UnknownAccount,
        Self::GroupAccount,
        Self::InactiveAccount,
        Self::UnknownFund,
        Self::UnknownGrant,
        Self::UnknownDimension,
        Self::FxRateRequired,
        Self::NoPeriod,
        Self::PeriodClosed,
        Self::PeriodLocked,
        Self::Unbalanced,
        Self::FundUnbalanced,
        Self::RestrictedNeedsGrant,
        Self::GrantFundMismatch,
        Self::GrantNotStarted,
        Self::GrantEnded,
        Self::GrantOverBudget,
        Self::SelfApproval,
        Self::Duplicate,
        Self::InvalidInput,
    ];

    /// The TS string literal, for example `"FX_RATE_REQUIRED"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Permission => "PERMISSION",
            Self::NotFound => "NOT_FOUND",
            Self::InvalidState => "INVALID_STATE",
            Self::TooFewLines => "TOO_FEW_LINES",
            Self::LineBothSides => "LINE_BOTH_SIDES",
            Self::LineZero => "LINE_ZERO",
            Self::NegativeAmount => "NEGATIVE_AMOUNT",
            Self::NotMinorUnits => "NOT_MINOR_UNITS",
            Self::UnknownAccount => "UNKNOWN_ACCOUNT",
            Self::GroupAccount => "GROUP_ACCOUNT",
            Self::InactiveAccount => "INACTIVE_ACCOUNT",
            Self::UnknownFund => "UNKNOWN_FUND",
            Self::UnknownGrant => "UNKNOWN_GRANT",
            Self::UnknownDimension => "UNKNOWN_DIMENSION",
            Self::FxRateRequired => "FX_RATE_REQUIRED",
            Self::NoPeriod => "NO_PERIOD",
            Self::PeriodClosed => "PERIOD_CLOSED",
            Self::PeriodLocked => "PERIOD_LOCKED",
            Self::Unbalanced => "UNBALANCED",
            Self::FundUnbalanced => "FUND_UNBALANCED",
            Self::RestrictedNeedsGrant => "RESTRICTED_NEEDS_GRANT",
            Self::GrantFundMismatch => "GRANT_FUND_MISMATCH",
            Self::GrantNotStarted => "GRANT_NOT_STARTED",
            Self::GrantEnded => "GRANT_ENDED",
            Self::GrantOverBudget => "GRANT_OVER_BUDGET",
            Self::SelfApproval => "SELF_APPROVAL",
            Self::Duplicate => "DUPLICATE",
            Self::InvalidInput => "INVALID_INPUT",
        }
    }
}

impl fmt::Display for IssueCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One broken rule (`errors.ts:34-41`). `line` and `overridable` are left out of the JSON when
/// absent, as TS leaves them `undefined`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    /// Which rule.
    pub code: IssueCode,
    /// The user-facing message.
    pub message: String,
    /// Index of the offending line, when the issue belongs to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// True when `override_reason` lets the entry post anyway.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overridable: Option<bool>,
}

impl Issue {
    /// An issue with no line and no override flag.
    #[must_use]
    pub fn new(code: IssueCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            line: None,
            overridable: None,
        }
    }

    /// Ties the issue to line `line` (0-based) with its override flag, as `validateEntry` does.
    #[must_use]
    pub fn on_line(mut self, line: usize, overridable: bool) -> Self {
        self.line = Some(line);
        self.overridable = Some(overridable);
        self
    }

    /// Sets the override flag of an issue that belongs to no line.
    #[must_use]
    pub fn overridable(mut self, overridable: bool) -> Self {
        self.overridable = Some(overridable);
        self
    }
}

/// A rejected ledger operation (`errors.ts:43-54`): one or more issues; the message is their
/// messages joined by one space and the code is the first issue's.
///
/// Holds at least one issue by construction, as TS reads `issues[0]` unchecked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerError {
    issues: Vec<Issue>,
}

impl LedgerError {
    /// A rejection from a list of issues; `None` when the list is empty.
    #[must_use]
    pub fn new(issues: Vec<Issue>) -> Option<Self> {
        (!issues.is_empty()).then_some(Self { issues })
    }

    /// `fail(code, message)` (`errors.ts:56-58`): a rejection with one issue.
    #[must_use]
    pub fn fail(code: IssueCode, message: impl Into<String>) -> Self {
        Self::from(Issue::new(code, message))
    }

    /// The first issue's code (`errors.ts:51-53`).
    #[must_use]
    pub fn code(&self) -> IssueCode {
        self.issues[0].code
    }

    /// Every issue, in the order they were found.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }
}

impl From<Issue> for LedgerError {
    fn from(issue: Issue) -> Self {
        Self {
            issues: vec![issue],
        }
    }
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, issue) in self.issues.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            f.write_str(&issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for LedgerError {}
