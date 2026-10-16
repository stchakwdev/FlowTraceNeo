//! What a backend call can fail with, and the reducer error text decoded back into a
//! [`LedgerError`] (`toLedgerError`, `flowtrace/src/lib/ledger/spacetime-backend.ts:159-168`).

use ledger_core::{Issue, LedgerError};
use ledger_module::codec::CodecError;
use ledger_module::command::LEDGER_ERROR_PREFIX;

/// Why a backend call failed.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The engine refused the command; the issues as the engine raised them.
    #[error(transparent)]
    Ledger(LedgerError),
    /// The reducer refused the call with a text that is not a ledger error: no membership, not
    /// an operator, malformed JSON, or a `money.ts` range error (`index.ts:512-518`).
    #[error("{0}")]
    Rejected(String),
    /// The caller is not a member of the organisation, or it has no books
    /// (`spacetime-backend.ts:147`).
    #[error("No ledger for organisation {0}, or you are not a member of it.")]
    NoLedger(String),
    /// The organisation id is not `^[A-Za-z0-9_-]{1,128}$` (`spacetime-backend.ts:117`).
    #[error("Invalid organisation id {0}.")]
    InvalidOrgId(String),
    /// The rows of an organisation did not rebuild into a `LedgerState`.
    #[error("The books did not load: {0}")]
    Codec(#[from] CodecError),
    /// The command did not serialise to JSON (never in practice).
    #[error("The command did not serialise: {0}")]
    Json(#[from] serde_json::Error),
    /// No `command_result` row and no reducer error within the wait (`RESULT_TIMEOUT_MS`,
    /// `spacetime-backend.ts:29`).
    #[error("The ledger did not confirm the command in time.")]
    Timeout,
    /// There is no live connection to send on.
    #[error("Not connected to the ledger.")]
    Disconnected,
    /// The SDK or the WebSocket failed.
    #[error("SpacetimeDB: {0}")]
    Sdk(String),
    /// The token file could not be read or written.
    #[error("Token file {path}: {source}")]
    Token {
        /// The file the caller named.
        path: std::path::PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
}

/// A reducer's error text as the caller sees it: `LEDGER_ERROR <issues JSON>` becomes
/// [`ClientError::Ledger`], anything else (or a prefix with no readable, non-empty issue list)
/// [`ClientError::Rejected`] with the text unchanged, as `toLedgerError` returns the original
/// error (`spacetime-backend.ts:159-168`).
#[must_use]
pub fn decode_reducer_error(text: &str) -> ClientError {
    text.strip_prefix(LEDGER_ERROR_PREFIX)
        .and_then(|json| serde_json::from_str::<Vec<Issue>>(json).ok())
        .and_then(LedgerError::new)
        .map_or_else(
            || ClientError::Rejected(text.to_owned()),
            ClientError::Ledger,
        )
}

/// `ORG_ID` (`spacetime-backend.ts:111`, `index.ts:533`): letters, digits, `_` and `-`, 1 to
/// 128 of them.
#[must_use]
pub fn is_valid_org_id(org_id: &str) -> bool {
    (1..=128).contains(&org_id.len())
        && org_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_core::IssueCode;
    use ledger_module::command::ledger_error_text;

    #[test]
    fn a_ledger_error_text_decodes_to_the_same_issues() {
        let error = LedgerError::new(vec![
            Issue::new(IssueCode::Unbalanced, "Debits and credits differ.").on_line(1, false),
            Issue::new(IssueCode::PeriodClosed, "The period is closed."),
        ])
        .unwrap();
        let ClientError::Ledger(back) = decode_reducer_error(&ledger_error_text(&error)) else {
            panic!("not a ledger error");
        };
        assert_eq!(back, error);
    }

    #[test]
    fn any_other_text_is_a_rejection_word_for_word() {
        let text = "Only an operator can import books.";
        assert!(matches!(decode_reducer_error(text), ClientError::Rejected(t) if t == text));
    }

    #[test]
    fn a_prefix_with_unreadable_or_empty_issues_stays_the_text() {
        for text in ["LEDGER_ERROR not json", "LEDGER_ERROR []"] {
            assert!(
                matches!(decode_reducer_error(text), ClientError::Rejected(t) if t == text),
                "{text}"
            );
        }
    }

    #[test]
    fn org_ids_follow_the_module_pattern() {
        assert!(is_valid_org_id("demo-org_1"));
        assert!(is_valid_org_id(&"a".repeat(128)));
        for bad in ["", "org 1", "org'1", "\u{e9}t\u{e9}", &"a".repeat(129)] {
            assert!(!is_valid_org_id(bad), "{bad}");
        }
    }
}
