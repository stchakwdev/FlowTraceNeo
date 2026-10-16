//! The Spend screen: two tabs, Claims (U5) and Bills (U6, `panels::bills::BillsPanel`). The claims tab
//! is the claims list with its status filter, the new claim form, and one claim's detail with
//! the actions its status allows.
//!
//! Everything shown comes from the view models in [`crate::claims`]; this file keeps the form
//! widgets and draws. Writes go through the store and the new books come back through its
//! change channel (README.md, "How a screen replaces its placeholder"). Components:
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/button>.

use std::sync::Arc;

use gpui_kit::component::IndexPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{Actor, ClaimId, ClaimStatus, DimensionKind, IsoDate, OrganizationId};
use ledger_core::{CommandResult, LedgerCommand, LedgerState};

use crate::accounts::{Failure, format_date, format_minor};
use crate::backend::BackendError;
use crate::claims::{
    ClaimAction, ClaimDetail, ClaimFilter, ClaimForm, ClaimRow, Dialog, FormErrors, LineField,
    Posting, ReimburseErrors, ReimburseForm, Tone, cash_accounts, claim_rows, claim_status_label,
    claim_tone, cost_accounts, dimension_choices, filter_counts, format_whole, fund_choices,
    grant_choices, has_overridable, list_notice,
};
use crate::panels::bills::BillsPanel;
use crate::panels::{demo_tag, failure_box, field, form_buttons, hint, page_title, section_header};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{palette, space, text};

/// The two tabs of the Spend entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendTab {
    /// Expense claims.
    Claims,
    /// Bills, payments and vendors (U6, drawn by [`BillsPanel`]).
    Bills,
}

/// What the claims tab shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ClaimsView {
    List,
    New,
    Detail(ClaimId),
}

/// A picker's items: labels in the order of the choices they stand for.
type Items = Vec<SharedString>;

/// The label of the empty choice of an optional picker.
const NO_GRANT: &str = "No grant";
const NO_PROJECT: &str = "No project";
const NO_PLACE: &str = "No place";

/// The widgets of one coded line of the new claim form.
struct LineWidgets {
    key: u64,
    description: Entity<InputState>,
    amount: Entity<InputState>,
    account: Entity<SelectState<Items>>,
    fund: Entity<SelectState<Items>>,
    grant: Entity<SelectState<Items>>,
    project: Entity<SelectState<Items>>,
    location: Entity<SelectState<Items>>,
    /// The grants the grant picker lists, after `No grant`.
    grants: Vec<(ledger_core::types::GrantId, String)>,
    _subscriptions: Vec<Subscription>,
}

/// Which picker of a line changed.
#[derive(Debug, Clone, Copy)]
enum LinePick {
    Account,
    Fund,
    Grant,
    Project,
    Location,
}

/// The Spend screen.
pub struct SpendPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    tab: SpendTab,
    view: ClaimsView,
    books: Option<Arc<LedgerState>>,
    actor: Option<Actor>,
    demo: bool,
    as_of: Option<IsoDate>,

    filter: Option<ClaimStatus>,
    counts: Vec<(ClaimFilter, usize)>,
    rows: Vec<ClaimRow>,
    cursor: usize,
    done: Option<String>,

    form: Option<ClaimForm>,
    form_errors: Option<FormErrors>,
    form_failure: Option<Failure>,
    date: Entity<InputState>,
    purpose: Entity<InputState>,
    lines: Vec<LineWidgets>,
    next_key: u64,

    detail: Option<ClaimDetail>,
    dialog: Option<Dialog>,
    dialog_failure: Option<Failure>,
    overriding: bool,
    note: Entity<InputState>,
    note_error: Option<String>,
    override_reason: Entity<InputState>,
    reimburse: ReimburseForm,
    reimburse_errors: ReimburseErrors,
    pay_date: Entity<InputState>,
    reference: Entity<InputState>,
    cash: Entity<SelectState<Items>>,
    action_failure: Option<Failure>,

    bills: Entity<BillsPanel>,

    _subscriptions: Vec<Subscription>,
}

impl SpendPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = cx.new(|cx| InputState::new(window, cx).placeholder("dd/mm/yyyy"));
        let purpose = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("The trip or task, e.g. Borehole site inspection")
        });
        let note = cx.new(|cx| InputState::new(window, cx));
        let override_reason = cx.new(|cx| InputState::new(window, cx));
        let pay_date = cx.new(|cx| InputState::new(window, cx).placeholder("dd/mm/yyyy"));
        let reference =
            cx.new(|cx| InputState::new(window, cx).placeholder("M-Pesa code or bank reference"));
        let cash = cx.new(|cx| SelectState::new(Items::new(), None, window, cx));
        let bills = cx.new(|cx| BillsPanel::new(store.clone(), window, cx));
        let enter = |event: &InputEvent| matches!(event, InputEvent::PressEnter { .. });
        let subscriptions = vec![
            cx.observe(&bills, |_, _, cx| cx.notify()),
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&date, window, move |this, _, e: &InputEvent, window, cx| {
                if enter(e) {
                    this.purpose.update(cx, |i, cx| i.focus(window, cx));
                }
            }),
            cx.subscribe_in(
                &purpose,
                window,
                move |this, _, e: &InputEvent, window, cx| {
                    if enter(e)
                        && let Some(first) = this.lines.first()
                    {
                        first.description.update(cx, |i, cx| i.focus(window, cx));
                    }
                },
            ),
            cx.subscribe_in(&note, window, move |this, _, e: &InputEvent, _, cx| {
                if enter(e) {
                    this.confirm(cx);
                }
            }),
            cx.subscribe_in(
                &override_reason,
                window,
                move |this, _, e: &InputEvent, _, cx| {
                    if enter(e) {
                        this.confirm(cx);
                    }
                },
            ),
            cx.subscribe_in(&reference, window, move |this, _, e: &InputEvent, _, cx| {
                if enter(e) {
                    this.confirm(cx);
                }
            }),
            cx.subscribe_in(&cash, window, Self::on_cash),
        ];
        let mut panel = Self {
            store,
            seen: None,
            tab: SpendTab::Claims,
            view: ClaimsView::List,
            books: None,
            actor: None,
            demo: false,
            as_of: None,
            filter: None,
            counts: Vec::new(),
            rows: Vec::new(),
            cursor: 0,
            done: None,
            form: None,
            form_errors: None,
            form_failure: None,
            date,
            purpose,
            lines: Vec::new(),
            next_key: 0,
            detail: None,
            dialog: None,
            dialog_failure: None,
            overriding: false,
            note,
            note_error: None,
            override_reason,
            reimburse: ReimburseForm::new(&today_utc()),
            reimburse_errors: ReimburseErrors::default(),
            pay_date,
            reference,
            cash,
            action_failure: None,
            bills,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// The reimbursement's Paid from picker changed.
    fn on_cash(
        &mut self,
        _: &Entity<SelectState<Items>>,
        e: &SelectEvent<Items>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(label) = e;
        self.reimburse.cash_account = self
            .books
            .as_deref()
            .and_then(|b| chosen(cash_accounts(b), label.as_ref().map(SharedString::as_ref)));
        cx.notify();
    }

    /// Re-derives the list and the open claim when the organisation or its books changed.
    fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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
        self.actor = store.actor();
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        // The demo is a snapshot as of its latest movement; other books run to today
        // (`useLedgerAsOf`, flowtrace/src/lib/ledger-client.ts:45-47).
        self.as_of = Some(if self.demo {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        });
        if org_changed {
            self.view = ClaimsView::List;
            self.form = None;
            self.lines.clear();
            self.dialog = None;
            self.done = None;
            self.cursor = 0;
        }
        self.derive();
        cx.notify();
    }

    fn derive(&mut self) {
        let Some(books) = self.books.as_deref() else {
            self.rows.clear();
            self.counts.clear();
            self.detail = None;
            return;
        };
        self.counts = filter_counts(books);
        self.rows = claim_rows(books, self.filter);
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        self.detail = match &self.view {
            ClaimsView::Detail(id) => ClaimDetail::new(books, id, self.actor.as_ref()),
            _ => None,
        };
    }

    /// Keys while the screen shows and no field has focus: `c` claims, `b` bills, `n` a new
    /// claim, `f` the next status filter, `j`/`k` or the arrows move through the list and Enter
    /// opens the claim; on a claim `s`, `a`, `r` and `p` start Submit, Approve, Reject and
    /// Reimburse, Enter confirms, Escape steps back.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let handled = match (key, &self.view) {
            ("c", _) => {
                self.tab = SpendTab::Claims;
                true
            }
            ("b", _) => {
                self.tab = SpendTab::Bills;
                self.bills
                    .update(cx, |bills, cx| bills.press_key(key, window, cx));
                true
            }
            // `b` also takes the bills back to their list.
            _ if self.tab == SpendTab::Bills => {
                return self
                    .bills
                    .update(cx, |bills, cx| bills.press_key(key, window, cx));
            }
            ("n", _) => {
                self.open_new(window, cx);
                true
            }
            ("escape", ClaimsView::List) => false,
            ("escape", _) => {
                if self.dialog.is_some() {
                    self.dialog = None;
                } else {
                    self.view = ClaimsView::List;
                    self.form = None;
                    self.derive();
                }
                true
            }
            ("f", ClaimsView::List) => {
                let i = crate::claims::CLAIM_FILTERS
                    .iter()
                    .position(|f| f.status == self.filter)
                    .unwrap_or(0);
                let next =
                    crate::claims::CLAIM_FILTERS[(i + 1) % crate::claims::CLAIM_FILTERS.len()];
                self.set_filter(next.status);
                true
            }
            ("j" | "down", ClaimsView::List) => {
                self.cursor = (self.cursor + 1).min(self.rows.len().saturating_sub(1));
                true
            }
            ("k" | "up", ClaimsView::List) => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            ("enter", ClaimsView::List) => match self.rows.get(self.cursor) {
                Some(row) => {
                    let id = row.claim.id.clone();
                    self.open_detail(id);
                    true
                }
                None => false,
            },
            ("enter", ClaimsView::Detail(_)) if self.dialog.is_some() => {
                self.confirm(cx);
                true
            }
            (k @ ("s" | "a" | "r" | "p"), ClaimsView::Detail(_)) => {
                let action = match k {
                    "s" => ClaimAction::Submit,
                    "a" => ClaimAction::Approve,
                    "r" => ClaimAction::Reject,
                    _ => ClaimAction::Reimburse,
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
        &mut self,
        command: LedgerCommand,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, Result<CommandResult, BackendError>, &mut Context<Self>) + 'static,
    ) {
        let store = self.store.clone();
        crate::panels::submit(self, &store, command, cx, then);
    }

    fn set_filter(&mut self, filter: Option<ClaimStatus>) {
        self.filter = filter;
        self.cursor = 0;
        self.derive();
    }

    fn open_detail(&mut self, id: ClaimId) {
        self.view = ClaimsView::Detail(id);
        self.dialog = None;
        self.action_failure = None;
        self.done = None;
        self.derive();
    }

    // The new claim form.

    fn open_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(books), Some(as_of)) = (self.books.clone(), self.as_of.clone()) else {
            return;
        };
        self.tab = SpendTab::Claims;
        self.view = ClaimsView::New;
        self.done = None;
        let form = ClaimForm::new(&books, &as_of);
        let date = form.date.clone();
        self.date.update(cx, |i, cx| i.set_value(date, window, cx));
        self.purpose.update(cx, |i, cx| i.set_value("", window, cx));
        self.lines.clear();
        for line in &form.lines {
            let w = self.line_widgets(&books, line, window, cx);
            self.lines.push(w);
        }
        self.form = Some(form);
        self.form_errors = None;
        self.form_failure = None;
        self.purpose.update(cx, |i, cx| i.focus(window, cx));
    }

    /// The widgets of a new line, filled from `line`.
    fn line_widgets(
        &mut self,
        books: &LedgerState,
        line: &crate::claims::LineDraft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> LineWidgets {
        self.next_key += 1;
        let key = self.next_key;
        let costs = cost_accounts(books);
        let funds = fund_choices(books);
        let grants = grant_choices(books, line.fund.as_ref());
        let projects = dimension_choices(books, DimensionKind::Project);
        let places = dimension_choices(books, DimensionKind::Location);
        let none = Some(IndexPath::new(0));
        let account_sel = index_of(&costs, line.account.as_ref(), 0);
        let fund_sel = index_of(&funds, line.fund.as_ref(), 0);
        let grant_sel = index_of(&grants, line.grant.as_ref(), 1).or(none);
        let project_sel = index_of(&projects, line.project.as_ref(), 1).or(none);
        let place_sel = index_of(&places, line.location.as_ref(), 1).or(none);
        let labels = |none: Option<&str>, list: Vec<String>| -> Items {
            none.map(SharedString::from)
                .into_iter()
                .chain(list.into_iter().map(SharedString::from))
                .collect()
        };
        let (account_items, fund_items) =
            (labels(None, names(&costs)), labels(None, names(&funds)));
        let grant_items = labels(Some(NO_GRANT), names(&grants));
        let project_items = labels(Some(NO_PROJECT), names(&projects));
        let place_items = labels(Some(NO_PLACE), names(&places));

        let description = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. Fuel, motorbike hire")
                .default_value(line.description.clone())
        });
        let amount = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("0.00")
                .default_value(line.amount.clone())
        });
        let select =
            |items: Items, sel: Option<IndexPath>, window: &mut Window, cx: &mut Context<Self>| {
                cx.new(|cx| SelectState::new(items, sel, window, cx))
            };
        let account = select(account_items, account_sel, window, cx);
        let fund = select(fund_items, fund_sel, window, cx);
        let grant = select(grant_items, grant_sel, window, cx);
        let project = select(project_items, project_sel, window, cx);
        let location = select(place_items, place_sel, window, cx);

        let amount_for_enter = amount.clone();
        let subscriptions = vec![
            cx.subscribe_in(
                &description,
                window,
                move |_, _, e: &InputEvent, window, cx| {
                    if matches!(e, InputEvent::PressEnter { .. }) {
                        amount_for_enter.update(cx, |i, cx| i.focus(window, cx));
                    }
                },
            ),
            cx.subscribe_in(&amount, window, |this, _, e: &InputEvent, window, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.save(true, window, cx);
                } else {
                    cx.notify();
                }
            }),
            Self::pick(&account, key, LinePick::Account, window, cx),
            Self::pick(&fund, key, LinePick::Fund, window, cx),
            Self::pick(&grant, key, LinePick::Grant, window, cx),
            Self::pick(&project, key, LinePick::Project, window, cx),
            Self::pick(&location, key, LinePick::Location, window, cx),
        ];
        LineWidgets {
            key,
            description,
            amount,
            account,
            fund,
            grant,
            project,
            location,
            grants,
            _subscriptions: subscriptions,
        }
    }

    fn pick(
        select: &Entity<SelectState<Items>>,
        key: u64,
        which: LinePick,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            select,
            window,
            move |this, _, e: &SelectEvent<Items>, window, cx| {
                let SelectEvent::Confirm(label) = e;
                this.on_pick(
                    key,
                    which,
                    label.as_ref().map(SharedString::as_ref),
                    window,
                    cx,
                );
            },
        )
    }

    /// A line's picker changed: record the choice; a new fund re-lists the grants
    /// (`withFund`, `-shared.tsx:356-361`).
    fn on_pick(
        &mut self,
        key: u64,
        which: LinePick,
        label: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Some(books), Some(form)) = (self.books.clone(), self.form.as_mut()) else {
            return;
        };
        let Some(index) = self.lines.iter().position(|w| w.key == key) else {
            return;
        };
        let line = &mut form.lines[index];
        match which {
            LinePick::Account => {
                line.account = chosen(cost_accounts(&books), label);
            }
            LinePick::Grant => {
                line.grant = chosen(self.lines[index].grants.clone(), label);
            }
            LinePick::Project => {
                line.project = chosen(dimension_choices(&books, DimensionKind::Project), label);
            }
            LinePick::Location => {
                line.location = chosen(dimension_choices(&books, DimensionKind::Location), label);
            }
            LinePick::Fund => {
                let picked = chosen(fund_choices(&books), label);
                form.set_fund(&books, index, picked);
                let line = &form.lines[index];
                let grants = grant_choices(&books, line.fund.as_ref());
                let selected = line
                    .grant
                    .as_ref()
                    .and_then(|g| grants.iter().position(|(id, _)| id == g))
                    .map_or(0, |i| i + 1);
                let mut items = vec![SharedString::from(NO_GRANT)];
                items.extend(grants.iter().map(|(_, n)| SharedString::from(n.clone())));
                let w = &mut self.lines[index];
                w.grants = grants;
                w.grant.update(cx, |s, cx| {
                    s.set_items(items, window, cx);
                    s.set_selected_index(Some(IndexPath::new(selected)), window, cx);
                });
            }
        }
        cx.notify();
    }

    /// Copies the typed text into the form.
    fn sync_form(&mut self, cx: &App) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.date = self.date.read(cx).value().to_string();
        form.purpose = self.purpose.read(cx).value().to_string();
        for (line, w) in form.lines.iter_mut().zip(&self.lines) {
            line.description = w.description.read(cx).value().to_string();
            line.amount = w.amount.read(cx).value().to_string();
        }
    }

    fn add_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_form(cx);
        let Some(books) = self.books.clone() else {
            return;
        };
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.add_line(&books);
        let Some(line) = form.lines.last().cloned() else {
            return;
        };
        let w = self.line_widgets(&books, &line, window, cx);
        w.description.update(cx, |i, cx| i.focus(window, cx));
        self.lines.push(w);
        cx.notify();
    }

    fn remove_line(&mut self, key: u64, cx: &mut Context<Self>) {
        self.sync_form(cx);
        let Some(index) = self.lines.iter().position(|w| w.key == key) else {
            return;
        };
        if self.form.as_mut().is_some_and(|f| f.remove_line(index)) {
            self.lines.remove(index);
        }
        cx.notify();
    }

    /// Save draft or Submit for approval (`claims.new.tsx:91-139`): opens the new claim once
    /// the engine has it.
    fn save(&mut self, submit: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.sync_form(cx);
        let (Some(books), Some(as_of), Some(form)) =
            (self.books.clone(), self.as_of.clone(), self.form.as_ref())
        else {
            return;
        };
        match form.command(&books, &as_of, submit) {
            Err(errors) => {
                self.form_errors = Some(errors);
                self.form_failure = None;
            }
            Ok(command) => {
                self.form_errors = None;
                self.submit(command, cx, move |this, result, _| match result {
                    Ok(CommandResult::Id { id }) => {
                        this.form = None;
                        this.lines.clear();
                        this.open_detail(ClaimId::from(id.as_str()));
                        this.done = Some(if submit {
                            "Submitted. A manager who is not you approves it.".to_owned()
                        } else {
                            "Saved as a draft.".to_owned()
                        });
                    }
                    Ok(_) => {
                        this.form = None;
                        this.view = ClaimsView::List;
                        this.derive();
                    }
                    Err(err) => this.form_failure = Some(Failure::new("Not saved", &err)),
                });
            }
        }
        cx.notify();
    }

    // Claim actions.

    fn start(&mut self, action: ClaimAction, window: &mut Window, cx: &mut Context<Self>) {
        self.done = None;
        self.action_failure = None;
        let Some(detail) = &self.detail else {
            return;
        };
        if action == ClaimAction::Submit {
            let command = detail.submit_command();
            self.submit(command, cx, move |this, result, _| match result {
                Ok(_) => this.done = Some(action.done_notice().to_owned()),
                Err(err) => this.action_failure = Some(Failure::new("Not submitted", &err)),
            });
            return;
        }
        self.dialog = detail.dialog(action);
        self.dialog_failure = None;
        self.overriding = false;
        self.note_error = None;
        self.reimburse_errors = ReimburseErrors::default();
        self.note.update(cx, |i, cx| i.set_value("", window, cx));
        self.override_reason
            .update(cx, |i, cx| i.set_value("", window, cx));
        match action {
            ClaimAction::Reject => self.note.update(cx, |i, cx| i.focus(window, cx)),
            ClaimAction::Reimburse => {
                let as_of = self.as_of.clone().unwrap_or_else(today_utc);
                self.reimburse = ReimburseForm::new(&as_of);
                let date = self.reimburse.date.clone();
                self.pay_date
                    .update(cx, |i, cx| i.set_value(date, window, cx));
                self.reference
                    .update(cx, |i, cx| i.set_value("", window, cx));
                let items: Items = self
                    .books
                    .as_deref()
                    .map(|b| {
                        cash_accounts(b)
                            .into_iter()
                            .map(|(_, l)| SharedString::from(l))
                            .collect()
                    })
                    .unwrap_or_default();
                self.cash.update(cx, |s, cx| {
                    s.set_items(items, window, cx);
                    s.set_selected_index(None, window, cx);
                });
            }
            ClaimAction::Approve | ClaimAction::Submit => {}
        }
    }

    /// Confirms the open dialog (`CommandDialog.confirm`, `-shared.tsx:634-652`).
    fn confirm(&mut self, cx: &mut Context<Self>) {
        let (Some(dialog), Some(detail)) = (self.dialog.clone(), self.detail.as_ref()) else {
            return;
        };
        let note = self.note.read(cx).value().to_string();
        let command = match dialog.action {
            ClaimAction::Approve => {
                let reason = self.override_reason.read(cx).value().trim().to_owned();
                if self.overriding && reason.is_empty() {
                    self.note_error = Some(
                        "Say why this should go through anyway. It is kept with the entry."
                            .to_owned(),
                    );
                    cx.notify();
                    return;
                }
                detail.approve_command(&note, self.overriding.then_some(reason.as_str()))
            }
            ClaimAction::Reject => match detail.reject_command(&note) {
                Ok(c) => c,
                Err(m) => {
                    self.note_error = Some(m);
                    cx.notify();
                    return;
                }
            },
            ClaimAction::Reimburse => {
                self.reimburse.date = self.pay_date.read(cx).value().to_string();
                self.reimburse.reference = self.reference.read(cx).value().to_string();
                match self.reimburse.command(&detail.claim.id) {
                    Ok(c) => c,
                    Err(e) => {
                        self.reimburse_errors = e;
                        cx.notify();
                        return;
                    }
                }
            }
            ClaimAction::Submit => detail.submit_command(),
        };
        self.submit(command, cx, move |this, result, _| match result {
            Ok(_) => {
                this.dialog = None;
                this.done = Some(dialog.action.done_notice().to_owned());
            }
            Err(err) => {
                this.overriding = dialog.action == ClaimAction::Approve && has_overridable(&err);
                let title = if this.overriding {
                    "The ledger flagged this"
                } else {
                    "Not done"
                };
                this.dialog_failure = Some(Failure::new(title, &err));
            }
        });
        cx.notify();
    }

    // Drawing.

    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let tab = |id: &'static str, label: &'static str, key: &'static str, this: SpendTab| {
            let active = self.tab == this;
            h_flex()
                .id(id)
                .h(px(space::NAV_ITEM))
                .px(px(14.0))
                .gap(px(8.0))
                .cursor_pointer()
                .border_b(px(space::RULE_HEAVY))
                .map(|t| {
                    if active {
                        t.border_color(p.ink_navy.to_hsla())
                            .font_weight(FontWeight::BOLD)
                    } else {
                        t.border_color(p.report_paper.to_hsla().opacity(0.0))
                            .text_color(p.ink_muted.to_hsla())
                            .hover(|t| t.bg(p.rail_hover.to_hsla()))
                    }
                })
                .child(div().text_size(px(text::LABEL)).child(key))
                .child(label)
                .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                    panel.tab = this;
                    cx.notify();
                }))
        };
        h_flex()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(tab("tab-claims", "Expense claims", "c", SpendTab::Claims))
            .child(tab("tab-bills", "Bills", "b", SpendTab::Bills))
    }

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

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let filters = self
            .counts
            .iter()
            .map(|(f, n)| {
                let active = f.status == self.filter;
                let status = f.status;
                h_flex()
                    .id(SharedString::from(format!("filter-{}", f.label)))
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
                    .child(f.label)
                    .child(n.to_string())
                    .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                        panel.set_filter(status);
                        cx.notify();
                    }))
            })
            .collect::<Vec<_>>();
        let header = h_flex()
            .h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(col_date().child("Date"))
            .child(col_claim().child("Claim"))
            .child(col_who().child("Claimant"))
            .child(col_grant().child("Fund and grant"))
            .child(col_amount().child("Amount"))
            .child(col_status().child("Status"));
        let rows: Vec<AnyElement> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| self.claim_row(i, r, cx))
            .collect();
        let notice = self
            .books
            .as_deref()
            .and_then(|b| list_notice(b, self.filter));
        v_flex()
            .gap(px(space::SECTION))
            .when_some(self.notice_line(cx), gpui_kit::ParentElement::child)
            .child(
                v_flex()
                    .child(section_header("Claims", self.meta(cx), cx))
                    .child(
                        h_flex()
                            .py(px(12.0))
                            .flex_wrap()
                            .gap(px(4.0))
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .children(filters),
                    )
                    .map(|v| match notice {
                        Some(n) => v.child(
                            v_flex()
                                .py(px(space::CELL))
                                .gap(px(6.0))
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(n.title))
                                .child(div().text_color(p.ink_muted.to_hsla()).child(n.body)),
                        ),
                        None => v.child(header).children(rows),
                    }),
            )
            .into_any_element()
    }

    fn claim_row(&self, i: usize, r: &ClaimRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.claim.id.clone();
        let focused = i == self.cursor;
        h_flex()
            .id(SharedString::from(format!("claim-{}", r.claim.id.as_str())))
            .min_h(px(52.0))
            .cursor_pointer()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .when(focused, |row| row.bg(p.wash.to_hsla()))
            .hover(|row| row.bg(p.rail_hover.to_hsla()))
            .child(
                col_date()
                    .text_color(p.ink_muted.to_hsla())
                    .child(format_date(&r.claim.claim_date)),
            )
            .child(
                col_claim()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.claim.purpose.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(text::MONO))
                            .text_color(p.ink_muted.to_hsla())
                            .child(r.claim.number.clone()),
                    ),
            )
            .child(col_who().child(r.claim.claimant_name.clone()))
            .child(
                col_grant().child(r.funds.clone()).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(r.grants.clone()),
                ),
            )
            .child(
                col_amount()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{} {}", r.claim.currency, format_minor(r.total))),
            )
            .child(col_status().child(status_tag(r.claim.status, cx)))
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.open_detail(id.clone());
                cx.notify();
            }))
            .into_any_element()
    }

    fn new_form(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.sync_form(cx);
        let p = palette(cx.theme().mode);
        let Some(form) = self.form.clone() else {
            return div().into_any_element();
        };
        let base = self
            .books
            .as_deref()
            .map_or_else(|| "KES".to_owned(), |b| b.settings.base_currency.clone());
        let errors = self.form_errors.clone().unwrap_or_default();
        let many = self.lines.len() > 1;
        let lines: Vec<AnyElement> = self
            .lines
            .iter()
            .enumerate()
            .map(|(n, w)| line_editor(n, w, many, &errors, &base, cx))
            .collect();
        let summary = self
            .form_errors
            .as_ref()
            .filter(|e| !e.summary().is_empty());
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(880.0))
            .child(div().text_color(p.ink_muted.to_hsla()).child(
                "Money you spent from your own pocket for the organisation. Once you submit \
                     it, a manager who is not you approves it, and then it is paid back to you.",
            ))
            .when_some(self.form_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .when_some(summary.cloned(), |v, e| {
                v.child(failure_box(
                    &Failure {
                        title: e.title().to_owned(),
                        messages: e.summary(),
                    },
                    cx,
                ))
            })
            .child(
                v_flex()
                    .child(section_header("The claim", div(), cx))
                    .child(
                        h_flex()
                            .py(px(space::CELL))
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                field(
                                    "Date spent",
                                    Input::new(&self.date),
                                    errors.date.clone(),
                                    cx,
                                )
                                .w(px(200.0)),
                            )
                            .child(
                                field(
                                    "Purpose",
                                    Input::new(&self.purpose),
                                    errors.purpose.clone(),
                                    cx,
                                )
                                .flex_1(),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .child(section_header(
                        "What you spent",
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!("In {base}")),
                        cx,
                    ))
                    .children(lines)
                    .child(Self::totals_row(&base, form.total(), cx)),
            )
            .child(Self::form_footer(cx))
            .into_any_element()
    }

    /// Add another line and the live total (`LineEditor`, `-shared.tsx:514-541`).
    fn totals_row(
        base: &str,
        total: ledger_core::Minor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .py(px(12.0))
            .justify_between()
            .border_t(px(2.0))
            .border_color(p.ink_navy.to_hsla())
            .child(
                Button::new("add-line")
                    .outline()
                    .label("Add another line")
                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                        panel.add_line(window, cx);
                    })),
            )
            .child(
                h_flex()
                    .gap(px(8.0))
                    .child(div().text_color(p.ink_muted.to_hsla()).child("Total"))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("{base} {}", total_text(total))),
                    ),
            )
    }

    /// Cancel, Save draft and Submit for approval (`claims.new.tsx:300-312`).
    fn form_footer(cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .pt(px(20.0))
            .gap(px(8.0))
            .justify_end()
            .border_t(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                Button::new("new-cancel")
                    .ghost()
                    .label("Cancel")
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                        panel.form = None;
                        panel.view = ClaimsView::List;
                        panel.derive();
                        cx.notify();
                    })),
            )
            .child(
                Button::new("new-draft")
                    .outline()
                    .label("Save draft")
                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                        panel.save(false, window, cx);
                    })),
            )
            .child(
                Button::new("new-submit")
                    .primary()
                    .label("Submit for approval")
                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                        panel.save(true, window, cx);
                    })),
            )
    }

    fn detail_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(d) = &self.detail else {
            return v_flex()
                .gap(px(8.0))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("This claim is not in the books"),
                )
                .child(
                    div()
                        .text_color(p.ink_muted.to_hsla())
                        .child("It may have been captured on another phone and not sent yet."),
                )
                .into_any_element();
        };
        let c = &d.claim;
        let receipts = c.receipt_names.len();
        let figures = h_flex()
            .py(px(space::CELL))
            .gap(px(40.0))
            .child(figure(
                &format_whole(d.total),
                &format!("Claimed, {}", c.currency),
                true,
                cx,
            ))
            .child(figure(
                &d.lines.len().to_string(),
                if d.lines.len() == 1 { "Line" } else { "Lines" },
                false,
                cx,
            ))
            .child(figure(
                &format_whole(d.outstanding),
                "Still to pay back",
                false,
                cx,
            ))
            .child(figure(
                &receipts.to_string(),
                if receipts == 1 { "Receipt" } else { "Receipts" },
                false,
                cx,
            ));
        let actions = d.actions.iter().map(|&action| {
            let b = Button::new(SharedString::from(format!("act-{}", action.label())))
                .label(action.label())
                .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                    panel.start(action, window, cx);
                    cx.notify();
                }));
            if action == ClaimAction::Reject {
                b.outline()
            } else {
                b.primary()
            }
        });
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(960.0))
            .child(
                h_flex()
                    .gap(px(10.0))
                    .child(status_tag(c.status, cx))
                    .child(div().text_color(p.ink_muted.to_hsla()).child(d.lede())),
            )
            .when_some(self.notice_line(cx), gpui_kit::ParentElement::child)
            .child(
                v_flex()
                    .child(section_header("Claim", self.meta(cx), cx))
                    .child(figures),
            )
            .child(
                v_flex()
                    .child(section_header("What happens next", div(), cx))
                    .child(
                        v_flex()
                            .py(px(12.0))
                            .gap(px(12.0))
                            .child(d.next.clone())
                            .when_some(self.action_failure.clone(), |v, f| {
                                v.child(failure_box(&f, cx))
                            })
                            .when(!d.actions.is_empty() && self.dialog.is_none(), |v| {
                                v.child(h_flex().gap(px(8.0)).children(actions))
                            })
                            .when(self.dialog.is_some(), |v| v.child(self.dialog_box(cx))),
                    ),
            )
            .when_some(d.posting.clone(), |v, posting| {
                v.child(posting_view(&posting, &c.currency, cx))
            })
            .child(lines_table(d, cx))
            .child(trail_view(d, cx))
            .child(books_view(d, cx))
            .into_any_element()
    }

    fn dialog_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(dialog) = &self.dialog else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        let confirm = if self.overriding {
            dialog.override_label()
        } else {
            dialog.confirm.to_owned()
        };
        let fields: AnyElement = match dialog.action {
            ClaimAction::Approve => v_flex()
                .gap(px(space::CELL))
                .child(field(
                    "Note to the claimant",
                    Input::new(&self.note),
                    None,
                    cx,
                ))
                .child(hint("For example, which receipts you checked", cx))
                .when(self.overriding, |v| {
                    v.child(field(
                        "Reason for overriding",
                        Input::new(&self.override_reason),
                        self.note_error.clone(),
                        cx,
                    ))
                    .child(hint("Recorded on the journal entry for auditors", cx))
                })
                .into_any_element(),
            ClaimAction::Reject => field(
                "Reason",
                Input::new(&self.note),
                self.note_error.clone(),
                cx,
            )
            .into_any_element(),
            ClaimAction::Reimburse => v_flex()
                .gap(px(space::CELL))
                .child(
                    field(
                        "Date paid",
                        Input::new(&self.pay_date),
                        self.reimburse_errors.date.clone(),
                        cx,
                    )
                    .w(px(200.0)),
                )
                .child(
                    field(
                        "Paid from",
                        Select::new(&self.cash).placeholder("Choose an account"),
                        self.reimburse_errors.cash.clone(),
                        cx,
                    )
                    .max_w(px(420.0)),
                )
                .child(field("Reference", Input::new(&self.reference), None, cx).max_w(px(420.0)))
                .into_any_element(),
            ClaimAction::Submit => div().into_any_element(),
        };
        v_flex()
            .p(px(16.0))
            .gap(px(12.0))
            .border(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::BOLD)
                    .child(dialog.title.clone()),
            )
            .child(
                div()
                    .text_color(p.ink_muted.to_hsla())
                    .child(dialog.description.clone()),
            )
            .when_some(self.dialog_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .child(fields)
            .child(form_buttons(
                "dialog",
                confirm,
                |panel: &mut Self, _, cx| panel.confirm(cx),
                |panel: &mut Self| panel.dialog = None,
                cx,
            ))
            .into_any_element()
    }
}

/// The index of the chosen id in a picker whose own choices start at `offset`.
fn index_of<T: PartialEq>(
    list: &[(T, String)],
    chosen: Option<&T>,
    offset: usize,
) -> Option<IndexPath> {
    let chosen = chosen?;
    list.iter()
        .position(|(id, _)| id == chosen)
        .map(|i| IndexPath::new(i + offset))
}

/// The labels of `list`, after the empty choice `none` when there is one.
fn names<T>(list: &[(T, String)]) -> Vec<String> {
    list.iter().map(|(_, n)| n.clone()).collect()
}

/// The id whose label is `label`.
fn chosen<T>(list: Vec<(T, String)>, label: Option<&str>) -> Option<T> {
    let label = label?;
    list.into_iter().find(|(_, n)| n == label).map(|(id, _)| id)
}

/// The live total of the form: `0.00` rather than a dash (`LineEditor`, `zero="0.00"`).
fn total_text(minor: ledger_core::Minor) -> String {
    if minor == 0 {
        "0.00".to_owned()
    } else {
        format_minor(minor)
    }
}

/// One line of the new claim form (`LineEditor`, `-shared.tsx:394-512`).
fn line_editor(
    n: usize,
    w: &LineWidgets,
    removable: bool,
    errors: &FormErrors,
    base: &str,
    cx: &mut Context<SpendPanel>,
) -> AnyElement {
    let p = palette(cx.theme().mode);
    let key = w.key;
    let err = |f| errors.line(n, f).map(str::to_owned);
    let row = || h_flex().gap(px(space::CELL)).items_start();
    v_flex()
        .py(px(16.0))
        .gap(px(space::CELL))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Line {}", n + 1)),
                )
                .when(removable, |h| {
                    h.child(
                        Button::new(SharedString::from(format!("remove-{key}")))
                            .ghost()
                            .small()
                            .label("Remove")
                            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                                panel.remove_line(key, cx);
                            })),
                    )
                }),
        )
        .child(
            row()
                .child(
                    field(
                        "What it was for",
                        Input::new(&w.description),
                        err(LineField::Description),
                        cx,
                    )
                    .flex_1(),
                )
                .child(
                    field(
                        &format!("Amount, {base}"),
                        Input::new(&w.amount),
                        err(LineField::Amount),
                        cx,
                    )
                    .w(px(200.0)),
                ),
        )
        .child(
            row()
                .child(
                    field(
                        "Kind of cost",
                        Select::new(&w.account).placeholder("Choose an account"),
                        err(LineField::Account),
                        cx,
                    )
                    .flex_1(),
                )
                .child(
                    field(
                        "Fund",
                        Select::new(&w.fund).placeholder("Choose a fund"),
                        err(LineField::Fund),
                        cx,
                    )
                    .flex_1(),
                ),
        )
        .child(
            row()
                .child(field("Grant", Select::new(&w.grant), err(LineField::Grant), cx).flex_1())
                .child(field("Project", Select::new(&w.project), None, cx).flex_1())
                .child(field("Place", Select::new(&w.location), None, cx).flex_1()),
        )
        .into_any_element()
}

/// A key figure: the number large, its label under it (`KeyFigure`).
fn figure(value: &str, label: &str, large: bool, cx: &App) -> impl IntoElement {
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

/// `DocStatus` (`-shared.tsx:68-98`): a square tag, dashed for drafts, dotted red for
/// rejected, filled once in the books.
fn status_tag(status: ClaimStatus, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let base = div()
        .px(px(6.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .text_size(px(text::LABEL))
        .font_weight(FontWeight::BOLD)
        .child(claim_status_label(status).to_uppercase());
    match claim_tone(status) {
        Tone::Quiet => base
            .border_1()
            .border_dashed()
            .border_color(p.ink_muted.to_hsla())
            .text_color(p.ink_muted.to_hsla()),
        Tone::Open => base
            .border_1()
            .border_color(p.ink_navy.to_hsla())
            .text_color(p.ink_navy.to_hsla()),
        Tone::Booked => base
            .border_1()
            .border_color(p.ink_navy.to_hsla())
            .bg(p.ink_navy.to_hsla())
            .text_color(p.report_paper.to_hsla()),
        Tone::Done => base
            .border_1()
            .border_color(p.report_blue.to_hsla())
            .bg(p.report_blue.to_hsla())
            .text_color(p.report_paper.to_hsla()),
        Tone::Rejected => base
            .border_2()
            .border_dashed()
            .border_color(p.rejection_red.to_hsla())
            .text_color(p.rejection_red.to_hsla()),
    }
}

/// The accrual entry and the grants it charged: entry number, then per grant what this claim
/// put on it and its spend so far against the award.
fn posting_view(posting: &Posting, currency: &str, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let grants = posting.grants.iter().map(|g| {
        h_flex()
            .min_h(px(44.0))
            .gap(px(16.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                div()
                    .flex_1()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(g.name.clone()),
            )
            .child(
                div()
                    .w(px(200.0))
                    .flex()
                    .justify_end()
                    .child(format!("Charged {currency} {}", format_minor(g.charged))),
            )
            .child(
                div()
                    .w(px(320.0))
                    .flex()
                    .justify_end()
                    .text_color(p.ink_muted.to_hsla())
                    .child(format!(
                        "Spent {} of {} awarded",
                        format_minor(g.spend),
                        format_minor(g.award)
                    )),
            )
    });
    v_flex()
        .child(section_header(
            "Posted to the grant",
            div()
                .text_size(px(text::MONO))
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("Entry {}", posting.number)),
            cx,
        ))
        .when(posting.grants.is_empty(), |v| {
            v.child(
                div()
                    .py(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child("The entry charged no grant."),
            )
        })
        .children(grants)
}

fn lines_table(d: &ClaimDetail, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let header = h_flex()
        .h(px(36.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted.to_hsla())
        .child(col_claim().child("Description"))
        .child(col_claim().child("Account"))
        .child(col_grant().child("Fund and grant"))
        .child(col_who().child("Project, place"))
        .child(col_amount().child("Amount"));
    let rows = d.lines.iter().map(|l| {
        h_flex()
            .min_h(px(48.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                col_claim()
                    .font_weight(FontWeight::MEDIUM)
                    .child(l.description.clone()),
            )
            .child(col_claim().child(l.account.clone()))
            .child(
                col_grant().child(l.fund.clone()).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(l.grant.clone()),
                ),
            )
            .child(
                col_who()
                    .text_color(p.ink_muted.to_hsla())
                    .child(l.dimensions.clone()),
            )
            .child(
                col_amount()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format_minor(l.amount)),
            )
    });
    let n = d.lines.len();
    v_flex()
        .child(section_header("Lines", div(), cx))
        .child(header)
        .children(rows)
        .child(
            h_flex()
                .min_h(px(44.0))
                .justify_between()
                .border_t(px(2.0))
                .border_color(p.ink_navy.to_hsla())
                .child(
                    div()
                        .px(px(8.0))
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(format!(
                            "Total, {n} {}",
                            if n == 1 { "line" } else { "lines" }
                        )),
                )
                .child(col_amount().font_weight(FontWeight::BOLD).child(format!(
                    "{} {}",
                    d.claim.currency,
                    format_minor(d.total)
                ))),
        )
}

fn trail_view(d: &ClaimDetail, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let steps = d.trail.iter().map(|s| {
        let ink = if s.rejected {
            p.rejection_red
        } else {
            p.ink_navy
        }
        .to_hsla();
        h_flex()
            .gap(px(12.0))
            .items_start()
            .py(px(8.0))
            .child(
                div()
                    .mt(px(4.0))
                    .size(px(14.0))
                    .border(px(3.0))
                    .border_color(ink),
            )
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(s.label),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!("{} \u{b7} {}", s.who, s.at)),
                    )
                    .when_some(s.note.clone(), |v, n| {
                        v.child(div().child(format!("\u{201c}{n}\u{201d}")))
                    })
                    .when_some(s.reference.clone(), |v, r| {
                        v.child(
                            div()
                                .text_size(px(text::MONO))
                                .child(format!("Reference {r}")),
                        )
                    }),
            )
    });
    v_flex()
        .child(section_header("Approval trail", div(), cx))
        .when(d.trail.is_empty(), |v| {
            v.child(
                div()
                    .py(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child("Nothing yet. The trail starts when the claim is submitted."),
            )
        })
        .child(v_flex().py(px(8.0)).children(steps))
}

fn books_view(d: &ClaimDetail, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let rows = d.books.iter().map(|b| {
        h_flex()
            .min_h(px(44.0))
            .justify_between()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(b.label.clone())
            .child(
                div()
                    .text_size(px(text::MONO))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(b.number.clone()),
            )
    });
    v_flex()
        .child(section_header("In the books", div(), cx))
        .when(d.books.is_empty(), |v| {
            v.child(
                div()
                    .py(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child("Not posted yet. A journal entry is made when the claim is approved."),
            )
        })
        .children(rows)
}

fn col_date() -> Div {
    div().w(px(112.0)).flex_shrink_0().px(px(8.0))
}

fn col_claim() -> Div {
    v_flex().flex_1().min_w(px(180.0)).px(px(8.0)).py(px(6.0))
}

fn col_who() -> Div {
    v_flex().w(px(160.0)).flex_shrink_0().px(px(8.0))
}

fn col_grant() -> Div {
    v_flex()
        .w(px(240.0))
        .flex_shrink_0()
        .px(px(8.0))
        .py(px(6.0))
}

fn col_amount() -> Div {
    div()
        .w(px(140.0))
        .flex_shrink_0()
        .px(px(8.0))
        .flex()
        .justify_end()
}

fn col_status() -> Div {
    div().w(px(176.0)).flex_shrink_0().px(px(8.0)).flex()
}

impl Render for SpendPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let (mut bills_header, mut bills_action) = (None, None);
        if self.tab == SpendTab::Bills {
            let (title, lede, action) = self.bills.update(cx, BillsPanel::header);
            bills_header = Some((title, lede));
            bills_action = Some(action);
        }
        let (title, lede) = match (self.tab, &self.view) {
            (SpendTab::Bills, _) => bills_header.take().unwrap_or_default(),
            (SpendTab::Claims, ClaimsView::New) => ("New expense claim".to_owned(), String::new()),
            (SpendTab::Claims, ClaimsView::Detail(_)) => (
                self.detail
                    .as_ref()
                    .map_or_else(|| "Expense claim".to_owned(), |d| d.claim.purpose.clone()),
                String::new(),
            ),
            (SpendTab::Claims, ClaimsView::List) => (
                "Expense claims".to_owned(),
                "Money staff spent from their own pockets: submitted, approved by someone else, \
                 posted to the books and paid back."
                    .to_owned(),
            ),
        };
        let action: AnyElement = match (self.tab, &self.view) {
            (SpendTab::Claims, ClaimsView::List) => Button::new("open-new")
                .primary()
                .label("New expense claim")
                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                    panel.open_new(window, cx);
                    cx.notify();
                }))
                .into_any_element(),
            (SpendTab::Claims, _) => Button::new("back")
                .ghost()
                .label("All claims")
                .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                    panel.view = ClaimsView::List;
                    panel.form = None;
                    panel.derive();
                    cx.notify();
                }))
                .into_any_element(),
            (SpendTab::Bills, _) => bills_action.unwrap_or_else(|| div().into_any_element()),
        };
        let body = if self.books.is_none() {
            div()
                .text_color(p.ink_muted.to_hsla())
                .child("No books loaded for this organisation.")
                .into_any_element()
        } else {
            match (self.tab, self.view.clone()) {
                (SpendTab::Bills, _) => self.bills.update(cx, BillsPanel::body),
                (SpendTab::Claims, ClaimsView::List) => self.list(cx),
                (SpendTab::Claims, ClaimsView::New) => self.new_form(cx),
                (SpendTab::Claims, ClaimsView::Detail(_)) => self.detail_view(cx),
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
            .child(self.tabs(cx))
            .child(body)
    }
}
