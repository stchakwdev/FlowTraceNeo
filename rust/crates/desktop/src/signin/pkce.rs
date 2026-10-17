//! PKCE (RFC 7636): the code verifier the app keeps, the challenge the authorize URL carries,
//! and the `state` that ties the callback to this sign-in.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

use super::SignInError;

/// One sign-in's secrets. `Debug` shows neither the verifier nor the state.
#[derive(Clone, PartialEq, Eq)]
pub struct Pkce {
    verifier: String,
    state: String,
}

impl Pkce {
    /// A fresh verifier (32 random bytes, 43 base64url characters: RFC 7636 section 4.1 asks
    /// for 43 to 128 and section 7.1 for 32 octets of entropy) and a 16-byte state.
    ///
    /// # Errors
    ///
    /// [`SignInError::Local`] when the OS random source fails.
    pub fn new() -> Result<Self, SignInError> {
        let mut verifier = [0u8; 32];
        let mut state = [0u8; 16];
        getrandom::fill(&mut verifier)
            .and_then(|()| getrandom::fill(&mut state))
            .map_err(|e| SignInError::Local(e.to_string()))?;
        Ok(Self {
            verifier: URL_SAFE_NO_PAD.encode(verifier),
            state: URL_SAFE_NO_PAD.encode(state),
        })
    }

    /// A pair from known values, for tests.
    #[must_use]
    pub fn from_parts(verifier: &str, state: &str) -> Self {
        Self {
            verifier: verifier.to_owned(),
            state: state.to_owned(),
        }
    }

    /// The verifier, sent with the code exchange.
    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The state the callback must carry back.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// `S256` challenge: `BASE64URL(SHA256(ASCII(verifier)))` (RFC 7636 section 4.2).
    #[must_use]
    pub fn challenge(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()))
    }
}

impl fmt::Debug for Pkce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pkce(..)")
    }
}
