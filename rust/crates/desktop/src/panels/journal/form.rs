//! The Journal screen's manual entry form and CSV journal import: their widgets, the actions
//! that send `entry.saveDraft`, `entry.updateDraft`, `entry.post`, `entry.postDraft` and
//! `entry.import`, and the drawing. The form's state and rules are
//! [`crate::entry_form`]'s; this file copies typed text in and draws what it derives. The open
//! dialog is GPUI's `App::prompt_for_paths` (the platform file chooser, the XDG desktop portal on
//! Linux); the path field works without it.

use std::path::PathBuf;

use gpui_kit::component::IndexPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight, IntoElement,
    ParentElement as _, PathPromptOptions, SharedString, Styled as _, Subscription, Window, div,
    px,
};
use ledger_core::LedgerState;
use ledger_core::types::{EntryId, GrantId};

use super::{Items, JournalPanel, JournalView};
use crate::accounts::{Failure, format_minor};
use crate::claims::{fund_choices, grant_choices, has_overridable};
use crate::entry_form::{
    EntryForm, ImportPreview, LineField, Side, account_choices, import_done, read_import_file,
    tag_choices,
};
use crate::journal::created_entry;
use crate::panels::{failure_box, field, hint, section_header};
use crate::statements::today_utc;
use crate::theme::{palette, space, text};

const NO_GRANT: &str = "No grant";
const NO_TAG: &str = "None";

/// The widgets of one line of the entry form, keyed by the line's stable key.
pub(super) struct LineWidgets {
    key: u64,
    account: Entity<SelectState<Items>>,
    fund: Entity<SelectState<Items>>,
    grant: Entity<SelectState<Items>>,
    tag: Entity<SelectState<Items>>,
    memo: Entity<InputState>,
    debit: Entity<InputState>,
    credit: Entity<InputState>,
    /// The grants the grant picker lists, after `No grant`.
    grants: Vec<(GrantId, String)>,
    _subscriptions: Vec<Subscription>,
}

/// Which picker of a line changed.
#[derive(Debug, Clone, Copy)]
enum LinePick {
    Account,
    Fund,
    Grant,
    Tag,
}

/// The form's and the import's own widgets.
pub(super) struct Widgets {
    pub(super) entry_date: Entity<InputState>,
    pub(super) entry_memo: Entity<InputState>,
    pub(super) form_override: Entity<InputState>,
    pub(super) import_path: Entity<InputState>,
    pub(super) import_fund: Entity<SelectState<Items>>,
}

/// Builds [`Widgets`].
pub(super) fn widgets(window: &mut Window, cx: &mut Context<JournalPanel>) -> Widgets {
    Widgets {
        entry_date: cx.new(|cx| InputState::new(window, cx).placeholder("dd/mm/yyyy")),
        entry_memo: cx
            .new(|cx| InputState::new(window, cx).placeholder("e.g. Accrue December audit fee")),
        form_override: cx.new(|cx| InputState::new(window, cx)),
        import_path: cx
            .new(|cx| InputState::new(window, cx).placeholder("~/Downloads/journal.csv")),
        import_fund: cx.new(|cx| SelectState::new(Items::new(), None, window, cx)),
    }
}

/// The subscriptions of the form's and the import's own widgets: Enter moves from the date to
/// the memo, Enter in the override reason posts with it, Enter in the path loads the file, and
/// the fund picker re-reads the file.
pub(super) fn subscriptions(
    w: &Widgets,
    window: &mut Window,
    cx: &mut Context<JournalPanel>,
) -> Vec<Subscription> {
    let (date, memo, override_reason, path, fund) = (
        &w.entry_date,
        &w.entry_memo,
        &w.form_override,
        &w.import_path,
        &w.import_fund,
    );
    let enter = |e: &InputEvent| matches!(e, InputEvent::PressEnter { .. });
    vec![
        cx.subscribe_in(date, window, move |this, _, e: &InputEvent, window, cx| {
            if enter(e) {
                this.entry_memo.update(cx, |i, cx| i.focus(window, cx));
            }
        }),
        cx.subscribe_in(memo, window, move |this, _, e: &InputEvent, window, cx| {
            if enter(e)
                && let Some(first) = this.form_lines.first()
            {
                first.debit.update(cx, |i, cx| i.focus(window, cx));
            }
        }),
        cx.subscribe_in(
            override_reason,
            window,
            move |this, _, e: &InputEvent, window, cx| {
                if enter(e) {
                    this.post_form(true, window, cx);
                }
            },
        ),
        cx.subscribe_in(path, window, move |this, _, e: &InputEvent, _, cx| {
            if enter(e) {
                this.load_import(cx);
            }
        }),
        cx.subscribe_in(fund, window, |this, _, e: &SelectEvent<Items>, _, cx| {
            let SelectEvent::Confirm(label) = e;
            this.import_fund_id = this
                .books
                .as_deref()
                .and_then(|b| chosen(fund_choices(b), label.as_ref().map(SharedString::as_ref)));
            this.import_created = None;
            this.derive();
            cx.notify();
        }),
    ]
}

/// The id whose label is `label`.
fn chosen<T>(list: Vec<(T, String)>, label: Option<&str>) -> Option<T> {
    let label = label?;
    list.into_iter().find(|(_, n)| n == label).map(|(id, _)| id)
}

/// The labels of `list`, after `none` when given.
fn items<T>(none: Option<&str>, list: &[(T, String)]) -> Items {
    none.map(SharedString::from)
        .into_iter()
        .chain(list.iter().map(|(_, n)| SharedString::from(n.clone())))
        .collect()
}

/// The index of `id` in `list`, after `offset` leading choices.
fn index_of<T: PartialEq>(
    list: &[(T, String)],
    id: Option<&T>,
    offset: usize,
) -> Option<IndexPath> {
    let id = id?;
    list.iter()
        .position(|(x, _)| x == id)
        .map(|i| IndexPath::new(i + offset))
}

impl JournalPanel {
    // The entry form.

    /// Opens the form on a new entry, or on the draft `draft` (`new.tsx`, `?draft=`).
    pub(super) fn open_form(
        &mut self,
        draft: Option<&EntryId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let as_of = self.as_of.clone().unwrap_or_else(today_utc);
        let form = match draft {
            None => EntryForm::new(&books, &as_of),
            Some(id) => match EntryForm::from_draft(&books, id) {
                Ok(form) => form,
                Err(notice) => {
                    self.action_failure = Some(Failure {
                        title: notice.title,
                        messages: vec![notice.body],
                    });
                    return;
                }
            },
        };
        self.view = JournalView::Form;
        self.done = None;
        self.pending = None;
        self.form_submitted = false;
        self.form_failure = None;
        self.form_needs_override = false;
        self.form_override_error = None;
        let (date, memo, reason) = (
            form.date.clone(),
            form.memo.clone(),
            form.override_reason.clone(),
        );
        self.entry_date
            .update(cx, |i, cx| i.set_value(date, window, cx));
        self.entry_memo
            .update(cx, |i, cx| i.set_value(memo, window, cx));
        self.form_override
            .update(cx, |i, cx| i.set_value(reason, window, cx));
        self.form_lines.clear();
        for line in &form.lines {
            let w = Self::line_widgets(&books, line, window, cx);
            self.form_lines.push(w);
        }
        self.form = Some(form);
        self.entry_memo.update(cx, |i, cx| i.focus(window, cx));
    }

    /// Cancel: back to the draft, or to the list (`new.tsx`, the Cancel link).
    pub(super) fn leave_form(&mut self) {
        let draft = self.form.as_ref().and_then(|f| f.draft.clone());
        self.form = None;
        self.form_lines.clear();
        match draft {
            Some(id) => {
                self.view = JournalView::Detail(id);
                self.derive();
            }
            None => self.back_to_list(),
        }
    }

    /// The widgets of a new line, filled from `line`.
    fn line_widgets(
        books: &LedgerState,
        line: &crate::entry_form::EntryLineDraft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> LineWidgets {
        let key = line.key;
        let accounts = account_choices(books);
        let funds = fund_choices(books);
        let grants = grant_choices(books, line.fund.as_ref());
        let tags = tag_choices(books);
        let none = Some(IndexPath::new(0));
        let select =
            |items: Items, sel: Option<IndexPath>, window: &mut Window, cx: &mut Context<Self>| {
                cx.new(|cx| SelectState::new(items, sel, window, cx))
            };
        let account = select(
            items(None, &accounts),
            index_of(&accounts, line.account.as_ref(), 0),
            window,
            cx,
        );
        let fund = select(
            items(None, &funds),
            index_of(&funds, line.fund.as_ref(), 0),
            window,
            cx,
        );
        let grant = select(
            items(Some(NO_GRANT), &grants),
            index_of(&grants, line.grant.as_ref(), 1).or(none),
            window,
            cx,
        );
        let tag = select(
            items(Some(NO_TAG), &tags),
            index_of(&tags, line.dimension.as_ref(), 1).or(none),
            window,
            cx,
        );
        let input =
            |value: &str, placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
                let (value, placeholder) = (value.to_owned(), placeholder.to_owned());
                cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(placeholder)
                        .default_value(value)
                })
            };
        let memo = input(&line.memo, "", window, cx);
        let debit = input(&line.debit, "0.00", window, cx);
        let credit = input(&line.credit, "0.00", window, cx);
        let subscriptions = vec![
            Self::pick(&account, key, LinePick::Account, window, cx),
            Self::pick(&fund, key, LinePick::Fund, window, cx),
            Self::pick(&grant, key, LinePick::Grant, window, cx),
            Self::pick(&tag, key, LinePick::Tag, window, cx),
            Self::amount(&debit, key, Side::Debit, window, cx),
            Self::amount(&credit, key, Side::Credit, window, cx),
        ];
        LineWidgets {
            key,
            account,
            fund,
            grant,
            tag,
            memo,
            debit,
            credit,
            grants,
            _subscriptions: subscriptions,
        }
    }

    fn pick(
        select: &Entity<SelectState<Items>>,
        key: u64,
        which: LinePick,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            select,
            window,
            move |this, _, e: &SelectEvent<Items>, window, cx| {
                let SelectEvent::Confirm(label) = e;
                this.on_pick(
                    key,
                    which,
                    label.as_ref().map(SharedString::as_ref),
                    window,
                    cx,
                );
            },
        )
    }

    /// A debit or credit changed: record it and clear the other side's field when the form
    /// cleared it (`updateLine`), so the balance check follows every keystroke. Enter posts.
    fn amount(
        input: &Entity<InputState>,
        key: u64,
        side: Side,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            input,
            window,
            move |this, input, e: &InputEvent, window, cx| match e {
                InputEvent::Change => {
                    let value = input.read(cx).value().to_string();
                    let Some(index) = this.form_lines.iter().position(|w| w.key == key) else {
                        return;
                    };
                    let Some(form) = this.form.as_mut() else {
                        return;
                    };
                    let other = match side {
                        Side::Debit => {
                            form.set_debit(index, &value);
                            form.lines[index]
                                .credit
                                .is_empty()
                                .then(|| this.form_lines[index].credit.clone())
                        }
                        Side::Credit => {
                            form.set_credit(index, &value);
                            form.lines[index]
                                .debit
                                .is_empty()
                                .then(|| this.form_lines[index].debit.clone())
                        }
                    };
                    if let Some(other) = other
                        && !other.read(cx).value().is_empty()
                    {
                        other.update(cx, |i, cx| i.set_value("", window, cx));
                    }
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.post_form(false, window, cx),
                _ => {}
            },
        )
    }

    /// A line's picker changed: record the choice; a new fund re-lists the grants.
    fn on_pick(
        &mut self,
        key: u64,
        which: LinePick,
        label: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Some(books), Some(form)) = (self.books.clone(), self.form.as_mut()) else {
            return;
        };
        let Some(index) = self.form_lines.iter().position(|w| w.key == key) else {
            return;
        };
        match which {
            LinePick::Account => form.lines[index].account = chosen(account_choices(&books), label),
            LinePick::Grant => {
                form.lines[index].grant = chosen(self.form_lines[index].grants.clone(), label);
            }
            LinePick::Tag => form.lines[index].dimension = chosen(tag_choices(&books), label),
            LinePick::Fund => {
                form.set_fund(&books, index, chosen(fund_choices(&books), label));
                let line = &form.lines[index];
                let grants = grant_choices(&books, line.fund.as_ref());
                let selected =
                    index_of(&grants, line.grant.as_ref(), 1).unwrap_or(IndexPath::new(0));
                let list = items(Some(NO_GRANT), &grants);
                let w = &mut self.form_lines[index];
                w.grants = grants;
                w.grant.update(cx, |s, cx| {
                    s.set_items(list, window, cx);
                    s.set_selected_index(Some(selected), window, cx);
                });
            }
        }
        cx.notify();
    }

    /// Copies the typed text into the form.
    fn sync_form(&mut self, cx: &App) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.date = self.entry_date.read(cx).value().to_string();
        form.memo = self.entry_memo.read(cx).value().to_string();
        form.override_reason = self.form_override.read(cx).value().to_string();
        for (line, w) in form.lines.iter_mut().zip(&self.form_lines) {
            line.memo = w.memo.read(cx).value().to_string();
            line.debit = w.debit.read(cx).value().to_string();
            line.credit = w.credit.read(cx).value().to_string();
        }
    }

    fn add_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_form(cx);
        let (Some(books), Some(form)) = (self.books.clone(), self.form.as_mut()) else {
            return;
        };
        form.add_line(&books);
        let Some(line) = form.lines.last().cloned() else {
            return;
        };
        let w = Self::line_widgets(&books, &line, window, cx);
        self.form_lines.push(w);
        cx.notify();
    }

    fn remove_line(&mut self, key: u64, cx: &mut Context<Self>) {
        self.sync_form(cx);
        let Some(index) = self.form_lines.iter().position(|w| w.key == key) else {
            return;
        };
        if self.form.as_mut().is_some_and(|f| f.remove_line(index)) {
            self.form_lines.remove(index);
        }
        cx.notify();
    }

    /// The title of a refusal: `Not saved` on a draft, `Not saved or posted` on a new entry.
    fn save_failure_title(&self) -> &'static str {
        if self.form.as_ref().is_some_and(|f| f.draft.is_some()) {
            "Not saved"
        } else {
            "Not saved or posted"
        }
    }

    /// Save draft (`new.tsx`, `saveDraft`): opens the saved draft.
    fn save_form(&mut self, cx: &mut Context<Self>) {
        self.sync_form(cx);
        self.form_submitted = true;
        self.form_failure = None;
        let Some(form) = self.form.clone() else {
            return;
        };
        let Ok(command) = form.save_command() else {
            cx.notify();
            return;
        };
        match self.submit(command, cx) {
            Ok(result) => {
                let id = created_entry(&result).or(form.draft);
                self.finish_form(
                    id,
                    "Saved as a draft. It is not in the books until posted.",
                    cx,
                );
            }
            Err(err) => self.form_failure = Some(Failure::new(self.save_failure_title(), &err)),
        }
        cx.notify();
    }

    /// Post (`new.tsx`, `post`): checks the entry first; a draft is saved, then posted.
    pub(super) fn post_form(
        &mut self,
        with_override: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_form(cx);
        self.form_submitted = true;
        self.form_failure = None;
        let (Some(books), Some(form)) = (self.books.clone(), self.form.clone()) else {
            return;
        };
        match form.post_commands(&books, with_override) {
            Err(blocked) => {
                self.form_needs_override |= blocked.needs_override;
                self.form_override_error = blocked.override_error;
            }
            Ok(commands) => {
                self.form_override_error = None;
                let mut id = form.draft.clone();
                for command in commands {
                    match self.submit(command, cx) {
                        Ok(result) => id = created_entry(&result).or(id),
                        Err(err) => {
                            self.form_needs_override |= has_overridable(&err);
                            self.form_failure = Some(Failure::new(self.save_failure_title(), &err));
                            cx.notify();
                            return;
                        }
                    }
                }
                self.finish_form(id, "Posted. The entry is in the books.", cx);
            }
        }
        cx.notify();
    }

    fn finish_form(&mut self, id: Option<EntryId>, notice: &str, cx: &mut Context<Self>) {
        self.form = None;
        self.form_lines.clear();
        match id {
            Some(id) => self.open_entry(id, cx),
            None => self.back_to_list(),
        }
        self.done = Some(notice.to_owned());
    }

    // The import.

    pub(super) fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view = JournalView::Import;
        self.done = None;
        self.import_failure = None;
        self.import_path.update(cx, |i, cx| i.focus(window, cx));
    }

    /// Fills the fund picker; a new organisation starts on its default fund (`import.tsx`,
    /// `useState(fund)`).
    pub(super) fn reset_import_fund(
        &mut self,
        reset: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let funds = self.books.as_deref().map(fund_choices).unwrap_or_default();
        if reset || self.import_fund_id.is_none() {
            self.import_fund_id = funds
                .iter()
                .find(|(id, _)| id.as_str() == ledger_core::coa_template::UNRESTRICTED_FUND_ID)
                .or(funds.first())
                .map(|(id, _)| id.clone());
        }
        let selected = index_of(&funds, self.import_fund_id.as_ref(), 0);
        let list = items(None, &funds);
        self.import_fund.update(cx, |s, cx| {
            s.set_items(list, window, cx);
            s.set_selected_index(selected, window, cx);
        });
        self.derive();
    }

    /// Reads the file in the path field (`readFile`).
    fn load_import(&mut self, cx: &mut Context<Self>) {
        let raw = self.import_path.read(cx).value().trim().to_owned();
        self.import_error = None;
        self.import_created = None;
        self.import_failure = None;
        if raw.is_empty() {
            self.import_error = Some("Enter the path of a CSV file, or choose one.".to_owned());
            cx.notify();
            return;
        }
        let path = expand_home(&raw);
        match read_import_file(&path) {
            Ok(text) => self.import_text = Some(text),
            Err(message) => {
                self.import_text = None;
                self.import_error = Some(message);
            }
        }
        self.derive();
        cx.notify();
    }

    /// `Choose...`: the platform open dialog; the chosen path fills the field and is read.
    fn choose_import_file(window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a journal CSV".into()),
        });
        cx.spawn_in(window, async move |panel, cx| {
            let outcome = chosen.await;
            let _ = panel.update_in(cx, |panel, window, cx| {
                match outcome {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            let text = path.display().to_string();
                            panel
                                .import_path
                                .update(cx, |i, cx| i.set_value(text, window, cx));
                            panel.load_import(cx);
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(err)) => {
                        panel.import_error = Some(format!(
                            "The open dialog is not available ({err}); type a path instead."
                        ));
                    }
                    Err(_) => {
                        panel.import_error =
                            Some("The open dialog closed; type a path instead.".to_owned());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Create drafts (`import.tsx`, `create`): one `entry.import`, all or nothing.
    fn create_import(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = &self.import_preview else {
            return;
        };
        let Some(command) = preview.command() else {
            return;
        };
        let n = preview.entries.len();
        self.import_failure = None;
        match self.submit(command, cx) {
            Ok(_) => {
                self.import_created = Some(n);
                self.import_text = None;
                self.import_preview = None;
            }
            Err(err) => self.import_failure = Some(Failure::new("No drafts created", &err)),
        }
        cx.notify();
    }

    /// `Review imported drafts`: the list on import drafts.
    fn review_imports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter = crate::journal::JournalFilter {
            status: Some(ledger_core::types::EntryStatus::Draft),
            source: Some(ledger_core::types::SourceType::Import),
            ..crate::journal::JournalFilter::default()
        };
        self.cursor = 0;
        self.import_created = None;
        self.back_to_list();
        self.reset_pickers(window, cx);
    }

    // Drawing.

    pub(super) fn form_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.sync_form(cx);
        let (Some(books), Some(form)) = (self.books.clone(), self.form.clone()) else {
            return div().into_any_element();
        };
        let p = palette(cx.theme().mode);
        let base = books.settings.base_currency.clone();
        let errors = form.field_errors();
        let issues = if self.form_submitted {
            form.issues(&books)
        } else {
            Vec::new()
        };
        let summary = form.summary(&books);
        let submitted = self.form_submitted;
        let many = self.form_lines.len() > 2;
        let lines: Vec<AnyElement> = self
            .form_lines
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let date_err = |side| {
                    submitted
                        .then(|| errors.line(i, side).map(str::to_owned))
                        .flatten()
                };
                let engine = |f| form.line_issue(&issues, i, f);
                let debit = date_err(Side::Debit).or_else(|| engine(LineField::Debit));
                let credit = date_err(Side::Credit).or_else(|| engine(LineField::Credit));
                line_editor(
                    i,
                    w,
                    many,
                    [
                        engine(LineField::Account),
                        engine(LineField::Fund),
                        engine(LineField::Grant),
                        debit,
                        credit,
                    ],
                    form.loose_issues(&issues, i),
                    cx,
                )
            })
            .collect();
        let balance = form.balance(&books);
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(1040.0))
            .when_some(self.form_failure.clone(), |v, f| {
                v.child(failure_box(&f, cx))
            })
            .when(submitted && !summary.items.is_empty(), |v| {
                v.child(summary_box(&summary, cx))
            })
            .child(self.entry_section(&errors, submitted, cx))
            .child(
                v_flex()
                    .child(section_header(
                        "Lines",
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(format!(
                                "{} lines \u{b7} amounts in {base}",
                                self.form_lines.len()
                            )),
                        cx,
                    ))
                    .children(lines)
                    .child(
                        h_flex().py(px(12.0)).child(
                            Button::new("add-line")
                                .outline()
                                .label("Add a line")
                                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                                    panel.add_line(window, cx);
                                })),
                        ),
                    )
                    .child(balance_check(balance, cx)),
            )
            .when(self.form_needs_override, |v| v.child(self.override_box(cx)))
            .child(Self::form_footer(cx))
            .into_any_element()
    }

    /// The date and memo (`new.tsx`, the Entry section).
    fn entry_section(
        &self,
        errors: &crate::entry_form::FieldErrors,
        submitted: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex().child(section_header("Entry", div(), cx)).child(
            h_flex()
                .py(px(space::CELL))
                .gap(px(space::CELL))
                .items_start()
                .child(
                    field(
                        "Date",
                        Input::new(&self.entry_date),
                        submitted.then(|| errors.date.clone()).flatten(),
                        cx,
                    )
                    .w(px(224.0))
                    .child(hint("Day, month, year", cx)),
                )
                .child(
                    field(
                        "Memo",
                        Input::new(&self.entry_memo),
                        submitted.then(|| errors.memo.clone()).flatten(),
                        cx,
                    )
                    .flex_1(),
                ),
        )
    }

    fn override_box(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .child(section_header("Override", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(div().text_color(p.ink_muted.to_hsla()).child(
                        "Only grant rules stop this entry, and they can be set aside with a reason \
                         a funder or auditor would accept. The reason is stored on the entry and in \
                         the audit trail.",
                    ))
                    .child(field(
                        "Override reason",
                        Input::new(&self.form_override),
                        self.form_override_error.clone(),
                        cx,
                    ))
                    .child(
                        h_flex().justify_end().child(
                            Button::new("post-override")
                                .primary()
                                .label("Post with override")
                                .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                                    panel.post_form(true, window, cx);
                                })),
                        ),
                    ),
            )
    }

    /// Cancel, Save draft and Post (`new.tsx`, the form footer).
    fn form_footer(cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        h_flex()
            .pt(px(20.0))
            .gap(px(8.0))
            .justify_end()
            .border_t(px(space::RULE_HEAVY))
            .border_color(p.ink_navy.to_hsla())
            .child(
                Button::new("form-cancel")
                    .ghost()
                    .label("Cancel")
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                        panel.leave_form();
                        cx.notify();
                    })),
            )
            .child(
                Button::new("form-draft")
                    .outline()
                    .label("Save draft")
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| panel.save_form(cx))),
            )
            .child(
                Button::new("form-post")
                    .primary()
                    .label("Post")
                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                        panel.post_form(false, window, cx);
                    })),
            )
    }

    pub(super) fn import_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(1200.0))
            .when_some(self.import_created, |v, n| {
                let (title, body) = import_done(n);
                v.child(
                    v_flex()
                        .p(px(16.0))
                        .gap(px(8.0))
                        .border_1()
                        .border_color(p.ink_navy.to_hsla())
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                        .child(div().text_color(p.ink_muted.to_hsla()).child(body))
                        .child(
                            div().child(
                                Button::new("review-imports")
                                    .outline()
                                    .label("Review imported drafts")
                                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                                        panel.review_imports(window, cx);
                                        cx.notify();
                                    })),
                            ),
                        ),
                )
            })
            .child(self.import_file(cx))
            .when_some(self.import_preview.clone(), |v, preview| {
                v.child(self.import_preview_view(&preview, cx))
            })
            .into_any_element()
    }

    fn import_file(&self, cx: &mut Context<Self>) -> Div {
        v_flex().child(section_header("The file", div(), cx)).child(
            h_flex()
                .py(px(space::CELL))
                .gap(px(space::CELL))
                .items_start()
                .child(
                    v_flex()
                        .flex_1()
                        .gap(px(6.0))
                        .child(field(
                            "CSV file",
                            h_flex()
                                .gap(px(8.0))
                                .child(div().flex_1().child(Input::new(&self.import_path)))
                                .child(
                                    Button::new("import-choose")
                                        .outline()
                                        .label("Choose...")
                                        .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                                            Self::choose_import_file(window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("import-load").primary().label("Load").on_click(
                                        cx.listener(|panel, _: &ClickEvent, _, cx| {
                                            panel.load_import(cx);
                                        }),
                                    ),
                                ),
                            self.import_error.clone(),
                            cx,
                        ))
                        .child(hint(
                            "QuickBooks Journal report (Date, Num, Account, Memo/Description, \
                             Debit, Credit) or Xero manual journal (Date, Narration, AccountCode, \
                             Amount with credits negative), up to 5 MB.",
                            cx,
                        )),
                )
                .child(
                    field(
                        "Fund for every line",
                        Select::new(&self.import_fund).placeholder("Choose a fund"),
                        None,
                        cx,
                    )
                    .w(px(288.0))
                    .child(hint(
                        "Imported lines all go to this fund. Change the fund on individual drafts \
                         before posting if needed.",
                        cx,
                    )),
                ),
        )
    }

    fn import_preview_view(&self, preview: &ImportPreview, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let muted = p.ink_muted.to_hsla();
        let entries = preview.entries.iter().map(|e| preview_entry(e, cx));
        v_flex()
            .child(section_header(
                "Preview",
                div()
                    .text_size(px(12.0))
                    .text_color(muted)
                    .child(preview.meta()),
                cx,
            ))
            .when(!preview.problems.is_empty(), |v| {
                v.child(div().pt(px(16.0)).child(failure_box(
                    &Failure {
                        title: "Rows that could not be read".to_owned(),
                        messages: preview.problems.clone(),
                    },
                    cx,
                )))
            })
            .when(!preview.unmapped.is_empty(), |v| {
                v.child(unmapped_box(&preview.unmapped, cx))
            })
            .map(|v| match preview.notice() {
                Some((title, body)) => v.child(
                    v_flex()
                        .py(px(space::CELL))
                        .gap(px(6.0))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                        .child(div().text_color(muted).child(body)),
                ),
                None => v.child(
                    v_flex()
                        .mt(px(16.0))
                        .border_t(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .children(entries),
                ),
            })
            .child(
                v_flex()
                    .mt(px(24.0))
                    .pt(px(20.0))
                    .gap(px(12.0))
                    .border_t(px(space::RULE_HEAVY))
                    .border_color(p.ink_navy.to_hsla())
                    .when_some(self.import_failure.clone(), |v, f| {
                        v.child(failure_box(&f, cx))
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .gap(px(12.0))
                            .child(div().max_w(px(520.0)).text_color(muted).child(
                                "Drafts are saved even when they have issues, so they can be fixed \
                                 here instead of in the file.",
                            ))
                            .when(!preview.entries.is_empty(), |h| {
                                h.child(
                                    Button::new("import-create")
                                        .primary()
                                        .label(preview.button_label())
                                        .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                                            panel.create_import(cx);
                                        })),
                                )
                            }),
                    ),
            )
    }
}

/// `~/x` as a path under the home directory; other text as given.
fn expand_home(raw: &str) -> PathBuf {
    match (raw.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(raw),
    }
}

/// The three figures under the lines: debits, credits and the difference, `Balanced` or the
/// gap in red (`new.tsx`, the `dl` under the lines).
fn balance_check(b: crate::entry_form::Balance, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let cell = |label: &str, value: String, red: bool, first: bool| {
        v_flex()
            .flex_1()
            .gap(px(4.0))
            .when(!first, |c| {
                c.pl(px(12.0))
                    .border_l(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
            })
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .text_size(px(text::TITLE))
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .when(red, |d| d.text_color(p.rejection_red.to_hsla()))
                    .child(value),
            )
    };
    h_flex()
        .py(px(12.0))
        .border_t(px(2.0))
        .border_color(p.ink_navy.to_hsla())
        .child(cell("Debits", b.debits_text(), false, true))
        .child(cell("Credits", b.credits_text(), false, false))
        .child(cell(
            "Difference",
            b.difference_text(),
            !b.is_balanced(),
            false,
        ))
}

/// One line of the entry form (`new.tsx`, a line's fieldset). `errors` are, in order, the
/// account, fund, grant, debit and credit messages.
fn line_editor(
    n: usize,
    w: &LineWidgets,
    removable: bool,
    errors: [Option<String>; 5],
    loose: Vec<String>,
    cx: &mut Context<JournalPanel>,
) -> AnyElement {
    let p = palette(cx.theme().mode);
    let key = w.key;
    let [account, fund, grant, debit, credit] = errors;
    let row = || h_flex().gap(px(space::CELL)).items_start();
    v_flex()
        .py(px(16.0))
        .gap(px(space::CELL))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Line {}", n + 1)),
                )
                .when(removable, |h| {
                    h.child(
                        Button::new(SharedString::from(format!("remove-{key}")))
                            .ghost()
                            .small()
                            .label("Remove")
                            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                                panel.remove_line(key, cx);
                            })),
                    )
                }),
        )
        .child(
            row()
                .child(
                    field(
                        "Account",
                        Select::new(&w.account).placeholder("Choose an account"),
                        account,
                        cx,
                    )
                    .flex_1(),
                )
                .child(field("Debit", Input::new(&w.debit), debit, cx).w(px(180.0)))
                .child(field("Credit", Input::new(&w.credit), credit, cx).w(px(180.0))),
        )
        .child(
            row()
                .child(
                    field(
                        "Fund",
                        Select::new(&w.fund).placeholder("Choose a fund"),
                        fund,
                        cx,
                    )
                    .flex_1(),
                )
                .child(field("Grant", Select::new(&w.grant), grant, cx).flex_1())
                .child(field("Tag", Select::new(&w.tag), None, cx).flex_1())
                .child(field("Line memo", Input::new(&w.memo), None, cx).flex_1()),
        )
        .children(loose.into_iter().map(|m| {
            div()
                .text_size(px(12.0))
                .text_color(p.rejection_red.to_hsla())
                .child(m)
        }))
        .into_any_element()
}

/// The summary box above the form (`new.tsx`, `showSummary`).
fn summary_box(summary: &crate::entry_form::Summary, cx: &App) -> impl IntoElement {
    v_flex()
        .gap(px(6.0))
        .child(failure_box(
            &Failure {
                title: summary.title(),
                messages: summary.items.clone(),
            },
            cx,
        ))
        .child(hint(
            "A draft can still be saved; it is checked again when posted.",
            cx,
        ))
}

/// Accounts of the file not in the chart (`import.tsx`, `unmappedAccounts`).
fn unmapped_box(unmapped: &[String], cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    let n = unmapped.len();
    v_flex()
        .mt(px(16.0))
        .p(px(16.0))
        .gap(px(6.0))
        .border_1()
        .border_color(p.hairline.to_hsla())
        .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
            "{n} {} not in the chart",
            if n == 1 { "account is" } else { "accounts are" }
        )))
        .child(div().text_color(p.ink_muted.to_hsla()).child(
            "Lines on these accounts are left out, so the drafts they belong to will not \
             balance. Add the accounts to the chart of accounts with the same code or name, \
             then load the file again.",
        ))
        .child(
            h_flex()
                .flex_wrap()
                .gap(px(8.0))
                .children(unmapped.iter().map(|a| {
                    div()
                        .px(px(8.0))
                        .border_1()
                        .border_color(p.hairline.to_hsla())
                        .text_size(px(text::MONO))
                        .child(a.clone())
                })),
        )
}

/// One draft of the preview: memo, date and lines, total, and its issues or "Ready".
fn preview_entry(e: &crate::entry_form::PreviewEntry, cx: &App) -> Div {
    let p = palette(cx.theme().mode);
    let muted = p.ink_muted.to_hsla();

    v_flex()
        .py(px(12.0))
        .gap(px(4.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            h_flex()
                .justify_between()
                .gap(px(12.0))
                .child(
                    v_flex()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(e.memo.clone()),
                        )
                        .child(div().text_size(px(12.0)).text_color(muted).child(format!(
                            "{} \u{b7} {} {}",
                            e.date,
                            e.line_count,
                            if e.line_count == 1 { "line" } else { "lines" }
                        ))),
                )
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format_minor(e.total)),
                ),
        )
        .map(|v| {
            if e.issues.is_empty() {
                v.child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Ready to post once reviewed"),
                )
            } else {
                v.children(e.issues.iter().map(|m| {
                    div()
                        .text_size(px(12.0))
                        .text_color(p.rejection_red.to_hsla())
                        .child(m.clone())
                }))
            }
        })
}
