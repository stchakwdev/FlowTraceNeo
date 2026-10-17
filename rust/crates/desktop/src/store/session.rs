//! The WorkOS sign-in on the store (`crate::signin`): opening the browser, waiting for it, the
//! saved session at launch and on reconnects, the refresh before the access token expires, and
//! sign-out. Every blocking step (the loopback wait, the calls to WorkOS, the token file) runs on
//! a thread of its own through [`off_thread`] and comes back in a `cx.spawn` task, so the UI
//! thread never waits (ADR 0002, decision 4). A generation number drops the answer of a step
//! that a later sign-in, sign-out or connect has replaced.

use std::future::Future;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui_kit::{Context, Task};
use ledger_client::TokenFile;

use super::LedgerStore;
use crate::backend::{BackendError, ConnectSettings};
use crate::config;
use crate::signin::flow::{self, Pending};
use crate::signin::loopback::SIGN_IN_TIMEOUT;
use crate::signin::state::Phase;
use crate::signin::workos::Workos;
use crate::signin::{Session, SignInError, WorkosSettings, refresh_delay, refresh_file};

/// How soon a refresh that could not reach WorkOS is tried again.
const REFRESH_RETRY: Duration = Duration::from_secs(30);

/// How long an access token must still be valid to be presented again without a refresh.
const REUSE_MARGIN: u64 = 30;

/// What the store needs to sign in: the `workos_*` keys of `config.toml` and the config dir,
/// where default token files live (`config::with_token`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInSetup {
    /// The WorkOS environment.
    pub workos: WorkosSettings,
    /// The config dir.
    pub dir: PathBuf,
}

impl SignInSetup {
    /// `settings` with their token file filled in.
    #[must_use]
    pub fn resolve(&self, settings: ConnectSettings) -> ConnectSettings {
        config::with_token(settings, &self.dir)
    }

    /// The sign-in token file for `settings`' server ([`refresh_file`]).
    #[must_use]
    pub fn refresh_file(&self, settings: &ConnectSettings) -> TokenFile {
        refresh_file(&self.resolve(settings.clone()).token_path)
    }

    /// Whether a sign-in is saved for `settings`' server.
    #[must_use]
    pub fn has_saved(&self, settings: &ConnectSettings) -> bool {
        matches!(self.refresh_file(settings).load(), Ok(Some(_)))
    }
}

/// The store's sign-in bookkeeping.
pub(super) struct Sessions {
    setup: Option<SignInSetup>,
    /// The server of the last connect, for sign-in and sign-out.
    settings: Option<ConnectSettings>,
    generation: u64,
    step: Option<Task<()>>,
    refresh: Option<Task<()>>,
}

impl Sessions {
    pub(super) fn new(setup: Option<SignInSetup>, settings: Option<ConnectSettings>) -> Self {
        Self {
            setup,
            settings,
            generation: 0,
            step: None,
            refresh: None,
        }
    }

    /// A new step replaces every earlier one.
    fn next(&mut self) -> u64 {
        self.generation += 1;
        self.step = None;
        self.refresh = None;
        self.generation
    }
}

/// Runs `work` on a thread of its own; the future resolves with its result, or `None` when the
/// thread could not start or ended without one.
fn off_thread<T: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> impl Future<Output = Option<T>> {
    let (tx, rx) = async_channel::bounded(1);
    let started = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            // The store may be gone; then nobody needs the answer.
            let _ = tx.send_blocking(work());
        })
        .is_ok();
    async move { if started { rx.recv().await.ok() } else { None } }
}

/// Seconds since 1970 now.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn lost() -> SignInError {
    SignInError::Local("the sign-in thread ended without an answer.".to_owned())
}

impl LedgerStore {
    /// Whether the Connect screen can offer the sign-in.
    #[must_use]
    pub fn can_sign_in(&self) -> bool {
        self.session.setup.is_some()
    }

    /// See [`LedgerStore::connect`].
    pub(super) fn connect_signed_in_or_device(
        &mut self,
        settings: ConnectSettings,
        cx: &mut Context<Self>,
    ) -> Result<(), BackendError> {
        let Some(setup) = self.session.setup.clone() else {
            self.session.settings = Some(settings.clone());
            return self.open_backend(settings, cx);
        };
        let settings = setup.resolve(settings);
        // An access token is good on any database; the session belongs to the token file's
        // server.
        let same_server = self
            .session
            .settings
            .as_ref()
            .is_some_and(|s| s.token_path == settings.token_path);
        self.session.settings = Some(settings.clone());
        let current = self
            .core
            .signin()
            .session()
            .filter(|_| same_server)
            .filter(|s| {
                s.claims
                    .expires_at
                    .is_some_and(|exp| exp > now() + REUSE_MARGIN)
            })
            .cloned();
        if let Some(session) = current {
            self.session.next();
            return self.open_session(session, settings, cx);
        }
        if setup.has_saved(&settings) {
            self.resume(settings, cx);
            return Ok(());
        }
        self.session.next();
        self.core.signin_mut().signed_out();
        self.open_backend(settings, cx)
    }

    /// Trades the saved refresh token for a session, then connects with it; with nothing saved
    /// (or a token WorkOS refuses) it connects with the device identity, and when WorkOS cannot
    /// be reached it stays offline with the reason on the Connect screen.
    pub(super) fn resume(&mut self, settings: ConnectSettings, cx: &mut Context<Self>) {
        let Some(setup) = self.session.setup.clone() else {
            return;
        };
        let generation = self.session.next();
        self.core.signin_mut().resuming();
        self.core.note_signin();
        cx.notify();
        let file = setup.refresh_file(&settings);
        let workos = setup.workos.clone();
        let answer = off_thread("desktop-signin-resume", move || {
            flow::resume(&Workos::new(workos), &file)
        });
        self.session.step = Some(cx.spawn(async move |this, cx| {
            let answer = answer.await.unwrap_or_else(|| Err(lost()));
            // The store is gone when this fails; nothing is left to update.
            let _ = this.update(cx, |store, cx| {
                if store.session.generation != generation {
                    return;
                }
                match answer {
                    Ok(Some(session)) => {
                        // A refused start shows on the Connect screen through the status.
                        let _ = store.open_session(session, settings, cx);
                    }
                    Ok(None) => {
                        store.core.signin_mut().signed_out();
                        let _ = store.open_backend(settings, cx);
                    }
                    Err(error @ SignInError::Refused(_)) => {
                        store.core.signin_mut().failed(&error);
                        let _ = store.open_backend(settings, cx);
                    }
                    Err(error) => store.core.signin_mut().failed(&error),
                }
                store.core.note_signin();
                cx.notify();
            });
        }));
    }

    /// Starts a sign-in on the server of `settings` (or of the last connect): opens the system
    /// browser at the AuthKit page and waits for it off the UI thread. Once WorkOS has answered
    /// the app connects as the signed-in person.
    pub fn sign_in(&mut self, settings: Option<ConnectSettings>, cx: &mut Context<Self>) {
        let Some(setup) = self.session.setup.clone() else {
            return;
        };
        let Some(settings) = settings
            .map(|s| setup.resolve(s))
            .or_else(|| self.session.settings.clone())
        else {
            self.core.signin_mut().failed(&SignInError::Local(
                "name the server first, then sign in.".to_owned(),
            ));
            cx.notify();
            return;
        };
        let generation = self.session.next();
        let pending: Pending = match flow::begin(&setup.workos) {
            Ok(pending) => pending,
            Err(error) => {
                self.core.signin_mut().failed(&error);
                cx.notify();
                return;
            }
        };
        // The URL holds the PKCE challenge and the state, neither of them a secret.
        cx.open_url(&pending.url);
        self.core.signin_mut().waiting();
        self.core.note_signin();
        self.session.settings = Some(settings.clone());
        cx.notify();
        let file = setup.refresh_file(&settings);
        let workos = setup.workos.clone();
        let answer = off_thread("desktop-signin-browser", move || {
            pending.finish(&Workos::new(workos), &file, SIGN_IN_TIMEOUT)
        });
        self.session.step = Some(cx.spawn(async move |this, cx| {
            let answer = answer.await.unwrap_or_else(|| Err(lost()));
            let _ = this.update(cx, |store, cx| {
                if store.session.generation != generation {
                    return;
                }
                match answer {
                    Ok(session) => {
                        let _ = store.open_session(session, settings, cx);
                    }
                    Err(error) => store.core.signin_mut().failed(&error),
                }
                store.core.note_signin();
                cx.notify();
            });
        }));
    }

    /// Signs out: forgets the session and the saved refresh token, then reconnects to the same
    /// server with the device identity.
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        let Some(setup) = self.session.setup.clone() else {
            return;
        };
        self.session.next();
        let settings = self.session.settings.clone();
        let cleared = settings
            .as_ref()
            .map_or(Ok(()), |s| flow::sign_out(&setup.refresh_file(s)));
        self.core.signin_mut().signed_out();
        if let Err(error) = cleared {
            self.core.signin_mut().failed(&error);
        }
        if let Some(settings) = settings {
            let device = ConnectSettings {
                access_token: None,
                ..settings
            };
            let _ = self.open_backend(device, cx);
        }
        self.core.note_signin();
        cx.notify();
    }

    /// Connects with `session`'s access token and schedules its refresh.
    fn open_session(
        &mut self,
        session: Session,
        settings: ConnectSettings,
        cx: &mut Context<Self>,
    ) -> Result<(), BackendError> {
        let expires_at = session.claims.expires_at;
        let signed_in = ConnectSettings {
            access_token: Some(session.access_token.clone()),
            ..settings
        };
        self.core.signin_mut().signed_in(session);
        self.schedule_refresh(refresh_delay(expires_at, now()), cx);
        self.open_backend(signed_in, cx)
    }

    /// Refreshes the session after `delay`: the new access token is kept for the next connect
    /// and the rotated refresh token saved; the open connection stays as it is.
    fn schedule_refresh(&mut self, delay: Duration, cx: &mut Context<Self>) {
        let (Some(setup), Some(settings)) =
            (self.session.setup.clone(), self.session.settings.clone())
        else {
            return;
        };
        let generation = self.session.generation;
        let file = setup.refresh_file(&settings);
        let workos = setup.workos;
        self.session.refresh = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let answer = off_thread("desktop-signin-refresh", move || {
                flow::resume(&Workos::new(workos), &file)
            })
            .await
            .unwrap_or_else(|| Err(lost()));
            let _ = this.update(cx, |store, cx| {
                if store.session.generation != generation
                    || store.core.signin().phase() != Phase::SignedIn
                {
                    return;
                }
                match answer {
                    Ok(Some(session)) => {
                        let next = refresh_delay(session.claims.expires_at, now());
                        store.core.signin_mut().refreshed(session);
                        store.schedule_refresh(next, cx);
                    }
                    Ok(None) => store.core.signin_mut().signed_out(),
                    Err(error @ SignInError::Refused(_)) => {
                        store.core.signin_mut().failed(&error);
                    }
                    Err(error) => {
                        store.core.signin_mut().refresh_failed(&error);
                        store.schedule_refresh(REFRESH_RETRY, cx);
                    }
                }
                store.core.note_signin();
                cx.notify();
            });
        }));
    }
}
