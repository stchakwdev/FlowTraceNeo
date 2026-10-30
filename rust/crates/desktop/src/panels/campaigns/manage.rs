//! One campaign on the Campaigns screen: the manage page (`campaigns/$id.tsx`) with its status
//! menu, the record-donation form (`public-donation-form.tsx` in `donation-modal.tsx`), and the
//! preview card with the link and embed code (`campaign-embed-widget.tsx`,
//! `social-share-buttons.tsx:48-79`, `embed-code-generator.tsx:140-187`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, ClipboardItem, Context, Div, FontWeight, IntoElement,
    ParentElement as _, SharedString, Styled, Window, div, px,
};

use super::{CampaignsPanel, CampaignsView, campaign_status_mark};
use crate::backend::CampaignWrite;
use crate::campaigns::{
    CampaignPage, Coverage, DonationErrors, DonationForm, DonationRow, EarmarkRow, EmbedOptions,
    NO_ACTIONS, PAYMENT_METHODS, PreviewCard, SUGGESTED_AMOUNTS, StatusAction, confirm_payment,
    embed_snippet, not_recorded, payment_label, payment_line, pledged, pledged_note,
};
use crate::panels::bills::{picked, picks};
use crate::panels::dashboard::{bar, figures_row, notice, proof_legend, small_muted, status_mark};
use crate::panels::{Pick, PickState, field, page_title, section_header};
use crate::theme::{self, Palette, palette, space, text};
use crate::web_format::format_amount;

fn load(select: &PickState, items: Vec<Pick>, value: &str, window: &mut Window, cx: &mut App) {
    let index = items
        .iter()
        .position(|p| p.value == value)
        .map(IndexPath::new);
    select.update(cx, |s: &mut SelectState<Vec<Pick>>, cx| {
        s.set_items(items, window, cx);
        s.set_selected_index(index, window, cx);
    });
}

impl CampaignsPanel {
    /// Fills the donation form's pickers: payment methods, and the open campaign's earmarks
    /// with "(this campaign)" on its own node (`public-donation-form.tsx:233-261`).
    pub(super) fn load_donation_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let methods: Vec<(String, String)> = PAYMENT_METHODS
            .iter()
            .map(|(v, l)| ((*v).to_owned(), (*l).to_owned()))
            .collect();
        let form = self.donation.clone().unwrap_or_default();
        load(
            &self.method_pick,
            picks(&methods, None),
            &form.payment_method,
            window,
            cx,
        );
        let earmarks: Vec<(String, String)> = self
            .page
            .iter()
            .flat_map(|page| page.report.earmarks.iter())
            .map(|e| {
                let mark = if e.is_target { " (this campaign)" } else { "" };
                (e.id.clone(), format!("{}{mark}", e.name))
            })
            .collect();
        load(
            &self.earmark_pick,
            picks(&earmarks, Some("Where it is needed most")),
            form.earmark.as_deref().unwrap_or(""),
            window,
            cx,
        );
    }

    /// Opens the record-donation form, while the campaign can receive gifts.
    pub(super) fn open_donation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.page.as_ref().is_some_and(|p| p.can_receive) {
            return;
        }
        self.donation = Some(DonationForm::default());
        self.donation_errors = DonationErrors::default();
        self.donation_failure = None;
        self.pledged = None;
        for input in [
            &self.amount,
            &self.donor_name,
            &self.donor_email,
            &self.donor_phone,
        ] {
            input.update(cx, |i, cx| i.set_value("", window, cx));
        }
        self.load_donation_choices(window, cx);
        self.amount.update(cx, |i, cx| i.focus(window, cx));
        cx.notify();
    }

    fn read_donation(&mut self, cx: &App) -> Option<DonationForm> {
        let mut form = self.donation.clone()?;
        let value = |input: &gpui_kit::Entity<gpui_kit::component::input::InputState>, cx: &App| {
            input.read(cx).value().to_string()
        };
        form.amount = value(&self.amount, cx);
        form.donor_name = value(&self.donor_name, cx);
        form.donor_email = value(&self.donor_email, cx);
        form.donor_phone = value(&self.donor_phone, cx);
        form.payment_method = picked(&self.method_pick, cx).unwrap_or_else(|| "mpesa".to_owned());
        form.earmark = picked(&self.earmark_pick, cx);
        self.donation = Some(form.clone());
        Some(form)
    }

    /// `handleSubmit` (`public-donation-form.tsx:95-131`): validate, then record as pending.
    fn submit_donation(&mut self, cx: &mut Context<Self>) {
        let (Some(form), Some(campaign)) = (
            self.read_donation(cx),
            self.page.as_ref().map(|p| p.campaign.clone()),
        ) else {
            return;
        };
        match form.to_input(&campaign) {
            Err(errors) => self.donation_errors = *errors,
            Ok(input) => {
                self.donation_errors = DonationErrors::default();
                let (amount, method) = (input.amount, form.payment_method.clone());
                self.write(
                    CampaignWrite::RecordDonation(input),
                    cx,
                    move |this, r, cx| {
                        match r {
                            Ok(_) => {
                                this.donation_failure = None;
                                this.pledged = Some((amount, method));
                            }
                            Err(e) => this.donation_failure = Some(not_recorded(&e.to_string())),
                        }
                        cx.notify();
                    },
                );
            }
        }
        cx.notify();
    }

    fn run_action(&mut self, action: StatusAction, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.page.as_ref().map(|p| p.campaign.id.clone()) else {
            return;
        };
        if action == StatusAction::DeleteDraft && !self.confirm_delete {
            self.confirm_delete = true;
            cx.notify();
            return;
        }
        self.confirm_delete = false;
        self.action_error = None;
        self.write(action.write(&id), cx, move |this, result, cx| {
            match result {
                Err(e) => this.action_error = Some(action.failed(&e.to_string())),
                Ok(_) if action == StatusAction::DeleteDraft => this.view = CampaignsView::List,
                Ok(_) => {}
            }
            cx.notify();
        });
        window.refresh();
    }

    fn confirm(&mut self, donation: &str, cx: &mut Context<Self>) {
        self.action_error = None;
        self.write(confirm_payment(donation), cx, |this, result, cx| {
            if let Err(e) = result {
                this.action_error = Some(format!("The payment was not confirmed: {e}"));
            }
            cx.notify();
        });
    }

    /// "Copy link" (`social-share-buttons.tsx:48-79`): the donate page's address.
    pub(super) fn copy_link(&mut self, cx: &mut Context<Self>) {
        if let Some(url) = self.page.as_ref().and_then(|p| p.donate_url.clone()) {
            cx.write_to_clipboard(ClipboardItem::new_string(url));
            self.copied = Some("Link copied");
        } else {
            self.copied = Some("This campaign has no public link. Make it public first.");
        }
        cx.notify();
    }

    /// "Copy embed code" (`CopyBlock`, `embed-code-generator.tsx:23-112`).
    pub(super) fn copy_embed(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = &self.page {
            let code = embed_snippet(
                &self.web_url,
                &page.campaign.id,
                Some(&page.campaign.name),
                self.embed,
            );
            cx.write_to_clipboard(ClipboardItem::new_string(code));
            self.copied = Some("Copied to your clipboard. Paste it into your site\u{2019}s HTML.");
            cx.notify();
        }
    }

    fn back(cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("back-to-campaigns")
            .ghost()
            .label("\u{2190} All campaigns  l")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.show(CampaignsView::List, window, cx);
            }))
    }

    pub(super) fn manage(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(page) = &self.page else {
            return v_flex()
                .gap(px(space::SECTION))
                .child(Self::back(cx))
                .child(notice(
                    "Campaign not found",
                    "It may have been deleted, or the link is wrong. Open it again from the \
                     campaign list.",
                    p,
                ))
                .into_any_element();
        };
        let c = &page.campaign;
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(Self::back(cx))
                    .child(page_title(&c.name))
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .text_size(px(text::BODY))
                            .text_color(p.ink_muted.to_hsla())
                            .child(campaign_status_mark(c.status, p))
                            .child(page.timing.label.clone())
                            .when_some(page.funds.clone(), |m, f| m.child(format!("Funds {f}"))),
                    )
                    .child(Self::actions(page, cx))
                    .when_some(self.action_error.clone(), |v, e| v.child(error_line(&e, p)))
                    .when(self.confirm_delete, |v| v.child(Self::delete_box(page, cx)))
                    .when_some(c.description.clone(), |v, d| {
                        v.child(div().max_w(px(760.0)).text_size(px(text::LEDE)).child(d))
                    }),
            )
            .when(self.donation.is_some(), |v| {
                v.child(self.donation_form(page, cx))
            })
            .child(
                v_flex()
                    .child(section_header(
                        "Funding coverage",
                        small_muted(&c.currency, p),
                        cx,
                    ))
                    .child(div().py(px(20.0)).child(coverage(&page.coverage, p)))
                    .child(figures_row(&page.figures, p)),
            )
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap(px(space::SECTION))
                            .child(Self::donations(page, cx))
                            .child(spending(page, cx)),
                    )
                    .child(
                        v_flex()
                            .w(px(360.0))
                            .flex_shrink_0()
                            .gap(px(space::SECTION))
                            .child(details(page, cx))
                            .when(!page.earmarks.is_empty(), |v| {
                                v.child(
                                    v_flex()
                                        .child(section_header("Where gifts can go", div(), cx))
                                        .children(page.earmarks.iter().map(|e| earmark(e, p))),
                                )
                            }),
                    ),
            )
            .child(self.share(page, cx))
            .into_any_element()
    }

    fn actions(page: &CampaignPage, cx: &mut Context<Self>) -> Div {
        let p = palette(cx.theme().mode);
        let mut row = h_flex().gap(px(8.0)).pt(px(8.0));
        if page.can_receive {
            row = row.child(
                Button::new("record-donation")
                    .primary()
                    .label("Record donation  d")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.open_donation(window, cx);
                    })),
            );
        }
        for action in page.actions.clone() {
            let button = Button::new(SharedString::from(format!("action-{}", action.label())))
                .label(action.label())
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.run_action(action, window, cx);
                }));
            row = row.child(if action == StatusAction::DeleteDraft {
                button.danger()
            } else {
                button.outline()
            });
        }
        if page.actions.is_empty() {
            row = row.child(small_muted(NO_ACTIONS, p));
        }
        row.child(
            Button::new("open-preview")
                .ghost()
                .label("Preview and embed  v")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    if let Some(id) = this.page.as_ref().map(|p| p.campaign.id.clone()) {
                        this.show(CampaignsView::Preview(id), window, cx);
                    }
                })),
        )
    }

    fn delete_box(page: &CampaignPage, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .mt(px(8.0))
            .p(px(16.0))
            .gap(px(8.0))
            .max_w(px(560.0))
            .border_1()
            .border_color(p.rejection_red.to_hsla())
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Delete this draft?"),
            )
            .child(format!(
                "{} has not been opened to donors. Deleting it cannot be undone.",
                page.campaign.name
            ))
            .child(
                h_flex()
                    .gap(px(8.0))
                    .child(
                        Button::new("keep-draft")
                            .ghost()
                            .label("Keep draft")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.confirm_delete = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("delete-draft")
                            .danger()
                            .label("Delete draft")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.run_action(StatusAction::DeleteDraft, window, cx);
                            })),
                    ),
            )
    }

    fn donations(page: &CampaignPage, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let meta = small_muted(&format!("{} recorded", page.donations.len()), p);
        let rows: Vec<AnyElement> = page
            .donations
            .iter()
            .map(|d| Self::donation_row(d, cx))
            .collect();
        v_flex()
            .child(section_header("Donations", meta, cx))
            .when(page.donations.is_empty(), |v| {
                v.child(notice(
                    "No donations yet",
                    "Gifts appear here as they are made on the donate page or recorded with \
                     Record donation.",
                    p,
                ))
            })
            .children(rows)
    }

    fn donation_row(d: &DonationRow, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let id = d.id.clone();
        let pending = d.status == ledger_module::transparency::types::PaymentStatus::Pending;
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
                    .child(div().font_weight(FontWeight::MEDIUM).child(d.donor.clone()))
                    .when_some(d.earmark.clone(), |v, e| v.child(small_muted(&e, p))),
            )
            .child(
                div()
                    .w(px(64.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(d.date.clone()),
            )
            .child(
                v_flex()
                    .w(px(150.0))
                    .child(status_mark(
                        payment_line(d.status),
                        payment_label(d.status),
                        p,
                    ))
                    .child(small_muted(&d.method, p)),
            )
            .child(
                div()
                    .w(px(120.0))
                    .flex()
                    .justify_end()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(d.amount.clone()),
            )
            .child(div().w(px(150.0)).flex().justify_end().when(pending, |c| {
                c.child(
                    Button::new(SharedString::from(format!("confirm-{}", d.id)))
                        .outline()
                        .label("Confirm payment")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.confirm(&id, cx);
                        })),
                )
            }))
            .into_any_element()
    }

    fn donation_form(&self, page: &CampaignPage, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let c = &page.campaign;
        if let Some((amount, method)) = &self.pledged {
            return Self::pledged_view(&c.name, *amount, &c.currency, method, cx);
        }
        let header = section_header("Record a donation", small_muted(&c.name, p), cx);
        let form = self.donation.clone().unwrap_or_default();
        let e = &self.donation_errors;
        let suggested = SUGGESTED_AMOUNTS.iter().map(|&a| {
            Button::new(SharedString::from(format!("suggest-{a}")))
                .outline()
                .label(format_amount(i64::from(a) * 100))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.amount
                        .update(cx, |i, cx| i.set_value(a.to_string(), window, cx));
                    cx.notify();
                }))
        });
        v_flex()
            .child(header)
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(space::CELL))
                    .max_w(px(720.0))
                    .child(div().pb(px(8.0)).child(coverage_short(&page.coverage, p)))
                    .child(field(
                        &format!("Amount ({}) *", c.currency),
                        v_flex()
                            .gap(px(6.0))
                            .child(h_flex().gap(px(6.0)).children(suggested))
                            .child(Input::new(&self.amount)),
                        e.amount.clone(),
                        cx,
                    ))
                    .when(!page.report.earmarks.is_empty(), |v| {
                        v.child(field(
                            "Where it goes",
                            Select::new(&self.earmark_pick),
                            None,
                            cx,
                        ))
                    })
                    .child(field("Pay with", Select::new(&self.method_pick), None, cx))
                    .child(field(
                        "M-Pesa phone number (for M-Pesa)",
                        Input::new(&self.donor_phone),
                        e.donor_phone.clone(),
                        cx,
                    ))
                    .child(field(
                        "Email",
                        Input::new(&self.donor_email),
                        e.donor_email.clone(),
                        cx,
                    ))
                    .child(field("Donor name", Input::new(&self.donor_name), None, cx))
                    .child(Self::donation_checks(&form, c.allow_overflow, cx))
                    .when_some(self.donation_failure.clone(), |v, f| {
                        v.child(error_line(&f, p))
                    })
                    .child(Self::donation_buttons(form.button_label(&c.currency), cx))
                    .child(small_muted(
                        "The gift is recorded as awaiting payment until the payment is \
                         confirmed. Nothing is charged: FlowTrace records gifts, it does not \
                         take payments.",
                        p,
                    )),
            )
            .into_any_element()
    }

    /// "Give anonymously" and, unless the campaign takes overflow, the reallocation choice
    /// (`public-donation-form.tsx:304-321`).
    fn donation_checks(form: &DonationForm, allow_overflow: bool, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap(px(space::CELL))
            .child(
                Checkbox::new("donation-anonymous")
                    .label("Give anonymously: the name is not shown publicly.")
                    .checked(form.is_anonymous)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        if let Some(f) = &mut this.donation {
                            f.is_anonymous = *checked;
                        }
                        cx.notify();
                    })),
            )
            .when(!allow_overflow, |v| {
                v.child(
                    Checkbox::new("donation-reallocate")
                        .label("If the target is already met, use the gift on another project")
                        .checked(form.allow_reallocation)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            if let Some(f) = &mut this.donation {
                                f.allow_reallocation = *checked;
                            }
                            cx.notify();
                        })),
                )
            })
    }

    fn donation_buttons(label: String, cx: &mut Context<Self>) -> Div {
        h_flex()
            .gap(px(8.0))
            .child(
                Button::new("donation-give")
                    .primary()
                    .label(label)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.submit_donation(cx);
                    })),
            )
            .child(
                Button::new("donation-cancel")
                    .ghost()
                    .label("Cancel")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.donation = None;
                        cx.notify();
                    })),
            )
    }

    /// The thank-you after a gift is recorded (`public-donation-form.tsx:133-169`).
    fn pledged_view(
        name: &str,
        amount: ledger_core::Minor,
        currency: &str,
        method: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx.theme().mode);
        v_flex()
            .child(section_header(
                "Record a donation",
                small_muted(name, p),
                cx,
            ))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(text::TITLE))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .child(pledged(amount, currency)),
                    )
                    .child(status_mark(
                        payment_line(ledger_module::transparency::types::PaymentStatus::Pending),
                        "Awaiting payment",
                        p,
                    ))
                    .child(small_muted(&pledged_note(method), p))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                Button::new("give-again")
                                    .outline()
                                    .label("Give again")
                                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                        this.open_donation(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("donation-close")
                                    .ghost()
                                    .label("Close")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.donation = None;
                                        this.pledged = None;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// The link and embed code with their copy buttons, on the manage page and the preview.
    fn share(&self, page: &CampaignPage, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx.theme().mode);
        let code = embed_snippet(
            &self.web_url,
            &page.campaign.id,
            Some(&page.campaign.name),
            self.embed,
        );
        v_flex()
            .child(section_header("Share and embed", div(), cx))
            .child(
                v_flex()
                    .py(px(space::CELL))
                    .gap(px(12.0))
                    .child(small_muted("Public link", p))
                    .child(mono(
                        page.donate_url
                            .as_deref()
                            .unwrap_or("No public link: the campaign is private."),
                        p,
                    ))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                Button::new("copy-link")
                                    .outline()
                                    .label("Copy link  c")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.copy_link(cx);
                                    })),
                            )
                            .child(
                                Button::new("copy-embed")
                                    .outline()
                                    .label("Copy embed code  e")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.copy_embed(cx);
                                    })),
                            ),
                    )
                    .when_some(self.copied, |v, note| v.child(small_muted(note, p)))
                    .child(small_muted("Embed code", p))
                    .child(mono(&code, p)),
            )
    }

    /// The widget options (`CampaignWidgetEmbed`): theme, amount, trust and timing, compact;
    /// each button toggles its option, the chosen ones filled.
    fn embed_options(&self, cx: &mut Context<Self>) -> Div {
        type Toggle = fn(&mut EmbedOptions);
        let o = self.embed;
        let options: [(&'static str, &'static str, bool, Toggle); 5] = [
            ("embed-light", "Light", !o.dark, |e| e.dark = false),
            ("embed-dark", "Dark", o.dark, |e| e.dark = true),
            ("embed-amount", "Show amount", o.show_amount, |e| {
                e.show_amount = !e.show_amount;
            }),
            (
                "embed-donors",
                "Show trust and timing",
                o.show_donors,
                |e| {
                    e.show_donors = !e.show_donors;
                },
            ),
            ("embed-compact", "Compact in code", o.compact, |e| {
                e.compact = !e.compact;
            }),
        ];
        h_flex()
            .gap(px(8.0))
            .children(options.into_iter().map(|(key, label, on, toggle)| {
                let button = Button::new(key).label(label);
                if on {
                    button.primary()
                } else {
                    button.outline()
                }
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    toggle(&mut this.embed);
                    cx.notify();
                }))
            }))
    }

    pub(super) fn preview(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx.theme().mode);
        let Some(page) = &self.page else {
            return self.manage(cx);
        };
        let card = PreviewCard::of(page, &self.web_url);
        let look: &Palette = if self.embed.dark {
            &theme::DARK
        } else {
            &theme::LIGHT
        };
        let id = page.campaign.id.clone();
        v_flex()
            .gap(px(space::SECTION))
            .child(
                v_flex()
                    .gap(px(8.0))
                    .child(
                        Button::new("back-to-campaign")
                            .ghost()
                            .label(format!("\u{2190} {}", page.campaign.name))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.show(CampaignsView::Manage(id.clone()), window, cx);
                            })),
                    )
                    .child(page_title("Preview card"))
                    .child(div().text_size(px(text::LEDE)).child(
                        "The campaign as the widget shows it on another site, drawn from the \
                         same figures.",
                    )),
            )
            .child(self.embed_options(cx))
            .child(
                h_flex()
                    .gap(px(space::SECTION))
                    .items_start()
                    .child(
                        v_flex()
                            .gap(px(8.0))
                            .child(small_muted("Full", p))
                            .child(full_card(&card, self.embed, look)),
                    )
                    .child(
                        v_flex()
                            .gap(px(8.0))
                            .child(small_muted("Compact", p))
                            .child(compact_card(&card, look)),
                    ),
            )
            .child(self.share(page, cx))
            .into_any_element()
    }
}

fn error_line(message: &str, p: &Palette) -> impl IntoElement {
    div()
        .text_size(px(text::BODY))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.rejection_red.to_hsla())
        .child(message.to_owned())
}

fn mono(code: &str, p: &Palette) -> impl IntoElement {
    div()
        .p(px(12.0))
        .max_w(px(900.0))
        .border_1()
        .border_color(p.hairline.to_hsla())
        .bg(p.wash.to_hsla())
        .font_family(text::MONO_FAMILY)
        .text_size(px(12.0))
        .children(
            code.lines()
                .map(|l| div().min_h(px(16.0)).child(l.to_owned()))
                .collect::<Vec<_>>(),
        )
}

/// `CampaignCoverage` at size `lg`: the raised figure, `raised of ... target`, the percentage,
/// the bar and the key.
fn coverage(c: &Coverage, p: &Palette) -> impl IntoElement {
    v_flex()
        .gap(px(12.0))
        .child(
            h_flex()
                .justify_between()
                .items_end()
                .child(
                    v_flex()
                        .child(
                            div()
                                .text_size(px(text::HEADLINE))
                                .line_height(px(text::HEADLINE))
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .child(c.raised.clone()),
                        )
                        .child(
                            div()
                                .pt(px(6.0))
                                .text_color(p.ink_muted.to_hsla())
                                .child(format!("raised of {} target", c.target)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(text::TITLE + 6.0))
                        .font_weight(FontWeight::BOLD)
                        .child(c.percent.clone()),
                ),
        )
        .child(bar(c.bar, 16.0, p))
        .child(
            h_flex()
                .gap(px(20.0))
                .children(c.key.iter().enumerate().map(|(i, (label, value))| {
                    let swatch = if c.bar.evidenced.is_some() && i == 0 {
                        p.report_blue
                    } else if label.starts_with("Still") {
                        p.gap_grey
                    } else {
                        p.coverage_blue
                    };
                    h_flex()
                        .gap(px(6.0))
                        .text_size(px(12.0))
                        .text_color(p.ink_muted.to_hsla())
                        .child(div().w(px(10.0)).h(px(10.0)).bg(swatch.to_hsla()))
                        .child(label.clone())
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.ink_navy.to_hsla())
                                .child(value.clone()),
                        )
                })),
        )
}

/// `CampaignCoverage` at size `sm` without the key, above the donation form.
fn coverage_short(c: &Coverage, p: &Palette) -> impl IntoElement {
    v_flex()
        .gap(px(8.0))
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_size(px(text::TITLE))
                        .child(format!("{} raised of {}", c.raised, c.target)),
                )
                .child(div().font_weight(FontWeight::BOLD).child(c.percent.clone())),
        )
        .child(bar(c.bar, 10.0, p))
}

fn spending(page: &CampaignPage, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    v_flex()
        .child(section_header(
            "Recent programme spending",
            proof_legend(p),
            cx,
        ))
        .when(page.spending.is_empty(), |v| {
            v.child(notice(
                "No spending recorded yet",
                "Payments out of this campaign's projects appear here with their proof once \
                 they are recorded as transactions.",
                p,
            ))
        })
        .children(page.spending.iter().map(|s| {
            h_flex()
                .py(px(12.0))
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
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(s.description.clone()),
                        )
                        .child(small_muted(&s.detail, p)),
                )
                .child(
                    v_flex()
                        .items_end()
                        .gap(px(2.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(s.amount.clone()),
                        )
                        .child(status_mark(
                            s.status,
                            crate::dashboard::proof_label(s.status),
                            p,
                        ))
                        .when_some(s.proofs.clone(), |v, n| v.child(small_muted(&n, p))),
                )
        }))
}

fn details(page: &CampaignPage, cx: &App) -> impl IntoElement {
    let p = palette(cx.theme().mode);
    v_flex()
        .child(section_header("Details", div(), cx))
        .children(page.details.iter().map(|(k, v)| {
            h_flex()
                .py(px(10.0))
                .justify_between()
                .gap(px(16.0))
                .border_b(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .text_size(px(text::BODY))
                .child(div().text_color(p.ink_muted.to_hsla()).child(*k))
                .child(div().font_weight(FontWeight::MEDIUM).child(v.clone()))
        }))
}

fn earmark(e: &EarmarkRow, p: &Palette) -> impl IntoElement {
    v_flex()
        .py(px(12.0))
        .gap(px(6.0))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .text_size(px(text::BODY))
        .child(
            h_flex()
                .justify_between()
                .gap(px(12.0))
                .child(
                    h_flex()
                        .gap(px(8.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .when(e.is_target, |d| d.text_color(p.report_blue.to_hsla()))
                                .child(e.name.clone()),
                        )
                        .when(e.is_target, |h| h.child(small_muted("this campaign", p))),
                )
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(e.received.clone()),
                ),
        )
        .child(bar(e.bar, 8.0, p))
        .child(small_muted(&e.note, p))
}

/// The full widget (`campaign-embed-widget.tsx:139-198`) in the chosen theme.
fn full_card(card: &PreviewCard, opts: EmbedOptions, p: &Palette) -> impl IntoElement {
    v_flex()
        .w(px(400.0))
        .p(px(16.0))
        .gap(px(4.0))
        .bg(p.sheet.to_hsla())
        .text_color(p.ink_navy.to_hsla())
        .border_t(px(space::RULE_HEAVY))
        .border_l(px(space::HAIRLINE))
        .border_r(px(space::HAIRLINE))
        .border_b(px(space::HAIRLINE))
        .border_color(p.ink_navy.to_hsla())
        .child(small_muted(&card.organization, p))
        .child(
            div()
                .text_size(px(text::TITLE))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(card.name.clone()),
        )
        .when_some(card.summary.clone(), |v, s| {
            v.child(
                div()
                    .text_size(px(text::BODY))
                    .text_color(p.ink_muted.to_hsla())
                    .child(s),
            )
        })
        .when(opts.show_amount, |v| v.child(card_amount(card, p)))
        .when(opts.show_donors, |v| v.child(card_trust(card, p)))
        .when(card.give_url.is_some(), |v| {
            v.child(div().mt(px(12.0)).child(give(p, true)))
        })
        .child(
            div()
                .pt(px(10.0))
                .child(small_muted("Figures from FlowTrace", p)),
        )
}

/// The compact widget (`campaign-embed-widget.tsx:113-137`).
fn compact_card(card: &PreviewCard, p: &Palette) -> impl IntoElement {
    v_flex()
        .w(px(400.0))
        .p(px(12.0))
        .gap(px(8.0))
        .bg(p.sheet.to_hsla())
        .text_color(p.ink_navy.to_hsla())
        .border_t(px(space::RULE_HEAVY))
        .border_l(px(space::HAIRLINE))
        .border_r(px(space::HAIRLINE))
        .border_b(px(space::HAIRLINE))
        .border_color(p.ink_navy.to_hsla())
        .child(
            h_flex()
                .gap(px(12.0))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(card.name.clone()),
                        )
                        .child(small_muted(&card.compact_line, p)),
                )
                .when(card.give_url.is_some(), |h| h.child(give(p, false))),
        )
        .child(bar(card.bar, 8.0, p))
}

/// The Give link as a filled button (it opens the donate page on the web).
fn give(p: &Palette, wide: bool) -> impl IntoElement {
    div()
        .h(px(40.0))
        .px(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .when(wide, Styled::w_full)
        .bg(p.report_blue.to_hsla())
        .text_color(p.report_paper.to_hsla())
        .font_weight(FontWeight::SEMIBOLD)
        .child("Give \u{2197}")
}

/// The raised figure, percentage, bar and spent-with-proof line of the full widget.
fn card_amount(card: &PreviewCard, p: &Palette) -> Div {
    v_flex()
        .pt(px(12.0))
        .gap(px(4.0))
        .child(
            h_flex()
                .justify_between()
                .items_end()
                .child(
                    div()
                        .text_size(px(30.0))
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child(card.raised.clone()),
                )
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .child(card.percent.clone()),
                ),
        )
        .child(small_muted(&card.of_target, p))
        .child(div().pt(px(6.0)).child(bar(card.bar, 12.0, p)))
        .when_some(card.evidenced.clone(), |v, e| {
            v.child(
                h_flex()
                    .pt(px(4.0))
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(p.ink_muted.to_hsla())
                    .child(div().w(px(10.0)).h(px(10.0)).bg(p.report_blue.to_hsla()))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink_navy.to_hsla())
                            .child(e),
                    )
                    .child("spent with verified proof"),
            )
        })
}

/// The Trust Score and timing row of the full widget.
fn card_trust(card: &PreviewCard, p: &Palette) -> Div {
    h_flex()
        .mt(px(12.0))
        .border_t(px(space::HAIRLINE))
        .border_b(px(space::HAIRLINE))
        .border_color(p.hairline.to_hsla())
        .child(
            v_flex()
                .flex_1()
                .py(px(8.0))
                .child(small_muted("Trust Score", p))
                .child(
                    h_flex()
                        .items_end()
                        .child(
                            div()
                                .text_size(px(20.0))
                                .font_weight(FontWeight::BOLD)
                                .child(card.trust_score.to_string()),
                        )
                        .child(small_muted("/100", p)),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .py(px(8.0))
                .pl(px(12.0))
                .border_l(px(space::HAIRLINE))
                .border_color(p.hairline.to_hsla())
                .child(small_muted("Timing", p))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(card.timing.clone()),
                ),
        )
}
