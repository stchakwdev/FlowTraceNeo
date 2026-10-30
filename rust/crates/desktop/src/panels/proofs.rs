//! "Attach receipt" on the claim form and "Attach proof" on the bill form, and the recorded
//! proofs on a claim's or bill's page. The rules live in [`crate::proofs`]; this file picks
//! files with the platform chooser (`App::prompt_for_paths`, as the journal import does), runs
//! the Worker calls on GPUI's background executor, and draws the lists.
//!
//! Files upload as soon as they are chosen (the web's `FileUpload`,
//! `flowtrace/src/components/file-upload.tsx`); once the claim or bill is saved and has an id,
//! [`record`] writes each one's key and hash to the books. Opening a recorded proof asks the
//! Worker for a ten-minute link and hands it to the system browser (`App::open_url`).

use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, IntoElement, ParentElement as _,
    PathPromptOptions, SharedString, Styled as _, Window, div, px,
};
use ledger_module::transparency::types::{Verification, Visibility};

use crate::panels::section_header;
use crate::proofs::{AttachList, AttachState, LinkKind, ProofsError, new_verification, read_file};
use crate::store::LedgerStore;
use crate::theme::{palette, space};

/// A screen with a form that attaches proof files.
pub(crate) trait HasProofs: Sized + 'static {
    /// The store the proofs go through.
    fn proofs_store(&self) -> &Entity<LedgerStore>;
    /// The files on the open form.
    fn attachments(&mut self) -> &mut AttachList;
    /// A line about the last proof action that went wrong, shown under the list.
    fn proof_note(&mut self) -> &mut Option<String>;
}

/// The platform chooser, then one upload per chosen file, each on the background executor.
/// `link_id` is the claim or bill id when it has one, else `new` (the Worker keeps it only as a
/// note; the books get the real id in [`record`]).
pub(crate) fn choose<T: HasProofs>(
    this: &mut T,
    kind: LinkKind,
    link_id: String,
    window: &mut Window,
    cx: &mut Context<T>,
) {
    let (client, org) = match this.proofs_store().read(cx).proofs_client() {
        Ok(found) => found,
        Err(e) => {
            *this.proof_note() = Some(e.to_string());
            cx.notify();
            return;
        }
    };
    *this.proof_note() = None;
    let chosen = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(
            match kind {
                LinkKind::Claim => "Choose receipts",
                LinkKind::Bill | LinkKind::Transaction => "Choose proof files",
            }
            .into(),
        ),
    });
    cx.spawn_in(window, async move |view, cx| {
        let paths: Vec<PathBuf> = match chosen.await {
            Ok(Ok(Some(paths))) => paths,
            Ok(Ok(None)) | Err(_) => return,
            Ok(Err(e)) => {
                let _ = view.update(cx, |this, cx| {
                    *this.proof_note() = Some(format!("The file chooser did not open: {e}"));
                    cx.notify();
                });
                return;
            }
        };
        for path in paths {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            let Ok(id) = view.update(cx, |this, cx| {
                cx.notify();
                this.attachments().start(&name)
            }) else {
                return;
            };
            let client = client.clone();
            let org = org.clone();
            let link_id = link_id.clone();
            let outcome = cx
                .background_spawn(async move {
                    let file = read_file(&path)?;
                    client.upload(org.as_str(), kind, &link_id, &file)
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                this.attachments().finish(id, outcome);
                cx.notify();
            });
        }
    })
    .detach();
}

/// Records every stored file on the form against the saved claim or bill `id` and clears the
/// list. A refusal goes to [`HasProofs::proof_note`]; the claim or bill itself is saved.
pub(crate) fn record<T: HasProofs>(this: &mut T, kind: LinkKind, id: &str, cx: &mut Context<T>) {
    let ready = std::mem::take(this.attachments()).ready();
    if ready.is_empty() {
        return;
    }
    let store = this.proofs_store().clone();
    let submissions: Vec<_> = ready
        .iter()
        .map(|a| {
            let proof = new_verification(kind, id, a);
            (
                a.name.clone(),
                store.update(cx, |s, _| s.attach_proof(proof)),
            )
        })
        .collect();
    cx.spawn(async move |view, cx| {
        let mut failed = Vec::new();
        for (name, submission) in submissions {
            if let Err(e) = submission.result().await {
                failed.push(format!("{name}: {e}"));
            }
        }
        let _ = view.update(cx, |this, cx| {
            if !failed.is_empty() {
                *this.proof_note() = Some(format!(
                    "Saved, but these files were not recorded on it: {}",
                    failed.join("; ")
                ));
            }
            cx.notify();
        });
    })
    .detach();
}

/// Asks the Worker for a ten-minute link to `key` and opens it in the system browser.
pub(crate) fn open<T: HasProofs>(this: &mut T, key: String, cx: &mut Context<T>) {
    let client = match this.proofs_store().read(cx).proofs_client() {
        Ok((client, _)) => client,
        Err(e) => {
            *this.proof_note() = Some(e.to_string());
            cx.notify();
            return;
        }
    };
    cx.spawn(async move |view, cx| {
        let outcome: Result<String, ProofsError> = cx
            .background_spawn(async move { client.signed_url(&key) })
            .await;
        let _ = view.update(cx, |this, cx| {
            match outcome {
                Ok(url) => {
                    *this.proof_note() = None;
                    cx.open_url(&url);
                }
                Err(e) => *this.proof_note() = Some(e.to_string()),
            }
            cx.notify();
        });
    })
    .detach();
}

/// The files on a form: name, where each stands, Remove, and the attach button.
pub(crate) fn attach_section<T: HasProofs>(
    title: &str,
    button: &'static str,
    rows: &AttachList,
    note: Option<&str>,
    on_attach: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
    cx: &mut Context<T>,
) -> AnyElement {
    let p = palette(cx.theme().mode);
    let items: Vec<AnyElement> = rows
        .rows()
        .iter()
        .map(|r| {
            let (status, colour) = match &r.state {
                AttachState::Uploading => ("Uploading...".to_owned(), p.ink_muted),
                AttachState::Ready(a) => (
                    format!("{} KB, ready", a.uploaded.size.div_ceil(1024)),
                    p.ink_muted,
                ),
                AttachState::Failed(why) => (why.clone(), p.rejection_red),
            };
            let id = r.id;
            h_flex()
                .py(px(8.0))
                .gap(px(12.0))
                .border_b_1()
                .border_color(p.hairline.to_hsla())
                .child(div().flex_1().child(r.name.clone()))
                .child(div().text_color(colour.to_hsla()).child(status))
                .child(
                    Button::new(SharedString::from(format!("proof-remove-{id}")))
                        .ghost()
                        .small()
                        .label("Remove")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.attachments().remove(id);
                            cx.notify();
                        })),
                )
                .into_any_element()
        })
        .collect();
    v_flex()
        .child(section_header(title, div(), cx))
        .children(items)
        .child(
            h_flex()
                .py(px(space::CELL))
                .gap(px(12.0))
                .child(
                    Button::new(SharedString::from(format!("proof-attach-{button}")))
                        .outline()
                        .label(button)
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            on_attach(this, window, cx);
                        })),
                )
                .child(
                    div()
                        .text_color(p.ink_muted.to_hsla())
                        .child("Photos (JPEG, PNG, WebP, GIF, HEIC) or PDF, up to 25 MB each."),
                ),
        )
        .when_some(note.map(str::to_owned), |v, note| {
            v.child(div().text_color(p.rejection_red.to_hsla()).child(note))
        })
        .into_any_element()
}

/// The proofs recorded on a claim or bill, each with Open when the viewer may see the file.
pub(crate) fn recorded_section<T: HasProofs>(
    title: &str,
    proofs: &[Verification],
    note: Option<&str>,
    cx: &mut Context<T>,
) -> AnyElement {
    let p = palette(cx.theme().mode);
    let items: Vec<AnyElement> = proofs
        .iter()
        .map(|v| {
            let name = v
                .metadata
                .get("file_name")
                .and_then(|n| n.as_str())
                .map(str::to_owned)
                .or_else(|| v.description.clone())
                .unwrap_or_else(|| v.id.clone());
            let key = match v.visible_as {
                Some(Visibility::Thumbnail) => v.thumbnail_key.clone(),
                _ => v.object_key.clone(),
            };
            let row = h_flex()
                .py(px(8.0))
                .gap(px(12.0))
                .border_b_1()
                .border_color(p.hairline.to_hsla())
                .child(div().flex_1().child(name));
            match key {
                Some(key) => row.child(
                    Button::new(SharedString::from(format!("proof-open-{}", v.id)))
                        .outline()
                        .small()
                        .label("Open")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            open(this, key.clone(), cx);
                        })),
                ),
                None => row.child(
                    div()
                        .text_color(p.ink_muted.to_hsla())
                        .child("Your access shows that it is attached, not the file."),
                ),
            }
            .into_any_element()
        })
        .collect();
    v_flex()
        .child(section_header(title, div(), cx))
        .when(items.is_empty(), |v| {
            v.child(
                div()
                    .py(px(8.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child("No files recorded on it."),
            )
        })
        .children(items)
        .when_some(note.map(str::to_owned), |v, note| {
            v.child(div().text_color(p.rejection_red.to_hsla()).child(note))
        })
        .into_any_element()
}
