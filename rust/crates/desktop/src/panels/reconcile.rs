//! The Reconcile screen (U8): the bank and mobile-money accounts with their book and statement
//! figures and the add-account form; one account's position against its statement, the
//! statement import with its preview, Auto-match, and each statement line with its status, its
//! candidate journal lines, Match, Unmatch and Post as a new entry.
//!
//! Everything shown comes from the view models in [`crate::reconcile`]; this file keeps the
//! widgets and draws. Writes go through the store and the new books come back through its
//! change channel (README.md, "How a screen replaces its placeholder"). Web screens mirrored:
//! `flowtrace/src/routes/_authenticated/reconcile/index.tsx` and `reconcile/$bankId.tsx`; the
//! dialogs are inline boxes, as on the other desktop screens. The open dialog is GPUI's
//! `App::prompt_for_paths` (the XDG desktop portal on Linux); the path field works without it.
//! Components: <https://gpui-kit.com/component/input>,
//! <https://gpui-kit.com/component/select>, <https://gpui-kit.com/component/button>.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, PathPromptOptions, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::types::{AccountId, BankAccountId, GrantId, StatementLineId};
use ledger_core::types::{BankAccountKind, FundId, IsoDate, LedgerState, OrganizationId};
use ledger_core::{CommandResult, LedgerCommand};

use crate::accounts::{Failure, format_date, format_minor};
use crate::claims::{cash_accounts, fund_choices, grant_choices};
use crate::panels::bills::{new_pick, picked, picks};
use crate::panels::reports::key_figures;
use crate::panels::{
    PickState, choice, demo_tag, failure_box, field, form_buttons, hint, page_title, section_header,
};
use crate::reconcile::{
    BankRow, CandidateRow, ImportPreview, LineRow, LineStatus, MANUAL_WINDOW, NewBankForm,
    Overview, Position, PostForm, STATUSES, auto_match_command, auto_match_summary, bank_rows,
    candidates, count, format_label, format_signed, import_notice, line_rows, match_command,
    matched_percent, overview, position, post_accounts, repeats, suspense_hint, unmatch_command,
};
use crate::statements::{KeyFigure, format_compact, today_utc};
use crate::store::LedgerStore;
use crate::theme::{Palette, palette, space, text};

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Page {
    Accounts,
    Bank(BankAccountId),
}

/// The bank kinds the add form offers, in `KIND_LABEL` order (`index.tsx:25`).
const KINDS: [(BankAccountKind, &str); 3] = [
    (BankAccountKind::Bank, "Bank"),
    (BankAccountKind::MobileMoney, "Mobile money"),
    (BankAccountKind::Cash, "Cash"),
];

/// The import box: the file read and its preview.
#[derive(Default)]
struct Import {
    loaded: Option<PathBuf>,
    preview: Option<ImportPreview>,
    read_error: Option<String>,
    failure: Option<Failure>,
}

/// The Reconcile screen.
pub struct ReconcilePanel {
    store: Entity<LedgerStore>,
    seen: Option<(Option<OrganizationId>, u64)>,
    books: Option<Arc<LedgerState>>,
    demo: bool,
    as_of: IsoDate,
    page: Page,

    rows: Vec<BankRow>,
    overview: Option<Overview>,
    cursor: usize,

    add: Option<NewBankForm>,
    add_error: Option<String>,
    add_failure: Option<Failure>,
    add_done: Option<String>,
    add_name: Entity<InputState>,
    add_provider: Entity<InputState>,
    add_number: Entity<InputState>,
    add_currency: Entity<InputState>,
    add_account: PickState,

    lines: Vec<LineRow>,
    shown: Vec<LineRow>,
    filter: Option<LineStatus>,
    line_cursor: usize,
    candidates: Vec<CandidateRow>,
    candidate_cursor: usize,
    position: Option<Position>,
    notice: Option<String>,
    auto: Option<[String; 3]>,
    failure: Option<Failure>,

    import: Option<Import>,
    import_path: Entity<InputState>,

    post: Option<(StatementLineId, PostForm)>,
    post_error: Option<String>,
    post_failure: Option<Failure>,
    post_account: PickState,
    post_fund: PickState,
    post_grant: PickState,
    post_memo: Entity<InputState>,

    _subscriptions: Vec<Subscription>,
}

impl ReconcilePanel {
    /// The screen over `store`.
    pub fn new(store: Entity<LedgerStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let add_name = input("For example Main operating account", window, cx);
        let add_provider = input("Equity Bank, Safaricom M-Pesa", window, cx);
        let add_number = input("Last digits only, for example •••• 4821", window, cx);
        let add_currency = input("KES", window, cx);
        let import_path = input("/path/to/statement.csv", window, cx);
        let post_memo = input("Shown on the journal entry", window, cx);
        let add_account = new_pick(window, cx, Vec::new(), None);
        let post_account = new_pick(window, cx, Vec::new(), None);
        let post_fund = new_pick(window, cx, Vec::new(), None);
        let post_grant = new_pick(window, cx, Vec::new(), None);
        let subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.refresh(window, cx);
            }),
            cx.subscribe_in(&add_number, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.submit_add(cx);
                }
            }),
            cx.subscribe_in(&import_path, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.enter_import(cx);
                }
            }),
            cx.subscribe_in(&post_memo, window, |this, _, e: &InputEvent, _, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    this.submit_post(cx);
                }
            }),
            cx.subscribe_in(
                &post_fund,
                window,
                |this, _, e: &SelectEvent<Vec<_>>, window, cx| {
                    let SelectEvent::Confirm(value) = e;
                    let fund = value.clone().filter(|v| !v.is_empty()).map(FundId::from);
                    this.set_post_fund(fund, window, cx);
                },
            ),
        ];
        let mut panel = Self {
            store,
            seen: None,
            books: None,
            demo: false,
            as_of: today_utc(),
            page: Page::Accounts,
            rows: Vec::new(),
            overview: None,
            cursor: 0,
            add: None,
            add_error: None,
            add_failure: None,
            add_done: None,
            add_name,
            add_provider,
            add_number,
            add_currency,
            add_account,
            lines: Vec::new(),
            shown: Vec::new(),
            filter: None,
            line_cursor: 0,
            candidates: Vec::new(),
            candidate_cursor: 0,
            position: None,
            notice: None,
            auto: None,
            failure: None,
            import: None,
            import_path,
            post: None,
            post_error: None,
            post_failure: None,
            post_account,
            post_fund,
            post_grant,
            post_memo,
            _subscriptions: subscriptions,
        };
        panel.refresh(window, cx);
        panel
    }

    /// Re-derives when the organisation or its books changed.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.store.read(cx).view();
        let org = view.selected().map(|o| o.id.clone());
        let key = (org.clone(), view.revision());
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        let org_changed = self.seen.as_ref().is_none_or(|(seen, _)| *seen != org);
        self.seen = Some(key);
        self.books = view.snapshot().cloned();
        self.demo = org.as_ref().is_some_and(|o| o.as_str() == DEMO_ORG_ID);
        // The demo is a snapshot as of its latest movement; other books run to today
        // (`useLedgerAsOf`, flowtrace/src/lib/ledger-client.ts:45-47).
        self.as_of = if self.demo {
            IsoDate::parse(DEMO_AS_OF).unwrap_or_else(|_| today_utc())
        } else {
            today_utc()
        };
        if org_changed {
            self.page = Page::Accounts;
            self.cursor = 0;
            self.add = None;
            self.add_done = None;
            self.close_bank_boxes();
        }
        if let Some(books) = self.books.clone() {
            let cash = picks(&cash_accounts(&books), None);
            let chosen = picked(&self.add_account, cx);
            self.add_account.update(cx, |s, cx| {
                s.set_items(cash, window, cx);
                if let Some(v) = &chosen {
                    s.set_selected_value(v, window, cx);
                }
            });
        }
        self.rederive();
        cx.notify();
    }

    fn close_bank_boxes(&mut self) {
        self.notice = None;
        self.auto = None;
        self.failure = None;
        self.import = None;
        self.post = None;
        self.filter = None;
        self.line_cursor = 0;
        self.candidate_cursor = 0;
    }

    /// Derives the list, and the open account's lines, position and candidates.
    fn rederive(&mut self) {
        let Some(books) = self.books.clone() else {
            self.rows.clear();
            self.overview = None;
            self.lines.clear();
            self.shown.clear();
            self.candidates.clear();
            self.position = None;
            return;
        };
        self.rows = bank_rows(&books, &self.as_of);
        self.overview = Some(overview(&books, &self.as_of, &self.rows));
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        if let Page::Bank(bank) = &self.page
            && !books.bank_accounts.iter().any(|b| &b.id == bank)
        {
            self.page = Page::Accounts;
        }
        match &self.page {
            Page::Bank(bank) => {
                self.lines = line_rows(&books, bank);
                self.position = Some(position(&books, bank, &self.as_of));
                // Rows already imported change with the books; the preview's rows do not.
                if let Some(pv) = self.import.as_mut().and_then(|i| i.preview.as_mut()) {
                    pv.repeats = repeats(&books, bank, &pv.rows);
                }
            }
            Page::Accounts => {
                self.lines.clear();
                self.position = None;
            }
        }
        self.reshow();
    }

    /// Applies the status filter and loads the highlighted line's candidates.
    fn reshow(&mut self) {
        self.shown = self
            .lines
            .iter()
            .filter(|r| self.filter.is_none_or(|f| r.status == f))
            .cloned()
            .collect();
        self.line_cursor = self.line_cursor.min(self.shown.len().saturating_sub(1));
        self.candidates = match (self.books.as_deref(), self.current_line()) {
            (Some(books), Some(line)) if !line.status.matched() => {
                candidates(books, &line.id, MANUAL_WINDOW)
            }
            _ => Vec::new(),
        };
        self.candidate_cursor = self
            .candidate_cursor
            .min(self.candidates.len().saturating_sub(1));
    }

    fn current_line(&self) -> Option<&LineRow> {
        self.shown.get(self.line_cursor)
    }

    fn current_bank(&self) -> Option<&BankRow> {
        match &self.page {
            Page::Bank(id) => self.rows.iter().find(|r| &r.id == id),
            Page::Accounts => None,
        }
    }

    fn open_bank(&mut self, id: BankAccountId, cx: &mut Context<Self>) {
        self.page = Page::Bank(id);
        self.close_bank_boxes();
        self.rederive();
        cx.notify();
    }

    /// Keys while the screen shows and no field has focus. On the accounts: `j`/`k` or the
    /// arrows move, Enter opens the account, `n` the add form. On an account: `i` import,
    /// `a` Auto-match, `f` the next status filter, `j`/`k` move through the lines, `c` the
    /// next candidate, `m` match the highlighted line to it, `u` unmatch, `p` post it as a new
    /// entry, Enter imports or posts the open box, Escape steps back.
    pub fn press_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let on_bank = matches!(self.page, Page::Bank(_));
        match key {
            "j" | "down" if !on_bank => {
                self.cursor = (self.cursor + 1).min(self.rows.len().saturating_sub(1));
            }
            "k" | "up" if !on_bank => self.cursor = self.cursor.saturating_sub(1),
            "enter" if !on_bank => {
                let Some(row) = self.rows.get(self.cursor) else {
                    return false;
                };
                let id = row.id.clone();
                self.open_bank(id, cx);
            }
            "n" if !on_bank => self.open_add(window, cx),
            "escape" if !on_bank && self.add.is_some() => self.add = None,
            "escape" if on_bank => {
                if self.import.is_some() || self.post.is_some() {
                    self.import = None;
                    self.post = None;
                } else {
                    self.page = Page::Accounts;
                    self.close_bank_boxes();
                    self.rederive();
                }
            }
            "i" if on_bank => self.open_import(window, cx),
            "a" if on_bank => self.run_auto_match(cx),
            "f" if on_bank => {
                let i = STATUSES.iter().position(|s| *s == self.filter).unwrap_or(0);
                self.filter = STATUSES[(i + 1) % STATUSES.len()];
                self.reshow();
            }
            "j" | "down" if on_bank => {
                self.line_cursor = (self.line_cursor + 1).min(self.shown.len().saturating_sub(1));
                self.candidate_cursor = 0;
                self.post = None;
                self.reshow();
            }
            "k" | "up" if on_bank => {
                self.line_cursor = self.line_cursor.saturating_sub(1);
                self.candidate_cursor = 0;
                self.post = None;
                self.reshow();
            }
            "c" if on_bank && !self.candidates.is_empty() => {
                self.candidate_cursor = (self.candidate_cursor + 1) % self.candidates.len();
            }
            "m" if on_bank => self.match_current(cx),
            "u" if on_bank => self.unmatch_current(cx),
            "p" if on_bank => self.open_post(window, cx),
            "enter" if on_bank && self.post.is_some() => self.submit_post(cx),
            "enter" if on_bank && self.import.is_some() => self.submit_import(cx),
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Submits `command`; `Err` holds the refusal as `title` with the engine's messages.
    fn submit(
        &mut self,
        command: LedgerCommand,
        title: &'static str,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, Result<CommandResult, Failure>, &mut Context<Self>) + 'static,
    ) {
        let store = self.store.clone();
        crate::panels::submit(self, &store, command, cx, move |this, result, cx| {
            then(this, result.map_err(|e| Failure::new(title, &e)), cx);
        });
    }

    // Accounts list and the add form.

    fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        let form = NewBankForm::new(&books);
        let currency = form.currency.clone();
        for input in [&self.add_name, &self.add_provider, &self.add_number] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        self.add_currency
            .update(cx, |i, cx| i.set_value(currency, window, cx));
        self.add = Some(form);
        self.add_error = None;
        self.add_failure = None;
        self.add_done = None;
        self.add_name.update(cx, |i, cx| i.focus(window, cx));
    }

    fn submit_add(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.add else {
            return;
        };
        form.name = self.add_name.read(cx).value().to_string();
        form.provider = self.add_provider.read(cx).value().to_string();
        form.number_masked = self.add_number.read(cx).value().to_string();
        form.currency = self.add_currency.read(cx).value().to_string();
        form.account = picked(&self.add_account, cx).map(AccountId::from);
        let command = match form.command() {
            Ok(c) => c,
            Err(e) => {
                self.add_error = Some(e);
                cx.notify();
                return;
            }
        };
        let name = form.name.trim().to_owned();
        self.submit(
            command,
            "Not added",
            cx,
            move |this, result, _| match result {
                Ok(_) => {
                    this.add = None;
                    this.add_error = None;
                    this.add_failure = None;
                    this.add_done = Some(format!("Added {name}. Open it to import its statement."));
                }
                Err(f) => this.add_failure = Some(f),
            },
        );
        cx.notify();
    }

    // The account page.

    fn run_auto_match(&mut self, cx: &mut Context<Self>) {
        let Page::Bank(bank) = self.page.clone() else {
            return;
        };
        self.notice = None;
        self.failure = None;
        let command = auto_match_command(&bank);
        self.submit(
            command,
            "That did not go through",
            cx,
            |this, result, _| match result {
                Ok(CommandResult::AutoMatch { result }) => {
                    this.auto = Some(auto_match_summary(&result));
                }
                Ok(_) => this.auto = None,
                Err(f) => {
                    this.auto = None;
                    this.failure = Some(f);
                }
            },
        );
    }

    fn match_current(&mut self, cx: &mut Context<Self>) {
        let (Some(line), Some(c)) = (
            self.current_line().cloned(),
            self.candidates.get(self.candidate_cursor).cloned(),
        ) else {
            return;
        };
        self.failure = None;
        self.auto = None;
        let command = match_command(&line.id, &c.journal_line);
        self.submit(
            command,
            "Not matched",
            cx,
            move |this, result, _| match result {
                Ok(_) => {
                    this.notice = Some(format!(
                        "Matched \u{201c}{}\u{201d} to {}.",
                        line.description, c.number
                    ));
                }
                Err(f) => this.failure = Some(f),
            },
        );
    }

    fn unmatch_current(&mut self, cx: &mut Context<Self>) {
        let Some(line) = self.current_line().cloned() else {
            return;
        };
        if !line.status.matched() {
            return;
        }
        self.failure = None;
        self.auto = None;
        let command = unmatch_command(&line.id);
        self.submit(
            command,
            "That did not go through",
            cx,
            move |this, result, _| match result {
                Ok(_) => {
                    this.notice = Some(format!(
                        "Unmatched \u{201c}{}\u{201d}. It is back in the unmatched lines.",
                        line.description
                    ));
                }
                Err(f) => this.failure = Some(f),
            },
        );
    }

    // Import.

    fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.post = None;
        self.import = Some(Import::default());
        self.import_path.update(cx, |i, cx| i.focus(window, cx));
    }

    /// Enter in the path field: loads the file, or imports it when it is already loaded.
    fn enter_import(&mut self, cx: &mut Context<Self>) {
        let path = PathBuf::from(self.import_path.read(cx).value().trim());
        let loaded = self
            .import
            .as_ref()
            .is_some_and(|i| i.loaded.as_ref() == Some(&path) && i.preview.is_some());
        if loaded {
            self.submit_import(cx);
        } else {
            self.load_import(path, cx);
        }
    }

    fn load_import(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let (Page::Bank(bank), Some(books)) = (self.page.clone(), self.books.clone()) else {
            return;
        };
        let import = self.import.get_or_insert_with(Import::default);
        import.failure = None;
        if let Ok(text) = std::fs::read_to_string(&path) {
            import.preview = ImportPreview::of(&books, &bank, &text);
            import.read_error = import
                .preview
                .is_none()
                .then(|| "The file is empty.".to_owned());
            import.loaded = Some(path);
        } else {
            import.preview = None;
            import.loaded = None;
            import.read_error =
                Some("The file could not be read. Save it as CSV and try again.".to_owned());
        }
        cx.notify();
    }

    fn choose_file(window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a statement CSV".into()),
        });
        cx.spawn_in(window, async move |panel, cx| {
            let outcome = chosen.await;
            let _ = panel.update_in(cx, |panel, window, cx| {
                let error = match outcome {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            let text = path.display().to_string();
                            panel
                                .import_path
                                .update(cx, |i, cx| i.set_value(text, window, cx));
                            panel.load_import(path, cx);
                        }
                        None
                    }
                    Ok(Ok(None)) => None,
                    Ok(Err(err)) => Some(format!(
                        "The open dialog is not available ({err}); type a path instead."
                    )),
                    Err(_) => Some("The open dialog closed; type a path instead.".to_owned()),
                };
                if let (Some(e), Some(import)) = (error, panel.import.as_mut()) {
                    import.read_error = Some(e);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn submit_import(&mut self, cx: &mut Context<Self>) {
        let Page::Bank(bank) = self.page.clone() else {
            return;
        };
        let Some(preview) = self.import.as_ref().and_then(|i| i.preview.clone()) else {
            return;
        };
        let Some(command) = preview.command(&bank) else {
            return;
        };
        self.submit(
            command,
            "Not imported",
            cx,
            move |this, result, _| match result {
                Ok(result) => {
                    let added = match result {
                        CommandResult::Ids { ids } => ids.len(),
                        _ => 0,
                    };
                    this.import = None;
                    this.auto = None;
                    this.failure = None;
                    this.notice = Some(import_notice(added, preview.rows.len()));
                }
                Err(f) => {
                    if let Some(import) = &mut this.import {
                        import.failure = Some(f);
                    }
                }
            },
        );
        cx.notify();
    }

    // Post as a new entry.

    fn open_post(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(books), Some(row)) = (self.books.clone(), self.current_line().cloned()) else {
            return;
        };
        if row.status.matched() {
            return;
        }
        let Some(line) = books.statement_lines.iter().find(|s| s.id == row.id) else {
            return;
        };
        let form = PostForm::new(&books, line);
        let accounts = picks(
            &post_accounts(&books, line.amount),
            Some("Suspense (leave empty)"),
        );
        let funds = picks(&fund_choices(&books), None);
        let fund = form.fund.as_ref().map(|f| f.as_str().to_owned());
        self.post_account
            .update(cx, |s, cx| s.set_items(accounts, window, cx));
        self.post_fund.update(cx, |s, cx| {
            s.set_items(funds, window, cx);
            if let Some(f) = &fund {
                s.set_selected_value(f, window, cx);
            }
        });
        self.load_grants(&books, form.fund.as_ref(), window, cx);
        let memo = form.memo.clone();
        self.post_memo
            .update(cx, |i, cx| i.set_value(memo, window, cx));
        self.import = None;
        self.post = Some((row.id, form));
        self.post_error = None;
        self.post_failure = None;
    }

    fn load_grants(
        &self,
        books: &LedgerState,
        fund: Option<&FundId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let grants = picks(&grant_choices(books, fund), Some("No grant"));
        self.post_grant
            .update(cx, |s, cx| s.set_items(grants, window, cx));
    }

    fn set_post_fund(&mut self, fund: Option<FundId>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(books) = self.books.clone() else {
            return;
        };
        self.load_grants(&books, fund.as_ref(), window, cx);
        if let Some((_, form)) = &mut self.post {
            form.set_fund(fund);
        }
        cx.notify();
    }

    fn submit_post(&mut self, cx: &mut Context<Self>) {
        let account = picked(&self.post_account, cx).map(AccountId::from);
        let fund = picked(&self.post_fund, cx).map(FundId::from);
        let grant = picked(&self.post_grant, cx).map(GrantId::from);
        let memo = self.post_memo.read(cx).value().to_string();
        let Some((line, form)) = &mut self.post else {
            return;
        };
        form.account = account;
        form.fund = fund;
        form.grant = grant;
        form.memo = memo;
        let line = line.clone();
        let command = match form.command(&line) {
            Ok(c) => c,
            Err(e) => {
                self.post_error = Some(e);
                cx.notify();
                return;
            }
        };
        let description = self
            .lines
            .iter()
            .find(|r| r.id == line)
            .map(|r| r.description.clone())
            .unwrap_or_default();
        self.submit(
            command,
            "Not posted",
            cx,
            move |this, result, _| match result {
                Ok(_) => {
                    this.post = None;
                    this.auto = None;
                    this.failure = None;
                    this.notice = Some(format!(
                        "Posted \u{201c}{description}\u{201d} as a new entry and matched it."
                    ));
                }
                Err(f) => this.post_failure = Some(f),
            },
        );
        cx.notify();
    }
}

// Drawing.

fn muted(s: impl Into<SharedString>, p: &Palette) -> Div {
    div()
        .text_size(px(text::BODY))
        .text_color(p.ink_muted.to_hsla())
        .child(s.into())
}

/// `ReportNotice` (`DESIGN.md`, Cards): a centred title and a muted line, 40px above and below.
fn report_notice(title: &str, line: &str, p: &Palette) -> Div {
    v_flex()
        .py(px(40.0))
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(muted(line.to_owned(), p))
}

/// `MatchLine` (`flowtrace/src/components/ledger/reports/match-mark.tsx:22-55`) drawn with
/// boxes: auto a heavy solid line, by hand a double line, posted a double line with a stop,
/// needs a decision a muted dashed line, unmatched a long orange dash with an X.
fn match_line(status: LineStatus, p: &Palette) -> Div {
    let ink = p.ink_navy.to_hsla();
    let flag = p.signal_orange.to_hsla();
    let muted_ink = p.ink_muted.to_hsla();
    match status {
        LineStatus::Auto => h_flex()
            .w(px(22.0))
            .child(div().w(px(22.0)).h(px(3.0)).bg(ink)),
        LineStatus::Manual => v_flex()
            .w(px(22.0))
            .gap(px(3.0))
            .child(div().w(px(22.0)).h(px(1.5)).bg(ink))
            .child(div().w(px(22.0)).h(px(1.5)).bg(ink)),
        LineStatus::Posted => h_flex()
            .w(px(22.0))
            .items_center()
            .child(
                v_flex()
                    .gap(px(3.0))
                    .child(div().w(px(18.0)).h(px(1.5)).bg(ink))
                    .child(div().w(px(18.0)).h(px(1.5)).bg(ink)),
            )
            .child(div().w(px(3.0)).h(px(8.0)).bg(ink)),
        LineStatus::Ambiguous => h_flex()
            .w(px(22.0))
            .gap(px(4.0))
            .children((0..2).map(|_| div().w(px(6.0)).h(px(3.0)).bg(muted_ink))),
        LineStatus::Unmatched => h_flex()
            .w(px(22.0))
            .items_center()
            .gap(px(2.0))
            .child(div().w(px(9.0)).h(px(3.0)).bg(flag))
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(flag)
                    .child("\u{d7}"),
            )
            .child(div().w(px(5.0)).h(px(3.0)).bg(flag)),
    }
}

/// `MatchMark` (`match-mark.tsx:57-71`): the line and its label, tinted for the open states.
fn match_mark(status: LineStatus, p: &Palette) -> Div {
    h_flex()
        .gap(px(8.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .when(status == LineStatus::Unmatched, |m| {
            m.text_color(p.signal_orange.to_hsla())
        })
        .when(status == LineStatus::Ambiguous, |m| {
            m.text_color(p.ink_muted.to_hsla())
        })
        .child(match_line(status, p))
        .child(status.label())
}

/// `CoverageBar` (`DESIGN.md`, Coverage bar): matched share in Report Blue on Gap Grey.
fn coverage_bar(row: &BankRow, p: &Palette) -> Div {
    #[expect(clippy::cast_precision_loss, reason = "line counts")]
    let share = if row.lines == 0 {
        0.0
    } else {
        row.matched as f32 / row.lines as f32
    };
    div().w_full().h(px(12.0)).bg(p.gap_grey.to_hsla()).child(
        div()
            .h_full()
            .w(gpui_kit::relative(share))
            .bg(p.report_blue.to_hsla()),
    )
}

fn figure(label: &str, value: impl Into<SharedString>, p: &Palette) -> Div {
    v_flex()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(text::LABEL))
                .text_color(p.ink_muted.to_hsla())
                .child(label.to_owned()),
        )
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value.into()))
}

impl ReconcilePanel {
    fn meta(&self, cx: &App) -> AnyElement {
        h_flex()
            .gap(px(8.0))
            .text_size(px(12.0))
            .child(format!("As at {}", format_date(&self.as_of)))
            .when(self.demo, |m| m.child(demo_tag(cx)))
            .into_any_element()
    }

    fn accounts_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let figures = self.overview.as_ref().map(|o| {
            vec![
                KeyFigure {
                    value: o.reconciled.clone(),
                    label: "Cash movement reconciled".to_owned(),
                },
                KeyFigure {
                    value: o.accounts.to_string(),
                    label: "Accounts".to_owned(),
                },
                KeyFigure {
                    value: o.lines.to_string(),
                    label: "Statement lines".to_owned(),
                },
                KeyFigure {
                    value: o.unmatched.to_string(),
                    label: "Unmatched lines".to_owned(),
                },
            ]
        });
        let base = self
            .books
            .as_ref()
            .map(|b| b.settings.base_currency.clone())
            .unwrap_or_default();
        let list = v_flex().children(
            self.rows
                .iter()
                .enumerate()
                .map(|(i, r)| self.account_row(i, r, cx)),
        );
        let action = Button::new("open-add-bank")
            .primary()
            .label("Add an account")
            .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                panel.open_add(window, cx);
                cx.notify();
            }));
        v_flex()
            .gap(px(space::SECTION))
            .child(
                h_flex()
                    .justify_between()
                    .items_start()
                    .gap(px(24.0))
                    .child(v_flex().gap(px(8.0)).child(page_title("Reconcile")).child(
                        div().max_w(px(720.0)).text_size(px(text::LEDE)).child(
                            "Each bank and mobile-money account set against its statement. \
                                 A line is reconciled when it is matched to a posted journal \
                                 line; anything left over is money the books have not explained \
                                 yet.",
                        ),
                    ))
                    .child(action),
            )
            .when_some(self.add_done.clone(), |v, done| v.child(div().child(done)))
            .when(self.add.is_some(), |v| v.child(self.add_form(cx)))
            .when_some(figures, |v, figures| {
                v.child(
                    v_flex()
                        .child(section_header("Across all accounts", self.meta(cx), cx))
                        .child(key_figures(&figures, p)),
                )
            })
            .child(
                v_flex()
                    .child(section_header(
                        "Accounts",
                        div()
                            .text_size(px(12.0))
                            .child(format!("{} accounts", self.rows.len())),
                        cx,
                    ))
                    .child(if self.rows.is_empty() {
                        report_notice(
                            "No bank or mobile-money accounts yet",
                            "Link a bank or mobile-money account to a cash account in the chart, \
                             then import its statement here.",
                            p,
                        )
                    } else {
                        list
                    })
                    .child(div().py(px(12.0)).child(hint(
                        &format!(
                            "Book balance in {base}, {}. j and k move, Enter opens an \
                                 account, n adds one.",
                            format_date(&self.as_of)
                        ),
                        cx,
                    ))),
            )
            .into_any_element()
    }

    fn account_row(&self, i: usize, r: &BankRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        h_flex()
            .id(SharedString::from(format!("bank-{}", r.id.as_str())))
            .py(px(16.0))
            .px(px(12.0))
            .gap(px(24.0))
            .items_start()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .cursor_pointer()
            .when(i == self.cursor, |row| row.bg(p.wash.to_hsla()))
            .hover(|row| row.bg(p.rail_hover.to_hsla()))
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.open_bank(id.clone(), cx);
            }))
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(4.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{} \u{2192}", r.name)),
                    )
                    .child(muted(
                        format!(
                            "{} \u{b7} {} \u{b7} {}",
                            r.kind, r.provider, r.number_masked
                        ),
                        p,
                    ))
                    .child(muted(
                        format!(
                            "{} \u{b7} book balance {} {}",
                            r.gl,
                            r.currency,
                            format_minor(r.ledger_balance)
                        ),
                        p,
                    )),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(8.0))
                    .child(if r.lines == 0 {
                        muted("No statement imported yet", p)
                    } else {
                        v_flex()
                            .gap(px(4.0))
                            .child(coverage_bar(r, p))
                            .child(div().text_size(px(12.0)).child(r.coverage_text()))
                    })
                    .child(
                        h_flex()
                            .gap(px(24.0))
                            .child(figure(
                                "Matched",
                                format!("{} of {}", r.matched, r.lines),
                                p,
                            ))
                            .child(figure("Unmatched", r.unmatched.to_string(), p))
                            .child(figure(
                                "Statement net",
                                format_signed(r.statement_balance),
                                p,
                            ))
                            .child(figure(
                                "Last statement line",
                                r.last
                                    .as_ref()
                                    .map_or_else(|| "None".to_owned(), format_date),
                                p,
                            )),
                    ),
            )
            .into_any_element()
    }

    fn position_table(s: &Position, open: usize, cx: &mut Context<Self>) -> Option<Div> {
        let p = palette(cx.theme().mode);
        let (from, to) = s.span.clone()?;
        let row = |label: Div, a: String, b: String, strong: bool| {
            h_flex()
                .py(px(8.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .when(strong, |r| {
                    r.font_weight(FontWeight::BOLD)
                        .border_t(px(1.0))
                        .border_color(p.ink_navy.to_hsla())
                })
                .child(label.flex_1())
                .child(div().w(px(160.0)).flex().justify_end().child(a))
                .child(div().w(px(160.0)).flex().justify_end().child(b))
        };
        let dash = "\u{2013}".to_owned();
        let difference = s.difference();
        Some(
            v_flex()
                .child(section_header(
                    "Statement against the books",
                    div().text_size(px(12.0)).child(format!(
                        "{} to {}",
                        format_date(&from),
                        format_date(&to)
                    )),
                    cx,
                ))
                .child(row(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child("Net movement"),
                    "Statement".to_owned(),
                    "Books".to_owned(),
                    false,
                ))
                .child(row(
                    div().child("Matched lines"),
                    format_minor(s.matched),
                    format_minor(s.matched),
                    false,
                ))
                .child(row(
                    v_flex()
                        .child("Only on the statement")
                        .child(muted(format!("{open} lines not yet in the books"), p)),
                    format_minor(s.only_statement),
                    dash.clone(),
                    false,
                ))
                .child(row(
                    v_flex().child("Only in the books").child(muted(
                        format!(
                            "{} journal lines in these dates not on the statement",
                            s.only_books_count
                        ),
                        p,
                    )),
                    dash,
                    format_minor(s.only_books),
                    false,
                ))
                .child(row(
                    div().child("Net movement"),
                    format_minor(s.statement_net()),
                    format_minor(s.books_net()),
                    true,
                ))
                .child(
                    h_flex()
                        .py(px(12.0))
                        .justify_between()
                        .gap(px(24.0))
                        .child(muted(s.verdict(), p))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                            "Difference {}",
                            if difference == 0 {
                                "0.00".to_owned()
                            } else {
                                format_minor(difference)
                            }
                        ))),
                ),
        )
    }

    fn add_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kind = self.add.as_ref().map_or(BankAccountKind::Bank, |f| f.kind);
        let kinds = h_flex()
            .gap(px(4.0))
            .children(KINDS.iter().map(|(k, label)| {
                let k = *k;
                choice(format!("bank-kind-{label}"), label, kind == k).on_click(cx.listener(
                    move |panel, _: &ClickEvent, _, cx| {
                        if let Some(form) = &mut panel.add {
                            form.kind = k;
                        }
                        cx.notify();
                    },
                ))
            }));
        v_flex()
            .max_w(px(640.0))
            .child(section_header("Add an account", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(field("Name", Input::new(&self.add_name), None, cx))
                    .child(field("Kind", kinds, None, cx))
                    .child(field("Provider", Input::new(&self.add_provider), None, cx))
                    .child(field(
                        "Cash account in the chart",
                        Select::new(&self.add_account).placeholder("Choose a cash account"),
                        None,
                        cx,
                    ))
                    .child(field("Currency", Input::new(&self.add_currency), None, cx))
                    .child(field(
                        "Number",
                        Input::new(&self.add_number),
                        self.add_error.clone(),
                        cx,
                    ))
                    .when_some(self.add_failure.as_ref(), |v, f| {
                        v.child(failure_box(f, cx))
                    })
                    .child(form_buttons(
                        "add-bank",
                        "Add account",
                        |panel: &mut Self, _, cx| panel.submit_add(cx),
                        |panel: &mut Self| panel.add = None,
                        cx,
                    )),
            )
    }

    fn bank_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(bank) = self.current_bank().cloned() else {
            return muted("This account is not in the books.", p).into_any_element();
        };
        let actions = h_flex()
            .gap(px(8.0))
            .child(
                Button::new("open-import")
                    .outline()
                    .label("Import statement")
                    .on_click(cx.listener(|panel, _: &ClickEvent, window, cx| {
                        panel.open_import(window, cx);
                        cx.notify();
                    })),
            )
            .child(
                Button::new("auto-match")
                    .primary()
                    .label("Auto-match")
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                        panel.run_auto_match(cx);
                        cx.notify();
                    })),
            );
        v_flex()
            .gap(px(space::SECTION))
            .child(
                h_flex()
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("back-to-accounts")
                                    .cursor_pointer()
                                    .text_size(px(12.0))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child("\u{2190} All accounts (Escape)")
                                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                                        panel.page = Page::Accounts;
                                        panel.close_bank_boxes();
                                        panel.rederive();
                                        cx.notify();
                                    })),
                            )
                            .child(page_title(&bank.name))
                            .child(muted(
                                format!(
                                    "{} \u{b7} {} \u{b7} {}",
                                    bank.provider, bank.number_masked, bank.currency
                                ),
                                p,
                            )),
                    )
                    .child(actions),
            )
            .when_some(self.auto.as_ref(), |v, lines| {
                v.child(auto_report(lines, p))
            })
            .when_some(self.notice.clone(), |v, n| {
                v.child(
                    div()
                        .py(px(12.0))
                        .border_t(px(space::HAIRLINE))
                        .border_b(px(space::HAIRLINE))
                        .border_color(p.hairline.to_hsla())
                        .child(n),
                )
            })
            .when_some(self.failure.as_ref(), |v, f| v.child(failure_box(f, cx)))
            .when(self.import.is_some(), |v| {
                v.child(self.import_box(&bank, cx))
            })
            .child(self.position_section(&bank, cx))
            .child(self.lines_section(cx))
            .into_any_element()
    }

    fn position_section(&self, bank: &BankRow, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let matched = self.lines.iter().filter(|r| r.status.matched()).count();
        let figures = vec![
            KeyFigure {
                value: format_compact(self.position.as_ref().map_or(0, |s| s.book_balance)),
                label: format!("Book balance, {}", bank.currency),
            },
            KeyFigure {
                value: matched_percent(&self.lines),
                label: format!("Lines matched, {matched} of {}", self.lines.len()),
            },
            KeyFigure {
                value: count(&self.lines, Some(LineStatus::Ambiguous)).to_string(),
                label: "Need a decision".to_owned(),
            },
            KeyFigure {
                value: count(&self.lines, Some(LineStatus::Unmatched)).to_string(),
                label: "Unmatched".to_owned(),
            },
        ];
        let open = count(&self.lines, Some(LineStatus::Ambiguous))
            + count(&self.lines, Some(LineStatus::Unmatched));
        let table = self
            .position
            .as_ref()
            .and_then(|s| Self::position_table(s, open, cx));
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .child(section_header("Position", self.meta(cx), cx))
                    .child(key_figures(&figures, p)),
            )
            .children(table)
    }

    fn import_box(&self, bank: &BankRow, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let import = self.import.as_ref();
        let preview = import.and_then(|i| i.preview.as_ref());
        let read_error = import.and_then(|i| i.read_error.clone());
        let failure = import.and_then(|i| i.failure.as_ref());
        let path_row = h_flex()
            .gap(px(8.0))
            .child(div().flex_1().child(Input::new(&self.import_path)))
            .child(
                Button::new("import-choose")
                    .outline()
                    .label("Choose...")
                    .on_click(cx.listener(|_, _: &ClickEvent, window, cx| {
                        Self::choose_file(window, cx);
                    })),
            )
            .child(
                Button::new("import-load")
                    .outline()
                    .label("Load")
                    .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| {
                        let path = PathBuf::from(panel.import_path.read(cx).value().trim());
                        panel.load_import(path, cx);
                    })),
            );
        let preview_el = preview.map(|pv| preview_view(pv, p));
        let label = preview.map_or_else(|| "Import".to_owned(), ImportPreview::button_label);
        v_flex()
            .max_w(px(880.0))
            .child(section_header(
                "Import statement",
                div().text_size(px(12.0)).child(format!(
                    "Into {}. Lines already on this statement are left out.",
                    bank.name
                )),
                cx,
            ))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(field("Statement file (CSV)", path_row, read_error, cx))
                    .child(hint(
                        "Bank CSVs and M-Pesa, MTN MoMo and Airtel Money statements are \
                         recognised from their column names. Enter loads the file; Enter again \
                         imports it.",
                        cx,
                    ))
                    .when_some(import.and_then(|i| i.loaded.as_ref()), |v, path| {
                        v.child(muted(format!("From {}", path.display()), p))
                    })
                    .children(preview_el)
                    .when_some(failure, |v, f| v.child(failure_box(f, cx)))
                    .child(form_buttons(
                        "import",
                        label,
                        |panel: &mut Self, _, cx| panel.submit_import(cx),
                        |panel: &mut Self| panel.import = None,
                        cx,
                    )),
            )
    }

    fn lines_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let chips = h_flex().gap(px(4.0)).children(STATUSES.iter().map(|s| {
            let s = *s;
            let label = s.map_or("All", LineStatus::short);
            let n = count(&self.lines, s);
            let chosen = self.filter == s;
            let button = Button::new(SharedString::from(format!("line-filter-{label}")))
                .small()
                .label(format!("{label} {n}"));
            if chosen {
                button.primary()
            } else {
                button.outline()
            }
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.filter = s;
                panel.reshow();
                cx.notify();
            }))
        }));
        let body: AnyElement = if self.lines.is_empty() {
            report_notice(
                "No statement imported yet",
                "Import a CSV from the bank, or an M-Pesa, MTN MoMo or Airtel Money statement, \
                 to start matching it against the books.",
                p,
            )
            .into_any_element()
        } else if self.shown.is_empty() {
            report_notice(
                "No lines in this state",
                "Every line on this statement is in another state.",
                p,
            )
            .into_any_element()
        } else {
            v_flex()
                .children(
                    self.shown
                        .iter()
                        .enumerate()
                        .map(|(i, r)| self.line_row(i, r, cx)),
                )
                .into_any_element()
        };
        v_flex()
            .child(section_header(
                "Statement lines",
                div()
                    .text_size(px(12.0))
                    .child("j and k move, c next candidate, m match, u unmatch, p post, f filter"),
                cx,
            ))
            .child(
                div()
                    .py(px(12.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(chips),
            )
            .child(body)
    }

    fn line_row(&self, i: usize, r: &LineRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let selected = i == self.line_cursor;
        let row = h_flex()
            .id(SharedString::from(format!("line-{}", r.id.as_str())))
            .py(px(12.0))
            .px(px(12.0))
            .gap(px(24.0))
            .items_start()
            .cursor_pointer()
            .when(selected, |row| row.bg(p.wash.to_hsla()))
            .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                panel.line_cursor = i;
                panel.candidate_cursor = 0;
                panel.post = None;
                panel.reshow();
                cx.notify();
            }))
            .child(
                div()
                    .w(px(110.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(format_date(&r.date)),
            )
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
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.ink_muted.to_hsla())
                            .child(
                                r.reference
                                    .clone()
                                    .unwrap_or_else(|| "No reference".to_owned()),
                            ),
                    )
                    .child(div().pt(px(6.0)).child(match_mark(r.status, p))),
            )
            .child(
                v_flex()
                    .w(px(150.0))
                    .items_end()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format_signed(r.amount)),
                    )
                    .child(muted(
                        if r.amount > 0 {
                            "Money in"
                        } else {
                            "Money out"
                        },
                        p,
                    )),
            )
            .child(Self::line_actions(r, cx));
        let mut block = v_flex()
            .border_b(px(space::HAIRLINE))
            .border_color(p.hairline.to_hsla())
            .child(row);
        if selected && !r.status.matched() {
            block = block.child(self.candidates_box(cx));
            if self.post.as_ref().is_some_and(|(line, _)| *line == r.id) {
                block = block.child(self.post_box(r, cx));
            }
        }
        block.into_any_element()
    }

    fn line_actions(r: &LineRow, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let id = r.id.clone();
        if r.status.matched() {
            h_flex()
                .w(px(220.0))
                .justify_end()
                .gap(px(8.0))
                .when_some(r.entry.clone(), |v, (_, number)| {
                    v.child(div().text_color(p.report_blue.to_hsla()).child(number))
                })
                .child(
                    Button::new(SharedString::from(format!("unmatch-{}", id.as_str())))
                        .ghost()
                        .small()
                        .label("Unmatch")
                        .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                            if let Some(i) = panel.shown.iter().position(|x| x.id == id) {
                                panel.line_cursor = i;
                            }
                            panel.unmatch_current(cx);
                            cx.notify();
                        })),
                )
        } else {
            let post_id = id.clone();
            h_flex().w(px(220.0)).justify_end().gap(px(8.0)).child(
                Button::new(SharedString::from(format!("post-{}", post_id.as_str())))
                    .ghost()
                    .small()
                    .label("Post as new entry")
                    .on_click(cx.listener(move |panel, _: &ClickEvent, window, cx| {
                        if let Some(i) = panel.shown.iter().position(|x| x.id == post_id) {
                            panel.line_cursor = i;
                            panel.reshow();
                        }
                        panel.open_post(window, cx);
                        cx.notify();
                    })),
            )
        }
    }

    fn candidates_box(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .ml(px(134.0))
            .mb(px(12.0))
            .mr(px(12.0))
            .border_1()
            .border_color(p.hairline.to_hsla())
            .bg(p.sheet.to_hsla())
            .child(
                div()
                    .px(px(16.0))
                    .py(px(10.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(if self.candidates.is_empty() {
                        format!(
                            "No journal line fits. Nothing posted to this account for exactly \
                             this amount within {MANUAL_WINDOW} days is still unmatched. Post the \
                             line as a new entry instead."
                        )
                    } else {
                        format!(
                            "Match to the books: posted lines on this account for the same \
                             amount within {MANUAL_WINDOW} days, closest first."
                        )
                    }),
            )
            .children(self.candidates.iter().enumerate().map(|(i, c)| {
                let chosen = i == self.candidate_cursor;
                h_flex()
                    .px(px(16.0))
                    .py(px(10.0))
                    .gap(px(16.0))
                    .justify_between()
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .when(chosen, |row| row.bg(p.wash.to_hsla()))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(c.number.clone()),
                                    )
                                    .child(muted(format!("\u{b7} {}", format_date(&c.date)), p)),
                            )
                            .child(div().child(c.memo.clone()))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(c.gap_text()),
                            ),
                    )
                    .child({
                        let b = Button::new(SharedString::from(format!(
                            "match-{}",
                            c.journal_line.as_str()
                        )))
                        .small()
                        .label(if chosen {
                            "Match (m)"
                        } else {
                            "Match"
                        });
                        if chosen { b.primary() } else { b.outline() }.on_click(cx.listener(
                            move |panel, _: &ClickEvent, _, cx| {
                                panel.candidate_cursor = i;
                                panel.match_current(cx);
                                cx.notify();
                            },
                        ))
                    })
            }))
    }

    fn post_box(&self, r: &LineRow, cx: &mut Context<Self>) -> impl IntoElement {
        let books = self.books.clone();
        let bank = self
            .current_bank()
            .map_or_else(|| "the account".to_owned(), |b| b.name.clone());
        let suspense = books.as_deref().map(suspense_hint).unwrap_or_default();
        v_flex()
            .ml(px(134.0))
            .mb(px(12.0))
            .mr(px(12.0))
            .max_w(px(640.0))
            .child(section_header("Post as a new entry", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .child(hint(
                        &format!(
                            "{} {bank}. Choose what it was for; the entry is posted on {} and \
                             matched to this line.",
                            if r.amount < 0 {
                                "Money left"
                            } else {
                                "Money came into"
                            },
                            format_date(&r.date)
                        ),
                        cx,
                    ))
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(field(
                                if r.amount < 0 {
                                    "Charge it to"
                                } else {
                                    "Record it as"
                                },
                                Select::new(&self.post_account).placeholder("Choose an account"),
                                None,
                                cx,
                            ))
                            .child(hint(&suspense, cx)),
                    )
                    .child(field(
                        "Fund",
                        Select::new(&self.post_fund).placeholder("Choose a fund"),
                        self.post_error.clone(),
                        cx,
                    ))
                    .child(field(
                        "Grant",
                        Select::new(&self.post_grant).placeholder("No grant"),
                        None,
                        cx,
                    ))
                    .child(field("Memo", Input::new(&self.post_memo), None, cx))
                    .when_some(self.post_failure.as_ref(), |v, f| {
                        v.child(failure_box(f, cx))
                    })
                    .child(form_buttons(
                        "post",
                        "Post and match",
                        |panel: &mut Self, _, cx| panel.submit_post(cx),
                        |panel: &mut Self| panel.post = None,
                        cx,
                    )),
            )
    }
}

/// `AutoMatchReport` (`$bankId.tsx:357-378`).
fn auto_report(lines: &[String; 3], p: &Palette) -> Div {
    v_flex()
        .py(px(12.0))
        .gap(px(6.0))
        .border_t(px(space::RULE_HEAVY))
        .border_color(p.ink_navy.to_hsla())
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Auto-match finished"),
        )
        .child(
            h_flex()
                .gap(px(24.0))
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .child(match_line(LineStatus::Auto, p))
                        .child(lines[0].clone()),
                )
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .child(match_line(LineStatus::Ambiguous, p))
                        .child(lines[1].clone()),
                )
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .child(match_line(LineStatus::Unmatched, p))
                        .child(lines[2].clone()),
                ),
        )
}

/// The import preview (`$bankId.tsx:638-707`).
fn preview_view(pv: &ImportPreview, p: &Palette) -> Div {
    let cell = |label: &str, value: String| {
        v_flex()
            .flex_1()
            .p(px(8.0))
            .child(
                div()
                    .text_size(px(text::LABEL))
                    .text_color(p.ink_muted.to_hsla())
                    .child(label.to_owned()),
            )
            .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
    };
    v_flex()
        .border_t(px(space::RULE_HEAVY))
        .border_color(p.ink_navy.to_hsla())
        .child(
            h_flex()
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(cell("Detected", format_label(pv.format).to_owned()))
                .child(cell("Rows to import", pv.rows.len().to_string()))
                .child(cell("Already on this statement", pv.repeats.to_string()))
                .child(cell("Money in", format_minor(pv.money_in)))
                .child(cell("Money out", format_minor(pv.money_out))),
        )
        .children(pv.rows.iter().take(6).map(|r| {
            h_flex()
                .py(px(6.0))
                .gap(px(12.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(div().w(px(110.0)).child(format_date(&r.date)))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(r.description.clone())
                        .when_some(r.reference.clone(), |v, re| {
                            v.child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(p.ink_muted.to_hsla())
                                    .child(re),
                            )
                        }),
                )
                .child(
                    div()
                        .w(px(140.0))
                        .flex()
                        .justify_end()
                        .child(format_signed(r.amount)),
                )
        }))
        .when(pv.rows.len() > 6, |v| {
            v.child(muted(format!("and {} more", pv.rows.len() - 6), p))
        })
        .when(!pv.skipped.is_empty(), |v| {
            v.child(
                v_flex()
                    .pt(px(12.0))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                        "{} {} skipped",
                        pv.skipped.len(),
                        if pv.skipped.len() == 1 { "row" } else { "rows" }
                    )))
                    .children(pv.skipped.iter().map(|s| muted(s.clone(), p))),
            )
        })
        .when(pv.rows.is_empty(), |v| {
            v.child(
                div()
                    .py(px(12.0))
                    .text_color(p.rejection_red.to_hsla())
                    .child("No rows could be read. Each row needs a date and an amount."),
            )
        })
}

impl Render for ReconcilePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        if self.books.is_none() {
            return v_flex()
                .gap(px(space::SECTION))
                .child(page_title("Reconcile"))
                .child(muted("No books loaded for this organisation.", p))
                .into_any_element();
        }
        match self.page {
            Page::Accounts => self.accounts_page(cx),
            Page::Bank(_) => self.bank_page(cx),
        }
    }
}
