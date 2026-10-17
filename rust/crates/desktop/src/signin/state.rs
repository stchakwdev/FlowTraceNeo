//! What the Connect screen shows of the sign-in, as plain transitions the store drives
//! (`store.rs`); tested without a window.

use crate::backend::ConnectionStatus;

use super::{Session, SignInError};

/// Where the sign-in stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No `workos_client_id` in `config.toml`: device identity only.
    Off,
    /// Not signed in: the app connects with its device identity.
    SignedOut,
    /// Trading the saved refresh token for an access token (at launch or on a reconnect).
    Resuming,
    /// The browser is open on the AuthKit page.
    Waiting,
    /// Signed in: the connection presents the session's access token.
    SignedIn,
}

/// The sign-in state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInState {
    phase: Phase,
    session: Option<Session>,
    error: Option<String>,
    refusal: Option<String>,
    seen_connecting: bool,
    reached: bool,
}

impl SignInState {
    /// The state at launch: [`Phase::SignedOut`] when sign-in is configured, else
    /// [`Phase::Off`].
    #[must_use]
    pub fn new(available: bool) -> Self {
        Self {
            phase: if available {
                Phase::SignedOut
            } else {
                Phase::Off
            },
            session: None,
            error: None,
            refusal: None,
            seen_connecting: false,
            reached: false,
        }
    }

    /// Where the sign-in stands.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The session while signed in.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The last sign-in or refresh failure, for the Connect screen.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The module refused the signed-in connection: the web's message naming the issuer and
    /// audience to register ([`super::TokenClaims::refusal_message`]).
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }

    /// The browser was opened.
    pub fn waiting(&mut self) {
        self.phase = Phase::Waiting;
        self.error = None;
    }

    /// The saved refresh token is being traded.
    pub fn resuming(&mut self) {
        self.phase = Phase::Resuming;
        self.error = None;
    }

    /// Signed in with `session`; its connection starts next.
    pub fn signed_in(&mut self, session: Session) {
        self.phase = Phase::SignedIn;
        self.session = Some(session);
        self.error = None;
        self.connecting();
    }

    /// A scheduled refresh brought a new access token; the open connection stays.
    pub fn refreshed(&mut self, session: Session) {
        self.session = Some(session);
        self.error = None;
    }

    /// A scheduled refresh could not reach WorkOS; the session stays and the store retries.
    pub fn refresh_failed(&mut self, error: &SignInError) {
        self.error = Some(format!("{error} The app tries again shortly."));
    }

    /// Signing in or resuming failed; the app is signed out.
    pub fn failed(&mut self, error: &SignInError) {
        self.signed_out();
        self.error = Some(error.to_string());
    }

    /// Signed out (or nothing saved): device identity from now on.
    pub fn signed_out(&mut self) {
        if self.phase != Phase::Off {
            self.phase = Phase::SignedOut;
        }
        self.session = None;
        self.error = None;
        self.refusal = None;
    }

    /// A new connection is starting: forget how the last one went.
    pub fn connecting(&mut self) {
        self.seen_connecting = false;
        self.reached = false;
        self.refusal = None;
    }

    /// The connection's status changed. While signed in, a connection that fails or closes
    /// before it ever opened was refused by the module (the host gives no reason, ADR 0002
    /// AUTH1 section), so the screen names the issuer and audience to register, as the web does
    /// (`flowtrace/src/lib/ledger/spacetime-backend.ts:107-109`).
    pub fn on_status(&mut self, status: &ConnectionStatus) {
        match status {
            ConnectionStatus::Connecting => self.seen_connecting = true,
            ConnectionStatus::Connected { .. } => {
                self.reached = true;
                self.refusal = None;
            }
            ConnectionStatus::Error { .. } | ConnectionStatus::Disconnected => {
                let failed_early =
                    matches!(status, ConnectionStatus::Error { .. }) || self.seen_connecting;
                if self.phase == Phase::SignedIn
                    && !self.reached
                    && failed_early
                    && let Some(session) = &self.session
                {
                    self.refusal = Some(session.claims.refusal_message());
                }
            }
        }
    }
}

/// The button the sign-in section offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// "Sign in with WorkOS".
    SignIn,
    /// "Sign out".
    SignOut,
}

/// The Connect screen's sign-in section, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// One sentence on where the sign-in stands.
    pub status: String,
    /// Labelled facts while signed in: name, email, WorkOS user, identity, issuer, audience.
    pub details: Vec<(&'static str, String)>,
    /// The button, if any.
    pub action: Option<Action>,
    /// The last failure.
    pub error: Option<String>,
    /// The module's refusal, with the issuer and audience to register.
    pub refusal: Option<String>,
}

impl Card {
    /// The section for `state`.
    #[must_use]
    pub fn of(state: &SignInState) -> Self {
        let (status, action) = match state.phase() {
            Phase::Off => (
                "Sign-in with WorkOS is off: set workos_client_id in config.toml to turn it on. \
                 Until then the app connects as this device.",
                None,
            ),
            Phase::SignedOut => (
                "Not signed in: the app connects as this device. Sign in to open the books your \
                 WorkOS account is a member of.",
                Some(Action::SignIn),
            ),
            Phase::Resuming => ("Signing in with the saved WorkOS session.", None),
            Phase::Waiting => (
                "Finish signing in in your browser; the app connects once WorkOS sends you back.",
                Some(Action::SignIn),
            ),
            Phase::SignedIn => (
                "Signed in with WorkOS. The identity follows the account, so it is the same on \
                 every device and on the web.",
                Some(Action::SignOut),
            ),
        };
        let details = state
            .session()
            .map(|s| {
                vec![
                    ("Name", s.user.name.clone()),
                    ("Email", s.user.email.clone()),
                    ("WorkOS user", s.user.id.clone()),
                    ("Identity", s.claims.identity()),
                    ("Issuer", s.claims.issuer.clone()),
                    (
                        "Audience",
                        s.claims
                            .audience
                            .clone()
                            .unwrap_or_else(|| "(none)".to_owned()),
                    ),
                ]
            })
            .unwrap_or_default();
        Self {
            status: status.to_owned(),
            details,
            action,
            error: state.error().map(str::to_owned),
            refusal: state.refusal().map(str::to_owned),
        }
    }
}
