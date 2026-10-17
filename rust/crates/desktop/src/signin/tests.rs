#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ledger_client::TokenFile;
use ledger_module::auth::{
    Admission, TokenClaims as ModuleClaims, check_connection, member_identity,
};
use ledger_module::tables::TrustedIssuer;
use spacetimedb::{Identity, Timestamp};

use super::flow;
use super::loopback::{Callback, Loopback, parse_request_line};
use super::pkce::Pkce;
use super::workos::{Workos, authorize_url};
use super::*;

const CLIENT: &str = "client_01TESTAUTH2";

fn issuer() -> String {
    format!("https://api.workos.com/user_management/{CLIENT}")
}

/// An unsigned JWT with `claims` as its payload (the app never checks signatures).
fn jwt(claims: &serde_json::Value) -> String {
    let part = |v: &serde_json::Value| URL_SAFE_NO_PAD.encode(v.to_string());
    format!(
        "{}.{}.c2ln",
        part(&serde_json::json!({"alg": "RS256", "typ": "JWT"})),
        part(claims)
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tmp"))
        .join(format!("desktop-signin-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

// Settings

#[test]
fn settings_default_to_the_workos_api_and_the_client_issuer() {
    let s = WorkosSettings::new(&format!(" {CLIENT} "), None, None);
    assert_eq!(s.client_id, CLIENT);
    assert_eq!(s.issuer, issuer());
    assert_eq!(s.api, "https://api.workos.com");
    let local = WorkosSettings::new(
        CLIENT,
        Some("http://127.0.0.1:4000/trusted"),
        Some("http://127.0.0.1:4000/trusted/"),
    );
    assert_eq!(local.issuer, "http://127.0.0.1:4000/trusted");
    assert_eq!(local.api, "http://127.0.0.1:4000/trusted");
}

#[test]
fn the_refresh_token_sits_beside_the_device_token() {
    assert_eq!(
        refresh_file("/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com").path(),
        std::path::Path::new(
            "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com.workos"
        )
    );
}

// PKCE

#[test]
fn the_challenge_is_the_rfc_7636_appendix_b_value() {
    let pkce = Pkce::from_parts("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", "s");
    assert_eq!(
        pkce.challenge(),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn a_new_pair_is_random_url_safe_and_never_printed() {
    let a = Pkce::new().unwrap();
    let b = Pkce::new().unwrap();
    assert_eq!(a.verifier().len(), 43);
    assert_eq!(a.state().len(), 22);
    assert_ne!(a.verifier(), b.verifier());
    assert_ne!(a.state(), b.state());
    assert!(
        a.verifier()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
    assert_eq!(format!("{a:?}"), "Pkce(..)");
}

// Authorize URL

#[test]
fn the_authorize_url_asks_authkit_for_a_code_with_pkce() {
    let settings = WorkosSettings::new(CLIENT, None, None);
    let pkce = Pkce::from_parts("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", "st-1");
    let url = authorize_url(&settings, "http://127.0.0.1:43123/callback", &pkce);
    let parsed = url::Url::parse(&url).unwrap();
    assert_eq!(parsed.host_str(), Some("api.workos.com"));
    assert_eq!(parsed.path(), "/user_management/authorize");
    let q: Vec<(String, String)> = parsed.query_pairs().into_owned().collect();
    let get = |k: &str| q.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    assert_eq!(get("response_type"), Some("code"));
    assert_eq!(get("provider"), Some("authkit"));
    assert_eq!(get("client_id"), Some(CLIENT));
    assert_eq!(get("redirect_uri"), Some("http://127.0.0.1:43123/callback"));
    assert_eq!(get("state"), Some("st-1"));
    assert_eq!(
        get("code_challenge"),
        Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM")
    );
    assert_eq!(get("code_challenge_method"), Some("S256"));
    assert!(
        !url.contains("dBjftJeZ4CVP"),
        "the verifier never leaves the app"
    );
}

// Loopback

#[test]
fn the_callback_line_gives_the_code_only_for_this_state() {
    assert_eq!(
        parse_request_line("GET /callback?code=01ABC&state=st-1 HTTP/1.1", "st-1"),
        Callback::Code("01ABC".to_owned())
    );
    assert!(matches!(
        parse_request_line("GET /callback?code=01ABC&state=other HTTP/1.1", "st-1"),
        Callback::Failed(_)
    ));
    assert_eq!(
        parse_request_line(
            "GET /callback?error=access_denied&error_description=User%20cancelled&state=st-1 HTTP/1.1",
            "st-1"
        ),
        Callback::Failed("User cancelled".to_owned())
    );
    assert_eq!(
        parse_request_line("GET /favicon.ico HTTP/1.1", "st-1"),
        Callback::Other
    );
    assert_eq!(
        parse_request_line("POST /callback?code=1&state=st-1 HTTP/1.1", "st-1"),
        Callback::Other
    );
}

fn browser_get(port: u16, target: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(stream, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    answer
}

#[test]
fn the_listener_takes_one_redirect_on_127_0_0_1_and_closes() {
    let loopback = Loopback::bind().unwrap();
    let port = loopback.port();
    assert_eq!(
        loopback.redirect_uri(),
        format!("http://127.0.0.1:{port}/callback")
    );
    let browser = std::thread::spawn(move || {
        let other = browser_get(port, "/favicon.ico");
        let done = browser_get(port, "/callback?code=01CODE&state=st-9");
        (other, done)
    });
    let code = loopback.wait("st-9", Duration::from_secs(10)).unwrap();
    assert_eq!(code, "01CODE");
    let (other, done) = browser.join().unwrap();
    assert!(other.starts_with("HTTP/1.1 404"), "{other}");
    assert!(done.starts_with("HTTP/1.1 200"), "{done}");
    assert!(done.contains("You can close this tab"));
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "nothing listens once the redirect is in"
    );
}

#[test]
fn the_listener_gives_up_after_its_timeout() {
    let loopback = Loopback::bind().unwrap();
    assert_eq!(
        loopback.wait("st", Duration::from_millis(100)),
        Err(SignInError::TimedOut)
    );
}

// Claims, refusal and identity

#[test]
fn claims_take_the_audience_from_aud_else_client_id() {
    let with_client_id = jwt(&serde_json::json!({
        "iss": issuer(), "sub": "user_01ALICE", "client_id": CLIENT, "exp": 1_800_000_000u64
    }));
    let claims = TokenClaims::of(&with_client_id).unwrap();
    assert_eq!(claims.issuer, issuer());
    assert_eq!(claims.subject, "user_01ALICE");
    assert_eq!(claims.audience.as_deref(), Some(CLIENT));
    assert_eq!(claims.expires_at, Some(1_800_000_000));

    let listed = jwt(&serde_json::json!({
        "iss": "https://auth.example.org", "sub": "u", "aud": ["flowtrace", "other"], "client_id": CLIENT
    }));
    assert_eq!(
        TokenClaims::of(&listed).unwrap().audience.as_deref(),
        Some("flowtrace")
    );
    assert_eq!(TokenClaims::of("not-a-jwt"), None);
}

#[test]
fn the_refusal_message_is_the_webs() {
    // flowtrace/src/lib/ledger/spacetime-auth.ts, refusalMessage, workos mode.
    let claims = TokenClaims {
        issuer: issuer(),
        subject: "user_01ALICE".to_owned(),
        audience: Some(CLIENT.to_owned()),
        expires_at: None,
    };
    assert_eq!(
        claims.refusal_message(),
        format!(
            "SpacetimeDB refused this sign-in: an operator must trust issuer {} for audience {CLIENT} on this database",
            issuer()
        )
    );
    let none = TokenClaims {
        audience: None,
        ..claims
    };
    assert_eq!(none.refusal_message(), "SpacetimeDB refused the connection");
}

fn trusted_row(issuer: &str, audience: &str) -> TrustedIssuer {
    TrustedIssuer {
        id: 1,
        issuer: issuer.to_owned(),
        audience: audience.to_owned(),
        label: "WorkOS staging".to_owned(),
        added_by: Identity::ZERO,
        added_at: Timestamp::UNIX_EPOCH,
    }
}

/// Deliverable 3: the identity the desktop derives for a WorkOS subject is the one the module's
/// `add_member_by_subject` gives the member row, so the two meet.
#[test]
fn the_desktop_identity_of_a_subject_is_the_modules_member_identity() {
    let rows = [trusted_row(&issuer(), CLIENT)];
    for subject in [
        "user_01ALICE",
        "user_01HZX9Q3R5",
        "aad0459e-6590-4f63-9608-2a578b08115f",
    ] {
        let token = jwt(&serde_json::json!({"iss": issuer(), "sub": subject, "client_id": CLIENT}));
        let desktop = TokenClaims::of(&token).unwrap().identity();
        let module = member_identity(&rows, &issuer(), subject).unwrap();
        assert_eq!(desktop, module.to_hex().to_string(), "{subject}");
        assert!(desktop.starts_with("c200"));
    }
    // The pair AUTH1 pinned against the local host (ledger-module auth.rs).
    let pinned = TokenClaims {
        issuer: "localhost".to_owned(),
        subject: "aad0459e-6590-4f63-9608-2a578b08115f".to_owned(),
        audience: None,
        expires_at: None,
    };
    assert_eq!(
        pinned.identity(),
        "c200f4a9e85b11d24644bd91cccc13e60094aedaabc166cd599932994c53ba96"
    );
}

/// The issuer and audience the desktop names in a refusal are the ones a trusted row needs:
/// the module admits the same claims once that row exists.
#[test]
fn the_refusal_names_what_lets_the_module_admit_the_token() {
    let claims = TokenClaims::of(&jwt(
        &serde_json::json!({"iss": issuer(), "sub": "user_01ALICE", "client_id": CLIENT}),
    ))
    .unwrap();
    let module = ModuleClaims {
        issuer: claims.issuer.clone(),
        subject: claims.subject.clone(),
        audience: Vec::new(),
        client_id: Some(CLIENT.to_owned()),
    };
    assert!(check_connection(false, false, Some(&module), true, &[]).is_err());
    let row = trusted_row(&claims.issuer, claims.audience.as_deref().unwrap());
    assert_eq!(
        check_connection(false, false, Some(&module), true, &[row]),
        Ok(Admission::Trusted)
    );
}

// Refresh schedule

#[test]
fn refresh_runs_before_expiry() {
    let now = 1_000;
    // A WorkOS five-minute token: four minutes in.
    assert_eq!(
        refresh_delay(Some(now + 300), now),
        Duration::from_secs(240)
    );
    // Two minutes: a fifth of it early.
    assert_eq!(refresh_delay(Some(now + 120), now), Duration::from_secs(96));
    // An hour: a minute early.
    assert_eq!(
        refresh_delay(Some(now + 3600), now),
        Duration::from_mins(59)
    );
    // Expired, nearly expired or undated: soon.
    assert_eq!(refresh_delay(Some(now - 10), now), Duration::from_secs(5));
    assert_eq!(refresh_delay(Some(now + 3), now), Duration::from_secs(5));
    assert_eq!(refresh_delay(None, now), Duration::from_secs(5));
}

// The WorkOS calls, against a stand-in on 127.0.0.1

/// A stand-in for `POST /user_management/authenticate`: answers each request with the next of
/// `answers` (status, JSON) and sends each request body on the returned channel.
fn fake_workos(
    answers: Vec<(u16, serde_json::Value)>,
) -> (WorkosSettings, mpsc::Receiver<(String, serde_json::Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, body) in answers {
            let (mut stream, _) = listener.accept().unwrap();
            let mut data = Vec::new();
            let mut buf = [0u8; 4096];
            let (head, body_in) = loop {
                let n = stream.read(&mut buf).unwrap();
                data.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&data).into_owned();
                if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);
                    if rest.len() >= length {
                        break (head.to_owned(), rest[..length].to_owned());
                    }
                }
            };
            let path = head
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            tx.send((path, serde_json::from_str(&body_in).unwrap()))
                .unwrap();
            let text = body.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .unwrap();
        }
    });
    let api = format!("http://127.0.0.1:{port}");
    (WorkosSettings::new(CLIENT, Some(&issuer()), Some(&api)), rx)
}

fn answer(subject: &str, refresh: &str) -> serde_json::Value {
    serde_json::json!({
        "user": {
            "object": "user", "id": subject, "email": "alice@example.org",
            "first_name": "Alice", "last_name": "Achieng", "email_verified": true
        },
        "organization_id": "org_01TEST",
        "access_token": jwt(&serde_json::json!({
            "iss": issuer(), "sub": subject, "client_id": CLIENT, "exp": 1_900_000_000u64
        })),
        "refresh_token": refresh,
        "authentication_method": "Password"
    })
}

#[test]
fn a_browser_sign_in_exchanges_the_code_and_keeps_only_the_refresh_token() {
    let (settings, requests) = fake_workos(vec![(200, answer("user_01ALICE", "refresh-1"))]);
    let workos = Workos::new(settings.clone());
    let pending = flow::begin(&settings).unwrap();
    let url = url::Url::parse(&pending.url).unwrap();
    let q: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
    let redirect = url::Url::parse(&q["redirect_uri"]).unwrap();
    assert_eq!(redirect.host_str(), Some("127.0.0.1"));
    let port = redirect.port().unwrap();
    let state = q["state"].clone();
    let challenge = q["code_challenge"].clone();
    let browser = std::thread::spawn(move || {
        browser_get(port, &format!("/callback?code=01CODE&state={state}"))
    });

    let dir = scratch("signin");
    let file = TokenFile::new(dir.join("token-127.0.0.1_3100.workos"));
    let session = pending
        .finish(&workos, &file, Duration::from_secs(10))
        .unwrap();
    assert!(browser.join().unwrap().starts_with("HTTP/1.1 200"));

    let (path, body) = requests.recv().unwrap();
    assert_eq!(path, "/user_management/authenticate");
    assert_eq!(body["grant_type"], "authorization_code");
    assert_eq!(body["client_id"], CLIENT);
    assert_eq!(body["code"], "01CODE");
    assert!(
        body.get("client_secret").is_none(),
        "a public client sends no secret"
    );
    let verifier = body["code_verifier"].as_str().unwrap();
    assert_eq!(Pkce::from_parts(verifier, "").challenge(), challenge);

    assert_eq!(session.user.name, "Alice Achieng");
    assert_eq!(session.user.email, "alice@example.org");
    assert_eq!(session.user.id, "user_01ALICE");
    assert_eq!(session.claims.subject, "user_01ALICE");
    assert_eq!(session.organization_id.as_deref(), Some("org_01TEST"));
    assert_eq!(file.load().unwrap().as_deref(), Some("refresh-1"));
    let saved = std::fs::read_to_string(file.path()).unwrap();
    assert!(
        !saved.contains(session.access_token.expose()),
        "the access token is never stored"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(file.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    assert!(!format!("{session:?}").contains(session.access_token.expose()));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn resume_refreshes_and_keeps_the_rotated_refresh_token() {
    let (settings, requests) = fake_workos(vec![
        (200, answer("user_01ALICE", "refresh-2")),
        (200, answer("user_01ALICE", "refresh-3")),
    ]);
    let workos = Workos::new(settings);
    let dir = scratch("resume");
    let file = TokenFile::new(dir.join("token.workos"));
    assert_eq!(
        flow::resume(&workos, &file).unwrap(),
        None,
        "no saved sign-in"
    );
    file.save("refresh-1").unwrap();

    let first = flow::resume(&workos, &file).unwrap().unwrap();
    let (_, body) = requests.recv().unwrap();
    assert_eq!(body["grant_type"], "refresh_token");
    assert_eq!(body["refresh_token"], "refresh-1");
    assert_eq!(body["client_id"], CLIENT);
    assert!(body.get("client_secret").is_none());
    assert_eq!(file.load().unwrap().as_deref(), Some("refresh-2"));
    assert_eq!(
        first.claims.identity(),
        TokenClaims::of(first.access_token.expose())
            .unwrap()
            .identity()
    );

    let second = flow::resume(&workos, &file).unwrap().unwrap();
    assert_eq!(requests.recv().unwrap().1["refresh_token"], "refresh-2");
    assert_eq!(file.load().unwrap().as_deref(), Some("refresh-3"));
    assert_eq!(
        first.claims.identity(),
        second.claims.identity(),
        "same person, same identity"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_refused_refresh_token_is_forgotten() {
    let (settings, _requests) = fake_workos(vec![(
        400,
        serde_json::json!({"error": "invalid_grant", "error_description": "Session has already ended."}),
    )]);
    let workos = Workos::new(settings);
    let dir = scratch("refused");
    let file = TokenFile::new(dir.join("token.workos"));
    file.save("refresh-old").unwrap();
    assert_eq!(
        flow::resume(&workos, &file),
        Err(SignInError::Refused(
            "Session has already ended.".to_owned()
        ))
    );
    assert_eq!(file.load().unwrap(), None);
    flow::sign_out(&file).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_unreachable_workos_keeps_the_saved_sign_in() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let api = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    drop(listener);
    let workos = Workos::new(WorkosSettings::new(CLIENT, None, Some(&api)));
    let dir = scratch("offline");
    let file = TokenFile::new(dir.join("token.workos"));
    file.save("refresh-1").unwrap();
    assert!(matches!(
        flow::resume(&workos, &file),
        Err(SignInError::Network(_))
    ));
    assert_eq!(file.load().unwrap().as_deref(), Some("refresh-1"));
    flow::sign_out(&file).unwrap();
    assert_eq!(file.load().unwrap(), None, "sign-out removes it");
    std::fs::remove_dir_all(dir).unwrap();
}

// The Connect screen's state

fn session_for(subject: &str) -> Session {
    let token = jwt(
        &serde_json::json!({"iss": issuer(), "sub": subject, "client_id": CLIENT, "exp": 1_900_000_000u64}),
    );
    Session {
        access_token: ledger_client::AccessToken::new(token.clone()),
        claims: TokenClaims::of(&token).unwrap(),
        user: SignedInUser {
            id: subject.to_owned(),
            name: "Alice Achieng".to_owned(),
            email: "alice@example.org".to_owned(),
        },
        organization_id: None,
    }
}

#[test]
fn a_signed_in_connection_closed_before_it_opened_shows_the_refusal() {
    use crate::backend::ConnectionStatus as S;
    use state::{Phase, SignInState};
    let mut state = SignInState::new(true);
    assert_eq!(state.phase(), Phase::SignedOut);
    state.waiting();
    assert_eq!(state.phase(), Phase::Waiting);
    state.signed_in(session_for("user_01ALICE"));
    state.on_status(&S::Disconnected);
    assert_eq!(
        state.refusal(),
        None,
        "the backend's state before it starts connecting"
    );
    state.on_status(&S::Connecting);
    state.on_status(&S::Error {
        message: "connection closed".to_owned(),
    });
    assert_eq!(
        state.refusal(),
        Some(
            format!(
                "SpacetimeDB refused this sign-in: an operator must trust issuer {} for audience {CLIENT} on this database",
                issuer()
            )
            .as_str()
        )
    );

    // Once an operator trusts the issuer, the next connection opens and the message goes.
    state.connecting();
    state.on_status(&S::Connecting);
    state.on_status(&S::Connected {
        identity: session_for("user_01ALICE").claims.identity(),
    });
    assert_eq!(state.refusal(), None);
    state.on_status(&S::Disconnected);
    assert_eq!(
        state.refusal(),
        None,
        "a drop after opening is not a refusal"
    );
}

#[test]
fn a_device_connection_is_never_called_a_refused_sign_in() {
    use crate::backend::ConnectionStatus as S;
    use state::{Phase, SignInState};
    let mut off = SignInState::new(false);
    off.on_status(&S::Connecting);
    off.on_status(&S::Error {
        message: "unreachable".to_owned(),
    });
    assert_eq!(off.refusal(), None);
    assert_eq!(off.phase(), Phase::Off);
    off.signed_out();
    assert_eq!(off.phase(), Phase::Off, "sign-out keeps sign-in off");
}

#[test]
fn failures_sign_out_but_a_failed_refresh_keeps_the_session() {
    use state::{Phase, SignInState};
    let mut state = SignInState::new(true);
    state.resuming();
    state.failed(&SignInError::Refused(
        "Session has already ended.".to_owned(),
    ));
    assert_eq!(state.phase(), Phase::SignedOut);
    assert_eq!(
        state.error(),
        Some("WorkOS refused the sign-in: Session has already ended.")
    );

    state.signed_in(session_for("user_01ALICE"));
    assert_eq!(state.error(), None);
    state.refresh_failed(&SignInError::Network("timed out".to_owned()));
    assert_eq!(state.phase(), Phase::SignedIn);
    assert!(state.session().is_some());
    assert!(
        state
            .error()
            .unwrap()
            .starts_with("WorkOS could not be reached: timed out")
    );
    state.refreshed(session_for("user_01ALICE"));
    assert_eq!(state.error(), None);
    state.signed_out();
    assert_eq!(state.session(), None);
}

#[test]
fn the_card_names_the_person_and_what_to_register() {
    use state::{Action, Card, SignInState};
    let mut state = SignInState::new(true);
    let out = Card::of(&state);
    assert_eq!(out.action, Some(Action::SignIn));
    assert_eq!(out.details.len(), 0);

    let session = session_for("user_01ALICE");
    let identity = session.claims.identity();
    state.signed_in(session);
    let card = Card::of(&state);
    assert_eq!(card.action, Some(Action::SignOut));
    assert_eq!(
        card.details,
        vec![
            ("Name", "Alice Achieng".to_owned()),
            ("Email", "alice@example.org".to_owned()),
            ("WorkOS user", "user_01ALICE".to_owned()),
            ("Identity", identity),
            ("Issuer", issuer()),
            ("Audience", CLIENT.to_owned()),
        ]
    );
    assert_eq!(Card::of(&SignInState::new(false)).action, None);
}
