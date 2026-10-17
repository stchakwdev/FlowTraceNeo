//! The two WorkOS AuthKit calls of a public client: the authorize URL the browser opens and
//! `POST /user_management/authenticate` for the code exchange and the refresh
//! (<https://workos.com/docs/reference/authkit/authentication/get-authorization-url/pkce>,
//! <https://workos.com/docs/reference/authkit/authentication/code>,
//! <https://workos.com/docs/reference/authkit/authentication/refresh-token>). No client secret
//! is sent: the verifier stands in for it on the code exchange, and the WorkOS Node SDK's public
//! client mode omits it on the refresh.

use std::fmt;
use std::time::Duration;

use ledger_client::AccessToken;
use serde::Deserialize;
use ureq::Agent;

use super::pkce::Pkce;
use super::{Session, SignInError, SignedInUser, TokenClaims, WorkosSettings};

/// How long one call to WorkOS may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The authorize URL for one sign-in: AuthKit's hosted page, which sends the browser back to
/// `redirect_uri` with `code` and `state`.
#[must_use]
pub fn authorize_url(settings: &WorkosSettings, redirect_uri: &str, pkce: &Pkce) -> String {
    let challenge = pkce.challenge();
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("response_type", "code")
        .append_pair("provider", "authkit")
        .append_pair("client_id", &settings.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", pkce.state())
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .finish();
    format!("{}/user_management/authorize?{query}", settings.api)
}

/// The WorkOS user object, the fields the app shows.
#[derive(Debug, Clone, Deserialize)]
struct User {
    id: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    first_name: Option<String>,
    #[serde(default)]
    last_name: Option<String>,
}

/// The authenticate answer (both grants).
#[derive(Clone, Deserialize)]
pub struct AuthAnswer {
    user: User,
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    organization_id: Option<String>,
}

impl fmt::Debug for AuthAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthAnswer")
            .field("user", &self.user.id)
            .finish_non_exhaustive()
    }
}

impl AuthAnswer {
    /// The refresh token to save.
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    /// The session the answer opens.
    ///
    /// # Errors
    ///
    /// [`SignInError::Network`] when the access token is not a JWT with `iss` and `sub`.
    pub fn session(&self) -> Result<Session, SignInError> {
        let claims = TokenClaims::of(&self.access_token).ok_or_else(|| {
            SignInError::Network("the access token WorkOS sent is not a JWT.".to_owned())
        })?;
        let name = [&self.user.first_name, &self.user.last_name]
            .into_iter()
            .flatten()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(Session {
            access_token: AccessToken::new(self.access_token.clone()),
            claims,
            user: SignedInUser {
                id: self.user.id.clone(),
                name: if name.is_empty() {
                    self.user.email.clone()
                } else {
                    name
                },
                email: self.user.email.clone(),
            },
            organization_id: self.organization_id.clone(),
        })
    }
}

/// WorkOS's error body: `error` and `error_description` (OAuth), or `message`.
#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// The WorkOS endpoints of one environment.
pub struct Workos {
    settings: WorkosSettings,
    agent: Agent,
}

impl fmt::Debug for Workos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Workos")
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

impl Workos {
    /// The endpoints `settings` names. ureq's default TLS (rustls with the webpki roots,
    /// <https://docs.rs/ureq/3.1.0/ureq/#tls-rustls-by-default>), as the proofs Worker client
    /// uses; error statuses are read as answers so WorkOS's error body reaches the screen.
    #[must_use]
    pub fn new(settings: WorkosSettings) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(CALL_TIMEOUT))
            .build()
            .new_agent();
        Self { settings, agent }
    }

    /// The settings these endpoints are for.
    #[must_use]
    pub fn settings(&self) -> &WorkosSettings {
        &self.settings
    }

    /// Trades the browser's `code` for tokens, proving the sign-in with `pkce`'s verifier.
    ///
    /// # Errors
    ///
    /// [`SignInError::Refused`] when WorkOS refuses the code, [`SignInError::Network`] when it
    /// cannot be reached.
    pub fn exchange_code(&self, code: &str, pkce: &Pkce) -> Result<AuthAnswer, SignInError> {
        self.authenticate(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": self.settings.client_id,
            "code": code,
            "code_verifier": pkce.verifier(),
        }))
    }

    /// Trades a refresh token for a new access token and, possibly rotated, refresh token.
    ///
    /// # Errors
    ///
    /// [`SignInError::Refused`] when WorkOS refuses the refresh token (revoked, expired or
    /// already used), [`SignInError::Network`] when it cannot be reached.
    pub fn refresh(&self, refresh_token: &str) -> Result<AuthAnswer, SignInError> {
        self.authenticate(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": self.settings.client_id,
            "refresh_token": refresh_token,
        }))
    }

    fn authenticate(&self, body: &serde_json::Value) -> Result<AuthAnswer, SignInError> {
        let network = |e: &dyn fmt::Display| SignInError::Network(e.to_string());
        let url = format!("{}/user_management/authenticate", self.settings.api);
        let mut response = self
            .agent
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send(body.to_string())
            .map_err(|e| network(&e))?;
        let status = response.status();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| network(&e))?;
        if status.is_success() {
            return serde_json::from_str(&text).map_err(|e| {
                SignInError::Network(format!("WorkOS answered in an unexpected shape ({e})."))
            });
        }
        let reason = serde_json::from_str::<ErrorBody>(&text)
            .ok()
            .and_then(|b| b.error_description.or(b.message).or(b.error))
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        if status.is_client_error() {
            Err(SignInError::Refused(reason))
        } else {
            Err(SignInError::Network(reason))
        }
    }
}
