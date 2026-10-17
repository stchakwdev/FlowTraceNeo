//! Services (pre-decision P17): identities that are not people, such as the model Worker (JV1),
//! registered once per database by an operator, each with the scopes of the reducers it may call.
//! A service is not a member of any organisation; the reducers gated on a scope take it instead
//! of a member row.

/// `record_flags`: add and refresh open flags.
pub const SCOPE_FLAGS: &str = "flags";
/// `record_rubric`: store the model's rubric scores.
pub const SCOPE_RUBRIC: &str = "rubric";
/// `recompute_trust`: compute the score and write a history snapshot.
pub const SCOPE_TRUST: &str = "trust";

/// Every scope a service can hold.
pub const SCOPES: [&str; 3] = [SCOPE_FLAGS, SCOPE_RUBRIC, SCOPE_TRUST];

/// The label and scopes `register_service` stores, or why it may not.
///
/// # Errors
///
/// The caller is not an operator, the label is blank, there are no scopes or one is unknown.
pub fn plan_register_service(
    sender_is_operator: bool,
    label: &str,
    scopes: &[String],
) -> Result<(String, Vec<String>), String> {
    if !sender_is_operator {
        return Err("Only an operator can register a service.".to_owned());
    }
    let label = crate::rules::js_trim(label);
    if label.is_empty() {
        return Err("A service needs a label.".to_owned());
    }
    if scopes.is_empty() {
        return Err(format!(
            "A service needs at least one scope: {}.",
            SCOPES.join(", ")
        ));
    }
    let mut out: Vec<String> = Vec::new();
    for s in scopes {
        if !SCOPES.contains(&s.as_str()) {
            return Err(format!(
                "Unknown service scope {s}; the scopes are {}.",
                SCOPES.join(", ")
            ));
        }
        if !out.contains(s) {
            out.push(s.clone());
        }
    }
    out.sort();
    Ok((label.to_owned(), out))
}

/// Whether `remove_service` may go ahead.
///
/// # Errors
///
/// The caller is not an operator, or there is no such service.
pub fn check_remove_service(sender_is_operator: bool, exists: bool) -> Result<(), String> {
    if !sender_is_operator {
        return Err("Only an operator can remove a service.".to_owned());
    }
    if !exists {
        return Err("No such service.".to_owned());
    }
    Ok(())
}

/// The caller as a service holding `scope`, by its label and scopes (`None` when the caller is
/// not a service); `what` names the call for the refusal.
///
/// # Errors
///
/// The caller is not a registered service, or lacks the scope.
pub fn require_scope<'a>(
    service: Option<(&'a str, &[String])>,
    scope: &str,
    what: &str,
) -> Result<&'a str, String> {
    match service {
        Some((label, scopes)) if scopes.iter().any(|s| s == scope) => Ok(label),
        Some(_) => Err(format!(
            "This service does not hold the {scope} scope, so it cannot {what}."
        )),
        None => Err(format!(
            "Only a registered service with the {scope} scope can {what}."
        )),
    }
}

/// How a service signs what it writes (`raised_by`, `computed_by`).
#[must_use]
pub fn signature(label: &str) -> String {
    format!("service:{label}")
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    fn scopes(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn only_an_operator_registers_a_service() {
        assert_eq!(
            plan_register_service(false, "jev", &scopes(&["flags"])).unwrap_err(),
            "Only an operator can register a service."
        );
    }

    #[test]
    fn a_registration_trims_the_label_and_sorts_and_dedupes_the_scopes() {
        assert_eq!(
            plan_register_service(true, "  jev worker ", &scopes(&["trust", "flags", "trust"]))
                .unwrap(),
            ("jev worker".to_owned(), scopes(&["flags", "trust"]))
        );
    }

    #[test]
    fn a_registration_refuses_a_blank_label_no_scopes_and_unknown_scopes() {
        assert_eq!(
            plan_register_service(true, " ", &scopes(&["flags"])).unwrap_err(),
            "A service needs a label."
        );
        assert!(
            plan_register_service(true, "jev", &[])
                .unwrap_err()
                .starts_with("A service needs at least one scope")
        );
        assert!(
            plan_register_service(true, "jev", &scopes(&["admin"]))
                .unwrap_err()
                .starts_with("Unknown service scope admin")
        );
    }

    #[test]
    fn removal_needs_an_operator_and_an_existing_service() {
        assert!(check_remove_service(false, true).is_err());
        assert_eq!(
            check_remove_service(true, false).unwrap_err(),
            "No such service."
        );
        assert!(check_remove_service(true, true).is_ok());
    }

    #[test]
    fn a_scope_gate_lets_only_a_service_with_the_scope_through() {
        let held = scopes(&["flags"]);
        assert_eq!(
            require_scope(Some(("jev", &held)), SCOPE_FLAGS, "record flags").unwrap(),
            "jev"
        );
        assert_eq!(
            require_scope(Some(("jev", &held)), SCOPE_TRUST, "recompute").unwrap_err(),
            "This service does not hold the trust scope, so it cannot recompute."
        );
        assert_eq!(
            require_scope(None, SCOPE_FLAGS, "record flags").unwrap_err(),
            "Only a registered service with the flags scope can record flags."
        );
    }
}
