//! The Flows screen (DT2): the web's "Money flow" page (`routes/_authenticated/flows.tsx`) with
//! its map (`components/flows/flow-map.tsx`) and details panel (`flow-details.tsx`). Everything
//! shown comes from [`crate::flows`]; this file draws. The lines are painted on a
//! `gpui_kit::canvas` with `PathBuilder` cubics (`cubic_bezier_to`, `dash_array`) and
//! `window.paint_path`; the nodes, labels and amount tag are positioned divs over it
//! (<https://docs.rs/gpui/latest/gpui/fn.canvas.html>, `PathBuilder` in gpui-pre 0.3.9
//! `path_builder.rs`). A click picks the node under it or the nearest line within its hit area
//! ([`FlowGraph::hit`]).
//!
//! Keys (the shell forwards them while the screen shows): Tab or Right moves to the next node or
//! line in money order, Left back, Esc clears; W, M, Q and Y pick the period; `-` and `=` zoom,
//! F fits; L opens the ledger lines of what is selected. The web's "Record a transaction" button
//! waits for the Transactions screen (DT3).

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Bounds, ClickEvent, Context, Div, Entity, EventEmitter, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, ParentElement as _,
    PathBuilder, Pixels, Point, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, canvas, div, point, px,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::flows::TransactionStatus;

use crate::dashboard::{Figure, proof_label};
use crate::flows::{
    Body, CLEAR_LABEL, Connected, Details, EMPTY_BODY, EMPTY_TITLE, FlowGraph, FlowPage, HELP,
    LEGEND_NOTE, LOADING_BODY, LOADING_TITLE, LedgerLink, MapNodeType, NAME_SIZE, NODE_PAD, NODE_W,
    PERIODS, PeriodId, Selection, WIDEST_PERIOD, ZOOM_MAX, ZOOM_MIN, Zoom, connected_set,
    cross_size, edge_dash, meta_baseline, name_baseline, node_amount,
};
use crate::panels::dashboard::{notice, proof_legend, small_muted, status_mark};
use crate::panels::{demo_tag, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};
use crate::web_format::format_money;

/// Emitted by a ledger link: the shell opens the General ledger on these lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenLedgerLines(pub LedgerLink);

/// `DIM` (`flow-map.tsx:15`): what is outside the selection fades to.
const DIM: f32 = 0.12;
/// The meta and label size in map units (`text-[13.5px]`).
const META_SIZE: f64 = 13.5;
/// Where a line of text starts above its baseline, as a share of its size.
const ASCENT: f64 = 0.8;
/// The details panel's width (`lg:grid-cols-[minmax(0,1fr)_19rem]`).
const DETAILS_W: f32 = 304.0;

/// The Flows screen.
pub struct FlowsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64, PeriodId)>,
    period: PeriodId,
    page: Option<FlowPage>,
    selection: Option<Selection>,
    active: Option<Connected>,
    zoom: Zoom,
    demo: bool,
    /// The map container's width, measured at paint.
    map_width: Rc<Cell<f32>>,
    /// The map's top left corner in the window at the last paint, for clicks.
    map_origin: Rc<Cell<Option<Point<Pixels>>>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenLedgerLines> for FlowsPanel {}

impl FlowsPanel {
    /// The screen over `store`, on the month (`flows.tsx:47`).
    pub fn new(store: Entity<LedgerStore>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&store, |this, _, cx| this.refresh(cx))];
        let mut panel = Self {
            store,
            seen: None,
            period: PeriodId::Month,
            page: None,
            selection: None,
            active: None,
            zoom: Zoom::default(),
            demo: false,
            map_width: Rc::new(Cell::new(0.0)),
            map_origin: Rc::new(Cell::new(None)),
            _subscriptions: subscriptions,
        };
        panel.refresh(cx);
        panel
    }

    /// Re-derives the page when the organisation, its records or the period changed; drops a
    /// selection the new map no longer holds (`flows.tsx:59-65`).
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision(), self.period);
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        self.page = view.transparency().map(|t| FlowPage::of(t, self.period));
        let kept = self
            .selection
            .take()
            .filter(|s| self.page.as_ref().is_some_and(|p| p.graph.contains(s)));
        self.select(kept, cx);
    }

    /// Opens the map on a period and, when given, the `n`th node or line in money order
    /// (`--flows <period>[:<n>]`, for screenshots).
    pub fn launch(&mut self, value: &str, cx: &mut Context<Self>) {
        let (period, nth) = value.split_once(':').unwrap_or((value, ""));
        if let Some(p) = PERIODS
            .iter()
            .find(|p| p.label.eq_ignore_ascii_case(period))
        {
            self.set_period(p.id, cx);
        }
        if let Ok(n) = nth.parse::<usize>() {
            let target = self
                .page
                .as_ref()
                .and_then(|p| p.graph.focus_order().get(n).cloned());
            self.select(target, cx);
        }
    }

    fn set_period(&mut self, period: PeriodId, cx: &mut Context<Self>) {
        self.period = period;
        self.refresh(cx);
        cx.notify();
    }

    fn select(&mut self, selection: Option<Selection>, cx: &mut Context<Self>) {
        self.active = self
            .page
            .as_ref()
            .and_then(|p| connected_set(&p.graph, selection.as_ref()));
        self.selection = selection;
        cx.notify();
    }

    /// Moves the selection along the money order (`flow-map.tsx:89-114`, Tab order).
    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(page) = &self.page else {
            return;
        };
        let order = page.graph.focus_order();
        if order.is_empty() {
            return;
        }
        let at = self
            .selection
            .as_ref()
            .and_then(|s| order.iter().position(|o| o == s));
        let next = match (at, forward) {
            (None, true) => 0,
            (None, false) => order.len() - 1,
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
        };
        self.select(order.get(next).cloned(), cx);
    }

    fn scale(&self) -> f64 {
        let width = self.page.as_ref().map_or(0.0, |p| p.graph.width);
        self.zoom.scale(f64::from(self.map_width.get()), width)
    }

    fn zoom_by(&mut self, up: bool, cx: &mut Context<Self>) {
        self.zoom = Zoom::step(self.scale(), up);
        cx.notify();
    }

    /// The keys of the module docs. Returns whether the key was used.
    pub fn press_key(&mut self, key: &str, _: &mut Window, cx: &mut Context<Self>) -> bool {
        let map = self.page.as_ref().is_some_and(|p| p.body() == Body::Map);
        let has = self.page.as_ref().is_some_and(FlowPage::has_movements);
        match key {
            "w" | "m" | "q" | "y" if has => {
                let period = match key {
                    "w" => PeriodId::Week,
                    "m" => PeriodId::Month,
                    "q" => PeriodId::Quarter,
                    _ => PeriodId::Year,
                };
                self.set_period(period, cx);
            }
            "tab" | "right" if map => self.step(true, cx),
            "left" if map => self.step(false, cx),
            "escape" if self.selection.is_some() => self.select(None, cx),
            "-" if map => self.zoom_by(false, cx),
            "=" | "+" if map => self.zoom_by(true, cx),
            "f" if map => {
                self.zoom = Zoom::Fit;
                cx.notify();
            }
            "l" => {
                let link = self
                    .page
                    .as_ref()
                    .zip(self.selection.as_ref())
                    .and_then(|(p, s)| Details::of(&p.graph, s))
                    .and_then(|d| d.ledger.map(|(_, link)| link));
                let Some(link) = link else {
                    return false;
                };
                cx.emit(OpenLedgerLines(link));
            }
            _ => return false,
        }
        true
    }

    fn on_map_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(origin), Some(page)) = (self.map_origin.get(), &self.page) else {
            return;
        };
        let z = self.scale();
        let x = f64::from(f32::from(event.position.x - origin.x)) / z;
        let y = f64::from(f32::from(event.position.y - origin.y)) / z;
        let hit = page.graph.hit(x, y);
        self.select(hit, cx);
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let lede = self
            .page
            .as_ref()
            .map_or_else(|| crate::flows::LEDE_NONE.to_owned(), FlowPage::lede);
        let control = self
            .page
            .as_ref()
            .is_some_and(FlowPage::has_movements)
            .then(|| self.period_control(cx));
        h_flex()
            .items_end()
            .justify_between()
            .gap(px(16.0))
            .child(
                v_flex()
                    .min_w(px(0.0))
                    .gap(px(8.0))
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .child(page_title("Money flow"))
                            .when(self.demo, |row| row.child(demo_tag(cx))),
                    )
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(lede),
                    ),
            )
            .children(control)
    }

    /// The Period control (`flows.tsx:115-130`): one pressed button at a time.
    fn period_control(&self, cx: &mut Context<Self>) -> Div {
        h_flex().children(PERIODS.iter().map(|period| {
            let id = period.id;
            let button = Button::new(SharedString::from(format!("period-{}", period.label)))
                .label(period.label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_period(id, cx)));
            if id == self.period {
                button.primary()
            } else {
                button.outline()
            }
        }))
    }

    fn zoom_actions(&self, cx: &mut Context<Self>) -> Div {
        let scale = self.scale();
        h_flex()
            .gap(px(4.0))
            .child(
                Button::new("zoom-out")
                    .ghost()
                    .label("Zoom out")
                    .disabled(scale <= ZOOM_MIN)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.zoom_by(false, cx))),
            )
            .child(
                div()
                    .w(px(44.0))
                    .flex()
                    .justify_center()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Zoom::percent(scale)),
            )
            .child(
                Button::new("zoom-in")
                    .ghost()
                    .label("Zoom in")
                    .disabled(scale >= ZOOM_MAX)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.zoom_by(true, cx))),
            )
            .child(
                Button::new("zoom-fit")
                    .ghost()
                    .label("Fit")
                    .disabled(self.zoom == Zoom::Fit)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.zoom = Zoom::Fit;
                        cx.notify();
                    })),
            )
    }

    fn body(&self, page: &FlowPage, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        match page.body() {
            Body::Loading => notice(LOADING_TITLE, LOADING_BODY, p).into_any_element(),
            Body::Empty => notice(EMPTY_TITLE, EMPTY_BODY, p).into_any_element(),
            Body::EmptyPeriod { title, body, widen } => {
                notice(&title, &body.unwrap_or_default(), p)
                    .when_some(widen, |n, label| {
                        n.child(
                            div().pt(px(8.0)).child(
                                Button::new("widen")
                                    .primary()
                                    .label(label)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.set_period(WIDEST_PERIOD, cx);
                                    })),
                            ),
                        )
                    })
                    .into_any_element()
            }
            Body::Map => {
                let details = self
                    .selection
                    .as_ref()
                    .and_then(|s| Details::of(&page.graph, s));
                v_flex()
                    .child(
                        h_flex()
                            .justify_between()
                            .py(px(12.0))
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .child(proof_legend(p))
                            .child(small_muted(LEGEND_NOTE, p)),
                    )
                    .child(
                        h_flex()
                            .pt(px(16.0))
                            .gap(px(24.0))
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .child(self.map(&page.graph, cx))
                                    .child(div().pt(px(8.0)).child(small_muted(HELP, p))),
                            )
                            .when_some(details, |row, d| row.child(Self::details(&d, cx))),
                    )
                    .into_any_element()
            }
        }
    }

    /// The map (`FlowMap`, `flow-map.tsx:34-122`) at the current scale, in a container that
    /// scrolls sideways.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's map element by element, read top to bottom"
    )]
    fn map(&self, g: &FlowGraph, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let z = self.scale();
        #[expect(clippy::cast_possible_truncation, reason = "pixels")]
        let at = |v: f64| px((v * z) as f32);
        let selected_edge = match &self.selection {
            Some(Selection::Edge(id)) => g.edge(id),
            _ => None,
        };
        let lines = EdgePaint::of(g, self.selection.as_ref(), self.active.as_ref(), z, p);
        let origin = Rc::clone(&self.map_origin);
        let width = Rc::clone(&self.map_width);
        let measured = width.get();

        let labels = g.columns.iter().filter_map(|c| {
            c.label.map(|label| {
                div()
                    .absolute()
                    .left(at(c.x))
                    .top(at(22.0 - META_SIZE * ASCENT))
                    .text_size(at(META_SIZE))
                    .font_weight(FontWeight::BOLD)
                    .text_color(p.ink_muted.to_hsla())
                    .whitespace_nowrap()
                    .child(label.to_uppercase())
            })
        });
        let nodes = g.nodes.iter().map(|n| {
            let selected = matches!(&self.selection, Some(Selection::Node(id)) if *id == n.id);
            let dimmed = self.active.as_ref().is_some_and(|a| !a.has_node(&n.id));
            let unassigned = n.node_type == MapNodeType::Unassigned;
            let (side, amount) = node_amount(n);
            let stroke = if selected {
                p.report_blue.to_hsla()
            } else if unassigned {
                p.signal_orange.to_hsla()
            } else {
                p.ink_navy.to_hsla()
            };
            div()
                .absolute()
                .left(at(n.x))
                .top(at(n.y))
                .w(at(NODE_W))
                .h(at(n.h))
                .border(at(if selected { 3.0 } else { 1.5 }))
                .border_color(stroke)
                .when(unassigned, gpui_kit::Styled::border_dashed)
                .bg(if selected {
                    p.report_blue.to_hsla().opacity(0.3)
                } else {
                    p.sheet.to_hsla()
                })
                .opacity(if dimmed { DIM } else { 1.0 })
                .children(n.lines.iter().enumerate().map(|(i, line)| {
                    div()
                        .absolute()
                        .left(at(NODE_PAD))
                        .top(at(name_baseline(i) - NAME_SIZE * ASCENT))
                        .text_size(at(NAME_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .whitespace_nowrap()
                        .child(line.clone())
                }))
                .child(
                    h_flex()
                        .absolute()
                        .left(at(NODE_PAD))
                        .right(at(NODE_PAD))
                        .top(at(meta_baseline(n) - META_SIZE * ASCENT))
                        .justify_between()
                        .text_size(at(META_SIZE))
                        .whitespace_nowrap()
                        .child(
                            div()
                                .text_color(p.ink_muted.to_hsla())
                                .child(n.node_type.label()),
                        )
                        .child(
                            h_flex()
                                .child(div().text_color(p.ink_muted.to_hsla()).child(side))
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(amount)),
                        ),
                )
        });
        let tag = selected_edge.map(|e| {
            let label = format_money(e.amount, "KES");
            #[expect(clippy::cast_precision_loss, reason = "a label's length")]
            let w = label.len() as f64 * 8.4 + 18.0;
            div()
                .absolute()
                .left(at(e.mid.0 - w / 2.0))
                .top(at(e.mid.1 - 26.0 - e.width / 2.0))
                .w(at(w))
                .h(at(24.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(p.ink_navy.to_hsla())
                .text_color(p.report_paper.to_hsla())
                .text_size(at(META_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label)
        });

        div()
            .id("flow-map")
            .relative()
            .w_full()
            .overflow_x_scroll()
            .border_1()
            .border_color(p.hairline.to_hsla())
            .bg(p.report_paper.to_hsla())
            .child(
                canvas(
                    move |bounds: Bounds<Pixels>, window, _| {
                        let w = f32::from(bounds.size.width);
                        if (w - measured).abs() > 0.5 {
                            width.set(w);
                            window.request_animation_frame();
                        }
                    },
                    |_, (), _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(
                div()
                    .relative()
                    .w(at(g.width))
                    .h(at(g.height))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_map_down))
                    .child(
                        canvas(
                            move |bounds: Bounds<Pixels>, _, _| {
                                origin.set(Some(bounds.origin));
                                bounds
                            },
                            move |_, bounds, window, _| lines.paint(bounds.origin, window),
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .children(labels)
                    .children(nodes)
                    .children(tag),
            )
    }

    /// The details panel (`FlowDetails`, `flow-details.tsx:22-170`).
    #[expect(
        clippy::too_many_lines,
        reason = "the web's panel element by element, read top to bottom"
    )]
    fn details(d: &Details, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let ledger_button =
            |id: SharedString, label: String, link: LedgerLink, cx: &mut Context<Self>| {
                Button::new(id)
                    .ghost()
                    .small()
                    .label(format!("{label} \u{2192}"))
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(OpenLedgerLines(link.clone()));
                    }))
            };
        let lists: Vec<AnyElement> = d
            .lists
            .iter()
            .enumerate()
            .map(|(li, list)| {
                let items: Vec<AnyElement> = list
                    .items
                    .iter()
                    .enumerate()
                    .map(|(ii, item)| {
                        let title = match item.direction {
                            Some(dir) => h_flex()
                                .gap(px(4.0))
                                .child(div().text_color(p.ink_muted.to_hsla()).child(dir))
                                .child(item.title.clone()),
                            None => h_flex().child(item.title.clone()),
                        };
                        let link = item.ledger.clone().map(|link| {
                            ledger_button(
                                SharedString::from(format!("ledger-{li}-{ii}")),
                                "Ledger lines".to_owned(),
                                link,
                                cx,
                            )
                        });
                        v_flex()
                            .py(px(12.0))
                            .gap(px(2.0))
                            .border_b(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .text_size(px(text::BODY))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .gap(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title.min_w(px(0.0)).overflow_hidden())
                                    .child(div().flex_shrink_0().child(item.amount.clone())),
                            )
                            .when_some(item.description.clone(), |row, desc| {
                                row.child(small_muted(&desc, p))
                            })
                            .child(
                                h_flex()
                                    .pt(px(4.0))
                                    .justify_between()
                                    .child(small_muted(&item.date, p))
                                    .child(status_mark(item.status, proof_label(item.status), p)),
                            )
                            .children(link)
                            .into_any_element()
                    })
                    .collect();
                v_flex()
                    .pt(px(24.0))
                    .child(
                        div()
                            .pb(px(8.0))
                            .text_size(px(text::LABEL))
                            .font_weight(FontWeight::BOLD)
                            .text_color(p.ink_muted.to_hsla())
                            .child(list.title.to_uppercase()),
                    )
                    .child(
                        v_flex()
                            .border_t(px(space::HAIRLINE))
                            .border_color(p.hairline.to_hsla())
                            .children(items),
                    )
                    .into_any_element()
            })
            .collect();
        let ledger = d
            .ledger
            .clone()
            .map(|(label, link)| ledger_button("ledger-panel".into(), label, link, cx));
        let figures = h_flex()
            .mt(px(16.0))
            .py(px(12.0))
            .justify_between()
            .items_end()
            .border_t(px(space::HAIRLINE))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .children(d.figures.iter().map(|(label, value)| {
                v_flex()
                    .child(
                        div()
                            .text_size(px(text::TITLE))
                            .font_weight(FontWeight::BOLD)
                            .child(value.clone()),
                    )
                    .child(small_muted(label, p))
            }))
            .when_some(d.status, |row, s| {
                row.child(status_mark(s, proof_label(s), p))
            });
        v_flex()
            .id("flow-details")
            .w(px(DETAILS_W))
            .flex_shrink_0()
            .pt(px(12.0))
            .border_t(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                h_flex()
                    .justify_between()
                    .items_start()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(text::TITLE))
                            .font_weight(FontWeight::BOLD)
                            .child(d.heading.clone()),
                    )
                    .child(
                        Button::new("clear-selection")
                            .ghost()
                            .small()
                            .label(CLEAR_LABEL)
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _, cx| this.select(None, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .pt(px(4.0))
                    .text_size(px(text::BODY))
                    .text_color(p.ink_muted.to_hsla())
                    .child(d.subtitle.clone()),
            )
            .child(figures)
            .children(ledger)
            .children(lists)
    }
}

/// One line to paint, in map units.
struct EdgeLine {
    path: crate::flows::EdgePath,
    width: f64,
    color: Hsla,
    dash: Option<[f64; 2]>,
    halo: Option<Hsla>,
    cross: Option<((f64, f64), f64, Hsla)>,
}

/// Every line of the map with its colour, pattern and fade (`MapEdge`, `flow-map.tsx:206-270`).
struct EdgePaint {
    lines: Vec<EdgeLine>,
    zoom: f64,
}

impl EdgePaint {
    fn of(
        g: &FlowGraph,
        selection: Option<&Selection>,
        active: Option<&Connected>,
        zoom: f64,
        p: &Palette,
    ) -> Self {
        let lines = g
            .edges
            .iter()
            .map(|e| {
                let dimmed = active.is_some_and(|a| !a.has_edge(&e.id));
                let fade = if dimmed { DIM } else { 1.0 };
                let base = match e.status {
                    TransactionStatus::Verified => p.ink_navy.to_hsla(),
                    TransactionStatus::Pending => p.ink_muted.to_hsla(),
                    TransactionStatus::Flagged => p.signal_orange.to_hsla(),
                    TransactionStatus::Rejected => p.rejection_red.to_hsla(),
                };
                let selected = matches!(selection, Some(Selection::Edge(id)) if *id == e.id);
                EdgeLine {
                    path: e.path,
                    width: e.width,
                    color: base.opacity(0.85 * fade),
                    dash: edge_dash(e.status, e.width),
                    halo: selected.then(|| p.report_blue.to_hsla().opacity(0.5 * fade)),
                    cross: (e.status == TransactionStatus::Flagged).then(|| {
                        (
                            e.mid,
                            cross_size(e.width),
                            p.signal_orange.to_hsla().opacity(fade),
                        )
                    }),
                }
            })
            .collect();
        Self { lines, zoom }
    }

    #[expect(clippy::cast_possible_truncation, reason = "pixels")]
    fn paint(&self, origin: Point<Pixels>, window: &mut Window) {
        let z = self.zoom;
        let at =
            |x: f64, y: f64| point(origin.x + px((x * z) as f32), origin.y + px((y * z) as f32));
        let curve = |l: &EdgeLine, width: f64, dash: Option<[f64; 2]>| {
            let mut b = PathBuilder::stroke(px((width * z) as f32));
            if let Some([on, off]) = dash {
                b = b.dash_array(&[px((on * z) as f32), px((off * z) as f32)]);
            }
            let c = l.path;
            b.move_to(at(c.x1, c.y1));
            b.cubic_bezier_to(at(c.x2, c.y2), at(c.mx, c.y1), at(c.mx, c.y2));
            b.build().ok()
        };
        for l in &self.lines {
            if let Some(halo) = l.halo
                && let Some(path) = curve(l, l.width + 10.0, None)
            {
                window.paint_path(path, halo);
            }
            if let Some(path) = curve(l, l.width, l.dash) {
                window.paint_path(path, l.color);
            }
            if let Some(((mx, my), k, color)) = l.cross {
                let mut b = PathBuilder::stroke(px((3.0 * z) as f32));
                b.move_to(at(mx - k, my - k));
                b.line_to(at(mx + k, my + k));
                b.move_to(at(mx + k, my - k));
                b.line_to(at(mx - k, my + k));
                if let Ok(path) = b.build() {
                    window.paint_path(path, color);
                }
            }
        }
    }
}

impl Render for FlowsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let page = self.page.clone();
        let show_map = page.as_ref().is_some_and(|pg| pg.body() == Body::Map);
        let actions = if show_map {
            self.zoom_actions(cx).into_any_element()
        } else {
            div().into_any_element()
        };
        let body = match &page {
            Some(pg) => self.body(pg, cx),
            None => notice(LOADING_TITLE, LOADING_BODY, p).into_any_element(),
        };
        v_flex()
            .w_full()
            .min_w(px(0.0))
            .gap(px(space::SECTION))
            .child(self.header(cx))
            .when_some(page.filter(|_| show_map), |col, pg| {
                col.child(figures(&pg.figures(), p))
            })
            .child(
                v_flex()
                    .min_w(px(0.0))
                    .child(section_header("Flow map", actions, cx))
                    .child(body),
            )
    }
}

/// The key figures (`KeyFigures`, `flows.tsx:267-302`): the details read muted, as the web's.
fn figures(figures: &[Figure], p: &Palette) -> Div {
    h_flex()
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .children(figures.iter().enumerate().map(|(i, f)| {
            let size = if i == 0 { text::FIGURE } else { text::TITLE };
            v_flex()
                .flex_1()
                .p(px(space::CELL))
                .when(i > 0, |cell| {
                    cell.border_l(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                })
                .child(
                    div()
                        .text_size(px(size))
                        .line_height(px(size))
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
                .when_some(f.detail.clone(), |cell, detail| {
                    cell.child(div().pt(px(4.0)).child(small_muted(&detail, p)))
                })
        }))
}
