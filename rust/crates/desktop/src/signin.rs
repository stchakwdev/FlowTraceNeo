//! Signing in with WorkOS AuthKit through the system browser (AUTH2, pre-decision P11), with no
//! GPUI types so every step is unit-tested; the store runs the blocking steps off the UI thread.
//!
//! The flow is the OAuth 2.0 authorization code grant for a public client with PKCE
//! (<https://www.rfc-editor.org/rfc/rfc8252>, native apps; RFC 7636, PKCE):
//!
//! 1. [`pkce::Pkce::new`] draws a verifier and a state; [`loopback::Loopback::bind`] listens on
//!    an ephemeral `127.0.0.1` port for that one callback.
//! 2. The system browser opens [`workos::authorize_url`]:
//!    `GET {api}/user_management/authorize?response_type=code&provider=authkit&client_id=..&redirect_uri=http://127.0.0.1:<port>/callback&state=..&code_challenge=..&code_challenge_method=S256`
//!    (<https://workos.com/docs/reference/authkit/authentication/get-authorization-url/pkce>).
//! 3. [`loopback::Loopback::wait`] takes the redirect, checks the state and returns the code.
//! 4. [`workos::Workos::exchange_code`]: `POST {api}/user_management/authenticate` with
//!    `grant_type=authorization_code`, `client_id`, `code` and `code_verifier`, no client secret
//!    (<https://workos.com/docs/reference/authkit/authentication/code>). The answer holds the
//!    user, an access token (a JWT) and a refresh token.
//! 5. The refresh token is saved in the sign-in token file (0600, [`ledger_client::TokenFile`]);
//!    the access token stays in memory ([`Session`]) and the connection presents it.
//! 6. Before the access token expires ([`refresh_delay`]) and on every launch,
//!    [`workos::Workos::refresh`] trades the refresh token for a new pair:
//!    `grant_type=refresh_token` with `client_id` and `refresh_token`, no secret for a public
//!    client (<https://workos.com/docs/reference/authkit/authentication/refresh-token>; the
//!    WorkOS Node SDK's public client mode omits `client_secret` the same way). WorkOS may rotate
//!    the refresh token, so the new one replaces the old in the file.
//!
//! Sign-out removes the file. Without a `workos_client_id` in `config.toml` none of this runs and
//! the app connects with its device identity, as before.

pub mod flow;
pub mod loopback;
pub mod pkce;
pub mod state;
pub mod workos;

use std::path::PathBuf;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ledger_client::{AccessToken, TokenFile, identity_of_subject};

/// The WorkOS API host when `config.toml` names none.
pub const WORKOS_API: &str = "https://api.workos.com";

/// What the app needs to sign in with WorkOS: the `workos_*` keys of `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkosSettings {
    /// The environment's client id, `client_...` (`workos_client_id`). A public client: the app
    /// holds no secret.
    pub client_id: String,
    /// The `iss` of the environment's access tokens (`workos_issuer`); by default
    /// `https://api.workos.com/user_management/<client id>` (`rust/docs/module-release.md`,
    /// section 5).
    pub issuer: String,
    /// The API host the authorize and authenticate endpoints live on (`workos_api`), WorkOS's
    /// own by default; the local proof points it at a test server.
    pub api: String,
}

impl WorkosSettings {
    /// The settings for `client_id`, with the default issuer and API host where `issuer` and
    /// `api` are `None`.
    #[must_use]
    pub fn new(client_id: &str, issuer: Option<&str>, api: Option<&str>) -> Self {
        let client_id = client_id.trim().to_owned();
        let api = api.map_or(WORKOS_API, str::trim).trim_end_matches('/');
        let issuer = issuer.map_or_else(
            || format!("{WORKOS_API}/user_management/{client_id}"),
            |i| i.trim().to_owned(),
        );
        Self {
            client_id,
            issuer,
            api: api.to_owned(),
        }
    }
}

/// The sign-in token file for the device token file at `token_path`: the same path with
/// `.workos` appended. A separate file, so a sign-out leaves the device identity as it was.
#[must_use]
pub fn refresh_file(token_path: &str) -> TokenFile {
    TokenFile::new(PathBuf::from(format!("{token_path}.workos")))
}

/// The signed-in person, from the WorkOS user object of the authenticate answer
/// (<https://workos.com/docs/reference/authkit/user>). WorkOS's authenticate endpoint returns no
/// id token, so the user object is where name and email come from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignedInUser {
    /// The WorkOS user id, `user_...`: the access token's `sub`.
    pub id: String,
    /// First and last name joined; the email when both are empty.
    pub name: String,
    /// Email address.
    pub email: String,
}

/// The claims of an access token the app needs: who it names and for whom it was issued. Read
/// without checking the signature, which is the host's job (ADR 0002, AUTH1 section).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenClaims {
    /// `iss`.
    pub issuer: String,
    /// `sub`.
    pub subject: String,
    /// The first `aud`, else `client_id`, as the module matches it
    /// (`crates/ledger-module/src/auth.rs`, `TokenClaims::audiences`).
    pub audience: Option<String>,
    /// `exp`, seconds since 1970.
    pub expires_at: Option<u64>,
}

impl TokenClaims {
    /// The claims of a JWT's payload; `None` when `token` is not a JWT with `iss` and `sub`.
    #[must_use]
    pub fn of(token: &str) -> Option<Self> {
        let payload = token.split('.').nth(1)?;
        let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
        let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let text = |key: &str| claims.get(key)?.as_str().map(str::to_owned);
        let aud = match claims.get("aud") {
            Some(serde_json::Value::String(a)) => Some(a.clone()),
            Some(serde_json::Value::Array(list)) => {
                list.iter().find_map(|a| a.as_str()).map(str::to_owned)
            }
            _ => None,
        };
        Some(Self {
            issuer: text("iss")?,
            subject: text("sub")?,
            audience: aud.or_else(|| text("client_id")),
            expires_at: claims.get("exp").and_then(serde_json::Value::as_u64),
        })
    }

    /// The identity the host gives this token ([`identity_of_subject`]).
    #[must_use]
    pub fn identity(&self) -> String {
        identity_of_subject(&self.issuer, &self.subject)
    }

    /// What to tell the person when the module refused the connection, word for word the web's
    /// (`flowtrace/src/lib/ledger/spacetime-auth.ts`, `refusalMessage`, workos mode): the issuer
    /// and audience an operator registers with `add_trusted_issuer`.
    #[must_use]
    pub fn refusal_message(&self) -> String {
        match &self.audience {
            Some(audience) => format!(
                "SpacetimeDB refused this sign-in: an operator must trust issuer {} for audience \
                 {audience} on this database",
                self.issuer
            ),
            None => "SpacetimeDB refused the connection".to_owned(),
        }
    }
}

/// A signed-in session: the access token the connection presents (memory only), its claims and
/// the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The access token; `Debug` hides it.
    pub access_token: AccessToken,
    /// Its claims.
    pub claims: TokenClaims,
    /// Who signed in.
    pub user: SignedInUser,
    /// The WorkOS organisation the session is for, when there is one.
    pub organization_id: Option<String>,
}

/// How long to wait before refreshing a token that expires at `expires_at` when it is `now`
/// (both seconds since 1970): its lifetime less a lead of a fifth of what is left, at most 60 s,
/// and never less than 5 s. WorkOS access tokens live five minutes by default, so the app
/// refreshes four minutes in; an expired or undated token is refreshed in 5 s.
#[must_use]
pub fn refresh_delay(expires_at: Option<u64>, now: u64) -> Duration {
    const FLOOR: u64 = 5;
    let Some(exp) = expires_at else {
        return Duration::from_secs(FLOOR);
    };
    let left = exp.saturating_sub(now);
    let lead = (left / 5).min(60);
    Duration::from_secs(left.saturating_sub(lead).max(FLOOR))
}

/// Why signing in failed, in words for the Connect screen. Never holds a token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignInError {
    /// The local listener or the random source failed.
    #[error("Signing in could not start: {0}")]
    Local(String),
    /// Nobody came back from the browser in time.
    #[error("Signing in timed out; nothing came back from the browser.")]
    TimedOut,
    /// The browser came back with an error, or with another state.
    #[error("Signing in was not completed: {0}")]
    Callback(String),
    /// WorkOS refused the code or the refresh token; signing in again fixes it.
    #[error("WorkOS refused the sign-in: {0}")]
    Refused(String),
    /// WorkOS could not be reached or answered in a way the app does not read.
    #[error("WorkOS could not be reached: {0}")]
    Network(String),
    /// The sign-in token file could not be read or written.
    #[error("{0}")]
    TokenFile(String),
}

#[cfg(test)]
mod tests;
