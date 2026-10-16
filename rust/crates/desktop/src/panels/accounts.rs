//! The Accounts screen: the chart of accounts with its add and edit forms, and the fiscal
//! periods with close, reopen, lock and the year-end close, as two tabs of one nav entry.
//!
//! Everything shown comes from the view models in [`crate::accounts`]; this file keeps the
//! form widgets and draws. Writes go through the store and the new books come back through
//! its change channel (README.md, "How a screen replaces its placeholder"). Components:
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/checkbox>, <https://gpui-kit.com/component/button>.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{AccountId, AccountType, ExpenseFunction, IsoDate, OrganizationId};
use ledger_core::{LedgerCommand, LedgerEnv as _, LedgerState};

use crate::accounts::{
    ACCOUNT_TYPES, AccountRow, EXPENSE_FUNCTIONS, EditAccountForm, Failure, FieldErrors,
    NewAccountForm, Pending, PeriodAction, PeriodRow, YearView, account_rows, accounts_meta,
    fiscal_years, function_label, next_year, normal_balance_label, open_year, period_status_label,
    type_label,
};
use crate::backend::{BackendError, SystemEnv};
use crate::panels::{demo_tag, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{palette, space, text};

/// The two tabs of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// The chart of accounts.
    Accounts,
    /// Fiscal years and periods.
    Periods,
}

/// The parent picker's items: `None, top level`, then `code · name` of each header.
type ParentItems = Vec<SharedString>;

/// The label of the top-level choice in the parent picker (`accounts.tsx:388`).
const TOP_LEVEL: &str = "None, top level";

/// The Accounts screen.
pub struct AccountsPanel {
    store: Entity<LedgerStore>,
    /// Organisation and revision the rows were derived from.
    seen: Option<(Option<OrganizationId>, u64)>,
    tab: Tab,
    books: Option<Arc<LedgerState>>,
    demo: bool,
    as_of: Option<IsoDate>,
    rows: Vec<AccountRow>,
    years: Vec<YearView>,
    /// A refused Deactivate or Reactivate.
    failure: Option<Failure>,

    add: Option<NewAccountForm>,
    add_errors: FieldErrors,
    add_failure: Option<Failure>,
    parents: Vec<(AccountId, String)>,
    code: Entity<InputState>,
    name: Entity<InputState>,
    parent: Entity<SelectState<ParentItems>>,

    edit: Option<EditAccountForm>,
    edit_failure: Option<Failure>,
    edit_name: Entity<InputState>,

    pending: Option<Pending>,
    pending_failure: Option<Failure>,
    reason_error: Option<String>,
    reason: Entity<InputState>,
    period_failure: Option<Failure>,
    done: Option<String>,

    _subscriptions: Vec<Subscription>,
}

impl AccountsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let code = cx.new(|cx| InputState::new(window, cx).placeholder("5150"));
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Account name"));
        let edit_name = cx.new(|cx| InputState::new(window, cx));
        let reason = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Why the period needs entries again")
        });
        let parent =
            cx.new(|cx| SelectState::new(vec![SharedString::from(TOP_LEVEL)], None, window, cx));
        let subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&code, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.name.update(cx, |input, cx| input.focus(window, cx));
                }
            }),
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit_add(window, cx);
                }
            }),
            cx.subscribe_in(&edit_name, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.save_edit(cx);
                }
            }),
            cx.subscribe_in(&reason, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.confirm(cx);
                }
            }),
            cx.subscribe_in(&parent, window, Self::on_parent),
        ];
        let mut panel = Self {
            store,
            seen: None,
            tab: Tab::Accounts,
            books: None,
            demo: false,
            as_of: None,
            rows: Vec::new(),
            years: Vec::new(),
            failure: None,
            add: None,
            add_errors: FieldErrors::default(),
            add_failure: None,
            parents: Vec::new(),
            code,
            name,
            parent,
            edit: None,
            edit_failure: None,
            edit_name,
            pending: None,
            pending_failure: None,
            reason_error: None,
            reason,
            period_failure: None,
            done: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Re-derives the rows when the organisation or its books changed.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
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
        self.as_of = if self.demo {
            IsoDate::parse(DEMO_AS_OF).ok()
        } else {
            IsoDate::parse(&SystemEnv.now().as_str()[..10]).ok()
        };
        if let Some(books) = self.books.as_deref() {
            self.rows = account_rows(books, self.as_of.as_ref());
            self.years = fiscal_years(books);
        } else {
            self.rows.clear();
            self.years.clear();
        }
        if org_changed {
            self.add = None;
            self.edit = None;
            self.pending = None;
            self.failure = None;
            self.period_failure = None;
            self.done = None;
        }
        if self.add.is_some() {
            self.load_parents(window, cx);
        }
        if let Some(edit) = &self.edit
            && !self.rows.iter().any(|r| r.account.id == edit.account.id)
        {
            self.edit = None;
        }
        cx.notify();
    }

    /// Keys while the screen shows and no field has focus: `a` the chart, `p` the periods,
    /// `n` the add form with the code field focused.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "a" => self.tab = Tab::Accounts,
            "p" => self.tab = Tab::Periods,
            "n" => {
                self.tab = Tab::Accounts;
                self.open_add(window, cx);
                self.code.update(cx, |input, cx| input.focus(window, cx));
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    fn submit(&self, command: LedgerCommand, cx: &mut Context<Self>) -> Result<(), BackendError> {
        self.store
            .update(cx, |store, _| store.submit(command))
            .map(|_| ())
    }

    // Accounts tab.

    fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.add.is_none() {
            self.add = Some(NewAccountForm::default());
            self.add_errors = FieldErrors::default();
            self.add_failure = None;
            self.code
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.name
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.load_parents(window, cx);
        }
        self.edit = None;
    }

    /// Fills the parent picker with the headers of the form's type; the choice resets.
    fn load_parents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(form), Some(books)) = (&self.add, self.books.as_deref()) else {
            return;
        };
        self.parents = form.parents(books);
        let mut items = vec![SharedString::from(TOP_LEVEL)];
        items.extend(self.parents.iter().map(|(_, label)| label.clone().into()));
        let selected = form.parent.as_ref().and_then(|id| {
            self.parents
                .iter()
                .position(|(p, _)| p == id)
                .map(|i| gpui_kit::component::IndexPath::new(i + 1))
        });
        self.parent.update(cx, |select, cx| {
            select.set_items(items, window, cx);
            select.set_selected_index(selected, window, cx);
        });
    }

    fn on_parent(
        &mut self,
        _: &Entity<SelectState<ParentItems>>,
        event: &SelectEvent<ParentItems>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(choice) = event;
        if let Some(form) = &mut self.add {
            form.parent = choice.as_ref().and_then(|label| {
                self.parents
                    .iter()
                    .find(|(_, l)| l.as_str() == label.as_ref())
                    .map(|(id, _)| id.clone())
            });
            cx.notify();
        }
    }

    fn set_type(&mut self, t: AccountType, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.add {
            form.set_type(t);
            self.load_parents(window, cx);
            cx.notify();
        }
    }

    fn submit_add(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &mut self.add else {
            return;
        };
        form.code = self.code.read(cx).value().to_string();
        form.name = self.name.read(cx).value().to_string();
        match form.command() {
            Err(errors) => {
                self.add_errors = errors;
                self.add_failure = None;
            }
            Ok(command) => {
                self.add_errors = FieldErrors::default();
                match self.submit(command, cx) {
                    Ok(()) => {
                        self.add = None;
                        self.add_failure = None;
                    }
                    Err(err) => self.add_failure = Some(Failure::new("Not added", &err)),
                }
            }
        }
        cx.notify();
    }

    fn open_edit(&mut self, id: &AccountId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|r| &r.account.id == id) else {
            return;
        };
        let form = EditAccountForm::new(&row.account);
        let name = form.name.clone();
        self.edit_name
            .update(cx, |input, cx| input.set_value(name, window, cx));
        self.edit = Some(form);
        self.edit_failure = None;
        self.add = None;
        cx.notify();
    }

    fn save_edit(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.edit else {
            return;
        };
        form.name = self.edit_name.read(cx).value().to_string();
        match form.command() {
            None => self.edit = None,
            Some(command) => match self.submit(command, cx) {
                Ok(()) => self.edit = None,
                Err(err) => self.edit_failure = Some(Failure::new("Not changed", &err)),
            },
        }
        cx.notify();
    }

    fn toggle(&mut self, id: &AccountId, cx: &mut Context<Self>) {
        let Some(command) = self
            .rows
            .iter()
            .find(|r| &r.account.id == id)
            .map(AccountRow::toggle_command)
        else {
            return;
        };
        self.failure = self
            .submit(command, cx)
            .err()
            .map(|err| Failure::new("Not changed", &err));
        cx.notify();
    }

    // Periods tab.

    fn period_action(
        &mut self,
        action: PeriodAction,
        row: &PeriodRow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.done = None;
        self.period_failure = None;
        match Pending::of(action, row) {
            None => match self.submit(row.close_command(), cx) {
                Ok(()) => self.done = Some(row.closed_notice()),
                Err(err) => self.period_failure = Some(Failure::new("Not closed", &err)),
            },
            Some(pending) => self.open_pending(pending, window, cx),
        }
        cx.notify();
    }

    fn open_pending(&mut self, pending: Pending, window: &mut Window, cx: &mut Context<Self>) {
        self.reason
            .update(cx, |input, cx| input.set_value("", window, cx));
        if pending.needs_reason() {
            self.reason.update(cx, |input, cx| input.focus(window, cx));
        }
        self.reason_error = None;
        self.pending_failure = None;
        self.pending = Some(pending);
        cx.notify();
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = &self.pending else {
            return;
        };
        let reason = self.reason.read(cx).value().to_string();
        match pending.command(&reason) {
            Err(message) => self.reason_error = Some(message),
            Ok((command, notice)) => match self.submit(command, cx) {
                Ok(()) => {
                    self.pending = None;
                    self.done = Some(notice);
                }
                Err(err) => self.pending_failure = Some(Failure::new("Not done", &err)),
            },
        }
        cx.notify();
    }

    fn open_next_year(&mut self, cx: &mut Context<Self>) {
        let Some(year) = next_year(&self.years) else {
            return;
        };
        let (command, notice) = open_year(year);
        self.period_failure = None;
        self.done = None;
        match self.submit(command, cx) {
            Ok(()) => self.done = Some(notice),
            Err(err) => self.period_failure = Some(Failure::new("Not opened", &err)),
        }
        cx.notify();
    }

    // Drawing.

    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let tab = |id: &'static str, label: &'static str, key: &'static str, this: Tab| {
            let active = self.tab == this;
            h_flex()
                .id(id)
                .h(px(space::NAV_ITEM))
                .px(px(14.0))
                .gap(px(8.0))
                .cursor_pointer()
                .text_size(px(text::BODY))
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
            .child(tab("tab-accounts", "Chart of accounts", "a", Tab::Accounts))
            .child(tab("tab-periods", "Periods", "p", Tab::Periods))
    }

    fn meta(&self, extra: Option<String>, cx: &App) -> Div {
        let p = palette(cx.theme().mode);
        h_flex()
            .gap(px(12.0))
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .when_some(extra, ParentElement::child)
            .when_some(self.as_of.clone(), |m, d| {
                m.child(format!("As of {}", crate::accounts::format_date(&d)))
            })
            .when(self.demo, |m| m.child(demo_tag(cx)))
    }

    fn accounts_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let header = h_flex()
            .h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(col_code().child("Code"))
            .child(col_name().child("Account"))
            .child(col_type().child("Type"))
            .child(col_normal().child("Normal"))
            .child(col_function().child("Function"))
            .child(col_balance().child("Balance"))
            .child(col_actions());
        let rows: Vec<AnyElement> = self
            .rows
            .iter()
            .map(|row| Self::account_row(row, cx))
            .collect();
        v_flex()
            .gap(px(space::SECTION))
            .when_some(self.failure.clone(), |v, f| v.child(failure_box(&f, cx)))
            .when(self.add.is_some(), |v| v.child(self.add_form(cx)))
            .when(self.edit.is_some(), |v| v.child(self.edit_form(cx)))
            .child(
                v_flex()
                    .child(section_header(
                        "Accounts",
                        self.meta(Some(accounts_meta(&self.rows)), cx),
                        cx,
                    ))
                    .child(header)
                    .when(self.rows.is_empty(), |v| {
                        v.child(
                            div().py(px(space::CELL)).child(
                                "No accounts yet. Add the first account to start the books.",
                            ),
                        )
                    })
                    .children(rows),
            )
            .into_any_element()
    }

    fn account_row(row: &AccountRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let a = &row.account;
        let muted = !a.is_active;
        let id = a.id.clone();
        let edit_id = a.id.clone();
        let tags = row.tags().into_iter().map(|t| tag(t, cx));
        h_flex()
            .min_h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .when(a.is_group && row.depth == 0, |r| {
                r.border_t(px(2.0)).border_color(p.ink_navy.to_hsla())
            })
            .when(muted, |r| r.text_color(p.ink_muted.to_hsla()))
            .child(col_code().text_size(px(text::MONO)).child(a.code.clone()))
            .child(
                col_name().child(
                    h_flex()
                        .pl(px(20.0 * f32::from(u16::try_from(row.depth).unwrap_or(0))))
                        .gap(px(8.0))
                        .flex_wrap()
                        .child(
                            div()
                                .when(a.is_group, |d| d.font_weight(FontWeight::SEMIBOLD))
                                .child(a.name.clone()),
                        )
                        .children(tags),
                ),
            )
            .child(col_type().child(type_label(a.account_type)))
            .child(col_normal().child(normal_balance_label(a.normal_balance)))
            .child(
                col_function()
                    .text_color(p.ink_muted.to_hsla())
                    .child(row.function_text()),
            )
            .child(
                col_balance()
                    .when(a.is_group, |c| c.font_weight(FontWeight::SEMIBOLD))
                    .child(row.balance_text()),
            )
            .child(
                col_actions()
                    .child(
                        Button::new(SharedString::from(format!("edit-{}", a.id.as_str())))
                            .ghost()
                            .small()
                            .label("Edit")
                            .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                                panel.open_edit(&edit_id, window, cx);
                            })),
                    )
                    .when_some(row.toggle_label(), |c, label| {
                        c.child(
                            Button::new(SharedString::from(format!("toggle-{}", a.id.as_str())))
                                .ghost()
                                .small()
                                .label(label)
                                .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                                    panel.toggle(&id, cx);
                                })),
                        )
                    }),
            )
            .into_any_element()
    }

    fn add_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(form) = &self.add else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        let (types, functions) = Self::add_choices(form, cx);
        let checks = Self::add_checks(form, cx);
        v_flex()
            .child(section_header("Add an account", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(
                                "New accounts are active straight away. An account cannot be \
                                 deleted once added, only deactivated.",
                            ),
                    )
                    .when_some(self.add_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(
                        h_flex()
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                field(
                                    "Code",
                                    Input::new(&self.code),
                                    self.add_errors.code.clone(),
                                    cx,
                                )
                                .w(px(140.0)),
                            )
                            .child(
                                field(
                                    "Name",
                                    Input::new(&self.name),
                                    self.add_errors.name.clone(),
                                    cx,
                                )
                                .flex_1(),
                            ),
                    )
                    .child(field(
                        "Type",
                        h_flex().gap(px(6.0)).flex_wrap().children(types),
                        None,
                        cx,
                    ))
                    .child(
                        field(
                            "Parent",
                            v_flex()
                                .gap(px(6.0))
                                .child(Select::new(&self.parent).placeholder(TOP_LEVEL))
                                .child(hint("Header accounts of the same type", cx)),
                            None,
                            cx,
                        )
                        .max_w(px(420.0)),
                    )
                    .child(checks)
                    .when(form.shows_function(), |v| {
                        v.child(field(
                            "Expense function",
                            h_flex().gap(px(6.0)).children(functions),
                            None,
                            cx,
                        ))
                    })
                    .child(Self::form_buttons(
                        "add",
                        "Add account",
                        Self::submit_add,
                        |panel| panel.add = None,
                        cx,
                    )),
            )
            .into_any_element()
    }

    /// The type and function choices of the add form (`accounts.tsx:375-417`).
    fn add_choices(form: &NewAccountForm, cx: &mut Context<Self>) -> (Vec<Button>, Vec<Button>) {
        let types = ACCOUNT_TYPES.into_iter().map(|t| {
            choice(
                format!("type-{}", type_label(t)),
                type_label(t),
                form.account_type == t,
            )
            .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                panel.set_type(t, window, cx);
            }))
        });
        let functions = EXPENSE_FUNCTIONS.into_iter().map(|f| {
            choice(
                format!("fn-{}", function_label(f)),
                function_label(f),
                form.expense_function == f,
            )
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                set_function(panel, f, cx);
            }))
        });
        (types.collect(), functions.collect())
    }

    /// A form's primary button and its Cancel: `submit` on the first, `cancel` then a redraw on
    /// the second.
    fn form_buttons(
        id: &'static str,
        label: &'static str,
        submit: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cancel: impl Fn(&mut Self) + 'static,
        cx: &mut Context<Self>,
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

    /// The header and cash checkboxes of the add form (`accounts.tsx:391-406`).
    fn add_checks(form: &NewAccountForm, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap(px(space::CELL))
            .child(
                Checkbox::new("add-group")
                    .label(
                        "Header account: groups other accounts; entries cannot post to it \
                         directly.",
                    )
                    .checked(form.is_group)
                    .on_click(cx.listener(|panel, checked: &bool, _, cx| {
                        if let Some(form) = &mut panel.add {
                            form.is_group = *checked;
                            cx.notify();
                        }
                    })),
            )
            .when(form.shows_cash(), |v| {
                v.child(
                    Checkbox::new("add-cash")
                        .label("Cash account: a bank, mobile money or petty cash account.")
                        .checked(form.is_cash)
                        .on_click(cx.listener(|panel, checked: &bool, _, cx| {
                            if let Some(form) = &mut panel.add {
                                form.is_cash = *checked;
                                cx.notify();
                            }
                        })),
                )
            })
    }

    fn edit_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(form) = &self.edit else {
            return div().into_any_element();
        };
        let functions = EXPENSE_FUNCTIONS.into_iter().map(|f| {
            choice(
                format!("edit-fn-{}", function_label(f)),
                function_label(f),
                form.expense_function == Some(f),
            )
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                if let Some(form) = &mut panel.edit {
                    form.expense_function = Some(f);
                    cx.notify();
                }
            }))
        });
        let title = format!("Edit {} {}", form.account.code, form.account.name);
        v_flex()
            .child(section_header(&title, div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .when_some(self.edit_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(field("Name", Input::new(&self.edit_name), None, cx).max_w(px(480.0)))
                    .child(
                        Checkbox::new("edit-active")
                            .label("Active: takes new lines")
                            .checked(form.is_active)
                            .on_click(cx.listener(|panel, checked: &bool, _, cx| {
                                if let Some(form) = &mut panel.edit {
                                    form.is_active = *checked;
                                    cx.notify();
                                }
                            })),
                    )
                    .when(form.shows_function(), |v| {
                        v.child(field(
                            "Expense function",
                            h_flex().gap(px(6.0)).children(functions),
                            None,
                            cx,
                        ))
                    })
                    .child(Self::form_buttons(
                        "edit",
                        "Save changes",
                        |panel, _, cx| panel.save_edit(cx),
                        |panel| panel.edit = None,
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn periods_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let years: Vec<AnyElement> = self.years.iter().map(|y| self.year(y, cx)).collect();
        v_flex()
            .gap(px(space::SECTION))
            .when_some(self.period_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .when_some(self.done.clone(), |v, done| {
                v.child(
                    div()
                        .p(px(16.0))
                        .border_1()
                        .border_color(p.hairline.to_hsla())
                        .child(done),
                )
            })
            .when(self.pending.is_some(), |v| v.child(self.pending_box(cx)))
            .child(
                h_flex()
                    .gap(px(24.0))
                    .flex_wrap()
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child("Open: takes entries")
                    .child("Closed: can be reopened with a reason")
                    .child("Locked: sealed for good"),
            )
            .when(self.years.is_empty(), |v| {
                v.child("No fiscal periods. Periods are created when the books are set up.")
            })
            .children(years)
            .into_any_element()
    }

    fn year(&self, y: &YearView, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let pending = y.close_pending();
        let action: AnyElement = if y.closed {
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Year closed")
                .into_any_element()
        } else {
            Button::new(SharedString::from(format!("close-year-{}", y.year)))
                .outline()
                .small()
                .label("Close the year")
                .disabled(pending.is_none())
                .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                    if let Some(pending) = pending.clone() {
                        panel.done = None;
                        panel.open_pending(pending, window, cx);
                    }
                }))
                .into_any_element()
        };
        let rows: Vec<AnyElement> = y.periods.iter().map(|r| Self::period(r, cx)).collect();
        v_flex()
            .child(section_header(
                &y.title(),
                self.meta(None, cx).child(action),
                cx,
            ))
            .when_some(y.note(), |v, note| {
                v.child(
                    div()
                        .py(px(8.0))
                        .border_b(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(note),
                )
            })
            .children(rows)
            .into_any_element()
    }

    fn period(r: &PeriodRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let status = r.period.status;
        let buttons = r.actions().iter().map(|&action| {
            let row = r.clone();
            let button = Button::new(SharedString::from(format!(
                "{}-{}",
                action.label(),
                r.period.id.as_str()
            )))
            .small()
            .label(action.label())
            .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                panel.period_action(action, &row, window, cx);
            }));
            if action == PeriodAction::Reopen {
                button.ghost()
            } else {
                button.outline()
            }
        });
        h_flex()
            .min_h(px(52.0))
            .gap(px(16.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                v_flex()
                    .w(px(180.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.period.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(r.dates_text()),
                    ),
            )
            .child(status_mark(status, cx))
            .child(
                div()
                    .flex_1()
                    .text_color(p.ink_muted.to_hsla())
                    .child(r.counts_text()),
            )
            .child(h_flex().gap(px(8.0)).children(buttons))
            .into_any_element()
    }

    fn pending_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(pending) = &self.pending else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        v_flex()
            .p(px(16.0))
            .gap(px(12.0))
            .border(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::BOLD)
                    .child(pending.title()),
            )
            .when_some(self.pending_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .children(pending.body().into_iter().map(|line| div().child(line)))
            .when(pending.needs_reason(), |v| {
                v.child(
                    field(
                        "Reason",
                        Input::new(&self.reason),
                        self.reason_error.clone(),
                        cx,
                    )
                    .max_w(px(560.0)),
                )
            })
            .child(Self::form_buttons(
                "pending",
                pending.confirm_label(),
                |panel, _, cx| panel.confirm(cx),
                |panel| panel.pending = None,
                cx,
            ))
            .into_any_element()
    }
}

/// Sets the add form's function.
fn set_function(panel: &mut AccountsPanel, f: ExpenseFunction, cx: &mut Context<AccountsPanel>) {
    if let Some(form) = &mut panel.add {
        form.expense_function = f;
        cx.notify();
    }
}

fn col_code() -> Div {
    div().w(px(72.0)).flex_shrink_0().px(px(8.0))
}

fn col_name() -> Div {
    div().flex_1().min_w(px(200.0)).px(px(8.0)).py(px(6.0))
}

fn col_type() -> Div {
    div().w(px(96.0)).flex_shrink_0().px(px(8.0))
}

fn col_normal() -> Div {
    div().w(px(72.0)).flex_shrink_0().px(px(8.0))
}

fn col_function() -> Div {
    div().w(px(168.0)).flex_shrink_0().px(px(8.0))
}

fn col_balance() -> Div {
    div()
        .w(px(128.0))
        .flex_shrink_0()
        .px(px(8.0))
        .flex()
        .justify_end()
}

fn col_actions() -> Div {
    div()
        .w(px(168.0))
        .flex_shrink_0()
        .flex()
        .justify_end()
        .gap(px(4.0))
}

/// A one-of-many choice as a button: the chosen one filled, the others outlined.
fn choice(id: String, label: &'static str, chosen: bool) -> Button {
    let button = Button::new(SharedString::from(id)).small().label(label);
    if chosen {
        button.primary()
    } else {
        button.outline()
    }
}

/// A labelled field: the label, the control, and the error under it in `rejection_red`.
fn field(label: &str, control: impl IntoElement, error: Option<String>, cx: &App) -> Div {
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

fn hint(text_: &str, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(palette(cx.theme().mode).ink_muted.to_hsla())
        .child(text_.to_owned())
}

/// `Tag` (`accounts.tsx:218-224`): a hairline box in small muted capitals.
fn tag(label: &str, cx: &App) -> impl IntoElement {
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

/// `PeriodStatusMark` (`-shared.tsx:84-97`): a line mark and the label; open is dashed and
/// muted, locked heavy.
fn status_mark(status: ledger_core::types::PeriodStatus, cx: &App) -> impl IntoElement {
    use ledger_core::types::PeriodStatus;
    let p = palette(cx.theme().mode);
    let ink = p.ink_navy.to_hsla();
    let mark = match status {
        PeriodStatus::Open => h_flex()
            .gap(px(3.0))
            .children((0..3).map(|_| div().w(px(5.0)).h(px(2.0)).bg(p.ink_muted.to_hsla()))),
        PeriodStatus::Closed => h_flex().child(div().w(px(21.0)).h(px(2.0)).bg(ink)),
        PeriodStatus::Locked => h_flex()
            .items_center()
            .child(div().w(px(2.0)).h(px(10.0)).bg(ink))
            .child(div().w(px(17.0)).h(px(4.0)).bg(ink))
            .child(div().w(px(2.0)).h(px(10.0)).bg(ink)),
    };
    h_flex()
        .w(px(110.0))
        .gap(px(8.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .when(status == PeriodStatus::Open, |m| {
            m.text_color(p.ink_muted.to_hsla())
        })
        .child(mark)
        .child(period_status_label(status))
}

/// `IssueSummary` (`flowtrace/src/components/ledger/page.tsx:156-190`): a red-ruled box with the
/// title and each engine message as given.
fn failure_box(f: &Failure, cx: &App) -> impl IntoElement {
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

impl Render for AccountsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let (title, lede) = match self.tab {
            Tab::Accounts => (
                "Chart of accounts",
                "The accounts every entry posts to, grouped under header accounts. Balances are \
                 the natural way round: credit accounts such as income and liabilities show as \
                 positive.",
            ),
            Tab::Periods => (
                "Periods",
                "Each month is a fiscal period. Close a month once its entries are complete, so \
                 nothing is backdated into figures already reported. Lock it once the accounts \
                 for it are signed off.",
            ),
        };
        let action: AnyElement = match self.tab {
            Tab::Accounts => Button::new("open-add")
                .primary()
                .label("Add an account")
                .disabled(self.books.is_none())
                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                    panel.open_add(window, cx);
                    panel.code.update(cx, |input, cx| input.focus(window, cx));
                    cx.notify();
                }))
                .into_any_element(),
            Tab::Periods => match next_year(&self.years) {
                Some(year) => Button::new("open-year")
                    .outline()
                    .label(format!("Open fiscal year {year}"))
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                        panel.open_next_year(cx);
                    }))
                    .into_any_element(),
                None => div().into_any_element(),
            },
        };
        let body = if self.books.is_none() {
            div()
                .text_color(p.ink_muted.to_hsla())
                .child("No books loaded for this organisation.")
                .into_any_element()
        } else {
            match self.tab {
                Tab::Accounts => self.accounts_tab(cx),
                Tab::Periods => self.periods_tab(cx),
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
                            .child(page_title(title))
                            .child(div().text_size(px(text::LEDE)).child(lede)),
                    )
                    .child(action),
            )
            .child(self.tabs(cx))
            .child(body)
    }
}
