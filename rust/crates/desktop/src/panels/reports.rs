//! The Reports panel (U9): a report picker, the parameters each report needs, the statement set
//! out as `ReportBody` sets it out on the web screen
//! (`flowtrace/src/routes/_authenticated/reports/index.tsx`), and CSV export.
//!
//! Everything shown comes from [`crate::statements`]; this file only forwards input and draws.
//! The tables are plain rows rather than gpui-kit's `DataTable`, because a statement needs
//! group headings, rules above subtotals and a heavy rule under totals, which a data table's
//! uniform rows do not give. Components: <https://gpui-kit.com/component/input>,
//! <https://gpui-kit.com/component/select>, <https://gpui-kit.com/component/button>. The save
//! dialog is GPUI's `App::prompt_for_new_path` (the platform's file chooser, the XDG desktop
//! portal on Linux); the path field works without it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, IndexPath, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::LedgerState;
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::reports::Basis;
use ledger_core::types::{IsoDate, OrganizationId};

use crate::panels::{demo_tag, page_title, section_header};
use crate::statements::{
    Block, Cell, Footing, KeyFigure, ReportForm, ReportKind, Row, RowKind, Span, Statement, Table,
    active_preset, default_export_path, format_minor, presets, resolve_export_path, statement,
    today_utc, write_export,
};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// The grant picker's items: grant names in book order.
type GrantItems = Vec<SharedString>;

/// Width of a figure column.
const FIGURE_COLUMN: f32 = 168.0;

/// The Reports panel.
pub struct ReportsPanel {
    store: Entity<LedgerStore>,
    form: ReportForm,
    /// The organisation and books revision the form was last derived for.
    seen: Option<(OrganizationId, u64)>,
    from: Entity<InputState>,
    to: Entity<InputState>,
    path: Entity<InputState>,
    /// Whether the user typed or chose an export path; until then it follows the report.
    path_chosen: bool,
    grant_select: Entity<SelectState<GrantItems>>,
    shown: Result<Statement, String>,
    export: Option<Result<String, String>>,
    _subscriptions: Vec<Subscription>,
}

impl ReportsPanel {
    /// Builds the panel over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let from = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let to = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let path = cx.new(|cx| InputState::new(window, cx).placeholder("~/Downloads/report.csv"));
        let grant_select = cx.new(|cx| SelectState::new(GrantItems::new(), None, window, cx));

        let subscriptions = vec![
            cx.observe_in(&store, window, |panel, _, window, cx| {
                panel.sync(window, cx);
            }),
            cx.subscribe_in(&from, window, |panel, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    panel.form.from = state.read(cx).value().to_string();
                    panel.recompute(cx);
                }
            }),
            cx.subscribe_in(&to, window, |panel, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    panel.form.to = state.read(cx).value().to_string();
                    panel.recompute(cx);
                }
            }),
            cx.subscribe_in(&path, window, |panel, _, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    panel.path_chosen = true;
                    panel.export = None;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&grant_select, window, Self::on_grant_selected),
        ];

        let mut panel = Self {
            store,
            form: ReportForm {
                kind: ReportKind::default(),
                from: String::new(),
                to: String::new(),
                basis: Basis::Accrual,
                grant: None,
            },
            seen: None,
            from,
            to,
            path,
            path_chosen: false,
            grant_select,
            shown: Err(String::new()),
            export: None,
            _subscriptions: subscriptions,
        };
        panel.sync(window, cx);
        panel
    }

    /// The selected organisation's books, and whether they are the demo's.
    fn books(&self, cx: &App) -> Option<(OrganizationId, Arc<LedgerState>, u64)> {
        let view = self.store.read(cx).view();
        let org = view.selected()?.id.clone();
        let books = view.snapshot()?.clone();
        Some((org, books, view.revision()))
    }

    /// The as-at date the screen opens on: the demo's own date for the demo books (as the web
    /// app's `useLedgerAsOf` in dev mode), today otherwise.
    fn as_of(org: &OrganizationId) -> IsoDate {
        if org.as_str() == DEMO_ORG_ID {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        }
    }

    /// Follows the store: a new organisation resets the parameters to its defaults (keeping the
    /// chosen report); new books for the same organisation only recompute the statement.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((org, books, revision)) = self.books(cx) else {
            self.seen = None;
            self.shown = Err("No books loaded for this organisation.".to_owned());
            cx.notify();
            return;
        };
        let same_org = self.seen.as_ref().is_some_and(|(seen, _)| seen == &org);
        if same_org && self.seen.as_ref().is_some_and(|(_, r)| *r == revision) {
            return;
        }
        if !same_org {
            let kind = self.form.kind;
            self.form = ReportForm {
                kind,
                ..ReportForm::new(&books, &Self::as_of(&org))
            };
            self.set_dates(window, cx);
            let names: GrantItems = books
                .grants
                .iter()
                .map(|g| SharedString::from(g.name.clone()))
                .collect();
            let selected = (!names.is_empty()).then(|| IndexPath::new(0));
            self.grant_select.update(cx, |select, cx| {
                select.set_items(names, window, cx);
                select.set_selected_index(selected, window, cx);
            });
            self.export = None;
        }
        self.seen = Some((org, revision));
        self.recompute(cx);
        self.follow_path(window, cx);
    }

    /// Puts the form's dates into the date fields (which emit no change for this).
    fn set_dates(&self, window: &mut Window, cx: &mut Context<Self>) {
        let (from, to) = (self.form.from.clone(), self.form.to.clone());
        self.from
            .update(cx, |input, cx| input.set_value(from, window, cx));
        self.to
            .update(cx, |input, cx| input.set_value(to, window, cx));
    }

    /// Re-derives the statement from the form and the books.
    fn recompute(&mut self, cx: &mut Context<Self>) {
        self.shown = match self.books(cx) {
            None => Err("No books loaded for this organisation.".to_owned()),
            Some((_, books, _)) => self
                .form
                .validate(&books)
                .map_err(|e| e.to_string())
                .and_then(|params| statement(&books, &params).map_err(|e| e.to_string())),
        };
        cx.notify();
    }

    /// While the user has not chosen a path, the path field shows the default for the report.
    fn follow_path(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.path_chosen {
            return;
        }
        let Some((_, books, _)) = self.books(cx) else {
            return;
        };
        let Ok(params) = self.form.validate(&books) else {
            return;
        };
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path = default_export_path(home.as_deref(), &params);
        let text = display_path(&path, home.as_deref());
        self.path
            .update(cx, |input, cx| input.set_value(text, window, cx));
    }

    /// Shows `kind`, keeping the dates, basis and grant.
    pub fn select(&mut self, kind: ReportKind, window: &mut Window, cx: &mut Context<Self>) {
        self.form.kind = kind;
        self.export = None;
        self.recompute(cx);
        self.follow_path(window, cx);
    }

    fn set_basis(&mut self, basis: Basis, window: &mut Window, cx: &mut Context<Self>) {
        self.form.basis = basis;
        self.recompute(cx);
        self.follow_path(window, cx);
    }

    fn apply_preset(
        &mut self,
        from: &IsoDate,
        to: &IsoDate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        from.as_str().clone_into(&mut self.form.from);
        to.as_str().clone_into(&mut self.form.to);
        self.set_dates(window, cx);
        self.recompute(cx);
        self.follow_path(window, cx);
    }

    fn on_grant_selected(
        &mut self,
        _: &Entity<SelectState<GrantItems>>,
        event: &SelectEvent<GrantItems>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(Some(name)) = event else {
            return;
        };
        let Some((_, books, _)) = self.books(cx) else {
            return;
        };
        self.form.grant = books
            .grants
            .iter()
            .find(|g| g.name == name.as_ref())
            .map(|g| g.id.clone());
        self.recompute(cx);
        self.follow_path(window, cx);
    }

    /// Writes the CSV to the path in the field.
    fn export(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.export = Some(self.write(cx));
        cx.notify();
    }

    fn write(&self, cx: &App) -> Result<String, String> {
        let (_, books, _) = self
            .books(cx)
            .ok_or("No books loaded for this organisation.")?;
        let params = self.form.validate(&books).map_err(|e| e.to_string())?;
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path = resolve_export_path(&self.path.read(cx).value(), home.as_deref())
            .ok_or("Enter a file path to export to.")?;
        write_export(&books, &params, &path).map_err(|e| e.to_string())?;
        Ok(format!("Saved {}.", path.display()))
    }

    /// Opens the platform's save dialog in the path's folder; the chosen path goes into the
    /// field.
    fn choose_path(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let current = resolve_export_path(&self.path.read(cx).value(), home.as_deref());
        let dir = current
            .as_deref()
            .and_then(Path::parent)
            .filter(|dir| dir.is_dir())
            .map(Path::to_path_buf)
            .or(home)
            .unwrap_or_else(|| PathBuf::from("."));
        let name = current
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned());
        let chosen = cx.prompt_for_new_path(&dir, name.as_deref());
        cx.spawn_in(window, async move |panel, cx| {
            let outcome = chosen.await;
            let _ = panel.update_in(cx, |panel, window, cx| {
                match outcome {
                    Ok(Ok(Some(path))) => {
                        let text = path.display().to_string();
                        panel
                            .path
                            .update(cx, |input, cx| input.set_value(text, window, cx));
                        panel.path_chosen = true;
                        panel.export = None;
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(err)) => {
                        panel.export = Some(Err(format!(
                            "The save dialog is not available ({err}); type a path instead."
                        )));
                    }
                    Err(_) => {
                        panel.export = Some(Err(
                            "The save dialog closed; type a path instead.".to_owned()
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The report picker: one segment per report, the chosen one in report blue (the web
    /// screen's report nav).
    fn picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let items = ReportKind::ALL.into_iter().map(|kind| {
            let active = self.form.kind == kind;
            segment(kind.id(), kind.short(), active, p).on_click(cx.listener(
                move |panel, _: &ClickEvent, window, cx| {
                    panel.select(kind, window, cx);
                },
            ))
        });
        h_flex().child(
            h_flex()
                .flex_wrap()
                .gap(px(space::HAIRLINE))
                .bg(p.hairline.to_hsla())
                .border_1()
                .border_color(p.hairline.to_hsla())
                .children(items),
        )
    }

    /// Presets, dates, basis and grant (`PeriodControls`).
    fn controls(&self, books: Option<&LedgerState>, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let kind = self.form.kind;
        let as_of = self
            .seen
            .as_ref()
            .map_or_else(today_utc, |(org, _)| Self::as_of(org));
        let ranges = books.map(|b| presets(b, &as_of)).unwrap_or_default();
        let active_key = active_preset(&ranges, &self.form);
        let preset_row = h_flex()
            .gap(px(space::HAIRLINE))
            .bg(p.hairline.to_hsla())
            .border_1()
            .border_color(p.hairline.to_hsla())
            .children(ranges.into_iter().map(|preset| {
                let active = active_key == Some(preset.key);
                let (from, to) = (preset.from.clone(), preset.to.clone());
                segment(preset.key, &preset.label, active, p).on_click(cx.listener(
                    move |panel, _: &ClickEvent, window, cx| {
                        panel.apply_preset(&from, &to, window, cx);
                    },
                ))
            }));
        let basis_row = h_flex()
            .gap(px(space::HAIRLINE))
            .bg(p.hairline.to_hsla())
            .border_1()
            .border_color(p.hairline.to_hsla())
            .children(
                [
                    (Basis::Accrual, "accrual", "Accrual"),
                    (Basis::Cash, "cash", "Cash"),
                ]
                .map(|(basis, id, label)| {
                    segment(id, label, self.form.basis == basis, p).on_click(cx.listener(
                        move |panel, _: &ClickEvent, window, cx| {
                            panel.set_basis(basis, window, cx);
                        },
                    ))
                }),
            );
        let to_label = match kind.span() {
            Span::Point => "As at",
            Span::Range => "To",
        };

        v_flex()
            .border_t(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                h_flex()
                    .flex_wrap()
                    .items_start()
                    .gap(px(space::CELL))
                    .py(px(space::CELL))
                    .child(field("Period", preset_row))
                    .when(kind.span() == Span::Range, |row| {
                        row.child(field(
                            "From",
                            div().w(px(140.0)).child(Input::new(&self.from)),
                        ))
                    })
                    .child(field(
                        to_label,
                        div().w(px(140.0)).child(Input::new(&self.to)),
                    ))
                    .when(kind.has_basis(), |row| row.child(field("Basis", basis_row)))
                    .when(kind == ReportKind::Grant, |row| {
                        row.child(field(
                            "Grant",
                            div().w(px(300.0)).child(
                                Select::new(&self.grant_select).placeholder("Choose a grant"),
                            ),
                        ))
                    }),
            )
            .when(kind.has_basis() && self.form.basis == Basis::Cash, |col| {
                col.child(
                    div()
                        .border_t(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .py(px(8.0))
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(
                            "Cash basis leaves out unpaid bills and claims, and counts each \
                             payment against the expenses it settled.",
                        ),
                )
            })
    }

    /// The export row: the path field, the save dialog, the export button and the outcome.
    fn export_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let can_export = self.shown.is_ok();
        v_flex().child(section_header("Export", div(), cx)).child(
            v_flex()
                .py(px(space::CELL))
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(
                            "The statement as a spreadsheet, amounts in major units, with \
                                 the same columns as the table.",
                        ),
                )
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(240.0))
                                .child(Input::new(&self.path)),
                        )
                        .child(
                            Button::new("choose-path")
                                .outline()
                                .label("Choose...")
                                .on_click(cx.listener(Self::choose_path)),
                        )
                        .child(
                            Button::new("export-csv")
                                .primary()
                                .label("Export CSV")
                                .disabled(!can_export)
                                .on_click(cx.listener(Self::export)),
                        ),
                )
                .when_some(self.export.clone(), |col, outcome| {
                    let (colour, message) = match outcome {
                        Ok(m) => (p.report_blue, m),
                        Err(m) => (p.rejection_red, m),
                    };
                    col.child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colour.to_hsla())
                            .child(message),
                    )
                }),
        )
    }
}

impl Render for ReportsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let books = self.books(cx);
        let demo = books
            .as_ref()
            .is_some_and(|(org, _, _)| org.as_str() == DEMO_ORG_ID);
        let org_name = self
            .store
            .read(cx)
            .view()
            .selected()
            .map(|org| org.name.clone());
        let body: AnyElement = match &self.shown {
            Ok(s) => statement_body(s, org_name.as_deref(), p).into_any_element(),
            Err(message) => div()
                .py(px(24.0))
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.rejection_red.to_hsla())
                .child(message.clone())
                .into_any_element(),
        };
        let title = self
            .shown
            .as_ref()
            .map_or_else(|_| self.form.kind.title().to_owned(), |s| s.title.clone());
        let meta = h_flex()
            .gap(px(8.0))
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .when_some(self.shown.as_ref().ok(), |row, s| {
                row.child(s.period.clone())
            })
            .when(demo, |row| row.child(demo_tag(cx)));

        v_flex()
            .gap(px(space::SECTION))
            .pb(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(12.0))
                    .child(page_title("Financial reports"))
                    .child(
                        div()
                            .text_size(px(text::LEDE))
                            .text_color(p.ink_muted.to_hsla())
                            .child(
                                "The statements a board, auditor or donor expects, read from \
                                 posted journal lines. Each one shows the check that proves it \
                                 adds up.",
                            ),
                    ),
            )
            .child(self.picker(cx))
            .child(self.controls(books.as_ref().map(|(_, b, _)| b.as_ref()), cx))
            .child(v_flex().child(section_header(&title, meta, cx)).child(body))
            .child(self.export_section(cx))
    }
}

/// `~/...` for a path under `home`, as typed in the field.
fn display_path(path: &Path, home: Option<&Path>) -> String {
    home.and_then(|home| path.strip_prefix(home).ok())
        .map_or_else(
            || path.display().to_string(),
            |rest| format!("~/{}", rest.display()),
        )
}

/// A labelled control (`fieldset` with a `legend` on the web screen).
fn field(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    v_flex()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
        .child(control)
}

/// One segment of a segmented control: report blue when chosen, muted with a wash on hover
/// otherwise.
fn segment(
    id: &'static str,
    label: &str,
    active: bool,
    p: &Palette,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(SharedString::from(id))
        .h(px(space::BUTTON))
        .px(px(12.0))
        .flex()
        .items_center()
        .text_size(px(text::BODY))
        .cursor_pointer()
        .map(|seg| {
            if active {
                seg.bg(p.report_blue.to_hsla())
                    .text_color(p.report_blue_ink.to_hsla())
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                seg.bg(p.report_paper.to_hsla())
                    .text_color(p.ink_muted.to_hsla())
                    .hover(|seg| seg.bg(p.wash.to_hsla()).text_color(p.ink_navy.to_hsla()))
            }
        })
        .child(label.to_owned())
}

/// The title block and the blocks of a statement (`ReportBody`, `StatementHead`).
fn statement_body(s: &Statement, org: Option<&str>, p: &Palette) -> impl IntoElement {
    let head = v_flex()
        .py(px(space::CELL))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .when_some(org, |head, org| {
            head.child(
                div()
                    .text_size(px(text::BODY))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(org.to_owned()),
            )
        })
        .child(div().text_size(px(text::TITLE)).child(s.title.clone()))
        .child(
            div()
                .pt(px(4.0))
                .text_size(px(text::BODY))
                .text_color(p.ink_muted.to_hsla())
                .child(format!(
                    "{} \u{b7} Amounts in {} \u{b7} {}",
                    s.period, s.currency, s.basis
                )),
        );
    v_flex()
        .child(head)
        .children(s.blocks.iter().map(|block| match block {
            Block::Figures(figures) => key_figures(figures, p).into_any_element(),
            Block::Table(table) => statement_table(table, p).into_any_element(),
            Block::Footing(footing) => footing_check(footing, p).into_any_element(),
        }))
}

/// A row of key figures (`KeyFigures`).
fn key_figures(figures: &[KeyFigure], p: &Palette) -> impl IntoElement {
    h_flex()
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .children(figures.iter().enumerate().map(|(i, f)| {
            v_flex()
                .flex_1()
                .p(px(space::CELL))
                .when(i > 0, |cell| {
                    cell.border_l(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                })
                .child(
                    div()
                        .text_size(px(if i == 0 { text::FIGURE } else { text::TITLE }))
                        .line_height(px(if i == 0 { text::FIGURE } else { text::TITLE }))
                        .font_weight(FontWeight::BOLD)
                        .child(f.value.clone()),
                )
                .child(
                    div()
                        .pt(px(6.0))
                        .text_size(px(text::LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink_muted.to_hsla())
                        .child(f.label.to_uppercase()),
                )
        }))
}

/// A statement table (`StatementTable`, `GroupHeading`, `AmountRow`, `GapRow`).
fn statement_table(table: &Table, p: &Palette) -> impl IntoElement {
    let title = table.title.clone().map(|title| {
        div()
            .mt(px(space::SECTION))
            .pb(px(8.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(text::SECTION_LABEL))
            .font_weight(FontWeight::BOLD)
            .child(title.to_uppercase())
    });
    let body = if let Some(empty) = &table.empty {
        div()
            .py(px(24.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(text::BODY))
            .text_color(p.ink_muted.to_hsla())
            .child(empty.clone())
            .into_any_element()
    } else {
        let head = h_flex()
            .py(px(10.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.ink_navy.to_hsla())
            .text_size(px(text::LABEL))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(div().flex_1().child(table.first_column.clone()))
            .children(table.columns.iter().map(|c| figure_cell(c.clone())));
        v_flex()
            .child(head)
            .children(table.rows.iter().map(|row| statement_row(row, p)))
            .into_any_element()
    };
    v_flex().children(title).child(body)
}

/// A right-aligned figure column cell.
fn figure_cell(content: String) -> impl IntoElement {
    div()
        .w(px(FIGURE_COLUMN))
        .flex_shrink_0()
        .pl(px(space::CELL))
        .flex()
        .justify_end()
        .whitespace_nowrap()
        .child(content)
}

/// One statement row: hairline under lines, an ink rule over subtotals, a heavy rule closing
/// totals.
fn statement_row(row: &Row, p: &Palette) -> AnyElement {
    let ink = p.ink_navy.to_hsla();
    match row.kind {
        RowKind::Gap => div().h(px(12.0)).into_any_element(),
        RowKind::Heading => div()
            .pt(px(24.0))
            .pb(px(6.0))
            .text_size(px(text::SECTION_LABEL))
            .font_weight(FontWeight::BOLD)
            .child(row.label.to_uppercase())
            .into_any_element(),
        kind => {
            let label = v_flex()
                .flex_1()
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .when_some(row.code.clone(), |line, code| {
                            line.child(
                                div()
                                    .text_size(px(text::LABEL))
                                    .font_family(text::MONO_FAMILY)
                                    .text_color(p.ink_muted.to_hsla())
                                    .font_weight(FontWeight::NORMAL)
                                    .child(code),
                            )
                        })
                        .child(row.label.clone()),
                )
                .when_some(row.detail.clone(), |col, detail| {
                    col.child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(p.ink_muted.to_hsla())
                            .child(detail),
                    )
                });
            h_flex()
                .py(px(8.0))
                .items_start()
                .map(|line| match kind {
                    RowKind::Subtotal => line
                        .border_t(px(space::HAIRLINE))
                        .border_color(ink)
                        .font_weight(FontWeight::SEMIBOLD),
                    RowKind::Total => line
                        .border_t(px(space::HAIRLINE))
                        .border_b(px(space::RULE_HEAVY))
                        .border_color(ink)
                        .font_weight(FontWeight::BOLD),
                    _ => line
                        .border_b(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla()),
                })
                .child(label)
                .children(row.cells.iter().map(|cell| figure_cell(cell_text(cell))))
                .into_any_element()
        }
    }
}

fn cell_text(cell: &Cell) -> String {
    cell.display()
}

/// The footing check (`FootingCheck`): a report-blue mark when it foots, a signal-orange one
/// when it does not, and the difference.
fn footing_check(f: &Footing, p: &Palette) -> impl IntoElement {
    let ok = f.foots();
    let mark = if ok { p.report_blue } else { p.signal_orange };
    let difference = if ok {
        "0.00".to_owned()
    } else {
        format_minor(f.difference)
    };
    h_flex()
        .py(px(12.0))
        .gap(px(space::CELL))
        .justify_between()
        .items_start()
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::BODY))
        .child(
            h_flex()
                .items_start()
                .gap(px(12.0))
                .child(div().mt(px(6.0)).w(px(22.0)).h(px(3.0)).bg(mark.to_hsla()))
                .child(
                    v_flex()
                        .child(
                            h_flex()
                                .gap(px(6.0))
                                .flex_wrap()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .when(!ok, |d| d.text_color(p.signal_orange.to_hsla()))
                                        .child(if ok { "Foots." } else { "Does not foot." }),
                                )
                                .child(f.statement.clone()),
                        )
                        .when_some(f.detail.clone(), |col, detail| {
                            col.child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(detail),
                            )
                        }),
                ),
        )
        .child(
            h_flex()
                .flex_shrink_0()
                .gap(px(6.0))
                .child(div().text_color(p.ink_muted.to_hsla()).child("Difference"))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(difference)),
        )
}
