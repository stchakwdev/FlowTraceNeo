//! The sign-in steps as blocking functions over the pieces in this module; the store runs each
//! on a thread of its own (`store.rs`, `off_thread`), never on the UI thread.

use std::time::Duration;

use ledger_client::TokenFile;

use super::loopback::Loopback;
use super::pkce::Pkce;
use super::workos::{AuthAnswer, Workos, authorize_url};
use super::{Session, SignInError, WorkosSettings};

/// A sign-in under way: the listener is open and the browser should go to `url`.
#[derive(Debug)]
pub struct Pending {
    /// The authorize URL to open in the system browser.
    pub url: String,
    pkce: Pkce,
    loopback: Loopback,
}

/// Starts a sign-in: draws the PKCE pair, opens the loopback listener and builds the URL.
///
/// # Errors
///
/// [`SignInError::Local`] when the random source or the listener fails.
pub fn begin(settings: &WorkosSettings) -> Result<Pending, SignInError> {
    let pkce = Pkce::new()?;
    let loopback = Loopback::bind()?;
    let url = authorize_url(settings, &loopback.redirect_uri(), &pkce);
    Ok(Pending {
        url,
        pkce,
        loopback,
    })
}

impl Pending {
    /// Waits for the browser, exchanges the code and saves the refresh token in `file`.
    ///
    /// # Errors
    ///
    /// Any [`SignInError`]: the browser did not come back in `timeout`, came back with an
    /// error, WorkOS refused the code, or the file could not be written.
    pub fn finish(
        self,
        workos: &Workos,
        file: &TokenFile,
        timeout: Duration,
    ) -> Result<Session, SignInError> {
        let code = self.loopback.wait(self.pkce.state(), timeout)?;
        let answer = workos.exchange_code(&code, &self.pkce)?;
        keep(&answer, file)
    }
}

/// Saves the answer's refresh token and returns its session.
fn keep(answer: &AuthAnswer, file: &TokenFile) -> Result<Session, SignInError> {
    let session = answer.session()?;
    file.save(answer.refresh_token())
        .map_err(|e| SignInError::TokenFile(e.to_string()))?;
    Ok(session)
}

/// A new session from the refresh token in `file`, saving the refresh token WorkOS sends back
/// (it may rotate). `Ok(None)` when there is no saved sign-in. A refresh token WorkOS refuses is
/// removed, so the next start connects with the device identity until the person signs in again.
///
/// # Errors
///
/// [`SignInError::Refused`] (the file is then cleared), [`SignInError::Network`], or
/// [`SignInError::TokenFile`].
pub fn resume(workos: &Workos, file: &TokenFile) -> Result<Option<Session>, SignInError> {
    let Some(refresh_token) = file
        .load()
        .map_err(|e| SignInError::TokenFile(e.to_string()))?
    else {
        return Ok(None);
    };
    match workos.refresh(&refresh_token) {
        Ok(answer) => keep(&answer, file).map(Some),
        Err(refused @ SignInError::Refused(_)) => {
            // A refused token is useless; keeping it would fail the same way at every start.
            let _ = file.clear();
            Err(refused)
        }
        Err(other) => Err(other),
    }
}

/// Signs out: removes the saved refresh token.
///
/// # Errors
///
/// [`SignInError::TokenFile`] when the file cannot be removed.
pub fn sign_out(file: &TokenFile) -> Result<(), SignInError> {
    file.clear()
        .map_err(|e| SignInError::TokenFile(e.to_string()))
}
