//! Typed ids. TS types every id as a plain `string` (`flowtrace/src/lib/ledger/types.ts`); Rust
//! wraps each kind in its own newtype so an account id cannot be passed where a fund id belongs.
//! All of them serialise as the bare string.

use std::borrow::Borrow;
use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($($(#[$doc:meta])* $name:ident;)+) => {$(
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an id string.
            #[must_use]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The id as a string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self(id.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    )+};
}

string_id! {
    /// An organisation (`OrgLedgerSettings.organization_id`).
    OrganizationId;
    /// A chart-of-accounts account (`Account.id`, for example `acc-1110`).
    AccountId;
    /// A fund (`Fund.id`, for example `fund-unrestricted`).
    FundId;
    /// A grant (`Grant.id`).
    GrantId;
    /// A dimension value (`DimensionValue.id`).
    DimensionId;
    /// A fiscal period (`FiscalPeriod.id`, for example `period-2026-01`).
    PeriodId;
    /// A journal entry (`JournalEntry.id`).
    EntryId;
    /// A journal line (`JournalLine.id`).
    LineId;
    /// A budget (`Budget.id`).
    BudgetId;
    /// A budget line (`BudgetLine.id`).
    BudgetLineId;
    /// A vendor (`Vendor.id`).
    VendorId;
    /// A bill (`Bill.id`).
    BillId;
    /// A coded line of a bill or claim (`CodedLine.id`).
    CodedLineId;
    /// A payment (`Payment.id`).
    PaymentId;
    /// An expense claim (`ExpenseClaim.id`).
    ClaimId;
    /// A bank, mobile money or cash account (`BankAccount.id`).
    BankAccountId;
    /// A statement line (`StatementLine.id`).
    StatementLineId;
    /// One statement import (`StatementLine.import_batch`).
    ImportBatchId;
    /// An audit event (`AuditEvent.id`).
    AuditEventId;
    /// A user or the system acting on the books (`Actor.id`, `created_by`, `by`, `claimant`).
    ActorId;
    /// A node of the flow map (`donor_node_id`, `node_id`, `flow_node_id`).
    FlowNodeId;
}
