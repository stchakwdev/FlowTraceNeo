//! What `create_org_ledger` seeds until ledger-core can open books itself.
//!
//! TS seeds a new organisation with `createOrgLedger` (`flowtrace/src/lib/ledger/setup.ts:11-25`):
//! default settings, the NGO chart of accounts, the unrestricted fund and twelve periods through
//! the engine. ledger-core has neither the template nor `ensureFiscalYear` yet (tasks C2 and C6),
//! so the scaffold stores the default settings alone and no ledger rows. Task S4 replaces this
//! module with the ledger-core call and writes the seeded state back through
//! [`crate::codec::state_to_rows`].

use ledger_core::types::OrgLedgerSettings;

/// `defaultSettings` (`flowtrace/src/lib/ledger/coa-template.ts:123-142`).
///
/// # Errors
///
/// Never in practice: the settings literal does not parse.
pub fn default_settings(
    organization_id: &str,
    base_currency: &str,
) -> Result<OrgLedgerSettings, serde_json::Error> {
    // Through the serde shape, so the enum literals stay the TS ones (`ledger_core::types`).
    serde_json::from_value(serde_json::json!({
        "organization_id": organization_id,
        "base_currency": base_currency,
        "fiscal_year_start_month": 1,
        "basis": "accrual",
        "reporting_standard": "ifrs_sme",
        "system_accounts": {
            "accounts_payable": "acc-2100",
            "claims_payable": "acc-2110",
            "suspense": "acc-1400",
            "net_assets_unrestricted": "acc-3100",
            "net_assets_restricted": "acc-3200",
            "fx_gain_loss": "acc-4400",
        },
    }))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    #[test]
    fn default_settings_match_every_ts_case() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/setup.json");
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for case in fixture["defaultSettings"].as_array().unwrap() {
            let args = case["args"].as_array().unwrap();
            let org = args[0].as_str().unwrap();
            // TS defaults the currency to KES (coa-template.ts:125).
            let currency = args.get(1).map_or("KES", |c| c.as_str().unwrap());
            let want: OrgLedgerSettings = serde_json::from_value(case["result"].clone()).unwrap();
            assert_eq!(default_settings(org, currency).unwrap(), want, "{case}");
        }
    }
}
