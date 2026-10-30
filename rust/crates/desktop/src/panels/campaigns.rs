//! The Campaigns screen (DT1): the campaign list with its figures, search and status filter
//! (`campaigns/index.tsx`); the new-campaign form (`campaign-create-modal.tsx`); one campaign's
//! manage page with its status menu, donations and the record-donation form (`campaigns/$id.tsx`,
//! `donation-modal.tsx`); and its preview card with the link and embed code
//! (`campaign-embed-widget.tsx`, `social-share-buttons.tsx`, `embed-code-generator.tsx`).
//!
//! Everything shown comes from [`crate::campaigns`]; this file and its two child modules keep
//! the widgets and draw. Writes go through the store ([`crate::panels::write_campaign`]) and the
//! new records come back through its change channel. Deliberate difference (P20): the web keeps
//! the embed code under Settings > Embed and the copy-link button on the donate page; the
//! desktop puts both on the campaign it manages (`rust/docs/handoff.md`). Components:
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/checkbox>, <https://gpui-kit.com/component/button>.

mod manage;
mod new_form;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, px,
};
use ledger_core::Minor;
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::types::CampaignStatus;

use crate::backend::SubmitResult;
use crate::campaigns::{
    CampaignErrors, CampaignList, CampaignPage, CampaignRow, DonationErrors, DonationForm,
    EmbedOptions, NewCampaignForm, ORIGIN_FALLBACK, StatusFilter, report_organization,
    status_label,
};
use crate::panels::bills::new_pick;
use crate::panels::dashboard::{bar, figures_row, notice, small_muted};
use crate::panels::{Pick, PickState, demo_tag, page_title, section_header};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};
use crate::web_format::format_date;

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CampaignsView {
    /// Every campaign.
    List,
    /// The new-campaign form.
    New,
    /// One campaign's manage page.
    Manage(String),
    /// One campaign's preview card, link and embed code.
    Preview(String),
}

impl CampaignsView {
    /// The view a `--campaigns` launch value names: `list`, `new`, `preview:<id>` or an id.
    #[must_use]
    pub fn from_launch(value: &str) -> Self {
        match value {
            "list" => Self::List,
            "new" => Self::New,
            other => other.strip_prefix("preview:").map_or_else(
                || Self::Manage(other.to_owned()),
                |id| Self::Preview(id.to_owned()),
            ),
        }
    }
}

/// Milliseconds since the Unix epoch now, the web's `Date.now()`.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// The Campaigns screen.
pub struct CampaignsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    records: Option<Arc<OrgTransparency>>,
    org_id: String,
    org_name: String,
    demo: bool,
    web_url: String,
    view: CampaignsView,

    filter: StatusFilter,
    query: Entity<InputState>,
    filter_pick: PickState,
    list: Option<CampaignList>,
    page: Option<CampaignPage>,

    form: NewCampaignForm,
    form_errors: CampaignErrors,
    form_failure: Option<String>,
    name: Entity<InputState>,
    summary: Entity<InputState>,
    story: Entity<InputState>,
    target: Entity<InputState>,
    ends: Entity<InputState>,
    currency_pick: PickState,
    project_pick: PickState,
    category_pick: PickState,

    donation: Option<DonationForm>,
    donation_errors: DonationErrors,
    donation_failure: Option<String>,
    pledged: Option<(Minor, String)>,
    amount: Entity<InputState>,
    donor_name: Entity<InputState>,
    donor_email: Entity<InputState>,
    donor_phone: Entity<InputState>,
    method_pick: PickState,
    earmark_pick: PickState,

    action_error: Option<String>,
    confirm_delete: bool,
    copied: Option<&'static str>,
    embed: EmbedOptions,

    _subscriptions: Vec<Subscription>,
}

impl CampaignsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
            let placeholder = placeholder.to_owned();
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let query = input("Search by name or category", window, cx);
        let filters: Vec<Pick> = StatusFilter::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| Pick::new(i.to_string(), f.label()))
            .collect();
        let filter_pick = new_pick(window, cx, filters, Some("0"));
        let empty = || Vec::<Pick>::new();
        let mut panel = Self {
            name: input("e.g. Clean water for Turkana schools", window, cx),
            summary: input("Shown on the donate page and in embeds", window, cx),
            story: input("What the money will do, where, and for whom.", window, cx),
            target: input("e.g. 500000", window, cx),
            ends: input("dd/mm/yyyy", window, cx),
            currency_pick: new_pick(window, cx, empty(), None),
            project_pick: new_pick(window, cx, empty(), None),
            category_pick: new_pick(window, cx, empty(), None),
            amount: input("Other amount", window, cx),
            donor_name: input("Donor name", window, cx),
            donor_email: input("For their receipt and updates", window, cx),
            donor_phone: input("+254 712 345 678", window, cx),
            method_pick: new_pick(window, cx, empty(), None),
            earmark_pick: new_pick(window, cx, empty(), None),
            _subscriptions: vec![
                cx.observe_in(&store, window, |this, _, window, cx| {
                    this.refresh(window, cx);
                }),
                cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.rederive(cx);
                    }
                }),
                cx.subscribe_in(
                    &filter_pick,
                    window,
                    |this, _, event: &SelectEvent<Vec<Pick>>, _, cx| {
                        let SelectEvent::Confirm(value) = event;
                        let index = value.as_deref().and_then(|v| v.parse::<usize>().ok());
                        this.filter = index
                            .and_then(|i| StatusFilter::ALL.get(i).copied())
                            .unwrap_or_default();
                        this.rederive(cx);
                    },
                ),
            ],
            store,
            seen: None,
            records: None,
            org_id: String::new(),
            org_name: String::new(),
            demo: false,
            web_url: ORIGIN_FALLBACK.to_owned(),
            view: CampaignsView::List,
            filter: StatusFilter::All,
            query,
            filter_pick,
            list: None,
            page: None,
            form: NewCampaignForm::default(),
            form_errors: CampaignErrors::default(),
            form_failure: None,
            donation: None,
            donation_errors: DonationErrors::default(),
            donation_failure: None,
            pledged: None,
            action_error: None,
            confirm_delete: false,
            copied: None,
            embed: EmbedOptions::default(),
        };
        panel.refresh(window, cx);
        panel
    }

    /// Sets the web app's address the links and embed code point to (`config.toml`,
    /// `web_url`).
    pub fn set_web_url(&mut self, url: String, cx: &mut Context<Self>) {
        self.web_url = url;
        self.rederive(cx);
    }

    /// Opens `view`, for the launch flags, the list's rows and the keys.
    pub fn show(&mut self, view: CampaignsView, window: &mut Window, cx: &mut Context<Self>) {
        if view == CampaignsView::New {
            self.open_new(window, cx);
        }
        if !matches!(&view, CampaignsView::Manage(id) if self.view == CampaignsView::Manage(id.clone()))
        {
            self.donation = None;
            self.pledged = None;
            self.confirm_delete = false;
            self.action_error = None;
            self.copied = None;
        }
        self.view = view;
        self.rederive(cx);
    }

    /// Keys while the screen shows and no field has focus: `l` the list, `n` a new campaign;
    /// on a campaign `d` records a donation, `v` its preview, `c` copies its link and `e` its
    /// embed code.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let id = match &self.view {
            CampaignsView::Manage(id) | CampaignsView::Preview(id) => Some(id.clone()),
            _ => None,
        };
        match (key, id) {
            ("l", _) => self.show(CampaignsView::List, window, cx),
            ("n", _) => self.show(CampaignsView::New, window, cx),
            ("d", Some(_)) => self.open_donation(window, cx),
            ("v", Some(id)) => self.show(CampaignsView::Preview(id), window, cx),
            ("c", Some(_)) => self.copy_link(cx),
            ("e", Some(_)) => self.copy_embed(cx),
            _ => return false,
        }
        true
    }

    /// Re-reads when the organisation or its transparency records changed.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        let org_changed = self.seen.as_ref().is_none_or(|(seen, _)| *seen != org);
        self.seen = Some(key);
        self.records = view.transparency().cloned();
        self.org_id = org
            .as_ref()
            .map(|o| o.as_str().to_owned())
            .unwrap_or_default();
        self.org_name = view.selected().map(|o| o.name.clone()).unwrap_or_default();
        self.demo = self.org_id == DEMO_ORG_ID;
        if org_changed {
            self.view = CampaignsView::List;
            self.donation = None;
            self.confirm_delete = false;
            self.action_error = None;
        }
        self.load_form_choices(window, cx);
        self.rederive(cx);
    }

    /// Derives the list and the open campaign.
    fn rederive(&mut self, cx: &mut Context<Self>) {
        let Some(t) = self.records.clone() else {
            self.list = None;
            self.page = None;
            cx.notify();
            return;
        };
        let now = now_ms();
        let query = self.query.read(cx).value().to_string();
        self.list = Some(CampaignList::of(&t, &query, self.filter, now));
        self.page = match &self.view {
            CampaignsView::Manage(id) | CampaignsView::Preview(id) => {
                let org = report_organization(&t, &self.org_id, &self.org_name);
                CampaignPage::of(&t, id, org, now, &self.web_url)
            }
            _ => None,
        };
        cx.notify();
    }

    /// [`crate::panels::write_campaign`] on this screen.
    fn write(
        &mut self,
        write: crate::backend::CampaignWrite,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, SubmitResult, &mut Context<Self>) + 'static,
    ) {
        let store = self.store.clone();
        crate::panels::write_campaign(self, &store, write, cx, then);
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .justify_between()
            .items_end()
            .child(
                v_flex().gap(px(8.0)).child(page_title("Campaigns")).child(
                    h_flex()
                        .gap(px(12.0))
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(format!(
                            "{} \u{b7} fundraising campaigns \u{b7} as of {}",
                            if self.org_name.is_empty() {
                                "Your organisation"
                            } else {
                                &self.org_name
                            },
                            format_date(today_utc().as_str())
                        ))
                        .when(self.demo, |m| m.child(demo_tag(cx))),
                ),
            )
            .child(
                Button::new("new-campaign")
                    .primary()
                    .label("New campaign  n")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.show(CampaignsView::New, window, cx);
                    })),
            )
    }

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(list) = &self.list else {
            return notice(
                "No transparency records loaded",
                "Campaigns load with the organisation's books from the server.",
                p,
            )
            .into_any_element();
        };
        let rows: Vec<AnyElement> = list.rows.iter().map(|r| Self::row(r, cx)).collect();
        let body = if let Some(message) = &list.no_match {
            notice("No campaigns match", message, p).into_any_element()
        } else if list.empty {
            notice(
                "No campaigns yet",
                "Campaigns you create appear here with what they have raised and how much of it \
                 is spent with proof.",
                p,
            )
            .into_any_element()
        } else {
            v_flex()
                .child(table_head(p))
                .children(rows)
                .into_any_element()
        };
        v_flex()
            .gap(px(space::SECTION))
            .child(self.header(cx))
            .child(figures_row(&list.figures, p))
            .child(
                v_flex()
                    .child(section_header(
                        "All campaigns",
                        small_muted(&list.meta, p),
                        cx,
                    ))
                    .child(
                        h_flex()
                            .py(px(12.0))
                            .gap(px(8.0))
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .child(div().w(px(320.0)).child(Input::new(&self.query)))
                            .child(div().w(px(200.0)).child(Select::new(&self.filter_pick))),
                    )
                    .child(body),
            )
            .into_any_element()
    }

    fn row(r: &CampaignRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        h_flex()
            .id(SharedString::from(format!("campaign-{}", r.id)))
            .py(px(12.0))
            .gap(px(16.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .hover(|row| row.bg(p.wash.to_hsla()))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.show(CampaignsView::Manage(id.clone()), window, cx);
            }))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(2.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{} \u{2192}", r.name)),
                    )
                    .child(small_muted(&r.subtitle, p)),
            )
            .child(div().w(px(110.0)).child(campaign_status_mark(r.status, p)))
            .child(
                v_flex()
                    .w(px(340.0))
                    .gap(px(6.0))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(r.raised.clone()),
                            )
                            .child(small_muted(&r.of_target, p)),
                    )
                    .child(bar(r.bar, 8.0, p)),
            )
            .child(div().w(px(180.0)).child(small_muted(&r.timing, p)))
            .into_any_element()
    }
}

impl Render for CampaignsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.view.clone() {
            CampaignsView::List => self.list(cx),
            CampaignsView::New => self.new_form(cx),
            CampaignsView::Manage(_) => self.manage(cx),
            CampaignsView::Preview(_) => self.preview(cx),
        }
    }
}

fn table_head(p: &Palette) -> impl IntoElement {
    let head = |label: &'static str| {
        div()
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(label)
    };
    h_flex()
        .py(px(8.0))
        .gap(px(16.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(head("Campaign").flex_1())
        .child(head("Status").w(px(110.0)))
        .child(head("Raised of target").w(px(340.0)))
        .child(head("Timing").w(px(180.0)))
}

/// `CampaignStatusMark` (`campaign-status-badge.tsx:39-66`): a line sample, solid report blue
/// while active, solid ink once completed, dashed muted for a draft or a pause, dotted when
/// cancelled, and the word.
pub(crate) fn campaign_status_mark(status: CampaignStatus, p: &Palette) -> impl IntoElement {
    let (color, dash, gap, ink) = match status {
        CampaignStatus::Active => (p.report_blue.to_hsla(), 22.0, 0.0, p.ink_navy.to_hsla()),
        CampaignStatus::Completed => (p.ink_navy.to_hsla(), 22.0, 0.0, p.ink_navy.to_hsla()),
        CampaignStatus::Draft | CampaignStatus::Paused => {
            (p.ink_muted.to_hsla(), 6.0, 5.0, p.ink_muted.to_hsla())
        }
        CampaignStatus::Cancelled => (p.ink_muted.to_hsla(), 2.0, 5.0, p.ink_muted.to_hsla()),
    };
    let mut segments = Vec::new();
    let (mut at, mut on) = (0.0, true);
    while at < 22.0 {
        let len = f32::min(if on { dash } else { gap }, 22.0 - at);
        if len <= 0.0 {
            break;
        }
        segments.push(div().w(px(len)).h(px(3.0)).when(on, |d| d.bg(color)));
        at += len;
        on = !on;
    }
    h_flex()
        .gap(px(8.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink)
        .whitespace_nowrap()
        .child(h_flex().w(px(22.0)).flex_shrink_0().children(segments))
        .child(status_label(status))
}
