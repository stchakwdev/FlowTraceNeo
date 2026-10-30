//! The Embed & badges screen (DT4): the "Trust Score badge" section of the web's
//! `routes/_authenticated/settings/embed.tsx` (`TrustBadgeEmbed`,
//! `components/embed-code-generator.tsx:274-378`): Style and Label, the live preview, and the
//! HTML and Markdown codes with copy buttons. The badge is two flat fields drawn as divs from
//! [`crate::badge::Badge`]; the codes are [`crate::badge`]'s, byte for byte the web's.
//!
//! The page's campaign widget lives on the Campaigns screen (DT1, "Copy embed code"); this
//! screen names how many campaigns are ready and links there. The web's "Embed settings"
//! section is not drawn (`rust/docs/handoff.md`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AppContext as _, ClickEvent, ClipboardItem, Context, Div, Entity, EventEmitter, FontWeight,
    IntoElement, ParentElement as _, Render, Styled as _, Subscription, Window, div, px, relative,
    rgb,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::types::CampaignStatus;

use crate::badge::{
    Badge, BadgeStyle, DEFAULT_LABEL, FONT_SIZE, LABEL_HINT, LABEL_MAX, html_snippet,
    markdown_snippet, shown_label,
};
use crate::campaigns::ORIGIN_FALLBACK;
use crate::model::NavEntry;
use crate::panels::bills::{new_pick, picked};
use crate::panels::dashboard::{notice, small_muted};
use crate::panels::trust_score::OpenScreen;
use crate::panels::{Pick, PickState, demo_tag, field, hint, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// Which code was copied last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Copied {
    Html,
    Markdown,
}

/// The copy note (`CopyBlock`'s status line, `embed-code-generator.tsx:86-91`).
const COPIED_NOTE: &str = "Copied to your clipboard. Paste it into your site\u{2019}s HTML.";

/// The Embed & badges screen.
pub struct EmbedBadgesPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    org: Option<OrganizationId>,
    org_name: Option<String>,
    demo: bool,
    /// The trust row's score; `None` when the records did not load.
    score: Option<i64>,
    /// Active, public campaigns.
    embeddable: usize,
    style: BadgeStyle,
    web_url: String,
    copied: Option<Copied>,
    label: Entity<InputState>,
    style_pick: PickState,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenScreen> for EmbedBadgesPanel {}

impl EmbedBadgesPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = cx.new(|cx| InputState::new(window, cx).default_value(DEFAULT_LABEL));
        let style_pick = new_pick(
            window,
            cx,
            BadgeStyle::ALL
                .iter()
                .map(|s| Pick::new(s.slug(), s.label()))
                .collect(),
            Some(BadgeStyle::Flat.slug()),
        );
        let subscriptions = vec![
            cx.observe(&store, |this, _, cx| this.refresh(cx)),
            cx.subscribe_in(
                &label,
                window,
                |this, input, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        // `maxLength={24}`.
                        let typed = input.read(cx).value().to_string();
                        if typed.chars().count() > LABEL_MAX {
                            let kept: String = typed.chars().take(LABEL_MAX).collect();
                            input.update(cx, |i, cx| i.set_value(kept, window, cx));
                        }
                        this.copied = None;
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &style_pick,
                window,
                |this, pick, _: &SelectEvent<Vec<Pick>>, _, cx| {
                    if let Some(style) = picked(pick, cx).and_then(|v| BadgeStyle::from_slug(&v)) {
                        this.style = style;
                    }
                    this.copied = None;
                    cx.notify();
                },
            ),
        ];
        let mut panel = Self {
            store,
            seen: None,
            org: None,
            org_name: None,
            demo: false,
            score: None,
            embeddable: 0,
            style: BadgeStyle::Flat,
            web_url: ORIGIN_FALLBACK.to_owned(),
            copied: None,
            label,
            style_pick,
            _subscriptions: subscriptions,
        };
        panel.refresh(cx);
        panel
    }

    /// The origin the codes point at (`web_url` in `config.toml`, `app.rs`).
    pub fn set_web_url(&mut self, url: String, cx: &mut Context<Self>) {
        self.web_url = url;
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        self.org.clone_from(&org);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        let records = view.transparency().cloned();
        self.org_name = records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.name.clone()));
        self.score = records
            .as_ref()
            .map(|t| t.trust.as_ref().map_or(0, |s| s.score));
        self.embeddable = records.as_ref().map_or(0, |t| {
            t.campaigns
                .iter()
                .filter(|c| c.status == CampaignStatus::Active && c.is_public)
                .count()
        });
        cx.notify();
    }

    fn shown(&self, cx: &Context<Self>) -> String {
        shown_label(&self.label.read(cx).value()).to_owned()
    }

    fn copy(&mut self, which: Copied, cx: &mut Context<Self>) {
        let Some(org) = self.org.clone() else {
            return;
        };
        let label = self.shown(cx);
        let code = match which {
            Copied::Html => html_snippet(&self.web_url, org.as_str(), self.style, &label),
            Copied::Markdown => markdown_snippet(&self.web_url, org.as_str(), self.style, &label),
        };
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.copied = Some(which);
        cx.notify();
    }

    /// Keys while the screen shows: `h` copies the HTML, `m` the Markdown.
    pub fn press_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        match key {
            "h" => self.copy(Copied::Html, cx),
            "m" => self.copy(Copied::Markdown, cx),
            _ => return false,
        }
        true
    }

    fn copy_block(
        &self,
        which: Copied,
        label: &'static str,
        code: String,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = palette(cx.theme().mode);
        let copied = self.copied == Some(which);
        let id = match which {
            Copied::Html => "copy-badge-html",
            Copied::Markdown => "copy-badge-md",
        };
        let button = Button::new(id)
            .label(if copied { "Copied" } else { "Copy code" })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.copy(which, cx)));
        let button = if copied {
            button.primary()
        } else {
            button.outline()
        };
        v_flex()
            .gap(px(8.0))
            .min_w(px(0.0))
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(label),
                    )
                    .child(button),
            )
            .child(
                div()
                    .p(px(12.0))
                    .border_1()
                    .border_color(p.hairline.to_hsla())
                    .bg(p.wash.to_hsla())
                    .font_family(text::MONO_FAMILY)
                    .text_size(px(12.0))
                    .child(code),
            )
            .child(
                div()
                    .min_h(px(16.0))
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .when(copied, |d| d.child(COPIED_NOTE)),
            )
    }

    fn badge_section(&self, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let label = self.shown(cx);
        let preview = match (&self.org, self.score) {
            (None, _) => small_muted("The badge appears once your organisation has loaded.", p)
                .text_size(px(text::BODY)),
            (Some(_), None) => div()
                .text_size(px(12.0))
                .text_color(p.rejection_red.to_hsla())
                .child("Trust Score could not be loaded."),
            (Some(_), Some(score)) => div().child(badge(&Badge::score(&label, score, self.style))),
        };
        let grid = self.score.map(|score| {
            let row = |title: &'static str, make: &dyn Fn(BadgeStyle) -> Badge| {
                v_flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .children(BadgeStyle::ALL.iter().map(|s| badge(&make(*s)))),
                    )
            };
            v_flex()
                .gap(px(16.0))
                .child(row("Score badge", &|s| Badge::score(&label, score, s)))
                .child(row("Grade badge", &|s| Badge::grade("FlowTrace", score, s)))
        });
        let (html, markdown) = self.org.as_ref().map_or_else(
            || (String::new(), String::new()),
            |org| {
                (
                    html_snippet(&self.web_url, org.as_str(), self.style, &label),
                    markdown_snippet(&self.web_url, org.as_str(), self.style, &label),
                )
            },
        );
        v_flex()
            .child(section_header("Trust Score badge", div(), cx))
            .child(
                h_flex()
                    .pt(px(20.0))
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        v_flex()
                            .flex_basis(relative(4.0 / 12.0))
                            .min_w(px(0.0))
                            .gap(px(20.0))
                            .child(field("Style", Select::new(&self.style_pick), None, cx))
                            .child(
                                field("Label", Input::new(&self.label), None, cx)
                                    .child(hint(LABEL_HINT, cx)),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_basis(relative(8.0 / 12.0))
                            .min_w(px(0.0))
                            .gap(px(24.0))
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(text::BODY))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Live preview"),
                                    )
                                    .child(
                                        h_flex()
                                            .min_h(px(64.0))
                                            .p(px(16.0))
                                            .border_1()
                                            .border_dashed()
                                            .border_color(p.field_stroke.to_hsla())
                                            .bg(p.sheet.to_hsla())
                                            .child(preview),
                                    )
                                    .child(small_muted(
                                        "Shows your current Trust Score, and updates when it changes.",
                                        p,
                                    )),
                            )
                            .when_some(grid, gpui_kit::ParentElement::child)
                            .child(self.copy_block(Copied::Html, "HTML, for websites", html, cx))
                            .child(self.copy_block(
                                Copied::Markdown,
                                "Markdown, for READMEs and docs",
                                markdown,
                                cx,
                            )),
                    ),
            )
    }

    fn widget_section(&self, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let go = Button::new("go-to-campaigns")
            .primary()
            .label("Go to campaigns")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                cx.emit(OpenScreen(NavEntry::Campaigns));
            }));
        let (title, body) = if self.embeddable == 0 {
            (
                "No campaign is ready to embed",
                "A widget needs an active, public campaign. Publish one and its widget code appears here.".to_owned(),
            )
        } else {
            (
                "Campaign widget code is on each campaign",
                format!(
                    "{} active, public {} ready to embed. Open one on the Campaigns screen and copy its embed code.",
                    self.embeddable,
                    if self.embeddable == 1 {
                        "campaign is"
                    } else {
                        "campaigns are"
                    }
                ),
            )
        };
        v_flex()
            .child(section_header("Campaign widget", div(), cx))
            .child(notice(title, &body, p))
            .child(div().child(go))
    }
}

/// `renderBadge` (`badge-utils.ts:64-95`) as two flat fields: the label on ink, the value on
/// its colour, white text centred, 11 px, weights 600 and 800.
#[expect(
    clippy::cast_precision_loss,
    reason = "badge sizes are a few hundred pixels"
)]
fn badge(b: &Badge) -> Div {
    let field_ = |width: u32, color: u32, content: String, weight: FontWeight| {
        div()
            .w(px(width as f32))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(color))
            .text_color(rgb(0x00FF_FFFF))
            .text_size(px(FONT_SIZE))
            .font_weight(weight)
            .child(content)
    };
    h_flex()
        .h(px(b.height as f32))
        .flex_shrink_0()
        .child(field_(
            b.left_width,
            b.label_color,
            b.left.clone(),
            FontWeight::SEMIBOLD,
        ))
        .child(field_(
            b.right_width,
            b.value_color,
            b.right.clone(),
            FontWeight::EXTRA_BOLD,
        ))
}

impl Render for EmbedBadgesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: &Palette = palette(cx.theme().mode);
        let back = Button::new("back-to-settings")
            .ghost()
            .label("\u{2190} Settings")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                cx.emit(OpenScreen(NavEntry::Settings));
            }));
        let lede = format!(
            "Put {}\u{2019}s live figures on your own website: a campaign widget that takes donations, and a badge that shows your Trust Score.",
            self.org_name.as_deref().unwrap_or("your organisation")
        );
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(div().child(back))
                    .child(page_title("Embed & badges"))
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(lede)
                            .when(self.demo, |m| m.child(demo_tag(cx))),
                    ),
            )
            .child(self.widget_section(cx))
            .child(self.badge_section(cx))
    }
}
