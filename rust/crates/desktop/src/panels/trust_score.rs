//! The Trust Score screen (DT4): the score and its grade, how it is made, its history, how to
//! raise it and the ledger signals, as the web's `routes/_authenticated/trust-score.tsx` lays
//! them out. Everything shown comes from [`crate::trust`]; this file only draws.
//!
//! "Recalculate score" sends `recompute_trust` ([`SettingsWrite::RecomputeTrust`]). The
//! history chart is a canvas: grid lines at the ticks, the score line 2.5 px wide, square dots
//! with the last one enlarged and its score printed beside it.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Bounds, ClickEvent, Context, Div, Entity, EventEmitter, FontWeight, Hsla,
    IntoElement, ParentElement, PathBuilder, Pixels, Render, SharedString, Styled as _,
    Subscription, Window, canvas, div, fill, point, px, relative, size,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{IsoDate, OrganizationId};

use crate::backend::SettingsWrite;
use crate::dashboard::Bar;
use crate::model::NavEntry;
use crate::panels::dashboard::{bar, notice, small_muted, status_mark};
use crate::panels::transactions::TransactionsView;
use crate::panels::{demo_tag, page_title, section_header, write_settings};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};
use crate::transactions::Filter;
use crate::trust::{
    CHANGE_NOT_YET, ComponentRow, HistoryChart, LEDE, NO_HISTORY, NOT_CALCULATED,
    NOTHING_OUTSTANDING, RaiseAction, RaiseLink, SIGNALS_LEDE, ScoreChange, Signal, TrustPage,
    recalc_error, signals,
};

/// Emitted by a "How to raise it" link: the shell opens Transactions on this view and filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTransactions(pub TransactionsView, pub Option<Filter>);

/// Emitted by a "Ledger signals" link: the shell shows that screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen(pub NavEntry);

/// The plot's height (`h-52 sm:h-60`).
const PLOT_HEIGHT: f32 = 240.0;

/// The Trust Score screen.
pub struct TrustScorePanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64, u64)>,
    org_name: String,
    demo: bool,
    page: Option<TrustPage>,
    signals: Vec<Signal>,
    recalculating: bool,
    recalc_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenTransactions> for TrustScorePanel {}
impl EventEmitter<OpenScreen> for TrustScorePanel {}

impl TrustScorePanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&store, |this, _, cx| this.refresh(cx))];
        let mut panel = Self {
            store,
            seen: None,
            org_name: String::new(),
            demo: false,
            page: None,
            signals: Vec::new(),
            recalculating: false,
            recalc_error: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(cx);
        panel
    }

    /// Re-derives when the organisation, its records or its books changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision(), view.revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        if self.seen.as_ref().is_some_and(|s| s.0 != org) {
            self.recalc_error = None;
        }
        self.seen = Some(key);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        let records = view.transparency().cloned();
        self.org_name = records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.name.clone()))
            .or_else(|| view.selected().map(|o| o.name.clone()))
            .unwrap_or_default();
        self.page = records.as_deref().map(TrustPage::of);
        // The demo is a snapshot as of its latest movement; other books run to today
        // (`useLedgerAsOf`, flowtrace/src/lib/ledger-client.ts:45-47).
        let as_of = if self.demo {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        };
        self.signals = view
            .snapshot()
            .map(|books| signals(books, &as_of))
            .unwrap_or_default();
        cx.notify();
    }

    /// "Recalculate score": `recompute_trust`, which appends a history point.
    fn recalculate(&mut self, cx: &mut Context<Self>) {
        if self.recalculating {
            return;
        }
        self.recalculating = true;
        self.recalc_error = None;
        let store = self.store.clone();
        write_settings(
            self,
            &store,
            SettingsWrite::RecomputeTrust,
            cx,
            |this, result, cx| {
                this.recalculating = false;
                if let Err(e) = result {
                    this.recalc_error = Some(recalc_error(&e.to_string()));
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    /// Keys while the screen shows: `r` recalculates.
    pub fn press_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        if key == "r" && self.page.is_some() {
            self.recalculate(cx);
            return true;
        }
        false
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let calculated = self.page.as_ref().and_then(|pg| pg.calculated.clone());
        let label = if self.recalculating {
            "Recalculating\u{2026}"
        } else {
            "Recalculate score"
        };
        let button = Button::new("recalculate-score")
            .outline()
            .label(label)
            .disabled(self.recalculating)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.recalculate(cx)));
        v_flex()
            .gap(px(8.0))
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .child(page_title("Trust Score"))
                    .when(self.page.is_some(), |row| row.child(button)),
            )
            .child(
                h_flex()
                    .gap(px(12.0))
                    .text_size(px(text::BODY))
                    .text_color(p.ink_muted.to_hsla())
                    .when(!self.org_name.is_empty(), |m| {
                        m.child(self.org_name.clone())
                    })
                    .when_some(calculated, ParentElement::child)
                    .when(self.demo, |m| m.child(demo_tag(cx))),
            )
            .child(small_muted(LEDE, p).text_size(px(text::BODY)))
            .when_some(self.recalc_error.clone(), |h, e| {
                h.child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(p.rejection_red.to_hsla())
                        .child(e),
                )
            })
    }

    fn score_section(pg: &TrustPage, cx: &Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let change = match &pg.change {
            ScoreChange::NotYet => small_muted(CHANGE_NOT_YET, p)
                .py(px(12.0))
                .into_any_element(),
            ScoreChange::Rows {
                since_previous,
                since_first,
                last,
            } => v_flex()
                .child(change_row(&since_previous.0, &since_previous.1, p))
                .child(change_row(&since_first.0, &since_first.1, p))
                .child(change_row("Last calculated", last, p))
                .into_any_element(),
        };
        v_flex()
            .child(section_header("Score", div(), cx))
            .child(
                v_flex()
                    .py(px(20.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        h_flex()
                            .items_baseline()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(96.0))
                                    .line_height(px(80.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(pg.score.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(text::TITLE))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child("/ 100"),
                            ),
                    )
                    .child(
                        h_flex().pt(px(16.0)).gap(px(4.0)).child("Grade").child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(pg.grade.clone()),
                        ),
                    ),
            )
            .child(change)
    }

    fn components_section(pg: &TrustPage, cx: &Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let body = if pg.components.is_empty() {
            notice(NOT_CALCULATED.0, NOT_CALCULATED.1, p).into_any_element()
        } else {
            v_flex()
                .children(pg.components.iter().map(|c| component_row(c, p)))
                .into_any_element()
        };
        v_flex()
            .child(section_header(
                "How it is made",
                small_muted("weighted", p),
                cx,
            ))
            .child(body)
    }

    fn history_section(pg: &TrustPage, cx: &Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let meta = pg
            .history_meta
            .clone()
            .map_or_else(div, |m| small_muted(&m, p));
        let body = pg.chart.as_ref().map_or_else(
            || notice(NO_HISTORY.0, NO_HISTORY.1, p).into_any_element(),
            |c| history_chart(c, p).into_any_element(),
        );
        v_flex()
            .child(section_header("History", meta, cx))
            .child(body)
    }

    fn raise_section(pg: &TrustPage, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let body = if pg.actions.is_empty() {
            notice(NOTHING_OUTSTANDING.0, NOTHING_OUTSTANDING.1, p).into_any_element()
        } else {
            let rows: Vec<AnyElement> = pg
                .actions
                .iter()
                .enumerate()
                .map(|(i, a)| raise_row(i, a, p, cx))
                .collect();
            v_flex().children(rows).into_any_element()
        };
        v_flex()
            .child(section_header("How to raise it", div(), cx))
            .child(body)
    }

    fn signals_section(&self, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let targets = [NavEntry::Reconcile, NavEntry::Grants, NavEntry::Grants];
        let cells: Vec<AnyElement> = self
            .signals
            .iter()
            .zip(targets)
            .enumerate()
            .map(|(i, (s, target))| {
                v_flex()
                    .flex_1()
                    .p(px(space::CELL))
                    .when(i > 0, |cell| {
                        cell.border_l(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                    })
                    .child(
                        div()
                            .text_size(px(text::TITLE))
                            .font_weight(FontWeight::BOLD)
                            .child(s.value.clone()),
                    )
                    .child(
                        div()
                            .pt(px(6.0))
                            .text_size(px(text::LABEL))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink_muted.to_hsla())
                            .child(s.label.to_uppercase()),
                    )
                    .child(small_muted(s.detail, p).pt(px(4.0)))
                    .child(
                        Button::new(SharedString::from(format!("signal-{i}")))
                            .link()
                            .label(s.link)
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                cx.emit(OpenScreen(target));
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        v_flex()
            .child(section_header(
                "Ledger signals",
                h_flex()
                    .gap(px(8.0))
                    .when(self.demo, |m| m.child(demo_tag(cx))),
                cx,
            ))
            .child(
                small_muted(SIGNALS_LEDE, p)
                    .py(px(12.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla()),
            )
            .child(
                h_flex()
                    .items_start()
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .children(cells),
            )
    }
}

impl Render for TrustScorePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let header = self.header(cx).into_any_element();
        let Some(pg) = self.page.clone() else {
            return v_flex()
                .gap(px(space::SECTION))
                .child(header)
                .child(
                    v_flex().child(section_header("Score", div(), cx)).child(notice(
                        "No organization selected",
                        "Your Trust Score appears once your account belongs to an organization. Ask an admin to add you, then reload this page.",
                        p,
                    )),
                )
                .into_any_element();
        };
        let signals = (!self.signals.is_empty()).then(|| self.signals_section(cx));
        v_flex()
            .gap(px(space::SECTION))
            .child(header)
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        Self::score_section(&pg, cx)
                            .flex_basis(relative(4.0 / 12.0))
                            .min_w(px(0.0)),
                    )
                    .child(
                        Self::components_section(&pg, cx)
                            .flex_basis(relative(8.0 / 12.0))
                            .min_w(px(0.0)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        Self::history_section(&pg, cx)
                            .flex_basis(relative(7.0 / 12.0))
                            .min_w(px(0.0)),
                    )
                    .child(
                        Self::raise_section(&pg, cx)
                            .flex_basis(relative(5.0 / 12.0))
                            .min_w(px(0.0)),
                    ),
            )
            .when_some(signals, ParentElement::child)
            .into_any_element()
    }
}

fn change_row(label: &str, value: &str, p: &Palette) -> Div {
    h_flex()
        .justify_between()
        .py(px(10.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::BODY))
        .child(
            div()
                .text_color(p.ink_muted.to_hsla())
                .child(label.to_owned()),
        )
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(value.to_owned()),
        )
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a 0 to 100 fill as bar units"
)]
fn component_row(c: &ComponentRow, p: &Palette) -> Div {
    let value = c.value.clone().map_or_else(
        || small_muted("Not yet scored", p).text_size(px(text::BODY)),
        |v| {
            h_flex()
                .items_baseline()
                .child(
                    div()
                        .text_size(px(text::TITLE))
                        .font_weight(FontWeight::BOLD)
                        .child(v),
                )
                .child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child("\u{a0}/ 100"),
                )
        },
    );
    let filled = (c.filled * 100.0).round() as i64;
    v_flex()
        .py(px(16.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            h_flex()
                .justify_between()
                .items_baseline()
                .gap(px(16.0))
                .child(
                    h_flex()
                        .gap(px(6.0))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(c.label))
                        .child(
                            div()
                                .text_color(p.ink_muted.to_hsla())
                                .child(c.weight.clone()),
                        ),
                )
                .child(value),
        )
        .child(div().mt(px(8.0)).child(bar(
            Bar {
                total: 10_000,
                filled,
                evidenced: Some(filled),
                pending: false,
            },
            8.0,
            p,
        )))
        .child(small_muted(&c.explain, p).mt(px(6.0)))
}

fn raise_row(
    i: usize,
    a: &RaiseAction,
    p: &Palette,
    cx: &mut Context<TrustScorePanel>,
) -> AnyElement {
    let link = a.link.clone().map(|(label, to)| {
        let event = match to {
            RaiseLink::Transactions(status) => OpenTransactions(
                TransactionsView::List,
                Filter::from_alias(match status {
                    ledger_module::transparency::flows::TransactionStatus::Flagged => "flagged",
                    _ => "pending",
                }),
            ),
            RaiseLink::Transaction(id) => OpenTransactions(TransactionsView::Detail(id), None),
        };
        Button::new(SharedString::from(format!("raise-{i}")))
            .link()
            .label(label)
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(event.clone())))
    });
    h_flex()
        .items_start()
        .gap(px(12.0))
        .py(px(16.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(div().pt(px(4.0)).child(status_mark(a.mark, "", p)))
        .child(
            v_flex()
                .min_w(px(0.0))
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(a.title.clone()),
                )
                .child(small_muted(&a.detail, p).text_size(px(text::BODY)))
                .when_some(link, ParentElement::child),
        )
        .into_any_element()
}

/// The History chart (`HistoryChart`, `trust-score.tsx:361-495`): y labels on the left, the
/// canvas, the latest score by its dot, the month labels under it.
#[expect(clippy::too_many_lines, reason = "the web's chart, read top to bottom")]
fn history_chart(c: &HistoryChart, p: &Palette) -> Div {
    let line: Hsla = p.report_blue.to_hsla();
    let grid: Hsla = p.hairline.to_hsla();
    let paper: Hsla = p.report_paper.to_hsla();
    let ticks: Vec<f64> = c.ticks.iter().map(|t| t.1).collect();
    let points = c.points.clone();
    let pct = |v: f64| {
        #[expect(clippy::cast_possible_truncation, reason = "a percentage of a box")]
        let f = (v / 100.0) as f32;
        f
    };
    let (last_x, last_y) = c.points.last().copied().unwrap_or((50.0, 50.0));
    let plot = div()
        .relative()
        .flex_1()
        .h(px(PLOT_HEIGHT))
        .child(
            canvas(
                move |bounds: Bounds<Pixels>, _, _| bounds,
                move |_, bounds, window, _| {
                    let width = f32::from(bounds.size.width);
                    let height = f32::from(bounds.size.height);
                    let at = |x: f64, y: f64| {
                        point(
                            bounds.origin.x + px(width * pct(x)),
                            bounds.origin.y + px(height * pct(y)),
                        )
                    };
                    for y in &ticks {
                        let mut b = PathBuilder::stroke(px(1.0));
                        b.move_to(at(0.0, *y));
                        b.line_to(at(100.0, *y));
                        if let Ok(path) = b.build() {
                            window.paint_path(path, grid);
                        }
                    }
                    if points.len() > 1 {
                        let mut b = PathBuilder::stroke(px(2.5));
                        for (i, (x, y)) in points.iter().enumerate() {
                            if i == 0 {
                                b.move_to(at(*x, *y));
                            } else {
                                b.line_to(at(*x, *y));
                            }
                        }
                        if let Ok(path) = b.build() {
                            window.paint_path(path, line);
                        }
                    }
                    let n = points.len();
                    for (i, (x, y)) in points.iter().enumerate() {
                        let centre = at(*x, *y);
                        if i + 1 == n {
                            // `size-3 ring-2 ring-background`: a 12 px square in a 2 px ring.
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(centre.x - px(8.0), centre.y - px(8.0)),
                                    size(px(16.0), px(16.0)),
                                ),
                                paper,
                            ));
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(centre.x - px(6.0), centre.y - px(6.0)),
                                    size(px(12.0), px(12.0)),
                                ),
                                line,
                            ));
                        } else {
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(centre.x - px(3.0), centre.y - px(3.0)),
                                    size(px(6.0), px(6.0)),
                                ),
                                line,
                            ));
                        }
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .child(
            // The latest score, up and to the left of its dot.
            div()
                .absolute()
                .top(relative(pct(last_y)))
                .left(relative(pct(last_x)))
                .mt(px(-34.0))
                .ml(px(-48.0))
                .w(px(44.0))
                .flex()
                .justify_end()
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD)
                .child(c.last_score.to_string()),
        );
    let y_labels = div()
        .relative()
        .w(px(28.0))
        .h(px(PLOT_HEIGHT))
        .flex_shrink_0()
        .children(c.ticks.iter().map(|(t, y)| {
            div()
                .absolute()
                .right_0()
                .top(relative(pct(*y)))
                .mt(px(-7.0))
                .text_size(px(11.0))
                .text_color(p.ink_muted.to_hsla())
                .child(t.to_string())
        }));
    let n = c.labels.len();
    let x_labels = div()
        .relative()
        .ml(px(36.0))
        .mt(px(8.0))
        .h(px(16.0))
        .children(c.labels.iter().enumerate().map(|(i, (x, label))| {
            let last = i + 1 == n && (*x - 100.0).abs() < 1e-9;
            div()
                .absolute()
                .top_0()
                .left(relative(pct(*x)))
                .when(*x > 0.0 && !last, |d| d.ml(px(-20.0)))
                .when(last, |d| d.ml(px(-40.0)).font_weight(FontWeight::SEMIBOLD))
                .text_size(px(11.0))
                .text_color(if last {
                    p.ink_navy.to_hsla()
                } else {
                    p.ink_muted.to_hsla()
                })
                .whitespace_nowrap()
                .child(label.clone())
        }));
    v_flex()
        .pt(px(16.0))
        .child(h_flex().gap(px(8.0)).child(y_labels).child(plot))
        .child(x_labels)
}
