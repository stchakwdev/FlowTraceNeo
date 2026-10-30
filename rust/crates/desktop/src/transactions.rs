//! The Transactions screens (DT3) as plain data: the list with its filters, counts, search and
//! total (`flowtrace/src/routes/_authenticated/transactions/index.tsx`), one transaction's page
//! (`transactions/$id.tsx`) and the record-a-transaction form with the web's checks
//! (`transactions/new.tsx`). No GPUI here; `crate::panels::transactions` draws it.
//!
//! Amounts are minor units, as the module keeps them; the web holds major units and formats
//! them with `formatAmount` (`crate::web_format::format_amount` is that over minor units).

use std::collections::BTreeSet;

use ledger_core::Minor;
use ledger_core::types::IsoDate;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::flows::{
    FlowNode, NewTransaction, NodeRef, NodeType, TransactionStatus, TransactionWithNodes,
};

use crate::claims::parse_day_month_year;
use crate::dashboard::{proof_label, transactions};
use crate::web_format::{format_amount, format_date, format_date_time_eat};

/// A status filter of the list (`FILTERS`, `index.tsx:20-25`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every transaction.
    #[default]
    All,
    /// `verified`.
    Evidenced,
    /// `pending`.
    AwaitingProof,
    /// `flagged`.
    Flagged,
}

impl Filter {
    /// In the web's order.
    pub const ALL: [Self; 4] = [
        Self::All,
        Self::Evidenced,
        Self::AwaitingProof,
        Self::Flagged,
    ];

    /// The label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Evidenced => "Evidenced",
            Self::AwaitingProof => "Awaiting proof",
            Self::Flagged => "Flagged",
        }
    }

    /// The stored state it keeps; none for All.
    #[must_use]
    pub const fn status(self) -> Option<TransactionStatus> {
        match self {
            Self::All => None,
            Self::Evidenced => Some(TransactionStatus::Verified),
            Self::AwaitingProof => Some(TransactionStatus::Pending),
            Self::Flagged => Some(TransactionStatus::Flagged),
        }
    }

    /// The filter a `--transactions` value or the web's `?status=` names, aliases included
    /// (`ALIASES`, `index.tsx:29-36`).
    #[must_use]
    pub fn from_alias(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "all" => Some(Self::All),
            "verified" | "evidenced" => Some(Self::Evidenced),
            "pending" | "awaiting-proof" | "awaiting" => Some(Self::AwaitingProof),
            "flagged" => Some(Self::Flagged),
            _ => None,
        }
    }
}

/// The counts behind the lede and the filters (`counts`, `index.tsx:72-79`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Every transaction.
    pub all: usize,
    /// Evidenced.
    pub verified: usize,
    /// Awaiting proof.
    pub pending: usize,
    /// Flagged.
    pub flagged: usize,
}

impl Counts {
    /// The count a filter shows.
    #[must_use]
    pub const fn of(self, filter: Filter) -> usize {
        match filter {
            Filter::All => self.all,
            Filter::Evidenced => self.verified,
            Filter::AwaitingProof => self.pending,
            Filter::Flagged => self.flagged,
        }
    }
}

/// The lede before any records arrived (`index.tsx:109`).
pub const LEDE_NONE: &str = "Every movement of money, with the proof attached to it";

/// The node name a row shows for a missing node.
pub const UNASSIGNED: &str = "Unassigned";

/// One row of the table (`TransactionTable`, `index.tsx:300-372`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The transaction.
    pub id: String,
    /// `18 Jan 2026`.
    pub date: String,
    /// What it was.
    pub description: String,
    /// Its reference, or `No reference`.
    pub reference: String,
    /// From, or `Unassigned`.
    pub from: String,
    /// To, or `Unassigned`.
    pub to: String,
    /// ISO code.
    pub currency: String,
    /// `2,500,000`.
    pub amount: String,
    /// Proofs attached.
    pub proofs: u64,
    /// The proof state, for the mark.
    pub status: TransactionStatus,
}

impl Row {
    fn of(x: &TransactionWithNodes) -> Self {
        let t = &x.transaction;
        let name = |n: &Option<NodeRef>| {
            n.as_ref()
                .map_or_else(|| UNASSIGNED.to_owned(), |n| n.name.clone())
        };
        Self {
            id: t.id.clone(),
            date: format_date(&t.transaction_date),
            description: t.description.clone(),
            reference: t
                .reference
                .clone()
                .unwrap_or_else(|| "No reference".to_owned()),
            from: name(&x.from_node),
            to: name(&x.to_node),
            currency: t.currency.clone(),
            amount: format_amount(t.amount),
            proofs: t.verification_count,
            status: t.verification_status,
        }
    }
}

/// The table's footer (`TotalLabel`, `index.tsx:254-263`), only when every row shown is in one
/// currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footer {
    /// `Total: 12 transactions` or `Shown: 1 transaction`.
    pub label: String,
    /// The currency.
    pub currency: String,
    /// The sum.
    pub amount: String,
}

/// A notice in place of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Its title.
    pub title: String,
    /// Its body.
    pub body: String,
    /// The button: `Record a transaction` or `Clear filters`.
    pub action: &'static str,
}

/// The list page (`TransactionList`, `index.tsx:60-243`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// The line under the title.
    pub lede: String,
    /// The counts of each filter.
    pub counts: Counts,
    /// The rows shown.
    pub rows: Vec<Row>,
    /// The total, when one currency.
    pub footer: Option<Footer>,
    /// A notice in place of the rows.
    pub notice: Option<Notice>,
}

impl List {
    /// The list of `t`'s transactions under `filter` and the search `query` (description,
    /// reference, category, from or to name, any case).
    #[must_use]
    pub fn of(t: &OrgTransparency, filter: Filter, query: &str) -> Self {
        let all = transactions(t);
        let mut counts = Counts::default();
        for x in &all {
            counts.all += 1;
            match x.transaction.verification_status {
                TransactionStatus::Verified => counts.verified += 1,
                TransactionStatus::Pending => counts.pending += 1,
                TransactionStatus::Flagged => counts.flagged += 1,
                TransactionStatus::Rejected => {}
            }
        }
        let needle = query.trim().to_lowercase();
        let shown: Vec<&TransactionWithNodes> = all
            .iter()
            .filter(|x| {
                filter
                    .status()
                    .is_none_or(|s| x.transaction.verification_status == s)
            })
            .filter(|x| needle.is_empty() || matches(x, &needle))
            .collect();
        let currencies: BTreeSet<&str> = shown
            .iter()
            .map(|x| x.transaction.currency.as_str())
            .collect();
        let filtered = filter != Filter::All || !query.is_empty();
        let footer = (currencies.len() == 1).then(|| {
            let total: Minor = shown.iter().map(|x| x.transaction.amount).sum();
            let n = shown.len();
            Footer {
                label: format!(
                    "{}: {n} {}",
                    if filtered { "Shown" } else { "Total" },
                    if n == 1 {
                        "transaction"
                    } else {
                        "transactions"
                    }
                ),
                currency: shown[0].transaction.currency.clone(),
                amount: format_amount(total),
            }
        });
        let notice = if counts.all == 0 {
            Some(Notice {
                title: "No transactions recorded yet".to_owned(),
                body: "Each grant received, budget allocated and payment made appears here with \
                       its proof. Record the first one to start the report."
                    .to_owned(),
                action: "Record a transaction",
            })
        } else if shown.is_empty() {
            let body = if query.is_empty() {
                "No transactions have this status.".to_owned()
            } else {
                let status = if filter == Filter::All {
                    String::new()
                } else {
                    format!("{} ", filter.label().to_lowercase())
                };
                format!("Nothing {status}matches \u{201c}{query}\u{201d}.")
            };
            Some(Notice {
                title: "No transactions match".to_owned(),
                body,
                action: "Clear filters",
            })
        } else {
            None
        };
        Self {
            lede: format!(
                "{} movements recorded \u{b7} {} awaiting proof \u{b7} {} flagged",
                counts.all, counts.pending, counts.flagged
            ),
            counts,
            rows: shown.into_iter().map(Row::of).collect(),
            footer,
            notice,
        }
    }
}

fn matches(x: &TransactionWithNodes, needle: &str) -> bool {
    let t = &x.transaction;
    [
        Some(t.description.as_str()),
        t.reference.as_deref(),
        t.category.as_deref(),
        x.from_node.as_ref().map(|n| n.name.as_str()),
        x.to_node.as_ref().map(|n| n.name.as_str()),
    ]
    .into_iter()
    .flatten()
    .any(|field| field.to_lowercase().contains(needle))
}

/// The web's node type in words (`n.type.replace(/_/g, ' ')`, shown capitalised).
#[must_use]
pub const fn node_type_word(t: NodeType) -> &'static str {
    match t {
        NodeType::Donor => "Donor",
        NodeType::Department => "Department",
        NodeType::Vendor => "Vendor",
        NodeType::Beneficiary => "Beneficiary",
        NodeType::Account => "Account",
        NodeType::Project => "Project",
    }
}

/// One side of the movement drawing (`FlowDrawing`, `$id.tsx:327-341`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    /// `From` or `To`.
    pub role: &'static str,
    /// The node's name, or `Not assigned`.
    pub name: String,
    /// The node's type, when there is a node.
    pub kind: Option<&'static str>,
}

impl Side {
    fn of(role: &'static str, n: Option<&NodeRef>) -> Self {
        Self {
            role,
            name: n.map_or_else(|| "Not assigned".to_owned(), |n| n.name.clone()),
            kind: n.map(|n| node_type_word(n.node_type)),
        }
    }
}

/// One fact of the "Record" list (`Fact`, `$id.tsx:255-290`): term, value, whether it is the
/// muted "none" wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// `Reference`.
    pub term: &'static str,
    /// Its value.
    pub value: String,
    /// The muted `None recorded` or `Uncategorised`.
    pub muted: bool,
}

/// One transaction's page (`TransactionDetail`, `$id.tsx`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The transaction.
    pub id: String,
    /// The title: its description.
    pub title: String,
    /// Its state.
    pub status: TransactionStatus,
    /// `Evidenced`.
    pub status_label: &'static str,
    /// `18 Jan 2026`.
    pub date: String,
    /// Its reference, when it has one.
    pub reference: Option<String>,
    /// ISO code.
    pub currency: String,
    /// `2,500,000`.
    pub amount: String,
    /// `about USD 19,380`, when the dollar amount is known.
    pub about_usd: Option<String>,
    /// Where the money came from.
    pub from: Side,
    /// Where it went.
    pub to: Side,
    /// The "Record" list.
    pub facts: Vec<Fact>,
    /// The notes, or `No notes recorded.`.
    pub notes: String,
    /// Whether there are notes.
    pub has_notes: bool,
}

/// The not-found page (`$id.tsx:102-130`).
pub const NOT_FOUND_TITLE: &str = "Transaction not found";
/// Its body.
pub const NOT_FOUND_BODY: &str = "It may have been removed, or the link is incomplete.";
/// Its button.
pub const NOT_FOUND_ACTION: &str = "See all transactions";

impl Detail {
    /// The page of transaction `id` of `t`; none when there is no such transaction.
    #[must_use]
    pub fn of(t: &OrgTransparency, id: &str) -> Option<Self> {
        let x = t.transactions.iter().find(|x| x.id == id)?;
        let w = ledger_module::transparency::flows::with_nodes(t, x);
        let mut facts = vec![
            Fact {
                term: "Date",
                value: format_date(&x.transaction_date),
                muted: false,
            },
            x.reference.clone().map_or(
                Fact {
                    term: "Reference",
                    value: "None recorded".to_owned(),
                    muted: true,
                },
                |r| Fact {
                    term: "Reference",
                    value: r,
                    muted: false,
                },
            ),
            x.category.clone().map_or(
                Fact {
                    term: "Category",
                    value: "Uncategorised".to_owned(),
                    muted: true,
                },
                |c| Fact {
                    term: "Category",
                    value: c,
                    muted: false,
                },
            ),
            Fact {
                term: "Status",
                value: proof_label(x.verification_status).to_owned(),
                muted: false,
            },
            Fact {
                term: "Recorded",
                value: format_date_time_eat(&x.created_at),
                muted: false,
            },
        ];
        if x.updated_at != x.created_at {
            facts.push(Fact {
                term: "Last changed",
                value: format_date_time_eat(&x.updated_at),
                muted: false,
            });
        }
        Some(Self {
            id: x.id.clone(),
            title: x.description.clone(),
            status: x.verification_status,
            status_label: proof_label(x.verification_status),
            date: format_date(&x.transaction_date),
            reference: x.reference.clone(),
            currency: x.currency.clone(),
            amount: format_amount(x.amount),
            about_usd: x
                .amount_usd
                .map(|usd| format!("about USD {}", format_amount(usd))),
            from: Side::of("From", w.from_node.as_ref()),
            to: Side::of("To", w.to_node.as_ref()),
            facts,
            notes: x
                .notes
                .clone()
                .unwrap_or_else(|| "No notes recorded.".to_owned()),
            has_notes: x.notes.is_some(),
        })
    }
}

/// The notice when files did not upload with a new transaction (`$id.tsx:166-181`).
#[must_use]
pub fn failed_uploads_title(n: usize) -> String {
    format!(
        "{n} {} did not upload",
        if n == 1 { "file" } else { "files" }
    )
}

/// Its body.
pub const FAILED_UPLOADS_BODY: &str = "The transaction was saved. Attach the missing proof below.";

/// `CATEGORIES` (`new.tsx:37-49`).
pub const CATEGORIES: [&str; 11] = [
    "Funding",
    "Budget",
    "Programs",
    "Operations",
    "Salaries",
    "Transport",
    "Equipment",
    "Training",
    "Supplies",
    "Utilities",
    "Other",
];

/// `CURRENCIES` (`new.tsx:51-57`).
pub const CURRENCIES: [(&str, &str); 5] = [
    ("KES", "Kenyan shilling"),
    ("USD", "US dollar"),
    ("UGX", "Ugandan shilling"),
    ("TZS", "Tanzanian shilling"),
    ("NGN", "Nigerian naira"),
];

/// `NODE_TYPE_LABEL` (`new.tsx:68-75`): a group heading of the From and To lists.
#[must_use]
pub const fn node_group_label(t: NodeType) -> &'static str {
    match t {
        NodeType::Donor => "Donors",
        NodeType::Account => "Accounts",
        NodeType::Department => "Departments",
        NodeType::Project => "Projects",
        NodeType::Vendor => "Vendors",
        NodeType::Beneficiary => "Beneficiaries",
    }
}

/// The From and To lists (`nodeGroups`, `new.tsx:192-200`): the nodes by name (`getNodes`,
/// `localeCompare`), grouped by type in the order the types first appear.
#[must_use]
pub fn node_groups(nodes: &[FlowNode]) -> Vec<(&'static str, Vec<(String, String)>)> {
    let mut sorted: Vec<&FlowNode> = nodes.iter().collect();
    sorted.sort_by_key(|n| n.name.to_lowercase());
    let mut groups: Vec<(NodeType, Vec<(String, String)>)> = Vec::new();
    for n in sorted {
        match groups.iter_mut().find(|(t, _)| *t == n.node_type) {
            Some((_, list)) => list.push((n.id.clone(), n.name.clone())),
            None => groups.push((n.node_type, vec![(n.id.clone(), n.name.clone())])),
        }
    }
    groups
        .into_iter()
        .map(|(t, list)| (node_group_label(t), list))
        .collect()
}

/// The section shown instead of the lists with fewer than two nodes (`new.tsx:376-392`).
pub const NODES_FIRST_TITLE: &str = "Add nodes first";
/// Its body.
pub const NODES_FIRST_BODY: &str = "A transaction moves money between two nodes, such as a donor \
                                    and a project. Add them on the Flows page.";

/// A field of the form, in the order the error summary lists them (`FIELD_ORDER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    /// What it was for.
    Description,
    /// Category.
    Category,
    /// From.
    From,
    /// To.
    To,
    /// Amount.
    Amount,
    /// Currency.
    Currency,
    /// Date.
    Date,
    /// Reference.
    Reference,
    /// Notes.
    Notes,
}

impl Field {
    /// `FIELD_LABEL` (`new.tsx:112-122`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Description => "Description",
            Self::Category => "Category",
            Self::From => "From",
            Self::To => "To",
            Self::Amount => "Amount",
            Self::Currency => "Currency",
            Self::Date => "Date",
            Self::Reference => "Reference",
            Self::Notes => "Notes",
        }
    }
}

/// `parseAmount` (`new.tsx:136-140`) into minor units: commas and spaces dropped, then digits
/// with at most two decimals; `None` for anything else. Read from the text, not through a
/// float, so `1,250.50` is exactly 125050.
#[must_use]
pub fn parse_amount(raw: &str) -> Option<Minor> {
    let cleaned: String = raw
        .chars()
        .filter(|c| *c != ',' && !c.is_whitespace())
        .collect();
    let (whole, cents) = cleaned.split_once('.').unwrap_or((&cleaned, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if cleaned.contains('.')
        && (cents.is_empty() || cents.len() > 2 || !cents.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let whole: Minor = whole.parse().ok()?;
    let cents: Minor = match cents.len() {
        0 => 0,
        1 => cents.parse::<Minor>().ok()? * 10,
        _ => cents.parse().ok()?,
    };
    whole.checked_mul(100)?.checked_add(cents)
}

/// `formatAmount(n, n % 1 ? 2 : 0)` of minor units: two decimals only when there are cents.
#[must_use]
pub fn format_amount_exact(minor: Minor) -> String {
    let whole = format_amount(minor - minor % 100);
    if minor % 100 == 0 {
        whole
    } else {
        format!("{whole}.{:02}", (minor % 100).abs())
    }
}

/// The record-a-transaction form as typed (`Fields`, `new.tsx:79-89`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewForm {
    /// What it was for.
    pub description: String,
    /// A category, or blank for none.
    pub category: String,
    /// The source node id.
    pub from: String,
    /// The destination node id.
    pub to: String,
    /// The amount as typed.
    pub amount: String,
    /// ISO code.
    pub currency: String,
    /// `dd/mm/yyyy`.
    pub date: String,
    /// Reference.
    pub reference: String,
    /// Notes.
    pub notes: String,
}

/// What is wrong with the form, field by field, in [`Field`] order, and the proof files' line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Errors {
    /// One line per field, in summary order.
    pub fields: Vec<(Field, String)>,
    /// The proof files' problem.
    pub proof: Option<String>,
}

impl Errors {
    /// The error of `field`.
    #[must_use]
    pub fn of(&self, field: Field) -> Option<&str> {
        self.fields
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, e)| e.as_str())
    }

    /// Whether nothing is wrong.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.proof.is_none()
    }

    /// The summary's title: `Fix this before saving` or `Fix these before saving`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        if self.fields.len() + usize::from(self.proof.is_some()) == 1 {
            "Fix this before saving"
        } else {
            "Fix these before saving"
        }
    }

    /// The summary's lines: `Description: Describe what this money was for.`, then the proof's.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.fields
            .iter()
            .map(|(f, e)| format!("{}: {e}", f.label()))
            .chain(self.proof.iter().map(|p| format!("Proof: {p}")))
            .collect()
    }
}

/// The summary when saving failed (`new.tsx:274-281`): `Not saved` and this line.
#[must_use]
pub fn save_failure(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        "The transaction was not saved. Your entries are still here; try again.".to_owned()
    } else {
        format!("The transaction was not saved: {message}. Your entries are still here; try again.")
    }
}

/// The save button (`new.tsx:630-637`).
#[must_use]
pub const fn save_label(saving: bool, uploading: bool) -> &'static str {
    if saving {
        "Saving\u{2026}"
    } else if uploading {
        "Uploading proof\u{2026}"
    } else {
        "Save transaction"
    }
}

impl NewForm {
    /// A fresh form (`new.tsx:173-183`): the organisation's currency, else KES, and the date
    /// `default_date`.
    #[must_use]
    pub fn new(currency: Option<&str>, default_date: &IsoDate) -> Self {
        Self {
            currency: currency.unwrap_or("KES").to_owned(),
            date: crate::claims::iso_to_day_month_year(default_date),
            ..Self::default()
        }
    }

    /// `validate` (`new.tsx:142-163`) with `today` as the latest date allowed.
    #[must_use]
    pub fn validate(&self, today: &IsoDate) -> Errors {
        let mut e = Vec::new();
        let description = self.description.trim();
        if description.is_empty() {
            e.push((Field::Description, "Describe what this money was for."));
        } else if description.chars().count() < 3 {
            e.push((
                Field::Description,
                "Use at least 3 characters so it can be found later.",
            ));
        }
        if self.from.is_empty() {
            e.push((Field::From, "Choose where the money came from."));
        }
        if self.to.is_empty() {
            e.push((Field::To, "Choose where the money went."));
        } else if self.to == self.from {
            e.push((Field::To, "Money must move between two different nodes."));
        }
        let mut fields: Vec<(Field, String)> =
            e.into_iter().map(|(f, s)| (f, s.to_owned())).collect();
        if self.amount.trim().is_empty() {
            fields.push((Field::Amount, "Enter the amount that moved.".to_owned()));
        } else {
            match parse_amount(&self.amount) {
                None => fields.push((
                    Field::Amount,
                    "Enter a number, like 250000 or 1,250.50.".to_owned(),
                )),
                Some(n) if n <= 0 => fields.push((
                    Field::Amount,
                    "The amount must be more than zero.".to_owned(),
                )),
                Some(_) => {}
            }
        }
        match parse_day_month_year(&self.date) {
            None => fields.push((Field::Date, "Enter the date the money moved.".to_owned())),
            Some(Err(message)) => fields.push((Field::Date, message)),
            Some(Ok(day)) if day.as_str() > today.as_str() => {
                fields.push((Field::Date, "The date cannot be in the future.".to_owned()))
            }
            Some(Ok(_)) => {}
        }
        if self.reference.chars().count() > 80 {
            fields.push((
                Field::Reference,
                "Keep the reference under 80 characters.".to_owned(),
            ));
        }
        fields.sort_by_key(|(f, _)| *f);
        Errors {
            fields,
            proof: None,
        }
    }

    /// The hint under the amount (`new.tsx:487-491`): `KES 1,250.50` once it reads as more
    /// than zero.
    #[must_use]
    pub fn amount_hint(&self) -> Option<String> {
        parse_amount(&self.amount)
            .filter(|n| *n > 0)
            .map(|n| format!("{} {}", self.currency, format_amount_exact(n)))
    }

    /// The `createTransaction` input (`new.tsx:223-238`); none while [`NewForm::validate`]
    /// finds something.
    #[must_use]
    pub fn input(&self, today: &IsoDate) -> Option<NewTransaction> {
        if !self.validate(today).is_empty() {
            return None;
        }
        let typed = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        Some(NewTransaction {
            from_node_id: Some(self.from.clone()),
            to_node_id: Some(self.to.clone()),
            amount: parse_amount(&self.amount)?,
            currency: self.currency.clone(),
            exchange_rate: None,
            amount_usd: None,
            description: self.description.trim().to_owned(),
            category: Some(self.category.clone()).filter(|c| !c.is_empty()),
            reference: typed(&self.reference),
            notes: typed(&self.notes),
            verification_status: TransactionStatus::Pending,
            transaction_date: parse_day_month_year(&self.date)?.ok()?.as_str().to_owned(),
        })
    }
}

/// `defaultDateISO` (`new.tsx:126-134`): the demo's latest movement, so a new entry sits in the
/// snapshot it reads as of; a live organisation's today.
#[must_use]
pub fn default_date(t: Option<&OrgTransparency>, demo: bool, today: &IsoDate) -> IsoDate {
    if !demo {
        return today.clone();
    }
    t.and_then(|t| {
        t.transactions
            .iter()
            .map(|x| x.transaction_date.as_str())
            .max()
    })
    .and_then(|d| IsoDate::parse(d.get(..10)?).ok())
    .unwrap_or_else(|| today.clone())
}

/// The ids of `t`'s transactions, to tell which one a save added ([`added_transaction`]).
#[must_use]
pub fn transaction_ids(t: Option<&OrgTransparency>) -> BTreeSet<String> {
    t.map(|t| t.transactions.iter().map(|x| x.id.clone()).collect())
        .unwrap_or_default()
}

/// The transaction `t` holds that `before` did not and that matches what was saved: the module's
/// reducer returns no id, so the screen finds its new row when the records come back.
#[must_use]
pub fn added_transaction(
    before: &BTreeSet<String>,
    t: &OrgTransparency,
    saved: &NewTransaction,
) -> Option<String> {
    t.transactions
        .iter()
        .rev()
        .find(|x| {
            !before.contains(&x.id)
                && x.description == saved.description
                && x.amount == saved.amount
                && x.transaction_date == saved.transaction_date
        })
        .map(|x| x.id.clone())
}

#[cfg(test)]
mod tests;
