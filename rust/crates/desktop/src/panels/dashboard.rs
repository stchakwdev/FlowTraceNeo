//! The Dashboard screen (DT1): key figures, funding received by donor and the eight latest
//! movements, as the web's `routes/_authenticated/dashboard.tsx` lays them out. Everything shown
//! comes from [`crate::dashboard::Dashboard`] over the store's transparency records; this file
//! only draws. Also the draw helpers Campaigns shares: a key-figure row with details, the
//! coverage bar of a [`Bar`] and the proof line mark (`StatusMark`).
//!
//! The web's header links ("Record a transaction", "Open the flow map") and the donor hover
//! that dims other movements are left out: the desktop has no transactions or flow map screen
//! yet (`rust/docs/handoff.md`).

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, Div, Entity, FontWeight, IntoElement, ParentElement, Render, Styled as _,
    Subscription, Window, div, px, relative,
};
use ledger_core::Minor;
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::flows::TransactionStatus;

use crate::dashboard::{Bar, Dashboard, DonorRow, Figure, MovementRow, proof_label};
use crate::panels::{demo_tag, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// The Dashboard screen.
pub struct DashboardPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    org_name: String,
    demo: bool,
    dashboard: Option<Dashboard>,
    _subscriptions: Vec<Subscription>,
}

impl DashboardPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&store, |this, _, cx| this.refresh(cx))];
        let mut panel = Self {
            store,
            seen: None,
            org_name: String::new(),
            demo: false,
            dashboard: None,
            _subscriptions: subscriptions,
        };
        panel.refresh(cx);
        panel
    }

    /// Re-derives when the organisation or its transparency records changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        let records = view.transparency().cloned();
        self.org_name = records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.name.clone()))
            .or_else(|| view.selected().map(|o| o.name.clone()))
            .unwrap_or_default();
        self.dashboard = records.as_deref().map(Dashboard::of);
        cx.notify();
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let as_of = self.dashboard.as_ref().and_then(|d| d.as_of.clone());
        v_flex().gap(px(8.0)).child(page_title("Dashboard")).child(
            h_flex()
                .gap(px(12.0))
                .text_size(px(text::BODY))
                .text_color(p.ink_muted.to_hsla())
                .when(!self.org_name.is_empty(), |m| {
                    m.child(self.org_name.clone())
                })
                .when_some(as_of, ParentElement::child)
                .when(self.demo, |m| m.child(demo_tag(cx))),
        )
    }

    fn body(d: &Dashboard, cx: &Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        if d.empty {
            return v_flex()
                .child(section_header("Key figures", div(), cx))
                .child(notice(
                    "No money movements recorded yet",
                    "Your key figures, funding by donor and recent movements will appear here \
                     once you record your first transaction.",
                    p,
                ))
                .into_any_element();
        }
        let donors = if d.donors.is_empty() {
            notice(
                "No donor funding recorded yet",
                "Record a transaction from a donor node and it will appear here with its proof \
                 status.",
                p,
            )
            .into_any_element()
        } else {
            v_flex()
                .children(d.donors.iter().map(|row| donor_row(row, p)))
                .child(
                    v_flex()
                        .mt(px(8.0))
                        .pt(px(12.0))
                        .gap(px(12.0))
                        .border_t(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .child(proof_legend(p))
                        .child(small_muted(&d.donor_total, p)),
                )
                .into_any_element()
        };
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .child(section_header("Key figures", div(), cx))
                    .child(figures_row(&d.figures, p)),
            )
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        v_flex()
                            .flex_basis(relative(5.0 / 12.0))
                            .min_w(px(0.0))
                            .child(section_header("Funding received, by donor", div(), cx))
                            .child(donors),
                    )
                    .child(
                        v_flex()
                            .flex_basis(relative(7.0 / 12.0))
                            .min_w(px(0.0))
                            .child(section_header("Recent movements", div(), cx))
                            .child(movements(&d.recent, p)),
                    ),
            )
            .into_any_element()
    }
}

impl Render for DashboardPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let body = match &self.dashboard {
            Some(d) => Self::body(d, cx),
            None => notice(
                "No transparency records loaded",
                "This organisation's movements and trust score load with its books from the \
                 server.",
                p,
            )
            .into_any_element(),
        };
        v_flex()
            .gap(px(space::SECTION))
            .child(self.header(cx))
            .child(body)
    }
}

fn donor_row(row: &DonorRow, p: &Palette) -> impl IntoElement {
    h_flex()
        .min_h(px(40.0))
        .py(px(8.0))
        .gap(px(16.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::BODY))
        .child(
            div()
                .w(px(170.0))
                .flex_shrink_0()
                .font_weight(FontWeight::MEDIUM)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(row.name.clone()),
        )
        .child(div().flex_1().min_w(px(0.0)).child(bar(row.bar, 12.0, p)))
        .child(
            div()
                .flex_shrink_0()
                .font_weight(FontWeight::SEMIBOLD)
                .whitespace_nowrap()
                .child(row.amount.clone()),
        )
}

fn movements(rows: &[MovementRow], p: &Palette) -> impl IntoElement {
    let head = |label: &'static str| {
        div()
            .text_size(px(12.0))
            .text_color(p.ink_muted.to_hsla())
            .child(label)
    };
    v_flex()
        .child(
            h_flex()
                .py(px(8.0))
                .gap(px(16.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(head("Movement").flex_1())
                .child(head("Date").w(px(96.0)))
                .child(head("Amount").w(px(130.0)).flex().justify_end())
                .child(head("Proof").w(px(120.0))),
        )
        .children(rows.iter().map(|r| {
            h_flex()
                .py(px(10.0))
                .gap(px(16.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .text_size(px(text::BODY))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(r.movement.clone()),
                        )
                        .child(small_muted(&r.description, p)),
                )
                .child(
                    div()
                        .w(px(96.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(r.date.clone()),
                )
                .child(
                    div()
                        .w(px(130.0))
                        .flex()
                        .justify_end()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(r.amount.clone()),
                )
                .child(
                    div()
                        .w(px(120.0))
                        .child(status_mark(r.status, proof_label(r.status), p)),
                )
        }))
}

// Shared with Campaigns.

/// A row of key figures with an optional flagged detail under each (`KeyFigures`,
/// `KeyFigure`): the first figure set larger.
pub(crate) fn figures_row(figures: &[Figure], p: &Palette) -> Div {
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
                    cell.child(
                        div()
                            .pt(px(4.0))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.signal_orange.to_hsla())
                            .child(detail),
                    )
                })
        }))
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
/// filled part in coverage blue (signal orange while awaiting proof), the gap in gap grey.
pub(crate) fn bar(b: Bar, height: f32, p: &Palette) -> Div {
    let evidenced = share(b.evidenced.unwrap_or(0), b.total);
    let filled = share(b.filled, b.total).max(evidenced);
    let rest = if b.pending {
        p.signal_orange.to_hsla().opacity(0.45)
    } else {
        p.coverage_blue.to_hsla()
    };
    h_flex()
        .w_full()
        .h(px(height))
        .bg(p.gap_grey.to_hsla())
        .child(
            div()
                .h_full()
                .w(relative(evidenced))
                .bg(p.report_blue.to_hsla()),
        )
        .child(div().h_full().w(relative(filled - evidenced)).bg(rest))
}

/// `StatusMark` (`components/report/status-line.tsx`): a short line sample and the label. The
/// pattern carries the state, not the colour alone: solid ink (evidenced), dashed muted
/// (awaiting), long dashes in signal orange (flagged), dots in rejection red (rejected).
pub(crate) fn status_mark(status: TransactionStatus, label: &str, p: &Palette) -> Div {
    let (color, dash, gap) = match status {
        TransactionStatus::Verified => (p.ink_navy.to_hsla(), 22.0, 0.0),
        TransactionStatus::Pending => (p.ink_muted.to_hsla(), 6.0, 5.0),
        TransactionStatus::Flagged => (p.signal_orange.to_hsla(), 9.0, 4.0),
        TransactionStatus::Rejected => (p.rejection_red.to_hsla(), 2.0, 4.0),
    };
    let mut segments = Vec::new();
    let (mut at, mut on) = (0.0, true);
    while at < 22.0 {
        let len = f32::min(if on { dash } else { gap }, 22.0 - at);
        if len <= 0.0 {
            break;
        }
        segments.push(div().w(px(len)).h(px(3.0)).when(on, |d| d.bg(color)));
        at += len;
        on = !on;
    }
    let text_color = match status {
        TransactionStatus::Verified => p.ink_navy.to_hsla(),
        TransactionStatus::Pending => p.ink_muted.to_hsla(),
        TransactionStatus::Flagged => p.signal_orange.to_hsla(),
        TransactionStatus::Rejected => p.rejection_red.to_hsla(),
    };
    h_flex()
        .gap(px(8.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(text_color)
        .whitespace_nowrap()
        .child(h_flex().w(px(22.0)).flex_shrink_0().children(segments))
        .child(label.to_owned())
}

/// `ProofLegend`: the three line patterns.
pub(crate) fn proof_legend(p: &Palette) -> Div {
    h_flex()
        .gap(px(20.0))
        .child(status_mark(TransactionStatus::Verified, "Evidenced", p))
        .child(status_mark(TransactionStatus::Pending, "Awaiting proof", p))
        .child(status_mark(TransactionStatus::Flagged, "Flagged", p))
}

/// `ReportNotice`: a title and a muted line.
pub(crate) fn notice(title: &str, body: &str, p: &Palette) -> Div {
    v_flex()
        .py(px(24.0))
        .gap(px(6.0))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_size(px(text::BODY))
                .text_color(p.ink_muted.to_hsla())
                .child(body.to_owned()),
        )
}

/// 12px muted text.
pub(crate) fn small_muted(text_: &str, p: &Palette) -> Div {
    div()
        .text_size(px(12.0))
        .text_color(p.ink_muted.to_hsla())
        .child(text_.to_owned())
}
