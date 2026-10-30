//! The Access groups screen's view models (DT4), plain Rust with no GPUI: the web's
//! `flowtrace/src/routes/_authenticated/settings/access-groups.tsx` over the module's access
//! groups and group members (`ledger_module::transparency::types::{AccessGroup, GroupMember}`).
//!
//! The module checks who may write, that a default group is not deleted and that the group
//! exists (`plan::create_access_group` and the rest); the form checks the web makes before it
//! sends (name length, slug pattern, duplicate names, slugs and emails, an end date not in the
//! past) live here with the web's texts.

use ledger_core::types::{IsoDate, Nullish};
use ledger_module::transparency::types::{
    AccessGroup, AccessGroupPatch, AccessGroupPermissions, GroupMember, NewAccessGroup, NodeScope,
};

use crate::claims::parse_day_month_year;
use crate::settings::is_slug;
use crate::web_format::format_date;

#[cfg(test)]
mod tests;

/// The page's lede (`access-groups.tsx:104-108`).
#[must_use]
pub fn lede(org_name: Option<&str>) -> String {
    format!(
        "Who sees what in {}. Each group sees its own depth of your flows, and FlowTrace holds the line.",
        org_name.unwrap_or("your organisation")
    )
}

/// The table's footnote (`access-groups.tsx:249-253`).
pub const FOOTNOTE: &str = "Proofs: \u{201c}Originals\u{201d} means the receipts and documents themselves; \u{201c}Summaries\u{201d} means type, date and status only. Default groups can be edited but not deleted.";

/// The empty state (`access-groups.tsx:146-178`): its title and text, and the standard-groups
/// action.
pub const NO_GROUPS: (&str, &str) = (
    "No access groups yet",
    "Groups decide who sees your flows and at what depth. The standard five run from your board to the public.",
);

/// The empty state's first action.
pub const CREATE_STANDARD: &str = "Create the standard groups";

/// The column heads, in order (`access-groups.tsx:190-224`).
pub const COLUMNS: [&str; 9] = [
    "Group",
    "Sees",
    "Amounts",
    "Proofs",
    "Who handled it",
    "Edits flows",
    "Edits transactions",
    "Manages people",
    "Members",
];

/// `SCOPE_LABEL` (`access-groups.tsx:61-65`).
#[must_use]
pub const fn scope_label(scope: NodeScope) -> &'static str {
    match scope {
        NodeScope::All => "All flows",
        NodeScope::Assigned => "Assigned nodes",
        NodeScope::Downstream => "What their money funds",
    }
}

/// `SCOPE_HINT` (`access-groups.tsx:67-71`).
#[must_use]
pub const fn scope_hint(scope: NodeScope) -> &'static str {
    match scope {
        NodeScope::All => "Every node in your flows.",
        NodeScope::Assigned => "Only the nodes you assign to this group.",
        NodeScope::Downstream => "The nodes downstream of the money they gave.",
    }
}

/// The three scopes in the select's order.
pub const SCOPES: [NodeScope; 3] = [NodeScope::All, NodeScope::Assigned, NodeScope::Downstream];

/// How much of a proof a group sees (`ProofDepth`, `access-groups.tsx:73-76`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofDepth {
    /// No proofs.
    None,
    /// Type, date and status.
    Summary,
    /// The files themselves.
    Full,
}

impl ProofDepth {
    /// The three in the select's order.
    pub const ALL: [Self; 3] = [Self::None, Self::Summary, Self::Full];

    /// `proofDepth(p)`.
    #[must_use]
    pub const fn of(p: &AccessGroupPermissions) -> Self {
        if p.can_view_full_proofs {
            Self::Full
        } else if p.can_view_proofs {
            Self::Summary
        } else {
            Self::None
        }
    }

    /// The select's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No proofs",
            Self::Summary => "Summaries",
            Self::Full => "Originals",
        }
    }

    /// The hint under the select (`access-groups.tsx:1219-1223`).
    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::None => "They see no proof attached to payments.",
            Self::Summary => "Type, date and status of each proof.",
            Self::Full => "Receipts, photos and documents themselves.",
        }
    }

    /// `p` with this depth set (`setDepth`).
    #[must_use]
    pub const fn apply(self, p: AccessGroupPermissions) -> AccessGroupPermissions {
        AccessGroupPermissions {
            can_view_proofs: !matches!(self, Self::None),
            can_view_full_proofs: matches!(self, Self::Full),
            ..p
        }
    }
}

/// A table cell that is a tick, a dash or a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// `YesNo` true.
    Yes,
    /// `YesNo` false.
    No,
    /// A word.
    Text(&'static str),
}

impl Cell {
    const fn yes_no(v: bool) -> Self {
        if v { Self::Yes } else { Self::No }
    }
}

/// One row of "What each group can see and do" (`GroupRow`, `access-groups.tsx:325-450`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    /// The group.
    pub id: String,
    /// Name.
    pub name: String,
    /// `Default · Public`, `Default`, `Public` or `Custom`.
    pub kind: String,
    /// `Nothing`, the scope, or the scope with `, names only`.
    pub sees: String,
    /// Amounts.
    pub amounts: Cell,
    /// `Originals`, `Summaries` or no.
    pub proofs: Cell,
    /// Who handled it.
    pub executor: Cell,
    /// Edits flows.
    pub edits_flows: Cell,
    /// Edits transactions.
    pub edits_transactions: Cell,
    /// `Members and groups`, `Members` or no.
    pub manages: Cell,
    /// How many people are in it.
    pub members: usize,
    /// Custom groups only.
    pub deletable: bool,
}

impl GroupRow {
    /// The row of `g` with `members` in it.
    #[must_use]
    pub fn of(g: &AccessGroup, members: usize) -> Self {
        let p = &g.permissions;
        let kind = [
            g.is_default.then_some("Default"),
            g.is_public.then_some("Public"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
        Self {
            id: g.id.clone(),
            name: g.name.clone(),
            kind: if kind.is_empty() {
                "Custom".to_owned()
            } else {
                kind
            },
            sees: if !p.can_view_nodes {
                "Nothing".to_owned()
            } else if p.can_view_amounts {
                scope_label(g.node_scope).to_owned()
            } else {
                format!("{}, names only", scope_label(g.node_scope))
            },
            amounts: Cell::yes_no(p.can_view_amounts),
            proofs: match ProofDepth::of(p) {
                ProofDepth::Full => Cell::Text("Originals"),
                ProofDepth::Summary => Cell::Text("Summaries"),
                ProofDepth::None => Cell::No,
            },
            executor: Cell::yes_no(p.can_view_executor),
            edits_flows: Cell::yes_no(p.can_edit_nodes),
            edits_transactions: Cell::yes_no(p.can_edit_transactions),
            manages: if p.can_manage_groups {
                Cell::Text("Members and groups")
            } else if p.can_manage_members {
                Cell::Text("Members")
            } else {
                Cell::No
            },
            members,
            deletable: !g.is_default,
        }
    }
}

/// The groups lowest priority first (`accessGroupsInOrder`,
/// `flowtrace/src/lib/ledger/transparency-model.ts:1099-1100`; a stable sort keeps equal ones in
/// row order).
#[must_use]
pub fn in_order(groups: &[AccessGroup]) -> Vec<&AccessGroup> {
    let mut list: Vec<&AccessGroup> = groups.iter().collect();
    list.sort_by_key(|g| g.priority);
    list
}

/// The table's rows.
#[must_use]
pub fn rows(groups: &[AccessGroup], members: &[GroupMember]) -> Vec<GroupRow> {
    in_order(groups)
        .into_iter()
        .map(|g| GroupRow::of(g, members.iter().filter(|m| m.group_id == g.id).count()))
        .collect()
}

/// A person who can be named: an organisation member ([`crate::backend::OrgMember`]).
pub use crate::backend::OrgMember as Person;

/// How long a membership lasts (`ChainLine` status in the members list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Term {
    /// No end date (`active`).
    Open,
    /// Ends later (`pending`).
    Limited,
    /// Ended (`revoked`).
    Expired,
}

/// The members legend (`access-groups.tsx:733-743`).
pub const LEGEND: [(Term, &str); 3] = [
    (Term::Open, "No end date"),
    (Term::Limited, "Time-limited"),
    (Term::Expired, "Expired, no access"),
];

/// One member row (`MembersSection`, `access-groups.tsx:652-728`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    /// Membership id.
    pub id: String,
    /// The person's name, else the invited address, else the user id.
    pub name: String,
    /// `a@b.org · Member since 4 Feb 2026`.
    pub sub: String,
    /// Notes.
    pub notes: Option<String>,
    /// The term.
    pub term: Term,
    /// `Expired 1 Jan 2026`, `Until 30 Jun 2027` or `No end date`.
    pub chip: String,
}

impl MemberRow {
    /// The row of `m`, at `now` (an ISO timestamp), with names from `people`.
    #[must_use]
    pub fn of(m: &GroupMember, people: &[Person], now: &str) -> Self {
        let full_name = m
            .user_id
            .as_ref()
            .and_then(|id| people.iter().find(|p| &p.user_id == id))
            .map(|p| p.name.clone());
        let email = m.user_email.clone().filter(|e| !e.is_empty());
        let name = full_name
            .clone()
            .or_else(|| email.clone())
            .or_else(|| m.user_id.clone())
            .unwrap_or_default();
        let prefix = match (&full_name, &email) {
            (Some(_), Some(e)) => format!("{e} \u{b7} "),
            _ => String::new(),
        };
        let who = if m.user_id.is_some() {
            "Member"
        } else {
            "Invited"
        };
        let until = m.valid_until.as_deref().filter(|u| !u.is_empty());
        let (term, chip) = match until {
            Some(u) if u < now => (Term::Expired, format!("Expired {}", format_date(u))),
            Some(u) => (Term::Limited, format!("Until {}", format_date(u))),
            None => (Term::Open, "No end date".to_owned()),
        };
        Self {
            id: m.id.clone(),
            name,
            sub: format!("{prefix}{who} since {}", format_date(&m.valid_from)),
            notes: m.notes.clone().filter(|n| !n.is_empty()),
            term,
            chip,
        }
    }
}

/// A group's members, newest first (`groupMembersWithProfiles`,
/// `transparency-model.ts:1111-1128`).
#[must_use]
pub fn member_rows(
    members: &[GroupMember],
    group_id: &str,
    people: &[Person],
    now: &str,
) -> Vec<MemberRow> {
    let mut list: Vec<&GroupMember> = members.iter().filter(|m| m.group_id == group_id).collect();
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    list.into_iter()
        .map(|m| MemberRow::of(m, people, now))
        .collect()
}

/// `Members of {group}` and its count (`access-groups.tsx:616-626`).
#[must_use]
pub fn members_meta(count: usize) -> String {
    format!("{count} {}", if count == 1 { "member" } else { "members" })
}

/// The empty members notice.
pub const NO_MEMBERS: (&str, &str) = (
    "No one is in this group yet",
    "Add a member of your organisation, or invite someone by email. An invite is recorded here; no email is sent.",
);

/// The people of the organisation not yet in the group (`AddMemberForm`'s `open`), by name.
#[must_use]
pub fn addable<'a>(
    people: &'a [Person],
    members: &[GroupMember],
    group_id: &str,
) -> Vec<&'a Person> {
    let mut open: Vec<&Person> = people
        .iter()
        .filter(|p| {
            !members
                .iter()
                .any(|m| m.group_id == group_id && m.user_id.as_deref() == Some(&p.user_id))
        })
        .collect();
    open.sort_by(|a, b| a.name.cmp(&b.name));
    open
}

/// `EMAIL_PATTERN` (`access-groups.tsx:456`): something, `@`, something, `.`, something, with
/// no spaces and one `@`.
#[must_use]
pub fn is_email(s: &str) -> bool {
    if s.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    // `[^\s@]+\.[^\s@]+$`: a dot with something on both sides, the regex backtracking to any dot.
    domain
        .char_indices()
        .any(|(i, c)| c == '.' && i > 0 && i + 1 < domain.len())
}

/// The Access ends hint.
pub const UNTIL_HINT: &str =
    "Day, month, year. For auditors and short reviews; access stops at the end of that day.";

/// The invite form's placeholder.
pub const EMAIL_PLACEHOLDER: &str = "name@organisation.org";

/// What the invite form sends, once it checks out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    /// The address, trimmed.
    pub email: String,
    /// The end of the last day, `YYYY-MM-DDT23:59:59Z`; `None` for no end.
    pub valid_until: Option<String>,
}

/// The invite form's refusals (`MembersSection`, `access-groups.tsx:578-597`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InviteErrors {
    /// The email field's.
    pub email: Option<&'static str>,
    /// The end date's.
    pub until: Option<String>,
}

/// Checks the invite form against the group's `members` with `today` as the earliest end.
///
/// # Errors
///
/// The field messages, as the web shows them once the form was submitted.
pub fn check_invite(
    email: &str,
    until: &str,
    members: &[GroupMember],
    group_id: &str,
    today: &IsoDate,
) -> Result<Invite, InviteErrors> {
    let address = email.trim();
    let email_error = if address.is_empty() {
        Some("Enter an email address.")
    } else if !is_email(address) {
        Some("That doesn\u{2019}t look like an email address. Check it and try again.")
    } else if members.iter().any(|m| {
        m.group_id == group_id
            && m.user_email
                .as_deref()
                .is_some_and(|e| e.to_lowercase() == address.to_lowercase())
    }) {
        Some("This person is already in the group.")
    } else {
        None
    };
    let (until_iso, until_error) = match parse_day_month_year(until) {
        None => (None, None),
        Some(Err(e)) => (None, Some(e)),
        Some(Ok(d)) if d.as_str() < today.as_str() => (
            None,
            Some("That date has passed. Choose today or a later day.".to_owned()),
        ),
        Some(Ok(d)) => (Some(d), None),
    };
    if email_error.is_some() || until_error.is_some() {
        return Err(InviteErrors {
            email: email_error,
            until: until_error,
        });
    }
    Ok(Invite {
        email: address.to_owned(),
        valid_until: until_iso.map(|d| format!("{}T23:59:59Z", d.as_str())),
    })
}

/// `{email} is invited to {group}.`
#[must_use]
pub fn invited(email: &str, group: &str) -> String {
    format!("{email} is invited to {group}.")
}

/// `Delete “{name}”?` and the dialog's text (`access-groups.tsx:266-276`).
#[must_use]
pub fn delete_title(name: &str) -> String {
    format!("Delete \u{201c}{name}\u{201d}?")
}

/// The delete dialog's text.
pub const DELETE_TEXT: &str = "Everyone in this group loses access straight away, and any chain link that granted this group stops showing your flows. This can\u{2019}t be undone.";

/// `Remove {name} from {group}?` (`access-groups.tsx:837-840`).
#[must_use]
pub fn remove_title(name: &str, group: &str) -> String {
    format!("Remove {name} from {group}?")
}

/// The remove dialog's text.
pub const REMOVE_TEXT: &str =
    "They lose this group\u{2019}s access straight away. You can invite them again later.";

/// `slugify` (`access-groups.tsx:868-873`): lowercase, every run of other characters a single
/// hyphen, none at either end.
#[must_use]
pub fn slugify(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::new();
    let mut gap = false;
    for c in lower.trim().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out
}

/// `EMPTY_PERMISSIONS` (`access-groups.tsx:875-885`): a new group sees the flow and amounts.
pub const NEW_PERMISSIONS: AccessGroupPermissions = AccessGroupPermissions {
    can_view_nodes: true,
    can_view_amounts: true,
    can_view_proofs: false,
    can_view_executor: false,
    can_view_full_proofs: false,
    can_edit_nodes: false,
    can_edit_transactions: false,
    can_manage_members: false,
    can_manage_groups: false,
};

/// A permission the form ticks (`FlagKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// The flow.
    Nodes,
    /// Amounts.
    Amounts,
    /// Who handled it.
    Executor,
    /// Edit flows.
    EditNodes,
    /// Edit transactions.
    EditTransactions,
    /// Manage members.
    ManageMembers,
    /// Manage groups.
    ManageGroups,
}

impl Flag {
    /// Whether `p` has it.
    #[must_use]
    pub const fn get(self, p: &AccessGroupPermissions) -> bool {
        match self {
            Self::Nodes => p.can_view_nodes,
            Self::Amounts => p.can_view_amounts,
            Self::Executor => p.can_view_executor,
            Self::EditNodes => p.can_edit_nodes,
            Self::EditTransactions => p.can_edit_transactions,
            Self::ManageMembers => p.can_manage_members,
            Self::ManageGroups => p.can_manage_groups,
        }
    }

    /// `p` with it set to `v`.
    #[must_use]
    pub const fn set(self, mut p: AccessGroupPermissions, v: bool) -> AccessGroupPermissions {
        match self {
            Self::Nodes => p.can_view_nodes = v,
            Self::Amounts => p.can_view_amounts = v,
            Self::Executor => p.can_view_executor = v,
            Self::EditNodes => p.can_edit_nodes = v,
            Self::EditTransactions => p.can_edit_transactions = v,
            Self::ManageMembers => p.can_manage_members = v,
            Self::ManageGroups => p.can_manage_groups = v,
        }
        p
    }
}

/// "What they see" (`SEE_FLAGS`, `access-groups.tsx:892-900`).
pub const SEE_FLAGS: [(Flag, &str, &str); 3] = [
    (
        Flag::Nodes,
        "The flow",
        "Nodes and how money moves between them.",
    ),
    (Flag::Amounts, "Amounts", "The KES value of each movement."),
    (
        Flag::Executor,
        "Who handled it",
        "The staff member or vendor behind each payment.",
    ),
];

/// "What they can do" (`DO_FLAGS`, `access-groups.tsx:902-911`).
pub const DO_FLAGS: [(Flag, &str, &str); 4] = [
    (Flag::EditNodes, "Edit flows", "Add and change nodes."),
    (
        Flag::EditTransactions,
        "Edit transactions",
        "Record transactions and attach proof.",
    ),
    (
        Flag::ManageMembers,
        "Manage members",
        "Invite and remove people.",
    ),
    (
        Flag::ManageGroups,
        "Manage groups",
        "Create groups and change access.",
    ),
];

/// The group form (`GroupFormDialog`, `access-groups.tsx:913-1258`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupForm {
    /// The group being edited; `None` for a new one.
    pub editing: Option<String>,
    /// Editing a default group: its short id is fixed.
    pub is_default: bool,
    /// The group's name, for the title.
    pub original_name: String,
    /// Name.
    pub name: String,
    /// Short id.
    pub slug: String,
    /// The short id was typed, so the name no longer sets it.
    pub slug_edited: bool,
    /// Description.
    pub description: String,
    /// Which flows.
    pub scope: NodeScope,
    /// What they see and do.
    pub permissions: AccessGroupPermissions,
}

/// The form's refusals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupErrors {
    /// Name.
    pub name: Option<&'static str>,
    /// Short id.
    pub slug: Option<&'static str>,
}

impl GroupErrors {
    /// Nothing to fix.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.name.is_none() && self.slug.is_none()
    }
}

/// The name's placeholder, the short id's hint and the description's placeholder.
pub const NAME_PLACEHOLDER: &str = "e.g. Harambee grant team";
/// The Short ID hint.
pub const SLUG_HINT: &str = "Used in links and the API.";
/// The Description placeholder.
pub const DESCRIPTION_PLACEHOLDER: &str = "Who is in this group and why";

impl GroupForm {
    /// A new group's form: assigned nodes, the flow and amounts.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            editing: None,
            is_default: false,
            original_name: String::new(),
            name: String::new(),
            slug: String::new(),
            slug_edited: false,
            description: String::new(),
            scope: NodeScope::Assigned,
            permissions: NEW_PERMISSIONS,
        }
    }

    /// The form for `g`.
    #[must_use]
    pub fn edit(g: &AccessGroup) -> Self {
        Self {
            editing: Some(g.id.clone()),
            is_default: g.is_default,
            original_name: g.name.clone(),
            name: g.name.clone(),
            slug: g.slug.clone(),
            slug_edited: true,
            description: g.description.clone().unwrap_or_default(),
            scope: g.node_scope,
            permissions: g.permissions,
        }
    }

    /// Types the name; a new group's short id follows it until the id is typed.
    pub fn set_name(&mut self, name: &str) {
        name.clone_into(&mut self.name);
        if !self.slug_edited {
            self.slug = slugify(name);
        }
    }

    /// Types the short id, lowercased.
    pub fn set_slug(&mut self, slug: &str) {
        self.slug = slug.to_lowercase();
        self.slug_edited = true;
    }

    /// The dialog's title.
    #[must_use]
    pub fn title(&self) -> String {
        if self.editing.is_some() {
            format!("Edit {}", self.original_name)
        } else {
            "Create an access group".to_owned()
        }
    }

    /// The dialog's description.
    #[must_use]
    pub const fn description_text(&self) -> &'static str {
        if self.is_default {
            "A default group. You can change its depth, but not delete it."
        } else {
            "Set what this group sees. You can change it at any time."
        }
    }

    /// The submit button: `Save group` or `Create group`, `Saving` or `Creating` while it runs.
    #[must_use]
    pub const fn submit_label(&self, pending: bool) -> &'static str {
        match (self.editing.is_some(), pending) {
            (true, false) => "Save group",
            (true, true) => "Saving",
            (false, false) => "Create group",
            (false, true) => "Creating",
        }
    }

    /// The form's refusals against the organisation's other groups.
    #[must_use]
    pub fn validate(&self, groups: &[AccessGroup]) -> GroupErrors {
        let others: Vec<&AccessGroup> = groups
            .iter()
            .filter(|g| Some(&g.id) != self.editing.as_ref())
            .collect();
        let name = self.name.trim();
        let name_error = if name.chars().count() < 2 {
            Some("Give the group a name.")
        } else if others
            .iter()
            .any(|g| g.name.to_lowercase() == name.to_lowercase())
        {
            Some("Another group already has this name.")
        } else {
            None
        };
        let slug_error = if self.slug.is_empty() {
            Some("Enter a short ID.")
        } else if !is_slug(&self.slug) {
            Some("Use lowercase letters, numbers and single hyphens.")
        } else if others.iter().any(|g| g.slug == self.slug) {
            Some("Another group already uses this ID.")
        } else {
            None
        };
        GroupErrors {
            name: name_error,
            slug: slug_error,
        }
    }

    fn description_value(&self) -> Option<String> {
        let d = self.description.trim();
        (!d.is_empty()).then(|| d.to_owned())
    }

    /// A new group as the web creates it: icon `users`, report blue, custom, private, priority
    /// 50 (`access-groups.tsx:977-991`).
    #[must_use]
    pub fn new_group(&self) -> NewAccessGroup {
        NewAccessGroup {
            name: self.name.trim().to_owned(),
            slug: self.slug.clone(),
            description: self.description_value(),
            icon: "users".to_owned(),
            color: "#1C5FA8".to_owned(),
            permissions: self.permissions,
            node_scope: self.scope,
            is_default: false,
            is_public: false,
            priority: 50,
        }
    }

    /// An edit as the web sends it: name, short id, description, scope and permissions
    /// (`access-groups.tsx:961-975`).
    #[must_use]
    pub fn patch(&self) -> AccessGroupPatch {
        AccessGroupPatch {
            name: Some(self.name.trim().to_owned()),
            slug: Some(self.slug.clone()),
            description: self
                .description_value()
                .map_or(Nullish::Null, Nullish::Value),
            node_scope: Some(self.scope),
            permissions: Some(self.permissions),
            ..AccessGroupPatch::default()
        }
    }
}

impl Default for GroupForm {
    fn default() -> Self {
        Self::new()
    }
}
