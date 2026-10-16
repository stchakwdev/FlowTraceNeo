//! The spike window: a status line, a one-field form that calls the reducer, and a
//! gpui-kit `DataTable` over the live rows.
//!
//! Component APIs: <https://gpui-kit.com/docs/getting-started> (window, `Input`,
//! `Button`) and the `gpui_kit::component::table` module docs at
//! <https://docs.rs/gpui-component/0.7.3/gpui_component/table/> (`TableDelegate`,
//! `TableState`, `DataTable`).

use std::sync::Arc;
use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme as _, Theme, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Div, Entity, IntoElement, ParentElement as _,
    Render, SharedString, Styled, Task, TextAlign, Window, div, prelude::FluentBuilder as _, px,
    rgb,
};

use crate::live::{Drive, Live, TICK};
use crate::module_bindings::{DbConnection, SpikeRow, upsert_rows as _};
use crate::rows::{RowEvent, RowStore};

/// Amount the form gives a new row, in minor units.
const NEW_ROW_AMOUNT_MINOR: i64 = 1_250;

/// Applies the light palette from `DESIGN.md` (oklch values converted to sRGB) over the
/// default gpui-kit theme: report paper and sheet surfaces, ink navy text, report blue
/// for the primary action, hairline borders and square corners.
pub fn apply_theme(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.background = rgb(0x00f5_f6f3).into();
        theme.foreground = rgb(0x0012_202f).into();
        theme.muted_foreground = rgb(0x0049_5766).into();
        theme.border = rgb(0x00ca_d2da).into();
        theme.input = rgb(0x00b5_bfc9).into();
        theme.primary = rgb(0x0021_5da5).into();
        theme.primary_hover = rgb(0x001b_4f8d).into();
        theme.primary_active = rgb(0x0017_4378).into();
        theme.primary_foreground = rgb(0x00fa_fcfe).into();
        // Buttons read their own tokens, not `primary`.
        theme.button_primary = rgb(0x0021_5da5).into();
        theme.button_primary_hover = rgb(0x001b_4f8d).into();
        theme.button_primary_active = rgb(0x0017_4378).into();
        theme.button_primary_foreground = rgb(0x00fa_fcfe).into();
        theme.table = rgb(0x00fc_fdfb).into();
        theme.table_head = rgb(0x00e8_ebef).into();
        theme.table_even = rgb(0x00f5_f6f3).into();
        theme.radius = px(0.0);
        theme.radius_lg = px(2.0);
    });
}

/// `amount_minor` as major units with two decimals and no thousands separator, the
/// way the TS engine formats money in messages (`toFixed(2)`,
/// flowtrace/src/lib/ledger/money.ts).
pub fn format_minor(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let abs = amount_minor.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// The table's data: the live rows and their columns.
pub struct RowsTable {
    store: RowStore<SpikeRow>,
    columns: Vec<Column>,
}

impl RowsTable {
    fn new() -> Self {
        Self {
            store: RowStore::default(),
            columns: vec![
                Column::new("id", "Id").width(px(90.0)).text_right(),
                Column::new("label", "Label").width(px(320.0)),
                Column::new("amount", "Amount")
                    .width(px(160.0))
                    .text_right(),
                Column::new("revision", "Revision")
                    .width(px(110.0))
                    .text_right(),
            ],
        }
    }

    fn cell(&self, row_ix: usize, col_ix: usize) -> String {
        let Some(row) = self.store.get(row_ix) else {
            return String::new();
        };
        match col_ix {
            0 => row.id.to_string(),
            1 => row.label.clone(),
            2 => format_minor(row.amount_minor),
            _ => row.revision.to_string(),
        }
    }

    /// `DataTable` sizes the cell but ignores `Column::align`, so the delegate aligns
    /// its own content (gpui-component 0.7.3 `table/state.rs`, `render_cell`).
    fn aligned(&self, col_ix: usize, text: SharedString) -> Div {
        let right = self
            .columns
            .get(col_ix)
            .is_some_and(|c| c.align == TextAlign::Right);
        h_flex()
            .size_full()
            .when(right, Styled::justify_end)
            .child(text)
    }

    fn next_id(&self) -> u32 {
        self.store
            .rows()
            .last()
            .map_or(1, |row| row.id.saturating_add(1))
    }
}

impl TableDelegate for RowsTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.store.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns.get(col_ix).cloned().unwrap_or_default()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let name = self.column(col_ix, cx).name;
        self.aligned(col_ix, name)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        self.aligned(col_ix, SharedString::from(self.cell(row_ix, col_ix)))
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        self.cell(row_ix, col_ix)
    }
}

/// Connection state shown in the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Connecting,
    Loading,
    Live,
    Failed(String),
    Closed(Option<String>),
}

/// The spike window's root entity. It owns the table entity and the connection; a
/// foreground task drains the channel and applies each drained batch in one update.
pub struct SpikeView {
    table: Entity<TableState<RowsTable>>,
    label: Entity<InputState>,
    conn: Arc<DbConnection>,
    drive: Drive,
    database: SharedString,
    status: Status,
    batches: u64,
    events: u64,
    last_batch: usize,
    /// When the connection started being driven; the batch log is relative to it.
    started: Instant,
    _pump: Task<()>,
    _tick: Option<Task<()>>,
}

impl SpikeView {
    /// Builds the view and starts driving `conn` the way `drive` says.
    pub fn new(
        conn: Arc<DbConnection>,
        rx: Receiver<Live>,
        drive: Drive,
        database: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let table = cx.new(|cx| TableState::new(RowsTable::new(), window, cx).row_selectable(true));
        let label = cx.new(|cx| InputState::new(window, cx).placeholder("Label for a new row"));

        // Wake on the first message, then take whatever else is already queued, so the
        // 10,000 initial inserts land as one batch and one repaint.
        let pump = cx.spawn(async move |this, cx| {
            while let Ok(first) = rx.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = rx.try_recv() {
                    batch.push(more);
                }
                if this.update(cx, |view, cx| view.apply(batch, cx)).is_err() {
                    break;
                }
            }
        });

        let started = Instant::now();
        let tick = match drive {
            Drive::Threaded => {
                // The handle is not joined: the thread ends when the socket closes.
                drop(conn.run_threaded());
                None
            }
            Drive::Tick => {
                let conn = Arc::clone(&conn);
                Some(cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor().timer(TICK).await;
                        if conn.frame_tick().is_err() {
                            break;
                        }
                    }
                }))
            }
        };

        Self {
            table,
            label,
            conn,
            drive,
            database,
            status: Status::Connecting,
            batches: 0,
            events: 0,
            last_batch: 0,
            started,
            _pump: pump,
            _tick: tick,
        }
    }

    fn apply(&mut self, batch: Vec<Live>, cx: &mut Context<Self>) {
        let mut rows: Vec<RowEvent<SpikeRow>> = Vec::with_capacity(batch.len());
        for message in batch {
            match message {
                Live::Connected => self.status = Status::Loading,
                Live::Applied => {
                    self.status = Status::Live;
                    eprintln!(
                        "desktop-spike: subscription applied at +{} ms",
                        self.started.elapsed().as_millis()
                    );
                }
                Live::Row(event) => rows.push(event),
                Live::Failed(err) => self.status = Status::Failed(err),
                Live::Disconnected(err) => self.status = Status::Closed(err),
            }
        }
        if !rows.is_empty() {
            self.batches += 1;
            self.events += rows.len() as u64;
            self.last_batch = rows.len();
            let count = rows.len();
            let applying = Instant::now();
            let total = self.table.update(cx, |state, cx| {
                let store = &mut state.delegate_mut().store;
                store.apply(rows);
                cx.notify();
                store.len()
            });
            eprintln!(
                "desktop-spike: {count} row events applied in {} us at +{} ms; {total} rows",
                applying.elapsed().as_micros(),
                self.started.elapsed().as_millis()
            );
        }
        cx.notify();
    }

    fn add_row(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.label.read(cx).value();
        let label = if text.trim().is_empty() {
            "New row".to_owned()
        } else {
            text.trim().to_owned()
        };
        let id = self.table.read(cx).delegate().next_id();
        if let Err(err) = self
            .conn
            .reducers
            .upsert_rows(id, 1, label, NEW_ROW_AMOUNT_MINOR)
        {
            self.status = Status::Failed(err.to_string());
            cx.notify();
        }
    }

    fn status_line(&self, cx: &App) -> String {
        let rows = self.table.read(cx).delegate().store.len();
        let state = match &self.status {
            Status::Connecting => "connecting".to_owned(),
            Status::Loading => "loading subscription".to_owned(),
            Status::Live => "live".to_owned(),
            Status::Failed(err) => format!("error: {err}"),
            Status::Closed(None) => "disconnected".to_owned(),
            Status::Closed(Some(err)) => format!("disconnected: {err}"),
        };
        let drive = match self.drive {
            Drive::Threaded => "run_threaded",
            Drive::Tick => "frame_tick 16 ms",
        };
        format!(
            "{state} | {} | {rows} rows | {} events in {} batches, last {} | drive {drive}",
            self.database, self.events, self.batches, self.last_batch
        )
    }
}

impl Render for SpikeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .p(px(24.0))
            .gap(px(16.0))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .gap(px(4.0))
                    .pb(px(12.0))
                    .border_b(px(3.0))
                    .border_color(theme.foreground)
                    .child(div().text_size(px(24.0)).child("FlowTrace Books"))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.muted_foreground)
                            .child(self.status_line(cx)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.0))
                    .child(div().w(px(360.0)).child(Input::new(&self.label)))
                    .child(
                        Button::new("add-row")
                            .primary()
                            .label("Add row")
                            .on_click(cx.listener(Self::add_row)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(DataTable::new(&self.table).stripe(true)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::format_minor;

    #[test]
    fn minor_units_format_as_major_with_two_decimals() {
        assert_eq!(format_minor(0), "0.00");
        assert_eq!(format_minor(5), "0.05");
        assert_eq!(format_minor(1_250), "12.50");
        assert_eq!(format_minor(-1_250), "-12.50");
        assert_eq!(format_minor(123_456_789), "1234567.89");
        assert_eq!(format_minor(i64::MIN), "-92233720368547758.08");
    }
}
