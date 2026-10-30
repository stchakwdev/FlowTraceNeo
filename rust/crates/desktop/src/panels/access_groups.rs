//! The Access groups screen (DT4): "What each group can see and do", the selected group's
//! members with "Add a member" and "Invite someone", and the create, edit, delete and remove
//! dialogs, as the web's `routes/_authenticated/settings/access-groups.tsx` lays them out.
//! Rows, labels, checks and texts are [`crate::access_groups`]'s; this file draws and sends.
//!
//! Dialogs are inline boxes with the ink rule, as on the desktop's other screens. Keys: `n`
//! create, `e` edit the selected group, `d` delete it, up and down move the selection,
//! Escape closes a dialog.

mod form;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Div, Entity, EventEmitter, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px, relative,
};
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::OrganizationId;
use ledger_module::transparency::chains::ChainStatus;
use ledger_module::transparency::types::{AccessGroup, GroupMember, NewGroupMember};

use crate::access_groups::{
    COLUMNS, CREATE_STANDARD, Cell, DELETE_TEXT, EMAIL_PLACEHOLDER, FOOTNOTE, GroupForm, GroupRow,
    InviteErrors, LEGEND, MemberRow, NO_GROUPS, NO_MEMBERS, Person, REMOVE_TEXT, Term, UNTIL_HINT,
    addable, check_invite, delete_title, in_order, invited, lede, member_rows, members_meta,
    remove_title, rows,
};
use crate::backend::{SettingsWrite, iso_millis};
use crate::model::NavEntry;
use crate::panels::bills::{new_pick, picked};
use crate::panels::chains::chain_line;
use crate::panels::dashboard::{notice, small_muted};
use crate::panels::trust_score::OpenScreen;
use crate::panels::{Pick, PickState, demo_tag, field, page_title, section_header, write_settings};
use crate::statements::today_utc;
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// An open dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Dialog {
    /// Create or edit a group.
    Group,
    /// Delete a custom group: its id and name.
    Delete(String, String),
    /// Remove a membership: its id and the person's name.
    Remove(String, String),
}

/// The Access groups screen.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the web page's separate form and request flags"
)]
pub struct AccessGroupsPanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    org: Option<OrganizationId>,
    org_name: Option<String>,
    demo: bool,
    loaded: bool,
    groups: Vec<AccessGroup>,
    rows: Vec<GroupRow>,
    members: Vec<GroupMember>,
    people: Vec<Person>,
    selected: Option<String>,
    dialog: Option<Dialog>,
    /// A write is on its way.
    pending: bool,
    /// The open dialog's refusal.
    dialog_error: Option<String>,
    defaults_error: Option<String>,
    group: GroupForm,
    group_submitted: bool,
    group_name: Entity<InputState>,
    group_slug: Entity<InputState>,
    group_description: Entity<InputState>,
    scope_pick: PickState,
    proofs_pick: PickState,
    email: Entity<InputState>,
    until: Entity<InputState>,
    invite_touched: bool,
    invite_error: Option<String>,
    last_invited: Option<String>,
    person_pick: PickState,
    add_note: Option<(bool, String)>,
    /// An invite went out: the fields empty on the next draw.
    clear_invite: bool,
    /// A group just created, to select once its row arrives: its short id.
    select_slug: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenScreen> for AccessGroupsPanel {}

impl AccessGroupsPanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let group_name = input(crate::access_groups::NAME_PLACEHOLDER, window, cx);
        let group_slug = input("", window, cx);
        let group_description = input(crate::access_groups::DESCRIPTION_PLACEHOLDER, window, cx);
        let email = input(EMAIL_PLACEHOLDER, window, cx);
        let until = input("dd/mm/yyyy", window, cx);
        let scope_pick = new_pick(window, cx, Vec::new(), None);
        let proofs_pick = new_pick(window, cx, Vec::new(), None);
        let person_pick = new_pick(window, cx, Vec::new(), None);
        let mut subscriptions = vec![cx.observe_in(&store, window, |this, _, window, cx| {
            this.refresh(window, cx);
        })];
        subscriptions.extend(form::subscribe(
            &group_name,
            &group_slug,
            &scope_pick,
            &proofs_pick,
            &email,
            &until,
            window,
            cx,
        ));
        let mut panel = Self {
            store,
            seen: None,
            org: None,
            org_name: None,
            demo: false,
            loaded: false,
            groups: Vec::new(),
            rows: Vec::new(),
            members: Vec::new(),
            people: Vec::new(),
            selected: None,
            dialog: None,
            pending: false,
            dialog_error: None,
            defaults_error: None,
            group: GroupForm::new(),
            group_submitted: false,
            group_name,
            group_slug,
            group_description,
            scope_pick,
            proofs_pick,
            email,
            until,
            invite_touched: false,
            invite_error: None,
            last_invited: None,
            person_pick,
            add_note: None,
            clear_invite: false,
            select_slug: None,
            _subscriptions: subscriptions,
        };
        form::load_choices(&panel.scope_pick, &panel.proofs_pick, window, cx);
        panel.refresh(window, cx);
        panel
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.transparency_revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        if self.org != org {
            self.selected = None;
            self.dialog = None;
            self.defaults_error = None;
            self.reset_invite(window, cx);
        }
        let store = self.store.read(cx);
        let view = store.view();
        self.org.clone_from(&org);
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        let records = view.transparency().cloned();
        self.loaded = records.is_some();
        self.org_name = records
            .as_ref()
            .and_then(|t| t.profile.as_ref().map(|p| p.name.clone()));
        self.people = store.org_members();
        let t = records.unwrap_or_default();
        self.groups = in_order(&t.access_groups).into_iter().cloned().collect();
        self.members.clone_from(&t.group_members);
        self.rows = rows(&self.groups, &self.members);
        if let Some(slug) = self.select_slug.take() {
            match self.groups.iter().find(|g| g.slug == slug) {
                Some(g) => self.selected = Some(g.id.clone()),
                None => self.select_slug = Some(slug),
            }
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|id| !self.groups.iter().any(|g| &g.id == id))
        {
            self.selected = None;
        }
        self.load_people(window, cx);
        cx.notify();
    }

    /// The selected group: the one picked, else the first (`selected`).
    fn selected_group(&self) -> Option<&AccessGroup> {
        self.selected
            .as_ref()
            .and_then(|id| self.groups.iter().find(|g| &g.id == id))
            .or_else(|| self.groups.first())
    }

    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.as_ref() != Some(&id) {
            self.selected = Some(id);
            self.reset_invite(window, cx);
            self.load_people(window, cx);
        }
        cx.notify();
    }

    /// The invite and add forms start over for another group (`useEffect` on `group.id`).
    fn reset_invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.email.update(cx, |i, cx| i.set_value("", window, cx));
        self.until.update(cx, |i, cx| i.set_value("", window, cx));
        self.invite_touched = false;
        self.invite_error = None;
        self.last_invited = None;
        self.add_note = None;
    }

    /// The Person select: the organisation's members not yet in the group.
    fn load_people(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(group) = self.selected_group().map(|g| g.id.clone()) else {
            return;
        };
        let open: Vec<Pick> = addable(&self.people, &self.members, &group)
            .into_iter()
            .map(|p| Pick::new(p.user_id.clone(), p.name.clone()))
            .collect();
        let first = open.first().map(|p| p.value.clone());
        self.person_pick.update(cx, |s, cx| {
            s.set_items(open, window, cx);
            if let Some(v) = &first {
                s.set_selected_value(v, window, cx);
            }
        });
    }

    /// Sends `write`; `then` runs with the result.
    fn send(
        &mut self,
        write: SettingsWrite,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, Result<(), String>, &mut Context<Self>) + 'static,
    ) {
        if self.pending {
            return;
        }
        self.pending = true;
        let store = self.store.clone();
        write_settings(self, &store, write, cx, move |this, result, cx| {
            this.pending = false;
            then(this, result.map(drop).map_err(|e| e.to_string()), cx);
            cx.notify();
        });
        cx.notify();
    }

    fn create_defaults(&mut self, cx: &mut Context<Self>) {
        self.defaults_error = None;
        self.send(SettingsWrite::CreateDefaultGroups, cx, |this, r, _| {
            this.defaults_error = r.err();
        });
    }

    fn open_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.dialog = Some(dialog);
        self.dialog_error = None;
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        self.dialog_error = None;
        cx.notify();
    }

    fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        self.send(
            SettingsWrite::DeleteGroup { id: id.clone() },
            cx,
            move |this, r, _| match r {
                Ok(()) => {
                    if this.selected.as_ref() == Some(&id) {
                        this.selected = None;
                    }
                    this.dialog = None;
                }
                Err(e) => this.dialog_error = Some(format!("Not deleted. {e}")),
            },
        );
    }

    fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        self.send(
            SettingsWrite::RemoveMember { id },
            cx,
            |this, r, _| match r {
                Ok(()) => this.dialog = None,
                Err(e) => this.dialog_error = Some(format!("Not removed. {e}")),
            },
        );
    }

    fn invite(&mut self, cx: &mut Context<Self>) {
        self.invite_touched = true;
        let Some(group) = self.selected_group().cloned() else {
            return;
        };
        let email = self.email.read(cx).value().to_string();
        let until = self.until.read(cx).value().to_string();
        let Ok(invite) = check_invite(&email, &until, &self.members, &group.id, &today_utc())
        else {
            cx.notify();
            return;
        };
        self.invite_error = None;
        let address = invite.email.clone();
        self.send(
            SettingsWrite::InviteMember {
                group: group.id,
                email: invite.email,
                valid_until: invite.valid_until,
            },
            cx,
            move |this, r, _| match r {
                Ok(()) => {
                    this.invite_touched = false;
                    this.last_invited = Some(address);
                    this.clear_invite = true;
                }
                Err(e) => this.invite_error = Some(format!("Invite not sent. {e}")),
            },
        );
    }

    fn add_member(&mut self, cx: &mut Context<Self>) {
        let Some(group) = self.selected_group().cloned() else {
            return;
        };
        let Some(user_id) = picked(&self.person_pick, cx) else {
            return;
        };
        let now = iso_millis(form::now_ms());
        let member = NewGroupMember {
            group_id: group.id.clone(),
            user_id: Some(user_id),
            user_email: None,
            valid_from: Some(now),
            valid_until: None,
            notes: None,
        };
        let name = group.name;
        self.send(SettingsWrite::AddMember(member), cx, move |this, r, _| {
            this.add_note = Some(match r {
                Ok(()) => (false, format!("Added to {name}.")),
                Err(e) => (true, format!("Not added. {e}")),
            });
        });
    }

    /// Keys while the screen shows and no field has focus.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "escape" if self.dialog.is_some() => self.close(cx),
            "n" if self.dialog.is_none() => self.open_group_form(None, window, cx),
            "e" if self.dialog.is_none() => {
                let g = self.selected_group().cloned();
                if let Some(g) = g {
                    self.open_group_form(Some(&g), window, cx);
                }
            }
            "d" if self.dialog.is_none() => {
                let g = self.selected_group().filter(|g| !g.is_default).cloned();
                if let Some(g) = g {
                    self.open_dialog(Dialog::Delete(g.id, g.name), cx);
                }
            }
            "up" | "down" if self.dialog.is_none() && !self.groups.is_empty() => {
                let at = self
                    .selected_group()
                    .and_then(|g| self.groups.iter().position(|x| x.id == g.id))
                    .unwrap_or(0);
                let next = if key == "up" {
                    at.saturating_sub(1)
                } else {
                    (at + 1).min(self.groups.len() - 1)
                };
                let id = self.groups[next].id.clone();
                self.select(id, window, cx);
            }
            _ => return false,
        }
        true
    }
}

impl Render for AccessGroupsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.clear_invite) {
            self.email.update(cx, |i, cx| i.set_value("", window, cx));
            self.until.update(cx, |i, cx| i.set_value("", window, cx));
        }
        let p = palette(cx.theme().mode);
        let back = Button::new("groups-back-to-settings")
            .ghost()
            .label("\u{2190} Settings")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                cx.emit(OpenScreen(NavEntry::Settings));
            }));
        let create = Button::new("create-group")
            .primary()
            .label("Create group")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.open_group_form(None, window, cx);
            }));
        let header = v_flex()
            .gap(px(8.0))
            .child(div().child(back))
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .child(page_title("Access groups"))
                    .child(create),
            )
            .child(
                h_flex()
                    .gap(px(12.0))
                    .text_size(px(text::BODY))
                    .text_color(p.ink_muted.to_hsla())
                    .child(lede(self.org_name.as_deref()))
                    .when(self.demo, |m| m.child(demo_tag(cx))),
            );
        let dialog = self.dialog_box(cx);
        let members = self.members_section(cx);
        v_flex()
            .gap(px(space::SECTION))
            .child(header)
            .when_some(dialog, ParentElement::child)
            .child(self.groups_section(cx))
            .when_some(members, ParentElement::child)
    }
}

impl AccessGroupsPanel {
    fn groups_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let meta = if self.loaded {
            small_muted(&format!("{} groups", self.rows.len()), p)
        } else {
            div()
        };
        let body = if !self.loaded {
            notice(
                "Access groups could not be loaded",
                "Check your connection and try again.",
                p,
            )
            .into_any_element()
        } else if self.rows.is_empty() {
            self.empty_state(cx)
        } else {
            let selected = self.selected_group().map(|g| g.id.clone());
            let rows: Vec<AnyElement> = self
                .rows
                .iter()
                .map(|r| Self::group_row(r, selected.as_deref() == Some(r.id.as_str()), cx))
                .collect();
            v_flex()
                .child(table_head(p))
                .children(rows)
                .child(small_muted(FOOTNOTE, p).mt(px(12.0)))
                .into_any_element()
        };
        v_flex()
            .child(section_header("What each group can see and do", meta, cx))
            .child(body)
            .into_any_element()
    }

    fn empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .child(notice(NO_GROUPS.0, NO_GROUPS.1, p))
            .when_some(self.defaults_error.clone(), |d, e| {
                d.child(
                    div()
                        .pb(px(12.0))
                        .text_size(px(text::BODY))
                        .text_color(p.rejection_red.to_hsla())
                        .child(e),
                )
            })
            .child(
                h_flex()
                    .gap(px(12.0))
                    .child(
                        Button::new("create-standard-groups")
                            .primary()
                            .label(CREATE_STANDARD)
                            .disabled(self.pending)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.create_defaults(cx);
                            })),
                    )
                    .child(
                        Button::new("create-group-empty")
                            .outline()
                            .label("Create group")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.open_group_form(None, window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn group_row(r: &GroupRow, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        let pick = id.clone();
        let edit_id = id.clone();
        let name = r.name.clone();
        let cell = |c: Cell| match c {
            Cell::Yes => div().child("\u{2713}"),
            Cell::No => div().text_color(p.ink_muted.to_hsla()).child("\u{2013}"),
            Cell::Text(t) => div().child(t),
        };
        let edit = Button::new(SharedString::from(format!("edit-{id}")))
            .ghost()
            .label("Edit")
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                let g = this.groups.iter().find(|g| g.id == edit_id).cloned();
                if let Some(g) = g {
                    this.open_group_form(Some(&g), window, cx);
                }
            }));
        let delete = r.deletable.then(|| {
            let del_id = id.clone();
            Button::new(SharedString::from(format!("delete-{id}")))
                .ghost()
                .label("Delete")
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.open_dialog(Dialog::Delete(del_id.clone(), name.clone()), cx);
                }))
        });
        h_flex()
            .id(SharedString::from(format!("group-{id}")))
            .py(px(12.0))
            .gap(px(12.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .text_size(px(text::BODY))
            .when(selected, |row| row.bg(p.wash.to_hsla()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.select(pick.clone(), window, cx);
            }))
            .child(
                v_flex()
                    .w(px(200.0))
                    .flex_shrink_0()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(selected, |d| d.text_color(p.report_blue.to_hsla()))
                            .child(r.name.clone()),
                    )
                    .child(small_muted(&r.kind, p)),
            )
            .child(div().w(px(180.0)).flex_shrink_0().child(r.sees.clone()))
            .child(cell(r.amounts).w(px(72.0)))
            .child(cell(r.proofs).w(px(88.0)))
            .child(cell(r.executor).w(px(96.0)))
            .child(cell(r.edits_flows).w(px(80.0)))
            .child(cell(r.edits_transactions).w(px(110.0)))
            .child(cell(r.manages).w(px(140.0)))
            .child(
                div()
                    .w(px(64.0))
                    .flex()
                    .justify_end()
                    .child(r.members.to_string()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .justify_end()
                    .gap(px(4.0))
                    .child(edit)
                    .when_some(delete, ParentElement::child),
            )
            .into_any_element()
    }

    fn members_section(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = palette(cx.theme().mode);
        let group = self.selected_group()?.clone();
        let now = iso_millis(form::now_ms());
        let list = member_rows(&self.members, &group.id, &self.people, &now);
        let rows: Vec<AnyElement> = list.iter().map(|m| Self::member_row(m, cx)).collect();
        let body = if rows.is_empty() {
            notice(NO_MEMBERS.0, NO_MEMBERS.1, p).into_any_element()
        } else {
            v_flex().children(rows).into_any_element()
        };
        let legend = h_flex()
            .mt(px(16.0))
            .gap(px(24.0))
            .children(LEGEND.iter().map(|(term, label)| {
                h_flex()
                    .gap(px(8.0))
                    .child(chain_line(term_status(*term), 28.0, p))
                    .child(small_muted(label, p))
            }));
        Some(
            v_flex()
                .child(section_header(
                    &format!("Members of {}", group.name),
                    small_muted(&members_meta(list.len()), p),
                    cx,
                ))
                .child(
                    h_flex()
                        .pt(px(8.0))
                        .gap(px(space::SECTION))
                        .items_start()
                        .child(
                            v_flex()
                                .flex_basis(relative(7.0 / 12.0))
                                .min_w(px(0.0))
                                .child(body)
                                .child(legend),
                        )
                        .child(
                            v_flex()
                                .flex_basis(relative(5.0 / 12.0))
                                .min_w(px(0.0))
                                .gap(px(32.0))
                                .pl(px(32.0))
                                .border_l(px(space::HAIRLINE))
                                .border_color(p.hairline.to_hsla())
                                .when_some(self.add_form(&group, cx), ParentElement::child)
                                .child(self.invite_form(&group, cx)),
                        ),
                )
                .into_any_element(),
        )
    }

    fn member_row(m: &MemberRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let expired = m.term == Term::Expired;
        let id = m.id.clone();
        let name = m.name.clone();
        h_flex()
            .py(px(12.0))
            .justify_between()
            .gap(px(16.0))
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .when(expired, |r| r.text_color(p.ink_muted.to_hsla()))
            .child(
                v_flex()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(m.name.clone()),
                    )
                    .child(small_muted(&m.sub, p))
                    .when_some(m.notes.clone(), |d, n| d.child(small_muted(&n, p))),
            )
            .child(
                h_flex()
                    .gap(px(12.0))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(m.term != Term::Open, |d| {
                                d.text_color(p.ink_muted.to_hsla())
                            })
                            .child(chain_line(term_status(m.term), 24.0, p))
                            .child(m.chip.clone()),
                    )
                    .child(
                        Button::new(SharedString::from(format!("remove-{id}")))
                            .ghost()
                            .label("Remove")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.open_dialog(Dialog::Remove(id.clone(), name.clone()), cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn add_form(&self, group: &AccessGroup, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = palette(cx.theme().mode);
        if self.people.is_empty() {
            return None;
        }
        let open = addable(&self.people, &self.members, &group.id);
        let body = if open.is_empty() {
            small_muted(
                &format!(
                    "Everyone in your organisation is already in {}.",
                    group.name
                ),
                p,
            )
            .text_size(px(text::BODY))
            .into_any_element()
        } else {
            v_flex()
                .gap(px(16.0))
                .child(field("Person", Select::new(&self.person_pick), None, cx))
                .child(
                    div().child(
                        Button::new("add-member")
                            .primary()
                            .label(if self.pending {
                                "Adding".to_owned()
                            } else {
                                format!("Add to {}", group.name)
                            })
                            .disabled(self.pending)
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _, cx| this.add_member(cx)),
                            ),
                    ),
                )
                .into_any_element()
        };
        Some(
            v_flex()
                .gap(px(16.0))
                .child(form_title("Add a member"))
                .child(body)
                .when_some(self.add_note.clone(), |d, (error, note)| {
                    d.child(
                        div()
                            .text_size(px(text::BODY))
                            .text_color(if error {
                                p.rejection_red.to_hsla()
                            } else {
                                p.ink_muted.to_hsla()
                            })
                            .child(note),
                    )
                })
                .into_any_element(),
        )
    }

    fn invite_form(&self, group: &AccessGroup, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let errors = if self.invite_touched {
            check_invite(
                &self.email.read(cx).value(),
                &self.until.read(cx).value(),
                &self.members,
                &group.id,
                &today_utc(),
            )
            .err()
            .unwrap_or_default()
        } else {
            InviteErrors::default()
        };
        let until_line = errors.until.clone().map_or_else(
            || small_muted(UNTIL_HINT, p),
            |e| {
                div()
                    .text_size(px(12.0))
                    .text_color(p.rejection_red.to_hsla())
                    .child(e)
            },
        );
        let note = match (&self.invite_error, &self.last_invited) {
            (Some(e), _) => Some((true, e.clone())),
            (None, Some(email)) => Some((false, invited(email, &group.name))),
            _ => None,
        };
        v_flex()
            .gap(px(16.0))
            .child(form_title("Invite someone"))
            .child(field(
                "Email",
                Input::new(&self.email),
                errors.email.map(str::to_owned),
                cx,
            ))
            .child(
                v_flex()
                    .gap(px(6.0))
                    .child(
                        h_flex()
                            .gap(px(4.0))
                            .text_size(px(text::BODY))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Access ends"))
                            .child(div().text_color(p.ink_muted.to_hsla()).child("(optional)")),
                    )
                    .child(div().w(px(224.0)).child(Input::new(&self.until)))
                    .child(until_line),
            )
            .child(
                div().child(
                    Button::new("invite-member")
                        .primary()
                        .label(if self.pending {
                            "Inviting".to_owned()
                        } else {
                            format!("Invite to {}", group.name)
                        })
                        .disabled(self.pending)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.invite(cx))),
                ),
            )
            .when_some(note, |d, (error, note)| {
                d.child(
                    div()
                        .text_size(px(text::BODY))
                        .text_color(if error {
                            p.rejection_red.to_hsla()
                        } else {
                            p.ink_muted.to_hsla()
                        })
                        .child(note),
                )
            })
            .into_any_element()
    }

    /// The open dialog as an inline box with the 3 px ink rule (`ReportDialog`,
    /// `AlertDialog`).
    fn dialog_box(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.dialog.clone()?;
        let p = palette(cx.theme().mode);
        let inner = match dialog {
            Dialog::Group => self.group_dialog(cx),
            Dialog::Delete(id, name) => self.confirm(
                &delete_title(&name),
                DELETE_TEXT,
                ("Keep group", "Delete group"),
                move |this, cx| this.delete(id.clone(), cx),
                cx,
            ),
            Dialog::Remove(id, name) => {
                let group = self
                    .selected_group()
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                self.confirm(
                    &remove_title(&name, &group),
                    REMOVE_TEXT,
                    ("Keep member", "Remove member"),
                    move |this, cx| this.remove(id.clone(), cx),
                    cx,
                )
            }
        };
        Some(
            v_flex()
                .id("access-dialog")
                .max_w(px(720.0))
                .p(px(24.0))
                .border_1()
                .border_t(px(space::RULE_HEAVY))
                .border_color(p.ink_navy.to_hsla())
                .bg(p.sheet.to_hsla())
                .child(inner)
                .into_any_element(),
        )
    }

    fn confirm(
        &self,
        title: &str,
        body: &str,
        (keep, go): (&'static str, &'static str),
        then: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .gap(px(12.0))
            .child(dialog_title(title))
            .child(div().text_size(px(text::BODY)).child(body.to_owned()))
            .when_some(self.dialog_error.clone(), |d, e| d.child(error_line(&e, p)))
            .child(
                h_flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::new("confirm-keep")
                            .outline()
                            .label(keep)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close(cx))),
                    )
                    .child(
                        Button::new("confirm-go")
                            .danger()
                            .label(go)
                            .disabled(self.pending)
                            .on_click(
                                cx.listener(move |this, _: &ClickEvent, _, cx| then(this, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// The chain line a membership term is drawn with (`ChainLine` status).
const fn term_status(term: Term) -> ChainStatus {
    match term {
        Term::Open => ChainStatus::Active,
        Term::Limited => ChainStatus::Pending,
        Term::Expired => ChainStatus::Revoked,
    }
}

fn table_head(p: &Palette) -> Div {
    let head = |label: &'static str, w: f32| {
        div()
            .w(px(w))
            .flex_shrink_0()
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink_muted.to_hsla())
            .child(label)
    };
    let widths = [200.0, 180.0, 72.0, 88.0, 96.0, 80.0, 110.0, 140.0, 64.0];
    h_flex()
        .py(px(12.0))
        .gap(px(12.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .children(
            COLUMNS
                .iter()
                .zip(widths)
                .enumerate()
                .map(|(i, (label, w))| {
                    let h = head(label, w);
                    if i + 1 == COLUMNS.len() {
                        h.flex().justify_end()
                    } else {
                        h
                    }
                }),
        )
}

fn form_title(title: &str) -> impl IntoElement {
    div()
        .text_size(px(20.0))
        .font_weight(FontWeight::BOLD)
        .child(title.to_owned())
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
