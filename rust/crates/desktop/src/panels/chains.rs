//! The Chains screen (DT2): the web's "Chain links" page (`routes/_authenticated/chains.tsx`)
//! with its request dialog (`components/chain-request-modal.tsx`), review dialog
//! (`components/chain-approval-modal.tsx`), revoke and cancel confirmations, and the chain line
//! drawing (`components/chain-status-badge.tsx`). Everything shown comes from [`crate::chains`];
//! writes go through the store ([`crate::panels::write_chain`]) and the new chains come back
//! through its change channel. Dialogs are inline boxes over the sections, as the other screens'
//! confirmations are.
//!
//! The web searches every organisation by name; the module has no search, so the request
//! dialog searches the published organisations (`public_organization`). Keys: N opens the
//! request dialog, Esc closes a dialog.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px, relative,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::chains::{ChainDetails, ChainStatus};

use crate::backend::{ChainWrite, SubmitResult};
use crate::chains::{
    ChainAction, ChainRow, ChainsView, Confirm, FALLBACK_ERROR, GROUP_LABEL, GROUP_NONE, INCOMING,
    KEEP_PENDING, LEGEND, NODE_HINT, NODE_LABEL, NODE_NONE, OUTGOING, OrgOption, REASON_LABEL,
    REASON_PLACEHOLDER, Review, ReviewMode, Search, Section, group_hint, lede, links_meta, request,
    request_error, search, status_label,
};
use crate::panels::bills::{new_pick, picked, picks};
use crate::panels::dashboard::{notice, small_muted};
use crate::panels::{PickState, demo_tag, field, hint, page_title, section_header};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// The dialog open over the sections.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Dialog {
    /// Request a chain link.
    Request,
    /// Approve or reject one incoming request.
    Review(String, ReviewMode),
    /// Revoke (true) or cancel (false) one chain.
    Confirm(String, bool),
}

/// The Chains screen.
pub struct ChainsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64, u64)>,
    org: Option<OrganizationId>,
    org_name: Option<String>,
    demo: bool,
    chains: Arc<Vec<ChainDetails>>,
    records: Option<Arc<OrgTransparency>>,
    view: ChainsView,

    dialog: Option<Dialog>,
    pending: bool,
    error: Option<String>,

    query: Entity<InputState>,
    message: Entity<InputState>,
    results: Search,
    chosen: Option<OrgOption>,

    group_pick: PickState,
    node_pick: PickState,
    reason: Entity<InputState>,
    missing_group: bool,

    subscriptions: Vec<Subscription>,
}

impl ChainsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(request::PLACEHOLDER));
        let message =
            cx.new(|cx| InputState::new(window, cx).placeholder(request::MESSAGE_PLACEHOLDER));
        let reason = cx.new(|cx| InputState::new(window, cx).placeholder(REASON_PLACEHOLDER));
        let group_pick = new_pick(window, cx, Vec::new(), None);
        let node_pick = new_pick(window, cx, Vec::new(), None);
        let subscriptions = vec![
            cx.observe(&store, |this, _, cx| this.refresh(cx)),
            cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.search(cx);
                }
            }),
        ];
        let mut panel = Self {
            store,
            seen: None,
            org: None,
            org_name: None,
            demo: false,
            chains: Arc::default(),
            records: None,
            view: ChainsView::default(),
            dialog: None,
            pending: false,
            error: None,
            query,
            message,
            results: Search::TooShort,
            chosen: None,
            group_pick,
            node_pick,
            reason,
            missing_group: false,
            subscriptions,
        };
        panel.refresh(cx);
        panel
    }

    /// Re-derives the rows when the organisation, the chains or its records changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (
            org.clone(),
            view.chains_revision(),
            view.transparency_revision(),
        );
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        if org != self.org {
            self.dialog = None;
        }
        self.seen = Some(key);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        self.chains = Arc::clone(view.chains());
        self.records = view.transparency().cloned();
        self.org_name = self
            .records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.name.clone()))
            .or_else(|| view.selected().map(|o| o.name.clone()));
        let id = org.as_ref().map_or("", OrganizationId::as_str);
        self.view = ChainsView::of(&self.chains, id);
        self.org = org;
        cx.notify();
    }

    /// Opens a dialog for screenshots (`--chains request|approve|reject|revoke|cancel`): on the
    /// first row that offers it.
    pub fn launch(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let action = match value {
            "request" => {
                self.open(Dialog::Request, window, cx);
                return;
            }
            "approve" => ChainAction::Approve,
            "reject" => ChainAction::Reject,
            "revoke" => ChainAction::Revoke,
            "cancel" => ChainAction::Cancel,
            _ => return,
        };
        let row = self
            .view
            .incoming
            .iter()
            .chain(&self.view.outgoing)
            .find(|r| r.actions.contains(&action))
            .map(|r| r.id.clone());
        if let Some(id) = row {
            self.act(action, id, window, cx);
        }
    }

    /// The keys of the module docs. Returns whether the key was used.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "n" if self.dialog.is_none() => self.open(Dialog::Request, window, cx),
            "escape" if self.dialog.is_some() => self.close(cx),
            _ => return false,
        }
        true
    }

    fn act(
        &mut self,
        action: ChainAction,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dialog = match action {
            ChainAction::Approve => Dialog::Review(id, ReviewMode::Approve),
            ChainAction::Reject => Dialog::Review(id, ReviewMode::Reject),
            ChainAction::Revoke => Dialog::Confirm(id, true),
            ChainAction::Cancel => Dialog::Confirm(id, false),
        };
        self.open(dialog, window, cx);
    }

    /// Opens `dialog` fresh: fields cleared, as the web's modals reset on close.
    fn open(&mut self, dialog: Dialog, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        self.pending = false;
        self.missing_group = false;
        self.chosen = None;
        self.results = Search::TooShort;
        for input in [&self.query, &self.message, &self.reason] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        if let Dialog::Review(_, ReviewMode::Approve) = &dialog {
            let (groups, nodes) = self.records.as_deref().map_or_else(
                || (Vec::new(), Vec::new()),
                |t| {
                    (
                        t.access_groups
                            .iter()
                            .map(|g| (g.id.clone(), g.name.clone()))
                            .collect::<Vec<_>>(),
                        t.flow_nodes
                            .iter()
                            .map(|n| (n.id.clone(), n.name.clone()))
                            .collect::<Vec<_>>(),
                    )
                },
            );
            self.group_pick = new_pick(window, cx, picks(&groups, Some(GROUP_NONE)), Some(""));
            self.node_pick = new_pick(window, cx, picks(&nodes, Some(NODE_NONE)), Some(""));
            let sub = cx.subscribe_in(
                &self.group_pick,
                window,
                |this, _, _: &SelectEvent<Vec<crate::panels::Pick>>, _, cx| {
                    this.missing_group = false;
                    cx.notify();
                },
            );
            self.subscriptions.push(sub);
        }
        if dialog == Dialog::Request {
            self.query.update(cx, |i, cx| i.focus(window, cx));
        }
        self.dialog = Some(dialog);
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        self.error = None;
        self.pending = false;
        cx.notify();
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        let q = self.query.read(cx).value().to_string();
        let orgs = self.store.read(cx).public_organizations();
        let own = self.org.as_ref().map_or("", OrganizationId::as_str);
        self.results = search(&orgs, &q, own, &self.view.linked);
        cx.notify();
    }

    fn row(&self, id: &str) -> Option<&ChainRow> {
        self.view
            .incoming
            .iter()
            .chain(&self.view.outgoing)
            .find(|r| r.id == id)
    }

    /// Sends a chain write; the dialog closes on success, else shows the refusal.
    fn send(&mut self, write: ChainWrite, request: bool, cx: &mut Context<Self>) {
        self.pending = true;
        self.error = None;
        let store = self.store.clone();
        crate::panels::write_chain(
            self,
            &store,
            write,
            cx,
            move |this, result: SubmitResult, cx| {
                this.pending = false;
                match result {
                    Ok(_) => this.close(cx),
                    Err(e) => {
                        let text = e.to_string();
                        let text = if text.is_empty() {
                            FALLBACK_ERROR.to_owned()
                        } else {
                            text
                        };
                        this.error = Some(if request { request_error(&text) } else { text });
                        cx.notify();
                    }
                }
            },
        );
    }

    fn submit_request(&mut self, cx: &mut Context<Self>) {
        let (Some(org), Some(chosen)) = (self.org.clone(), self.chosen.clone()) else {
            return;
        };
        let message = self.message.read(cx).value().trim().to_owned();
        let write = ChainWrite::Request {
            source: org,
            target: OrganizationId::from(chosen.id.as_str()),
            message: (!message.is_empty()).then_some(message),
        };
        self.send(write, true, cx);
    }

    fn submit_review(&mut self, id: String, mode: ReviewMode, cx: &mut Context<Self>) {
        let write = match mode {
            ReviewMode::Approve => {
                let Some(group) = picked(&self.group_pick, cx) else {
                    self.missing_group = true;
                    cx.notify();
                    return;
                };
                ChainWrite::Approve {
                    id,
                    group,
                    node: picked(&self.node_pick, cx),
                }
            }
            ReviewMode::Reject => {
                let reason = self.reason.read(cx).value().trim().to_owned();
                ChainWrite::Reject {
                    id,
                    reason: (!reason.is_empty()).then_some(reason),
                }
            }
        };
        self.send(write, false, cx);
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .items_end()
            .justify_between()
            .gap(px(16.0))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .child(page_title("Chain links"))
                            .when(self.demo, |row| row.child(demo_tag(cx))),
                    )
                    .child(
                        div()
                            .max_w(px(672.0))
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(lede(self.org_name.as_deref())),
                    ),
            )
            .child(Self::request_button("request-link", cx))
    }

    fn request_button(id: &'static str, cx: &mut Context<Self>) -> Button {
        Button::new(id)
            .primary()
            .label("Request a link")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.open(Dialog::Request, window, cx);
            }))
    }

    /// The key figures (`chains.tsx:118-150`).
    fn figures(&self, p: &Palette) -> Div {
        let v = &self.view;
        let cells = [
            (v.active, "Active links", text::FIGURE),
            (v.awaiting, "Awaiting your review", text::TITLE),
            (v.requested, "Your requests pending", text::TITLE),
            (v.ended, "Ended", text::LEDE),
        ];
        h_flex()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .children(cells.into_iter().enumerate().map(|(i, (n, label, size))| {
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
                            .child(n.to_string()),
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

    fn section(s: Section, rows: &[ChainRow], cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let body = if rows.is_empty() {
            let mut n = notice(s.empty_title, s.empty_body, p);
            if s.empty_action {
                n = n.child(
                    div()
                        .pt(px(8.0))
                        .child(Self::request_button("request-empty", cx)),
                );
            }
            n.into_any_element()
        } else {
            let items: Vec<AnyElement> = rows.iter().map(|r| Self::chain_row(r, cx)).collect();
            v_flex().children(items).into_any_element()
        };
        v_flex()
            .child(section_header(
                s.title,
                small_muted(&links_meta(rows.len()), p),
                cx,
            ))
            .child(body)
    }

    /// One chain (`ChainRow`, `chains.tsx:323-492`).
    #[expect(
        clippy::too_many_lines,
        reason = "one row of the web's layout, read top to bottom"
    )]
    fn chain_row(r: &ChainRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let ink = if r.ended { p.ink_muted } else { p.ink_navy }.to_hsla();
        let you = |is: bool| v_flex().when(is, |c| c.child(small_muted("You", p)));
        let pair = h_flex()
            .gap(px(12.0))
            .items_center()
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.source.clone()),
                    )
                    .child(you(r.direction == crate::chains::Direction::Outgoing)),
            )
            .child(
                v_flex()
                    .w(px(112.0))
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if r.status == ChainStatus::Active {
                                p.ink_navy.to_hsla()
                            } else {
                                p.ink_muted.to_hsla()
                            })
                            .child(status_label(r.status).to_uppercase()),
                    )
                    .child(chain_line(r.status, 112.0, p)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .items_end()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.target.clone()),
                    )
                    .child(you(r.direction == crate::chains::Direction::Incoming)),
            );
        let left = v_flex()
            .flex_basis(relative(0.5))
            .min_w(px(0.0))
            .child(pair)
            .child(div().pt(px(8.0)).child(small_muted(&r.when, p)))
            .when_some(r.message.clone(), |c, m| {
                c.child(
                    div()
                        .mt(px(12.0))
                        .pl(px(12.0))
                        .border_l(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(m),
                )
            })
            .when_some(r.reason.clone(), |c, reason| {
                c.child(
                    h_flex()
                        .mt(px(12.0))
                        .text_size(px(text::BODY))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("Reason: "))
                        .child(reason),
                )
            });
        let facts = h_flex()
            .flex_basis(relative(1.0 / 3.0))
            .gap(px(16.0))
            .items_start()
            .text_size(px(text::BODY))
            .child(
                v_flex()
                    .flex_1()
                    .child(small_muted("Access granted", p))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.access.clone()),
                    )
                    .when_some(r.access_note.clone(), |c, n| c.child(small_muted(&n, p))),
            )
            .child(
                v_flex()
                    .flex_1()
                    .child(small_muted("Chain Trust Score", p))
                    .child(
                        h_flex()
                            .items_end()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(text::TITLE))
                                    .font_weight(FontWeight::BOLD)
                                    .child(r.score.to_string()),
                            )
                            .child(small_muted("/ 100", p)),
                    )
                    .child(small_muted(&r.score_note, p)),
            );
        let buttons: Vec<Button> = r
            .actions
            .iter()
            .map(|&action| {
                let id = r.id.clone();
                let button = Button::new(SharedString::from(format!("{action:?}-{}", r.id)))
                    .label(action.label())
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.act(action, id.clone(), window, cx);
                    }));
                if action == ChainAction::Approve {
                    button.primary()
                } else {
                    button.outline()
                }
            })
            .collect();
        h_flex()
            .py(px(20.0))
            .gap(px(16.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_color(ink)
            .child(left)
            .child(facts)
            .child(
                h_flex()
                    .flex_1()
                    .justify_end()
                    .gap(px(8.0))
                    .children(buttons),
            )
            .into_any_element()
    }

    /// The open dialog, as an inline box with the 3px ink rule (`ReportDialog`).
    fn dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.dialog.clone()?;
        let p = palette(cx.theme().mode);
        let inner = match dialog {
            Dialog::Request => self.request_dialog(cx),
            Dialog::Review(id, mode) => {
                let chain = self.chains.iter().find(|c| c.chain.id == id)?.clone();
                self.review_dialog(&chain, mode, cx)
            }
            Dialog::Confirm(id, revoke) => {
                let row = self.row(&id)?;
                let c = if revoke {
                    Confirm::revoke(row)
                } else {
                    Confirm::cancel(row)
                };
                let write = if revoke {
                    ChainWrite::Revoke { id }
                } else {
                    ChainWrite::Cancel { id }
                };
                v_flex()
                    .gap(px(12.0))
                    .child(dialog_title(&c.title))
                    .child(div().text_size(px(text::BODY)).child(c.body))
                    .when_some(self.error.clone(), |d, e| d.child(error_line(&e, p)))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                Button::new("confirm-keep")
                                    .outline()
                                    .label(c.keep)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.close(cx);
                                    })),
                            )
                            .child(
                                Button::new("confirm-go")
                                    .danger()
                                    .label(c.confirm)
                                    .disabled(self.pending)
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.send(write.clone(), false, cx);
                                    })),
                            ),
                    )
                    .into_any_element()
            }
        };
        Some(
            v_flex()
                .id("chain-dialog")
                .max_w(px(560.0))
                .p(px(24.0))
                .border_1()
                .border_t(px(space::RULE_HEAVY))
                .border_color(p.ink_navy.to_hsla())
                .bg(p.sheet.to_hsla())
                .child(inner)
                .into_any_element(),
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the web's dialog, read top to bottom"
    )]
    fn request_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let org_area = if let Some(org) = &self.chosen {
            h_flex()
                .justify_between()
                .py(px(12.0))
                .border_t(px(space::HAIRLINE))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(org_line(org, p))
                .child(
                    Button::new("change-org")
                        .outline()
                        .label("Change")
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.chosen = None;
                            this.query.update(cx, |i, cx| i.focus(window, cx));
                            cx.notify();
                        })),
                )
                .into_any_element()
        } else {
            {
                let results: AnyElement = match &self.results {
                    Search::TooShort => div().into_any_element(),
                    Search::NoMatch(text_) => div()
                        .py(px(12.0))
                        .text_size(px(text::BODY))
                        .text_color(p.ink_muted.to_hsla())
                        .child(text_.clone())
                        .into_any_element(),
                    Search::Results(found) => v_flex()
                        .border_t(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .children(found.iter().map(|org| {
                            let pick = org.clone();
                            h_flex()
                                .id(SharedString::from(format!("org-{}", org.id)))
                                .min_h(px(48.0))
                                .px(px(8.0))
                                .justify_between()
                                .border_b(px(space::HAIRLINE))
                                .border_color(p.hairline.to_hsla())
                                .child(org_line(org, p))
                                .when(org.linked, |row| {
                                    row.opacity(0.6).child(small_muted(request::LINKED, p))
                                })
                                .when(!org.linked, |row| {
                                    row.cursor_pointer()
                                        .hover(|r| r.bg(p.wash.to_hsla()))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.chosen = Some(pick.clone());
                                                this.query.update(cx, |i, cx| {
                                                    i.set_value("", window, cx);
                                                });
                                                this.results = Search::TooShort;
                                                cx.notify();
                                            },
                                        ))
                                })
                        }))
                        .into_any_element(),
                };
                v_flex()
                    .gap(px(6.0))
                    .child(Input::new(&self.query))
                    .child(hint(request::HINT, cx))
                    .child(results)
                    .into_any_element()
            }
        };
        let sending = if self.pending {
            request::SENDING
        } else {
            request::SEND
        };
        v_flex()
            .gap(px(16.0))
            .child(dialog_title(request::TITLE))
            .child(small_muted(request::DESCRIPTION, p))
            .child(field("Organisation", org_area, None, cx))
            .child(
                v_flex()
                    .gap(px(6.0))
                    .child(optional_label("Message", p))
                    .child(Input::new(&self.message))
                    .child(hint(request::MESSAGE_HINT, cx)),
            )
            .when_some(self.error.clone(), |d, e| d.child(error_line(&e, p)))
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::new("request-cancel")
                            .outline()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    )
                    .child(
                        Button::new("request-send")
                            .primary()
                            .label(sending)
                            .disabled(self.chosen.is_none() || self.pending)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.submit_request(cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn review_dialog(
        &self,
        chain: &ChainDetails,
        mode: ReviewMode,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        let r = Review::of(chain, mode);
        let facts = v_flex()
            .py(px(12.0))
            .gap(px(8.0))
            .border_t(px(space::HAIRLINE))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(text::BODY))
            .children(r.facts.iter().map(|(label, value)| {
                h_flex()
                    .gap(px(16.0))
                    .child(
                        div()
                            .w(px(110.0))
                            .flex_shrink_0()
                            .text_color(p.ink_muted.to_hsla())
                            .child(*label),
                    )
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(value.clone()))
            }));
        let controls = match mode {
            ReviewMode::Approve => {
                let chosen = picked(&self.group_pick, cx).and_then(|id| {
                    self.records
                        .as_ref()
                        .and_then(|t| t.access_groups.iter().find(|g| g.id == id))
                        .map(|g| g.permissions)
                });
                let hint_text =
                    group_hint(self.missing_group, self.records.is_none(), chosen.as_ref());
                let hint_color = if self.missing_group {
                    p.rejection_red
                } else {
                    p.ink_muted
                };
                v_flex()
                    .gap(px(16.0))
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(text::BODY))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(GROUP_LABEL),
                            )
                            .child(Select::new(&self.group_pick))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(hint_color.to_hsla())
                                    .child(hint_text),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(optional_label(NODE_LABEL, p))
                            .child(Select::new(&self.node_pick))
                            .child(hint(NODE_HINT, cx)),
                    )
            }
            ReviewMode::Reject => v_flex()
                .gap(px(6.0))
                .child(optional_label(REASON_LABEL, p))
                .child(Input::new(&self.reason)),
        };
        let (idle, busy) = r.submit;
        let id = chain.chain.id.clone();
        let submit = Button::new("review-submit")
            .label(if self.pending { busy } else { idle })
            .disabled(self.pending)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.submit_review(id.clone(), mode, cx);
            }));
        let submit = match mode {
            ReviewMode::Approve => submit.primary(),
            ReviewMode::Reject => submit.danger(),
        };
        v_flex()
            .gap(px(16.0))
            .child(dialog_title(&r.title))
            .child(small_muted(r.description, p))
            .child(facts)
            .child(controls)
            .when_some(self.error.clone(), |d, e| d.child(error_line(&e, p)))
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::new("review-keep")
                            .outline()
                            .label(KEEP_PENDING)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    )
                    .child(submit),
            )
            .into_any_element()
    }
}

impl Render for ChainsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let incoming = self.view.incoming.clone();
        let outgoing = self.view.outgoing.clone();
        let dialog = self.dialog(cx);
        v_flex()
            .w_full()
            .min_w(px(0.0))
            .gap(px(40.0))
            .child(self.header(cx))
            .children(dialog)
            .child(self.figures(p))
            .child(legend(p))
            .child(Self::section(INCOMING, &incoming, cx))
            .child(Self::section(OUTGOING, &outgoing, cx))
    }
}

fn dialog_title(title: &str) -> impl IntoElement {
    div()
        .text_size(px(text::TITLE))
        .font_weight(FontWeight::BOLD)
        .child(title.to_owned())
}

fn error_line(e: &str, p: &Palette) -> impl IntoElement {
    div()
        .text_size(px(text::BODY))
        .text_color(p.rejection_red.to_hsla())
        .child(e.to_owned())
}

/// A field label with `(optional)` muted after it.
fn optional_label(label: &str, p: &Palette) -> impl IntoElement {
    h_flex()
        .gap(px(4.0))
        .text_size(px(text::BODY))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
        .child(div().text_color(p.ink_muted.to_hsla()).child("(optional)"))
}

/// `OrgLine` (`chain-request-modal.tsx:238-249`).
fn org_line(org: &OrgOption, p: &Palette) -> impl IntoElement {
    v_flex()
        .min_w(px(0.0))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(org.name.clone()),
        )
        .child(small_muted(&org.line, p))
}

/// `ChainLegend` (`chain-status-badge.tsx:113-131`).
fn legend(p: &Palette) -> impl IntoElement {
    h_flex()
        .gap(px(24.0))
        .children(LEGEND.iter().map(|(status, label)| {
            h_flex()
                .gap(px(8.0))
                .child(chain_line(*status, 28.0, p))
                .child(small_muted(label, p))
        }))
}

/// `ChainLine` (`chain-status-badge.tsx:16-77`): the pattern carries the state. Active is a
/// solid ink line, pending dashed 7 5 in muted ink, revoked or rejected broken at 42% to 58%
/// with two cut marks.
fn chain_line(status: ChainStatus, width: f32, p: &Palette) -> Div {
    let bar = |w: f32, color| div().w(px(w)).h(px(3.0)).bg(color);
    let muted = p.ink_muted.to_hsla();
    match status {
        ChainStatus::Active => h_flex()
            .h(px(12.0))
            .items_center()
            .child(bar(width, p.ink_navy.to_hsla())),
        ChainStatus::Pending => {
            let mut row = h_flex().h(px(12.0)).w(px(width)).items_center();
            let mut at = 0.0;
            while at < width {
                let len = f32::min(7.0, width - at);
                row = row.child(bar(len, muted));
                at += len;
                if at < width {
                    let gap = f32::min(5.0, width - at);
                    row = row.child(div().w(px(gap)));
                    at += gap;
                }
            }
            row
        }
        ChainStatus::Revoked | ChainStatus::Rejected => {
            let cut = |rotate: bool| {
                div()
                    .w(px(2.0))
                    .h(px(12.0))
                    .bg(muted)
                    .when(rotate, |d| d.ml(px(width * 0.02)))
            };
            h_flex()
                .h(px(12.0))
                .w(px(width))
                .items_center()
                .child(bar(width * 0.42, muted))
                .child(div().w(px(width * 0.05)))
                .child(cut(false))
                .child(cut(true))
                .child(div().flex_1())
                .child(bar(width * 0.42, muted))
        }
    }
}
