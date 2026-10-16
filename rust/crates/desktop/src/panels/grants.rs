//! The Grants screen (U7): the grants list with its totals, warnings and add form; one grant's
//! detail; and the budget against actual with its filters, chart, table, change and add forms,
//! as the web's `grants/index.tsx`, `grants/$id.tsx` and `grants/budgets.tsx` lay them out.
//!
//! Everything shown comes from the view models in [`crate::grants`]; this file keeps the form
//! widgets and draws. Writes go through the store and the new books come back through its
//! change channel (README.md, "How a screen replaces its placeholder"). The budget chart is
//! gpui-kit's `BarChart` (<https://gpui-kit.com/component/chart>), horizontal, one budget bar
//! and one actual bar per line; the table reuses the Reports statement table. Components:
//! <https://gpui-kit.com/component/input>, <https://gpui-kit.com/component/select>,
//! <https://gpui-kit.com/component/button>.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::chart::BarChart;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::plot::shape::BarAlignment;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window, div, px, relative,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::reports::{Basis, WarningLevel};
use ledger_core::types::{
    AccountId, BudgetId, DimensionId, FundId, GrantId, IsoDate, LedgerState, OrganizationId,
};
use ledger_core::{LedgerCommand, Minor};

use crate::accounts::{Failure, format_date, format_minor};
use crate::backend::BackendError;
use crate::claims::{format_whole, fund_choices, grant_choices};
use crate::grants::{
    Bar, BarKind, BudgetFilter, BudgetLineForm, BudgetRow, BudgetView, GrantDetail, GrantErrors,
    GrantRow, GrantTotals, LineEdit, NewGrantForm, budget_view, expense_choices, grant_detail,
    grant_rows, grant_totals, totals_figures,
};
use crate::panels::reports::{key_figures, segment, statement_table};
use crate::panels::{demo_tag, failure_box, field, form_buttons, hint, page_title, section_header};
use crate::statements::{format_compact, today_utc};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantsView {
    /// Every grant.
    List,
    /// One grant.
    Detail(GrantId),
    /// The budget against actual.
    Budgets,
}

impl GrantsView {
    /// The view a `--grants` launch value names: `list`, `budgets` or a grant id.
    #[must_use]
    pub fn from_launch(value: &str) -> Self {
        match value {
            "list" => Self::List,
            "budgets" => Self::Budgets,
            id => Self::Detail(GrantId::from(id)),
        }
    }
}

/// A picker's items: labels in the order of its choices.
type Items = Vec<SharedString>;

/// A select over ids: its widget and the ids its labels stand for (`None` for the "all" or
/// "none" choice).
struct Picker<T> {
    state: Entity<SelectState<Items>>,
    choices: Vec<(Option<T>, String)>,
}

impl<T: Clone + PartialEq + 'static> Picker<T> {
    fn new(window: &mut Window, cx: &mut Context<GrantsPanel>) -> Self {
        Self {
            state: cx.new(|cx| SelectState::new(Items::new(), None, window, cx)),
            choices: Vec::new(),
        }
    }

    /// Fills the select with `choices`, showing `chosen`.
    fn load(
        &mut self,
        choices: Vec<(Option<T>, String)>,
        chosen: Option<&T>,
        window: &mut Window,
        cx: &mut Context<GrantsPanel>,
    ) {
        let items: Items = choices.iter().map(|(_, l)| l.clone().into()).collect();
        let index = choices
            .iter()
            .position(|(id, _)| id.as_ref() == chosen)
            .map(IndexPath::new);
        self.choices = choices;
        self.state.update(cx, |select, cx| {
            select.set_items(items, window, cx);
            select.set_selected_index(index, window, cx);
        });
    }

    /// The id a confirmed label stands for: `Some(None)` is the "all" or "none" choice.
    #[expect(
        clippy::option_option,
        reason = "nothing picked versus the none choice"
    )]
    fn pick(&self, event: &SelectEvent<Items>) -> Option<Option<T>> {
        let SelectEvent::Confirm(label) = event;
        let label = label.as_ref()?;
        self.choices
            .iter()
            .find(|(_, l)| l.as_str() == label.as_ref())
            .map(|(id, _)| id.clone())
    }
}

/// `(Some(id), label)` choices after one `(None, none)` choice.
fn with_none<T>(none: &str, choices: Vec<(T, String)>) -> Vec<(Option<T>, String)> {
    std::iter::once((None, none.to_owned()))
        .chain(choices.into_iter().map(|(id, l)| (Some(id), l)))
        .collect()
}

/// The Grants screen.
pub struct GrantsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    view: GrantsView,
    books: Option<Arc<LedgerState>>,
    demo: bool,
    as_of: IsoDate,
    base: String,
    rows: Result<Vec<GrantRow>, String>,
    totals: GrantTotals,
    detail: Option<Result<GrantDetail, String>>,
    filter: BudgetFilter,
    budget: Option<BudgetView>,

    add: Option<NewGrantForm>,
    add_errors: GrantErrors,
    add_failure: Option<Failure>,
    name: Entity<InputState>,
    donor: Entity<InputState>,
    award: Entity<InputState>,
    currency: Entity<InputState>,
    start: Entity<InputState>,
    end: Entity<InputState>,
    terms: Entity<InputState>,
    grant_fund: Picker<FundId>,

    budget_pick: Picker<BudgetId>,
    filter_grant: Picker<GrantId>,
    filter_dimension: Picker<DimensionId>,

    line: Option<BudgetLineForm>,
    line_error: Option<String>,
    line_failure: Option<Failure>,
    line_amount: Entity<InputState>,
    line_account: Picker<AccountId>,
    line_fund: Picker<FundId>,
    line_grant: Picker<GrantId>,
    line_dimension: Picker<DimensionId>,

    edit: Option<LineEdit>,
    edit_error: Option<String>,
    edit_failure: Option<Failure>,
    edit_line: Picker<usize>,
    edit_amount: Entity<InputState>,
    saved: Option<String>,

    _subscriptions: Vec<Subscription>,
}

impl GrantsPanel {
    /// The screen over `store`.
    #[expect(
        clippy::too_many_lines,
        reason = "one subscription per widget of the screen"
    )]
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
            let placeholder = placeholder.to_owned();
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let name = input("Grant name", window, cx);
        let donor = input("Donor", window, cx);
        let award = input("250000", window, cx);
        let currency = input("KES", window, cx);
        let start = input("YYYY-MM-DD", window, cx);
        let end = input("YYYY-MM-DD", window, cx);
        let terms = input("Restrictions and reporting terms", window, cx);
        let line_amount = input("250000", window, cx);
        let edit_amount = input("250000", window, cx);
        let grant_fund = Picker::new(window, cx);
        let budget_pick = Picker::new(window, cx);
        let filter_grant = Picker::new(window, cx);
        let filter_dimension = Picker::new(window, cx);
        let line_account = Picker::new(window, cx);
        let line_fund = Picker::new(window, cx);
        let line_grant = Picker::new(window, cx);
        let line_dimension = Picker::new(window, cx);
        let edit_line = Picker::new(window, cx);

        let subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&grant_fund.state, window, |this, _, event, _, cx| {
                if let (Some(fund), Some(form)) = (this.grant_fund.pick(event), &mut this.add) {
                    form.fund = fund;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&budget_pick.state, window, |this, _, event, window, cx| {
                if let Some(id) = this.budget_pick.pick(event) {
                    this.filter.budget = id;
                    this.rederive(window, cx);
                }
            }),
            cx.subscribe_in(&filter_grant.state, window, |this, _, event, window, cx| {
                if let Some(id) = this.filter_grant.pick(event) {
                    this.filter.grant = id;
                    this.rederive(window, cx);
                }
            }),
            cx.subscribe_in(
                &filter_dimension.state,
                window,
                |this, _, event, window, cx| {
                    if let Some(id) = this.filter_dimension.pick(event) {
                        this.filter.dimension = id;
                        this.rederive(window, cx);
                    }
                },
            ),
            cx.subscribe_in(&line_account.state, window, |this, _, event, _, cx| {
                if let (Some(id), Some(form)) = (this.line_account.pick(event), &mut this.line) {
                    form.account = id;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&line_fund.state, window, |this, _, event, window, cx| {
                let (Some(id), Some(books)) = (this.line_fund.pick(event), this.books.clone())
                else {
                    return;
                };
                if let Some(form) = &mut this.line {
                    form.set_fund(id, &books);
                    this.load_line_grants(window, cx);
                    cx.notify();
                }
            }),
            cx.subscribe_in(&line_grant.state, window, |this, _, event, _, cx| {
                if let (Some(id), Some(form)) = (this.line_grant.pick(event), &mut this.line) {
                    form.grant = id;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&line_dimension.state, window, |this, _, event, _, cx| {
                if let (Some(id), Some(form)) = (this.line_dimension.pick(event), &mut this.line) {
                    form.dimension = id;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&edit_line.state, window, |this, _, event, window, cx| {
                if let Some(Some(index)) = this.edit_line.pick(event) {
                    this.open_edit(index, window, cx);
                }
            }),
        ];
        let mut panel = Self {
            store,
            seen: None,
            view: GrantsView::List,
            books: None,
            demo: false,
            as_of: today_utc(),
            base: String::new(),
            rows: Ok(Vec::new()),
            totals: GrantTotals::default(),
            detail: None,
            filter: BudgetFilter::default(),
            budget: None,
            add: None,
            add_errors: GrantErrors::default(),
            add_failure: None,
            name,
            donor,
            award,
            currency,
            start,
            end,
            terms,
            grant_fund,
            budget_pick,
            filter_grant,
            filter_dimension,
            line: None,
            line_error: None,
            line_failure: None,
            line_amount,
            line_account,
            line_fund,
            line_grant,
            line_dimension,
            edit: None,
            edit_error: None,
            edit_failure: None,
            edit_line,
            edit_amount,
            saved: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Opens `view`, for the launch flags and the list's rows.
    pub fn show(&mut self, view: GrantsView, window: &mut Window, cx: &mut Context<Self>) {
        self.view = view;
        self.rederive(window, cx);
    }

    /// Keys while the screen shows and no field has focus: `g` the grants, `b` the budget,
    /// `n` the add-grant form.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "g" => self.show(GrantsView::List, window, cx),
            "b" => self.show(GrantsView::Budgets, window, cx),
            "n" => {
                self.view = GrantsView::List;
                self.open_add(window, cx);
            }
            _ => return false,
        }
        true
    }

    /// Re-derives when the organisation or its books changed.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
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
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        };
        self.base = self
            .books
            .as_ref()
            .map(|b| b.settings.base_currency.clone())
            .unwrap_or_default();
        if org_changed {
            self.view = GrantsView::List;
            self.filter = BudgetFilter::default();
            self.add = None;
            self.line = None;
            self.edit = None;
            self.saved = None;
        }
        self.load_filters(window, cx);
        self.rederive(window, cx);
    }

    /// Derives the list, the open grant and the budget page.
    fn rederive(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            self.rows = Ok(Vec::new());
            self.budget = None;
            self.detail = None;
            cx.notify();
            return;
        };
        self.rows = grant_rows(&books, &self.as_of).map_err(|e| e.to_string());
        self.totals = self.rows.as_deref().map(grant_totals).unwrap_or_default();
        self.detail = match &self.view {
            GrantsView::Detail(id) => {
                Some(grant_detail(&books, id, &self.as_of).map_err(|e| e.to_string()))
            }
            _ => None,
        };
        self.budget = budget_view(&books, &self.filter, &self.as_of);
        let lines: Vec<(Option<usize>, String)> = self
            .budget
            .iter()
            .flat_map(|b| b.rows.iter().enumerate())
            .map(|(i, r)| (Some(i), r.row.label.clone()))
            .collect();
        self.edit_line.load(lines, None, window, cx);
        if let Some(budget) = &self.budget {
            let id = budget.budget.id.clone();
            if let Some(form) = &mut self.line {
                form.budget = id;
            }
        }
        cx.notify();
    }

    /// Fills the budget page's filter pickers from the books.
    fn load_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let mut budgets = books.budgets.clone();
        budgets.sort_by_key(|b| std::cmp::Reverse(b.fiscal_year));
        let budget_choices = budgets.into_iter().map(|b| (Some(b.id), b.name)).collect();
        self.budget_pick
            .load(budget_choices, self.filter.budget.as_ref(), window, cx);
        let grants = grant_choices(&books, None);
        self.filter_grant.load(
            with_none("All lines", grants),
            self.filter.grant.as_ref(),
            window,
            cx,
        );
        let dimensions = books
            .dimensions
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();
        self.filter_dimension.load(
            with_none("All lines", dimensions),
            self.filter.dimension.as_ref(),
            window,
            cx,
        );
    }

    fn submit(&self, command: LedgerCommand, cx: &mut Context<Self>) -> Result<(), BackendError> {
        self.store
            .update(cx, |store, _| store.submit(command))
            .map(|_| ())
    }

    // The add-grant form.

    fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let form = NewGrantForm::new(&books);
        for input in [
            &self.name,
            &self.donor,
            &self.award,
            &self.start,
            &self.end,
            &self.terms,
        ] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        let currency = form.currency.clone();
        self.currency
            .update(cx, |i, cx| i.set_value(currency, window, cx));
        self.grant_fund.load(
            with_none("Choose a fund", fund_choices(&books)),
            None,
            window,
            cx,
        );
        self.add = Some(form);
        self.add_errors = GrantErrors::default();
        self.add_failure = None;
        self.name.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    fn submit_add(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut form) = self.add.clone() else {
            return;
        };
        let value = |input: &Entity<InputState>, cx: &App| input.read(cx).value().to_string();
        form.name = value(&self.name, cx);
        form.donor = value(&self.donor, cx);
        form.award = value(&self.award, cx);
        form.currency = value(&self.currency, cx);
        form.start = value(&self.start, cx);
        form.end = value(&self.end, cx);
        form.terms = value(&self.terms, cx);
        match form.command() {
            Err(errors) => {
                self.add_errors = *errors;
                self.add_failure = None;
                self.add = Some(form);
            }
            Ok(command) => {
                self.add_errors = GrantErrors::default();
                match self.submit(command, cx) {
                    Ok(()) => {
                        self.add = None;
                        self.add_failure = None;
                    }
                    Err(err) => {
                        self.add_failure = Some(Failure::new("Not added", &err));
                        self.add = Some(form);
                    }
                }
            }
        }
        cx.notify();
    }

    // The budget page's forms.

    fn open_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(books), Some(budget)) = (self.books.clone(), &self.budget) else {
            return;
        };
        self.line = Some(BudgetLineForm::new(budget.budget.id.clone()));
        self.line_error = None;
        self.line_failure = None;
        self.saved = None;
        self.line_amount
            .update(cx, |i, cx| i.set_value("", window, cx));
        self.line_account.load(
            with_none("All expense accounts", expense_choices(&books)),
            None,
            window,
            cx,
        );
        self.line_fund.load(
            with_none("Any fund", fund_choices(&books)),
            None,
            window,
            cx,
        );
        let dimensions = books
            .dimensions
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();
        self.line_dimension
            .load(with_none("None", dimensions), None, window, cx);
        self.load_line_grants(window, cx);
        cx.notify();
    }

    /// The grant picker of the add-line form: the grants of the chosen fund (`GrantSelect`).
    fn load_line_grants(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(books), Some(form)) = (self.books.clone(), &self.line) else {
            return;
        };
        let chosen = form.grant.clone();
        self.line_grant.load(
            with_none("No grant", grant_choices(&books, form.fund.as_ref())),
            chosen.as_ref(),
            window,
            cx,
        );
    }

    fn submit_line(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut form) = self.line.clone() else {
            return;
        };
        form.amount = self.line_amount.read(cx).value().to_string();
        match form.command() {
            Err(message) => self.line_error = Some(message),
            Ok(command) => {
                self.line_error = None;
                match self.submit(command, cx) {
                    Ok(()) => {
                        self.line = None;
                        self.line_failure = None;
                        self.saved = Some("Budget line added.".to_owned());
                    }
                    Err(err) => self.line_failure = Some(Failure::new("Line not added", &err)),
                }
            }
        }
        if self.line.is_some() {
            self.line = Some(form);
        }
        cx.notify();
    }

    fn open_edit(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.budget.as_ref().and_then(|b| b.rows.get(index)) else {
            return;
        };
        let edit = LineEdit::new(row);
        let value = edit.value.clone();
        self.edit_amount
            .update(cx, |i, cx| i.set_value(value, window, cx));
        self.edit = Some(edit);
        self.edit_error = None;
        self.edit_failure = None;
        self.saved = None;
        cx.notify();
    }

    fn save_edit(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut edit) = self.edit.clone() else {
            return;
        };
        edit.value = self.edit_amount.read(cx).value().to_string();
        match edit.command(&self.base) {
            Err(message) => self.edit_error = Some(message),
            Ok((command, saved)) => {
                self.edit_error = None;
                match self.submit(command, cx) {
                    Ok(()) => {
                        self.edit = None;
                        self.edit_failure = None;
                        self.saved = Some(saved);
                        cx.notify();
                        return;
                    }
                    Err(err) => self.edit_failure = Some(Failure::new("Budget not changed", &err)),
                }
            }
        }
        self.edit = Some(edit);
        cx.notify();
    }

    // Drawing.

    fn meta(&self, extra: Option<String>, cx: &App) -> Div {
        let p = palette(cx.theme().mode);
        h_flex()
            .gap(px(12.0))
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .when_some(extra, ParentElement::child)
            .child(format!("As of {}", format_date(&self.as_of)))
            .when(self.demo, |m| m.child(demo_tag(cx)))
    }

    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let budgets = self.view == GrantsView::Budgets;
        let tab = |id: &'static str, label: &'static str, key: &'static str, active: bool| {
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
        };
        h_flex()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                tab("tab-grants", "Grants", "g", !budgets).on_click(cx.listener(
                    |panel, _: &ClickEvent, window, cx| panel.show(GrantsView::List, window, cx),
                )),
            )
            .child(
                tab("tab-budgets", "Budget vs actual", "b", budgets).on_click(cx.listener(
                    |panel, _: &ClickEvent, window, cx| {
                        panel.show(GrantsView::Budgets, window, cx);
                    },
                )),
            )
    }

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let rows = match &self.rows {
            Ok(rows) => rows,
            Err(message) => return error_text(message, p).into_any_element(),
        };
        let items: Vec<AnyElement> = rows.iter().map(|row| Self::grant_row(row, cx)).collect();
        v_flex()
            .gap(px(space::SECTION))
            .when(self.add.is_some(), |v| v.child(self.add_form(cx)))
            .child(
                v_flex()
                    .child(section_header("Across all grants", self.meta(None, cx), cx))
                    .child(key_figures(&totals_figures(&self.totals, &self.base), p)),
            )
            .child(
                v_flex()
                    .child(section_header(
                        "Awards",
                        self.meta(Some(format!("{} grants", rows.len())), cx),
                        cx,
                    ))
                    .when(rows.is_empty(), |v| {
                        v.child(notice(
                            "No grants recorded",
                            "Add a grant to start charging spend to it.",
                            p,
                        ))
                    })
                    .children(items)
                    .child(legend(p)),
            )
            .into_any_element()
    }

    fn grant_row(row: &GrantRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let s = &row.summary;
        let id = row.grant.id.clone();
        h_flex()
            .id(SharedString::from(format!("grant-{}", row.grant.id)))
            .py(px(16.0))
            .gap(px(24.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .hover(|r| r.bg(p.wash.to_hsla()))
            .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                panel.show(GrantsView::Detail(id.clone()), window, cx);
            }))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(4.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{} \u{2192}", row.grant.name)),
                    )
                    .child(muted(&row.subtitle(), p))
                    .child(status_text(row.status_line(), s.status, p)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(8.0))
                    .child(coverage_bar(s.award, s.spent + s.committed, s.spent, p))
                    .child(muted(&row.coverage_label(), p))
                    .child(three_figures(
                        [
                            ("Spent".to_owned(), format_whole(s.spent)),
                            ("Committed".to_owned(), format_whole(s.committed)),
                            (
                                format!("Remaining of {}", format_whole(s.award)),
                                format_whole(s.remaining),
                            ),
                        ],
                        p,
                    )),
            )
            .into_any_element()
    }

    fn add_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let e = &self.add_errors;
        let row = || h_flex().gap(px(space::CELL)).items_start();
        v_flex()
            .child(section_header("Add a grant", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(muted(
                        "The award is in minor-unit-free figures, like 250000 for 250,000.00. \
                         The ledger checks the fund, the award and the dates.",
                        p,
                    ))
                    .when_some(self.add_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(
                        row()
                            .child(
                                field("Name", Input::new(&self.name), e.name.clone(), cx).flex_1(),
                            )
                            .child(
                                field("Donor", Input::new(&self.donor), e.donor.clone(), cx)
                                    .flex_1(),
                            ),
                    )
                    .child(
                        row()
                            .child(
                                field(
                                    "Fund",
                                    Select::new(&self.grant_fund.state)
                                        .placeholder("Choose a fund"),
                                    e.fund.clone(),
                                    cx,
                                )
                                .flex_1(),
                            )
                            .child(
                                field("Award", Input::new(&self.award), e.award.clone(), cx)
                                    .w(px(200.0)),
                            )
                            .child(
                                field("Currency", Input::new(&self.currency), None, cx)
                                    .w(px(100.0)),
                            ),
                    )
                    .child(
                        row()
                            .child(
                                field("Starts", Input::new(&self.start), e.start.clone(), cx)
                                    .w(px(200.0)),
                            )
                            .child(
                                field("Ends", Input::new(&self.end), e.end.clone(), cx)
                                    .w(px(200.0)),
                            ),
                    )
                    .child(field("Terms", Input::new(&self.terms), None, cx))
                    .child(form_buttons(
                        "add-grant",
                        "Add grant",
                        Self::submit_add,
                        |panel: &mut Self| panel.add = None,
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn detail(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let back = Button::new("back-to-grants")
            .ghost()
            .small()
            .label("\u{2190} All grants")
            .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                panel.show(GrantsView::List, window, cx);
            }));
        let Some(Ok(d)) = &self.detail else {
            return v_flex()
                .gap(px(space::CELL))
                .child(h_flex().child(back))
                .child(notice(
                    "This grant is not in the books",
                    "The grant belongs to another organisation, or the id is wrong.",
                    p,
                ))
                .into_any_element();
        };
        let s = &d.summary;
        let runway = div()
            .text_size(px(text::BODY))
            .map(|r| {
                if d.runway.flag {
                    r.font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.signal_orange.to_hsla())
                } else {
                    r.text_color(p.ink_muted.to_hsla())
                }
            })
            .child(d.runway.text.clone());
        let budget: Vec<AnyElement> = d
            .budget
            .iter()
            .map(|r| budget_line(r, p).into_any_element())
            .collect();
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(h_flex().child(back))
                    .child(page_title(&d.grant.name))
                    .child(div().text_size(px(text::LEDE)).child(d.lede.clone())),
            )
            .child(
                v_flex()
                    .child(section_header("Award", self.meta(None, cx), cx))
                    .child(key_figures(&d.figures, p))
                    .child(key_figures(&d.more, p))
                    .child(
                        v_flex()
                            .py(px(space::CELL))
                            .gap(px(10.0))
                            .child(coverage_bar(s.award, s.spent + s.committed, s.spent, p))
                            .child(muted(&d.coverage_label, p))
                            .child(legend(p))
                            .child(runway),
                    ),
            )
            .child(
                v_flex().child(section_header("Terms", div(), cx)).child(
                    div()
                        .py(px(space::CELL))
                        .max_w(px(720.0))
                        .child(d.terms.clone()),
                ),
            )
            .child(
                v_flex()
                    .child(section_header("Budget vs actual", div(), cx))
                    .when(budget.is_empty(), |v| {
                        v.child(notice(
                            "No budget lines for this grant",
                            "Add budget lines for this grant on the budget tab to compare spend \
                             against them.",
                            p,
                        ))
                    })
                    .children(budget),
            )
            .child(
                v_flex()
                    .child(section_header(
                        "Charged to this grant",
                        self.meta(Some(format!("{} lines", d.lines.len())), cx),
                        cx,
                    ))
                    .child(lines_table(d, p)),
            )
            .into_any_element()
    }

    fn budgets(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(view) = &self.budget else {
            return notice(
                "No budget set up",
                "A budget for the financial year has to be created before lines can be \
                 compared with spend.",
                p,
            )
            .into_any_element();
        };
        let basis = self.filter.basis;
        let basis_name = match basis {
            Basis::Accrual => "Accrual basis",
            Basis::Cash => "Cash basis",
        };
        let basis_note = match basis {
            Basis::Accrual => "Accrual counts spend when a bill is posted or a claim approved.",
            Basis::Cash => "Cash counts spend only when the money has left the bank or M-Pesa.",
        };
        let filters = self.filters(view, cx);
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .child(section_header(
                        &view.budget.name,
                        self.meta(Some(basis_name.to_owned()), cx),
                        cx,
                    ))
                    .child(key_figures(&view.figures(&self.base), p))
                    .child(filters)
                    .child(div().py(px(10.0)).child(muted(basis_note, p))),
            )
            .when_some(self.saved.clone(), |v, saved| {
                v.child(
                    div()
                        .py(px(12.0))
                        .border_t(px(space::HAIRLINE))
                        .border_b(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(saved),
                )
            })
            .when(!view.rows.is_empty(), |v| {
                v.child(chart_section(view, p, cx))
            })
            .child(
                v_flex()
                    .child(section_header(
                        "Lines",
                        self.meta(Some(format!("{} lines", view.rows.len())), cx),
                        cx,
                    ))
                    .child(statement_table(&view.table(), p)),
            )
            .child(self.edit_form(cx))
            .child(self.line_form(cx))
            .into_any_element()
    }

    /// The budget page's filter bar (`budgets.tsx`): budget, grant, dimension and basis.
    fn filters(&self, view: &BudgetView, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let basis = self.filter.basis;
        let seg = |id: &'static str, label: &str, b: Basis, cx: &mut Context<Self>| {
            segment(id, label, basis == b, p).on_click(cx.listener(
                move |panel, _: &ClickEvent, window, cx| {
                    panel.filter.basis = b;
                    panel.rederive(window, cx);
                },
            ))
        };
        h_flex()
            .py(px(space::CELL))
            .gap(px(space::CELL))
            .items_start()
            .flex_wrap()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .when(view.budgets.len() > 1, |row| {
                row.child(
                    field("Budget", Select::new(&self.budget_pick.state), None, cx).w(px(220.0)),
                )
            })
            .child(
                field(
                    "Grant",
                    Select::new(&self.filter_grant.state).placeholder("All lines"),
                    None,
                    cx,
                )
                .w(px(260.0)),
            )
            .child(
                field(
                    "Department, project or place",
                    Select::new(&self.filter_dimension.state).placeholder("All lines"),
                    None,
                    cx,
                )
                .w(px(260.0)),
            )
            .child(field(
                "Basis",
                h_flex()
                    .border_1()
                    .border_color(p.hairline.to_hsla())
                    .child(seg("basis-accrual", "Accrual", Basis::Accrual, cx))
                    .child(seg("basis-cash", "Cash", Basis::Cash, cx)),
                None,
                cx,
            ))
    }

    fn edit_form(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .child(section_header("Change a line's budget", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .when_some(self.edit_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(
                        h_flex()
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                field(
                                    "Line",
                                    Select::new(&self.edit_line.state).placeholder("Choose a line"),
                                    None,
                                    cx,
                                )
                                .flex_1(),
                            )
                            .when(self.edit.is_some(), |row| {
                                row.child(
                                    field(
                                        &format!("New budget, {}", self.base),
                                        Input::new(&self.edit_amount),
                                        self.edit_error.clone(),
                                        cx,
                                    )
                                    .w(px(220.0)),
                                )
                            }),
                    )
                    .when(self.edit.is_some(), |v| {
                        v.child(form_buttons(
                            "edit-line",
                            "Save",
                            Self::save_edit,
                            |panel: &mut Self| panel.edit = None,
                            cx,
                        ))
                    }),
            )
            .into_any_element()
    }

    fn line_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(_) = &self.line else {
            return v_flex()
                .child(section_header("Add a budget line", div(), cx))
                .child(
                    div().py(px(space::CELL)).child(
                        Button::new("open-line")
                            .outline()
                            .label("Add a budget line")
                            .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                                panel.open_line(window, cx);
                            })),
                    ),
                )
                .into_any_element();
        };
        let row = || h_flex().gap(px(space::CELL)).items_start();
        v_flex()
            .child(section_header("Add a budget line", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .when_some(self.line_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(
                        row()
                            .child(
                                field(
                                    "Expense account",
                                    v_flex()
                                        .gap(px(6.0))
                                        .child(Select::new(&self.line_account.state))
                                        .child(hint(
                                            "Leave as all for a department allocation",
                                            cx,
                                        )),
                                    None,
                                    cx,
                                )
                                .flex_1(),
                            )
                            .child(
                                field("Fund", Select::new(&self.line_fund.state), None, cx)
                                    .flex_1(),
                            ),
                    )
                    .child(
                        row()
                            .child(
                                field("Grant", Select::new(&self.line_grant.state), None, cx)
                                    .flex_1(),
                            )
                            .child(
                                field(
                                    "Department, project or place",
                                    Select::new(&self.line_dimension.state),
                                    None,
                                    cx,
                                )
                                .flex_1(),
                            )
                            .child(
                                field(
                                    &format!("Amount for the year, {}", self.base),
                                    Input::new(&self.line_amount),
                                    self.line_error.clone(),
                                    cx,
                                )
                                .w(px(220.0)),
                            ),
                    )
                    .child(form_buttons(
                        "add-line",
                        "Add line",
                        Self::submit_line,
                        |panel: &mut Self| panel.line = None,
                        cx,
                    )),
            )
            .into_any_element()
    }
}

impl Render for GrantsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        if self.books.is_none() {
            return v_flex()
                .gap(px(space::SECTION))
                .child(page_title("Grants"))
                .child(muted("No books loaded for this organisation.", p))
                .into_any_element();
        }
        if matches!(self.view, GrantsView::Detail(_)) {
            return self.detail(cx);
        }
        let budgets = self.view == GrantsView::Budgets;
        let (title, lede) = if budgets {
            (
                "Budget vs actual",
                "Every budget line for the year against what has been spent on it. Lines are \
                 flagged at 80% used and again when they go over.",
            )
        } else {
            (
                "Grants",
                "Each award with what has been spent against it, what is committed and what is \
                 left. Spend posts here when a claim is approved or a bill is posted.",
            )
        };
        let action: AnyElement = if budgets {
            div().into_any_element()
        } else {
            Button::new("open-add-grant")
                .primary()
                .label("Add a grant")
                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                    panel.open_add(window, cx);
                }))
                .into_any_element()
        };
        let body = if budgets {
            self.budgets(cx)
        } else {
            self.list(cx)
        };
        v_flex()
            .gap(px(space::SECTION))
            .pb(px(space::SECTION))
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
            .into_any_element()
    }
}

/// The colour of a status: rejection red over, signal orange at 80%, muted within
/// (`text-destructive`, `text-flag`, `text-muted-foreground` on the web).
fn status_color(level: WarningLevel, p: &Palette) -> Hsla {
    match level {
        WarningLevel::Over => p.rejection_red.to_hsla(),
        WarningLevel::Warning => p.signal_orange.to_hsla(),
        WarningLevel::Within => p.ink_muted.to_hsla(),
    }
}

fn status_text(text_: String, level: WarningLevel, p: &Palette) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(status_color(level, p))
        .child(text_)
}

fn muted(text_: &str, p: &Palette) -> impl IntoElement {
    div()
        .text_size(px(text::BODY))
        .text_color(p.ink_muted.to_hsla())
        .child(text_.to_owned())
}

fn error_text(message: &str, p: &Palette) -> impl IntoElement {
    div()
        .py(px(24.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.rejection_red.to_hsla())
        .child(message.to_owned())
}

/// `ReportNotice`: a title and a muted line.
fn notice(title: &str, body: &str, p: &Palette) -> impl IntoElement {
    v_flex()
        .py(px(24.0))
        .gap(px(6.0))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(muted(body, p))
}

/// The share of `part` in `total`, within 0 to 1.
fn share(part: Minor, total: Minor) -> f32 {
    if total <= 0 {
        return 0.0;
    }
    #[expect(clippy::cast_precision_loss, reason = "a bar width, not a figure")]
    let ratio = part.clamp(0, total) as f32 / total as f32;
    ratio
}

/// `CoverageBar` (`components/report`): the evidenced part in report blue, the rest of the
/// filled part in coverage blue, the gap in gap grey.
fn coverage_bar(total: Minor, filled: Minor, evidenced: Minor, p: &Palette) -> impl IntoElement {
    let evidenced_share = share(evidenced, total);
    let filled_share = share(filled, total).max(evidenced_share);
    h_flex()
        .w_full()
        .h(px(12.0))
        .bg(p.gap_grey.to_hsla())
        .child(
            div()
                .h_full()
                .w(relative(evidenced_share))
                .bg(p.report_blue.to_hsla()),
        )
        .child(
            div()
                .h_full()
                .w(relative(filled_share - evidenced_share))
                .bg(p.coverage_blue.to_hsla()),
        )
}

fn swatch(label: &str, color: Hsla, p: &Palette) -> impl IntoElement {
    h_flex()
        .gap(px(6.0))
        .text_color(p.ink_muted.to_hsla())
        .child(div().w(px(20.0)).h(px(12.0)).bg(color))
        .child(label.to_owned())
}

/// The coverage legend: Spent, Committed, Remaining.
fn legend(p: &Palette) -> impl IntoElement {
    h_flex()
        .py(px(12.0))
        .gap(px(20.0))
        .text_size(px(12.0))
        .child(swatch("Spent", p.report_blue.to_hsla(), p))
        .child(swatch("Committed", p.coverage_blue.to_hsla(), p))
        .child(swatch("Remaining", p.gap_grey.to_hsla(), p))
}

/// Three small labelled figures, the last right-aligned (the `dl` under a coverage bar).
fn three_figures(items: [(String, String); 3], p: &Palette) -> impl IntoElement {
    h_flex()
        .text_size(px(12.0))
        .children(items.into_iter().enumerate().map(|(i, (label, value))| {
            v_flex()
                .flex_1()
                .when(i == 2, Styled::items_end)
                .child(div().text_color(p.ink_muted.to_hsla()).child(label))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
        }))
}

/// One budget line on the grant page: label and status, then its bar and figures.
fn budget_line(r: &BudgetRow, p: &Palette) -> impl IntoElement {
    let filled = r.row.actual.min(r.row.budget);
    let label = r.row.account.as_ref().map_or_else(
        || r.row.label.clone(),
        |a| format!("{} \u{b7} {}", a.code, a.name),
    );
    let used = crate::grants::used_percent(r.row.used).unwrap_or(100);
    h_flex()
        .py(px(16.0))
        .gap(px(24.0))
        .items_start()
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            v_flex()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(4.0))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(label))
                .child(status_text(r.status_line(), r.row.status, p)),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(8.0))
                .child(coverage_bar(r.row.budget, filled, filled, p))
                .child(muted(&format!("{used}% of budget spent"), p))
                .child(three_figures(
                    [
                        ("Budget".to_owned(), format_whole(r.row.budget)),
                        ("Actual".to_owned(), format_whole(r.row.actual)),
                        (
                            r.variance_label().to_owned(),
                            match r.row.variance.abs() {
                                0 => "0".to_owned(),
                                v => format_whole(v),
                            },
                        ),
                    ],
                    p,
                )),
        )
}

/// The posted lines charged to a grant (`LinesTable` in `grants/$id.tsx`).
fn lines_table(d: &GrantDetail, p: &Palette) -> AnyElement {
    if d.lines.is_empty() {
        return notice(
            "Nothing posted to this grant yet",
            "Income received and costs charged to the grant appear here once posted.",
            p,
        )
        .into_any_element();
    }
    let cell = |w: f32| div().w(px(w)).flex_shrink_0().pr(px(12.0));
    let amount = |w: f32| cell(w).flex().justify_end();
    let head = h_flex()
        .py(px(10.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.ink_navy.to_hsla())
        .text_size(px(text::LABEL))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted.to_hsla())
        .child(cell(110.0).child("Date"))
        .child(cell(110.0).child("Entry"))
        .child(cell(260.0).child("Account"))
        .child(div().flex_1().min_w(px(0.0)).child("Memo"))
        .child(amount(140.0).child("Debit"))
        .child(amount(140.0).child("Credit"));
    v_flex()
        .child(head)
        .children(d.lines.iter().map(|l| {
            h_flex()
                .py(px(8.0))
                .items_start()
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(
                    cell(110.0)
                        .text_color(p.ink_muted.to_hsla())
                        .child(format_date(&l.date)),
                )
                .child(
                    cell(110.0)
                        .text_size(px(text::LABEL))
                        .font_family(text::MONO_FAMILY)
                        .child(l.entry_number.clone()),
                )
                .child(
                    cell(260.0).child(
                        h_flex()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(text::LABEL))
                                    .font_family(text::MONO_FAMILY)
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(l.code.clone()),
                            )
                            .child(l.account.clone()),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(l.memo.clone()),
                )
                .child(amount(140.0).child(format_minor(l.debit)))
                .child(amount(140.0).child(format_minor(l.credit)))
        }))
        .into_any_element()
}

/// The budget chart: gpui-kit `BarChart` (<https://gpui-kit.com/component/chart>) with
/// horizontal bars, a budget bar in gap grey and an actual bar coloured by the line's status.
fn budget_chart(bars: &[Bar], p: &'static Palette) -> impl IntoElement {
    let height = 24.0 + 18.0 * f32::from(u16::try_from(bars.len()).unwrap_or(u16::MAX));
    div().w_full().h(px(height)).py(px(8.0)).child(
        BarChart::new(bars.to_vec())
            .band(|b: &Bar| SharedString::from(b.band.clone()))
            .value(|b: &Bar| b.major)
            .alignment(BarAlignment::Left)
            .value_axis(true)
            .value_tick_format(|v| format_compact(major_to_minor(v)))
            .tooltip_value(|_, v| format_compact(major_to_minor(v)).into())
            .fill(move |b: &Bar, _, _, _| match b.kind {
                BarKind::Budget => p.gap_grey.to_hsla(),
                BarKind::Actual => match b.status {
                    WarningLevel::Within => p.report_blue.to_hsla(),
                    level => status_color(level, p),
                },
            }),
    )
}

/// The chart's section: its header with the key, then the chart.
fn chart_section(view: &BudgetView, p: &'static Palette, cx: &App) -> impl IntoElement {
    v_flex()
        .child(section_header(
            "Budget and actual by line",
            h_flex()
                .gap(px(12.0))
                .text_size(px(12.0))
                .child(swatch("Budget", p.gap_grey.to_hsla(), p))
                .child(swatch("Actual", p.report_blue.to_hsla(), p))
                .child(swatch("80% used", p.signal_orange.to_hsla(), p))
                .child(swatch("Over", p.rejection_red.to_hsla(), p)),
            cx,
        ))
        .child(budget_chart(&view.bars(), p))
}

/// A chart value in major units back to minor units for [`format_compact`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "chart ticks of safe amounts, far inside i64"
)]
fn major_to_minor(v: f64) -> Minor {
    (v * 100.0).round() as Minor
}
