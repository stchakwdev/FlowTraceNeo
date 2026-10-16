//! The accounts-payable half of the Spend entry: the bills list with its status filter, the
//! vendors with their add form, the new-bill form, and the bill detail with Approve, Post and
//! Pay, its journal entries, the payments' fund split and the status trail.
//!
//! Everything shown comes from the view models in [`crate::bills`]; this file keeps the widgets
//! and draws. Writes go through the store and the new books come back through its change
//! channel (README.md, "How a screen replaces its placeholder"). Web screens mirrored:
//! `flowtrace/src/routes/_authenticated/spend/bills.index.tsx`, `bills.new.tsx`,
//! `bills.$id.tsx`. Components: <https://gpui-kit.com/component/input>,
//! <https://gpui-kit.com/component/select>, <https://gpui-kit.com/component/button>.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{
    AccountId, Actor, BillId, BillStatus, DimensionId, FundId, GrantId, IsoDate, LedgerState,
    OrganizationId, VendorId,
};
use ledger_core::{CommandResult, LedgerCommand};

use crate::accounts::{Failure, format_date, format_minor};
use crate::bills::{
    BillAction, BillDetail, BillErrors, BillField, BillFilter, BillRow, CURRENCIES, EntryView,
    FILTERS, LineDraft, LineField, NewBillForm, NewVendorForm, PayErrors, PayForm, Tone,
    VendorRow, bill_detail, bill_rows, cash_accounts, filter_count, filtered, grants_for,
    line_accounts, no_match_text, overdue_text, status_label, status_tone, vendor_rows,
    vendors_meta,
};
use crate::panels::{
    Pick, PickState, choice, demo_tag, failure_box, field, form_buttons, hint, page_title,
    section_header,
};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// The tabs of the accounts-payable screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// The bills list, the new-bill form and a bill's detail.
    Bills,
    /// The vendors.
    Vendors,
}

/// What the Bills tab shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Page {
    List,
    New,
    Detail(BillId),
}

/// The widgets of one coded line of the new-bill form.
struct LineWidgets {
    description: Entity<InputState>,
    amount: Entity<InputState>,
    account: PickState,
    fund: PickState,
    grant: PickState,
    dimension: PickState,
    _subscriptions: Vec<Subscription>,
}

/// The bills, payments and vendors screen.
pub struct BillsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    books: Option<Arc<LedgerState>>,
    demo: bool,
    as_of: IsoDate,
    actor: Option<Actor>,
    tab: Tab,
    page: Page,

    filter: BillFilter,
    rows: Vec<BillRow>,
    shown: Vec<BillRow>,
    cursor: usize,

    vendors: Vec<VendorRow>,
    vendor_open: bool,
    vendor_error: Option<String>,
    vendor_failure: Option<Failure>,
    vendor_done: Option<String>,
    vendor_name: Entity<InputState>,
    vendor_details: Entity<InputState>,

    form: Option<NewBillForm>,
    form_errors: BillErrors,
    form_failure: Option<Failure>,
    vendor_pick: PickState,
    number: Entity<InputState>,
    bill_date: Entity<InputState>,
    due_date: Entity<InputState>,
    fx_rate: Entity<InputState>,
    lines: Vec<LineWidgets>,

    detail: Option<BillDetail>,
    done: Option<String>,
    action_failure: Option<Failure>,
    pay: Option<PayForm>,
    pay_errors: PayErrors,
    pay_failure: Option<Failure>,
    pay_amount: Entity<InputState>,
    pay_date: Entity<InputState>,
    pay_reference: Entity<InputState>,
    pay_cash: PickState,

    _subscriptions: Vec<Subscription>,
}

/// `(id, label)` choices as select items, with a leading empty choice when `none` is given.
fn picks<T: AsRef<str>>(choices: &[(T, String)], none: Option<&str>) -> Vec<Pick> {
    none.map(|label| Pick::new("", label.to_owned()))
        .into_iter()
        .chain(
            choices
                .iter()
                .map(|(id, label)| Pick::new(id.as_ref(), label.clone())),
        )
        .collect()
}

/// The chosen value of a select, `None` for nothing or the empty choice.
fn picked(select: &PickState, cx: &App) -> Option<String> {
    select
        .read(cx)
        .selected_value()
        .filter(|v| !v.is_empty())
        .cloned()
}

fn new_pick(window: &mut Window, cx: &mut App, items: Vec<Pick>, value: Option<&str>) -> PickState {
    let index = value.and_then(|v| items.iter().position(|p| p.value == v));
    cx.new(|cx| SelectState::new(items, index.map(IndexPath::new), window, cx))
}

impl BillsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let vendor_name = input("As it appears on the bill", window, cx);
        let vendor_details = input("Paybill, till or bank account", window, cx);
        let number = input("As printed on the invoice", window, cx);
        let bill_date = input("dd/mm/yyyy", window, cx);
        let due_date = input("dd/mm/yyyy", window, cx);
        let fx_rate = input("e.g. 129.45", window, cx);
        let pay_amount = input("0.00", window, cx);
        let pay_date = input("dd/mm/yyyy", window, cx);
        let pay_reference = input("EFT, cheque or M-Pesa code", window, cx);
        let vendor_pick = new_pick(window, cx, Vec::new(), None);
        let pay_cash = new_pick(window, cx, Vec::new(), None);
        let subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&vendor_details, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.add_vendor(cx);
                }
            }),
            cx.subscribe_in(&pay_reference, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.confirm_pay(cx);
                }
            }),
        ];
        let mut panel = Self {
            store,
            seen: None,
            books: None,
            demo: false,
            as_of: today_utc(),
            actor: None,
            tab: Tab::Bills,
            page: Page::List,
            filter: BillFilter::All,
            rows: Vec::new(),
            shown: Vec::new(),
            cursor: 0,
            vendors: Vec::new(),
            vendor_open: false,
            vendor_error: None,
            vendor_failure: None,
            vendor_done: None,
            vendor_name,
            vendor_details,
            form: None,
            form_errors: BillErrors::default(),
            form_failure: None,
            vendor_pick,
            number,
            bill_date,
            due_date,
            fx_rate,
            lines: Vec::new(),
            detail: None,
            done: None,
            action_failure: None,
            pay: None,
            pay_errors: PayErrors::default(),
            pay_failure: None,
            pay_amount,
            pay_date,
            pay_reference,
            pay_cash,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Re-derives the rows when the organisation or its books changed.
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
        self.actor = store.actor();
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        // The demo is a snapshot as of its latest movement; other books run to today
        // (`useLedgerAsOf`, flowtrace/src/lib/ledger-client.ts:45-47).
        self.as_of = if self.demo {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        };
        if org_changed {
            self.page = Page::List;
            self.form = None;
            self.lines.clear();
            self.pay = None;
            self.done = None;
            self.vendor_open = false;
            self.vendor_done = None;
        }
        if let Some(books) = self.books.clone() {
            self.rows = bill_rows(&books, &self.as_of);
            self.vendors = vendor_rows(&books);
            let vendor = picked(&self.vendor_pick, cx);
            let items = picks(
                &self
                    .vendors
                    .iter()
                    .map(|v| (v.id.as_str().to_owned(), v.name.clone()))
                    .collect::<Vec<_>>(),
                None,
            );
            self.vendor_pick.update(cx, |s, cx| {
                s.set_items(items, window, cx);
                if let Some(v) = &vendor {
                    s.set_selected_value(v, window, cx);
                }
            });
            let cash = picks(&cash_accounts(&books), None);
            self.pay_cash
                .update(cx, |s, cx| s.set_items(cash, window, cx));
        } else {
            self.rows.clear();
            self.vendors.clear();
        }
        self.reshow();
        self.load_detail();
        cx.notify();
    }

    fn reshow(&mut self) {
        self.shown = filtered(&self.rows, self.filter);
        self.cursor = self.cursor.min(self.shown.len().saturating_sub(1));
    }

    fn load_detail(&mut self) {
        self.detail = match (&self.page, self.books.as_deref()) {
            (Page::Detail(id), Some(books)) => {
                bill_detail(books, id, self.actor.as_ref(), &self.as_of)
            }
            _ => None,
        };
        if self
            .detail
            .as_ref()
            .is_none_or(|d| !d.actions.contains(&BillAction::Pay))
        {
            self.pay = None;
        }
    }

    /// Keys while the screen shows and no field has focus: `b` the bills, `v` the vendors,
    /// `n` a new bill, `f` the next status filter, `j`/`k` or Down/Up move through the list,
    /// Enter opens the bill, Escape goes back to the list.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "b" => {
                self.tab = Tab::Bills;
                self.page = Page::List;
            }
            "v" => self.tab = Tab::Vendors,
            "n" => {
                self.tab = Tab::Bills;
                self.open_new(window, cx);
            }
            "f" if self.tab == Tab::Bills && self.page == Page::List => {
                let i = FILTERS.iter().position(|f| *f == self.filter).unwrap_or(0);
                self.set_filter(FILTERS[(i + 1) % FILTERS.len()]);
            }
            "j" | "down" if self.tab == Tab::Bills && self.page == Page::List => {
                self.cursor = (self.cursor + 1).min(self.shown.len().saturating_sub(1));
            }
            "k" | "up" if self.tab == Tab::Bills && self.page == Page::List => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            "enter" if self.tab == Tab::Bills && self.page == Page::List => {
                let Some(id) = self.shown.get(self.cursor).map(|r| r.id.clone()) else {
                    return false;
                };
                self.open_detail(id);
            }
            "escape" if self.tab == Tab::Bills && self.page != Page::List => {
                self.page = Page::List;
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    fn set_filter(&mut self, filter: BillFilter) {
        self.filter = filter;
        self.cursor = 0;
        self.reshow();
    }

    fn open_detail(&mut self, id: BillId) {
        self.page = Page::Detail(id);
        self.done = None;
        self.action_failure = None;
        self.pay = None;
        self.load_detail();
    }

    fn submit(&self, command: LedgerCommand, cx: &mut Context<Self>) -> Result<CommandResult, Failure> {
        self.store
            .update(cx, |store, _| store.submit(command))
            .map_err(|err| Failure::new("Not done", &err))
    }

    // Vendors.

    fn open_vendor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.vendor_open = true;
        self.vendor_error = None;
        self.vendor_failure = None;
        self.vendor_done = None;
        for input in [&self.vendor_name, &self.vendor_details] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        self.vendor_name.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    fn add_vendor(&mut self, cx: &mut Context<Self>) {
        let form = NewVendorForm {
            name: self.vendor_name.read(cx).value().to_string(),
            payment_details: self.vendor_details.read(cx).value().to_string(),
        };
        match form.command() {
            Err(message) => self.vendor_error = Some(message),
            Ok(command) => {
                self.vendor_error = None;
                match self.submit(command, cx) {
                    Ok(_) => {
                        self.vendor_open = false;
                        self.vendor_failure = None;
                        self.vendor_done = Some(format!("Added {}.", form.name.trim()));
                    }
                    Err(f) => self.vendor_failure = Some(Failure { title: "Not added".into(), ..f }),
                }
            }
        }
        cx.notify();
    }

    // The new-bill form.

    fn open_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let form = NewBillForm::new(&books, &self.as_of);
        self.number.update(cx, |i, cx| i.set_value("", window, cx));
        let (b, d) = (form.bill_date.clone(), form.due_date.clone());
        self.bill_date.update(cx, |i, cx| i.set_value(b, window, cx));
        self.due_date.update(cx, |i, cx| i.set_value(d, window, cx));
        self.fx_rate.update(cx, |i, cx| i.set_value("", window, cx));
        self.vendor_pick
            .update(cx, |s, cx| s.set_selected_index(None, window, cx));
        self.lines.clear();
        for line in &form.lines {
            let widgets = Self::line_widgets(&books, line, window, cx);
            self.lines.push(widgets);
        }
        self.form = Some(form);
        self.form_errors = BillErrors::default();
        self.form_failure = None;
        self.page = Page::New;
        self.number.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    fn line_widgets(
        books: &LedgerState,
        line: &LineDraft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> LineWidgets {
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
        let account = new_pick(
            window,
            cx,
            picks(&line_accounts(books), None),
            line.account_id.as_ref().map(AccountId::as_str),
        );
        let funds: Vec<(FundId, String)> = books
            .funds
            .iter()
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();
        let fund = new_pick(
            window,
            cx,
            picks(&funds, None),
            line.fund_id.as_ref().map(FundId::as_str),
        );
        let grant = new_pick(
            window,
            cx,
            picks(&grants_for(books, line.fund_id.as_ref()), Some("No grant")),
            Some(line.grant_id.as_ref().map_or("", GrantId::as_str)),
        );
        let dims: Vec<(DimensionId, String)> = books
            .dimensions
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();
        let dimension = new_pick(
            window,
            cx,
            picks(&dims, Some("None")),
            Some(line.dimension_id.as_ref().map_or("", DimensionId::as_str)),
        );
        let grant_for_fund = grant.clone();
        let subscriptions = vec![cx.subscribe_in(
            &fund,
            window,
            move |this, _, event: &SelectEvent<Vec<Pick>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                this.fund_changed(&grant_for_fund, value, window, cx);
            },
        )];
        LineWidgets {
            description,
            amount,
            account,
            fund,
            grant,
            dimension,
            _subscriptions: subscriptions,
        }
    }

    /// `withFund` on the line whose grant select is `grant`: refills the grants of the new fund
    /// and keeps or picks the grant as the view model says.
    fn fund_changed(
        &mut self,
        grant: &PickState,
        fund: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let Some(index) = self.lines.iter().position(|l| &l.grant == grant) else {
            return;
        };
        self.sync_form(cx);
        let Some(line) = self.form.as_mut().and_then(|f| f.lines.get_mut(index)) else {
            return;
        };
        line.set_fund(&books, FundId::from(fund));
        let items = picks(&grants_for(&books, line.fund_id.as_ref()), Some("No grant"));
        let chosen = line
            .grant_id
            .as_ref()
            .map_or_else(String::new, |g| g.as_str().to_owned());
        grant.update(cx, |s, cx| {
            s.set_items(items, window, cx);
            s.set_selected_value(&chosen, window, cx);
        });
        cx.notify();
    }

    /// Reads the form's widgets into the form.
    fn sync_form(&mut self, cx: &App) {
        let Some(form) = &mut self.form else {
            return;
        };
        form.vendor = picked(&self.vendor_pick, cx).map(VendorId::from);
        form.number = self.number.read(cx).value().to_string();
        form.bill_date = self.bill_date.read(cx).value().to_string();
        form.due_date = self.due_date.read(cx).value().to_string();
        form.fx_rate = self.fx_rate.read(cx).value().to_string();
        form.lines = self
            .lines
            .iter()
            .map(|w| LineDraft {
                description: w.description.read(cx).value().to_string(),
                amount: w.amount.read(cx).value().to_string(),
                account_id: picked(&w.account, cx).map(AccountId::from),
                fund_id: picked(&w.fund, cx).map(FundId::from),
                grant_id: picked(&w.grant, cx).map(GrantId::from),
                dimension_id: picked(&w.dimension, cx).map(DimensionId::from),
            })
            .collect();
    }

    fn add_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        self.sync_form(cx);
        let Some(form) = &mut self.form else {
            return;
        };
        form.add_line(&books);
        let Some(line) = form.lines.last().cloned() else {
            return;
        };
        let widgets = Self::line_widgets(&books, &line, window, cx);
        widgets
            .description
            .update(cx, |i, cx| i.focus(window, cx));
        self.lines.push(widgets);
        cx.notify();
    }

    fn remove_line(&mut self, index: usize, cx: &mut Context<Self>) {
        self.sync_form(cx);
        if let Some(form) = &mut self.form
            && form.lines.len() > 1
            && index < self.lines.len()
        {
            form.remove_line(index);
            self.lines.remove(index);
        }
        cx.notify();
    }

    fn set_currency(&mut self, code: &str, cx: &mut Context<Self>) {
        if let Some(form) = &mut self.form {
            code.clone_into(&mut form.currency);
            cx.notify();
        }
    }

    fn save_bill(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        self.sync_form(cx);
        let Some(form) = &self.form else {
            return;
        };
        match form.command(&books) {
            Err(errors) => {
                self.form_errors = errors;
                self.form_failure = None;
            }
            Ok(command) => {
                self.form_errors = BillErrors::default();
                match self.submit(command, cx) {
                    Ok(CommandResult::Id { id }) => {
                        self.form = None;
                        self.lines.clear();
                        self.open_detail(BillId::from(id));
                        self.done = Some(
                            "Saved as a draft. Someone with approval rights who did not enter \
                             it approves it next."
                                .to_owned(),
                        );
                    }
                    Ok(_) => {
                        self.form = None;
                        self.page = Page::List;
                    }
                    Err(f) => self.form_failure = Some(f),
                }
            }
        }
        cx.notify();
    }

    // The detail's actions.

    fn act(&mut self, action: BillAction, window: &mut Window, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else {
            return;
        };
        self.done = None;
        self.action_failure = None;
        if action == BillAction::Pay {
            let form = PayForm::new(detail.outstanding, &self.as_of);
            let (amount, date) = (form.amount.clone(), form.date.clone());
            self.pay_amount
                .update(cx, |i, cx| i.set_value(amount, window, cx));
            self.pay_date.update(cx, |i, cx| i.set_value(date, window, cx));
            self.pay_reference
                .update(cx, |i, cx| i.set_value("", window, cx));
            self.pay_cash
                .update(cx, |s, cx| s.set_selected_index(None, window, cx));
            self.pay = Some(form);
            self.pay_errors = PayErrors::default();
            self.pay_failure = None;
            self.pay_amount.update(cx, |i, cx| i.focus(window, cx));
        } else if let Some(command) = action.command(&detail.bill.id) {
            match self.submit(command, cx) {
                Ok(_) => self.done = Some(action.done().to_owned()),
                Err(f) => self.action_failure = Some(f),
            }
        }
        cx.notify();
    }

    fn confirm_pay(&mut self, cx: &mut Context<Self>) {
        let (Some(detail), Some(form)) = (&self.detail, &mut self.pay) else {
            return;
        };
        form.amount = self.pay_amount.read(cx).value().to_string();
        form.date = self.pay_date.read(cx).value().to_string();
        form.reference = self.pay_reference.read(cx).value().to_string();
        form.cash_account = picked(&self.pay_cash, cx).map(AccountId::from);
        match form.command(&detail.bill.id, detail.outstanding.unwrap_or(0)) {
            Err(errors) => self.pay_errors = errors,
            Ok(command) => {
                self.pay_errors = PayErrors::default();
                match self.submit(command, cx) {
                    Ok(_) => {
                        self.pay = None;
                        self.done = Some(BillAction::Pay.done().to_owned());
                    }
                    Err(f) => self.pay_failure = Some(f),
                }
            }
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
                    if this == Tab::Bills {
                        panel.page = Page::List;
                    }
                    cx.notify();
                }))
        };
        h_flex()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(tab("tab-bills", "Bills", "b", Tab::Bills))
            .child(tab("tab-vendors", "Vendors", "v", Tab::Vendors))
    }

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

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let filters = FILTERS.into_iter().map(|f| {
            let active = self.filter == f;
            let label = format!("{}  {}", f.label(), filter_count(&self.rows, f));
            let b = Button::new(SharedString::from(format!("filter-{}", f.label())))
                .small()
                .label(label)
                .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                    panel.set_filter(f);
                    cx.notify();
                }));
            if active { b.primary() } else { b.outline() }
        });
        let filters: Vec<Button> = filters.collect();
        let header = h_flex()
            .h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(col_bill().child("Bill"))
            .child(col_date().child("Dated"))
            .child(col_date().child("Due"))
            .child(col_money().child("Amount"))
            .child(col_money().child("Outstanding"))
            .child(col_status().child("Status"));
        let rows: Vec<AnyElement> = self
            .shown
            .iter()
            .enumerate()
            .map(|(i, r)| self.bill_row(i, r, cx))
            .collect();
        let empty = if self.rows.is_empty() {
            Some((
                "No bills yet".to_owned(),
                "Record a supplier\u{2019}s invoice here to approve it, post it and pay it."
                    .to_owned(),
            ))
        } else if self.shown.is_empty() {
            Some(("No bills match".to_owned(), no_match_text(self.filter)))
        } else {
            None
        };
        v_flex()
            .child(section_header("Bills", self.meta(None, cx), cx))
            .child(
                h_flex()
                    .py(px(12.0))
                    .gap(px(6.0))
                    .flex_wrap()
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .children(filters),
            )
            .map(|v| match empty {
                Some((title, body)) => v.child(notice(&title, &body, cx)),
                None => v.child(header).children(rows),
            })
            .child(hint(
                "j and k move, Enter opens, f changes the filter, n starts a new bill",
                cx,
            ))
            .into_any_element()
    }

    fn bill_row(&self, i: usize, r: &BillRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        let current = i == self.cursor;
        h_flex()
            .id(SharedString::from(format!("bill-{}", r.id.as_str())))
            .min_h(px(52.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .when(current, |row| row.bg(p.wash.to_hsla()))
            .hover(|row| row.bg(p.rail_hover.to_hsla()))
            .child(
                col_bill()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(r.vendor.clone()))
                    .child(
                        div()
                            .text_size(px(text::MONO))
                            .text_color(p.ink_muted.to_hsla())
                            .child(r.number.clone()),
                    ),
            )
            .child(
                col_date()
                    .text_color(p.ink_muted.to_hsla())
                    .child(format_date(&r.bill_date)),
            )
            .child(
                col_date()
                    .child(format_date(&r.due_date))
                    .when(r.overdue_days > 0, |c| {
                        c.child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.signal_orange.to_hsla())
                                .child(overdue_text(r.overdue_days)),
                        )
                    }),
            )
            .child(col_money().child(money(r.total, &r.currency)))
            .child(
                col_money()
                    .map(|c| match r.outstanding {
                        None => c
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child("Not posted"),
                        Some(_) => c.font_weight(FontWeight::BOLD).child(r.outstanding_text()),
                    }),
            )
            .child(col_status().child(status_tag(r.status, p)))
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.cursor = i;
                panel.open_detail(id.clone());
                cx.notify();
            }))
            .into_any_element()
    }

    fn vendors_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let header = h_flex()
            .h(px(36.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(col_bill().child("Supplier"))
            .child(col_wide().child("Payment details"))
            .child(col_date().child("Flow node"))
            .child(col_count().child("Bills"))
            .child(col_money().child("Outstanding"));
        let rows = self.vendors.iter().map(|v| {
            h_flex()
                .min_h(px(44.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(col_bill().font_weight(FontWeight::SEMIBOLD).child(v.name.clone()))
                .child(
                    col_wide()
                        .text_color(p.ink_muted.to_hsla())
                        .child(v.payment_details.clone().unwrap_or_else(|| "\u{2013}".into())),
                )
                .child(
                    col_date()
                        .text_size(px(text::MONO))
                        .text_color(p.ink_muted.to_hsla())
                        .child(v.flow_node.clone().unwrap_or_else(|| "\u{2013}".into())),
                )
                .child(col_count().child(v.bills.to_string()))
                .child(
                    col_money()
                        .when(v.outstanding > 0, |c| c.font_weight(FontWeight::BOLD))
                        .child(format_minor(v.outstanding)),
                )
        });
        v_flex()
            .gap(px(space::SECTION))
            .when_some(self.vendor_done.clone(), |v, done| v.child(done_box(&done, p)))
            .when(self.vendor_open, |v| v.child(self.vendor_form(cx)))
            .child(
                v_flex()
                    .child(section_header(
                        "Vendors",
                        self.meta(Some(vendors_meta(&self.vendors)), cx),
                        cx,
                    ))
                    .child(header)
                    .when(self.vendors.is_empty(), |v| {
                        v.child(notice(
                            "No suppliers yet",
                            "Add a supplier to enter its bills.",
                            cx,
                        ))
                    })
                    .children(rows),
            )
            .child(hint(
                "Outstanding is what posted bills still owe, in the books\u{2019} currency at \
                 each bill\u{2019}s rate.",
                cx,
            ))
            .into_any_element()
    }

    fn vendor_form(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .child(section_header("Add a supplier", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .when_some(self.vendor_failure.clone(), |v, f| v.child(failure_box(&f, cx)))
                    .child(
                        h_flex()
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                field(
                                    "Supplier name",
                                    Input::new(&self.vendor_name),
                                    self.vendor_error.clone(),
                                    cx,
                                )
                                .flex_1(),
                            )
                            .child(
                                field("Payment details", Input::new(&self.vendor_details), None, cx)
                                    .flex_1(),
                            ),
                    )
                    .child(form_buttons(
                        "vendor",
                        "Add supplier",
                        |panel: &mut Self, _, cx| panel.add_vendor(cx),
                        |panel: &mut Self| panel.vendor_open = false,
                        cx,
                    )),
            )
            .into_any_element()
    }

    #[expect(clippy::too_many_lines, reason = "one linear layout of the form's sections")]
    fn new_form(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(form) = &self.form else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        let errors = &self.form_errors;
        let currencies = CURRENCIES.into_iter().map(|(code, _)| {
            choice(format!("currency-{code}"), code, form.currency == code).on_click(cx.listener(
                move |panel, _: &ClickEvent, _, cx| panel.set_currency(code, cx),
            ))
        });
        let currencies: Vec<Button> = currencies.collect();
        let total = {
            let lines: Vec<LineDraft> = self
                .lines
                .iter()
                .map(|w| LineDraft {
                    amount: w.amount.read(cx).value().to_string(),
                    ..LineDraft::default()
                })
                .collect();
            crate::bills::lines_total(&lines)
        };
        let lines: Vec<AnyElement> = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, w)| self.line_editor(i, w, &form.currency, cx))
            .collect();
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(880.0))
            .when(!errors.is_empty(), |v| {
                v.child(
                    v_flex()
                        .p(px(16.0))
                        .gap(px(6.0))
                        .border_1()
                        .border_color(p.rejection_red.to_hsla())
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.rejection_red.to_hsla())
                                .child(errors.title()),
                        )
                        .children(errors.summary().into_iter().map(|m| div().child(m))),
                )
            })
            .when_some(self.form_failure.clone(), |v, f| v.child(failure_box(&f, cx)))
            .child(
                v_flex()
                    .child(section_header("Supplier", div(), cx))
                    .child(
                        div().py(px(space::CELL)).max_w(px(420.0)).child(field(
                            "Supplier",
                            Select::new(&self.vendor_pick).placeholder(if self.vendors.is_empty() {
                                "No suppliers yet"
                            } else {
                                "Choose a supplier"
                            }),
                            errors.field(BillField::Vendor),
                            cx,
                        )),
                    )
                    .child(hint("A new supplier is added on the Vendors tab (key v).", cx)),
            )
            .child(
                v_flex()
                    .child(section_header("The bill", div(), cx))
                    .child(
                        v_flex()
                            .py(px(space::CELL))
                            .gap(px(space::CELL))
                            .child(
                                field(
                                    "Bill number",
                                    Input::new(&self.number),
                                    errors.field(BillField::Number),
                                    cx,
                                )
                                .max_w(px(420.0)),
                            )
                            .child(
                                h_flex()
                                    .gap(px(space::CELL))
                                    .items_start()
                                    .child(
                                        field(
                                            "Bill date",
                                            Input::new(&self.bill_date),
                                            errors.field(BillField::BillDate),
                                            cx,
                                        )
                                        .flex_1(),
                                    )
                                    .child(
                                        field(
                                            "Due date",
                                            Input::new(&self.due_date),
                                            errors.field(BillField::DueDate),
                                            cx,
                                        )
                                        .flex_1(),
                                    ),
                            )
                            .child(field(
                                "Currency",
                                h_flex().gap(px(6.0)).flex_wrap().children(currencies),
                                None,
                                cx,
                            ))
                            .map(|v| {
                                if form.foreign() {
                                    v.child(
                                        field(
                                            &format!(
                                                "Exchange rate, {} per {}",
                                                form.base, form.currency
                                            ),
                                            Input::new(&self.fx_rate),
                                            errors.field(BillField::FxRate),
                                            cx,
                                        )
                                        .max_w(px(320.0)),
                                    )
                                } else {
                                    v.child(hint(
                                        &format!(
                                            "{} is the books\u{2019} own currency, so no exchange \
                                             rate is needed.",
                                            form.base
                                        ),
                                        cx,
                                    ))
                                }
                            }),
                    ),
            )
            .child(
                v_flex()
                    .child(section_header(
                        "Coding",
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!("In {}", form.currency)),
                        cx,
                    ))
                    .children(lines)
                    .child(
                        h_flex()
                            .justify_between()
                            .py(px(12.0))
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
                                            .text_size(px(text::TITLE))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!(
                                                "{} {}",
                                                form.currency,
                                                if total == 0 {
                                                    "0.00".to_owned()
                                                } else {
                                                    format_minor(total)
                                                }
                                            )),
                                    ),
                            ),
                    ),
            )
            .child(hint(
                "Proof files are kept with the web app; the desktop records bills without them.",
                cx,
            ))
            .child(
                div()
                    .pt(px(16.0))
                    .border_t(px(space::RULE_HEAVY))
                    .border_color(p.ink_navy.to_hsla())
                    .child(form_buttons(
                        "bill",
                        "Save bill",
                        Self::save_bill,
                        |panel: &mut Self| panel.page = Page::List,
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn line_editor(
        &self,
        i: usize,
        w: &LineWidgets,
        currency: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        let errors = &self.form_errors;
        let err = |f: LineField| errors.line(i, f);
        v_flex()
            .py(px(16.0))
            .gap(px(12.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                h_flex()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!("Line {}", i + 1)))
                    .when(self.lines.len() > 1, |h| {
                        h.child(
                            Button::new(SharedString::from(format!("remove-line-{i}")))
                                .ghost()
                                .small()
                                .label(format!("Remove line {}", i + 1))
                                .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                                    panel.remove_line(i, cx);
                                })),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
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
                            &format!("Amount, {currency}"),
                            Input::new(&w.amount),
                            err(LineField::Amount),
                            cx,
                        )
                        .w(px(200.0)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
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
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
                    .child(
                        field(
                            "Grant",
                            Select::new(&w.grant).placeholder("No grant"),
                            err(LineField::Grant),
                            cx,
                        )
                        .flex_1(),
                    )
                    .child(
                        field(
                            "Project, department or place",
                            Select::new(&w.dimension).placeholder("None"),
                            None,
                            cx,
                        )
                        .flex_1(),
                    ),
            )
            .into_any_element()
    }

    fn detail_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(d) = &self.detail else {
            return notice(
                "This bill is not in the books",
                "It may have been entered in another organisation.",
                cx,
            )
            .into_any_element();
        };
        let p = palette(cx.theme().mode);
        let b = &d.bill;
        let figures = [
            (format_whole(d.total), format!("Billed, {}", b.currency)),
            (
                d.outstanding.map_or_else(|| "\u{2013}".to_owned(), format_whole),
                "Outstanding".to_owned(),
            ),
            (format_whole(d.paid), "Paid".to_owned()),
            if d.overdue_days > 0 {
                (d.overdue_days.to_string(), "Days overdue".to_owned())
            } else {
                (d.days_until_due.to_string(), "Days until due".to_owned())
            },
        ];
        let actions = d.actions.iter().map(|&a| {
            Button::new(SharedString::from(format!("act-{}", a.label())))
                .primary()
                .label(a.label())
                .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                    panel.act(a, window, cx);
                }))
        });
        let actions: Vec<Button> = actions.collect();
        v_flex()
            .gap(px(space::SECTION))
            .child(
                h_flex()
                    .gap(px(10.0))
                    .text_size(px(text::LEDE))
                    .child(status_tag(b.status, p))
                    .child(div().text_size(px(text::MONO)).child(b.number.clone()))
                    .child(format!(
                        "dated {}{}due {}",
                        format_date(&b.bill_date),
                        " \u{b7} ",
                        format_date(&b.due_date)
                    )),
            )
            .when_some(self.done.clone(), |v, done| v.child(done_box(&done, p)))
            .child(
                v_flex()
                    .child(section_header("Bill", self.meta(None, cx), cx))
                    .child(figure_row(&figures, p))
                    .when(b.currency != self.books.as_ref().map_or(String::new(), |s| s.settings.base_currency.clone()), |v| {
                        v.child(hint(&format!("Booked at {} per {}.", b.fx_rate, b.currency), cx))
                    }),
            )
            .child(
                v_flex()
                    .child(section_header("What happens next", div(), cx))
                    .child(
                        v_flex()
                            .py(px(space::CELL))
                            .gap(px(12.0))
                            .child(div().child(d.next.clone()))
                            .when_some(self.action_failure.clone(), |v, f| {
                                v.child(failure_box(&f, cx))
                            })
                            .when(self.pay.is_none(), |v| {
                                v.child(h_flex().gap(px(8.0)).children(actions))
                            })
                            .when(self.pay.is_some(), |v| v.child(self.pay_form(d, cx))),
                    ),
            )
            .child(
                v_flex()
                    .child(section_header("Coding", div(), cx))
                    .child(coded_lines(d, p)),
            )
            .child(Self::payments(d, cx))
            .child(Self::in_the_books(d, cx))
            .child(
                v_flex()
                    .child(section_header("Status trail", div(), cx))
                    .children(d.trail.iter().map(|t| {
                        h_flex()
                            .min_h(px(40.0))
                            .gap(px(16.0))
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .child(
                                div()
                                    .w(px(120.0))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(t.when.clone()),
                            )
                            .child(div().flex_1().child(t.what.clone()))
                            .child(div().text_color(p.ink_muted.to_hsla()).child(t.who.clone()))
                    })),
            )
            .into_any_element()
    }

    fn pay_form(&self, d: &BillDetail, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let e = &self.pay_errors;
        v_flex()
            .p(px(16.0))
            .gap(px(12.0))
            .max_w(px(640.0))
            .border(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::BOLD)
                    .child("Record a payment"),
            )
            .child(div().text_color(p.ink_muted.to_hsla()).child(format!(
                "{} {} outstanding. Part payments are allowed.",
                d.bill.currency,
                format_minor(d.outstanding.unwrap_or(0))
            )))
            .when_some(self.pay_failure.clone(), |v, f| v.child(failure_box(&f, cx)))
            .child(
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
                    .child(
                        field(
                            &format!("Amount, {}", d.bill.currency),
                            Input::new(&self.pay_amount),
                            e.amount.clone(),
                            cx,
                        )
                        .flex_1(),
                    )
                    .child(
                        field("Date paid", Input::new(&self.pay_date), e.date.clone(), cx)
                            .flex_1(),
                    ),
            )
            .child(field(
                "Paid from",
                Select::new(&self.pay_cash).placeholder("Bank, M-Pesa or petty cash"),
                e.cash.clone(),
                cx,
            ))
            .child(field("Reference", Input::new(&self.pay_reference), None, cx))
            .child(form_buttons(
                "pay",
                "Record payment",
                |panel: &mut Self, _, cx| panel.confirm_pay(cx),
                |panel: &mut Self| panel.pay = None,
                cx,
            ))
            .into_any_element()
    }

    fn payments(d: &BillDetail, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .child(section_header(
                "Payments",
                div().child(d.payments.len().to_string()),
                cx,
            ))
            .when(d.payments.is_empty(), |v| {
                v.child(
                    div()
                        .py(px(space::CELL))
                        .text_color(p.ink_muted.to_hsla())
                        .child("No payments recorded yet."),
                )
            })
            .children(d.payments.iter().map(|pay| {
                let split = pay
                    .fund_split
                    .iter()
                    .map(|(_, name, amount)| format!("{name} {}", format_minor(*amount)))
                    .collect::<Vec<_>>()
                    .join(" \u{b7} ");
                v_flex()
                    .py(px(10.0))
                    .gap(px(2.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                                "{} on {}",
                                money(pay.amount, &pay.currency),
                                format_date(&pay.date)
                            )))
                            .when_some(pay.entry.as_ref(), |h, e| {
                                h.child(div().text_size(px(text::MONO)).child(e.detail.number.clone()))
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(pay.source.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!("Fund split: {split}")),
                    )
            }))
            .into_any_element()
    }

    fn in_the_books(d: &BillDetail, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let mut entries: Vec<(String, &EntryView)> = Vec::new();
        if let Some(e) = &d.posting {
            entries.push(("Bill posted: costs and the amount owed".to_owned(), e));
        }
        for pay in &d.payments {
            if let Some(e) = &pay.entry {
                entries.push((format!("Payment of {}", format_date(&pay.date)), e));
            }
        }
        v_flex()
            .child(section_header("In the books", div(), cx))
            .when(entries.is_empty(), |v| {
                v.child(
                    div()
                        .py(px(space::CELL))
                        .text_color(p.ink_muted.to_hsla())
                        .child(
                            "Not posted yet. A journal entry is made when the bill is posted.",
                        ),
                )
            })
            .children(entries.into_iter().map(|(label, e)| entry_table(&label, e, p)))
            .into_any_element()
    }
}

/// Amounts with the currency, as `Money value currency`.
fn money(minor: i64, currency: &str) -> String {
    format!("{currency} {}", format_minor(minor))
}

/// `formatMinor(x, { decimals: 0, zero: '0' })`: whole units with grouping.
fn format_whole(minor: i64) -> String {
    if minor == 0 {
        return "0".to_owned();
    }
    // Rounded half away from zero, as Intl rounds.
    let rounded = (minor.abs() + 50) / 100 * 100 * minor.signum();
    if rounded == 0 {
        return "0".to_owned();
    }
    format_minor(rounded).replace(".00", "")
}

fn col_bill() -> Div {
    div().flex_1().min_w(px(200.0)).px(px(8.0)).py(px(6.0))
}

fn col_wide() -> Div {
    div().w(px(240.0)).flex_shrink_0().px(px(8.0))
}

fn col_date() -> Div {
    div().w(px(128.0)).flex_shrink_0().px(px(8.0))
}

fn col_count() -> Div {
    div().w(px(64.0)).flex_shrink_0().px(px(8.0)).flex().justify_end()
}

fn col_money() -> Div {
    div()
        .w(px(150.0))
        .flex_shrink_0()
        .px(px(8.0))
        .flex()
        .flex_col()
        .items_end()
}

fn col_status() -> Div {
    div().w(px(140.0)).flex_shrink_0().px(px(8.0))
}

/// `DocStatus` (`-shared.tsx:88-118`): a square tag whose frame follows the status's tone.
fn status_tag(status: BillStatus, p: &Palette) -> impl IntoElement {
    let tag = div()
        .px(px(6.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .text_size(px(text::LABEL))
        .font_weight(FontWeight::BOLD)
        .child(status_label(status).to_uppercase());
    match status_tone(status) {
        Tone::Quiet => tag
            .border_1()
            .border_color(p.ink_muted.to_hsla())
            .text_color(p.ink_muted.to_hsla()),
        Tone::Open => tag.border_1().border_color(p.ink_navy.to_hsla()),
        Tone::Booked => tag
            .bg(p.ink_navy.to_hsla())
            .text_color(p.report_paper.to_hsla()),
        Tone::Done => tag
            .bg(p.report_blue.to_hsla())
            .text_color(p.report_paper.to_hsla()),
        Tone::Rejected => tag
            .border_2()
            .border_color(p.rejection_red.to_hsla())
            .text_color(p.rejection_red.to_hsla()),
    }
}

/// `ReportNotice`: a title and a line under it.
fn notice(title: &str, body: &str, cx: &App) -> Div {
    let p = palette(cx.theme().mode);
    v_flex()
        .py(px(space::CELL))
        .gap(px(4.0))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(title.to_owned()))
        .child(div().text_color(p.ink_muted.to_hsla()).child(body.to_owned()))
}

fn done_box(text_: &str, p: &Palette) -> Div {
    div()
        .py(px(12.0))
        .border_t(px(space::HAIRLINE))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .font_weight(FontWeight::SEMIBOLD)
        .child(text_.to_owned())
}

/// `KeyFigures`: the first figure large, the rest at title size.
fn figure_row(figures: &[(String, String)], p: &Palette) -> Div {
    h_flex()
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .children(figures.iter().enumerate().map(|(i, (value, label))| {
            let size = if i == 0 { text::FIGURE } else { text::TITLE };
            v_flex()
                .flex_1()
                .p(px(space::CELL))
                .when(i > 0, |c| {
                    c.border_l(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                })
                .child(
                    div()
                        .text_size(px(size))
                        .line_height(px(size))
                        .font_weight(FontWeight::BOLD)
                        .child(value.clone()),
                )
                .child(
                    div()
                        .pt(px(6.0))
                        .text_size(px(text::LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink_muted.to_hsla())
                        .child(label.to_uppercase()),
                )
        }))
}

/// `CodedLines` (`-shared.tsx:185-282`).
fn coded_lines(d: &BillDetail, p: &Palette) -> Div {
    let header = h_flex()
        .h(px(36.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted.to_hsla())
        .child(col_bill().child("Description"))
        .child(col_wide().child("Account"))
        .child(col_wide().child("Fund and grant"))
        .child(col_date().child("Project, place"))
        .child(col_money().child("Amount"));
    let count = d.lines.len();
    v_flex()
        .child(header)
        .children(d.lines.iter().map(|l| {
            h_flex()
                .min_h(px(44.0))
                .items_start()
                .py(px(6.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(col_bill().font_weight(FontWeight::MEDIUM).child(l.description.clone()))
                .child(col_wide().child(l.account.clone()))
                .child(
                    col_wide().child(l.fund.clone()).child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(l.grant.clone()),
                    ),
                )
                .child(
                    col_date()
                        .text_color(p.ink_muted.to_hsla())
                        .child(l.dimensions.clone()),
                )
                .child(col_money().font_weight(FontWeight::SEMIBOLD).child(format_minor(l.amount)))
        }))
        .child(
            h_flex()
                .min_h(px(40.0))
                .border_t(px(2.0))
                .border_color(p.ink_navy.to_hsla())
                .child(
                    col_bill()
                        .flex_1()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(format!(
                            "Total, {count} {}",
                            if count == 1 { "line" } else { "lines" }
                        )),
                )
                .child(
                    col_money()
                        .font_weight(FontWeight::BOLD)
                        .child(money(d.total, &d.bill.currency)),
                ),
        )
}

/// One journal entry under the bill: its number, memo and lines with debits and credits in
/// base currency (U4's `EntryDetail`), closed by the totals.
fn entry_table(label: &str, e: &EntryView, p: &Palette) -> AnyElement {
    let d = &e.detail;
    let muted = p.ink_muted.to_hsla();
    let amount = |m: i64| {
        if m == 0 {
            String::new()
        } else {
            format_minor(m)
        }
    };
    v_flex()
        .pt(px(12.0))
        .child(
            h_flex()
                .justify_between()
                .pb(px(6.0))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(label.to_owned()))
                .child(
                    h_flex()
                        .gap(px(10.0))
                        .text_color(muted)
                        .child(format_date(&d.date))
                        .child(div().text_size(px(text::MONO)).text_color(p.ink_navy.to_hsla()).child(d.number.clone())),
                ),
        )
        .child(div().pb(px(6.0)).text_size(px(12.0)).text_color(muted).child(d.memo.clone()))
        .child(
            h_flex()
                .h(px(32.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(muted)
                .child(col_bill().child("Account"))
                .child(col_wide().child("Fund"))
                .child(col_money().child("Debit"))
                .child(col_money().child("Credit")),
        )
        .children(d.lines.iter().map(|l| {
            h_flex()
                .min_h(px(36.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(col_bill().child(l.account.clone()))
                .child(col_wide().text_color(muted).child(l.fund.clone()))
                .child(col_money().child(amount(l.debit)))
                .child(col_money().child(amount(l.credit)))
        }))
        .child(
            h_flex()
                .min_h(px(36.0))
                .border_t(px(2.0))
                .border_color(p.ink_navy.to_hsla())
                .font_weight(FontWeight::BOLD)
                .child(col_bill().child(if e.balanced() { "Balanced" } else { "Out of balance" }))
                .child(col_wide())
                .child(col_money().child(format_minor(d.debits)))
                .child(col_money().child(format_minor(d.credits))),
        )
        .into_any_element()
}

impl Render for BillsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let (title, lede) = match (self.tab, &self.page) {
            (Tab::Vendors, _) => (
                "Vendors".to_owned(),
                "The suppliers the books pay, with what their posted bills still owe.".to_owned(),
            ),
            (Tab::Bills, Page::List) => (
                "Bills".to_owned(),
                "What suppliers have invoiced: approved, posted to the books as money owed, then \
                 paid."
                    .to_owned(),
            ),
            (Tab::Bills, Page::New) => (
                "New bill".to_owned(),
                "Enter a supplier\u{2019}s invoice as it was received. It is saved as a draft; \
                 once approved and posted it shows in the books as money owed."
                    .to_owned(),
            ),
            (Tab::Bills, Page::Detail(_)) => (
                self.detail
                    .as_ref()
                    .map_or_else(|| "Bill".to_owned(), |d| d.vendor.clone()),
                "Escape goes back to all bills.".to_owned(),
            ),
        };
        let action: AnyElement = match (self.tab, &self.page) {
            (Tab::Vendors, _) => Button::new("open-vendor")
                .primary()
                .label("Add a supplier")
                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                    panel.open_vendor(window, cx);
                }))
                .into_any_element(),
            (Tab::Bills, Page::New) => div().into_any_element(),
            (Tab::Bills, Page::Detail(_)) => Button::new("back")
                .outline()
                .label("All bills")
                .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                    panel.page = Page::List;
                    cx.notify();
                }))
                .into_any_element(),
            (Tab::Bills, Page::List) => Button::new("open-new")
                .primary()
                .label("New bill")
                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                    panel.open_new(window, cx);
                }))
                .into_any_element(),
        };
        let body = if self.books.is_none() {
            div()
                .text_color(p.ink_muted.to_hsla())
                .child("No books loaded for this organisation.")
                .into_any_element()
        } else {
            match (self.tab, &self.page) {
                (Tab::Vendors, _) => self.vendors_tab(cx),
                (Tab::Bills, Page::List) => self.list(cx),
                (Tab::Bills, Page::New) => self.new_form(cx),
                (Tab::Bills, Page::Detail(_)) => self.detail_view(cx),
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
                            .child(div().text_size(px(text::LEDE)).child(lede)),
                    )
                    .child(action),
            )
            .child(self.tabs(cx))
            .child(body)
    }
}
