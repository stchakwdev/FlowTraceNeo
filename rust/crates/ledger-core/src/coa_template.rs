//! The default chart of accounts for an NGO and the default settings, mirroring
//! `flowtrace/src/lib/ledger/coa-template.ts`: presented the IFRS for SMEs way and tagged with
//! functional classes for the Statement of Functional Expenses; seeded for every new
//! organisation and editable afterwards (`coa-template.ts:1-4`).

use crate::types::{Account, AccountId, OrgLedgerSettings};

/// `UNRESTRICTED_FUND_ID` (`coa-template.ts:144`).
pub const UNRESTRICTED_FUND_ID: &str = "fund-unrestricted";

/// `accountId(code)` (`coa-template.ts:89`): `acc-<code>`.
#[must_use]
pub fn account_id(code: &str) -> AccountId {
    let _ = code;
    AccountId::from("")
}

/// `ngoChartOfAccounts()` (`coa-template.ts:102-121`).
#[must_use]
pub fn ngo_chart_of_accounts() -> Vec<Account> {
    Vec::new()
}

/// `defaultSettings(organizationId, baseCurrency = 'KES')` (`coa-template.ts:123-142`).
#[must_use]
pub fn default_settings(organization_id: &str, base_currency: &str) -> OrgLedgerSettings {
    let _ = (organization_id, base_currency);
    todo_settings()
}

fn todo_settings() -> OrgLedgerSettings {
    let a = AccountId::from("");
    OrgLedgerSettings {
        organization_id: "".into(),
        base_currency: String::new(),
        fiscal_year_start_month: 1,
        basis: crate::types::Basis::Accrual,
        reporting_standard: crate::types::ReportingStandard::IfrsSme,
        system_accounts: crate::types::SystemAccounts {
            accounts_payable: a.clone(),
            claims_payable: a.clone(),
            suspense: a.clone(),
            net_assets_unrestricted: a.clone(),
            net_assets_restricted: a.clone(),
            fx_gain_loss: a,
        },
    }
}
