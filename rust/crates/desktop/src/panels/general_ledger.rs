//! The General ledger panel: a filter bar driving a [`GlFilter`], key figures, a gpui-kit
//! `DataTable` of every posted line with its running balance and a totals row, and a side panel
//! with the entry of the clicked line.
//!
//! The rows come from [`GlCache`], keyed by the store's revision and the filter, so a re-render
//! never derives them again. The table virtualises its rows: `TableState` renders the body
//! through GPUI's `uniform_list`, which lays out only the rows in view
//! (<https://gpui-kit.com/component/data-table>; gpui-component 0.7.3 `table/state.rs`,
//! `uniform_list("table-uniform-list", ...)`). Column alignment is done in `render_td` and
//! `render_th` because `DataTable` ignores `Column::align` (ADR 0002).
//!
//! A click on a line shows its entry here and emits [`OpenEntry`] for the Journal screen (U3)
//! to hook.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, AppContext as _, Context, Div, Entity, EventEmitter, FontWeight, IntoElement,
    ParentElement as _, Render, SharedString, Styled, Subscription, Window, div, px,
};
use ledger_core::types::{EntryId, IsoDate, LedgerState, OrganizationId};

use crate::accounts::{format_date, format_minor};
use crate::general_ledger::{
    COLUMNS, Choice, EntryDetail, FilterChoices, GlCache, GlColumn, GlFilter, GlTable, balance,
    count,
};
use crate::panels::{Pick, PickState, demo_tag, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{palette, space, text};

/// Emitted when a line is clicked: the Journal screen (U3) opens this entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenEntry(pub EntryId);



fn picks<T: AsRef<str>>(choices: &[Choice<T>]) -> Vec<Pick> {
    choices
        .iter()
        .map(|c| Pick {
            value: c
                .value
                .as_ref()
                .map_or_else(String::new, |v| v.as_ref().to_owned()),
            label: c.label.clone().into(),
        })
        .collect()
}

/// The select value of an optional id: the id, or empty for "All ...".
fn id_of<T: AsRef<str>>(id: Option<&T>) -> String {
    id.map_or_else(String::new, |v| v.as_ref().to_owned())
}

/// Which select changed.
#[derive(Debug, Clone, Copy)]
enum Field {
    Account,
    Fund,
    Grant,
    Dimension,
    Period,
}

/// The table's rows: the cached [`GlTable`].
pub struct GlRows {
    table: Arc<GlTable>,
    columns: Vec<Column>,
}

impl GlRows {
    fn new() -> Self {
        Self {
            table: Arc::default(),
            columns: COLUMNS
                .iter()
                .map(|c| Column::new(c.label(), c.label()).width(px(c.width())))
                .collect(),
        }
    }

    fn aligned(col_ix: usize, text: impl Into<SharedString>) -> Div {
        let right = COLUMNS.get(col_ix).is_some_and(|c| c.is_amount());
        h_flex()
            .size_full()
            .overflow_hidden()
            .when(right, Styled::justify_end)
            .child(text.into())
    }
}

impl TableDelegate for GlRows {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.table.rows_with_totals()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns.get(col_ix).cloned().unwrap_or_default()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let label = COLUMNS.get(col_ix).map_or("", |c| c.label());
        Self::aligned(col_ix, label).font_weight(FontWeight::SEMIBOLD)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let column = COLUMNS.get(col_ix).copied().unwrap_or(GlColumn::Memo);
        let cell = Self::aligned(col_ix, self.table.cell(row_ix, column));
        if self.table.is_totals_row(row_ix) {
            cell.font_weight(FontWeight::BOLD)
        } else if column == GlColumn::Date || column == GlColumn::Fund {
            cell.text_color(palette(cx.theme().mode).ink_muted.to_hsla())
        } else {
            cell
        }
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        COLUMNS
            .get(col_ix)
            .map(|c| self.table.cell(row_ix, *c))
            .unwrap_or_default()
    }
}

/// What the app was asked to show first on this screen (`main.rs` flags), for screenshots and
/// the scroll measurement.
#[derive(Debug, Clone, Default)]
pub struct GlLaunch {
    /// Filter on the account with this template code, for example `1110`.
    pub account_code: Option<String>,
    /// Scroll to this row and open its entry.
    pub row: Option<usize>,
    /// Scroll the whole table once, a few rows a frame, and log the frame times.
    pub scroll_bench: bool,
}

/// Rows scrolled per frame by the scroll measurement: about one viewport and a half.
const BENCH_STEP: usize = 25;

/// The General ledger panel.
pub struct GeneralLedgerPanel {
    store: Entity<LedgerStore>,
    filter: GlFilter,
    cache: GlCache,
    rows: Arc<GlTable>,
    table: Entity<TableState<GlRows>>,
    account: PickState,
    fund: PickState,
    grant: PickState,
    dimension: PickState,
    period: PickState,
    from: Entity<InputState>,
    to: Entity<InputState>,
    date_error: Option<SharedString>,
    detail: Option<EntryDetail>,
    org: Option<OrganizationId>,
    choices_revision: Option<u64>,
    bench: Option<Bench>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenEntry> for GeneralLedgerPanel {}

impl GeneralLedgerPanel {
    /// Builds the panel over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table = cx.new(|cx| {
            TableState::new(GlRows::new(), window, cx)
                .row_selectable(true)
                .col_movable(false)
                .sortable(false)
        });
        let select = |cx: &mut Context<Self>, window: &mut Window| {
            cx.new(|cx| SelectState::new(Vec::<Pick>::new(), Some(IndexPath::new(0)), window, cx))
        };
        let account = select(cx, window);
        let fund = select(cx, window);
        let grant = select(cx, window);
        let dimension = select(cx, window);
        let period = select(cx, window);
        let from = cx.new(|cx| InputState::new(window, cx).placeholder("From, YYYY-MM-DD"));
        let to = cx.new(|cx| InputState::new(window, cx).placeholder("To, YYYY-MM-DD"));

        let mut subscriptions = vec![
            cx.observe_in(&store, window, |panel, _, window, cx| {
                panel.refresh(window, cx);
            }),
            cx.subscribe_in(&table, window, Self::on_table_event),
            cx.subscribe_in(&from, window, Self::on_date),
            cx.subscribe_in(&to, window, Self::on_date),
        ];
        for (state, field) in [
            (&account, Field::Account),
            (&fund, Field::Fund),
            (&grant, Field::Grant),
            (&dimension, Field::Dimension),
            (&period, Field::Period),
        ] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                move |panel, _, event: &SelectEvent<Vec<Pick>>, window, cx| {
                    let SelectEvent::Confirm(value) = event;
                    panel.on_pick(field, value.as_deref().unwrap_or(""), window, cx);
                },
            ));
        }

        let mut panel = Self {
            store,
            filter: GlFilter::default(),
            cache: GlCache::default(),
            rows: Arc::default(),
            table,
            account,
            fund,
            grant,
            dimension,
            period,
            from,
            to,
            date_error: None,
            detail: None,
            org: None,
            choices_revision: None,
            bench: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Applies the launch flags: a filter, a row to open, the scroll measurement.
    pub fn launch(&mut self, launch: &GlLaunch, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(code) = &launch.account_code {
            self.filter.account = Some(ledger_core::coa_template::account_id(code));
            self.choices_revision = None;
            self.refresh(window, cx);
        }
        if let Some(row) = launch.row {
            self.table.update(cx, |t, cx| {
                t.scroll_to_row(row, cx);
                t.set_selected_row(row, cx);
            });
        }
        if launch.scroll_bench {
            self.bench = Some(Bench::default());
            let this = cx.entity().downgrade();
            window.on_next_frame(move |window, cx| Self::bench_step(this, 0, window, cx));
        }
    }

    fn books(&self, cx: &App) -> (u64, Option<Arc<LedgerState>>, Option<OrganizationId>) {
        let view = self.store.read(cx).view();
        (
            view.revision(),
            view.snapshot().cloned(),
            view.selected().map(|o| o.id.clone()),
        )
    }

    /// Re-derives the rows (from the cache) and, when the books changed, the select options.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (revision, books, org) = self.books(cx);
        if org != self.org {
            // Another organisation's ids mean nothing here.
            self.org = org;
            self.filter = GlFilter::default();
            self.detail = None;
            self.date_error = None;
            self.choices_revision = None;
            for input in [&self.from, &self.to] {
                input.update(cx, |i, cx| i.set_value("", window, cx));
            }
        }
        let started = Instant::now();
        let before = self.cache.derivations();
        let rows = self.cache.table(revision, books.as_deref(), &self.filter);
        if self.cache.derivations() != before {
            log_timing("rows derived", rows.rows.len(), started.elapsed());
        }
        if !Arc::ptr_eq(&rows, &self.rows) {
            self.rows = Arc::clone(&rows);
            self.table.update(cx, |t, cx| {
                t.delegate_mut().table = rows;
                t.clear_selection(cx);
                t.scroll_to_row(0, cx);
                cx.notify();
            });
        }
        if self.choices_revision != Some(revision) {
            self.choices_revision = Some(revision);
            self.reset_choices(books.as_deref(), window, cx);
        }
        cx.notify();
    }

    /// Fills every select from the books and points each at the filter's value.
    fn reset_choices(
        &mut self,
        books: Option<&LedgerState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(books) = books else {
            return;
        };
        let choices = FilterChoices::new(books, &self.filter);
        let sets = [
            (
                &self.account,
                picks(&choices.accounts),
                id_of(self.filter.account.as_ref()),
            ),
            (
                &self.fund,
                picks(&choices.funds),
                id_of(self.filter.fund.as_ref()),
            ),
            (
                &self.grant,
                picks(&choices.grants),
                id_of(self.filter.grant.as_ref()),
            ),
            (
                &self.dimension,
                picks(&choices.dimensions),
                id_of(self.filter.dimension.as_ref()),
            ),
            (
                &self.period,
                picks(&choices.periods),
                id_of(self.filter.period.as_ref()),
            ),
        ];
        for (state, items, selected) in sets {
            state.update(cx, |s, cx| {
                s.set_items(items, window, cx);
                s.set_selected_value(&selected, window, cx);
            });
        }
    }

    fn on_pick(&mut self, field: Field, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let id = |v: &str| (!v.is_empty()).then(|| v.to_owned());
        let Some(books) = self.books(cx).1 else {
            return;
        };
        match field {
            Field::Account => self.filter.account = id(value).map(Into::into),
            Field::Fund => {
                self.filter.set_fund(id(value).map(Into::into), &books);
                // The grant list follows the fund.
                self.choices_revision = None;
            }
            Field::Grant => self.filter.grant = id(value).map(Into::into),
            Field::Dimension => self.filter.dimension = id(value).map(Into::into),
            Field::Period => self.filter.period = id(value).map(Into::into),
        }
        self.detail = None;
        self.refresh(window, cx);
    }

    fn on_date(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }
        let parse = |input: &Entity<InputState>, cx: &App| {
            let text = input.read(cx).value().trim().to_owned();
            if text.is_empty() {
                Ok(None)
            } else {
                IsoDate::parse(&text).map(Some).map_err(|_| text)
            }
        };
        match (parse(&self.from, cx), parse(&self.to, cx)) {
            (Ok(from), Ok(to)) => {
                self.date_error = None;
                if (from.clone(), to.clone()) != (self.filter.from.clone(), self.filter.to.clone())
                {
                    self.filter.from = from;
                    self.filter.to = to;
                    self.detail = None;
                    self.refresh(window, cx);
                }
            }
            (Err(text), _) | (_, Err(text)) => {
                // Half-typed dates are left alone until they read as a date.
                self.date_error =
                    Some(format!("{text} is not a date: write it as YYYY-MM-DD.").into());
                cx.notify();
            }
        }
    }

    fn on_table_event(
        &mut self,
        _: &Entity<TableState<GlRows>>,
        event: &TableEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let TableEvent::SelectRow(ix) = event else {
            return;
        };
        let Some(row) = self.rows.rows.get(*ix) else {
            return;
        };
        let entry = row.entry_id.clone();
        if let Some(books) = self.books(cx).1 {
            self.detail = EntryDetail::of(&books, &entry);
        }
        cx.emit(OpenEntry(entry));
        cx.notify();
    }

    /// One step of the scroll measurement: scroll on, then come back on the next frame.
    fn bench_step(
        this: gpui_kit::WeakEntity<Self>,
        step: usize,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(panel) = this.upgrade() else {
            return;
        };
        let done = panel.update(cx, |panel, cx| {
            let rows = panel.rows.rows_with_totals();
            let bench = panel.bench.get_or_insert_with(Bench::default);
            bench.tick();
            let row = step * BENCH_STEP;
            if rows == 0 || row >= rows {
                bench.finish(rows);
                panel.bench = None;
                return true;
            }
            panel.table.update(cx, |t, cx| t.scroll_to_row(row, cx));
            false
        });
        if !done {
            window.on_next_frame(move |window, cx| Self::bench_step(this, step + 1, window, cx));
        }
    }

    fn filters(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let field = |label: &'static str, control: gpui_kit::AnyElement| {
            v_flex()
                .w(px(248.0))
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(text::SECTION_LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(label),
                )
                .child(control)
        };
        v_flex()
            .py(px(space::CELL))
            .gap(px(12.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap(px(space::CELL))
                    .child(field(
                        "Account",
                        Select::new(&self.account).into_any_element(),
                    ))
                    .child(field("Fund", Select::new(&self.fund).into_any_element()))
                    .child(field("Grant", Select::new(&self.grant).into_any_element()))
                    .child(field(
                        "Tag",
                        Select::new(&self.dimension).into_any_element(),
                    ))
                    .child(field(
                        "Period",
                        Select::new(&self.period).into_any_element(),
                    ))
                    .child(field("From", Input::new(&self.from).into_any_element()))
                    .child(field("To", Input::new(&self.to).into_any_element())),
            )
            .when_some(self.date_error.clone(), |col, message| {
                col.child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.rejection_red.to_hsla())
                        .child(message),
                )
            })
    }

    fn key_figures(&self, currency: &str, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let t = &self.rows.totals;
        let figure = |value: String, label: String| {
            v_flex()
                .min_w(px(160.0))
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(text::TITLE))
                        .font_weight(FontWeight::BOLD)
                        .child(value),
                )
                .child(
                    div()
                        .text_size(px(text::SECTION_LABEL))
                        .text_color(p.ink_muted.to_hsla())
                        .child(label),
                )
        };
        h_flex()
            .py(px(space::CELL))
            .gap(px(space::SECTION))
            .child(figure(balance(t.debits), format!("Debits, {currency}")))
            .child(figure(balance(t.credits), format!("Credits, {currency}")))
            .child(figure(count(t.lines), "Lines".to_owned()))
            .child(figure(count(t.accounts), "Accounts".to_owned()))
    }

    fn detail_panel(detail: &EntryDetail, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let muted = p.ink_muted.to_hsla();
        let lines = detail.lines.iter().map(|l| {
            v_flex()
                .py(px(8.0))
                .gap(px(2.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(l.account.clone()),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(muted)
                                .child(l.fund.clone()),
                        )
                        .child(if l.debit > 0 {
                            format!("Dr {}", format_minor(l.debit))
                        } else {
                            format!("Cr {}", format_minor(l.credit))
                        }),
                )
                .when_some(l.memo.clone(), |col, memo| {
                    col.child(div().text_size(px(12.0)).text_color(muted).child(memo))
                })
        });
        v_flex()
            .w(px(320.0))
            .flex_shrink_0()
            .p(px(space::CELL))
            .gap(px(8.0))
            .bg(p.sheet.to_hsla())
            .border_1()
            .border_color(p.hairline.to_hsla())
            .child(
                div()
                    .text_size(px(text::SECTION_LABEL))
                    .font_weight(FontWeight::BOLD)
                    .text_color(muted)
                    .child(format!("ENTRY {}", detail.status.to_uppercase())),
            )
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::BOLD)
                    .child(detail.number.clone()),
            )
            .child(div().text_color(muted).child(format!(
                "{} \u{b7} id {}",
                format_date(&detail.date),
                detail.id.as_str()
            )))
            .child(div().pb(px(8.0)).child(detail.memo.clone()))
            .children(lines)
            .child(
                h_flex()
                    .justify_between()
                    .font_weight(FontWeight::BOLD)
                    .child("Totals")
                    .child(format!(
                        "Dr {} \u{b7} Cr {}",
                        balance(detail.debits),
                        balance(detail.credits)
                    )),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(muted)
                    .child("The Journal screen opens this entry once it lands (U3)."),
            )
    }
}

impl Render for GeneralLedgerPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let view = self.store.read(cx).view();
        let demo = view
            .selected()
            .is_some_and(|org| org.id.as_str() == ledger_core::demo::DEMO_ORG_ID);
        let currency = view
            .selected()
            .map_or_else(String::new, |org| org.base_currency.clone());
        let as_of = self
            .filter
            .to
            .as_ref()
            .map(|d| format!("To {}", format_date(d)));
        let meta = h_flex()
            .gap(px(8.0))
            .when_some(as_of, |row, text| {
                row.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(text),
                )
            })
            .when(demo, |row| row.child(demo_tag(cx)));

        let body = if let Some(error) = &self.rows.error {
            div()
                .py(px(space::CELL))
                .text_color(p.rejection_red.to_hsla())
                .child(error.clone())
                .into_any_element()
        } else if self.rows.rows.is_empty() {
            v_flex()
                .py(px(space::CELL))
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .child("No posted lines match"),
                )
                .child(
                    div()
                        .text_color(p.ink_muted.to_hsla())
                        .child("Drafts are not shown here. Widen the dates or remove a filter."),
                )
                .into_any_element()
        } else {
            h_flex()
                .w_full()
                .items_start()
                .gap(px(space::CELL))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(560.0))
                        .overflow_hidden()
                        .child(DataTable::new(&self.table).stripe(true)),
                )
                .when_some(self.detail.as_ref(), |row, detail| {
                    row.child(Self::detail_panel(detail, cx))
                })
                .into_any_element()
        };

        v_flex()
            .w_full()
            .min_w(px(0.0))
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(page_title("General ledger"))
                    .child(div().text_size(px(text::LEDE)).child(
                        "Every posted line with the balance of its account after it. Header \
                         accounts include all of their sub-accounts.",
                    )),
            )
            .child(
                v_flex()
                    .child(section_header("Filters", div(), cx))
                    .child(self.filters(cx)),
            )
            .child(
                v_flex()
                    .child(section_header("Ledger", meta, cx))
                    .child(self.key_figures(&currency, cx))
                    .child(body),
            )
    }
}

/// Frame times of the scroll measurement.
#[derive(Debug, Default)]
struct Bench {
    last: Option<Instant>,
    frames: Vec<Duration>,
    started: Option<Instant>,
}

impl Bench {
    fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last {
            self.frames.push(now - last);
        }
        self.started.get_or_insert(now);
        self.last = Some(now);
    }

    fn finish(&mut self, rows: usize) {
        let mut sorted = self.frames.clone();
        sorted.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let pick = |q: f64| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "a rank within a frame count"
            )]
            let ix = ((sorted.len().saturating_sub(1)) as f64 * q).round() as usize;
            sorted.get(ix).copied().map_or(0.0, ms)
        };
        let total: Duration = self.frames.iter().sum();
        #[expect(clippy::cast_precision_loss, reason = "a frame count")]
        let mean = if self.frames.is_empty() {
            0.0
        } else {
            ms(total) / self.frames.len() as f64
        };
        let slow = self.frames.iter().filter(|d| ms(**d) > 33.4).count();
        eprintln!(
            "desktop: general ledger scroll over {rows} rows, {step} rows a frame: {} frames in {:.1} s; \
             frame time mean {mean:.2} ms, p50 {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms; \
             {slow} frames over 33.4 ms",
            self.frames.len(),
            self.started.map_or(0.0, |s| s.elapsed().as_secs_f64()),
            pick(0.5),
            pick(0.95),
            pick(0.99),
            pick(1.0),
            step = BENCH_STEP,
        );
    }
}

/// Logs a derivation's cost when it is large enough to matter.
fn log_timing(what: &str, rows: usize, took: Duration) {
    if rows >= 1_000 {
        eprintln!(
            "desktop: general ledger {what}: {rows} rows in {:.1} ms",
            took.as_secs_f64() * 1000.0
        );
    }
}
