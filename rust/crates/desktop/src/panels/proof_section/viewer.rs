//! The proof viewer (`VerificationViewer`, `verification-viewer.tsx`): the file on the left on
//! dark ink with "Proof i of n", zoom for a photo and Close; the record on the right with its
//! type, description, state, trust points, facts and "Download original". Esc closes, the arrows
//! step (the shell forwards them, [`ProofsSection::press_key`]). Desktop addition: the publish
//! action for managers.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, ClickEvent, Context, FontWeight, ImageSource, IntoElement, ObjectFit,
    ParentElement as _, Styled as _, StyledImage as _, div, img, px,
};

use super::{Loaded, ProofsSection, proof_state_mark, type_mark};
use crate::theme::{palette, space};
use crate::verification::{
    Fetch, NO_FILE, NO_PREVIEW, NOT_LOADED, OPENING, POINTS_NOTE, ProofRow, ViewerBody, file_key,
    position, publish_label, publish_note, trust_points, type_label, viewer_body, viewer_facts,
};

/// The photo area's height at 100%.
const IMAGE_HEIGHT: f32 = 640.0;

impl ProofsSection {
    /// The viewer on proof `index`.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's viewer element by element, read top to bottom"
    )]
    pub(super) fn viewer(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(v) = self.proofs.get(index).cloned() else {
            return div().into_any_element();
        };
        let row = ProofRow::of(&v);
        let loaded = file_key(&v).and_then(|k| self.images.get(k)).cloned();
        let fetch = match &loaded {
            Some(Loaded::Ready(_)) => Fetch::Ready,
            Some(Loaded::Failed(why)) => Fetch::Failed(why.clone()),
            Some(Loaded::Loading) | None => Fetch::Pending,
        };
        let body = viewer_body(&v, &fetch);
        let ink = p.masthead_paper.to_hsla();
        let message = |title: String, line: Option<&str>| {
            v_flex()
                .max_w(px(380.0))
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(28.0))
                        .font_weight(FontWeight::BOLD)
                        .child(type_mark(v.kind, row.media)),
                )
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .when_some(line.map(str::to_owned), |m, l| {
                    m.child(div().text_size(px(13.0)).opacity(0.8).child(l))
                })
                .into_any_element()
        };
        let file_area = match &body {
            ViewerBody::Opening => message(OPENING.to_owned(), None),
            ViewerBody::Failed(why) => message(NOT_LOADED.to_owned(), Some(why)),
            ViewerBody::NoPreview => message(type_label(v.kind).to_owned(), Some(NO_PREVIEW)),
            ViewerBody::NoFile => message(type_label(v.kind).to_owned(), Some(NO_FILE)),
            ViewerBody::Image => match loaded {
                Some(Loaded::Ready(image)) => div()
                    .h(px(IMAGE_HEIGHT * self.zoom.0))
                    .w(px(IMAGE_HEIGHT * self.zoom.0))
                    .child(
                        img(ImageSource::Image(Arc::clone(&image)))
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    )
                    .into_any_element(),
                _ => message(OPENING.to_owned(), None),
            },
        };
        let count = self.proofs.len();
        let zoom = (body == ViewerBody::Image).then(|| {
            h_flex()
                .gap(px(4.0))
                .child(
                    Button::new("viewer-zoom-out")
                        .ghost()
                        .small()
                        .label("Zoom out  -")
                        .disabled(!self.zoom.can_out())
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.zoom = this.zoom.out();
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("viewer-zoom-reset")
                        .ghost()
                        .small()
                        .label(self.zoom.percent())
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.zoom = crate::verification::Zoom::default();
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("viewer-zoom-in")
                        .ghost()
                        .small()
                        .label("Zoom in  =")
                        .disabled(!self.zoom.can_in())
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.zoom = this.zoom.into();
                            cx.notify();
                        })),
                )
        });
        let top = h_flex()
            .h(px(56.0))
            .px(px(16.0))
            .justify_between()
            .child(div().text_size(px(13.0)).children(position(index, count)))
            .child(
                h_flex().gap(px(8.0)).children(zoom).child(
                    Button::new("viewer-close")
                        .outline()
                        .small()
                        .label("Close proof viewer  Esc")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.close_viewer(cx);
                        })),
                ),
            );
        let steps = h_flex()
            .px(px(16.0))
            .pb(px(16.0))
            .justify_between()
            .child(
                Button::new("viewer-prev")
                    .ghost()
                    .small()
                    .label("Previous proof  \u{2190}")
                    .disabled(index == 0)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.step(false, cx))),
            )
            .child(
                Button::new("viewer-next")
                    .ghost()
                    .small()
                    .label("Next proof  \u{2192}")
                    .disabled(index + 1 >= count)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.step(true, cx))),
            );
        let left = v_flex()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(IMAGE_HEIGHT + 120.0))
            .bg(p.ink_navy.to_hsla())
            .text_color(ink)
            .child(top)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .px(px(32.0))
                    .child(file_area),
            )
            .when(count > 1, |l| l.child(steps));
        h_flex()
            .items_stretch()
            .border_1()
            .border_color(p.ink_navy.to_hsla())
            .child(left)
            .child(self.record_panel(&v, &row, cx))
            .into_any_element()
    }

    /// The record on the right.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's record panel fact by fact, read top to bottom"
    )]
    fn record_panel(
        &self,
        v: &ledger_module::transparency::types::Verification,
        row: &ProofRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        let facts: Vec<AnyElement> = viewer_facts(v)
            .into_iter()
            .map(|f| {
                h_flex()
                    .py(px(10.0))
                    .gap(px(12.0))
                    .items_start()
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        div()
                            .w(px(120.0))
                            .flex_shrink_0()
                            .text_color(p.ink_muted.to_hsla())
                            .child(f.term),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(div().font_weight(FontWeight::MEDIUM).child(f.value))
                            .when_some(f.note, |c, n| {
                                c.child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.ink_muted.to_hsla())
                                        .child(n),
                                )
                            }),
                    )
                    .into_any_element()
            })
            .collect();
        let publish = publish_label(v, self.manager).map(|label| {
            v_flex()
                .gap(px(6.0))
                .child(
                    Button::new("viewer-publish")
                        .outline()
                        .label(format!("{label}  p"))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.toggle_published(cx);
                        })),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(publish_note(v.published)),
                )
        });
        v_flex()
            .w(px(384.0))
            .flex_shrink_0()
            .p(px(20.0))
            .gap(px(4.0))
            .bg(p.sheet.to_hsla())
            .child(
                div()
                    .text_size(px(24.0))
                    .font_weight(FontWeight::BOLD)
                    .child(row.label),
            )
            .when_some(row.description.clone(), |c, d| {
                c.child(div().text_color(p.ink_muted.to_hsla()).child(d))
            })
            .child(div().pt(px(8.0)).child(proof_state_mark(row, p)))
            .child(
                div()
                    .pt(px(16.0))
                    .text_size(px(36.0))
                    .font_weight(FontWeight::BOLD)
                    .child(format!("+{}", trust_points(v))),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(POINTS_NOTE),
            )
            .child(
                v_flex()
                    .mt(px(16.0))
                    .border_t(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .children(facts),
            )
            .when(file_key(v).is_some(), |c| {
                c.child(
                    div().pt(px(16.0)).child(
                        Button::new("viewer-download")
                            .outline()
                            .w_full()
                            .label("Download original  d")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.download(cx);
                            })),
                    ),
                )
            })
            .when_some(publish, |c, b| c.child(div().pt(px(16.0)).child(b)))
            .when_some(self.note.clone(), |c, note| {
                c.child(
                    div()
                        .pt(px(12.0))
                        .text_size(px(12.0))
                        .text_color(p.rejection_red.to_hsla())
                        .child(note),
                )
            })
            .into_any_element()
    }
}
