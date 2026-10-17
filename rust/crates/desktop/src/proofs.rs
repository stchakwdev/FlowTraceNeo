//! Proof files (receipts on claims, proof on bills) through the proofs Worker
//! (`workers/proofs/README.md`): the file goes to R2 behind the Worker, which answers with the
//! object key and SHA-256; the app then records both on the books with the module's
//! `create_verification` reducer (pre-decision P13; the Worker never writes to the module).
//!
//! No GPUI here, so the requests are unit-tested against a local stub server. Calls block (ureq);
//! screens run them on GPUI's background executor, never on the UI thread.
//!
//! The Worker checks the caller from the bearer token: the app sends its SpacetimeDB token (the
//! token file of the connection), which the Worker verifies at the server's
//! `GET /v1/identity/:identity/verify` (<https://spacetimedb.com/docs/http/identity>).

use std::fmt;
use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use ledger_module::transparency::types::{
    NewVerification, VerificationStatus, VerificationType, Visibility,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The Worker's upload limit (`workers/proofs/README.md`, 25 MB).
pub const MAX_BYTES: u64 = 25 * 1024 * 1024;

/// How long one request may take before the app gives up.
const TIMEOUT: Duration = Duration::from_secs(120);

/// What a proof belongs to; the Worker's `link_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// An expense claim (`receipt_names` on the claim).
    Claim,
    /// A bill (`proof_ids` on the bill).
    Bill,
}

impl LinkKind {
    /// The Worker's and the module's word for it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Bill => "bill",
        }
    }

    /// The proof type recorded on the books: a receipt for a claim, an invoice for a bill
    /// (`VerificationType`, `flowtrace/src/lib/supabase.ts:95-107`).
    #[must_use]
    pub const fn proof_type(self) -> VerificationType {
        match self {
            Self::Claim => VerificationType::Receipt,
            Self::Bill => VerificationType::Invoice,
        }
    }
}

/// A file read from disk and checked before it is sent.
#[derive(Clone, PartialEq, Eq)]
pub struct ProofFile {
    /// The file name, shown on the claim or bill.
    pub name: String,
    /// What the bytes are, from their first bytes ([`sniff`]).
    pub content_type: &'static str,
    /// The contents.
    pub bytes: Vec<u8>,
}

impl fmt::Debug for ProofFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProofFile")
            .field("name", &self.name)
            .field("content_type", &self.content_type)
            .field("size", &self.bytes.len())
            .finish()
    }
}

/// What the Worker answers to an upload (`POST /upload`, `workers/proofs/README.md`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Uploaded {
    /// R2 object key, `org/<org>/<sha256>`.
    pub key: String,
    /// SHA-256 of the file, lowercase hex.
    pub sha256: String,
    /// Size in bytes.
    pub size: u64,
    /// The content type the Worker stored.
    pub content_type: String,
    /// R2 key of the small version, when the Worker made one.
    pub thumbnail_key: Option<String>,
}

/// A file attached on a form: its name and what the Worker stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// The file name.
    pub name: String,
    /// The Worker's answer.
    pub uploaded: Uploaded,
}

/// Why a proof file could not be sent or opened. Messages are for the screen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProofsError {
    /// The app has no Worker address or no token to send.
    #[error("{0}")]
    Unavailable(String),
    /// The file could not be read.
    #[error("{name} could not be read: {message}")]
    Read {
        /// The file name.
        name: String,
        /// What went wrong.
        message: String,
    },
    /// The file is empty.
    #[error("{0} is empty.")]
    Empty(String),
    /// The file is over [`MAX_BYTES`].
    #[error("{0} is larger than 25 MB.")]
    TooLarge(String),
    /// Not a photo or a PDF.
    #[error("{0} is not a photo or a PDF. Attach a JPEG, PNG, WebP, GIF, HEIC or PDF file.")]
    NotAllowed(String),
    /// The Worker refused; its own message.
    #[error("{message}")]
    Refused {
        /// HTTP status.
        status: u16,
        /// The Worker's `error` text, or the status when it sent none.
        message: String,
    },
    /// The Worker could not be reached.
    #[error("The proof store could not be reached: {0}")]
    Network(String),
    /// The Worker answered something the app cannot read.
    #[error("The proof store sent an answer the app cannot read: {0}")]
    BadReply(String),
    /// The Worker's hash is not the file's.
    #[error("The proof store's copy of {0} does not match the file. Try again.")]
    HashMismatch(String),
}

/// The content type of a photo or PDF from its first bytes, or `None` for anything else. The
/// Worker decides the same way and refuses what this refuses (`workers/proofs/README.md`).
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    let at = |offset: usize, magic: &[u8]| bytes.get(offset..offset + magic.len()) == Some(magic);
    if at(0, &[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if at(0, &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if at(0, b"GIF87a") || at(0, b"GIF89a") {
        Some("image/gif")
    } else if at(0, b"RIFF") && at(8, b"WEBP") {
        Some("image/webp")
    } else if at(0, b"%PDF-") {
        Some("application/pdf")
    } else if at(4, b"ftyp") {
        match bytes.get(8..12) {
            Some(b"heic" | b"heix" | b"hevc" | b"hevx") => Some("image/heic"),
            Some(b"mif1" | b"msf1" | b"heif") => Some("image/heif"),
            _ => None,
        }
    } else {
        None
    }
}

/// Checks `bytes` named `name` as the Worker will: not empty, at most [`MAX_BYTES`], a photo or
/// a PDF by its first bytes.
///
/// # Errors
///
/// [`ProofsError::Empty`], [`ProofsError::TooLarge`] or [`ProofsError::NotAllowed`].
pub fn check(name: &str, bytes: Vec<u8>) -> Result<ProofFile, ProofsError> {
    if bytes.is_empty() {
        return Err(ProofsError::Empty(name.to_owned()));
    }
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ProofsError::TooLarge(name.to_owned()));
    }
    let content_type = sniff(&bytes).ok_or_else(|| ProofsError::NotAllowed(name.to_owned()))?;
    Ok(ProofFile {
        name: name.to_owned(),
        content_type,
        bytes,
    })
}

/// Reads and [`check`]s the file at `path`; a file over the limit is refused before it is read.
///
/// # Errors
///
/// [`ProofsError::Read`] when the file cannot be read, else as [`check`].
pub fn read_file(path: &Path) -> Result<ProofFile, ProofsError> {
    let name = path.file_name().map_or_else(
        || "The file".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let read_error = |e: std::io::Error| ProofsError::Read {
        name: name.clone(),
        message: e.to_string(),
    };
    let file = std::fs::File::open(path).map_err(read_error)?;
    let size = file.metadata().map_err(read_error)?.len();
    if size > MAX_BYTES {
        return Err(ProofsError::TooLarge(name));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    check(&name, bytes)
}

/// SHA-256 of `bytes` in lowercase hex, as the Worker and the module write it.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// The proof record for an uploaded file, for `create_verification` (ADR 0004, Reducers):
/// linked to the claim or bill `link_id`, pending, shown in full to those allowed, the file name
/// as its description and in its metadata as the web writes it (`file_name`, `mime_type`,
/// `size`; `flowtrace/src/components/file-upload.tsx`, `useAttachProofs`).
#[must_use]
pub fn new_verification(kind: LinkKind, link_id: &str, a: &Attachment) -> NewVerification {
    let mut metadata = serde_json::Map::new();
    metadata.insert("file_name".to_owned(), a.name.clone().into());
    metadata.insert(
        "mime_type".to_owned(),
        a.uploaded.content_type.clone().into(),
    );
    metadata.insert("size".to_owned(), a.uploaded.size.into());
    let (claim_id, bill_id) = match kind {
        LinkKind::Claim => (Some(link_id.to_owned()), None),
        LinkKind::Bill => (None, Some(link_id.to_owned())),
    };
    NewVerification {
        transaction_id: None,
        claim_id,
        bill_id,
        kind: kind.proof_type(),
        object_key: Some(a.uploaded.key.clone()),
        sha256: Some(a.uploaded.sha256.clone()),
        thumbnail_key: a.uploaded.thumbnail_key.clone(),
        ocr_data: None,
        metadata,
        status: VerificationStatus::Pending,
        visibility_level: Visibility::Full,
        executor_name: None,
        gps_latitude: None,
        gps_longitude: None,
        gps_accuracy: None,
        location_name: None,
        description: Some(a.name.clone()),
    }
}

/// Where one file on a form stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachState {
    /// Being sent.
    Uploading,
    /// In the proof store.
    Ready(Attachment),
    /// Not sent; why.
    Failed(String),
}

/// One file on a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachRow {
    /// Stable id for buttons and for the upload's answer.
    pub id: u64,
    /// The file name.
    pub name: String,
    /// Where it stands.
    pub state: AttachState,
}

/// The files attached on a claim or bill form, uploaded as soon as they are chosen (as the web's
/// `FileUpload`, `flowtrace/src/components/file-upload.tsx`) and recorded on the books once the
/// claim or bill is saved and has an id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachList {
    rows: Vec<AttachRow>,
    next: u64,
}

impl AttachList {
    /// Adds `name` as uploading and returns its id.
    pub fn start(&mut self, name: &str) -> u64 {
        self.next += 1;
        self.rows.push(AttachRow {
            id: self.next,
            name: name.to_owned(),
            state: AttachState::Uploading,
        });
        self.next
    }

    /// Records how the upload of row `id` went; a row removed meanwhile stays removed.
    pub fn finish(&mut self, id: u64, outcome: Result<Uploaded, ProofsError>) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            row.state = match outcome {
                Ok(uploaded) => AttachState::Ready(Attachment {
                    name: row.name.clone(),
                    uploaded,
                }),
                Err(e) => AttachState::Failed(e.to_string()),
            };
        }
    }

    /// Takes row `id` off the form. The file stays in the store: it is content-addressed and
    /// nothing on the books points at it (the Worker has no delete, `workers/proofs/README.md`).
    pub fn remove(&mut self, id: u64) {
        self.rows.retain(|r| r.id != id);
    }

    /// Every row, in the order chosen.
    #[must_use]
    pub fn rows(&self) -> &[AttachRow] {
        &self.rows
    }

    /// Whether a file is still being sent; saving waits for it.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.rows.iter().any(|r| r.state == AttachState::Uploading)
    }

    /// The files in the store, in order.
    #[must_use]
    pub fn ready(&self) -> Vec<Attachment> {
        self.rows
            .iter()
            .filter_map(|r| match &r.state {
                AttachState::Ready(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    /// The names of the files in the store: the claim's `receipt_names` or the bill's
    /// `proof_ids`, as the web fills them (`claims.new.tsx:119`, `bills.new.tsx:182`).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.ready().into_iter().map(|a| a.name).collect()
    }
}

/// The Worker at `base` as the holder of `token`. Debug output leaves the token out.
#[derive(Clone)]
pub struct ProofsClient {
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl fmt::Debug for ProofsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProofsClient")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

/// The Worker's answer to `GET /file/<key>`.
#[derive(Deserialize)]
struct SignedUrl {
    url: String,
}

/// The Worker's error body.
#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

impl ProofsClient {
    /// The Worker at `base` (for example `https://proofs.example.org`, no trailing slash
    /// needed), sending `token` as the bearer.
    #[must_use]
    pub fn new(base: &str, token: impl Into<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent();
        Self {
            base: base.trim_end_matches('/').to_owned(),
            token: token.into(),
            agent,
        }
    }

    /// Uploads `file` for the organisation `org`, linked to the claim or bill `link_id`
    /// (`POST /upload`), and checks the Worker's hash against the file's own.
    ///
    /// # Errors
    ///
    /// [`ProofsError::Refused`] with the Worker's message, [`ProofsError::Network`],
    /// [`ProofsError::BadReply`] or [`ProofsError::HashMismatch`].
    pub fn upload(
        &self,
        org: &str,
        kind: LinkKind,
        link_id: &str,
        file: &ProofFile,
    ) -> Result<Uploaded, ProofsError> {
        let sha256 = sha256_hex(&file.bytes);
        let boundary = boundary(&sha256);
        let body = multipart(
            &boundary,
            &[
                ("org", org),
                ("link_kind", kind.as_str()),
                ("link_id", link_id),
            ],
            file,
        );
        let response = self
            .agent
            .post(format!("{}/upload", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .send(&body[..])
            .map_err(|e| ProofsError::Network(e.to_string()))?;
        let uploaded: Uploaded = read_reply(response)?;
        if uploaded.sha256 != sha256 {
            return Err(ProofsError::HashMismatch(file.name.clone()));
        }
        Ok(uploaded)
    }

    /// A link to the file at `key` that works for ten minutes without the token
    /// (`GET /file/<key>`), for the system browser.
    ///
    /// # Errors
    ///
    /// [`ProofsError::Refused`] (not allowed, or no such file), [`ProofsError::Network`] or
    /// [`ProofsError::BadReply`].
    pub fn signed_url(&self, key: &str) -> Result<String, ProofsError> {
        let path: Vec<String> = key.split('/').map(percent_encode).collect();
        let response = self
            .agent
            .get(format!("{}/file/{}", self.base, path.join("/")))
            .header("Authorization", format!("Bearer {}", self.token))
            .call()
            .map_err(|e| ProofsError::Network(e.to_string()))?;
        let signed: SignedUrl = read_reply(response)?;
        Ok(signed.url)
    }
}

/// The body of a 2xx reply as `T`, else [`ProofsError::Refused`] with the Worker's `error`.
fn read_reply<T: serde::de::DeserializeOwned>(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<T, ProofsError> {
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| ProofsError::Network(e.to_string()))?;
    if !(200..300).contains(&status) {
        let message = serde_json::from_str::<ErrorBody>(&text).map_or_else(
            |_| format!("The proof store refused the request (HTTP {status})."),
            |b| b.error,
        );
        return Err(ProofsError::Refused { status, message });
    }
    serde_json::from_str(&text).map_err(|e| ProofsError::BadReply(e.to_string()))
}

/// One path segment with everything but unreserved characters percent-encoded (RFC 3986 2.3).
fn percent_encode(segment: &str) -> String {
    segment
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// The multipart boundary for a file with this hash: a file holding `--` and its own hash's
/// prefix is not a practical worry.
fn boundary(sha256: &str) -> String {
    format!("flowtrace-proof-{}", sha256.get(..32).unwrap_or(sha256))
}

/// A `multipart/form-data` body (RFC 7578) with `fields` then `file` as the part named `file`.
/// Quotes, backslashes and line breaks in the file name become `_` so the header stays one line.
fn multipart(boundary: &str, fields: &[(&str, &str)], file: &ProofFile) -> Vec<u8> {
    let mut body = Vec::with_capacity(file.bytes.len() + 512);
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    let filename: String = file
        .name
        .chars()
        .map(|c| {
            if matches!(c, '"' | '\\' | '\r' | '\n') {
                '_'
            } else {
                c
            }
        })
        .collect();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {}\r\n\r\n",
            file.content_type
        )
        .as_bytes(),
    );
    body.extend_from_slice(&file.bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

#[cfg(test)]
mod tests;
