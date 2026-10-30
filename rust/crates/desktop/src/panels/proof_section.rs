//! The Proofs section of a transaction, claim or bill page (DT3): the gallery of
//! `verification-gallery.tsx`, the attach panel of `transactions/$id.tsx:402-584` with
//! `FileUpload` and `EnhancedUploadForm` (`file-upload.tsx`), and the full-screen viewer of
//! `verification-viewer.tsx`. One entity per page; the page draws it as a child and forwards
//! keys to [`ProofsSection::press_key`] (Esc, the arrows, `a`, `v`).
//!
//! The rules live in [`crate::verification`] and [`crate::proofs`]; this file picks files with
//! the platform chooser, runs the Worker calls on GPUI's background executor, fetches photos as
//! bytes from a signed link and draws them with `ImageSource::Image`
//! (<https://docs.rs/gpui/latest/gpui/enum.ImageSource.html>), falling back to the type's mark
//! when that fails, as the web's `ProofPreview` does. Opening a file logs a view, downloading a
//! download (`log_verification_access`). Desktop addition: a manager can publish a proof at the
//! public link from the viewer (`mark_published`; the web has no such control,
//! `rust/docs/handoff.md`).

mod attach;
mod viewer;

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::InputState;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, FontWeight, Image, ImageFormat,
    ImageSource, InteractiveElement as _, IntoElement, ObjectFit, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Subscription,
    Window, div, img, px,
};
use ledger_core::types::OrganizationId;
use ledger_module::transparency::types::{AccessType, Verification, VerificationType};

use crate::panels::dashboard::small_muted;
use crate::panels::section_header;
use crate::proofs::{AttachList, LinkKind, ProofsError};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space};
use crate::verification::{
    AttachForm, EMPTY_BODY, EMPTY_TITLE, MediaKind, ProofRow, Zoom, file_key, media_kind,
    newest_first, proofs_meta,
};

/// A photo fetched for drawing.
#[derive(Clone)]
enum Loaded {
    /// Under way.
    Loading,
    /// The image.
    Ready(Arc<Image>),
    /// It failed; why.
    Failed(String),
}

/// What the section last read: the organisation, its records' revision and the link.
type Seen = (Option<OrganizationId>, u64, Option<(LinkKind, String)>);

/// The Proofs section.
pub struct ProofsSection {
    store: Entity<LedgerStore>,
    link: Option<(LinkKind, String)>,
    seen: Option<Seen>,
    proofs: Vec<Verification>,
    manager: bool,
    /// A line about the last action that went wrong (open, publish).
    note: Option<String>,
    /// Files that did not upload with a new transaction (`?failed=n`).
    failed_uploads: usize,

    attach_open: bool,
    form: AttachForm,
    files: AttachList,
    saving: bool,
    problem: Option<String>,
    kind_pick: crate::panels::PickState,
    visibility_pick: crate::panels::PickState,
    description: Entity<InputState>,
    executor: Entity<InputState>,
    latitude: Entity<InputState>,
    longitude: Entity<InputState>,
    accuracy: Entity<InputState>,
    place: Entity<InputState>,

    viewer: Option<String>,
    zoom: Zoom,
    images: HashMap<String, Loaded>,
    _subscriptions: Vec<Subscription>,
}

impl ProofsSection {
    /// The section over `store`, showing nothing until [`ProofsSection::show`].
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
            let placeholder = placeholder.to_owned();
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let (kind_pick, visibility_pick) = attach::picks(window, cx);
        let mut section = Self {
            description: input(
                "e.g. Receipt from Lokichar Hardware for drilling pipes",
                window,
                cx,
            ),
            executor: input("Name of the person in the field", window, cx),
            latitude: input("e.g. -3.11950", window, cx),
            longitude: input("e.g. 35.59720", window, cx),
            accuracy: input("Metres, if known", window, cx),
            place: input("e.g. Lokichar", window, cx),
            _subscriptions: attach::subscriptions(&store, &kind_pick, &visibility_pick, window, cx),
            kind_pick,
            visibility_pick,
            store,
            link: None,
            seen: None,
            proofs: Vec::new(),
            manager: false,
            note: None,
            failed_uploads: 0,
            attach_open: false,
            form: AttachForm::default(),
            files: AttachList::default(),
            saving: false,
            problem: None,
            viewer: None,
            zoom: Zoom::default(),
            images: HashMap::new(),
        };
        section.refresh(cx);
        section
    }

    /// Shows the proofs of `link`'s claim, bill or transaction; a new link closes the panel and
    /// the viewer and forgets the failed-upload notice.
    pub fn show(&mut self, link: Option<(LinkKind, String)>, cx: &mut Context<Self>) {
        if self.link != link {
            self.link = link;
            self.attach_open = false;
            self.viewer = None;
            self.note = None;
            self.problem = None;
            self.failed_uploads = 0;
            self.files = AttachList::default();
        }
        self.refresh(cx);
    }

    /// Opens the viewer on the newest proof, if there is one (`o` on a claim or bill page).
    pub fn open_first(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.proofs.first().map(|v| v.id.clone()) else {
            return false;
        };
        self.open_viewer(&id, cx);
        true
    }

    /// Notes that `n` files did not upload with a new transaction, as `?failed=n` does.
    pub fn set_failed_uploads(&mut self, n: usize, cx: &mut Context<Self>) {
        self.failed_uploads = n;
        cx.notify();
    }

    /// Whether the viewer is open; the page then draws only this entity.
    #[must_use]
    pub fn viewing(&self) -> bool {
        self.viewer.is_some()
    }

    /// Whether the attach panel is open.
    #[must_use]
    pub fn attaching(&self) -> bool {
        self.attach_open
    }

    /// Keys while the page shows and no field has focus: in the viewer Esc closes, the arrows
    /// step, `-` and `=` zoom, `d` downloads; else `a` opens the attach panel, `b` browses for files
    /// and Enter attaches while it is open, Esc closes it, `v` opens the viewer on the first proof.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.viewer.is_some() {
            match key {
                "escape" => self.close_viewer(cx),
                "left" => self.step(false, cx),
                "right" => self.step(true, cx),
                "-" => self.zoom = self.zoom.out(),
                "=" | "+" => self.zoom = self.zoom.into(),
                "d" => self.download(cx),
                "p" if self.manager => self.toggle_published(cx),
                _ => return false,
            }
            cx.notify();
            return true;
        }
        match key {
            "a" if self.link.is_some() && !self.attach_open => self.open_attach(window, cx),
            "escape" if self.attach_open && !self.saving => self.close_attach(cx),
            "b" if self.attach_open => self.choose(window, cx),
            "enter" if self.attach_open => self.save(cx),
            "v" if !self.proofs.is_empty() => {
                let id = self.proofs[0].id.clone();
                self.open_viewer(&id, cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Re-reads the proofs when the organisation, its records or the link changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let view = store.view();
        let key = (
            view.selected().map(|o| o.id.clone()),
            view.transparency_revision(),
            self.link.clone(),
        );
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        self.manager = store.actor().is_some_and(|a| a.can_close);
        self.proofs = self
            .link
            .as_ref()
            .map(|(kind, id)| newest_first(store.proofs_of(*kind, id)))
            .unwrap_or_default();
        if self
            .viewer
            .as_ref()
            .is_some_and(|id| !self.proofs.iter().any(|v| &v.id == id))
        {
            self.viewer = None;
        }
        let wanted: Vec<String> = self
            .proofs
            .iter()
            .filter(|v| media_kind(v) == Some(MediaKind::Image))
            .filter_map(|v| crate::verification::preview_key(v).map(str::to_owned))
            .collect();
        for key in wanted {
            self.load(key, cx);
        }
        cx.notify();
    }

    /// Fetches the photo at `key` once: a signed link from the Worker, then its bytes.
    fn load(&mut self, key: String, cx: &mut Context<Self>) {
        if self.images.contains_key(&key) {
            return;
        }
        let Some(format) = self.format_of(&key) else {
            return;
        };
        let client = match self.store.read(cx).proofs_client() {
            Ok((client, _)) => client,
            Err(e) => {
                self.images.insert(key, Loaded::Failed(e.to_string()));
                return;
            }
        };
        self.images.insert(key.clone(), Loaded::Loading);
        cx.spawn(async move |this, cx| {
            let fetch_key = key.clone();
            let fetched: Result<Vec<u8>, ProofsError> = cx
                .background_spawn(async move {
                    let url = client.signed_url(&fetch_key)?;
                    client.fetch_bytes(&url)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = match fetched {
                    Ok(bytes) => Loaded::Ready(Arc::new(Image::from_bytes(format, bytes))),
                    Err(e) => Loaded::Failed(e.to_string()),
                };
                this.images.insert(key, state);
                cx.notify();
            });
        })
        .detach();
    }

    /// The image format of the proof whose file or thumbnail is `key`, from its recorded type;
    /// none for a type GPUI does not draw (HEIC).
    fn format_of(&self, key: &str) -> Option<ImageFormat> {
        let v = self.proofs.iter().find(|v| {
            v.object_key.as_deref() == Some(key) || v.thumbnail_key.as_deref() == Some(key)
        })?;
        if v.thumbnail_key.as_deref() == Some(key) && v.object_key.as_deref() != Some(key) {
            // The Worker's thumbnails are WebP (`workers/proofs/README.md`).
            return Some(ImageFormat::Webp);
        }
        match v.metadata.get("mime_type").and_then(|m| m.as_str())? {
            "image/png" => Some(ImageFormat::Png),
            "image/jpeg" => Some(ImageFormat::Jpeg),
            "image/webp" => Some(ImageFormat::Webp),
            "image/gif" => Some(ImageFormat::Gif),
            _ => None,
        }
    }

    fn open_viewer(&mut self, id: &str, cx: &mut Context<Self>) {
        self.viewer = Some(id.to_owned());
        self.zoom = Zoom::default();
        if let Some(v) = self.proofs.iter().find(|v| v.id == id).cloned()
            && let Some(key) = file_key(&v)
        {
            if media_kind(&v) == Some(MediaKind::Image) {
                self.load(key.to_owned(), cx);
            }
            self.log(&v.id, AccessType::View, cx);
        }
        cx.notify();
    }

    fn close_viewer(&mut self, cx: &mut Context<Self>) {
        self.viewer = None;
        cx.notify();
    }

    /// The previous or next proof.
    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(index) = self.viewer_index() else {
            return;
        };
        let next = if forward {
            index + 1
        } else {
            index.wrapping_sub(1)
        };
        if let Some(id) = self.proofs.get(next).map(|v| v.id.clone()) {
            self.open_viewer(&id, cx);
        }
    }

    fn viewer_index(&self) -> Option<usize> {
        let id = self.viewer.as_ref()?;
        self.proofs.iter().position(|v| &v.id == id)
    }

    /// Records a look at proof `id` (`log_verification_access`). The web logs nothing, so a
    /// refusal is only noted under the record, not shown as a failure.
    fn log(&mut self, id: &str, access: AccessType, cx: &mut Context<Self>) {
        let submission = self
            .store
            .update(cx, |s, _| s.log_verification_access(id, access));
        cx.spawn(async move |this, cx| {
            if let Err(e) = submission.result().await {
                let _ = this.update(cx, |this, cx| {
                    this.note = Some(format!("The access log did not take this view: {e}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// "Download original": a ten-minute link to the file in the system browser, logged as a
    /// download.
    fn download(&mut self, cx: &mut Context<Self>) {
        let Some(v) = self
            .viewer_index()
            .and_then(|i| self.proofs.get(i))
            .cloned()
        else {
            return;
        };
        let Some(key) = file_key(&v).map(str::to_owned) else {
            return;
        };
        let client = match self.store.read(cx).proofs_client() {
            Ok((client, _)) => client,
            Err(e) => {
                self.note = Some(e.to_string());
                return;
            }
        };
        self.log(&v.id, AccessType::Download, cx);
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { client.signed_url(&key) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match outcome {
                    Ok(url) => cx.open_url(&url),
                    Err(e) => this.note = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Publishes the open proof at the public link, or takes it back.
    fn toggle_published(&mut self, cx: &mut Context<Self>) {
        let Some(v) = self
            .viewer_index()
            .and_then(|i| self.proofs.get(i))
            .cloned()
        else {
            return;
        };
        let submission = self
            .store
            .update(cx, |s, _| s.mark_published(&v.id, !v.published));
        self.note = None;
        cx.spawn(async move |this, cx| {
            let result = submission.result().await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.note = Some(e.to_string());
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The gallery: one row per proof, or the empty notice with "Attach proof".
    fn gallery(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        if self.proofs.is_empty() {
            return v_flex()
                .py(px(20.0))
                .gap(px(8.0))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(EMPTY_TITLE))
                .child(small_muted(EMPTY_BODY, p))
                .when(!self.attach_open && self.link.is_some(), |v| {
                    v.child(
                        div().pt(px(8.0)).child(
                            Button::new("proofs-empty-attach")
                                .primary()
                                .label("Attach proof  a")
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.open_attach(window, cx);
                                })),
                        ),
                    )
                })
                .into_any_element();
        }
        let rows: Vec<AnyElement> = self
            .proofs
            .iter()
            .map(|v| self.entry(&ProofRow::of(v), cx))
            .collect();
        v_flex().children(rows).into_any_element()
    }

    /// `ProofEntry`: tile, type, points, description, state, added, limited audience.
    fn entry(&self, row: &ProofRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = row.id.clone();
        h_flex()
            .id(SharedString::from(format!("proof-{}", row.id)))
            .py(px(14.0))
            .gap(px(16.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .hover(|r| r.bg(p.wash.to_hsla()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.open_viewer(&id, cx);
            }))
            .child(self.tile(row, 60.0, p))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(2.0))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap(px(16.0))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(row.label))
                            .child(
                                h_flex()
                                    .gap(px(4.0))
                                    .items_baseline()
                                    .child(
                                        div()
                                            .text_size(px(18.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child(row.points.clone()),
                                    )
                                    .child(small_muted("trust points", p)),
                            ),
                    )
                    .when_some(row.description.clone(), |v, d| v.child(small_muted(&d, p)))
                    .child(
                        h_flex()
                            .pt(px(6.0))
                            .gap(px(16.0))
                            .flex_wrap()
                            .child(proof_state_mark(row, p))
                            .child(small_muted(&row.added, p))
                            .when_some(row.limited, |h, label| {
                                h.child(small_muted(&format!("\u{2298} {label}"), p))
                            })
                            .when(row.published, |h| h.child(small_muted("Published", p))),
                    ),
            )
            .into_any_element()
    }

    /// `ProofPreview`: the photo, else the type's mark.
    fn tile(&self, row: &ProofRow, size: f32, p: &Palette) -> AnyElement {
        let base = div()
            .size(px(size))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .border_1()
            .border_color(p.hairline.to_hsla())
            .bg(p.wash.to_hsla());
        let image = (row.media == Some(MediaKind::Image))
            .then(|| row.preview.as_ref().and_then(|k| self.images.get(k)))
            .flatten();
        match image {
            Some(Loaded::Ready(image)) => base
                .child(
                    img(ImageSource::Image(Arc::clone(image)))
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .into_any_element(),
            _ => base
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(p.ink_muted.to_hsla())
                        .child(type_mark(row.kind, row.media)),
                )
                .into_any_element(),
        }
    }
}

/// The tile's mark when there is no photo: the type's short name (the web draws a Phosphor
/// icon; the desktop has no icon font, `rust/docs/handoff.md`).
fn type_mark(kind: VerificationType, media: Option<MediaKind>) -> &'static str {
    if media == Some(MediaKind::Pdf) {
        return "PDF";
    }
    match kind {
        VerificationType::Receipt => "RCPT",
        VerificationType::Photo => "PHOTO",
        VerificationType::Gps => "GPS",
        VerificationType::Signature => "SIGN",
        VerificationType::Biometric => "BIO",
        VerificationType::BankStatement => "BANK",
        VerificationType::Document => "DOC",
        VerificationType::Invoice => "INV",
        VerificationType::Video => "VIDEO",
        VerificationType::Audit => "AUDIT",
        VerificationType::Contract => "CTRCT",
        VerificationType::GovernmentDoc => "GOV",
    }
}

/// `StatusMark` with `PROOF_STATE_LABEL`: the proof's own state as a line sample and its word.
fn proof_state_mark(row: &ProofRow, p: &Palette) -> impl IntoElement {
    use ledger_module::transparency::flows::TransactionStatus;
    use ledger_module::transparency::types::VerificationStatus;
    let status = match row.status {
        VerificationStatus::Verified => TransactionStatus::Verified,
        VerificationStatus::Pending => TransactionStatus::Pending,
        VerificationStatus::Rejected => TransactionStatus::Rejected,
    };
    crate::panels::dashboard::status_mark(status, row.state, p)
}

impl Render for ProofsSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(index) = self.viewer_index() {
            return self.viewer(index, cx);
        }
        let p = palette(cx.theme().mode);
        let meta = h_flex()
            .gap(px(12.0))
            .when_some(proofs_meta(&self.proofs), |h, m| {
                h.child(small_muted(&m, p))
            })
            .when(
                !self.attach_open && !self.proofs.is_empty() && self.link.is_some(),
                |h| {
                    h.child(
                        Button::new("proofs-attach")
                            .primary()
                            .small()
                            .label("Attach proof  a")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.open_attach(window, cx);
                            })),
                    )
                },
            );
        v_flex()
            .when(self.failed_uploads > 0, |v| v.child(self.failed_notice(cx)))
            .child(section_header("Proofs", meta, cx))
            .when(self.attach_open, |v| v.child(self.attach_panel(cx)))
            .child(self.gallery(cx))
            .when_some(self.note.clone(), |v, note| {
                v.child(
                    div()
                        .pt(px(8.0))
                        .text_color(p.rejection_red.to_hsla())
                        .child(note),
                )
            })
            .into_any_element()
    }
}
