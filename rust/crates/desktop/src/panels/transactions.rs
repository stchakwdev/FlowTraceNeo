//! The Transactions screen (DT3): the list with its status filters, counts, search and total
//! (`transactions/index.tsx`), one transaction's page with its movement, Proofs section, record
//! and notes (`transactions/$id.tsx`), and the record-a-transaction form (`transactions/new.tsx`,
//! in the child module `new_form`).
//!
//! Everything shown comes from [`crate::transactions`] over the store's transparency records;
//! proofs come from the shared [`ProofsSection`]. Writes go through the store
//! ([`crate::panels::write_flow`], `FlowWrite::CreateTransaction`) and the new row comes back
//! through its change channel. Keys (no digit is left): `n` records a transaction, `l` the list,
//! `f` the next filter, Up and Down move through the rows and Enter opens one, Esc goes back;
//! on a page the Proofs section's keys (`a`, `v`, Esc, the arrows).

mod new_form;

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Window, div, px,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::NewTransaction;

use crate::panels::dashboard::{notice, proof_legend, small_muted, status_mark};
use crate::panels::proof_section::ProofsSection;
use crate::panels::{demo_tag, page_title, section_header};
use crate::proofs::{LinkKind, ProofFile};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};
use crate::transactions::{
    Detail, Errors, Filter, List, NOT_FOUND_ACTION, NOT_FOUND_BODY, NOT_FOUND_TITLE, NewForm, Row,
};

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionsView {
    /// Every transaction.
    List,
    /// The record-a-transaction form.
    New,
    /// One transaction's page.
    Detail(String),
}

impl TransactionsView {
    /// The view a `--transactions` value names: `list`, `new`, a filter (`evidenced`,
    /// `awaiting-proof`, `flagged`, the web's aliases) or a transaction id.
    #[must_use]
    pub fn from_launch(value: &str) -> (Self, Option<Filter>) {
        match value {
            "list" => (Self::List, None),
            "new" => (Self::New, None),
            other => Filter::from_alias(other).map_or_else(
                || (Self::Detail(other.to_owned()), None),
                |f| (Self::List, Some(f)),
            ),
        }
    }
}

/// A save waiting for its row: the ids before it, what was sent, the queued proof files.
struct Awaiting {
    before: BTreeSet<String>,
    saved: NewTransaction,
    files: Vec<ProofFile>,
    kind: ledger_module::transparency::types::VerificationType,
}

/// The Transactions screen.
pub struct TransactionsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    records: Option<Arc<OrgTransparency>>,
    demo: bool,
    currency: Option<String>,
    view: TransactionsView,

    filter: Filter,
    query: Entity<InputState>,
    list: Option<List>,
    highlight: Option<usize>,
    detail: Option<Detail>,
    section: Entity<ProofsSection>,

    form: NewForm,
    errors: Option<Errors>,
    save_error: Option<String>,
    saving: bool,
    uploading: bool,
    awaiting: Option<Awaiting>,
    inputs: new_form::Inputs,

    _subscriptions: Vec<Subscription>,
}

impl TransactionsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Description, reference or node"));
        let section = cx.new(|cx| ProofsSection::new(store.clone(), window, cx));
        let inputs = new_form::Inputs::new(window, cx);
        let mut subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx)
            }),
            cx.observe(&section, |_, _, cx| cx.notify()),
            cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.rederive(cx);
                }
            }),
        ];
        subscriptions.extend(inputs.subscriptions(window, cx));
        let mut panel = Self {
            store,
            seen: None,
            records: None,
            demo: false,
            currency: None,
            view: TransactionsView::List,
            filter: Filter::All,
            query,
            list: None,
            highlight: None,
            detail: None,
            section,
            form: NewForm::default(),
            errors: None,
            save_error: None,
            saving: false,
            uploading: false,
            awaiting: None,
            inputs,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Opens `view`, for the launch flag, the rail, the Dashboard's button and the keys.
    pub fn show(&mut self, view: TransactionsView, window: &mut Window, cx: &mut Context<Self>) {
        if view == TransactionsView::New && self.view != TransactionsView::New {
            self.open_new(window, cx);
        }
        self.view = view;
        self.highlight = None;
        self.rederive(cx);
    }

    /// Sets the status filter.
    pub fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        self.filter = filter;
        self.highlight = None;
        self.rederive(cx);
    }

    /// Keys while the screen shows and no field has focus.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if matches!(self.view, TransactionsView::Detail(_))
            && self
                .section
                .update(cx, |s, cx| s.press_key(key, window, cx))
        {
            return true;
        }
        match (key, &self.view) {
            ("n", _) => self.show(TransactionsView::New, window, cx),
            ("l", _) => self.show(TransactionsView::List, window, cx),
            ("escape", TransactionsView::Detail(_) | TransactionsView::New) if !self.saving => {
                self.show(TransactionsView::List, window, cx);
            }
            ("f", TransactionsView::List) => {
                let at = Filter::ALL
                    .iter()
                    .position(|f| *f == self.filter)
                    .unwrap_or(0);
                self.set_filter(Filter::ALL[(at + 1) % Filter::ALL.len()], cx);
            }
            ("down" | "up", TransactionsView::List) => {
                let n = self.list.as_ref().map_or(0, |l| l.rows.len());
                if n == 0 {
                    return false;
                }
                self.highlight = Some(match (self.highlight, key) {
                    (None, _) => 0,
                    (Some(i), "down") => (i + 1).min(n - 1),
                    (Some(i), _) => i.saturating_sub(1),
                });
                cx.notify();
            }
            ("enter", TransactionsView::List) => {
                let Some(id) = self
                    .highlight
                    .and_then(|i| self.list.as_ref()?.rows.get(i))
                    .map(|r| r.id.clone())
                else {
                    return false;
                };
                self.show(TransactionsView::Detail(id), window, cx);
            }
            ("enter", TransactionsView::New) => self.save(window, cx),
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
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        self.currency = self
            .records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.currency.clone()))
            .or_else(|| view.selected().map(|o| o.base_currency.clone()));
        if org_changed {
            self.view = TransactionsView::List;
            self.awaiting = None;
            self.saving = false;
        }
        self.inputs.load_nodes(self.records.as_deref(), window, cx);
        self.find_saved(cx);
        self.rederive(cx);
    }

    /// Derives the list, the open page and the Proofs section's link.
    fn rederive(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().to_string();
        self.list = self
            .records
            .as_deref()
            .map(|t| List::of(t, self.filter, &query));
        self.detail = match (&self.view, self.records.as_deref()) {
            (TransactionsView::Detail(id), Some(t)) => Detail::of(t, id),
            _ => None,
        };
        let link = self
            .detail
            .as_ref()
            .map(|d| (LinkKind::Transaction, d.id.clone()));
        self.section.update(cx, |s, cx| s.show(link, cx));
        cx.notify();
    }

    fn header(&self, lede: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .justify_between()
            .items_end()
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(page_title("Transactions"))
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(lede.to_owned())
                            .when(self.demo, |m| m.child(demo_tag(cx))),
                    ),
            )
            .child(
                Button::new("record-transaction")
                    .primary()
                    .label("Record a transaction  n")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.show(TransactionsView::New, window, cx);
                    })),
            )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the web's list page, read top to bottom"
    )]
    fn list_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(list) = &self.list else {
            return v_flex()
                .gap(px(space::SECTION))
                .child(self.header(crate::transactions::LEDE_NONE, cx))
                .child(notice(
                    "Transactions could not be loaded",
                    "Transactions load with the organisation's records from the server. Check \
                     your connection, then try again.",
                    p,
                ))
                .into_any_element();
        };
        let filters: Vec<AnyElement> = Filter::ALL
            .iter()
            .map(|f| {
                let f = *f;
                let active = f == self.filter;
                let label = format!("{}  {}", f.label(), list.counts.of(f));
                let button = Button::new(SharedString::from(format!("txn-filter-{}", f.label())))
                    .label(label)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.set_filter(f, cx);
                    }));
                if active {
                    button.primary()
                } else {
                    button.outline()
                }
                .into_any_element()
            })
            .collect();
        let body = if let Some(n) = &list.notice {
            v_flex()
                .child(notice(&n.title, &n.body, p))
                .child(
                    div().child(
                        Button::new("txn-notice-action")
                            .outline()
                            .label(n.action)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                if this.list.as_ref().is_some_and(|l| l.counts.all == 0) {
                                    this.show(TransactionsView::New, window, cx);
                                } else {
                                    this.query.update(cx, |q, cx| q.set_value("", window, cx));
                                    this.set_filter(Filter::All, cx);
                                }
                            })),
                    ),
                )
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = list
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| Self::row(r, self.highlight == Some(i), cx))
                .collect();
            v_flex()
                .child(table_head(p))
                .children(rows)
                .when_some(list.footer.clone(), |v, f| {
                    v.child(
                        h_flex()
                            .py(px(12.0))
                            .gap(px(16.0))
                            .border_t(px(2.0))
                            .border_color(p.ink_navy.to_hsla())
                            .child(div().flex_1().child(small_muted(&f.label, p)))
                            .child(
                                h_flex()
                                    .w(px(170.0))
                                    .justify_end()
                                    .gap(px(6.0))
                                    .child(small_muted(&f.currency, p))
                                    .child(
                                        div().font_weight(FontWeight::BOLD).child(f.amount.clone()),
                                    ),
                            )
                            .child(div().w(px(220.0))),
                    )
                })
                .into_any_element()
        };
        v_flex()
            .gap(px(space::SECTION))
            .child(self.header(&list.lede, cx))
            .child(
                v_flex()
                    .child(section_header("Movements", proof_legend(p), cx))
                    .child(
                        h_flex()
                            .py(px(12.0))
                            .gap(px(8.0))
                            .justify_between()
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .child(h_flex().gap(px(4.0)).children(filters))
                            .child(div().w(px(320.0)).child(Input::new(&self.query))),
                    )
                    .child(body),
            )
            .into_any_element()
    }

    fn row(r: &Row, highlighted: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        h_flex()
            .id(SharedString::from(format!("txn-{}", r.id)))
            .py(px(12.0))
            .gap(px(16.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .when(highlighted, |row| row.bg(p.wash.to_hsla()))
            .hover(|row| row.bg(p.wash.to_hsla()))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.show(TransactionsView::Detail(id.clone()), window, cx);
            }))
            .child(div().w(px(110.0)).child(small_muted(&r.date, p)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(2.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.description.clone()),
                    )
                    .child(small_muted(&r.reference, p)),
            )
            .child(
                div()
                    .w(px(300.0))
                    .child(format!("{} \u{2192} {}", r.from, r.to)),
            )
            .child(
                h_flex()
                    .w(px(170.0))
                    .justify_end()
                    .gap(px(6.0))
                    .child(small_muted(&r.currency, p))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.amount.clone()),
                    ),
            )
            .child(
                div()
                    .w(px(70.0))
                    .flex()
                    .justify_end()
                    .text_color(if r.proofs == 0 {
                        p.ink_muted.to_hsla()
                    } else {
                        p.ink_navy.to_hsla()
                    })
                    .child(r.proofs.to_string()),
            )
            .child(div().w(px(150.0)).pl(px(16.0)).child(status_mark(
                r.status,
                crate::dashboard::proof_label(r.status),
                p,
            )))
            .into_any_element()
    }

    fn back(cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("txn-back")
            .ghost()
            .small()
            .label("\u{2190} Transactions  Esc")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.show(TransactionsView::List, window, cx);
            }))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the web's transaction page, read top to bottom"
    )]
    fn detail_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        if self.section.read(cx).viewing() {
            return self.section.clone().into_any_element();
        }
        let Some(d) = &self.detail else {
            return v_flex()
                .gap(px(16.0))
                .child(div().child(Self::back(cx)))
                .child(notice(NOT_FOUND_TITLE, NOT_FOUND_BODY, p))
                .child(
                    div().child(
                        Button::new("txn-see-all")
                            .primary()
                            .label(NOT_FOUND_ACTION)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.show(TransactionsView::List, window, cx);
                            })),
                    ),
                )
                .into_any_element();
        };
        let header = v_flex()
            .gap(px(12.0))
            .child(div().child(Self::back(cx)))
            .child(page_title(&d.title))
            .child(
                h_flex()
                    .gap(px(16.0))
                    .text_size(px(text::BODY))
                    .text_color(p.ink_muted.to_hsla())
                    .child(status_mark(d.status, d.status_label, p))
                    .child(d.date.clone())
                    .when_some(d.reference.clone(), ParentElement::child)
                    .when(self.demo, |m| m.child(demo_tag(cx))),
            );
        let movement = v_flex().child(section_header("Movement", div(), cx)).child(
            v_flex()
                .py(px(20.0))
                .child(
                    h_flex()
                        .items_baseline()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(24.0))
                                .text_color(p.ink_muted.to_hsla())
                                .child(d.currency.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(text::FIGURE))
                                .line_height(px(text::FIGURE))
                                .font_weight(FontWeight::BOLD)
                                .child(d.amount.clone()),
                        ),
                )
                .when_some(d.about_usd.clone(), |v, usd| {
                    v.child(div().pt(px(8.0)).child(small_muted(&usd, p)))
                })
                .child(flow_drawing(d, p)),
        );
        let facts: Vec<AnyElement> = d
            .facts
            .iter()
            .map(|f| {
                h_flex()
                    .py(px(10.0))
                    .gap(px(12.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        div()
                            .w(px(112.0))
                            .flex_shrink_0()
                            .text_color(p.ink_muted.to_hsla())
                            .child(f.term),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .when(f.muted, |d| d.text_color(p.ink_muted.to_hsla()))
                            .child(f.value.clone()),
                    )
                    .into_any_element()
            })
            .collect();
        let aside = v_flex()
            .w(px(380.0))
            .flex_shrink_0()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .child(section_header("Record", div(), cx))
                    .children(facts),
            )
            .child(
                v_flex().child(section_header("Notes", div(), cx)).child(
                    div()
                        .py(px(12.0))
                        .when(!d.has_notes, |n| n.text_color(p.ink_muted.to_hsla()))
                        .child(d.notes.clone()),
                ),
            );
        v_flex()
            .gap(px(space::SECTION))
            .child(header)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(40.0))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap(px(space::SECTION))
                            .child(movement)
                            .child(self.section.clone()),
                    )
                    .child(aside),
            )
            .into_any_element()
    }
}

/// `FlowDrawing` (`$id.tsx:320-386`): from, the line in the state's pattern with its word, to.
fn flow_drawing(d: &Detail, p: &Palette) -> impl IntoElement {
    let side = |s: &crate::transactions::Side, right: bool| {
        v_flex()
            .w(px(220.0))
            .when(right, Styled::items_end)
            .child(small_muted(s.role, p))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(s.name.clone()),
            )
            .when_some(s.kind, |v, k| v.child(small_muted(k, p)))
    };
    h_flex()
        .mt(px(24.0))
        .pt(px(20.0))
        .gap(px(20.0))
        .border_t(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(side(&d.from, false))
        .child(
            v_flex()
                .flex_1()
                .items_center()
                .gap(px(6.0))
                .child(line(d.status, p))
                .child(small_muted(d.status_label, p)),
        )
        .child(side(&d.to, true))
}

/// The movement's line, 4px, in the state's pattern (`proofStroke`).
fn line(status: ledger_module::transparency::flows::TransactionStatus, p: &Palette) -> AnyElement {
    use ledger_module::transparency::flows::TransactionStatus as S;
    let (color, dash, gap) = match status {
        S::Verified => (p.ink_navy.to_hsla(), 1.0, 0.0),
        S::Pending => (p.ink_muted.to_hsla(), 12.0, 8.0),
        S::Flagged => (p.signal_orange.to_hsla(), 18.0, 6.0),
        S::Rejected => (p.rejection_red.to_hsla(), 3.0, 6.0),
    };
    if gap == 0.0 {
        return div().w_full().h(px(4.0)).bg(color).into_any_element();
    }
    h_flex()
        .w_full()
        .h(px(4.0))
        .gap(px(gap))
        .overflow_hidden()
        .children((0..60).map(|_| div().w(px(dash)).h(px(4.0)).flex_shrink_0().bg(color)))
        .into_any_element()
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
        .child(head("Date").w(px(110.0)))
        .child(head("Description").flex_1())
        .child(head("From \u{2192} To").w(px(300.0)))
        .child(head("Amount").w(px(170.0)).flex().justify_end())
        .child(head("Proofs").w(px(70.0)).flex().justify_end())
        .child(head("Status").w(px(150.0)).pl(px(16.0)))
}

impl Render for TransactionsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.view {
            TransactionsView::List => self.list_page(cx),
            TransactionsView::New => self.new_page(cx),
            TransactionsView::Detail(_) => self.detail_page(cx),
        }
    }
}
