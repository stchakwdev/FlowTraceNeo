//! Generated binding rows into the cache's [`Row`]s.
//!
//! The `my_*` views return the module's table rows unchanged
//! (`crates/ledger-module/src/views.rs`), so `spacetime generate` gives structs with the same
//! fields as `ledger_module::tables`; journal entries and lines move field by field into the
//! module structs and doc rows into [`DocColumns`]. Nothing is reshaped, so pre-decision P6
//! (rows that do not map back one to one) does not apply.

use ledger_module::tables;

use crate::backend::MemberInfo;
use crate::cache::{DocColumns, DocTable, MemberRow, OrgHeader, ResultRow, Row};
use crate::module_bindings as b;

/// Moves every listed field of `$from` into a `$to` of the same field names.
macro_rules! same_fields {
    ($from:expr => $to:path { $($field:ident),+ $(,)? }) => {{
        let r = $from;
        $to { $($field: r.$field),+ }
    }};
}

impl From<b::JournalEntryRow> for Row {
    fn from(r: b::JournalEntryRow) -> Self {
        Self::Entry(same_fields!(r => tables::JournalEntryRow {
            key, org_id, id, seq, number, date, period_id, memo, source_type, source_id, status,
            reversal_of, reversed_by, override_reason, created_by, created_at, posted_by,
            posted_at,
        }))
    }
}

impl From<b::JournalLineRow> for Row {
    fn from(r: b::JournalLineRow) -> Self {
        Self::Line(same_fields!(r => tables::JournalLineRow {
            key, org_id, id, seq, entry_id, account_id, debit, credit, currency, fx_rate,
            base_amount, fund_id, grant_id, dimension_ids, flow_node_id, memo,
        }))
    }
}

impl From<b::LedgerOrg> for Row {
    fn from(r: b::LedgerOrg) -> Self {
        Self::Org(OrgHeader {
            org_id: r.org_id,
            settings_json: r.settings_json,
            counters_json: r.counters_json,
        })
    }
}

impl From<b::Member> for Row {
    fn from(r: b::Member) -> Self {
        Self::Member(MemberRow {
            key: r.key,
            org_id: r.org_id,
            info: MemberInfo {
                identity: r.identity.to_hex().to_string(),
                user_id: r.user_id,
                name: r.name,
                can_post: r.can_post,
                can_approve: r.can_approve,
                can_close: r.can_close,
            },
        })
    }
}

impl From<b::CommandResult> for Row {
    fn from(r: b::CommandResult) -> Self {
        Self::Result(ResultRow {
            org_id: r.org_id,
            request_id: r.request_id,
            result_json: r.result_json,
        })
    }
}

/// `From` for each doc table's binding row.
macro_rules! doc_rows {
    ($($row:ident => $table:ident;)+) => {$(
        impl From<b::$row> for Row {
            fn from(r: b::$row) -> Self {
                Self::Doc(
                    DocTable::$table,
                    same_fields!(r => DocColumns { key, org_id, id, seq, data }),
                )
            }
        }
    )+};
}

doc_rows! {
    AccountRow => Account;
    FundRow => Fund;
    GrantRow => Grant;
    DimensionRow => Dimension;
    FiscalPeriodRow => FiscalPeriod;
    BudgetRow => Budget;
    BudgetLineRow => BudgetLine;
    VendorRow => Vendor;
    BillRow => Bill;
    PaymentRow => Payment;
    ExpenseClaimRow => ExpenseClaim;
    BankAccountRow => BankAccount;
    StatementLineRow => StatementLine;
    AuditEventRow => AuditEvent;
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_module::codec::{entry_to_row, line_to_row};
    use spacetimedb_sdk::Identity;

    use crate::testing::demo_state;

    #[test]
    fn a_binding_entry_row_becomes_the_module_row() {
        let entry = demo_state().entries[0].clone();
        let m = entry_to_row("demo", &entry, 4).unwrap();
        let binding = same_fields!(m.clone() => b::JournalEntryRow {
            key, org_id, id, seq, number, date, period_id, memo, source_type, source_id, status,
            reversal_of, reversed_by, override_reason, created_by, created_at, posted_by,
            posted_at,
        });
        assert_eq!(Row::from(binding), Row::Entry(m));
    }

    #[test]
    fn a_binding_line_row_becomes_the_module_row() {
        let line = demo_state().lines[0].clone();
        let m = line_to_row("demo", &line, 9).unwrap();
        let binding = same_fields!(m.clone() => b::JournalLineRow {
            key, org_id, id, seq, entry_id, account_id, debit, credit, currency, fx_rate,
            base_amount, fund_id, grant_id, dimension_ids, flow_node_id, memo,
        });
        assert_eq!(Row::from(binding), Row::Line(m));
    }

    #[test]
    fn a_member_row_carries_the_identity_as_hex() {
        let row = Row::from(b::Member {
            key: "demo:ab".to_owned(),
            identity: Identity::from_byte_array([0xab; 32]),
            org_id: "demo".to_owned(),
            user_id: "u1".to_owned(),
            name: "Ama".to_owned(),
            can_post: true,
            can_approve: false,
            can_close: true,
        });
        let Row::Member(m) = row else {
            panic!("not a member");
        };
        assert_eq!(m.info.identity, "ab".repeat(32));
    }
}
