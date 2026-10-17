//! The loopback redirect of a native app (RFC 8252 section 7.3): a listener on `127.0.0.1` with
//! a port the OS picks, for the one browser redirect that ends a sign-in. It answers that one
//! request with a short page and closes; anything else in the meantime (a favicon request) gets
//! a 404. WorkOS allows `http://127.0.0.1` redirects with a wildcard port in sandbox
//! environments (<https://workos.com/docs/sso/redirect-uris>: "a wildcard can replace the port
//! number ... strictly limited to localhost and loopback IP addresses").

use std::io::{self, Read as _, Write as _};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use super::SignInError;

/// The callback path the redirect URI names.
pub const CALLBACK_PATH: &str = "/callback";

/// How long the app waits for the browser by default.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

/// A listener for one callback.
#[derive(Debug)]
pub struct Loopback {
    listener: TcpListener,
    port: u16,
}

/// The page the browser shows once the app has the answer.
const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>FlowTrace Books</title>\
<body style=\"font-family:sans-serif;margin:3rem\"><h1>Signed in</h1>\
<p>You can close this tab and go back to FlowTrace Books.</p>";

/// The page for a callback that did not sign in.
const FAILED_PAGE: &str = "<!doctype html><meta charset=utf-8><title>FlowTrace Books</title>\
<body style=\"font-family:sans-serif;margin:3rem\"><h1>Not signed in</h1>\
<p>Go back to FlowTrace Books; it says what happened.</p>";

/// What one request to the listener was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    /// The redirect, with its code.
    Code(String),
    /// The redirect, refused or for another sign-in.
    Failed(String),
    /// Some other request.
    Other,
}

/// Reads one request line `GET <target> HTTP/1.1` as a callback for `state`.
#[must_use]
pub fn parse_request_line(line: &str, state: &str) -> Callback {
    let mut parts = line.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Callback::Other;
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Callback::Other;
    }
    let mut code = None;
    let mut got_state = None;
    let mut error = None;
    let mut description = None;
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => got_state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            "error_description" => description = Some(value.into_owned()),
            _ => {}
        }
    }
    if let Some(error) = error {
        return Callback::Failed(description.unwrap_or(error));
    }
    if got_state.as_deref() != Some(state) {
        return Callback::Failed("the browser answered for another sign-in.".to_owned());
    }
    match code {
        Some(code) if !code.is_empty() => Callback::Code(code),
        _ => Callback::Failed("the browser came back without a code.".to_owned()),
    }
}

impl Loopback {
    /// Listens on `127.0.0.1` with a port the OS picks.
    ///
    /// # Errors
    ///
    /// [`SignInError::Local`] when no port can be had.
    pub fn bind() -> Result<Self, SignInError> {
        let local = |e: io::Error| SignInError::Local(e.to_string());
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(local)?;
        let port = listener.local_addr().map_err(local)?.port();
        listener.set_nonblocking(true).map_err(local)?;
        Ok(Self { listener, port })
    }

    /// The port the listener has.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// `http://127.0.0.1:<port>/callback`.
    #[must_use]
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}{CALLBACK_PATH}", self.port)
    }

    /// Waits up to `timeout` for the redirect carrying `state` and returns its code. The
    /// listener closes when this returns, whatever the outcome.
    ///
    /// # Errors
    ///
    /// [`SignInError::TimedOut`], [`SignInError::Callback`] for an error or another state, or
    /// [`SignInError::Local`] when the listener fails.
    pub fn wait(self, state: &str, timeout: Duration) -> Result<String, SignInError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => match answer(stream, state) {
                    Callback::Code(code) => return Ok(code),
                    Callback::Failed(why) => return Err(SignInError::Callback(why)),
                    Callback::Other => {}
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(SignInError::TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(SignInError::Local(e.to_string())),
            }
        }
    }
}

/// Reads one request from `stream`, answers it and says what it was. A request that cannot be
/// read is [`Callback::Other`].
fn answer(mut stream: TcpStream, state: &str) -> Callback {
    let head = read_head(&mut stream).unwrap_or_default();
    let line = head.lines().next().unwrap_or_default();
    let callback = parse_request_line(line, state);
    let (status, body) = match &callback {
        Callback::Code(_) => ("200 OK", DONE_PAGE),
        Callback::Failed(_) => ("400 Bad Request", FAILED_PAGE),
        Callback::Other => ("404 Not Found", ""),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The browser may already have gone; the code is in hand either way.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    callback
}

/// The request head, up to 16 KiB, read within 5 s.
fn read_head(stream: &mut TcpStream) -> io::Result<String> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut head = Vec::new();
    let mut buf = [0u8; 2048];
    while head.len() < 16 * 1024 && !head.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&buf[..n]);
    }
    Ok(String::from_utf8_lossy(&head).into_owned())
}
