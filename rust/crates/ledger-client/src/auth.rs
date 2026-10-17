//! Signing in with an OpenID Connect token instead of a device identity (AUTH2).
//!
//! A client presents either the device token kept in a [`crate::TokenFile`], which the host
//! minted and the client saves again on every connect, or an [`AccessToken`] from a sign-in
//! (a WorkOS AuthKit access token), which is held in memory only and never written: the host
//! derives the identity from the token's issuer and subject, so a fresh token for the same person
//! is the same identity ([`identity_of_subject`]; `rust/docs/adr/0002-gpui-spacetime.md`, dated
//! AUTH1 section).

use std::fmt;

/// An access token from a sign-in. Its `Debug` never shows the token, and nothing in this crate
/// writes it to disk or into an error.
#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken(String);

impl AccessToken {
    /// Wraps `token`, a JWT.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The token itself, for the connection's `Authorization` and nothing else.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(..)")
    }
}

/// The identity, as 64 hex digits, that the host gives a token with issuer `issuer` and subject
/// `subject`: `Identity::from_claims`, a blake3 hash of `"{iss}|{sub}"` with the `c200` prefix
/// (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/struct.JwtClaims.html#method.identity>). The
/// module's `add_member_by_subject` computes the same (`ledger_module::auth::member_identity`).
#[must_use]
pub fn identity_of_subject(issuer: &str, subject: &str) -> String {
    spacetimedb_sdk::Identity::from_claims(issuer, subject)
        .to_hex()
        .to_string()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use crate::{ClientConfig, TokenFile};

    #[test]
    fn an_access_token_is_presented_and_the_returned_token_is_not_saved() {
        let dir =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tmp"))
                .join(format!("ledger-client-auth-{}", std::process::id()));
        let file = TokenFile::new(dir.join("token"));
        file.save("device-token").unwrap();
        let device = ClientConfig::new("http://127.0.0.1:3100", "ft-x").with_token_file(file);
        assert_eq!(
            device.presented_token().unwrap(),
            (Some("device-token".to_owned()), true),
            "the device token is presented and the host's answer saved"
        );
        let signed_in = device.with_access_token(AccessToken::new("eyJ.access.sig"));
        assert_eq!(
            signed_in.presented_token().unwrap(),
            (Some("eyJ.access.sig".to_owned()), false),
            "a sign-in presents its access token and saves nothing"
        );
        assert!(!format!("{signed_in:?}").contains("eyJ.access.sig"));
        let anonymous = ClientConfig::new("http://127.0.0.1:3100", "ft-x");
        assert_eq!(anonymous.presented_token().unwrap(), (None, false));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn debug_never_shows_the_token() {
        let token = AccessToken::new("eyJ.secret.sig");
        assert_eq!(format!("{token:?}"), "AccessToken(..)");
        assert_eq!(token.expose(), "eyJ.secret.sig");
    }

    #[test]
    fn the_identity_of_a_subject_is_the_one_the_local_host_minted() {
        // The pair the module's own test pins (crates/ledger-module/src/auth.rs,
        // from_claims_matches_an_identity_the_local_host_minted).
        assert_eq!(
            identity_of_subject("localhost", "aad0459e-6590-4f63-9608-2a578b08115f"),
            "c200f4a9e85b11d24644bd91cccc13e60094aedaabc166cd599932994c53ba96"
        );
    }
}
