//! Authorisation and validation rules of the TS module (`spacetimedb/src/index.ts:507-691`), as
//! pure functions over rows so host tests cover them. The reducers in [`crate::reducers`] only
//! look rows up, call these and write what they return.
//!
//! Every error text is the TS `SenderError` text word for word; a reducer returns it as its
//! `Err(String)`, which is what a TS reducer's `SenderError` becomes for the caller
//! (<https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.reducer.html>).

use ledger_core::types::{Actor, ActorId};
use spacetimedb::Identity;

use crate::codec::row_key;
use crate::tables::Member;

/// `memberKey` (`index.ts:359`): `{org_id}:{identity as lowercase hex}`.
#[must_use]
pub fn member_key(org_id: &str, identity: Identity) -> String {
    row_key(org_id, identity.to_hex().as_ref())
}

/// `String.prototype.trim`: strips ECMAScript white space and line terminators
/// (<https://tc39.es/ecma262/#sec-string.prototype.trim>). Unlike `str::trim` it strips U+FEFF and
/// keeps U+0085.
#[must_use]
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}')
}

/// `/^[A-Za-z0-9_-]{1,128}$/` (`index.ts:533`).
fn is_org_id(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `/^[A-Z]{3}$/` (`index.ts:560`).
fn is_currency_code(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase())
}

/// The `can_close` check of `addMember` and `removeMember` (`index.ts:610`, `index.ts:645`).
fn require_close(actor: &Actor) -> Result<(), String> {
    if actor.can_close {
        Ok(())
    } else {
        Err("Only members who can close periods can manage members.".to_owned())
    }
}

/// The checks `createOrgLedger` runs before it seeds anything (`index.ts:557-562`), in TS order.
///
/// # Errors
///
/// The TS message of the first check that fails.
pub fn check_new_org(
    sender_is_operator: bool,
    org_id: &str,
    base_currency: &str,
    fiscal_year: u32,
    org_exists: bool,
) -> Result<(), String> {
    if !sender_is_operator {
        return Err("Only an operator can open books for a new organisation.".to_owned());
    }
    if !is_org_id(org_id) {
        return Err("Organisation id must be 1-128 letters, digits, _ or -.".to_owned());
    }
    if !is_currency_code(base_currency) {
        return Err("Base currency must be an ISO 4217 code, like KES.".to_owned());
    }
    if !(2000..=2100).contains(&fiscal_year) {
        return Err("Fiscal year must be between 2000 and 2100.".to_owned());
    }
    if org_exists {
        return Err(format!("Organisation {org_id} already has a ledger."));
    }
    Ok(())
}

/// `addOperator` (`index.ts:546-552`): only an operator adds operators.
///
/// # Errors
///
/// When the sender is not an operator.
pub fn check_add_operator(sender_is_operator: bool) -> Result<(), String> {
    if sender_is_operator {
        Ok(())
    } else {
        Err("Only an operator can add operators.".to_owned())
    }
}

/// `import_books` has no TS counterpart: it replaces an organisation's books, so like opening
/// books (`index.ts:557-558`) only an operator may do it.
///
/// # Errors
///
/// When the sender is not an operator.
pub fn check_import_books(sender_is_operator: bool) -> Result<(), String> {
    if sender_is_operator {
        Ok(())
    } else {
        Err("Only an operator can import books.".to_owned())
    }
}

/// `remove_operator` has no TS counterpart (`index.ts` only adds operators). It follows
/// `addOperator` (`index.ts:549`) and keeps at least one operator, since only `init` makes one
/// otherwise.
///
/// # Errors
///
/// When the sender is not an operator, the target is not one, or the target is the last one.
pub fn check_remove_operator(
    sender_is_operator: bool,
    target_is_operator: bool,
    operator_count: u64,
) -> Result<(), String> {
    if !sender_is_operator {
        return Err("Only an operator can remove operators.".to_owned());
    }
    if !target_is_operator {
        return Err("No such operator.".to_owned());
    }
    if operator_count <= 1 {
        return Err("The last operator cannot be removed.".to_owned());
    }
    Ok(())
}

/// `actorOf` (`index.ts:507-511`): the caller's member row as the engine's actor.
///
/// # Errors
///
/// When the caller is not a member of the organisation.
pub fn actor_of(org_id: &str, member: Option<&Member>) -> Result<Actor, String> {
    let m = member.ok_or_else(|| format!("You are not a member of organisation {org_id}."))?;
    Ok(Actor {
        id: ActorId::from(m.user_id.as_str()),
        name: m.name.clone(),
        can_post: m.can_post,
        can_approve: m.can_approve,
        can_close: m.can_close,
    })
}

/// One audit event to append (`appendAudit`, `index.ts:521-527`); the reducer adds id and time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditNote {
    /// Who acted: their user id.
    pub by: ActorId,
    /// What, for example `member.add`.
    pub action: &'static str,
    /// The record acted on.
    pub subject: String,
    /// Free-text detail.
    pub detail: String,
}

/// What `create_org_ledger` writes once the checks pass: the owner's member row and the
/// `ledger.create` event (`index.ts:581-593`).
#[derive(Debug, Clone, PartialEq)]
pub struct NewOrg {
    /// The caller as a member with every permission.
    pub owner: Member,
    /// The `ledger.create` event.
    pub audit: AuditNote,
}

/// The owner and audit event of a new organisation (`index.ts:581-593`). The owner's user id is
/// their JWT subject when they have one, else their identity; `display_name` is only a label.
#[must_use]
pub fn new_org(
    org_id: &str,
    sender: Identity,
    jwt_subject: Option<&str>,
    base_currency: &str,
    fiscal_year: u32,
    display_name: &str,
) -> NewOrg {
    let user_id = jwt_subject.map_or_else(|| sender.to_hex().to_string(), str::to_owned);
    let name = match js_trim(display_name) {
        "" => "Owner",
        trimmed => trimmed,
    };
    NewOrg {
        owner: Member {
            key: member_key(org_id, sender),
            identity: sender,
            org_id: org_id.to_owned(),
            user_id: user_id.clone(),
            name: name.to_owned(),
            can_post: true,
            can_approve: true,
            can_close: true,
        },
        audit: AuditNote {
            by: ActorId::from(user_id),
            action: "ledger.create",
            subject: org_id.to_owned(),
            detail: format!("Opened books in {base_currency} for {fiscal_year}."),
        },
    }
}

/// The arguments of `add_member` (`index.ts:599-607`), less the organisation.
#[derive(Debug, Clone, Copy)]
pub struct MemberArgs<'a> {
    /// The member's identity.
    pub identity: Identity,
    /// Their user id; ignored for an existing member, whose user id is fixed.
    pub user_id: &'a str,
    /// Display name; the user id when blank.
    pub name: &'a str,
    /// May post.
    pub can_post: bool,
    /// May approve.
    pub can_approve: bool,
    /// May close periods and manage members.
    pub can_close: bool,
}

/// What `add_member` writes: the member row, whether it replaces one, and the audit event.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberChange {
    /// The row to insert or update.
    pub row: Member,
    /// True when a member with that identity exists (`member.update`).
    pub replaces: bool,
    /// The `member.add` or `member.update` event.
    pub audit: AuditNote,
}

/// `addMember` (`index.ts:597-638`) over the organisation's member rows.
///
/// # Errors
///
/// The TS message of the first rule that fails.
pub fn plan_add_member(
    org_id: &str,
    sender: Identity,
    org_members: &[Member],
    args: &MemberArgs<'_>,
) -> Result<MemberChange, String> {
    let actor = actor_of(org_id, org_members.iter().find(|m| m.identity == sender))?;
    require_close(&actor)?;
    let given = js_trim(args.user_id);
    if given.is_empty() {
        return Err("A member needs a user id.".to_owned());
    }
    let existing = org_members.iter().find(|m| m.identity == args.identity);
    // A user id is the person behind approvals and the audit trail, so it is fixed once given
    // and can never be borrowed from another member (index.ts:614-616).
    let user_id = existing.map_or(given, |m| m.user_id.as_str());
    if org_members
        .iter()
        .any(|m| m.user_id == user_id && m.identity != args.identity)
    {
        return Err("That user id belongs to another member.".to_owned());
    }
    if args.identity == sender && !args.can_close {
        return Err("You cannot remove your own close permission.".to_owned());
    }
    let name = match js_trim(args.name) {
        "" => user_id,
        trimmed => trimmed,
    };
    let perms: Vec<&str> = [
        (args.can_post, "post"),
        (args.can_approve, "approve"),
        (args.can_close, "close"),
    ]
    .into_iter()
    .filter_map(|(granted, perm)| granted.then_some(perm))
    .collect();
    let perms = if perms.is_empty() {
        "read only".to_owned()
    } else {
        perms.join(", ")
    };
    Ok(MemberChange {
        row: Member {
            key: member_key(org_id, args.identity),
            identity: args.identity,
            org_id: org_id.to_owned(),
            user_id: user_id.to_owned(),
            name: name.to_owned(),
            can_post: args.can_post,
            can_approve: args.can_approve,
            can_close: args.can_close,
        },
        replaces: existing.is_some(),
        audit: AuditNote {
            by: actor.id,
            action: if existing.is_some() {
                "member.update"
            } else {
                "member.add"
            },
            subject: user_id.to_owned(),
            detail: format!("{name}: {perms}"),
        },
    })
}

/// What `remove_member` deletes and the event it appends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRemoval {
    /// The member row's key.
    pub key: String,
    /// The `member.remove` event.
    pub audit: AuditNote,
}

/// `removeMember` (`index.ts:640-653`) over the organisation's member rows.
///
/// # Errors
///
/// The TS message of the first rule that fails.
pub fn plan_remove_member(
    org_id: &str,
    sender: Identity,
    org_members: &[Member],
    identity: Identity,
) -> Result<MemberRemoval, String> {
    let actor = actor_of(org_id, org_members.iter().find(|m| m.identity == sender))?;
    require_close(&actor)?;
    if identity == sender {
        return Err("You cannot remove yourself.".to_owned());
    }
    let existing = org_members
        .iter()
        .find(|m| m.identity == identity)
        .ok_or_else(|| "No such member.".to_owned())?;
    Ok(MemberRemoval {
        key: existing.key.clone(),
        audit: AuditNote {
            by: actor.id,
            action: "member.remove",
            subject: existing.user_id.clone(),
            detail: existing.name.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    const OWNER: Identity = Identity::from_byte_array([1; 32]);
    const CLERK: Identity = Identity::from_byte_array([2; 32]);
    const STRANGER: Identity = Identity::from_byte_array([3; 32]);

    fn member(identity: Identity, user_id: &str, can_close: bool) -> Member {
        Member {
            key: member_key("org-1", identity),
            identity,
            org_id: "org-1".to_owned(),
            user_id: user_id.to_owned(),
            name: user_id.to_uppercase(),
            can_post: true,
            can_approve: false,
            can_close,
        }
    }

    fn org() -> Vec<Member> {
        vec![member(OWNER, "owner", true), member(CLERK, "clerk", false)]
    }

    fn args(identity: Identity, user_id: &str) -> MemberArgs<'_> {
        MemberArgs {
            identity,
            user_id,
            name: "New Person",
            can_post: true,
            can_approve: true,
            can_close: false,
        }
    }

    // member_key and js_trim

    #[test]
    fn member_key_is_org_colon_lowercase_identity_hex() {
        assert_eq!(
            member_key("org-1", OWNER),
            format!("org-1:{}", "01".repeat(32))
        );
    }

    #[test]
    fn js_trim_strips_the_bom_and_line_terminators() {
        assert_eq!(js_trim("\u{feff}\u{2028} Ama \n\t"), "Ama");
    }

    #[test]
    fn js_trim_keeps_next_line_which_is_not_js_white_space() {
        assert_eq!(js_trim("\u{85}Ama"), "\u{85}Ama");
    }

    // create_org_ledger (index.ts:557-562)

    #[test]
    fn new_org_needs_an_operator() {
        assert_eq!(
            check_new_org(false, "org-1", "KES", 2026, false).unwrap_err(),
            "Only an operator can open books for a new organisation."
        );
    }

    #[test]
    fn new_org_checks_the_operator_before_the_arguments() {
        assert_eq!(
            check_new_org(false, "", "kes", 1999, true).unwrap_err(),
            "Only an operator can open books for a new organisation."
        );
    }

    #[test]
    fn new_org_accepts_letters_digits_underscore_and_dash() {
        assert_eq!(check_new_org(true, "Org_1-a", "KES", 2026, false), Ok(()));
    }

    #[test]
    fn new_org_refuses_an_empty_id() {
        assert_eq!(
            check_new_org(true, "", "KES", 2026, false).unwrap_err(),
            "Organisation id must be 1-128 letters, digits, _ or -."
        );
    }

    #[test]
    fn new_org_refuses_an_id_of_129_characters() {
        assert_eq!(
            check_new_org(true, &"a".repeat(129), "KES", 2026, false).unwrap_err(),
            "Organisation id must be 1-128 letters, digits, _ or -."
        );
    }

    #[test]
    fn new_org_accepts_an_id_of_128_characters() {
        assert_eq!(
            check_new_org(true, &"a".repeat(128), "KES", 2026, false),
            Ok(())
        );
    }

    #[test]
    fn new_org_refuses_an_id_with_a_colon() {
        assert_eq!(
            check_new_org(true, "org:1", "KES", 2026, false).unwrap_err(),
            "Organisation id must be 1-128 letters, digits, _ or -."
        );
    }

    #[test]
    fn new_org_refuses_a_lowercase_currency() {
        assert_eq!(
            check_new_org(true, "org-1", "kes", 2026, false).unwrap_err(),
            "Base currency must be an ISO 4217 code, like KES."
        );
    }

    #[test]
    fn new_org_refuses_a_four_letter_currency() {
        assert_eq!(
            check_new_org(true, "org-1", "KESH", 2026, false).unwrap_err(),
            "Base currency must be an ISO 4217 code, like KES."
        );
    }

    #[test]
    fn new_org_refuses_1999() {
        assert_eq!(
            check_new_org(true, "org-1", "KES", 1999, false).unwrap_err(),
            "Fiscal year must be between 2000 and 2100."
        );
    }

    #[test]
    fn new_org_refuses_2101() {
        assert_eq!(
            check_new_org(true, "org-1", "KES", 2101, false).unwrap_err(),
            "Fiscal year must be between 2000 and 2100."
        );
    }

    #[test]
    fn new_org_accepts_2000_and_2100() {
        assert_eq!(
            (
                check_new_org(true, "org-1", "KES", 2000, false),
                check_new_org(true, "org-1", "KES", 2100, false)
            ),
            (Ok(()), Ok(()))
        );
    }

    #[test]
    fn new_org_refuses_an_organisation_that_has_books() {
        assert_eq!(
            check_new_org(true, "org-1", "KES", 2026, true).unwrap_err(),
            "Organisation org-1 already has a ledger."
        );
    }

    #[test]
    fn new_org_owner_takes_the_jwt_subject_and_every_permission() {
        let made = new_org("org-1", OWNER, Some("user_42"), "KES", 2026, "  Ama  ");
        assert_eq!(
            made.owner,
            Member {
                key: member_key("org-1", OWNER),
                identity: OWNER,
                org_id: "org-1".to_owned(),
                user_id: "user_42".to_owned(),
                name: "Ama".to_owned(),
                can_post: true,
                can_approve: true,
                can_close: true,
            }
        );
    }

    #[test]
    fn new_org_owner_without_a_jwt_is_their_identity_and_owner() {
        let made = new_org("org-1", OWNER, None, "KES", 2026, " ");
        assert_eq!(
            (made.owner.user_id, made.owner.name),
            ("01".repeat(32), "Owner".to_owned())
        );
    }

    #[test]
    fn new_org_appends_ledger_create() {
        let made = new_org("org-1", OWNER, Some("user_42"), "UGX", 2027, "Ama");
        assert_eq!(
            made.audit,
            AuditNote {
                by: ActorId::from("user_42"),
                action: "ledger.create",
                subject: "org-1".to_owned(),
                detail: "Opened books in UGX for 2027.".to_owned(),
            }
        );
    }

    // operators (index.ts:546-552)

    #[test]
    fn add_operator_needs_an_operator() {
        assert_eq!(
            check_add_operator(false).unwrap_err(),
            "Only an operator can add operators."
        );
    }

    #[test]
    fn add_operator_lets_an_operator_through() {
        assert_eq!(check_add_operator(true), Ok(()));
    }

    #[test]
    fn remove_operator_needs_an_operator() {
        assert_eq!(
            check_remove_operator(false, true, 2).unwrap_err(),
            "Only an operator can remove operators."
        );
    }

    #[test]
    fn remove_operator_refuses_an_identity_that_is_not_one() {
        assert_eq!(
            check_remove_operator(true, false, 2).unwrap_err(),
            "No such operator."
        );
    }

    #[test]
    fn remove_operator_keeps_the_last_operator() {
        assert_eq!(
            check_remove_operator(true, true, 1).unwrap_err(),
            "The last operator cannot be removed."
        );
    }

    #[test]
    fn remove_operator_removes_one_of_two() {
        assert_eq!(check_remove_operator(true, true, 2), Ok(()));
    }

    // actorOf (index.ts:507-511)

    #[test]
    fn actor_of_refuses_a_non_member() {
        assert_eq!(
            actor_of("org-1", None).unwrap_err(),
            "You are not a member of organisation org-1."
        );
    }

    #[test]
    fn actor_of_maps_the_member_row() {
        assert_eq!(
            actor_of("org-1", Some(&member(CLERK, "clerk", false))).unwrap(),
            Actor {
                id: ActorId::from("clerk"),
                name: "CLERK".to_owned(),
                can_post: true,
                can_approve: false,
                can_close: false,
            }
        );
    }

    // addMember (index.ts:597-638)

    #[test]
    fn add_member_refuses_a_non_member() {
        assert_eq!(
            plan_add_member("org-1", STRANGER, &org(), &args(STRANGER, "me")).unwrap_err(),
            "You are not a member of organisation org-1."
        );
    }

    #[test]
    fn add_member_needs_close_permission() {
        assert_eq!(
            plan_add_member("org-1", CLERK, &org(), &args(STRANGER, "new")).unwrap_err(),
            "Only members who can close periods can manage members."
        );
    }

    #[test]
    fn add_member_needs_a_user_id() {
        assert_eq!(
            plan_add_member("org-1", OWNER, &org(), &args(STRANGER, " \n")).unwrap_err(),
            "A member needs a user id."
        );
    }

    #[test]
    fn add_member_refuses_a_user_id_of_another_member() {
        assert_eq!(
            plan_add_member("org-1", OWNER, &org(), &args(STRANGER, " clerk ")).unwrap_err(),
            "That user id belongs to another member."
        );
    }

    #[test]
    fn add_member_refuses_dropping_your_own_close_permission() {
        assert_eq!(
            plan_add_member("org-1", OWNER, &org(), &args(OWNER, "owner")).unwrap_err(),
            "You cannot remove your own close permission."
        );
    }

    #[test]
    fn add_member_inserts_a_new_member_with_trimmed_ids() {
        let mut new = args(STRANGER, "  new_1 ");
        new.name = " Esi ";
        let change = plan_add_member("org-1", OWNER, &org(), &new).unwrap();
        assert_eq!(
            (change.row, change.replaces),
            (
                Member {
                    key: member_key("org-1", STRANGER),
                    identity: STRANGER,
                    org_id: "org-1".to_owned(),
                    user_id: "new_1".to_owned(),
                    name: "Esi".to_owned(),
                    can_post: true,
                    can_approve: true,
                    can_close: false,
                },
                false
            )
        );
    }

    #[test]
    fn add_member_audits_member_add_with_the_permissions() {
        let change = plan_add_member("org-1", OWNER, &org(), &args(STRANGER, "new_1")).unwrap();
        assert_eq!(
            change.audit,
            AuditNote {
                by: ActorId::from("owner"),
                action: "member.add",
                subject: "new_1".to_owned(),
                detail: "New Person: post, approve".to_owned(),
            }
        );
    }

    #[test]
    fn add_member_names_a_blank_name_after_the_user_id() {
        let mut new = args(STRANGER, "new_1");
        new.name = "  ";
        let change = plan_add_member("org-1", OWNER, &org(), &new).unwrap();
        assert_eq!(change.row.name, "new_1");
    }

    #[test]
    fn add_member_writes_read_only_when_no_permission_is_given() {
        let mut new = args(STRANGER, "viewer");
        (new.can_post, new.can_approve, new.can_close) = (false, false, false);
        let change = plan_add_member("org-1", OWNER, &org(), &new).unwrap();
        assert_eq!(change.audit.detail, "New Person: read only");
    }

    #[test]
    fn add_member_keeps_an_existing_members_user_id() {
        let change = plan_add_member("org-1", OWNER, &org(), &args(CLERK, "renamed")).unwrap();
        assert_eq!(
            (
                change.row.user_id.as_str(),
                change.replaces,
                change.audit.action
            ),
            ("clerk", true, "member.update")
        );
    }

    #[test]
    fn add_member_lets_an_owner_keep_their_close_permission() {
        let mut me = args(OWNER, "ignored");
        me.can_close = true;
        let change = plan_add_member("org-1", OWNER, &org(), &me).unwrap();
        assert_eq!(change.audit.detail, "New Person: post, approve, close");
    }

    // removeMember (index.ts:640-653)

    #[test]
    fn remove_member_refuses_a_non_member() {
        assert_eq!(
            plan_remove_member("org-1", STRANGER, &org(), CLERK).unwrap_err(),
            "You are not a member of organisation org-1."
        );
    }

    #[test]
    fn remove_member_needs_close_permission() {
        assert_eq!(
            plan_remove_member("org-1", CLERK, &org(), OWNER).unwrap_err(),
            "Only members who can close periods can manage members."
        );
    }

    #[test]
    fn remove_member_refuses_removing_yourself() {
        assert_eq!(
            plan_remove_member("org-1", OWNER, &org(), OWNER).unwrap_err(),
            "You cannot remove yourself."
        );
    }

    #[test]
    fn remove_member_refuses_an_identity_that_is_not_a_member() {
        assert_eq!(
            plan_remove_member("org-1", OWNER, &org(), STRANGER).unwrap_err(),
            "No such member."
        );
    }

    #[test]
    fn remove_member_deletes_the_row_and_audits_member_remove() {
        assert_eq!(
            plan_remove_member("org-1", OWNER, &org(), CLERK).unwrap(),
            MemberRemoval {
                key: member_key("org-1", CLERK),
                audit: AuditNote {
                    by: ActorId::from("owner"),
                    action: "member.remove",
                    subject: "clerk".to_owned(),
                    detail: "CLERK".to_owned(),
                },
            }
        );
    }

    // import_books (no TS counterpart)

    #[test]
    fn import_books_needs_an_operator() {
        assert_eq!(
            check_import_books(false).unwrap_err(),
            "Only an operator can import books."
        );
    }

    #[test]
    fn import_books_lets_an_operator_through() {
        assert_eq!(check_import_books(true), Ok(()));
    }
}
