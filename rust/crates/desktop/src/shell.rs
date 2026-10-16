//! The window's root view: the masthead, the nav rail with the organisation picker, the content
//! area (one panel per nav entry, or the connect screen) and the status bar.
//!
//! Shapes follow `DESIGN.md` (Navigation, Layout): a 56px ink masthead, a 224px rail whose
//! active item is an inverted ink block, square corners, and the 3px rule that opens each
//! section. State lives in [`Navigation`], [`ConnectForm`] and the store's [`LedgerView`];
//! this file only forwards input and draws. Components: <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/status-bar>.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme as _, IndexPath, ThemeMode, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, FocusHandle, FontWeight,
    InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, point,
    px,
};

use crate::model::{ConnectForm, LedgerView, NavEntry, Navigation, Screen, scroll_target};
use crate::panels;
use crate::panels::accounts::AccountsPanel;
use crate::panels::general_ledger::GeneralLedgerPanel;
use crate::panels::reports::ReportsPanel;
use crate::statements::ReportKind;
use crate::store::LedgerStore;
use crate::theme::{self, palette, space, text};

/// The organisation picker's items: organisation names in the store's order.
type OrgItems = Vec<SharedString>;

/// The root view.
pub struct Shell {
    store: Entity<LedgerStore>,
    nav: Navigation,
    accounts: Entity<AccountsPanel>,
    general_ledger: Entity<GeneralLedgerPanel>,
    org_select: Entity<SelectState<OrgItems>>,
    host: Entity<InputState>,
    database: Entity<InputState>,
    token_path: Entity<InputState>,
    connect_error: Option<SharedString>,
    reports: Entity<ReportsPanel>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    /// Builds the shell over `store`, showing `nav`.
    pub fn new(
        store: Entity<LedgerStore>,
        nav: Navigation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let names = org_names(store.read(cx).view());
        let selected = (!names.is_empty()).then(|| IndexPath::new(0));
        let org_select = cx.new(|cx| SelectState::new(names, selected, window, cx));

        let form = ConnectForm::default();
        let host = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(form.host.clone())
                .placeholder("http://127.0.0.1:3100")
        });
        let database = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(form.database.clone())
                .placeholder("flowtrace-ledger-dev")
        });
        let token_path = cx.new(|cx| {
            InputState::new(window, cx).placeholder("~/.config/flowtrace/token (optional)")
        });

        let accounts = cx.new(|cx| AccountsPanel::new(store.clone(), window, cx));
        let reports = cx.new(|cx| ReportsPanel::new(store.clone(), window, cx));
        let general_ledger = cx.new(|cx| GeneralLedgerPanel::new(store.clone(), window, cx));

        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe_in(&org_select, window, Self::on_org_selected),
        ];

        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        Self {
            store,
            nav,
            accounts,
            general_ledger,
            org_select,
            host,
            database,
            token_path,
            connect_error: None,
            reports,
            scroll: ScrollHandle::new(),
            focus,
            _subscriptions: subscriptions,
        }
    }

    /// The General ledger panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn general_ledger(&self) -> &Entity<GeneralLedgerPanel> {
        &self.general_ledger
    }

    fn on_org_selected(
        &mut self,
        _: &Entity<SelectState<OrgItems>>,
        event: &SelectEvent<OrgItems>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(name)) = event else {
            return;
        };
        let org = self
            .store
            .read(cx)
            .view()
            .organisations()
            .iter()
            .find(|org| org.name == name.as_ref())
            .map(|org| org.id.clone());
        if let Some(org) = org {
            self.store
                .update(cx, |store, cx| store.select_org(&org, cx));
        }
    }

    /// Keys 1 to 7 open their nav entry (`Navigation::press_key`), Page Up, Page Down, Home
    /// and End scroll the content (`scroll_target`), and other keys go to the screen showing.
    /// Only while the shell itself has focus, so typing in a field never switches screens.
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() || !self.focus.is_focused(window) {
            return;
        }
        let key = event.keystroke.key.as_str();
        let (offset, max) = (self.scroll.offset(), self.scroll.max_offset());
        let page = f32::from(self.scroll.bounds().size.height);
        if let Some(target) = scroll_target(key, -f32::from(offset.y), page, f32::from(max.y)) {
            self.scroll.set_offset(point(offset.x, px(-target)));
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let handled = self.nav.press_key(key)
            || (self.nav.screen == Screen::Books
                && self.nav.entry == NavEntry::Accounts
                && self
                    .accounts
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)));
        if handled {
            cx.stop_propagation();
            cx.notify();
        }
    }

    /// Opens the Reports entry on `kind`.
    pub fn show_report(&mut self, kind: ReportKind, window: &mut Window, cx: &mut Context<Self>) {
        self.reports
            .update(cx, |reports, cx| reports.select(kind, window, cx));
        self.select_entry(NavEntry::Reports, window, cx);
    }

    fn select_entry(&mut self, entry: NavEntry, window: &mut Window, cx: &mut Context<Self>) {
        self.nav.select(entry);
        self.scroll.set_offset(point(px(0.0), px(0.0)));
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn open_connect(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.nav.open_connect();
        self.connect_error = None;
        cx.notify();
    }

    fn close_connect(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.nav.close_connect();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn connect(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let form = ConnectForm {
            host: self.host.read(cx).value().to_string(),
            database: self.database.read(cx).value().to_string(),
            token_path: self.token_path.read(cx).value().to_string(),
        };
        let outcome = form.validate().and_then(|settings| {
            self.store
                .update(cx, |store, _| store.connect(settings))
                .map_err(|err| err.to_string())
        });
        match outcome {
            Ok(()) => {
                self.connect_error = None;
                self.nav.close_connect();
                window.focus(&self.focus, cx);
            }
            Err(message) => self.connect_error = Some(message.into()),
        }
        cx.notify();
    }

    fn toggle_theme(_: &ClickEvent, _: &mut Window, cx: &mut App) {
        let next = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        theme::apply(next, cx);
    }

    /// The content of `entry`. A screen task replaces its arm with its own entity, for example
    /// `NavEntry::Accounts => self.accounts.clone().into_any_element()`.
    fn panel(&self, entry: NavEntry, cx: &App) -> AnyElement {
        let view = self.store.read(cx).view();
        let demo = view
            .selected()
            .is_some_and(|org| org.id.as_str() == ledger_core::demo::DEMO_ORG_ID);
        match entry {
            NavEntry::Accounts => self.accounts.clone().into_any_element(),
            NavEntry::GeneralLedger => self.general_ledger.clone().into_any_element(),
            NavEntry::Journal | NavEntry::Spend | NavEntry::Grants | NavEntry::Reconcile => {
                panels::placeholder(entry, view, demo, cx).into_any_element()
            }
            NavEntry::Reports => self.reports.clone().into_any_element(),
        }
    }

    fn masthead(&self, cx: &App) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let org = self
            .store
            .read(cx)
            .view()
            .selected()
            .map_or_else(|| "No organisation".to_owned(), |org| org.name.clone());
        h_flex()
            .h(px(space::MASTHEAD))
            .flex_shrink_0()
            .px(px(20.0))
            .gap(px(16.0))
            .bg(p.masthead_ink.to_hsla())
            .text_color(p.masthead_paper.to_hsla())
            .child(wordmark(cx))
            .child(
                div()
                    .w(px(space::HAIRLINE))
                    .h(px(24.0))
                    .bg(p.masthead_paper.to_hsla().opacity(0.3)),
            )
            .child(div().text_size(px(text::BODY)).child(org))
    }

    fn rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let current = (self.nav.screen == Screen::Books).then_some(self.nav.entry);
        let items = NavEntry::ALL.into_iter().map(|entry| {
            let active = current == Some(entry);
            h_flex()
                .id(entry.label())
                .h(px(space::NAV_ITEM))
                .px(px(12.0))
                .gap(px(12.0))
                .text_size(px(text::BODY))
                .cursor_pointer()
                .map(|item| {
                    if active {
                        item.bg(p.ink_navy.to_hsla())
                            .text_color(p.report_paper.to_hsla())
                            .font_weight(FontWeight::BOLD)
                    } else {
                        item.font_weight(FontWeight::MEDIUM)
                            .hover(|item| item.bg(p.rail_hover.to_hsla()))
                    }
                })
                .child(
                    div()
                        .w(px(12.0))
                        .text_size(px(text::LABEL))
                        .when(!active, |key| key.text_color(p.ink_muted.to_hsla()))
                        .child(entry.key()),
                )
                .child(entry.label())
                .on_click(cx.listener(move |shell, _: &ClickEvent, window, cx| {
                    shell.select_entry(entry, window, cx);
                }))
        });

        v_flex()
            .w(px(space::RAIL))
            .flex_shrink_0()
            .h_full()
            .bg(p.rail.to_hsla())
            .border_r(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                v_flex()
                    .p(px(12.0))
                    .gap(px(8.0))
                    .child(rail_label("Organisation", cx))
                    .child(Select::new(&self.org_select).placeholder("Choose an organisation")),
            )
            .child(
                v_flex()
                    .px(px(12.0))
                    .pt(px(8.0))
                    .child(div().pb(px(8.0)).child(rail_label("Books", cx)))
                    .children(items),
            )
            .child(div().flex_1())
            .child(
                div().p(px(12.0)).child(
                    Button::new("open-connect")
                        .outline()
                        .w_full()
                        .label("Connection")
                        .on_click(cx.listener(Self::open_connect)),
                ),
            )
    }

    fn connect_screen(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let field = |label: &'static str, input: &Entity<InputState>| {
            v_flex()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(text::BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(label),
                )
                .child(Input::new(input))
        };
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(560.0))
            .child(panels::page_title("Connect"))
            .child(
                v_flex()
                    .child(panels::section_header("SpacetimeDB server", div(), cx))
                    .child(
                        v_flex()
                            .py(px(space::CELL))
                            .gap(px(space::CELL))
                            .child(
                                div()
                                    .text_size(px(text::BODY))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(
                                        "The live client arrives with U10. For now Connect records \
                                         these values and marks the in-process books connected.",
                                    ),
                            )
                            .child(field("Server", &self.host))
                            .child(field("Database", &self.database))
                            .child(field("Token file (optional)", &self.token_path))
                            .when_some(self.connect_error.clone(), |form, message| {
                                form.child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.rejection_red.to_hsla())
                                        .child(message),
                                )
                            })
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .child(
                                        Button::new("connect")
                                            .primary()
                                            .label("Connect")
                                            .on_click(cx.listener(Self::connect)),
                                    )
                                    .child(
                                        Button::new("back")
                                            .outline()
                                            .label("Back to the books")
                                            .on_click(cx.listener(Self::close_connect)),
                                    ),
                            ),
                    ),
            )
    }

    fn status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let status = self.store.read(cx).view().status_text();
        let theme_label = if cx.theme().mode.is_dark() {
            "Light theme"
        } else {
            "Dark theme"
        };
        StatusBar::new()
            .h(px(space::STATUS_BAR))
            .flex_shrink_0()
            .bg(p.rail.to_hsla())
            .border_t(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .left(div().child(status))
            .right(
                div()
                    .text_color(p.ink_muted.to_hsla())
                    .child("Keys 1 to 7 switch screens"),
            )
            .right(
                Button::new("toggle-theme")
                    .ghost()
                    .label(theme_label)
                    .on_click(Self::toggle_theme),
            )
    }
}

/// The organisation names, in the store's order, for the picker.
fn org_names(view: &LedgerView) -> OrgItems {
    view.organisations()
        .iter()
        .map(|org| SharedString::from(org.name.clone()))
        .collect()
}

/// A section label on the rail: 0.75rem bold uppercase in muted ink.
fn rail_label(label: &str, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(text::SECTION_LABEL))
        .font_weight(FontWeight::BOLD)
        .text_color(palette(cx.theme().mode).ink_muted.to_hsla())
        .child(label.to_uppercase())
}

/// The wordmark: three square stations on one line, then the product name (`DESIGN.md`,
/// Shapes and Navigation).
fn wordmark(cx: &App) -> impl IntoElement {
    let paper = palette(cx.theme().mode).masthead_paper.to_hsla();
    let station = || div().size(px(8.0)).bg(paper);
    let line = || div().w(px(8.0)).h(px(2.0)).bg(paper);
    h_flex()
        .gap(px(10.0))
        .child(
            h_flex()
                .child(station())
                .child(line())
                .child(station())
                .child(line())
                .child(station()),
        )
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child("FlowTrace Books"),
        )
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let content: AnyElement = match self.nav.screen {
            Screen::Books => self.panel(self.nav.entry, cx),
            Screen::Connect => self.connect_screen(cx).into_any_element(),
        };
        v_flex()
            .id("shell")
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .bg(p.report_paper.to_hsla())
            .text_color(p.ink_navy.to_hsla())
            .text_size(px(text::BODY))
            .child(self.masthead(cx))
            .child(
                h_flex().flex_1().min_h(px(0.0)).child(self.rail(cx)).child(
                    div()
                        .id("content")
                        .track_scroll(&self.scroll)
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .overflow_y_scroll()
                        .px(px(space::GUTTER))
                        .py(px(space::SECTION))
                        .child(content),
                ),
            )
            .child(self.status_bar(cx))
    }
}
