//! The create and edit group dialog of the Access groups screen (`GroupFormDialog`,
//! `access-groups.tsx:913-1258`), and the widget wiring the screen's forms share.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Entity, FontWeight, IntoElement, ParentElement as _,
    SharedString, Styled as _, Subscription, Window, div, px,
};
use ledger_module::transparency::types::{AccessGroup, NodeScope};

use super::{AccessGroupsPanel, Dialog, dialog_title, error_line};
use crate::access_groups::{
    DO_FLAGS, Flag, GroupForm, ProofDepth, SCOPES, SEE_FLAGS, SLUG_HINT, scope_hint, scope_label,
};
use crate::backend::SettingsWrite;
use crate::panels::bills::picked;
use crate::panels::{Pick, PickState, field, hint};
use crate::theme::{palette, space, text};

/// Milliseconds since the Unix epoch now, the web's `new Date()`.
pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The value a scope is picked by.
const fn scope_value(s: NodeScope) -> &'static str {
    match s {
        NodeScope::All => "all",
        NodeScope::Assigned => "assigned",
        NodeScope::Downstream => "downstream",
    }
}

/// The value a proof depth is picked by.
const fn depth_value(d: ProofDepth) -> &'static str {
    match d {
        ProofDepth::None => "none",
        ProofDepth::Summary => "summary",
        ProofDepth::Full => "full",
    }
}

/// Fills the Which flows and Proofs selects.
pub(super) fn load_choices(
    scope: &PickState,
    proofs: &PickState,
    window: &mut Window,
    cx: &mut App,
) {
    let scopes: Vec<Pick> = SCOPES
        .iter()
        .map(|s| Pick::new(scope_value(*s), scope_label(*s)))
        .collect();
    let depths: Vec<Pick> = ProofDepth::ALL
        .iter()
        .map(|d| Pick::new(depth_value(*d), d.label()))
        .collect();
    scope.update(cx, |s, cx| s.set_items(scopes, window, cx));
    proofs.update(cx, |s, cx| s.set_items(depths, window, cx));
}

/// The screen's widget subscriptions: the group form's name, short id and selects, and the
/// invite fields.
#[expect(clippy::too_many_arguments, reason = "the six widgets the screen owns")]
pub(super) fn subscribe(
    name: &Entity<InputState>,
    slug: &Entity<InputState>,
    scope: &PickState,
    proofs: &PickState,
    email: &Entity<InputState>,
    until: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<AccessGroupsPanel>,
) -> Vec<Subscription> {
    vec![
        cx.subscribe_in(
            name,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let typed = input.read(cx).value().to_string();
                    this.group.set_name(&typed);
                    if !this.group.slug_edited {
                        let slug = this.group.slug.clone();
                        this.group_slug
                            .update(cx, |i, cx| i.set_value(slug, window, cx));
                    }
                    cx.notify();
                }
            },
        ),
        cx.subscribe_in(
            slug,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let typed = input.read(cx).value().to_string();
                    this.group.set_slug(&typed);
                    if this.group.slug != typed {
                        let kept = this.group.slug.clone();
                        input.update(cx, |i, cx| i.set_value(kept, window, cx));
                    }
                    cx.notify();
                }
            },
        ),
        cx.subscribe_in(
            scope,
            window,
            |this, pick, _: &SelectEvent<Vec<Pick>>, _, cx| {
                if let Some(s) =
                    picked(pick, cx).and_then(|v| SCOPES.into_iter().find(|s| scope_value(*s) == v))
                {
                    this.group.scope = s;
                }
                cx.notify();
            },
        ),
        cx.subscribe_in(
            proofs,
            window,
            |this, pick, _: &SelectEvent<Vec<Pick>>, _, cx| {
                if let Some(d) = picked(pick, cx)
                    .and_then(|v| ProofDepth::ALL.into_iter().find(|d| depth_value(*d) == v))
                {
                    this.group.permissions = d.apply(this.group.permissions);
                }
                cx.notify();
            },
        ),
        cx.subscribe_in(email, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.last_invited = None;
                cx.notify();
            }
        }),
        cx.subscribe_in(until, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        }),
    ]
}

impl AccessGroupsPanel {
    /// Opens the group dialog for `group`, or for a new group.
    pub(super) fn open_group_form(
        &mut self,
        group: Option<&AccessGroup>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.group = group.map_or_else(GroupForm::new, GroupForm::edit);
        self.group_submitted = false;
        let f = self.group.clone();
        self.group_name
            .update(cx, |i, cx| i.set_value(f.name.clone(), window, cx));
        self.group_slug
            .update(cx, |i, cx| i.set_value(f.slug.clone(), window, cx));
        self.group_description
            .update(cx, |i, cx| i.set_value(f.description.clone(), window, cx));
        let scope = scope_value(f.scope).to_owned();
        self.scope_pick
            .update(cx, |s, cx| s.set_selected_value(&scope, window, cx));
        let depth = depth_value(ProofDepth::of(&f.permissions)).to_owned();
        self.proofs_pick
            .update(cx, |s, cx| s.set_selected_value(&depth, window, cx));
        self.open_dialog(Dialog::Group, cx);
        self.group_name.update(cx, |i, cx| i.focus(window, cx));
    }

    /// "Create group" or "Save group" (`onSubmit`): checks, then creates or saves.
    fn submit_group(&mut self, cx: &mut Context<Self>) {
        self.group_submitted = true;
        self.group.description = self.group_description.read(cx).value().to_string();
        if !self.group.validate(&self.groups).is_empty() {
            cx.notify();
            return;
        }
        self.dialog_error = None;
        let write = match &self.group.editing {
            Some(id) => SettingsWrite::UpdateGroup {
                id: id.clone(),
                patch: self.group.patch(),
            },
            None => SettingsWrite::CreateGroup(self.group.new_group()),
        };
        let editing = self.group.editing.clone();
        let slug = self.group.slug.clone();
        self.send(write, cx, move |this, r, _| match r {
            Ok(()) => {
                // `onSaved(id)`: the saved group is selected; a new one once its row arrives,
                // found by its short id.
                match editing {
                    Some(id) => this.selected = Some(id),
                    None => match this.groups.iter().find(|g| g.slug == slug) {
                        Some(g) => this.selected = Some(g.id.clone()),
                        None => this.select_slug = Some(slug),
                    },
                }
                this.dialog = None;
            }
            Err(e) => this.dialog_error = Some(format!("Not saved. {e}")),
        });
    }

    fn flag(
        &self,
        (flag, label, hint_): (Flag, &'static str, &'static str),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .min_h(px(40.0))
            .py(px(6.0))
            .gap(px(12.0))
            .items_start()
            .child(
                Checkbox::new(SharedString::from(format!("flag-{label}")))
                    .checked(flag.get(&self.group.permissions))
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.group.permissions = flag.set(this.group.permissions, *checked);
                        cx.notify();
                    })),
            )
            .child(
                v_flex()
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(hint_),
                    ),
            )
            .into_any_element()
    }

    /// The dialog's body.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's dialog, read top to bottom"
    )]
    pub(super) fn group_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let f = &self.group;
        let errors = if self.group_submitted {
            f.validate(&self.groups)
        } else {
            crate::access_groups::GroupErrors::default()
        };
        let slug_line = errors.slug.map_or_else(
            || hint(SLUG_HINT, cx).into_any_element(),
            |e| error_line(e, p).into_any_element(),
        );
        let see: Vec<AnyElement> = SEE_FLAGS.iter().map(|x| self.flag(*x, cx)).collect();
        let act: Vec<AnyElement> = DO_FLAGS.iter().map(|x| self.flag(*x, cx)).collect();
        let depth = ProofDepth::of(&f.permissions);
        let slug_input = Input::new(&self.group_slug).disabled(f.is_default);
        v_flex()
            .gap(px(20.0))
            .child(
                v_flex().gap(px(6.0)).child(dialog_title(&f.title())).child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(f.description_text()),
                ),
            )
            .child(
                h_flex()
                    .gap(px(16.0))
                    .items_start()
                    .child(div().flex_1().child(field(
                        "Name",
                        Input::new(&self.group_name),
                        errors.name.map(str::to_owned),
                        cx,
                    )))
                    .child(
                        div()
                            .flex_1()
                            .child(field("Short ID", slug_input, None, cx).child(slug_line)),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(6.0))
                    .child(
                        h_flex()
                            .gap(px(4.0))
                            .text_size(px(text::BODY))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Description"))
                            .child(div().text_color(p.ink_muted.to_hsla()).child("(optional)")),
                    )
                    .child(Input::new(&self.group_description)),
            )
            .child(
                v_flex()
                    .gap(px(8.0))
                    .pt(px(16.0))
                    .border_t(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("What they see"),
                    )
                    .child(
                        h_flex()
                            .gap(px(16.0))
                            .items_start()
                            .child(
                                div().flex_1().child(
                                    field("Which flows", Select::new(&self.scope_pick), None, cx)
                                        .child(hint(scope_hint(f.scope), cx)),
                                ),
                            )
                            .child(
                                div().flex_1().child(
                                    field("Proofs", Select::new(&self.proofs_pick), None, cx)
                                        .child(hint(depth.hint(), cx)),
                                ),
                            ),
                    )
                    .child(h_flex().flex_wrap().gap(px(16.0)).children(see)),
            )
            .child(
                v_flex()
                    .gap(px(8.0))
                    .pt(px(16.0))
                    .border_t(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("What they can do"),
                    )
                    .child(h_flex().flex_wrap().gap(px(16.0)).children(act)),
            )
            .when_some(self.dialog_error.clone(), |d, e| d.child(error_line(&e, p)))
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::new("group-cancel")
                            .outline()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    )
                    .child(
                        Button::new("group-submit")
                            .primary()
                            .label(f.submit_label(self.pending))
                            .disabled(self.pending)
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _, cx| this.submit_group(cx)),
                            ),
                    ),
            )
            .into_any_element()
    }
}
