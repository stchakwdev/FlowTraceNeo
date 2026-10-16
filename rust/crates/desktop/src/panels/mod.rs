//! The content panel of each nav entry. U1 ships a placeholder per entry; each screen task (U2
//! to U9) replaces one arm of [`crate::shell::Shell::panel`] with its own view (README.md, "How
//! a screen replaces its placeholder").

pub mod accounts;
pub mod general_ledger;
pub mod reports;
pub mod spend;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ClickEvent, Context, Div, FontWeight, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};

use crate::accounts::Failure;
use crate::model::{LedgerView, NavEntry};
use crate::theme::{palette, space, text};

/// The task that replaces `entry`'s placeholder (`PRIMER.md` section 8).
#[must_use]
pub fn task_of(entry: NavEntry) -> &'static str {
    match entry {
        NavEntry::Accounts => "U2",
        NavEntry::Journal => "U3",
        NavEntry::GeneralLedger => "U4",
        NavEntry::Spend => "U5 and U6",
        NavEntry::Grants => "U7",
        NavEntry::Reconcile => "U8",
        NavEntry::Reports => "U9",
    }
}

/// A page title in the `DESIGN.md` headline style.
#[must_use]
pub fn page_title(title: &str) -> impl IntoElement {
    div()
        .text_size(px(text::HEADLINE))
        .line_height(px(text::HEADLINE))
        .font_weight(FontWeight::EXTRA_BOLD)
        .child(title.to_owned())
}

/// A report section header: the 3px ink rule, then a 44px row with the section label on the
/// left and `meta` on the right, closed by a hairline (`DESIGN.md`, Layout and Cards).
pub fn section_header(label: &str, meta: impl IntoElement, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    h_flex()
        .min_h(px(space::SECTION_HEADER))
        .justify_between()
        .border_t(px(space::RULE_HEAVY))
        .border_b(px(space::HAIRLINE))
        .border_color(p.ink_navy.to_hsla())
        .child(
            div()
                .text_size(px(text::SECTION_LABEL))
                .font_weight(FontWeight::BOLD)
                .child(label.to_uppercase()),
        )
        .child(meta)
}

/// The `DESIGN.md` Demo data tag: a hairline box with "DEMO DATA" in muted letters.
pub fn demo_tag(cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    div()
        .px(px(6.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .border_1()
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::LABEL))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted.to_hsla())
        .child("DEMO DATA")
}

/// What the loaded books hold, so the placeholder shows the store is wired: counts only, every
/// one read from the snapshot.
#[must_use]
pub fn books_line(view: &LedgerView) -> String {
    match view.snapshot() {
        Some(books) => format!(
            "Books loaded: {} accounts, {} journal entries, {} periods, revision {}.",
            books.accounts.len(),
            books.entries.len(),
            books.periods.len(),
            view.revision()
        ),
        None => "No books loaded for this organisation.".to_owned(),
    }
}

/// The placeholder panel of `entry`: its title, a section naming the task that fills it, one
/// line of what it will hold, and what the loaded books hold.
pub fn placeholder(entry: NavEntry, view: &LedgerView, demo: bool, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let meta = h_flex()
        .gap(px(8.0))
        .when(demo, |row| row.child(demo_tag(cx)));
    v_flex()
        .gap(px(space::SECTION))
        .child(page_title(entry.label()))
        .child(
            v_flex()
                .child(section_header(
                    &format!("Arrives with {}", task_of(entry)),
                    meta,
                    cx,
                ))
                .child(
                    v_flex()
                        .py(px(space::CELL))
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(text::LEDE))
                                .child(entry.summary().to_owned()),
                        )
                        .child(
                            div()
                                .text_size(px(text::BODY))
                                .text_color(p.ink_muted.to_hsla())
                                .child(books_line(view)),
                        ),
                ),
        )
}

/// A one-of-many choice as a button: the chosen one filled, the others outlined.
#[must_use]
pub fn choice(id: String, label: &'static str, chosen: bool) -> Button {
    let button = Button::new(SharedString::from(id)).small().label(label);
    if chosen {
        button.primary()
    } else {
        button.outline()
    }
}

/// A labelled field: the label, the control, and the error under it in `rejection_red`.
pub fn field(label: &str, control: impl IntoElement, error: Option<String>, cx: &App) -> Div {
    let p = palette(cx.theme().mode);
    v_flex()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
        .child(control)
        .when_some(error, |f, e| {
            f.child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.rejection_red.to_hsla())
                    .child(e),
            )
        })
}

/// A muted hint under a field.
pub fn hint(text_: &str, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(palette(cx.theme().mode).ink_muted.to_hsla())
        .child(text_.to_owned())
}

/// `Tag` (`accounts.tsx:218-224`): a hairline box in small muted capitals.
pub fn tag(label: &str, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    div()
        .px(px(6.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .border_1()
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::LABEL))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted.to_hsla())
        .child(label.to_uppercase())
}

/// `IssueSummary` (`flowtrace/src/components/ledger/page.tsx:156-190`): a red-ruled box with the
/// title and each engine message as given.
pub fn failure_box(f: &Failure, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    v_flex()
        .p(px(16.0))
        .gap(px(6.0))
        .border_1()
        .border_color(p.rejection_red.to_hsla())
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.rejection_red.to_hsla())
                .child(f.title.clone()),
        )
        .children(f.messages.iter().map(|m| div().child(m.clone())))
}

/// A form's primary button and its Cancel: `submit` on the first, `cancel` then a redraw on
/// the second.
pub fn form_buttons<T: 'static>(
    id: &'static str,
    label: impl Into<SharedString>,
    submit: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
    cancel: impl Fn(&mut T) + 'static,
    cx: &mut Context<T>,
) -> Div {
    h_flex()
        .gap(px(8.0))
        .child(
            Button::new(SharedString::from(format!("{id}-submit")))
                .primary()
                .label(label)
                .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                    submit(panel, window, cx);
                })),
        )
        .child(
            Button::new(SharedString::from(format!("{id}-cancel")))
                .ghost()
                .label("Cancel")
                .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                    cancel(panel);
                    cx.notify();
                })),
        )
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use std::sync::Arc;

    use super::*;
    use crate::backend::ConnectionStatus;
    use ledger_core::demo::build_demo_ledger;

    #[test]
    fn every_entry_names_its_task() {
        let tasks: Vec<_> = NavEntry::ALL.into_iter().map(task_of).collect();
        assert_eq!(tasks, ["U2", "U3", "U4", "U5 and U6", "U7", "U8", "U9"]);
    }

    #[test]
    fn the_books_line_counts_the_snapshot() {
        let books = build_demo_ledger().unwrap();
        let line = format!(
            "Books loaded: {} accounts, {} journal entries, {} periods, revision 0.",
            books.accounts.len(),
            books.entries.len(),
            books.periods.len()
        );
        let view = LedgerView::new(
            vec![],
            ConnectionStatus::Disconnected,
            Some(Arc::new(books)),
        );
        assert_eq!(books_line(&view), line);
        let empty = LedgerView::new(vec![], ConnectionStatus::Disconnected, None);
        assert_eq!(books_line(&empty), "No books loaded for this organisation.");
    }
}
