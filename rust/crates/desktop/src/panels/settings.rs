//! The Settings screen (DT4): the "Organisation profile" form and "Access and sharing" with the
//! "Show on public pages" switch and its three links, as the web's
//! `routes/_authenticated/settings/index.tsx` lays them out. The form's values, checks and
//! texts are [`crate::settings`]'s; this file only draws and sends.
//!
//! Saving sends `update_organization_profile` with every field; the switch sends `is_public`
//! alone, as `components/public-pages-switch.tsx` does.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, EventEmitter, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px, relative,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::types::OrgType;

use crate::backend::SettingsWrite;
use crate::model::NavEntry;
use crate::panels::bills::{new_pick, picked};
use crate::panels::dashboard::{notice, small_muted};
use crate::panels::trust_score::OpenScreen;
use crate::panels::{
    Pick, PickState, demo_tag, field, hint, page_title, section_header, write_settings,
};
use crate::settings::{
    COUNTRIES, CURRENCIES, CURRENCY_HINT, NAME_HINT, ORG_TYPES, Profile, ProfileForm, lede, links,
    normalise_slug, public_patch, public_status, save_status, slug_hint,
};
use crate::store::LedgerStore;
use crate::theme::{palette, space, text};

/// The value a type is picked by.
fn type_value(t: OrgType) -> &'static str {
    match t {
        OrgType::Ngo => "NGO",
        OrgType::Sme => "SME",
        OrgType::SocialEnterprise => "SOCIAL_ENTERPRISE",
        OrgType::Government => "GOVERNMENT",
    }
}

/// The Settings screen.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the web form's submitted, saving and saved flags"
)]
pub struct SettingsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    org: Option<OrganizationId>,
    demo: bool,
    /// The stored profile, as last read.
    profile: Option<Profile>,
    /// What the form holds now.
    form: Option<ProfileForm>,
    groups: usize,
    submitted: bool,
    saving: bool,
    saved: bool,
    save_error: Option<String>,
    switching: Option<bool>,
    switch_error: Option<String>,
    name: Entity<InputState>,
    slug: Entity<InputState>,
    type_pick: PickState,
    country_pick: PickState,
    currency_pick: PickState,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenScreen> for SettingsPanel {}

impl SettingsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        let slug = cx.new(|cx| InputState::new(window, cx));
        let pick = |items: Vec<Pick>, window: &mut Window, cx: &mut Context<Self>| {
            new_pick(window, cx, items, None)
        };
        let type_pick = pick(
            ORG_TYPES
                .iter()
                .map(|(t, l)| Pick::new(type_value(*t), *l))
                .collect(),
            window,
            cx,
        );
        let country_pick = pick(
            COUNTRIES.iter().map(|(c, l)| Pick::new(*c, *l)).collect(),
            window,
            cx,
        );
        let currency_pick = pick(
            CURRENCIES.iter().map(|(c, l)| Pick::new(*c, *l)).collect(),
            window,
            cx,
        );
        let mut subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.edited(cx);
                }
            }),
            cx.subscribe_in(
                &slug,
                window,
                |this, input, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        // `e.target.value.toLowerCase().trim()` as typed.
                        let typed = input.read(cx).value().to_string();
                        let kept = normalise_slug(&typed);
                        if kept != typed {
                            input.update(cx, |i, cx| i.set_value(kept, window, cx));
                        }
                        this.edited(cx);
                    }
                },
            ),
        ];
        for select in [&type_pick, &country_pick, &currency_pick] {
            subscriptions.push(cx.subscribe_in(
                select,
                window,
                |this, _, _: &SelectEvent<Vec<Pick>>, _, cx| this.edited(cx),
            ));
        }
        let mut panel = Self {
            store,
            seen: None,
            org: None,
            demo: false,
            profile: None,
            form: None,
            groups: 0,
            submitted: false,
            saving: false,
            saved: false,
            save_error: None,
            switching: None,
            switch_error: None,
            name,
            slug,
            type_pick,
            country_pick,
            currency_pick,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Re-reads the profile when the organisation or its records changed; the form keeps what
    /// was typed unless the organisation changed or nothing was edited.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        let org_changed = self.org != org;
        self.seen = Some(key);
        self.org.clone_from(&org);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        let selected = view.selected().cloned();
        let records = view.transparency().cloned();
        self.groups = records.as_ref().map_or(0, |t| t.access_groups.len());
        self.profile = match (&records, &selected) {
            (Some(t), Some(o)) => Some(Profile::of(t, &o.name, &o.base_currency)),
            _ => None,
        };
        let clean = self.dirty(cx).is_none_or(|d| !d);
        if org_changed || clean || self.form.is_none() {
            if org_changed {
                self.saved = false;
                self.save_error = None;
                self.switch_error = None;
                self.submitted = false;
            }
            self.load_form(window, cx);
        }
        cx.notify();
    }

    /// Puts the stored profile into the widgets (`toForm`).
    fn load_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.profile.as_ref().map(|p| p.form.clone()) else {
            self.form = None;
            return;
        };
        self.name
            .update(cx, |i, cx| i.set_value(form.name.clone(), window, cx));
        self.slug.update(cx, |i, cx| {
            i.set_value(form.public_slug.clone(), window, cx);
        });
        let select = |pick: &PickState, value: &str, window: &mut Window, cx: &mut App| {
            let value = value.to_owned();
            pick.update(cx, |s, cx| s.set_selected_value(&value, window, cx));
        };
        select(&self.type_pick, type_value(form.org_type), window, cx);
        select(&self.country_pick, &form.country, window, cx);
        select(&self.currency_pick, &form.currency, window, cx);
        self.form = Some(form);
    }

    /// Reads the widgets into the form after an edit (`set`): the saved note and a refusal
    /// clear.
    fn edited(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.name = self.name.read(cx).value().to_string();
        form.public_slug = self.slug.read(cx).value().to_string();
        if let Some(t) = picked(&self.type_pick, cx)
            .and_then(|v| ORG_TYPES.iter().find(|(x, _)| type_value(*x) == v))
        {
            form.org_type = t.0;
        }
        if let Some(c) = picked(&self.country_pick, cx) {
            form.country = c;
        }
        if let Some(c) = picked(&self.currency_pick, cx) {
            form.currency = c;
        }
        self.saved = false;
        self.save_error = None;
        cx.notify();
    }

    /// Whether the form differs from the stored profile; `None` with no form.
    fn dirty(&self, _: &App) -> Option<bool> {
        let form = self.form.as_ref()?;
        let stored = &self.profile.as_ref()?.form;
        Some(form != stored)
    }

    /// "Save profile" (`onSubmit`): checks, then saves every field.
    fn save(&mut self, cx: &mut Context<Self>) {
        self.submitted = true;
        let Some(form) = self.form.clone() else {
            return;
        };
        if self.saving || !form.validate().is_empty() || self.dirty(cx) != Some(true) {
            cx.notify();
            return;
        }
        self.saving = true;
        self.save_error = None;
        let store = self.store.clone();
        write_settings(
            self,
            &store,
            SettingsWrite::UpdateProfile(form.patch()),
            cx,
            |this, result, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        this.submitted = false;
                        this.saved = true;
                        // The stored profile now equals the form; the next refresh reloads it.
                        if let (Some(p), Some(f)) = (this.profile.as_mut(), this.form.as_ref()) {
                            p.form = f.clone();
                        }
                    }
                    Err(e) => this.save_error = Some(e.to_string()),
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    /// "Discard changes": back to the stored profile.
    fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.submitted = false;
        self.save_error = None;
        self.load_form(window, cx);
        cx.notify();
    }

    /// The switch: `is_public` alone.
    fn switch(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.switching.is_some() {
            return;
        }
        self.switching = Some(on);
        self.switch_error = None;
        let store = self.store.clone();
        write_settings(
            self,
            &store,
            SettingsWrite::UpdateProfile(public_patch(on)),
            cx,
            |this, result, cx| {
                this.switching = None;
                if let Err(e) = result {
                    this.switch_error = Some(e.to_string());
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    /// Keys while the screen shows: `a` Access groups, `e` Embed & badges.
    pub fn press_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let target = match key {
            "a" => NavEntry::AccessGroups,
            "e" => NavEntry::EmbedBadges,
            _ => return false,
        };
        cx.emit(OpenScreen(target));
        true
    }

    #[expect(clippy::too_many_lines, reason = "the web's form, read top to bottom")]
    fn profile_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let meta = self
            .profile
            .as_ref()
            .and_then(|pr| pr.last_changed.clone())
            .map_or_else(div, |m| small_muted(&m, p));
        let Some(form) = self.form.clone() else {
            return v_flex()
                .child(section_header("Organisation profile", meta, cx))
                .child(notice(
                    "Your organisation profile could not be loaded",
                    "Check your connection and try again.",
                    p,
                ))
                .into_any_element();
        };
        let errors = form.shown_errors(self.submitted);
        let dirty = self.dirty(cx).unwrap_or(false);
        let status = save_status(self.save_error.as_deref(), self.saved, dirty);
        let status_color = if self.save_error.is_some() {
            p.rejection_red.to_hsla()
        } else {
            p.ink_muted.to_hsla()
        };
        let with_hint =
            |control: AnyElement, error: Option<&'static str>, text_: String, cx: &App| {
                let f = field("", control, error.map(str::to_owned), cx);
                if error.is_none() {
                    f.child(hint(&text_, cx))
                } else {
                    f
                }
            };
        let name = with_hint(
            Input::new(&self.name).into_any_element(),
            errors.name,
            NAME_HINT.to_owned(),
            cx,
        );
        let slug = with_hint(
            div()
                .w(px(320.0))
                .child(Input::new(&self.slug))
                .into_any_element(),
            errors.public_slug,
            slug_hint(&form.public_slug),
            cx,
        );
        let save = Button::new("save-profile")
            .primary()
            .label(if self.saving {
                "Saving"
            } else {
                "Save profile"
            })
            .disabled(!dirty || self.saving)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save(cx)));
        let discard = (dirty && !self.saving).then(|| {
            Button::new("discard-profile")
                .ghost()
                .label("Discard changes")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.discard(window, cx)))
        });
        v_flex()
            .child(section_header("Organisation profile", meta, cx))
            .child(
                v_flex()
                    .pt(px(20.0))
                    .gap(px(20.0))
                    .child(label_over("Name", name))
                    .child(
                        h_flex()
                            .gap(px(20.0))
                            .child(div().flex_1().child(field(
                                "Type",
                                Select::new(&self.type_pick),
                                None,
                                cx,
                            )))
                            .child(div().flex_1().child(field(
                                "Country",
                                Select::new(&self.country_pick),
                                None,
                                cx,
                            ))),
                    )
                    .child(
                        field(
                            "Reporting currency",
                            div().w(px(320.0)).child(Select::new(&self.currency_pick)),
                            None,
                            cx,
                        )
                        .child(hint(CURRENCY_HINT, cx)),
                    )
                    .child(label_over("Public link name", slug))
                    .child(
                        h_flex()
                            .gap(px(16.0))
                            .pt(px(16.0))
                            .border_t(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .child(save)
                            .when_some(discard, ParentElement::child)
                            .when_some(status, |row, s| {
                                row.child(
                                    div()
                                        .text_size(px(text::BODY))
                                        .text_color(status_color)
                                        .child(s),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn sharing_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let stored = self.profile.as_ref().is_some_and(|pr| pr.is_public);
        let on = self.switching.unwrap_or(stored);
        let switch = self.profile.is_some().then(|| {
            v_flex()
                .gap(px(8.0))
                .py(px(16.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(
                    h_flex()
                        .justify_between()
                        .gap(px(16.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Show on public pages"),
                        )
                        .child(
                            Switch::new("org-public")
                                .checked(on)
                                .disabled(self.switching.is_some())
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.switch(*checked, cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(if self.switch_error.is_some() {
                            p.rejection_red.to_hsla()
                        } else {
                            p.ink_muted.to_hsla()
                        })
                        .child(public_status(on, self.switch_error.as_deref())),
                )
        });
        let targets = [
            NavEntry::AccessGroups,
            NavEntry::EmbedBadges,
            NavEntry::Chains,
        ];
        let rows: Vec<AnyElement> = links(self.groups)
            .into_iter()
            .zip(targets)
            .enumerate()
            .map(|(i, (l, target))| {
                h_flex()
                    .id(SharedString::from(format!("settings-link-{i}")))
                    .min_h(px(64.0))
                    .py(px(16.0))
                    .justify_between()
                    .gap(px(16.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .cursor_pointer()
                    .hover(|s| s.bg(p.wash.to_hsla()))
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(OpenScreen(target));
                    }))
                    .child(
                        v_flex()
                            .gap(px(4.0))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(l.title))
                            .child(small_muted(&l.description, p).text_size(px(text::BODY))),
                    )
                    .child(div().text_color(p.ink_muted.to_hsla()).child("\u{2192}"))
                    .into_any_element()
            })
            .collect();
        v_flex()
            .child(section_header("Access and sharing", div(), cx))
            .when_some(switch, ParentElement::child)
            .children(rows)
            .into_any_element()
    }
}

/// A form label set over a field built by [`field`] with an empty label.
fn label_over(label: &str, body: Div) -> Div {
    v_flex()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
        .child(body)
}

impl Render for SettingsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let name = self.profile.as_ref().map(|pr| pr.form.name.clone());
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex().gap(px(8.0)).child(page_title("Settings")).child(
                    h_flex()
                        .gap(px(12.0))
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(lede(name.as_deref()))
                        .when(self.demo, |m| m.child(demo_tag(cx))),
                ),
            )
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        div()
                            .flex_basis(relative(7.0 / 12.0))
                            .min_w(px(0.0))
                            .child(self.profile_section(cx)),
                    )
                    .child(
                        div()
                            .flex_basis(relative(5.0 / 12.0))
                            .min_w(px(0.0))
                            .child(self.sharing_section(cx)),
                    ),
            )
    }
}
