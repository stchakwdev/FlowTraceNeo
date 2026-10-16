//! Ledger core: money, types, engine, commands, reports, reconcile, importers and demo books, with no I/O.

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    #[test]
    fn test_module_should_compile_under_workspace_lints() {
        let parsed: u32 = "42".parse().unwrap();
        assert_eq!(parsed, 42);
    }
}
