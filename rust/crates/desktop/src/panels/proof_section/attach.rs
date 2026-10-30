//! The attach panel of the Proofs section (`AttachProof`, `transactions/$id.tsx:415-584`):
//! kind of proof, files uploaded as soon as they are chosen (`FileUpload`), what it shows, who
//! captured it, who may see it and a typed GPS reading (`EnhancedUploadForm`); one proof record
//! per stored file, or one record without a file for a GPS reading (`useAttachProofs`).

use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, FontWeight, IntoElement,
    ParentElement as _, PathPromptOptions, SharedString, Styled as _, Subscription, Window, div,
    px,
};
use ledger_module::transparency::types::{VerificationType, Visibility};

use super::ProofsSection;
use crate::panels::bills::new_pick;
use crate::panels::dashboard::small_muted;
use crate::panels::{Pick, PickState, field, hint};
use crate::proofs::{AttachState, SCREEN_MAX_BYTES, new_verification_for};
use crate::store::LedgerStore;
use crate::theme::{palette, space};
use crate::transactions::{FAILED_UPLOADS_BODY, failed_uploads_title};
use crate::verification::{
    AttachForm, KIND_LOCKED, MAX_FILES, SCREEN_MAX_MB, VISIBILITY_ORDER, attach_choices,
    attach_failure, attach_label, file_row_status, read_proof_file, upload_failure, upload_hint,
    visibility_description, visibility_label,
};

/// The "Kind of proof" and "Who can see the file" selects.
pub(super) fn picks(
    window: &mut Window,
    cx: &mut Context<ProofsSection>,
) -> (PickState, PickState) {
    let kinds: Vec<Pick> = attach_choices()
        .into_iter()
        .map(|(k, label)| Pick::new(kind_value(k), label))
        .collect();
    let levels: Vec<Pick> = VISIBILITY_ORDER
        .iter()
        .map(|v| Pick::new(visibility_value(*v), visibility_label(*v)))
        .collect();
    (
        new_pick(window, cx, kinds, Some("receipt")),
        new_pick(window, cx, levels, Some("full")),
    )
}

/// The module's word for a kind, as its JSON says it.
fn kind_value(kind: VerificationType) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn visibility_value(level: Visibility) -> String {
    serde_json::to_value(level)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub(super) fn subscriptions(
    store: &Entity<LedgerStore>,
    kind_pick: &PickState,
    visibility_pick: &PickState,
    window: &mut Window,
    cx: &mut Context<ProofsSection>,
) -> Vec<Subscription> {
    vec![
        cx.observe(store, |this, _, cx| this.refresh(cx)),
        cx.subscribe_in(
            kind_pick,
            window,
            |this, _, event: &SelectEvent<Vec<Pick>>, _, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Ok(kind) = serde_json::from_value(serde_json::json!(value)) {
                    this.form.kind = kind;
                    this.problem = None;
                    cx.notify();
                }
            },
        ),
        cx.subscribe_in(
            visibility_pick,
            window,
            |this, _, event: &SelectEvent<Vec<Pick>>, _, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Ok(level) = serde_json::from_value(serde_json::json!(value)) {
                    this.form.visibility = level;
                    cx.notify();
                }
            },
        ),
    ]
}

impl ProofsSection {
    /// Opens the panel with the web's defaults: a receipt, shown in full, nothing typed.
    pub(super) fn open_attach(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.attach_open = true;
        self.problem = None;
        self.form = AttachForm::default();
        self.files = crate::proofs::AttachList::default();
        self.kind_pick.update(cx, |s, cx| {
            s.set_selected_value(&"receipt".to_owned(), window, cx);
        });
        self.visibility_pick.update(cx, |s, cx| {
            s.set_selected_value(&"full".to_owned(), window, cx);
        });
        for input in [
            &self.description,
            &self.executor,
            &self.latitude,
            &self.longitude,
            &self.accuracy,
            &self.place,
        ] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        cx.notify();
    }

    pub(super) fn close_attach(&mut self, cx: &mut Context<Self>) {
        self.attach_open = false;
        self.problem = None;
        cx.notify();
    }

    /// The platform chooser, then each file read and checked as the web does (kind's types, 10
    /// MB, five at once) and uploaded on the background executor.
    pub(super) fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((link, link_id)) = self.link.clone() else {
            return;
        };
        let (client, org) = match self.store.read(cx).proofs_client() {
            Ok(found) => found,
            Err(e) => {
                self.problem = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let kind = self.form.kind;
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Choose proof files".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let paths: Vec<PathBuf> = match chosen.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(e)) => {
                    let _ = view.update(cx, |this, cx| {
                        this.problem = Some(format!("The file chooser did not open: {e}"));
                        cx.notify();
                    });
                    return;
                }
            };
            for path in paths {
                let Ok(room) = view.update(cx, |this, _| this.files.rows().len() < MAX_FILES)
                else {
                    return;
                };
                if !room {
                    break;
                }
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let size = std::fs::metadata(&path).map_or(0, |m| m.len());
                let Ok(id) = view.update(cx, |this, cx| {
                    cx.notify();
                    this.files.start_sized(&name, size)
                }) else {
                    return;
                };
                let (client, org, link_id) = (client.clone(), org.clone(), link_id.clone());
                let outcome = cx
                    .background_spawn(async move {
                        let file = read_proof_file(&path, kind, SCREEN_MAX_MB)?;
                        debug_assert!(file.bytes.len() as u64 <= SCREEN_MAX_BYTES);
                        client
                            .upload(org.as_str(), link, &link_id, &file)
                            .map_err(|e| upload_failure(&file.name, &e.to_string()))
                    })
                    .await;
                let _ = view.update(cx, |this, cx| {
                    match outcome {
                        Ok(uploaded) => this.files.finish(id, Ok(uploaded)),
                        Err(message) => this.files.fail(id, message),
                    }
                    this.problem = None;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Records one proof per stored file, or one record without a file for a GPS reading.
    pub(super) fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.files.busy() {
            return;
        }
        let Some((link, link_id)) = self.link.clone() else {
            return;
        };
        let read = |i: &Entity<gpui_kit::component::input::InputState>, cx: &Context<Self>| {
            i.read(cx).value().to_string()
        };
        self.form.description = read(&self.description, cx);
        self.form.executor = read(&self.executor, cx);
        self.form.latitude = read(&self.latitude, cx);
        self.form.longitude = read(&self.longitude, cx);
        self.form.accuracy = read(&self.accuracy, cx);
        self.form.place = read(&self.place, cx);
        let ready = self.files.ready();
        let details = match self.form.check(ready.len()) {
            Ok(details) => details,
            Err(problem) => {
                self.problem = Some(problem);
                cx.notify();
                return;
            }
        };
        let records: Vec<_> = if ready.is_empty() {
            vec![new_verification_for(link, &link_id, &details, None)]
        } else {
            ready
                .iter()
                .map(|a| new_verification_for(link, &link_id, &details, Some(a)))
                .collect()
        };
        self.problem = None;
        self.saving = true;
        let submissions: Vec<_> = records
            .into_iter()
            .map(|r| self.store.update(cx, |s, _| s.attach_proof(r)))
            .collect();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut failure = None;
            for submission in submissions {
                if let Err(e) = submission.result().await {
                    failure.get_or_insert(e.to_string());
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                if let Some(message) = failure {
                    this.problem = Some(attach_failure(&message));
                } else {
                    this.attach_open = false;
                    this.failed_uploads = 0;
                    this.files = crate::proofs::AttachList::default();
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The failed-uploads notice of a new transaction (`$id.tsx:166-181`).
    pub(super) fn failed_notice(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .mb(px(space::SECTION))
            .p(px(16.0))
            .gap(px(6.0))
            .border_1()
            .border_color(p.rejection_red.to_hsla())
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.rejection_red.to_hsla())
                    .child(failed_uploads_title(self.failed_uploads)),
            )
            .child(div().child(FAILED_UPLOADS_BODY))
            .child(
                div().pt(px(6.0)).child(
                    Button::new("proofs-attach-again")
                        .outline()
                        .label("Attach proof again")
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.open_attach(window, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    /// The panel itself.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's attach panel field by field, read top to bottom"
    )]
    pub(super) fn attach_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let locked = !self.files.rows().is_empty();
        let ready = self.files.ready().len();
        let rows: Vec<AnyElement> = self
            .files
            .rows()
            .iter()
            .map(|r| {
                let (status, failed) = match &r.state {
                    AttachState::Uploading => (file_row_status(r.size, true, None), false),
                    AttachState::Ready(a) => (file_row_status(a.uploaded.size, false, None), false),
                    AttachState::Failed(why) => (file_row_status(r.size, false, Some(why)), true),
                };
                let id = r.id;
                let uploading = r.state == AttachState::Uploading;
                h_flex()
                    .py(px(8.0))
                    .gap(px(12.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(div().font_weight(FontWeight::MEDIUM).child(r.name.clone()))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(if failed {
                                        p.rejection_red.to_hsla()
                                    } else {
                                        p.ink_muted.to_hsla()
                                    })
                                    .child(status),
                            ),
                    )
                    .child(
                        Button::new(SharedString::from(format!("proof-file-remove-{id}")))
                            .ghost()
                            .small()
                            .label(format!("Remove {}", r.name))
                            .disabled(uploading || self.saving)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.files.remove(id);
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        let full = self.files.rows().len() >= MAX_FILES;
        let dropzone = h_flex()
            .p(px(16.0))
            .gap(px(12.0))
            .border_2()
            .border_dashed()
            .border_color(p.field_stroke.to_hsla())
            .child(
                v_flex()
                    .flex_1()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Choose files to attach"),
                    )
                    .child(small_muted(
                        &upload_hint(self.form.kind, self.files.rows().len()),
                        p,
                    )),
            )
            .child(
                Button::new("proof-browse")
                    .outline()
                    .label("Browse files")
                    .disabled(full || self.saving)
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.choose(window, cx);
                    })),
            );
        let gps = v_flex()
            .gap(px(6.0))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("GPS reading"))
            .child(hint(
                "Type the place: the desktop cannot read its own location.",
                cx,
            ))
            .child(
                h_flex()
                    .gap(px(8.0))
                    .child(div().w(px(160.0)).child(Input::new(&self.latitude)))
                    .child(div().w(px(160.0)).child(Input::new(&self.longitude)))
                    .child(div().w(px(140.0)).child(Input::new(&self.accuracy)))
                    .child(div().flex_1().child(Input::new(&self.place))),
            );
        v_flex()
            .my(px(16.0))
            .p(px(20.0))
            .gap(px(18.0))
            .border_1()
            .border_color(p.field_stroke.to_hsla())
            .bg(p.sheet.to_hsla())
            .child(
                h_flex()
                    .justify_between()
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(24.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Attach proof"),
                    )
                    .child(small_muted(&self.form.points_line(), p)),
            )
            .child(
                field(
                    "Kind of proof",
                    div()
                        .w(px(360.0))
                        .child(Select::new(&self.kind_pick).disabled(locked || self.saving)),
                    None,
                    cx,
                )
                .when(locked, |f| f.child(hint(KIND_LOCKED, cx))),
            )
            .child(dropzone)
            .when(!rows.is_empty(), |v| v.child(v_flex().children(rows)))
            .child(field(
                "What it shows",
                Input::new(&self.description),
                None,
                cx,
            ))
            .child(
                h_flex()
                    .gap(px(16.0))
                    .items_start()
                    .child(div().flex_1().child(field(
                        "Captured by",
                        Input::new(&self.executor),
                        None,
                        cx,
                    )))
                    .child(
                        div().flex_1().child(
                            field(
                                "Who can see the file",
                                Select::new(&self.visibility_pick).disabled(self.saving),
                                None,
                                cx,
                            )
                            .child(hint(visibility_description(self.form.visibility), cx)),
                        ),
                    ),
            )
            .child(gps)
            .when_some(self.problem.clone(), |v, problem| {
                v.child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.rejection_red.to_hsla())
                        .child(problem),
                )
            })
            .child(
                h_flex()
                    .pt(px(16.0))
                    .gap(px(8.0))
                    .justify_end()
                    .border_t(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        Button::new("proof-attach-cancel")
                            .ghost()
                            .label("Cancel")
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.close_attach(cx);
                            })),
                    )
                    .child(
                        Button::new("proof-attach-save")
                            .primary()
                            .label(attach_label(self.saving, ready))
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.save(cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}
