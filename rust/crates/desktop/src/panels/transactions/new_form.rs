//! Record a transaction (`transactions/new.tsx`): what moved, between which nodes (grouped by
//! node type), amount and date, notes, and optional proof files that upload once the
//! transaction is saved; then the new transaction's page, with the failed-uploads notice when a
//! file did not go up (`?failed=n`).

use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectGroup, SelectState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, ClickEvent, Context, Entity, FontWeight, IntoElement,
    ParentElement as _, PathPromptOptions, SharedString, Styled as _, Subscription, Window, div,
    px,
};
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::types::VerificationType;

use super::{Awaiting, TransactionsPanel, TransactionsView};
use crate::backend::FlowWrite;
use crate::panels::bills::{new_pick, picked};
use crate::panels::dashboard::small_muted;
use crate::panels::{Pick, PickState, field, hint, page_title, section_header};
use crate::proofs::{LinkKind, ProofDetails, ProofFile, new_verification_for};
use crate::statements::today_utc;
use crate::theme::{palette, space, text};
use crate::transactions::{
    CATEGORIES, CURRENCIES, Field, NODES_FIRST_BODY, NODES_FIRST_TITLE, NewForm, added_transaction,
    default_date, node_groups, save_failure, save_label, transaction_ids,
};
use crate::verification::{
    MAX_FILES, SCREEN_MAX_MB, accepted, describe_accepted, format_file_size, new_form_choices,
    read_proof_file,
};

/// A select over nodes grouped by type.
type NodePick = Entity<SelectState<SearchableVec<SelectGroup<Pick>>>>;

/// The form's widgets and its queued proof files.
pub(super) struct Inputs {
    description: Entity<InputState>,
    amount: Entity<InputState>,
    date: Entity<InputState>,
    reference: Entity<InputState>,
    notes: Entity<InputState>,
    category: PickState,
    currency: PickState,
    from: NodePick,
    to: NodePick,
    proof_kind: PickState,
    kind: VerificationType,
    files: Vec<ProofFile>,
    proof_error: Option<String>,
    nodes: usize,
}

fn node_pick(window: &mut Window, cx: &mut gpui_kit::App) -> NodePick {
    cx.new(|cx| SelectState::new(SearchableVec::new(Vec::new()), None, window, cx))
}

impl Inputs {
    pub(super) fn new(window: &mut Window, cx: &mut Context<TransactionsPanel>) -> Self {
        let input =
            |placeholder: &str, window: &mut Window, cx: &mut Context<TransactionsPanel>| {
                let placeholder = placeholder.to_owned();
                cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
            };
        let categories: Vec<Pick> = std::iter::once(Pick::new("", "Uncategorised"))
            .chain(CATEGORIES.iter().map(|c| Pick::new(*c, *c)))
            .collect();
        let currencies: Vec<Pick> = CURRENCIES
            .iter()
            .map(|(code, name)| Pick::new(*code, format!("{code} \u{b7} {name}")))
            .collect();
        let kinds: Vec<Pick> = new_form_choices()
            .into_iter()
            .map(|(k, label)| Pick::new(kind_value(k), label))
            .collect();
        Self {
            description: input("Borehole drilling, phase 1", window, cx),
            amount: input("e.g. 250,000", window, cx),
            date: input("dd/mm/yyyy", window, cx),
            reference: input("Bank, M-Pesa or invoice reference", window, cx),
            notes: input(
                "Anything a funder or auditor should know about this movement",
                window,
                cx,
            ),
            category: new_pick(window, cx, categories, Some("")),
            currency: new_pick(window, cx, currencies, Some("KES")),
            from: node_pick(window, cx),
            to: node_pick(window, cx),
            proof_kind: new_pick(window, cx, kinds, Some("receipt")),
            kind: VerificationType::Receipt,
            files: Vec::new(),
            proof_error: None,
            nodes: 0,
        }
    }

    pub(super) fn subscriptions(
        &self,
        window: &mut Window,
        cx: &mut Context<TransactionsPanel>,
    ) -> Vec<Subscription> {
        vec![cx.subscribe_in(
            &self.proof_kind,
            window,
            |this, _, event: &SelectEvent<Vec<Pick>>, _, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Ok(kind) = serde_json::from_value(serde_json::json!(value)) {
                    this.inputs.kind = kind;
                    cx.notify();
                }
            },
        )]
    }

    /// Fills the From and To lists from the organisation's nodes.
    pub(super) fn load_nodes(
        &mut self,
        t: Option<&OrgTransparency>,
        window: &mut Window,
        cx: &mut Context<TransactionsPanel>,
    ) {
        let nodes = t.map(|t| t.flow_nodes.as_slice()).unwrap_or_default();
        self.nodes = nodes.len();
        let groups = || {
            SearchableVec::new(
                node_groups(nodes)
                    .into_iter()
                    .map(|(label, list)| {
                        SelectGroup::new(label)
                            .items(list.into_iter().map(|(id, name)| Pick::new(id, name)))
                    })
                    .collect::<Vec<_>>(),
            )
        };
        for pick in [&self.from, &self.to] {
            pick.update(cx, |s, cx| s.set_items(groups(), window, cx));
        }
    }

    fn value(input: &Entity<InputState>, cx: &gpui_kit::App) -> String {
        input.read(cx).value().to_string()
    }

    /// The form as typed and chosen.
    fn read(&self, cx: &gpui_kit::App) -> NewForm {
        let node = |pick: &NodePick| pick.read(cx).selected_value().cloned().unwrap_or_default();
        NewForm {
            description: Self::value(&self.description, cx),
            category: picked(&self.category, cx).unwrap_or_default(),
            from: node(&self.from),
            to: node(&self.to),
            amount: Self::value(&self.amount, cx),
            currency: picked(&self.currency, cx).unwrap_or_else(|| "KES".to_owned()),
            date: Self::value(&self.date, cx),
            reference: Self::value(&self.reference, cx),
            notes: Self::value(&self.notes, cx),
        }
    }
}

fn kind_value(kind: VerificationType) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

impl TransactionsPanel {
    /// A fresh form: the organisation's currency and the default date.
    pub(super) fn open_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let today = today_utc();
        let date = default_date(self.records.as_deref(), self.demo, &today);
        self.form = NewForm::new(self.currency.as_deref(), &date);
        self.errors = None;
        self.save_error = None;
        self.saving = false;
        self.uploading = false;
        self.inputs.files.clear();
        self.inputs.proof_error = None;
        let form = self.form.clone();
        for (input, value) in [
            (&self.inputs.description, ""),
            (&self.inputs.amount, ""),
            (&self.inputs.date, form.date.as_str()),
            (&self.inputs.reference, ""),
            (&self.inputs.notes, ""),
        ] {
            input.update(cx, |i, cx| i.set_value(value.to_owned(), window, cx));
        }
        let currency = if CURRENCIES.iter().any(|(c, _)| *c == form.currency) {
            form.currency.clone()
        } else {
            "KES".to_owned()
        };
        self.inputs.currency.update(cx, |s, cx| {
            s.set_selected_value(&currency, window, cx);
        });
        self.inputs.category.update(cx, |s, cx| {
            s.set_selected_value(&String::new(), window, cx);
        });
        for pick in [&self.inputs.from, &self.inputs.to] {
            pick.update(cx, |s, cx| s.set_selected_index(None, window, cx));
        }
        self.inputs.kind = VerificationType::Receipt;
        self.inputs.proof_kind.update(cx, |s, cx| {
            s.set_selected_value(&"receipt".to_owned(), window, cx);
        });
    }

    /// Validates and sends `createTransaction`; the page opens once the row comes back.
    pub(super) fn save(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.saving || self.uploading {
            return;
        }
        let today = today_utc();
        self.form = self.inputs.read(cx);
        let mut errors = self.form.validate(&today);
        errors.proof.clone_from(&self.inputs.proof_error);
        self.save_error = None;
        let Some(input) = self.form.input(&today).filter(|_| errors.is_empty()) else {
            self.errors = Some(errors);
            cx.notify();
            return;
        };
        self.errors = None;
        self.saving = true;
        self.awaiting = Some(Awaiting {
            before: transaction_ids(self.records.as_deref()),
            saved: input.clone(),
            files: std::mem::take(&mut self.inputs.files),
            kind: self.inputs.kind,
        });
        let store = self.store.clone();
        crate::panels::write_flow(
            self,
            &store,
            FlowWrite::CreateTransaction(input),
            cx,
            |this, result, cx| {
                if let Err(e) = result {
                    this.saving = false;
                    if let Some(a) = this.awaiting.take() {
                        this.inputs.files = a.files;
                    }
                    this.save_error = Some(save_failure(&e.to_string()));
                }
                this.find_saved(cx);
                cx.notify();
            },
        );
    }

    /// Once the saved row is in the records: uploads the queued files, records each, and opens
    /// the new transaction's page with how many did not go up.
    pub(super) fn find_saved(&mut self, cx: &mut Context<Self>) {
        let Some(t) = self.records.clone() else {
            return;
        };
        let Some(id) = self
            .awaiting
            .as_ref()
            .and_then(|a| added_transaction(&a.before, &t, &a.saved))
        else {
            return;
        };
        let Some(awaiting) = self.awaiting.take() else {
            return;
        };
        self.saving = false;
        let files = awaiting.files;
        if files.is_empty() {
            self.view = TransactionsView::Detail(id);
            self.rederive(cx);
            return;
        }
        let client = self.store.read(cx).proofs_client();
        let Ok((client, org)) = client else {
            let failed = files.len();
            self.open_saved(id, failed, cx);
            return;
        };
        self.uploading = true;
        cx.notify();
        let details = ProofDetails::of(awaiting.kind);
        cx.spawn(async move |this, cx| {
            let mut failed = 0;
            for file in files {
                let (client, org, link) = (client.clone(), org.clone(), id.clone());
                let uploaded = cx
                    .background_spawn(async move {
                        client
                            .upload(org.as_str(), LinkKind::Transaction, &link, &file)
                            .map(|u| crate::proofs::Attachment {
                                name: file.name.clone(),
                                uploaded: u,
                            })
                    })
                    .await;
                let Ok(attachment) = uploaded else {
                    failed += 1;
                    continue;
                };
                let record =
                    new_verification_for(LinkKind::Transaction, &id, &details, Some(&attachment));
                let Ok(submission) = this.update(cx, |this, cx| {
                    this.store.update(cx, |s, _| s.attach_proof(record))
                }) else {
                    return;
                };
                if submission.result().await.is_err() {
                    failed += 1;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.uploading = false;
                this.open_saved(id, failed, cx);
            });
        })
        .detach();
    }

    fn open_saved(&mut self, id: String, failed: usize, cx: &mut Context<Self>) {
        self.view = TransactionsView::Detail(id);
        self.rederive(cx);
        self.section
            .update(cx, |s, cx| s.set_failed_uploads(failed, cx));
    }

    /// The platform chooser for the queued proof files, each checked as the web does.
    fn choose_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let kind = self.inputs.kind;
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Choose proof files".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let Ok(Ok(Some(paths))) = chosen.await else {
                return;
            };
            let paths: Vec<PathBuf> = paths;
            let read = cx
                .background_spawn(async move {
                    paths
                        .iter()
                        .map(|p| read_proof_file(p, kind, SCREEN_MAX_MB))
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                let mut problems = Vec::new();
                let mut ok = Vec::new();
                for r in read {
                    match r {
                        Ok(file) => ok.push(file),
                        Err(problem) => problems.push(problem),
                    }
                }
                let room = MAX_FILES.saturating_sub(this.inputs.files.len());
                if ok.len() > room {
                    problems.push(format!("Only {MAX_FILES} files can be added at once."));
                }
                this.inputs.files.extend(ok.into_iter().take(room));
                this.inputs.proof_error = (!problems.is_empty()).then(|| problems.join(" "));
                cx.notify();
            });
        })
        .detach();
    }

    fn error_of(&self, f: Field) -> Option<String> {
        self.errors
            .as_ref()
            .and_then(|e| e.of(f))
            .map(str::to_owned)
    }

    fn summary(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = palette(cx.theme().mode);
        let (title, lines) = if let Some(e) = &self.save_error {
            ("Not saved".to_owned(), vec![e.clone()])
        } else {
            let e = self.errors.as_ref()?;
            (e.title().to_owned(), e.lines())
        };
        Some(
            v_flex()
                .p(px(16.0))
                .gap(px(4.0))
                .border_1()
                .border_color(p.rejection_red.to_hsla())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.rejection_red.to_hsla())
                        .child(title),
                )
                .children(lines.into_iter().map(|l| div().child(l)))
                .into_any_element(),
        )
    }

    /// The form.
    #[expect(
        clippy::too_many_lines,
        reason = "the web's form section by section, read top to bottom"
    )]
    pub(super) fn new_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let section = |title: &str, body: AnyElement, cx: &mut Context<Self>| {
            v_flex()
                .child(section_header(title, div(), cx))
                .child(div().pt(px(space::CELL)).child(body))
        };
        let nodes = if self.inputs.nodes < 2 {
            v_flex()
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(NODES_FIRST_TITLE),
                )
                .child(small_muted(NODES_FIRST_BODY, p))
                .into_any_element()
        } else {
            h_flex()
                .gap(px(space::CELL))
                .items_start()
                .child(div().flex_1().child(field(
                    "From *",
                    Select::new(&self.inputs.from).placeholder("Choose the source"),
                    self.error_of(Field::From),
                    cx,
                )))
                .child(div().flex_1().child(field(
                    "To *",
                    Select::new(&self.inputs.to).placeholder("Choose the destination"),
                    self.error_of(Field::To),
                    cx,
                )))
                .into_any_element()
        };
        let form = self.inputs.read(cx);
        let accepted_types = describe_accepted(accepted(self.inputs.kind));
        let files: Vec<AnyElement> = self
            .inputs
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                h_flex()
                    .py(px(8.0))
                    .gap(px(12.0))
                    .border_b(px(space::HAIRLINE))
                    .border_color(p.hairline.to_hsla())
                    .child(
                        v_flex()
                            .flex_1()
                            .child(div().font_weight(FontWeight::MEDIUM).child(f.name.clone()))
                            .child(small_muted(
                                &format!(
                                    "{}{}",
                                    format_file_size(f.bytes.len() as u64),
                                    if self.uploading {
                                        " \u{b7} Uploading\u{2026}"
                                    } else {
                                        ""
                                    }
                                ),
                                p,
                            )),
                    )
                    .child(
                        Button::new(SharedString::from(format!("txn-file-remove-{i}")))
                            .ghost()
                            .small()
                            .label(format!("Remove {}", f.name))
                            .disabled(self.saving || self.uploading)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if i < this.inputs.files.len() {
                                    this.inputs.files.remove(i);
                                }
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        v_flex()
            .gap(px(space::SECTION))
            .max_w(px(860.0))
            .child(div().child(Self::back(cx)))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(page_title("Record a transaction"))
                    .child(div().text_size(px(text::LEDE)).child(
                        "Record one movement of money between two nodes. It is marked as \
                         awaiting proof until receipts or statements are attached.",
                    )),
            )
            .children(self.summary(cx))
            .child(section(
                "What moved",
                h_flex()
                    .gap(px(space::CELL))
                    .items_start()
                    .child(
                        v_flex()
                            .flex_1()
                            .child(field(
                                "Description *",
                                Input::new(&self.inputs.description),
                                self.error_of(Field::Description),
                                cx,
                            ))
                            .child(hint(
                                "As it should read in the report, e.g. Borehole drilling, phase 1",
                                cx,
                            )),
                    )
                    .child(div().w(px(260.0)).child(field(
                        "Category",
                        Select::new(&self.inputs.category),
                        None,
                        cx,
                    )))
                    .into_any_element(),
                cx,
            ))
            .child(section("Between which nodes", nodes, cx))
            .child(section(
                "Amount and date",
                v_flex()
                    .gap(px(space::CELL))
                    .child(
                        h_flex()
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .child(field(
                                        "Amount *",
                                        Input::new(&self.inputs.amount),
                                        self.error_of(Field::Amount),
                                        cx,
                                    ))
                                    .when_some(
                                        form.amount_hint()
                                            .filter(|_| self.error_of(Field::Amount).is_none()),
                                        |v, h| v.child(hint(&h, cx)),
                                    ),
                            )
                            .child(div().w(px(260.0)).child(field(
                                "Currency",
                                Select::new(&self.inputs.currency),
                                None,
                                cx,
                            ))),
                    )
                    .child(
                        h_flex()
                            .gap(px(space::CELL))
                            .items_start()
                            .child(
                                v_flex()
                                    .w(px(260.0))
                                    .child(field(
                                        "Date *",
                                        Input::new(&self.inputs.date),
                                        self.error_of(Field::Date),
                                        cx,
                                    ))
                                    .child(hint("Day, month, year", cx)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .child(field(
                                        "Reference",
                                        Input::new(&self.inputs.reference),
                                        self.error_of(Field::Reference),
                                        cx,
                                    ))
                                    .child(hint(
                                        "Bank, M-Pesa or invoice reference, if there is one",
                                        cx,
                                    )),
                            ),
                    )
                    .into_any_element(),
                cx,
            ))
            .child(section(
                "Notes",
                Input::new(&self.inputs.notes).into_any_element(),
                cx,
            ))
            .child(
                v_flex()
                    .child(section_header(
                        "Proof",
                        small_muted("Optional \u{b7} you can attach it later", p),
                        cx,
                    ))
                    .child(
                        v_flex()
                            .pt(px(space::CELL))
                            .gap(px(12.0))
                            .child(field(
                                "Kind of proof",
                                div().w(px(360.0)).child(
                                    Select::new(&self.inputs.proof_kind)
                                        .disabled(!self.inputs.files.is_empty()),
                                ),
                                None,
                                cx,
                            ))
                            .child(
                                h_flex()
                                    .p(px(16.0))
                                    .gap(px(12.0))
                                    .border_2()
                                    .border_dashed()
                                    .border_color(p.field_stroke.to_hsla())
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child("Choose files to attach"),
                                            )
                                            .child(small_muted(
                                                &format!(
                                                    "{accepted_types}, up to {SCREEN_MAX_MB} MB \
                                                     each. Files upload when you save."
                                                ),
                                                p,
                                            )),
                                    )
                                    .child(
                                        Button::new("txn-browse")
                                            .outline()
                                            .label("Browse files")
                                            .disabled(
                                                self.saving || self.inputs.files.len() >= MAX_FILES,
                                            )
                                            .on_click(cx.listener(
                                                |this, _: &ClickEvent, window, cx| {
                                                    this.choose_files(window, cx);
                                                },
                                            )),
                                    ),
                            )
                            .when_some(self.inputs.proof_error.clone(), |v, e| {
                                v.child(div().text_color(p.rejection_red.to_hsla()).child(e))
                            })
                            .children(files),
                    ),
            )
            .child(
                h_flex()
                    .pt(px(20.0))
                    .gap(px(8.0))
                    .justify_end()
                    .border_t(px(3.0))
                    .border_color(p.ink_navy.to_hsla())
                    .child(
                        Button::new("txn-new-cancel")
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.show(TransactionsView::List, window, cx);
                            })),
                    )
                    .child(
                        Button::new("txn-new-save")
                            .primary()
                            .label(save_label(self.saving, self.uploading))
                            .disabled(self.saving || self.uploading)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.save(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}
