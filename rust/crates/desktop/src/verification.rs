//! The web's proof vocabulary and the rules of its proof screens, with no GPUI types: the labels
//! of `flowtrace/src/components/verification-gallery.tsx` (types, visibility, states, trust
//! points, media kind, the gallery row), the upload rules of `file-upload.tsx` (accepted types,
//! `describeAccepted`, `validateProofFile`, `formatFileSize`, the visibility descriptions), the
//! attach panel of `transactions/$id.tsx:402-584` and the record panel of
//! `verification-viewer.tsx`. The Proofs section (`crate::panels::proofs`) draws what these hold
//! on the transaction, claim and bill pages alike.
//!
//! Desktop differences, each also in `rust/docs/handoff.md`: there is no geolocation, so the GPS
//! reading is typed (latitude, longitude, accuracy, place); a file's type is read from its first
//! bytes ([`crate::proofs::sniff`]), not from the browser's MIME guess, so only the photos and
//! PDFs the Worker stores can pass; a PDF or video opens in the browser rather than inside the
//! viewer.

use ledger_module::transparency::types::{
    Verification, VerificationStatus, VerificationType, Visibility,
};

use crate::proofs::{Gps, ProofDetails};
use crate::web_format::{format_date, format_date_time};

/// `ATTACH_TYPES` (`transactions/$id.tsx:388-400`): the kinds the attach panel offers, in order.
pub const ATTACH_TYPES: [VerificationType; 11] = [
    VerificationType::Receipt,
    VerificationType::Invoice,
    VerificationType::Photo,
    VerificationType::BankStatement,
    VerificationType::Contract,
    VerificationType::Signature,
    VerificationType::Gps,
    VerificationType::Video,
    VerificationType::Audit,
    VerificationType::GovernmentDoc,
    VerificationType::Document,
];

/// `PROOF_TYPES` (`transactions/new.tsx:59-66`): the kinds the new-transaction form offers.
pub const NEW_FORM_TYPES: [VerificationType; 6] = [
    VerificationType::Receipt,
    VerificationType::Invoice,
    VerificationType::BankStatement,
    VerificationType::Photo,
    VerificationType::Contract,
    VerificationType::Document,
];

/// `VisibilitySelector`'s order, `Object.keys(PROOF_VISIBILITY_LABEL)`
/// (`file-upload.tsx`, `verification-gallery.tsx:62-67`).
pub const VISIBILITY_ORDER: [Visibility; 4] = [
    Visibility::Full,
    Visibility::Thumbnail,
    Visibility::Summary,
    Visibility::Hidden,
];

/// At most this many files at once on a form (`FileUpload` `maxFiles = 5`, `MAX_PROOF_FILES`).
pub const MAX_FILES: usize = 5;

/// The limit the screens state, in MB ([`crate::proofs::SCREEN_MAX_BYTES`]).
pub const SCREEN_MAX_MB: u64 = 10;

/// `PROOF_TYPE_LABEL` (`verification-gallery.tsx:48-61`).
#[must_use]
pub const fn type_label(kind: VerificationType) -> &'static str {
    match kind {
        VerificationType::Receipt => "Receipt",
        VerificationType::Photo => "Photo",
        VerificationType::Document => "Document",
        VerificationType::BankStatement => "Bank statement",
        VerificationType::Signature => "Signature",
        VerificationType::Gps => "GPS reading",
        VerificationType::Biometric => "Biometric",
        VerificationType::Invoice => "Invoice",
        VerificationType::Video => "Video",
        VerificationType::Audit => "Third-party audit",
        VerificationType::Contract => "Contract",
        VerificationType::GovernmentDoc => "Government document",
    }
}

/// `PROOF_VISIBILITY_LABEL` (`verification-gallery.tsx:62-67`).
#[must_use]
pub const fn visibility_label(level: Visibility) -> &'static str {
    match level {
        Visibility::Full => "Full file",
        Visibility::Thumbnail => "Thumbnail only",
        Visibility::Summary => "Summary only",
        Visibility::Hidden => "Hidden",
    }
}

/// `VISIBILITY_DESCRIPTION` (`verification-viewer.tsx:24-29`, the same words as
/// `VISIBILITY_DESCRIPTIONS` in `file-upload.tsx`).
#[must_use]
pub const fn visibility_description(level: Visibility) -> &'static str {
    match level {
        Visibility::Full => "Board and staff see the original file.",
        Visibility::Thumbnail => "Donors see a low-resolution version.",
        Visibility::Summary => "Partners see that proof is attached, not the file.",
        Visibility::Hidden => "Not shown outside the organisation.",
    }
}

/// `PROOF_STATE_LABEL` (`verification-gallery.tsx:70-74`): a proof's own state.
#[must_use]
pub const fn state_label(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Verified => "Verified",
        VerificationStatus::Pending => "Awaiting review",
        VerificationStatus::Rejected => "Rejected",
    }
}

/// `proofTrustPoints` (`verification-gallery.tsx:76-78`): the row's points, else its type's.
#[must_use]
pub const fn trust_points(v: &Verification) -> i64 {
    if v.trust_points == 0 {
        v.kind.trust_points()
    } else {
        v.trust_points
    }
}

/// What kind of file a proof holds (`proofMediaKind`, `verification-gallery.tsx:81-93`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// A photo.
    Image,
    /// A PDF.
    Pdf,
    /// A video.
    Video,
}

/// `proofMediaKind` from the recorded MIME type: a proof's key carries no file extension
/// (`org/<org>/<sha256>`), so only `metadata.mime_type` tells.
#[must_use]
pub fn media_kind(v: &Verification) -> Option<MediaKind> {
    let mime = v
        .metadata
        .get("mime_type")
        .and_then(|m| m.as_str())
        .unwrap_or_default();
    if mime.starts_with("image/") {
        Some(MediaKind::Image)
    } else if mime == "application/pdf" {
        Some(MediaKind::Pdf)
    } else if mime.starts_with("video/") {
        Some(MediaKind::Video)
    } else {
        None
    }
}

/// The key a gallery tile draws: the small version, else the file (`ProofPreview`,
/// `thumbnail_url || file_url`). Both are gone from a row the reader may only see as a summary.
#[must_use]
pub fn preview_key(v: &Verification) -> Option<&str> {
    v.thumbnail_key.as_deref().or(v.object_key.as_deref())
}

/// The key the viewer opens and "Download original" fetches (`file_url`): the file, else the
/// small version a thumbnail-only reader is given (`plan::redact`).
#[must_use]
pub fn file_key(v: &Verification) -> Option<&str> {
    v.object_key.as_deref().or(v.thumbnail_key.as_deref())
}

/// `PROOF_ACCEPT` (`file-upload.tsx:77-96`): the file types each kind of proof accepts.
#[must_use]
pub const fn accepted(kind: VerificationType) -> &'static [&'static str] {
    const PDF: &str = "application/pdf";
    const JPEG: &str = "image/jpeg";
    const PNG: &str = "image/png";
    const WEBP: &str = "image/webp";
    match kind {
        VerificationType::Receipt => &[JPEG, PNG, WEBP, PDF],
        VerificationType::Photo | VerificationType::Signature => &[JPEG, PNG, WEBP],
        VerificationType::Document => &[
            PDF,
            JPEG,
            PNG,
            "application/msword",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ],
        VerificationType::BankStatement
        | VerificationType::Invoice
        | VerificationType::Contract
        | VerificationType::GovernmentDoc => &[PDF, JPEG, PNG],
        VerificationType::Gps => &["application/json", "text/plain", JPEG, PNG],
        VerificationType::Biometric => &["application/json"],
        VerificationType::Video => &["video/mp4", "video/webm", "video/quicktime"],
        VerificationType::Audit => &[PDF],
    }
}

/// `EXTENSION` (`file-upload.tsx:98-111`): the name a file type is described by.
fn extension(mime: &str) -> &str {
    match mime {
        "image/jpeg" => "JPG",
        "image/png" => "PNG",
        "image/webp" => "WebP",
        "application/pdf" => "PDF",
        "application/msword" => "DOC",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "DOCX",
        "application/json" => "JSON",
        "text/plain" => "TXT",
        "video/mp4" => "MP4",
        "video/webm" => "WebM",
        "video/quicktime" => "MOV",
        other => other,
    }
}

/// `describeAccepted` (`file-upload.tsx:114-118`): `PDF, JPG or PNG`.
#[must_use]
pub fn describe_accepted(types: &[&str]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for t in types {
        let name = extension(t);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    match names.split_last() {
        None => String::new(),
        Some((only, [])) => (*only).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
    }
}

/// `formatFileSize` (`file-upload.tsx:136-142`): `0 B`, `512 B`, `1.5 KB`, `2 MB`: one decimal
/// by `toFixed(1)`, a trailing `.0` dropped by `parseFloat`.
#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_owned();
    }
    const SIZES: [&str; 4] = ["B", "KB", "MB", "GB"];
    #[expect(
        clippy::cast_precision_loss,
        reason = "a JS number, as the web holds it"
    )]
    let n = bytes as f64;
    let mut i = 0;
    while i + 1 < SIZES.len() && n >= 1024_f64.powi(i32::try_from(i + 1).unwrap_or(3)) {
        i += 1;
    }
    let value = n / 1024_f64.powi(i32::try_from(i).unwrap_or(0));
    let fixed = crate::statements::to_fixed(value, 1);
    let shown = fixed.strip_suffix(".0").unwrap_or(&fixed);
    format!("{shown} {}", SIZES[i])
}

/// `validateProofFile` (`file-upload.tsx:121-134`): why a file cannot be used as proof, or
/// `None`. `content_type` is what the first bytes say ([`crate::proofs::sniff`]); `None` when
/// they name no type the Worker stores.
#[must_use]
pub fn validate_proof_file(
    name: &str,
    content_type: Option<&str>,
    size: u64,
    accepted: &[&str],
    max_mb: u64,
) -> Option<String> {
    if !content_type.is_some_and(|t| accepted.contains(&t)) {
        return Some(format!(
            "{name} is not a {} file.",
            describe_accepted(accepted)
        ));
    }
    if size > max_mb * 1024 * 1024 {
        return Some(format!(
            "{name} is {}; the limit is {max_mb} MB.",
            format_file_size(size)
        ));
    }
    None
}

/// The upload hint (`FileUpload`, `file-upload.tsx`): `Receipt: JPG, PNG, WebP or PDF, up to 10
/// MB each`, or the limit once [`MAX_FILES`] are on the form.
#[must_use]
pub fn upload_hint(kind: VerificationType, files: usize) -> String {
    if files >= MAX_FILES {
        return format!("That's the limit of {MAX_FILES} files at once.");
    }
    format!(
        "{}: {}, up to {SCREEN_MAX_MB} MB each",
        type_label(kind),
        describe_accepted(accepted(kind))
    )
}

/// The line under a file on a form (`FileRow`): its size and where it stands, or its error.
#[must_use]
pub fn file_row_status(size: u64, uploading: bool, error: Option<&str>) -> String {
    error.map_or_else(
        || {
            format!(
                "{} \u{b7} {}",
                format_file_size(size),
                if uploading {
                    "Uploading\u{2026}"
                } else {
                    "Uploaded"
                }
            )
        },
        str::to_owned,
    )
}

/// The empty notice of the gallery (`VerificationGallery`, `verification-gallery.tsx:166-184`).
pub const EMPTY_TITLE: &str = "No proof attached yet";
/// Its body.
pub const EMPTY_BODY: &str = "Receipts, photos, bank statements and GPS readings attached to \
                              this transaction will be listed here. Each one adds trust points.";

/// One row of the gallery (`ProofEntry`, `verification-gallery.tsx:205-262`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofRow {
    /// The proof.
    pub id: String,
    /// Its kind, for the tile's icon.
    pub kind: VerificationType,
    /// `Receipt`.
    pub label: &'static str,
    /// `+5`.
    pub points: String,
    /// What it shows.
    pub description: Option<String>,
    /// Its state, for the mark.
    pub status: VerificationStatus,
    /// `Awaiting review`.
    pub state: &'static str,
    /// `Added 4 Feb 2026 by Achieng, Lokichar`.
    pub added: String,
    /// The visibility label when it is not the full file.
    pub limited: Option<&'static str>,
    /// What the file is.
    pub media: Option<MediaKind>,
    /// The key of its tile, when the reader may see one.
    pub preview: Option<String>,
    /// Shown at the Worker's public link (module addition, STO1).
    pub published: bool,
}

impl ProofRow {
    /// The row of `v`.
    #[must_use]
    pub fn of(v: &Verification) -> Self {
        let who: Vec<&str> = [v.executor_name.as_deref(), v.location_name.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect();
        let mut added = format!("Added {}", format_date(&v.created_at));
        if !who.is_empty() {
            added.push_str(&format!(" by {}", who.join(", ")));
        }
        Self {
            id: v.id.clone(),
            kind: v.kind,
            label: type_label(v.kind),
            points: format!("+{}", trust_points(v)),
            description: v.description.clone().filter(|d| !d.is_empty()),
            status: v.status,
            state: state_label(v.status),
            added,
            limited: (v.visibility_level != Visibility::Full)
                .then(|| visibility_label(v.visibility_level)),
            media: media_kind(v),
            preview: preview_key(v).map(str::to_owned),
            published: v.published,
        }
    }
}

/// The Proofs section's meta (`transactions/$id.tsx`): `2 proofs · +13 trust points`; none
/// without proofs.
#[must_use]
pub fn proofs_meta(proofs: &[Verification]) -> Option<String> {
    if proofs.is_empty() {
        return None;
    }
    let points: i64 = proofs.iter().map(trust_points).sum();
    Some(format!(
        "{} {} \u{b7} +{points} trust points",
        proofs.len(),
        if proofs.len() == 1 { "proof" } else { "proofs" }
    ))
}

/// The gallery's tail when it shows fewer than all (`VerificationGallery`): `3 more proofs`.
#[must_use]
pub fn more_line(hidden: usize) -> String {
    format!(
        "{hidden} more {}",
        if hidden == 1 { "proof" } else { "proofs" }
    )
}

/// `getVerifications` order (`flowtrace/src/lib/demo/api.ts:276-282`): newest `created_at`
/// first, ties in record order.
#[must_use]
pub fn newest_first(mut proofs: Vec<Verification>) -> Vec<Verification> {
    proofs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    proofs
}

/// "Kind of proof" (`AttachProof`): `Receipt (+5)` for each of [`ATTACH_TYPES`].
#[must_use]
pub fn attach_choices() -> Vec<(VerificationType, String)> {
    ATTACH_TYPES
        .iter()
        .map(|k| (*k, format!("{} (+{})", type_label(*k), k.trust_points())))
        .collect()
}

/// "Kind of proof" on the new-transaction form: `Receipt (+5 trust points)`.
#[must_use]
pub fn new_form_choices() -> Vec<(VerificationType, String)> {
    NEW_FORM_TYPES
        .iter()
        .map(|k| {
            (
                *k,
                format!("{} (+{} trust points)", type_label(*k), k.trust_points()),
            )
        })
        .collect()
}

/// The hint under a locked "Kind of proof" (`AttachProof`).
pub const KIND_LOCKED: &str = "Remove the files to change the kind of proof.";

/// The attach panel's fields (`AttachProof`, `transactions/$id.tsx:415-584`, with
/// `EnhancedUploadForm`): the kind of proof, who may see it, what it shows, who captured it, and
/// the typed GPS reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachForm {
    /// The kind of proof; `receipt` first.
    pub kind: VerificationType,
    /// Who can see the file; `full` first.
    pub visibility: Visibility,
    /// What it shows.
    pub description: String,
    /// Captured by.
    pub executor: String,
    /// Latitude as typed.
    pub latitude: String,
    /// Longitude as typed.
    pub longitude: String,
    /// Accuracy in metres as typed.
    pub accuracy: String,
    /// Place name as typed.
    pub place: String,
}

impl Default for AttachForm {
    fn default() -> Self {
        Self {
            kind: VerificationType::Receipt,
            visibility: Visibility::Full,
            description: String::new(),
            executor: String::new(),
            latitude: String::new(),
            longitude: String::new(),
            accuracy: String::new(),
            place: String::new(),
        }
    }
}

/// A typed number, or `None` for a blank field.
fn number(text: &str) -> Option<Result<f64, ()>> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    Some(t.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or(()))
}

impl AttachForm {
    /// `+5 trust points each` (`AttachProof` header).
    #[must_use]
    pub fn points_line(&self) -> String {
        format!("+{} trust points each", self.kind.trust_points())
    }

    /// The typed GPS reading: none when latitude and longitude are both blank. Desktop rule (no
    /// geolocation): both coordinates are needed, latitude within -90 to 90, longitude within
    /// -180 to 180, accuracy, when given, zero or more.
    ///
    /// # Errors
    ///
    /// The field to fix, in a sentence.
    pub fn gps(&self) -> Result<Option<Gps>, String> {
        let lat = number(&self.latitude);
        let lon = number(&self.longitude);
        if lat.is_none() && lon.is_none() {
            return Ok(None);
        }
        let latitude = match lat {
            Some(Ok(n)) if (-90.0..=90.0).contains(&n) => n,
            _ => return Err("Enter the latitude as a number from -90 to 90.".to_owned()),
        };
        let longitude = match lon {
            Some(Ok(n)) if (-180.0..=180.0).contains(&n) => n,
            _ => return Err("Enter the longitude as a number from -180 to 180.".to_owned()),
        };
        let accuracy = match number(&self.accuracy) {
            None => None,
            Some(Ok(n)) if n >= 0.0 => Some(n),
            Some(_) => {
                return Err("Enter the accuracy in metres, or leave it blank.".to_owned());
            }
        };
        let place = self.place.trim();
        Ok(Some(Gps {
            latitude,
            longitude,
            accuracy,
            location_name: (!place.is_empty()).then(|| place.to_owned()),
        }))
    }

    /// The check before attaching (`save`, `AttachProof`): a file, or for a GPS reading a
    /// location; then the details the record carries.
    ///
    /// # Errors
    ///
    /// The panel's problem line.
    pub fn check(&self, files: usize) -> Result<ProofDetails, String> {
        let gps = self.gps()?;
        let gps_only = self.kind == VerificationType::Gps && gps.is_some();
        if files == 0 && !gps_only {
            return Err(if self.kind == VerificationType::Gps {
                "Add a file or your current location before attaching.".to_owned()
            } else {
                "Add at least one file before attaching.".to_owned()
            });
        }
        Ok(ProofDetails {
            kind: self.kind,
            visibility: self.visibility,
            description: self.description.clone(),
            executor: self.executor.clone(),
            gps,
        })
    }
}

/// The attach button (`AttachProof`): `Attaching…`, `Attach 2 proofs` or `Attach proof`.
#[must_use]
pub fn attach_label(saving: bool, files: usize) -> String {
    if saving {
        "Attaching\u{2026}".to_owned()
    } else if files > 1 {
        format!("Attach {files} proofs")
    } else {
        "Attach proof".to_owned()
    }
}

/// The problem line when recording failed (`AttachProof`, `save`).
#[must_use]
pub fn attach_failure(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        "The proof was not attached. Your files are still here; try again.".to_owned()
    } else {
        format!("The proof was not attached: {message}. Your files are still here; try again.")
    }
}

/// The viewer's file states (`verification-viewer.tsx:204-258`).
pub const OPENING: &str = "Opening the file\u{2026}";
/// The file could not be fetched.
pub const NOT_LOADED: &str = "The file could not be loaded";
/// A file the app cannot draw.
pub const NO_PREVIEW: &str = "This file type cannot be previewed here. Download it to open it.";
/// A record with no file.
pub const NO_FILE: &str = "This proof is a record without an attached file.";
/// The line under the points.
pub const POINTS_NOTE: &str = "trust points toward the organisation's score";

/// The viewer's position line: `Proof 2 of 3`, none for a single proof.
#[must_use]
pub fn position(index: usize, count: usize) -> Option<String> {
    (count > 1).then(|| format!("Proof {} of {count}", index + 1))
}

/// The viewer's image zoom (`verification-viewer.tsx`): 50% to 300% in steps of 25%.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zoom(pub f32);

impl Default for Zoom {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Zoom {
    /// Zoom out, not below 50%.
    #[must_use]
    pub fn out(self) -> Self {
        Self((self.0 - 0.25).max(0.5))
    }

    /// Zoom in, not past 300%.
    #[must_use]
    pub fn into(self) -> Self {
        Self((self.0 + 0.25).min(3.0))
    }

    /// Whether zooming out is possible.
    #[must_use]
    pub fn can_out(self) -> bool {
        self.0 > 0.5
    }

    /// Whether zooming in is possible.
    #[must_use]
    pub fn can_in(self) -> bool {
        self.0 < 3.0
    }

    /// `Math.round(zoom * 100)%`.
    #[must_use]
    pub fn percent(self) -> String {
        format!("{}%", (self.0 * 100.0).round())
    }
}

/// One fact of the record panel (`Fact`, `verification-viewer.tsx`): term, value, the small line
/// under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// `Captured by`.
    pub term: &'static str,
    /// The value.
    pub value: String,
    /// The muted line under it.
    pub note: Option<String>,
}

/// `toFixed(5)` of a coordinate.
fn coordinate(n: f64) -> String {
    crate::statements::to_fixed(n, 5)
}

/// The record panel's facts (`verification-viewer.tsx:286-314`): Captured by, Location, Added,
/// Verified, Who can see it.
#[must_use]
pub fn viewer_facts(v: &Verification) -> Vec<Fact> {
    let mut facts = Vec::new();
    if let Some(who) = v.executor_name.clone().filter(|s| !s.is_empty()) {
        facts.push(Fact {
            term: "Captured by",
            value: who,
            note: None,
        });
    }
    if v.location_name.is_some() || v.gps_latitude.is_some() {
        let note = match (v.gps_latitude, v.gps_longitude) {
            (Some(lat), Some(lon)) => {
                let mut line = format!("{}, {}", coordinate(lat), coordinate(lon));
                if let Some(acc) = v.gps_accuracy {
                    line.push_str(&format!(" (\u{b1}{} m)", acc.round()));
                }
                Some(line)
            }
            _ => None,
        };
        facts.push(Fact {
            term: "Location",
            value: v.location_name.clone().unwrap_or_default(),
            note,
        });
    }
    facts.push(Fact {
        term: "Added",
        value: format_date_time(&v.created_at),
        note: None,
    });
    if let Some(at) = &v.verified_at
        && v.status == VerificationStatus::Verified
    {
        facts.push(Fact {
            term: "Verified",
            value: format_date(at),
            note: None,
        });
    }
    facts.push(Fact {
        term: "Who can see it",
        value: visibility_label(v.visibility_level).to_owned(),
        note: Some(visibility_description(v.visibility_level).to_owned()),
    });
    facts
}

/// What the viewer's file area shows, from the proof and how fetching its bytes went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerBody {
    /// Fetching.
    Opening,
    /// The fetch failed; the reason.
    Failed(String),
    /// The photo, drawn in the app.
    Image,
    /// A file the app does not draw (PDF, video, other): "Download original" opens it.
    NoPreview,
    /// No file: a record such as a GPS reading, or a file the reader may not see.
    NoFile,
}

/// How fetching the viewer's photo stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetch {
    /// Not asked for, or under way.
    Pending,
    /// The bytes are here.
    Ready,
    /// It failed.
    Failed(String),
}

/// The file area for `v` (`verification-viewer.tsx:204-258`).
#[must_use]
pub fn viewer_body(v: &Verification, fetch: &Fetch) -> ViewerBody {
    if file_key(v).is_none() {
        return ViewerBody::NoFile;
    }
    if media_kind(v) != Some(MediaKind::Image) {
        return ViewerBody::NoPreview;
    }
    match fetch {
        Fetch::Pending => ViewerBody::Opening,
        Fetch::Ready => ViewerBody::Image,
        Fetch::Failed(why) => ViewerBody::Failed(format!(
            "{} Check your connection and try again.",
            why.trim_end()
        )),
    }
}

/// The publish action of the viewer (desktop addition, `rust/docs/handoff.md`): only a manager
/// (`can_close`, `plan::mark_published`) sees it, and only on a proof with a file.
#[must_use]
pub fn publish_label(v: &Verification, manager: bool) -> Option<&'static str> {
    if !manager || (v.object_key.is_none() && !v.published) {
        return None;
    }
    Some(if v.published {
        "Take off the public link"
    } else {
        "Publish at the public link"
    })
}

/// The line under the publish action.
#[must_use]
pub const fn publish_note(published: bool) -> &'static str {
    if published {
        "Anyone with the proof's public link can open this file."
    } else {
        "Only people your organisation lets in can open this file."
    }
}

#[cfg(test)]
mod tests;
