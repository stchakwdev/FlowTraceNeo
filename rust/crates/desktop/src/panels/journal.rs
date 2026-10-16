//! The Journal screen: the entry list with its filters, one entry's detail with the actions its
//! status allows (post, delete, reverse), the manual entry form and the CSV journal import.
//!
//! Everything shown comes from the view models in [`crate::journal`]; this file keeps the
//! widgets and draws. Writes go through the store and the new books come back through its
//! change channel (README.md, "How a screen replaces its placeholder"). Components:
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/button>.

mod form;

use std::sync::Arc;

use gpui_kit::component::IndexPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{EntryId, EntryStatus, IsoDate, OrganizationId, PeriodId, SourceType};
use ledger_core::{CommandResult, LedgerCommand, LedgerState};

use crate::accounts::{Failure, format_date, format_minor};
use crate::backend::BackendError;
use crate::claims::{format_whole, has_overridable};
use crate::entry_form::{EntryForm, ImportPreview};
use crate::journal::{
    DeleteDialog, EntryAction, EntryRow, EntryView, JournalFilter, ReverseErrors, ReverseForm,
    STATUS_FILTERS, StatusCounts, created_entry, entry_rows, entry_status_label, format_signed,
    list_footer, list_notice, override_reason, period_choices, source_choices, source_label,
};
use crate::panels::{demo_tag, failure_box, field, form_buttons, hint, page_title, section_header};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{palette, space, text};

/// A picker's items: labels in the order of the choices they stand for.
type Items = Vec<SharedString>;

/// The most rows the list draws; past that the user filters (large synthetic books only).
const MAX_ROWS: usize = 500;

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum JournalView {
    List,
    Detail(EntryId),
    /// The manual entry form, new or on a draft.
    Form,
    /// The CSV journal import.
    Import,
}

/// The confirmation box open on an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Delete(DeleteDialog),
    Reverse(ReverseForm),
    Override,
}

/// The Journal screen.
pub struct JournalPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    view: JournalView,
    books: Option<Arc<LedgerState>>,
    demo: bool,
    as_of: Option<IsoDate>,

    all: Vec<EntryRow>,
    counts: StatusCounts,
    filter: JournalFilter,
    rows: Vec<EntryRow>,
    cursor: usize,
    sources: Vec<(Option<SourceType>, &'static str)>,
    periods: Vec<(Option<PeriodId>, String)>,
    search: Entity<InputState>,
    source_select: Entity<SelectState<Items>>,
    period_select: Entity<SelectState<Items>>,

    detail: Option<EntryView>,
    done: Option<String>,
    pending: Option<Pending>,
    action_failure: Option<Failure>,
    reverse_errors: ReverseErrors,
    reverse_date: Entity<InputState>,
    reverse_reason: Entity<InputState>,
    override_text: Entity<InputState>,
    override_error: Option<String>,

    form: Option<EntryForm>,
    form_submitted: bool,
    form_failure: Option<Failure>,
    form_needs_override: bool,
    form_override_error: Option<String>,
    entry_date: Entity<InputState>,
    entry_memo: Entity<InputState>,
    form_override: Entity<InputState>,
    form_lines: Vec<form::LineWidgets>,

    import_path: Entity<InputState>,
    import_fund: Entity<SelectState<Items>>,
    import_fund_id: Option<ledger_core::types::FundId>,
    import_text: Option<String>,
    import_error: Option<String>,
    import_preview: Option<ImportPreview>,
    import_failure: Option<Failure>,
    import_created: Option<usize>,

    _subscriptions: Vec<Subscription>,
}

impl JournalPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Number, memo or override reason"));
        let source_select = cx.new(|cx| SelectState::new(Items::new(), None, window, cx));
        let period_select = cx.new(|cx| SelectState::new(Items::new(), None, window, cx));
        let reverse_date = cx.new(|cx| InputState::new(window, cx).placeholder("dd/mm/yyyy"));
        let reverse_reason = cx.new(|cx| InputState::new(window, cx));
        let override_text = cx.new(|cx| InputState::new(window, cx));
        let enter = |event: &InputEvent| matches!(event, InputEvent::PressEnter { .. });
        let w = form::widgets(window, cx);
        let mut subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&search, window, |this, input, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::Change) {
                    this.filter.query = input.read(cx).value().to_string();
                    this.cursor = 0;
                    this.derive();
                    cx.notify();
                }
            }),
            cx.subscribe_in(&source_select, window, Self::on_source),
            cx.subscribe_in(&period_select, window, Self::on_period),
            cx.subscribe_in(
                &reverse_date,
                window,
                move |this, _, e: &InputEvent, window, cx| {
                    if enter(e) {
                        this.reverse_reason.update(cx, |i, cx| i.focus(window, cx));
                    }
                },
            ),
            cx.subscribe_in(&reverse_reason, window, move |this, _, e, window, cx| {
                if enter(e) {
                    this.confirm(window, cx);
                }
            }),
            cx.subscribe_in(&override_text, window, move |this, _, e, window, cx| {
                if enter(e) {
                    this.confirm(window, cx);
                }
            }),
        ];
        subscriptions.extend(form::subscriptions(&w, window, cx));
        let mut panel = Self {
            store,
            seen: None,
            view: JournalView::List,
            books: None,
            demo: false,
            as_of: None,
            all: Vec::new(),
            counts: StatusCounts::default(),
            filter: JournalFilter::default(),
            rows: Vec::new(),
            cursor: 0,
            sources: Vec::new(),
            periods: Vec::new(),
            search,
            source_select,
            period_select,
            detail: None,
            done: None,
            pending: None,
            action_failure: None,
            reverse_errors: ReverseErrors::default(),
            reverse_date,
            reverse_reason,
            override_text,
            override_error: None,
            form: None,
            form_submitted: false,
            form_failure: None,
            form_needs_override: false,
            form_override_error: None,
            entry_date: w.entry_date,
            entry_memo: w.entry_memo,
            form_override: w.form_override,
            form_lines: Vec::new(),
            import_path: w.import_path,
            import_fund: w.import_fund,
            import_fund_id: None,
            import_text: None,
            import_error: None,
            import_preview: None,
            import_failure: None,
            import_created: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    fn on_source(
        &mut self,
        _: &Entity<SelectState<Items>>,
        e: &SelectEvent<Items>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(label) = e;
        self.filter.source = label.as_ref().and_then(|l| {
            self.sources
                .iter()
                .find(|(_, name)| *name == l.as_ref())
                .and_then(|(s, _)| *s)
        });
        self.cursor = 0;
        self.derive();
        cx.notify();
    }

    fn on_period(
        &mut self,
        _: &Entity<SelectState<Items>>,
        e: &SelectEvent<Items>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(label) = e;
        self.filter.period = label.as_ref().and_then(|l| {
            self.periods
                .iter()
                .find(|(_, name)| name == l.as_ref())
                .and_then(|(p, _)| p.clone())
        });
        self.cursor = 0;
        self.derive();
        cx.notify();
    }

    /// Re-derives the list and the open entry when the organisation or its books changed.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let view = store.view();
        let org = view.selected().map(|org| org.id.clone());
        let key = (org.clone(), view.revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        let org_changed = self.seen.as_ref().is_none_or(|(seen, _)| *seen != org);
        self.seen = Some(key);
        self.books = view.snapshot().cloned();
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        // The demo is a snapshot as of its latest movement; other books run to today
        // (`useLedgerAsOf`, flowtrace/src/lib/ledger-client.ts:45-47).
        self.as_of = Some(if self.demo {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        });
        if org_changed {
            self.view = JournalView::List;
            self.filter = JournalFilter::default();
            self.pending = None;
            self.done = None;
            self.cursor = 0;
            self.search.update(cx, |i, cx| i.set_value("", window, cx));
            self.form = None;
            self.form_lines.clear();
            self.import_text = None;
            self.import_created = None;
        }
        self.derive();
        self.reset_pickers(window, cx);
        self.reset_import_fund(org_changed, window, cx);
        cx.notify();
    }

    /// Refills the source and period pickers from the books, keeping the chosen filter.
    fn reset_pickers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sources: Items = self
            .sources
            .iter()
            .map(|(_, l)| SharedString::from(*l))
            .collect();
        let periods: Items = self
            .periods
            .iter()
            .map(|(_, l)| SharedString::from(l.clone()))
            .collect();
        let source_ix = self
            .sources
            .iter()
            .position(|(s, _)| *s == self.filter.source);
        let period_ix = self
            .periods
            .iter()
            .position(|(p, _)| *p == self.filter.period);
        self.source_select.update(cx, |s, cx| {
            s.set_items(sources, window, cx);
            s.set_selected_index(source_ix.map(IndexPath::new), window, cx);
        });
        self.period_select.update(cx, |s, cx| {
            s.set_items(periods, window, cx);
            s.set_selected_index(period_ix.map(IndexPath::new), window, cx);
        });
    }

    fn derive(&mut self) {
        let Some(books) = self.books.as_deref() else {
            self.all.clear();
            self.rows.clear();
            self.detail = None;
            return;
        };
        self.all = entry_rows(books);
        self.counts = StatusCounts::of(&self.all);
        self.sources = source_choices(&self.all);
        self.periods = period_choices(books);
        self.rows = self.filter.apply(&self.all);
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        self.detail = match &self.view {
            JournalView::Detail(id) => EntryView::new(books, id),
            _ => None,
        };
        self.import_preview = self
            .import_text
            .as_deref()
            .and_then(|t| ImportPreview::new(t, books, self.import_fund_id.as_ref()));
    }

    /// Opens the entry `id` (from a row, a link, or the General ledger's
    /// [`crate::panels::general_ledger::OpenEntry`]).
    pub fn open_entry(&mut self, id: EntryId, cx: &mut Context<Self>) {
        self.view = JournalView::Detail(id);
        self.pending = None;
        self.action_failure = None;
        self.done = None;
        self.derive();
        cx.notify();
    }

    fn back_to_list(&mut self) {
        self.view = JournalView::List;
        self.form = None;
        self.form_lines.clear();
        self.pending = None;
        self.action_failure = None;
        self.derive();
    }

    /// Keys while the screen shows and no field has focus: `f` the next status filter, `j`/`k`
    /// or the arrows move through the list and Enter opens the entry; on an entry `p` posts a
    /// draft, `d` deletes it, `r` reverses a posted entry, Enter confirms the open box and
    /// Escape steps back.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let handled = match (key, &self.view) {
            ("escape", JournalView::List) => false,
            ("n", JournalView::List) => {
                self.open_form(None, window, cx);
                true
            }
            ("i", JournalView::List) => {
                self.open_import(window, cx);
                true
            }
            ("escape", JournalView::Form) => {
                self.leave_form();
                true
            }
            ("escape", _) => {
                if self.pending.is_some() {
                    self.pending = None;
                } else {
                    self.back_to_list();
                }
                true
            }
            ("f", JournalView::List) => {
                let i = STATUS_FILTERS
                    .iter()
                    .position(|s| *s == self.filter.status)
                    .unwrap_or(0);
                self.set_status(STATUS_FILTERS[(i + 1) % STATUS_FILTERS.len()]);
                true
            }
            ("j" | "down", JournalView::List) => {
                self.cursor = (self.cursor + 1).min(self.rows.len().saturating_sub(1));
                true
            }
            ("k" | "up", JournalView::List) => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            ("enter", JournalView::List) => match self.rows.get(self.cursor) {
                Some(row) => {
                    let id = row.entry.id.clone();
                    self.open_entry(id, cx);
                    true
                }
                None => false,
            },
            ("enter", JournalView::Detail(_)) if self.pending.is_some() => {
                self.confirm(window, cx);
                true
            }
            (k @ ("p" | "d" | "r" | "e"), JournalView::Detail(_)) => {
                let action = match k {
                    "p" => EntryAction::Post,
                    "d" => EntryAction::Delete,
                    "e" => EntryAction::Edit,
                    _ => EntryAction::Reverse,
                };
                let allowed = self
                    .detail
                    .as_ref()
                    .is_some_and(|d| d.actions.contains(&action));
                if allowed {
                    self.start(action, window, cx);
                }
                allowed
            }
            _ => false,
        };
        if handled {
            cx.notify();
        }
        handled
    }

    fn submit(
        &self,
        command: LedgerCommand,
        cx: &mut Context<Self>,
    ) -> Result<CommandResult, BackendError> {
        self.store.update(cx, |store, _| store.submit(command))
    }

    fn set_status(&mut self, status: Option<EntryStatus>) {
        self.filter.status = status;
        self.cursor = 0;
        self.derive();
    }

    // Entry actions.

    fn start(&mut self, action: EntryAction, window: &mut Window, cx: &mut Context<Self>) {
        self.done = None;
        self.action_failure = None;
        let Some(detail) = self.detail.clone() else {
            return;
        };
        match action {
            EntryAction::Post => {
                self.pending = None;
                self.post(&detail, None, window, cx);
            }
            EntryAction::Delete => {
                self.pending = Some(Pending::Delete(DeleteDialog::of(&detail.entry)));
            }
            EntryAction::Reverse => {
                let as_of = self.as_of.clone().unwrap_or_else(today_utc);
                let form = ReverseForm::new(&detail.entry, &as_of);
                let date = form.date.clone();
                self.reverse_date
                    .update(cx, |i, cx| i.set_value(date, window, cx));
                self.reverse_reason
                    .update(cx, |i, cx| i.set_value("", window, cx));
                self.reverse_reason.update(cx, |i, cx| i.focus(window, cx));
                self.reverse_errors = ReverseErrors::default();
                self.pending = Some(Pending::Reverse(form));
            }
            EntryAction::Edit => {
                let id = detail.entry.id.clone();
                self.open_form(Some(&id), window, cx);
            }
        }
    }

    /// `post` (`entries.$id.tsx`): a refusal every issue of which an override lets through
    /// opens the Post with an override box.
    fn post(
        &mut self,
        detail: &EntryView,
        reason: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.submit(detail.post_command(reason), cx) {
            Ok(_) => {
                self.pending = None;
                self.done = Some("Posted. The entry is in the books.".to_owned());
            }
            Err(err) => {
                self.action_failure = Some(Failure::new(detail.failure_title(), &err));
                if has_overridable(&err) && self.pending != Some(Pending::Override) {
                    self.override_error = None;
                    self.override_text
                        .update(cx, |i, cx| i.set_value("", window, cx));
                    self.override_text.update(cx, |i, cx| i.focus(window, cx));
                    self.pending = Some(Pending::Override);
                }
            }
        }
    }

    /// Confirms the open box.
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(pending), Some(detail)) = (self.pending.clone(), self.detail.clone()) else {
            return;
        };
        match pending {
            Pending::Override => match override_reason(&self.override_text.read(cx).value()) {
                Ok(reason) => {
                    self.override_error = None;
                    self.post(&detail, Some(&reason), window, cx);
                }
                Err(message) => self.override_error = Some(message),
            },
            Pending::Delete(_) => match self.submit(detail.delete_command(), cx) {
                Ok(_) => {
                    self.back_to_list();
                    self.done = Some(format!("{} deleted.", detail.entry.number));
                }
                Err(err) => {
                    self.pending = None;
                    self.action_failure = Some(Failure::new("Not done", &err));
                }
            },
            Pending::Reverse(mut form) => {
                form.date = self.reverse_date.read(cx).value().to_string();
                form.reason = self.reverse_reason.read(cx).value().to_string();
                match form.command(&detail.entry.id) {
                    Err(errors) => self.reverse_errors = errors,
                    Ok(command) => match self.submit(command, cx) {
                        Ok(result) => {
                            let number = detail.entry.number.clone();
                            match created_entry(&result) {
                                Some(id) => self.open_entry(id, cx),
                                None => self.pending = None,
                            }
                            self.done = Some(format!(
                                "{number} is reversed. This is the reversing entry."
                            ));
                        }
                        Err(err) => {
                            self.reverse_errors = ReverseErrors::default();
                            self.action_failure = Some(Failure::new("Not reversed", &err));
                        }
                    },
                }
            }
        }
        cx.notify();
    }

    // Drawing.

    fn meta(&self, cx: &App) -> Div {
        let p = palette(cx.theme().mode);
        h_flex()
            .gap(px(12.0))
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .when_some(self.as_of.clone(), |m, d| {
                m.child(format!("As of {}", format_date(&d)))
            })
            .when(self.demo, |m| m.child(demo_tag(cx)))
    }

    fn notice_line(&self, cx: &App) -> Option<impl IntoElement> {
        let p = palette(cx.theme().mode);
        self.done.clone().map(|done| {
            div()
                .py(px(12.0))
                .border_t(px(space::HAIRLINE))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .font_weight(FontWeight::SEMIBOLD)
                .child(done)
        })
    }

    fn list_figures(&self, cx: &App) -> Div {
        let c = &self.counts;
        h_flex()
            .py(px(space::CELL))
            .gap(px(40.0))
            .items_start()
            .child(figure(&c.posted.to_string(), "Posted", true, cx))
            .child(
                figure(&c.draft.to_string(), "Drafts", false, cx)
                    .child(hint("Not yet in the books", cx)),
            )
            .child(figure(&c.reversed.to_string(), "Reversed", false, cx))
            .child(figure(&c.all.to_string(), "All entries", false, cx))
    }

    /// The status choices with their counts, the search, and the source and period pickers
    /// (`index.tsx`, the filter bar).
    fn filter_bar(&self, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let filters = STATUS_FILTERS
            .iter()
            .map(|&status| {
                let active = status == self.filter.status;
                let label = status.map_or("All", entry_status_label);
                h_flex()
                    .id(SharedString::from(format!("status-{label}")))
                    .h(px(space::BUTTON))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .cursor_pointer()
                    .border_1()
                    .border_color(p.hairline.to_hsla())
                    .map(|b| {
                        if active {
                            b.bg(p.ink_navy.to_hsla())
                                .text_color(p.report_paper.to_hsla())
                                .font_weight(FontWeight::SEMIBOLD)
                        } else {
                            b.text_color(p.ink_muted.to_hsla())
                                .hover(|b| b.bg(p.rail_hover.to_hsla()))
                        }
                    })
                    .child(label)
                    .child(self.counts.get(status).to_string())
                    .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                        panel.set_status(status);
                        cx.notify();
                    }))
            })
            .collect::<Vec<_>>();
        v_flex()
            .py(px(12.0))
            .gap(px(12.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(h_flex().flex_wrap().gap(px(4.0)).children(filters))
            .child(
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
                    .child(field("Search", Input::new(&self.search), None, cx).flex_1())
                    .child(
                        field(
                            "Source",
                            Select::new(&self.source_select).placeholder("All sources"),
                            None,
                            cx,
                        )
                        .w(px(224.0)),
                    )
                    .child(
                        field(
                            "Period",
                            Select::new(&self.period_select).placeholder("All periods"),
                            None,
                            cx,
                        )
                        .w(px(192.0)),
                    ),
            )
    }

    /// `Total: 22 entries` and the debits shown (`index.tsx`, under the table).
    fn list_total(&self, cx: &App) -> Div {
        let p = palette(cx.theme().mode);
        let base = self
            .books
            .as_deref()
            .map_or_else(|| "KES".to_owned(), |b| b.settings.base_currency.clone());
        let shown_total: ledger_core::Minor = self.rows.iter().map(|r| r.total).sum();
        h_flex()
            .py(px(12.0))
            .justify_between()
            .border_t(px(2.0))
            .border_color(p.ink_navy.to_hsla())
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .child(list_footer(self.rows.len(), &self.filter))
            .child(
                h_flex().gap(px(6.0)).child(format!("Debits {base}")).child(
                    div()
                        .text_size(px(text::BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink_navy.to_hsla())
                        .child(format_minor(shown_total)),
                ),
            )
    }

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let header = h_flex()
            .h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(col_number().child("Number"))
            .child(col_date().child("Date"))
            .child(col_memo().child("Memo"))
            .child(col_source().child("Source"))
            .child(col_fund().child("Fund"))
            .child(col_status().child("Status"))
            .child(col_amount().child("Total"));
        let rows: Vec<AnyElement> = self
            .rows
            .iter()
            .take(MAX_ROWS)
            .enumerate()
            .map(|(i, r)| self.entry_row(i, r, cx))
            .collect();
        let notice = list_notice(self.all.len(), self.rows.len(), &self.filter);
        v_flex()
            .gap(px(space::SECTION))
            .when_some(self.notice_line(cx), gpui_kit::ParentElement::child)
            .child(
                v_flex()
                    .child(section_header("Entries in the books", self.meta(cx), cx))
                    .child(self.list_figures(cx)),
            )
            .child(
                v_flex()
                    .child(section_header(
                        "Entries",
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!("{} shown", self.rows.len())),
                        cx,
                    ))
                    .child(self.filter_bar(cx))
                    .map(|v| match notice {
                        Some(n) => v.child(
                            v_flex()
                                .py(px(space::CELL))
                                .gap(px(6.0))
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(n.title))
                                .child(div().text_color(p.ink_muted.to_hsla()).child(n.body)),
                        ),
                        None => v
                            .child(header)
                            .children(rows)
                            .when(self.rows.len() > MAX_ROWS, |v| {
                                v.child(hint(
                                    &format!(
                                        "Showing the latest {MAX_ROWS} of {} entries. Filter \
                                         to narrow the list.",
                                        self.rows.len()
                                    ),
                                    cx,
                                ))
                            })
                            .child(self.list_total(cx)),
                    }),
            )
            .into_any_element()
    }

    fn entry_row(&self, i: usize, r: &EntryRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.entry.id.clone();
        let focused = i == self.cursor;
        h_flex()
            .id(SharedString::from(format!("entry-{}", r.entry.id.as_str())))
            .min_h(px(52.0))
            .cursor_pointer()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .when(focused, |row| row.bg(p.wash.to_hsla()))
            .hover(|row| row.bg(p.rail_hover.to_hsla()))
            .child(
                col_number()
                    .text_size(px(text::MONO))
                    .font_weight(FontWeight::MEDIUM)
                    .child(r.entry.number.clone()),
            )
            .child(
                col_date()
                    .text_color(p.ink_muted.to_hsla())
                    .child(format_date(&r.entry.date)),
            )
            .child(
                col_memo()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.entry.memo.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(r.detail()),
                    ),
            )
            .child(col_source().child(source_label(r.entry.source_type)))
            .child(
                col_fund()
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(r.funds.clone()),
            )
            .child(col_status().child(status_mark(r.entry.status, cx)))
            .child(
                col_amount()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format_minor(r.total)),
            )
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.open_entry(id.clone(), cx);
            }))
            .into_any_element()
    }

    fn detail_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(d) = &self.detail else {
            return v_flex()
                .gap(px(8.0))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("This entry is not in the books"),
                )
                .child(
                    div()
                        .text_color(p.ink_muted.to_hsla())
                        .child("It may have been a draft that was deleted."),
                )
                .into_any_element();
        };
        let base = self
            .books
            .as_deref()
            .map_or_else(|| "KES".to_owned(), |b| b.settings.base_currency.clone());
        let figures = h_flex()
            .py(px(space::CELL))
            .gap(px(40.0))
            .items_start()
            .child(figure(
                &format_whole(d.total),
                &format!("Total, {base}"),
                true,
                cx,
            ))
            .child(figure(
                &format_date(&d.entry.date),
                &d.date_label,
                false,
                cx,
            ))
            .child(figure(&d.lines.len().to_string(), "Lines", false, cx))
            .child(figure(d.source, "Source", false, cx));
        let facts = Self::facts(d, cx);
        let actions = d.actions.iter().map(|&action| {
            let b = Button::new(SharedString::from(format!("act-{}", action.label())))
                .label(action.label())
                .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                    panel.start(action, window, cx);
                    cx.notify();
                }));
            match action {
                EntryAction::Post => b.primary(),
                EntryAction::Delete => b.danger(),
                EntryAction::Reverse | EntryAction::Edit => b.outline(),
            }
        });
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(1040.0))
            .child(
                h_flex()
                    .justify_between()
                    .gap(px(16.0))
                    .child(div().text_size(px(text::LEDE)).child(d.entry.memo.clone()))
                    .child(h_flex().gap(px(8.0)).children(actions)),
            )
            .when_some(self.notice_line(cx), gpui_kit::ParentElement::child)
            .when_some(self.action_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .when(self.pending.is_some(), |v| v.child(self.pending_box(cx)))
            .child(
                v_flex()
                    .child(section_header(
                        "Entry",
                        h_flex()
                            .gap(px(12.0))
                            .child(status_mark(d.entry.status, cx))
                            .when(self.demo, |m| m.child(demo_tag(cx))),
                        cx,
                    ))
                    .child(figures)
                    .child(facts),
            )
            .child(lines_table(d, &base, cx))
            .child(audit_view(d, cx))
            .into_any_element()
    }

    /// Created, posted, the override reason and the reversal links (`entries.$id.tsx`, `dl`).
    fn facts(d: &EntryView, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        v_flex()
            .py(px(12.0))
            .gap(px(10.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(fact("Created", d.created.clone(), cx))
            .child(fact("Posted", d.posted.clone(), cx))
            .when_some(d.override_reason.clone(), |v, r| {
                v.child(fact("Override reason", r, cx))
            })
            .when_some(d.reverses.clone(), |v, link| {
                v.child(Self::link_fact("Reverses", link, cx))
            })
            .when_some(d.reversed_by.clone(), |v, link| {
                v.child(Self::link_fact("Reversed by", link, cx))
            })
    }

    fn link_fact(label: &str, link: crate::journal::EntryLink, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let id = link.id.clone();
        v_flex()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .id(SharedString::from(format!("link-{}", link.id.as_str())))
                    .cursor_pointer()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.report_blue.to_hsla())
                    .underline()
                    .hover(gpui_kit::Styled::underline)
                    .child(link.text)
                    .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                        panel.open_entry(id.clone(), cx);
                    })),
            )
    }

    fn pending_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(pending) = &self.pending else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        let (title, description, body, confirm): (String, String, AnyElement, &'static str) =
            match pending {
                Pending::Delete(d) => (
                    d.title.clone(),
                    d.description.clone(),
                    div().child(d.memo.clone()).into_any_element(),
                    "Delete draft",
                ),
                Pending::Reverse(f) => (
                    f.title.clone(),
                    f.description.clone(),
                    v_flex()
                        .gap(px(space::CELL))
                        .child(
                            field(
                                "Reversal date",
                                Input::new(&self.reverse_date),
                                self.reverse_errors.date.clone(),
                                cx,
                            )
                            .w(px(224.0)),
                        )
                        .child(hint("Day, month, year. Must fall in an open period.", cx))
                        .child(field(
                            "Reason",
                            Input::new(&self.reverse_reason),
                            self.reverse_errors.reason.clone(),
                            cx,
                        ))
                        .into_any_element(),
                    "Post reversal",
                ),
                Pending::Override => (
                    "Post with an override".to_owned(),
                    "The rules above can be set aside when there is a reason a funder or \
                     auditor would accept. The reason is stored on the entry and in the audit \
                     trail."
                        .to_owned(),
                    field(
                        "Override reason",
                        Input::new(&self.override_text),
                        self.override_error.clone(),
                        cx,
                    )
                    .into_any_element(),
                    "Post with override",
                ),
            };
        let cancel_label = matches!(pending, Pending::Delete(_));
        v_flex()
            .p(px(16.0))
            .gap(px(12.0))
            .border(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(div().text_color(p.ink_muted.to_hsla()).child(description))
            .child(body)
            .child(
                form_buttons(
                    "pending",
                    confirm,
                    |panel: &mut Self, window, cx| panel.confirm(window, cx),
                    |panel: &mut Self| panel.pending = None,
                    cx,
                )
                .when(cancel_label, |b| {
                    b.child(hint("Cancel keeps the draft.", cx))
                }),
            )
            .into_any_element()
    }
}

/// A key figure: the number large, its label under it (`KeyFigure`).
fn figure(value: &str, label: &str, large: bool, cx: &App) -> Div {
    let p = palette(cx.theme().mode);
    v_flex()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(if large { text::FIGURE } else { text::TITLE }))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(value.to_owned()),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(p.ink_muted.to_hsla())
                .child(label.to_owned()),
        )
}

/// A labelled fact of the entry (`entries.$id.tsx`, the `dl`).
fn fact(label: &str, value: String, cx: &App) -> Div {
    let p = palette(cx.theme().mode);
    v_flex()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(p.ink_muted.to_hsla())
                .child(label.to_owned()),
        )
        .child(div().child(value))
}

/// `EntryStatusMark` (`-shared.tsx:46-77`): a short line sample, dashed for a draft (not yet in
/// the books), solid when posted, dotted when reversed, then the label; muted unless posted.
fn status_mark(status: EntryStatus, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let ink: Hsla = if status == EntryStatus::Posted {
        p.ink_navy
    } else {
        p.ink_muted
    }
    .to_hsla();
    let (segment, gap) = match status {
        EntryStatus::Draft => (6.0, 5.0),
        EntryStatus::Reversed => (2.0, 5.0),
        EntryStatus::Posted => (22.0, 0.0),
    };
    let mut sample = h_flex().w(px(22.0)).gap(px(gap)).overflow_hidden();
    let mut used = 0.0;
    while used < 22.0 {
        sample = sample.child(div().flex_shrink_0().w(px(segment)).h(px(3.0)).bg(ink));
        used += segment + gap;
    }
    h_flex()
        .gap(px(8.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink)
        .child(sample)
        .child(entry_status_label(status))
}

/// The lines table (`entries.$id.tsx`, Lines): account and memo, fund and grant, tags, debit
/// and credit in the line's currency, and the signed base amount; then the totals.
fn lines_table(d: &EntryView, base: &str, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let muted = p.ink_muted.to_hsla();
    let header = h_flex()
        .h(px(36.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(muted)
        .child(col_memo().child("Account"))
        .child(col_grant().child("Fund and grant"))
        .child(col_tags().child("Tags"))
        .child(col_amount().child("Debit"))
        .child(col_amount().child("Credit"))
        .child(col_amount().child(format!("Base, {base}")));
    let money = |v: ledger_core::Minor, currency: &Option<String>| match currency {
        Some(c) if v != 0 => format!("{c} {}", format_minor(v)),
        _ => format_minor(v),
    };
    let rows = d.lines.iter().map(|l| {
        h_flex()
            .min_h(px(52.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                col_memo()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(l.account.clone()),
                    )
                    .when_some(l.memo.clone(), |c, m| {
                        c.child(div().text_size(px(12.0)).text_color(muted).child(m))
                    }),
            )
            .child(
                col_grant().child(l.fund.clone()).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(muted)
                        .child(l.grant.clone()),
                ),
            )
            .child(col_tags().text_color(muted).child(l.tags.clone()))
            .child(col_amount().py(px(6.0)).child(money(l.debit, &l.currency)))
            .child(col_amount().py(px(6.0)).child(money(l.credit, &l.currency)))
            .child(
                v_flex()
                    .w(px(140.0))
                    .flex_shrink_0()
                    .px(px(8.0))
                    .py(px(6.0))
                    .items_end()
                    .child(format_signed(l.base_amount, "\u{2013}"))
                    .when_some(l.rate.clone(), |c, r| {
                        c.child(div().text_size(px(12.0)).text_color(muted).child(r))
                    }),
            )
    });
    v_flex()
        .child(section_header("Lines", div(), cx))
        .child(header)
        .children(rows)
        .child(
            h_flex()
                .min_h(px(44.0))
                .border_t(px(2.0))
                .border_color(p.ink_navy.to_hsla())
                .font_weight(FontWeight::SEMIBOLD)
                .child(
                    col_memo()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(muted)
                        .child(format!("Totals in {base}")),
                )
                .child(col_grant())
                .child(col_tags())
                .child(col_amount().child(format_minor(d.debits)))
                .child(col_amount().child(format_minor(d.credits)))
                .child(col_amount().child(format_signed(d.debits - d.credits, "0.00"))),
        )
}

/// The audit trail (`entries.$id.tsx`, Audit trail), oldest first.
fn audit_view(d: &EntryView, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let muted = p.ink_muted.to_hsla();
    let rows = d.audit.iter().map(|a| {
        h_flex()
            .py(px(12.0))
            .gap(px(16.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                div()
                    .w(px(192.0))
                    .flex_shrink_0()
                    .text_color(muted)
                    .child(a.at.clone()),
            )
            .child(
                v_flex()
                    .child(
                        h_flex()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(a.label.clone()),
                            )
                            .child(div().text_color(muted).child(format!("by {}", a.by))),
                    )
                    .when_some(a.reason.clone(), |v, r| {
                        v.child(div().text_color(muted).child(format!("Reason: {r}")))
                    }),
            )
    });
    let n = d.audit.len();
    v_flex()
        .child(section_header(
            "Audit trail",
            div()
                .text_size(px(12.0))
                .text_color(muted)
                .child(format!("{n} events")),
            cx,
        ))
        .when(d.audit.is_empty(), |v| {
            v.child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(6.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("No recorded events"),
                    )
                    .child(
                        div()
                            .text_color(muted)
                            .child("This entry was created before the audit trail began."),
                    ),
            )
        })
        .children(rows)
}

fn col_number() -> Div {
    div().w(px(96.0)).flex_shrink_0().px(px(8.0))
}

fn col_date() -> Div {
    div().w(px(112.0)).flex_shrink_0().px(px(8.0))
}

fn col_memo() -> Div {
    v_flex().flex_1().min_w(px(200.0)).px(px(8.0)).py(px(6.0))
}

fn col_source() -> Div {
    div().w(px(150.0)).flex_shrink_0().px(px(8.0))
}

fn col_fund() -> Div {
    div().w(px(170.0)).flex_shrink_0().px(px(8.0))
}

fn col_status() -> Div {
    div().w(px(110.0)).flex_shrink_0().px(px(8.0)).flex()
}

fn col_grant() -> Div {
    v_flex()
        .w(px(220.0))
        .flex_shrink_0()
        .px(px(8.0))
        .py(px(6.0))
}

fn col_tags() -> Div {
    div().w(px(160.0)).flex_shrink_0().px(px(8.0)).py(px(6.0))
}

fn col_amount() -> Div {
    div()
        .w(px(140.0))
        .flex_shrink_0()
        .px(px(8.0))
        .flex()
        .justify_end()
}

impl JournalPanel {
    /// The page title and lede of the view showing.
    fn heading(&self) -> (String, String) {
        match &self.view {
            JournalView::List => (
                "Journal".to_owned(),
                "Every entry in the books, in date order. Drafts are not in the books until \
                 they are posted; posted entries are never edited, only reversed."
                    .to_owned(),
            ),
            JournalView::Detail(_) => (
                self.detail
                    .as_ref()
                    .map_or_else(|| "Journal entry".to_owned(), |d| d.entry.number.clone()),
                String::new(),
            ),
            JournalView::Form => (
                self.form
                    .as_ref()
                    .map_or("New journal entry", EntryForm::title)
                    .to_owned(),
                "A manual entry moves amounts between accounts. Debits must equal credits, in \
                 total and within each fund. Save it as a draft to finish later, or post it to \
                 put it in the books."
                    .to_owned(),
            ),
            JournalView::Import => (
                "Import journals".to_owned(),
                "Bring entries across from QuickBooks or Xero. Each journal in the file becomes \
                 a draft here; nothing reaches the books until someone reviews and posts it."
                    .to_owned(),
            ),
        }
    }
}

impl Render for JournalPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let (title, lede) = self.heading();
        let action: AnyElement = match &self.view {
            JournalView::List => h_flex()
                .gap(px(8.0))
                .child(
                    Button::new("open-import")
                        .outline()
                        .label("Import journals")
                        .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                            panel.open_import(window, cx);
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("open-new")
                        .primary()
                        .label("New journal entry")
                        .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                            panel.open_form(None, window, cx);
                            cx.notify();
                        })),
                )
                .into_any_element(),
            JournalView::Form => Button::new("back")
                .ghost()
                .label(if self.form.as_ref().is_some_and(|f| f.draft.is_some()) {
                    "Back to the draft"
                } else {
                    "Journal"
                })
                .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                    panel.leave_form();
                    cx.notify();
                }))
                .into_any_element(),
            JournalView::Detail(_) | JournalView::Import => Button::new("back")
                .ghost()
                .label("Journal")
                .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                    panel.back_to_list();
                    cx.notify();
                }))
                .into_any_element(),
        };
        let body = if self.books.is_none() {
            div()
                .text_color(p.ink_muted.to_hsla())
                .child("No books loaded for this organisation.")
                .into_any_element()
        } else {
            match &self.view {
                JournalView::List => self.list(cx),
                JournalView::Detail(_) => self.detail_view(cx),
                JournalView::Form => self.form_view(cx),
                JournalView::Import => self.import_view(cx),
            }
        };
        v_flex()
            .gap(px(space::SECTION))
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .gap(px(16.0))
                    .child(
                        v_flex()
                            .gap(px(8.0))
                            .max_w(px(720.0))
                            .child(page_title(&title))
                            .when(!lede.is_empty(), |v| {
                                v.child(div().text_size(px(text::LEDE)).child(lede))
                            }),
                    )
                    .child(action),
            )
            .child(body)
    }
}
