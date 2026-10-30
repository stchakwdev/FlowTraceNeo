//! The new-campaign form of the Campaigns screen (`campaign-create-modal.tsx`), drawn inline as
//! the desktop's other add forms are.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Entity, IntoElement, ParentElement as _, Styled as _,
    Window, div, px,
};

use super::{CampaignsPanel, CampaignsView};
use crate::backend::{CampaignWrite, iso_millis};
use crate::campaigns::{
    CATEGORIES, CURRENCIES, CampaignErrors, NewCampaignForm, not_saved, project_choices,
};
use crate::panels::bills::{picked, picks};
use crate::panels::dashboard::small_muted;
use crate::panels::{Pick, PickState, field, hint, page_title, section_header};
use crate::statements::today_utc;
use crate::theme::{palette, space, text};

/// Fills `select` with `items`, choosing the one whose value is `value`.
fn load(
    select: &PickState,
    items: Vec<Pick>,
    value: Option<&str>,
    window: &mut Window,
    cx: &mut App,
) {
    let index = value
        .and_then(|v| items.iter().position(|p| p.value == v))
        .map(IndexPath::new);
    select.update(cx, |s: &mut SelectState<Vec<Pick>>, cx| {
        s.set_items(items, window, cx);
        s.set_selected_index(index, window, cx);
    });
}

impl CampaignsPanel {
    /// Fills the form's pickers from the records: currencies, projects, categories, payment
    /// methods and the donation's earmarks.
    pub(super) fn load_form_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let currencies: Vec<(String, String)> = CURRENCIES
            .iter()
            .map(|c| ((*c).to_owned(), (*c).to_owned()))
            .collect();
        load(
            &self.currency_pick,
            picks(&currencies, None),
            Some(self.form.currency.as_str()),
            window,
            cx,
        );
        let projects = self
            .records
            .as_deref()
            .map(project_choices)
            .unwrap_or_default();
        load(
            &self.project_pick,
            picks(&projects, Some("Not tied to one project")),
            Some(self.form.target_node.as_deref().unwrap_or("")),
            window,
            cx,
        );
        let categories: Vec<(String, String)> = CATEGORIES
            .iter()
            .map(|(v, l)| ((*v).to_owned(), (*l).to_owned()))
            .collect();
        load(
            &self.category_pick,
            picks(&categories, Some("None")),
            Some(self.form.category.as_deref().unwrap_or("")),
            window,
            cx,
        );
        self.load_donation_choices(window, cx);
    }

    /// Opens an empty form (`EMPTY_FORM`).
    pub(super) fn open_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.form = NewCampaignForm::default();
        self.form_errors = CampaignErrors::default();
        self.form_failure = None;
        for input in [
            &self.name,
            &self.summary,
            &self.story,
            &self.target,
            &self.ends,
        ] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        self.load_form_choices(window, cx);
        self.name.update(cx, |i, cx| i.focus(window, cx));
    }

    /// Reads the widgets into the form.
    fn read_form(&mut self, cx: &App) {
        let value = |input: &Entity<_>, cx: &App| {
            let state: &gpui_kit::component::input::InputState = input.read(cx);
            state.value().to_string()
        };
        self.form.name = value(&self.name, cx);
        self.form.short_description = value(&self.summary, cx);
        self.form.description = value(&self.story, cx);
        self.form.target = value(&self.target, cx);
        self.form.ends_at = value(&self.ends, cx);
        self.form.currency = picked(&self.currency_pick, cx).unwrap_or_else(|| "KES".to_owned());
        self.form.target_node = picked(&self.project_pick, cx);
        self.form.category = picked(&self.category_pick, cx);
    }

    /// `submit(asDraft)` (`campaign-create-modal.tsx:107-146`): validate, then create; on success
    /// the form closes to the list, where the new campaign is first (newest first).
    fn submit_new(&mut self, as_draft: bool, cx: &mut Context<Self>) {
        self.read_form(cx);
        let now = iso_millis(u64::try_from(super::now_ms()).unwrap_or(0));
        match self.form.to_input(as_draft, &today_utc(), &now) {
            Err(errors) => {
                self.form_errors = *errors;
                self.form_failure = None;
            }
            Ok(input) => {
                self.form_errors = CampaignErrors::default();
                self.write(CampaignWrite::Create(input), cx, |this, result, cx| {
                    match result {
                        Ok(_) => {
                            this.form_failure = None;
                            this.view = CampaignsView::List;
                        }
                        Err(e) => this.form_failure = Some(not_saved(&e.to_string())),
                    }
                    cx.notify();
                });
            }
        }
        cx.notify();
    }

    pub(super) fn new_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let e = &self.form_errors;
        let row = || h_flex().gap(px(space::CELL)).items_start();
        let failure = self.form_failure.clone().map(|f| {
            div()
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_color(p.rejection_red.to_hsla())
                .child(f)
        });
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(page_title("New campaign"))
                    .child(div().text_size(px(text::LEDE)).child(
                        "Set what you are raising for. You can save it as a draft and open it to \
                         donors later.",
                    )),
            )
            .child(
                v_flex().child(section_header("Campaign", div(), cx)).child(
                    v_flex()
                        .py(px(space::CELL))
                        .gap(px(space::CELL))
                        .max_w(px(720.0))
                        .child(field(
                            "Campaign name *",
                            Input::new(&self.name),
                            e.name.clone(),
                            cx,
                        ))
                        .child(field(
                            "One-line summary",
                            Input::new(&self.summary),
                            None,
                            cx,
                        ))
                        .child(hint(
                            "Shown on the donate page and in embeds. Up to 150 characters.",
                            cx,
                        ))
                        .child(field("The story", Input::new(&self.story), None, cx))
                        .child(
                            row()
                                .child(div().flex_1().child(field(
                                    "Target *",
                                    Input::new(&self.target),
                                    e.target.clone(),
                                    cx,
                                )))
                                .child(div().w(px(130.0)).child(field(
                                    "Currency",
                                    Select::new(&self.currency_pick),
                                    None,
                                    cx,
                                ))),
                        )
                        .child(field(
                            "Project it funds",
                            Select::new(&self.project_pick),
                            None,
                            cx,
                        ))
                        .child(hint(
                            "Donations flow into this project, so spending from it shows as \
                                 proof on the campaign.",
                            cx,
                        ))
                        .child(
                            row()
                                .child(div().flex_1().child(field(
                                    "Category",
                                    Select::new(&self.category_pick),
                                    None,
                                    cx,
                                )))
                                .child(div().flex_1().child(field(
                                    "End date",
                                    Input::new(&self.ends),
                                    e.ends_at.clone(),
                                    cx,
                                ))),
                        )
                        .child(hint("Day, month, year. Leave empty for no end date.", cx))
                        .child(self.options(cx))
                        .children(failure)
                        .child(Self::new_buttons(cx)),
                ),
            )
            .child(small_muted(
                "Keys: l back to the list. Campaign names and targets cannot be edited after \
                 saving, as on the web.",
                p,
            ))
            .into_any_element()
    }

    fn options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(px(space::CELL))
            .pt(px(8.0))
            .child(
                Checkbox::new("campaign-public")
                    .label(
                        "Public donate page: anyone with the link can see the campaign and give.",
                    )
                    .checked(self.form.is_public)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.form.is_public = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("campaign-overflow")
                    .label("Keep accepting gifts after the target is reached")
                    .checked(self.form.allow_overflow)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.form.allow_overflow = *checked;
                        cx.notify();
                    })),
            )
    }

    fn new_buttons(cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap(px(8.0))
            .pt(px(8.0))
            .child(
                Button::new("campaign-cancel")
                    .ghost()
                    .label("Cancel")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.show(CampaignsView::List, window, cx);
                    })),
            )
            .child(
                Button::new("campaign-draft")
                    .outline()
                    .label("Save as draft")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.submit_new(true, cx);
                    })),
            )
            .child(
                Button::new("campaign-open")
                    .primary()
                    .label("Create and open to donors")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.submit_new(false, cx);
                    })),
            )
    }
}
