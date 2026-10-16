//! Read access: per-caller `my_*` views over the private tables (`index.ts:193-345`).
//!
//! Each ledger view returns the rows of every organisation the caller is a member of, as the TS
//! views do with `member.where(identity = sender).rightSemijoin(table, org_id)`. Rust 2.12 has the
//! same query builder, so these are query views the server keeps up to date incrementally
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.view.html>, `impl Query<T>` returns).
//! `my_member` is the exception, see [`my_member`].

use spacetimedb::{Identity, Query, ViewContext};

use crate::tables::{
    AccountRow, AuditEventRow, BankAccountRow, BillRow, BudgetLineRow, BudgetRow, CommandResult,
    DimensionRow, ExpenseClaimRow, FiscalPeriodRow, FundRow, GrantRow, JournalEntryRow,
    JournalLineRow, LedgerOrg, Member, PaymentRow, StatementLineRow, VendorRow,
};
// The query-builder and read-only handle traits the table macro generates for each table.
use crate::tables::{
    account__query, audit_event__query, bank_account__query, bill__query, budget__query,
    budget_line__query, command_result__query, dimension__query, expense_claim__query,
    fiscal_period__query, fund__query, grant__query, journal_entry__query, journal_line__query,
    ledger_org__query, member__query, member__view, payment__query, statement_line__query,
    vendor__query,
};

/// Declares one view per line: the rows of `$table` in the caller's organisations.
macro_rules! member_views {
    ($($(#[$doc:meta])* $view:ident => $table:ident: $row:ty;)+) => {$(
        $(#[$doc])*
        #[spacetimedb::view(accessor = $view, public)]
        pub fn $view(ctx: &ViewContext) -> impl Query<$row> {
            ctx.from
                .member()
                .r#where(|m| m.identity.eq(ctx.sender()))
                .right_semijoin(ctx.from.$table(), |m, r| m.org_id.eq(r.org_id))
                .build()
        }
    )+};
}

member_views! {
    /// The caller's organisations (`index.ts:197-204`).
    my_ledger_org => ledger_org: LedgerOrg;
    /// Journal entries (`index.ts:213-220`).
    my_journal_entry => journal_entry: JournalEntryRow;
    /// Journal lines (`index.ts:221-228`).
    my_journal_line => journal_line: JournalLineRow;
    /// Accounts (`index.ts:229-236`).
    my_account => account: AccountRow;
    /// Funds (`index.ts:237-244`).
    my_fund => fund: FundRow;
    /// Grants (`index.ts:245-252`).
    my_grant => grant: GrantRow;
    /// Dimension values (`index.ts:253-260`).
    my_dimension => dimension: DimensionRow;
    /// Fiscal periods (`index.ts:261-268`).
    my_fiscal_period => fiscal_period: FiscalPeriodRow;
    /// Budgets (`index.ts:269-276`).
    my_budget => budget: BudgetRow;
    /// Budget lines (`index.ts:277-284`).
    my_budget_line => budget_line: BudgetLineRow;
    /// Vendors (`index.ts:285-292`).
    my_vendor => vendor: VendorRow;
    /// Bills (`index.ts:293-300`).
    my_bill => bill: BillRow;
    /// Payments (`index.ts:301-308`).
    my_payment => payment: PaymentRow;
    /// Expense claims (`index.ts:309-316`).
    my_expense_claim => expense_claim: ExpenseClaimRow;
    /// Bank, mobile money and cash accounts (`index.ts:317-324`).
    my_bank_account => bank_account: BankAccountRow;
    /// Statement lines (`index.ts:325-332`).
    my_statement_line => statement_line: StatementLineRow;
    /// The audit trail (`index.ts:333-340`).
    my_audit_event => audit_event: AuditEventRow;
}

/// Every member of the caller's organisations (`index.ts:205-212`).
///
/// TS writes this as a semijoin of `member` with itself. The Rust 2.12 query builder names both
/// sides by the table name alone (`spacetimedb-query-builder` 2.12.0 `join.rs`, `build`), so
/// a self-join would come out as the ambiguous `FROM "member" JOIN "member"`. This view reads
/// the two btree indexes instead, through [`members_visible_to`].
#[spacetimedb::view(accessor = my_member, public)]
#[must_use]
pub fn my_member(ctx: &ViewContext) -> Vec<Member> {
    members_visible_to(
        ctx.sender(),
        |identity| ctx.db.member().identity().filter(identity).collect(),
        |org_id| ctx.db.member().org_id().filter(org_id).collect(),
    )
}

/// The caller's latest command result (`index.ts:341-345`).
#[spacetimedb::view(accessor = my_command_result, public)]
pub fn my_command_result(ctx: &ViewContext) -> impl Query<CommandResult> {
    ctx.from
        .command_result()
        .r#where(|r| r.identity.eq(ctx.sender()))
        .build()
}

/// The members `sender` may see: every member of each organisation `sender` belongs to, in the
/// order of `sender`'s own member rows. The member key makes one row per identity and
/// organisation, so no organisation comes twice.
#[must_use]
pub fn members_visible_to(
    sender: Identity,
    members_of_identity: impl Fn(Identity) -> Vec<Member>,
    members_of_org: impl Fn(&str) -> Vec<Member>,
) -> Vec<Member> {
    members_of_identity(sender)
        .iter()
        .flat_map(|mine| members_of_org(&mine.org_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALLER: Identity = Identity::from_byte_array([0xab; 32]);

    fn hex() -> String {
        "ab".repeat(32)
    }

    fn semijoin_sql(table: &str) -> String {
        format!(
            r#"SELECT "{table}".* FROM "member" JOIN "{table}" ON "member"."org_id" = "{table}"."org_id" WHERE ("member"."identity" = 0x{})"#,
            hex()
        )
    }

    /// Every semijoin view with the table it reads.
    fn ledger_views() -> Vec<(&'static str, String)> {
        let ctx = ViewContext::new(CALLER);
        vec![
            ("ledger_org", my_ledger_org(&ctx).sql().to_owned()),
            ("journal_entry", my_journal_entry(&ctx).sql().to_owned()),
            ("journal_line", my_journal_line(&ctx).sql().to_owned()),
            ("account", my_account(&ctx).sql().to_owned()),
            ("fund", my_fund(&ctx).sql().to_owned()),
            ("grant", my_grant(&ctx).sql().to_owned()),
            ("dimension", my_dimension(&ctx).sql().to_owned()),
            ("fiscal_period", my_fiscal_period(&ctx).sql().to_owned()),
            ("budget", my_budget(&ctx).sql().to_owned()),
            ("budget_line", my_budget_line(&ctx).sql().to_owned()),
            ("vendor", my_vendor(&ctx).sql().to_owned()),
            ("bill", my_bill(&ctx).sql().to_owned()),
            ("payment", my_payment(&ctx).sql().to_owned()),
            ("expense_claim", my_expense_claim(&ctx).sql().to_owned()),
            ("bank_account", my_bank_account(&ctx).sql().to_owned()),
            ("statement_line", my_statement_line(&ctx).sql().to_owned()),
            ("audit_event", my_audit_event(&ctx).sql().to_owned()),
        ]
    }

    fn member(identity: Identity, org_id: &str, user_id: &str) -> Member {
        Member {
            key: format!("{org_id}:{user_id}"),
            identity,
            org_id: org_id.to_owned(),
            user_id: user_id.to_owned(),
            name: user_id.to_owned(),
            can_post: false,
            can_approve: false,
            can_close: false,
        }
    }

    fn all_members() -> Vec<Member> {
        let other = Identity::from_byte_array([1; 32]);
        let third = Identity::from_byte_array([2; 32]);
        vec![
            member(CALLER, "org-b", "me-b"),
            member(other, "org-a", "other-a"),
            member(CALLER, "org-a", "me-a"),
            member(other, "org-b", "other-b"),
            member(third, "org-c", "third-c"),
        ]
    }

    fn visible(sender: Identity) -> Vec<String> {
        let all = all_members();
        members_visible_to(
            sender,
            |id| all.iter().filter(|m| m.identity == id).cloned().collect(),
            |org| all.iter().filter(|m| m.org_id == org).cloned().collect(),
        )
        .into_iter()
        .map(|m| m.user_id)
        .collect()
    }

    #[test]
    fn there_is_a_view_for_each_of_the_seventeen_ts_ledger_views() {
        assert_eq!(ledger_views().len(), 17);
    }

    #[test]
    fn every_ledger_view_returns_only_rows_of_the_callers_organisations() {
        for (table, sql) in ledger_views() {
            assert_eq!(sql, semijoin_sql(table), "my_{table}");
        }
    }

    #[test]
    fn my_command_result_returns_only_the_callers_row() {
        let ctx = ViewContext::new(CALLER);
        assert_eq!(
            my_command_result(&ctx).sql(),
            format!(
                r#"SELECT * FROM "command_result" WHERE ("command_result"."identity" = 0x{})"#,
                hex()
            )
        );
    }

    #[test]
    fn a_member_sees_every_member_of_their_organisations_and_no_other() {
        assert_eq!(visible(CALLER), ["me-b", "other-b", "other-a", "me-a"]);
    }

    #[test]
    fn a_stranger_sees_no_member() {
        assert_eq!(
            visible(Identity::from_byte_array([9; 32])),
            Vec::<String>::new()
        );
    }
}
