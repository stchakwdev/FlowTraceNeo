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

use crate::backend::ConnectionStatus;
use crate::model::{
    ConnectForm, LedgerView, NavEntry, Navigation, Screen, connection_summary, scroll_target,
    short_identity, status_label,
};
use crate::panels;
use crate::panels::accounts::AccountsPanel;
use crate::panels::campaigns::CampaignsPanel;
use crate::panels::chains::ChainsPanel;
use crate::panels::dashboard::{DashboardPanel, OpenFlows, RecordTransaction};
use crate::panels::flows::{FlowsPanel, OpenLedgerLines};
use crate::panels::general_ledger::{GeneralLedgerPanel, OpenEntry};
use crate::panels::grants::GrantsPanel;
use crate::panels::journal::JournalPanel;
use crate::panels::reconcile::ReconcilePanel;
use crate::panels::reports::ReportsPanel;
use crate::panels::spend::SpendPanel;
use crate::panels::transactions::{TransactionsPanel, TransactionsView};
use crate::signin::state::{Action, Card, Phase};
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
    journal: Entity<JournalPanel>,
    org_select: Entity<SelectState<OrgItems>>,
    org_items: OrgItems,
    host: Entity<InputState>,
    database: Entity<InputState>,
    token_path: Entity<InputState>,
    connect_error: Option<SharedString>,
    reports: Entity<ReportsPanel>,
    spend: Entity<SpendPanel>,
    grants: Entity<GrantsPanel>,
    reconcile: Entity<ReconcilePanel>,
    dashboard: Entity<DashboardPanel>,
    transactions: Entity<TransactionsPanel>,
    campaigns: Entity<CampaignsPanel>,
    flows: Entity<FlowsPanel>,
    chains: Entity<ChainsPanel>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    /// Builds the shell over `store`, showing `nav`, with the connect screen's fields filled
    /// from `form`.
    pub fn new(
        store: Entity<LedgerStore>,
        nav: Navigation,
        form: &ConnectForm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let names = org_names(store.read(cx).view());
        let selected = (!names.is_empty()).then(|| IndexPath::new(0));
        let org_items = names.clone();
        let org_select = cx.new(|cx| SelectState::new(names, selected, window, cx));

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
            InputState::new(window, cx)
                .default_value(form.token_path.clone())
                .placeholder("Next to config.toml when empty")
        });

        let accounts = cx.new(|cx| AccountsPanel::new(store.clone(), window, cx));
        let reports = cx.new(|cx| ReportsPanel::new(store.clone(), window, cx));
        let general_ledger = cx.new(|cx| GeneralLedgerPanel::new(store.clone(), window, cx));
        let spend = cx.new(|cx| SpendPanel::new(store.clone(), window, cx));
        let grants = cx.new(|cx| GrantsPanel::new(store.clone(), window, cx));
        let journal = cx.new(|cx| JournalPanel::new(store.clone(), window, cx));
        let reconcile = cx.new(|cx| ReconcilePanel::new(store.clone(), window, cx));
        let dashboard = cx.new(|cx| DashboardPanel::new(store.clone(), window, cx));
        let transactions = cx.new(|cx| TransactionsPanel::new(store.clone(), window, cx));
        let campaigns = cx.new(|cx| CampaignsPanel::new(store.clone(), window, cx));
        let flows = cx.new(|cx| FlowsPanel::new(store.clone(), window, cx));
        let chains = cx.new(|cx| ChainsPanel::new(store.clone(), window, cx));

        let subscriptions = vec![
            cx.observe_in(&store, window, |shell, _, window, cx| {
                shell.sync_orgs(window, cx);
                cx.notify();
            }),
            cx.subscribe_in(&org_select, window, Self::on_org_selected),
            cx.subscribe_in(&general_ledger, window, Self::on_open_entry),
            cx.subscribe_in(&flows, window, Self::on_open_ledger_lines),
            cx.subscribe_in(&dashboard, window, |shell, _, _: &OpenFlows, window, cx| {
                shell.select_entry(NavEntry::Flows, window, cx);
            }),
            cx.subscribe_in(
                &dashboard,
                window,
                |shell, _, _: &RecordTransaction, window, cx| {
                    shell.transactions.update(cx, |panel, cx| {
                        panel.show(TransactionsView::New, window, cx);
                    });
                    shell.select_entry(NavEntry::Transactions, window, cx);
                },
            ),
        ];

        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        Self {
            store,
            nav,
            accounts,
            general_ledger,
            journal,
            org_select,
            org_items,
            host,
            database,
            token_path,
            connect_error: None,
            reports,
            spend,
            grants,
            reconcile,
            dashboard,
            transactions,
            campaigns,
            flows,
            chains,
            scroll: ScrollHandle::new(),
            focus,
            _subscriptions: subscriptions,
        }
    }

    /// Keeps the organisation picker on the store's list and selection: a live backend's list
    /// arrives after the window opens and changes when a membership does.
    fn sync_orgs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let names = org_names(view);
        let selected = view
            .selected()
            .and_then(|org| view.organisations().iter().position(|o| o.id == org.id))
            .map(IndexPath::new);
        if names != self.org_items {
            self.org_items.clone_from(&names);
            self.org_select.update(cx, |select, cx| {
                select.set_items(names, window, cx);
                select.set_selected_index(selected, window, cx);
            });
        }
    }

    /// The Grants panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn grants(&self) -> &Entity<GrantsPanel> {
        &self.grants
    }

    /// The Transactions panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn transactions(&self) -> &Entity<TransactionsPanel> {
        &self.transactions
    }

    /// The Campaigns panel, for the launch flags and `web_url` (`app.rs`).
    #[must_use]
    pub fn campaigns(&self) -> &Entity<CampaignsPanel> {
        &self.campaigns
    }

    /// The General ledger panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn general_ledger(&self) -> &Entity<GeneralLedgerPanel> {
        &self.general_ledger
    }

    /// The Chains panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn chains(&self) -> &Entity<ChainsPanel> {
        &self.chains
    }

    /// The Flows panel, for the launch flags (`app.rs`).
    #[must_use]
    pub fn flows(&self) -> &Entity<FlowsPanel> {
        &self.flows
    }

    /// A ledger link on the flow map opens the General ledger on those lines only, as the web's
    /// links to `/ledger/general?source=` and `?node=` do.
    fn on_open_ledger_lines(
        &mut self,
        _: &Entity<FlowsPanel>,
        event: &OpenLedgerLines,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let records = self.store.read(cx).view().transparency().cloned();
        let filter = event.0.filter(records.as_deref());
        self.general_ledger
            .update(cx, |gl, cx| gl.open_scope(filter, window, cx));
        self.select_entry(NavEntry::GeneralLedger, window, cx);
    }

    /// A line clicked in the General ledger opens its entry on the Journal screen.
    fn on_open_entry(
        &mut self,
        _: &Entity<GeneralLedgerPanel>,
        event: &OpenEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = event.0.clone();
        self.journal
            .update(cx, |journal, cx| journal.open_entry(id, cx));
        self.select_entry(NavEntry::Journal, window, cx);
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

    /// Keys 0 to 8 open their nav entry (`Navigation::press_key`), Page Up, Page Down, Home
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
        let books = self.nav.screen == Screen::Books;
        let handled = self.nav.press_key(key)
            || (books
                && self.nav.entry == NavEntry::Accounts
                && self
                    .accounts
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Journal
                && self
                    .journal
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Spend
                && self
                    .spend
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Grants
                && self
                    .grants
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Reconcile
                && self
                    .reconcile
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Flows
                && self
                    .flows
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Chains
                && self
                    .chains
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Transactions
                && self
                    .transactions
                    .update(cx, |panel, cx| panel.press_key(key, window, cx)))
            || (books
                && self.nav.entry == NavEntry::Campaigns
                && self
                    .campaigns
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
        let outcome = self.form_settings(cx).and_then(|settings| {
            self.store
                .update(cx, |store, cx| store.connect(settings, cx))
                .map_err(|err| err.to_string())
        });
        match outcome {
            Ok(()) => {
                self.connect_error = None;
                window.focus(&self.focus, cx);
            }
            Err(message) => self.connect_error = Some(message.into()),
        }
        cx.notify();
    }

    /// The form's settings, or the form's message.
    fn form_settings(&self, cx: &App) -> Result<crate::backend::ConnectSettings, String> {
        ConnectForm {
            host: self.host.read(cx).value().to_string(),
            database: self.database.read(cx).value().to_string(),
            token_path: self.token_path.read(cx).value().to_string(),
        }
        .validate()
    }

    fn sign_in(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        match self.form_settings(cx) {
            Ok(settings) => {
                self.connect_error = None;
                self.store
                    .update(cx, |store, cx| store.sign_in(Some(settings), cx));
            }
            Err(message) => self.connect_error = Some(message.into()),
        }
        cx.notify();
    }

    fn sign_out(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.store.update(cx, LedgerStore::sign_out);
        cx.notify();
    }

    fn disconnect(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.store.update(cx, |store, _| store.disconnect());
        cx.notify();
    }

    fn open_org(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let org = self
            .store
            .read(cx)
            .view()
            .organisations()
            .get(index)
            .map(|org| org.id.clone());
        if let Some(org) = org {
            self.store
                .update(cx, |store, cx| store.select_org(&org, cx));
            self.sync_orgs(window, cx);
            self.nav.close_connect();
            window.focus(&self.focus, cx);
            cx.notify();
        }
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
    fn panel(&self, entry: NavEntry) -> AnyElement {
        match entry {
            NavEntry::Accounts => self.accounts.clone().into_any_element(),
            NavEntry::GeneralLedger => self.general_ledger.clone().into_any_element(),
            NavEntry::Journal => self.journal.clone().into_any_element(),
            NavEntry::Reconcile => self.reconcile.clone().into_any_element(),
            NavEntry::Spend => self.spend.clone().into_any_element(),
            NavEntry::Grants => self.grants.clone().into_any_element(),
            NavEntry::Reports => self.reports.clone().into_any_element(),
            NavEntry::Dashboard => self.dashboard.clone().into_any_element(),
            NavEntry::Transactions => self.transactions.clone().into_any_element(),
            NavEntry::Campaigns => self.campaigns.clone().into_any_element(),
            NavEntry::Flows => self.flows.clone().into_any_element(),
            NavEntry::Chains => self.chains.clone().into_any_element(),
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
        let mut sections: Vec<(Option<&'static str>, Vec<AnyElement>)> = Vec::new();
        for entry in NavEntry::ALL {
            let active = current == Some(entry);
            let item = h_flex()
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
                        .child(entry.key().unwrap_or_default()),
                )
                .child(entry.label())
                .on_click(cx.listener(move |shell, _: &ClickEvent, window, cx| {
                    shell.select_entry(entry, window, cx);
                }))
                .into_any_element();
            match sections.last_mut() {
                Some((section, items)) if *section == entry.section() => items.push(item),
                _ => sections.push((entry.section(), vec![item])),
            }
        }
        // `navSections` (`flowtrace/src/lib/navigation.ts`): Dashboard alone at the top, then a
        // labelled group per section.
        let groups: Vec<AnyElement> = sections
            .into_iter()
            .map(|(section, items)| {
                v_flex()
                    .px(px(12.0))
                    .pt(px(8.0))
                    .when_some(section, |group, label| {
                        group.child(div().pt(px(8.0)).pb(px(8.0)).child(rail_label(label, cx)))
                    })
                    .children(items)
                    .into_any_element()
            })
            .collect();

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
            .children(groups)
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
        let field = connect_field;
        let view = self.store.read(cx).view();
        let status = view.status().clone();
        let remote = view.is_remote();
        let live = matches!(status, ConnectionStatus::Connected { .. });
        let signed_in = self.store.read(cx).signin().phase() == Phase::SignedIn;
        let status_line = match &status {
            ConnectionStatus::Connected { identity } if signed_in => format!(
                "Connected as {}, the identity of your WorkOS sign-in.",
                short_identity(identity)
            ),
            _ => connection_summary(&status, remote),
        };
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(560.0))
            .child(panels::page_title("Connect"))
            .child(
                v_flex()
                    .child(panels::section_header(
                        "Status",
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(status_label(&status)),
                        cx,
                    ))
                    .child(
                        div()
                            .py(px(space::CELL))
                            .text_size(px(text::BODY))
                            .when(matches!(status, ConnectionStatus::Error { .. }), |d| {
                                d.text_color(p.rejection_red.to_hsla())
                            })
                            .child(status_line),
                    ),
            )
            .child(self.signin_section(cx))
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
                                        "Connect opens the books this identity is a member of \
                                         and remembers the server for the next start.",
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
                                            .label(if remote { "Reconnect" } else { "Connect" })
                                            .on_click(cx.listener(Self::connect)),
                                    )
                                    .when(
                                        remote && !matches!(status, ConnectionStatus::Disconnected),
                                        |row| {
                                            row.child(
                                                Button::new("disconnect")
                                                    .outline()
                                                    .label("Disconnect")
                                                    .on_click(cx.listener(Self::disconnect)),
                                            )
                                        },
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
            .child(self.org_section(live, cx))
    }

    /// Who is signed in, with the button to sign in or out, the last failure and the module's
    /// refusal (`signin::state::Card`).
    fn signin_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let card = Card::of(self.store.read(cx).signin());
        let red = |message: String| {
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.rejection_red.to_hsla())
                .child(message)
        };
        let details = card.details.into_iter().map(|(label, value)| {
            h_flex()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(96.0))
                        .flex_shrink_0()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(label),
                )
                .child(div().text_size(px(text::BODY)).child(value))
        });
        v_flex()
            .child(panels::section_header("Sign-in", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(div().text_size(px(text::BODY)).child(card.status))
                    .children(details)
                    .when_some(card.refusal, |d, message| d.child(red(message)))
                    .when_some(card.error, |d, message| d.child(red(message)))
                    .when_some(card.action, |d, action| {
                        d.child(
                            h_flex().child(match action {
                                Action::SignIn => Button::new("sign-in")
                                    .primary()
                                    .label("Sign in with WorkOS")
                                    .on_click(cx.listener(Self::sign_in)),
                                Action::SignOut => Button::new("sign-out")
                                    .outline()
                                    .label("Sign out")
                                    .on_click(cx.listener(Self::sign_out)),
                            }),
                        )
                    }),
            )
    }

    /// The organisations this identity can open, with a count.
    fn org_section(&self, live: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let org_count = self.store.read(cx).view().organisations().len().to_string();
        let orgs = self.org_rows(cx);
        let no_orgs = orgs.is_empty();
        v_flex()
            .child(panels::section_header(
                "Organisations",
                div()
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(org_count),
                cx,
            ))
            .when(no_orgs, |list| {
                list.child(div().py(px(space::CELL)).child(panels::hint(
                    if live {
                        "This identity is not a member of any organisation yet. Ask an \
                                 operator to add it, then the books appear here."
                    } else {
                        "The organisations show once the books have loaded."
                    },
                    cx,
                )))
            })
            .children(orgs)
    }

    /// The Connection screen's organisation rows; a click opens that organisation's books.
    fn org_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::Stateful<gpui_kit::Div>> {
        let p = palette(cx.theme().mode);
        let view = self.store.read(cx).view();
        let selected = view.selected().map(|org| org.id.clone());
        view.organisations()
            .iter()
            .enumerate()
            .map(|(index, org)| {
                let current = selected.as_ref() == Some(&org.id);
                h_flex()
                    .id(("org", index))
                    .h(px(space::NAV_ITEM))
                    .px(px(12.0))
                    .gap(px(12.0))
                    .justify_between()
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .cursor_pointer()
                    .hover(|row| row.bg(p.rail_hover.to_hsla()))
                    .child(
                        div()
                            .font_weight(if current {
                                FontWeight::BOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(org.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(if current {
                                format!("{} · open", org.base_currency)
                            } else {
                                org.base_currency.clone()
                            }),
                    )
                    .on_click(cx.listener(move |shell, _: &ClickEvent, window, cx| {
                        shell.open_org(index, window, cx);
                    }))
            })
            .collect()
    }

    /// The notice above every screen while the books on a server are not live
    /// (`LedgerView::offline_notice`).
    fn offline_notice(&self, cx: &App) -> Option<impl IntoElement> {
        let p = palette(cx.theme().mode);
        let text_ = self.store.read(cx).view().offline_notice()?;
        Some(
            div()
                .mb(px(space::CELL))
                .px(px(16.0))
                .py(px(10.0))
                .border_1()
                .border_color(p.signal_orange.to_hsla())
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(text_),
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
                    .child("Keys 0 to 8 switch screens"),
            )
            .right(
                Button::new("toggle-theme")
                    .ghost()
                    .label(theme_label)
                    .on_click(Self::toggle_theme),
            )
    }
}

/// One labelled field of the connect form.
fn connect_field(label: &'static str, input: &Entity<InputState>) -> impl IntoElement {
    v_flex()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        )
        .child(Input::new(input))
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
            Screen::Books => v_flex()
                .children(self.offline_notice(cx))
                .child(self.panel(self.nav.entry))
                .into_any_element(),
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
