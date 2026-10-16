//! Who may connect, and who manages that: the rules behind `client_connected`, the trusted
//! issuers, the device-identity switch and members added by token subject. Pure functions over
//! rows, so host tests cover them; the reducers in [`crate::reducers`] look the rows up, call
//! these and write what they return. None of this has a TS counterpart (`spacetimedb/src/index.ts`
//! accepts every connection).
//!
//! What the SpacetimeDB 2.12 host does before a reducer sees a caller:
//!
//! - Every connection carries an OpenID Connect token, over a WebSocket or on an HTTP reducer
//!   call. A client without one gets a token the host signs itself, with issuer `localhost` and
//!   audience `spacetimedb` (<https://spacetimedb.com/docs/core-concepts/authentication>: "If a
//!   client connects without supplying a token, SpacetimeDB creates a new identity and returns a
//!   server-issued token"; the issuer and audience are `LOCALHOST` and `"spacetimedb"` in
//!   `crates/client-api/src/auth.rs` at v2.12.0, and both the local server and maincloud issued
//!   exactly that on 2026-10-16). Those are the device identities: the desktop's token file and
//!   the web's dummy mode.
//! - Any other issuer is checked against the keys its `{iss}/.well-known/openid-configuration`
//!   names, for any issuer, with no per-database setting and no audience check
//!   (`crates/core/src/auth/token_validation.rs` at v2.12.0: "We should require a specific
//!   audience at some point"; `validate_aud = false`). So the module checks issuer and audience
//!   itself in `client_connected`, as
//!   <https://spacetimedb.com/docs/core-concepts/authentication/usage> asks ("it's imperative
//!   that you check the `aud` claim").
//! - The identity is derived from `iss` and `sub` alone ([`Identity::from_claims`],
//!   <https://docs.rs/spacetimedb/2.12.0/spacetimedb/struct.JwtClaims.html#method.identity>), so
//!   one person at one issuer is one identity on every client and every database.
//! - An `Err` from `client_connected` refuses the connection; an HTTP reducer call gets 403 with
//!   the message (`crates/client-api/src/routes/database.rs:443-447` at v2.12.0, which also shows
//!   that HTTP calls run `client_connected`).

use spacetimedb::{Identity, JwtClaims};

use crate::rules::{self, MemberArgs, MemberChange, js_trim};
use crate::tables::{AuthSetting, Member, TrustedIssuer};

/// The issuer of the tokens the host signs itself: device identities (`LOCALHOST` in
/// `crates/client-api/src/auth.rs` at v2.12.0). No other issuer can use it: the host refuses a
/// `localhost` token it did not sign (`FullTokenValidator`, `token_validation.rs`).
pub const DEVICE_ISSUER: &str = "localhost";

/// The longest issuer or subject the host accepts (`crates/auth/src/identity.rs:106-111` at
/// v2.12.0: "The issuer and subject must be less than 128 bytes").
pub const MAX_CLAIM_BYTES: usize = 128;

/// The claims of a connection's token that decide whether it may connect.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenClaims {
    /// `iss`.
    pub issuer: String,
    /// `sub`.
    pub subject: String,
    /// `aud`, one string or a list in the token; empty when absent.
    pub audience: Vec<String>,
    /// `client_id`: WorkOS AuthKit access tokens carry no `aud` by default but name the
    /// application they were issued to here
    /// (<https://workos.com/blog/verify-workos-access-tokens-in-your-own-api>).
    pub client_id: Option<String>,
}

impl TokenClaims {
    /// The claims of a token the host has already validated.
    #[must_use]
    pub fn of(jwt: &JwtClaims) -> Self {
        Self {
            issuer: jwt.issuer().to_owned(),
            subject: jwt.subject().to_owned(),
            audience: jwt.audience().to_vec(),
            client_id: client_id_of(jwt.raw_payload()),
        }
    }

    /// Whether the host signed this token itself: a device identity.
    #[must_use]
    pub fn is_device(&self) -> bool {
        self.issuer == DEVICE_ISSUER
    }

    /// Who the token is meant for: its `aud`, or its `client_id` when it has no `aud`.
    fn audiences(&self) -> Vec<&str> {
        if self.audience.is_empty() {
            self.client_id.iter().map(String::as_str).collect()
        } else {
            self.audience.iter().map(String::as_str).collect()
        }
    }
}

/// The `client_id` claim of a JWT payload, when it is a string.
#[must_use]
pub fn client_id_of(raw_payload: &str) -> Option<String> {
    let payload: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    payload.get("client_id")?.as_str().map(str::to_owned)
}

/// Why a connection was let in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The caller is an operator, whatever their token: an operator can never lock themselves
    /// out by turning device identities off or removing an issuer.
    Operator,
    /// A device identity, while the database allows them.
    Device,
    /// A token whose issuer and audience match a [`TrustedIssuer`] row.
    Trusted,
    /// A registered service: an identity an operator registered with `register_service`.
    Service,
}

/// `client_connected`: whether a connection may proceed.
///
/// In order: an operator always connects; so does a registered service (MOD2,
/// `transparency/services.rs`), which an operator vouched for by its identity, so a service keeps
/// working whatever its token's issuer and whether device identities are on; a connection
/// without a token or with a host-signed
/// token connects while device identities are allowed ([`device_identities_allowed`]), and
/// never through a [`TrustedIssuer`] row; any other token connects when a [`TrustedIssuer`] row
/// has its issuer and one of its audiences ([`TokenClaims::audience`], else
/// [`TokenClaims::client_id`]).
///
/// # Errors
///
/// The refusal the caller sees.
pub fn check_connection(
    is_operator: bool,
    is_service: bool,
    claims: Option<&TokenClaims>,
    device_identities: bool,
    trusted: &[TrustedIssuer],
) -> Result<Admission, String> {
    if is_operator {
        return Ok(Admission::Operator);
    }
    if is_service {
        return Ok(Admission::Service);
    }
    let Some(claims) = claims.filter(|c| !c.is_device()) else {
        return if device_identities {
            Ok(Admission::Device)
        } else {
            Err(
                "This database does not accept device identities; sign in with an account it trusts."
                    .to_owned(),
            )
        };
    };
    let rows: Vec<&TrustedIssuer> = trusted
        .iter()
        .filter(|row| row.issuer == claims.issuer)
        .collect();
    if rows.is_empty() {
        return Err(format!(
            "This database does not trust tokens from {}.",
            claims.issuer
        ));
    }
    let audiences = claims.audiences();
    if rows
        .iter()
        .any(|row| audiences.contains(&row.audience.as_str()))
    {
        return Ok(Admission::Trusted);
    }
    let shown = if audiences.is_empty() {
        "(none)".to_owned()
    } else {
        audiences.join(", ")
    };
    Err(format!(
        "This database does not trust tokens from {} for audience {shown}.",
        claims.issuer
    ))
}

/// Whether device identities may connect: the [`AuthSetting`] row, and yes while there is none,
/// so a database published before this setting keeps its desktop and dummy-mode clients.
#[must_use]
pub fn device_identities_allowed(setting: Option<&AuthSetting>) -> bool {
    setting.is_none_or(|s| s.allow_device_identities)
}

/// What `add_trusted_issuer` writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerChange {
    /// The issuer URL, exactly as the tokens' `iss`.
    pub issuer: String,
    /// The audience the tokens must carry.
    pub audience: String,
    /// A name for people; the issuer when blank.
    pub label: String,
    /// The id of the row with this issuer and audience, which gets the new label.
    pub replaces: Option<u64>,
}

/// The operator check every trusted-issuer reducer runs first.
fn require_operator_for_issuers(sender_is_operator: bool) -> Result<(), String> {
    if sender_is_operator {
        Ok(())
    } else {
        Err("Only an operator can manage trusted issuers.".to_owned())
    }
}

/// The row with exactly this issuer and audience.
fn find_row<'a>(
    rows: &'a [TrustedIssuer],
    issuer: &str,
    audience: &str,
) -> Option<&'a TrustedIssuer> {
    rows.iter()
        .find(|row| row.issuer == issuer && row.audience == audience)
}

/// `add_trusted_issuer`: an operator trusts tokens from `issuer` meant for `audience`.
///
/// # Errors
///
/// When the caller is not an operator, the issuer is not an `http(s)` URL of at most
/// [`MAX_CLAIM_BYTES`] (the host only fetches keys from those, `validate_url_scheme` in
/// `token_validation.rs`), the issuer is the host's own, or the audience is blank.
pub fn plan_add_trusted_issuer(
    sender_is_operator: bool,
    existing: &[TrustedIssuer],
    issuer: &str,
    audience: &str,
    label: &str,
) -> Result<IssuerChange, String> {
    require_operator_for_issuers(sender_is_operator)?;
    let issuer = js_trim(issuer);
    if issuer == DEVICE_ISSUER {
        return Err(format!(
            "{DEVICE_ISSUER} is this host's own issuer: device identities are turned on and off with set_device_identities."
        ));
    }
    if !(issuer.starts_with("https://") || issuer.starts_with("http://")) {
        return Err(
            "An issuer is the URL in the tokens' iss claim, starting with https:// or http://."
                .to_owned(),
        );
    }
    if issuer.len() > MAX_CLAIM_BYTES {
        return Err(format!("An issuer is at most {MAX_CLAIM_BYTES} bytes."));
    }
    let audience = js_trim(audience);
    if audience.is_empty() {
        return Err(
            "A trusted issuer needs the audience its tokens carry: the aud claim, or the WorkOS client id."
                .to_owned(),
        );
    }
    let label = match js_trim(label) {
        "" => issuer,
        trimmed => trimmed,
    };
    Ok(IssuerChange {
        issuer: issuer.to_owned(),
        audience: audience.to_owned(),
        label: label.to_owned(),
        replaces: find_row(existing, issuer, audience).map(|row| row.id),
    })
}

/// `remove_trusted_issuer`: the id of the row to delete. Connections already open stay open;
/// the next connection with such a token is refused.
///
/// # Errors
///
/// When the caller is not an operator or no row has that issuer and audience.
pub fn plan_remove_trusted_issuer(
    sender_is_operator: bool,
    existing: &[TrustedIssuer],
    issuer: &str,
    audience: &str,
) -> Result<u64, String> {
    require_operator_for_issuers(sender_is_operator)?;
    let (issuer, audience) = (js_trim(issuer), js_trim(audience));
    find_row(existing, issuer, audience)
        .map(|row| row.id)
        .ok_or_else(|| format!("No trusted issuer {issuer} for audience {audience}."))
}

/// `set_device_identities`: only an operator turns device identities on or off.
///
/// # Errors
///
/// When the caller is not an operator.
pub fn check_set_device_identities(sender_is_operator: bool) -> Result<(), String> {
    if sender_is_operator {
        Ok(())
    } else {
        Err("Only an operator can change whether device identities connect.".to_owned())
    }
}

/// `add_member_by_subject`: the identity the host will give a person who signs in at `issuer`
/// as `subject` ([`Identity::from_claims`]), so they can be made a member before they first
/// connect. The issuer must be trusted, which catches a mistyped issuer (a trailing slash makes
/// another identity).
///
/// # Errors
///
/// When no [`TrustedIssuer`] row has that issuer, or the subject is blank or longer than
/// [`MAX_CLAIM_BYTES`].
pub fn member_identity(
    trusted: &[TrustedIssuer],
    issuer: &str,
    subject: &str,
) -> Result<Identity, String> {
    let issuer = js_trim(issuer);
    if !trusted.iter().any(|row| row.issuer == issuer) {
        return Err(format!(
            "No trusted issuer {issuer}; an operator adds it with add_trusted_issuer first."
        ));
    }
    let subject = js_trim(subject);
    if subject.is_empty() {
        return Err(
            "A member needs the subject (sub claim) their tokens carry, such as a WorkOS user id."
                .to_owned(),
        );
    }
    if subject.len() > MAX_CLAIM_BYTES {
        return Err(format!("A subject is at most {MAX_CLAIM_BYTES} bytes."));
    }
    Ok(Identity::from_claims(issuer, subject))
}

/// The arguments of `add_member_by_subject`, less the organisation.
#[derive(Debug, Clone, Copy)]
pub struct SubjectArgs<'a> {
    /// The issuer of the person's tokens, a [`TrustedIssuer`] row's issuer.
    pub issuer: &'a str,
    /// Their `sub` claim, such as a WorkOS user id; also their member user id.
    pub subject: &'a str,
    /// Display name; the subject when blank.
    pub name: &'a str,
    /// May post.
    pub can_post: bool,
    /// May approve.
    pub can_approve: bool,
    /// May close periods and manage members.
    pub can_close: bool,
}

/// `add_member_by_subject`: `add_member` (`index.ts:597-638`) for the identity
/// [`member_identity`] derives, with the subject as the user id, as `create_org_ledger` makes
/// the JWT subject the owner's user id (`index.ts:581`). The caller's permission is checked
/// before the issuer, so only a closing member learns which issuers are trusted.
///
/// # Errors
///
/// The messages of [`rules::plan_add_member`] and [`member_identity`].
pub fn plan_add_member_by_subject(
    org_id: &str,
    sender: Identity,
    org_members: &[Member],
    trusted: &[TrustedIssuer],
    args: &SubjectArgs<'_>,
) -> Result<MemberChange, String> {
    let actor = rules::actor_of(org_id, org_members.iter().find(|m| m.identity == sender))?;
    rules::require_close(&actor)?;
    let identity = member_identity(trusted, args.issuer, args.subject)?;
    rules::plan_add_member(
        org_id,
        sender,
        org_members,
        &MemberArgs {
            identity,
            user_id: js_trim(args.subject),
            name: args.name,
            can_post: args.can_post,
            can_approve: args.can_approve,
            can_close: args.can_close,
        },
    )
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use spacetimedb::{AuthCtx, Timestamp};

    use super::*;

    const WORKOS: &str = "https://api.workos.com/user_management/client_01TEST";
    const OPERATOR: Identity = Identity::from_byte_array([9; 32]);

    fn trusted(id: u64, issuer: &str, audience: &str) -> TrustedIssuer {
        TrustedIssuer {
            id,
            issuer: issuer.to_owned(),
            audience: audience.to_owned(),
            label: "WorkOS".to_owned(),
            added_by: OPERATOR,
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn workos_rows() -> Vec<TrustedIssuer> {
        vec![trusted(1, WORKOS, "client_01TEST")]
    }

    fn claims(issuer: &str, audience: &[&str], client_id: Option<&str>) -> TokenClaims {
        TokenClaims {
            issuer: issuer.to_owned(),
            subject: "user_01ABC".to_owned(),
            audience: audience.iter().map(|&a| a.to_owned()).collect(),
            client_id: client_id.map(str::to_owned),
        }
    }

    fn device() -> TokenClaims {
        claims(DEVICE_ISSUER, &["spacetimedb"], None)
    }

    fn setting(allow: bool) -> AuthSetting {
        AuthSetting {
            id: 0,
            allow_device_identities: allow,
            changed_by: OPERATOR,
            changed_at: Timestamp::UNIX_EPOCH,
        }
    }

    // The identity the host derives (crates/auth/src/identity.rs at v2.12.0)

    #[test]
    fn from_claims_matches_an_identity_the_local_host_minted() {
        // POST http://127.0.0.1:3100/v1/identity on 2026-10-16 returned identity c200f4a9...
        // with a token whose iss was "localhost" and sub this UUID.
        assert_eq!(
            Identity::from_claims("localhost", "aad0459e-6590-4f63-9608-2a578b08115f"),
            Identity::from_hex("c200f4a9e85b11d24644bd91cccc13e60094aedaabc166cd599932994c53ba96")
                .unwrap()
        );
    }

    // TokenClaims::of

    fn of(payload: &str) -> TokenClaims {
        let auth = AuthCtx::from_jwt_payload(payload.to_owned());
        TokenClaims::of(auth.jwt().unwrap())
    }

    #[test]
    fn token_claims_read_a_workos_access_token_without_aud() {
        assert_eq!(
            of(&format!(
                r#"{{"iss":"{WORKOS}","sub":"user_01ABC","sid":"session_1","client_id":"client_01TEST","exp":1,"iat":0}}"#
            )),
            claims(WORKOS, &[], Some("client_01TEST"))
        );
    }

    #[test]
    fn token_claims_read_a_device_token() {
        assert_eq!(
            of(
                r#"{"hex_identity":"c200","iss":"localhost","sub":"user_01ABC","aud":["spacetimedb"],"iat":0}"#
            ),
            device()
        );
    }

    #[test]
    fn token_claims_read_a_single_string_aud() {
        assert_eq!(
            of(r#"{"iss":"https://idp.example","sub":"user_01ABC","aud":"flowtrace"}"#).audience,
            vec!["flowtrace".to_owned()]
        );
    }

    #[test]
    fn client_id_of_ignores_a_client_id_that_is_not_a_string() {
        assert_eq!(client_id_of(r#"{"client_id":7}"#), None);
    }

    #[test]
    fn client_id_of_ignores_a_payload_that_is_not_json() {
        assert_eq!(client_id_of("not json"), None);
    }

    // check_connection: operators

    #[test]
    fn an_operator_connects_with_devices_off_and_no_trusted_issuer() {
        assert_eq!(
            check_connection(true, false, Some(&device()), false, &[]),
            Ok(Admission::Operator)
        );
    }

    #[test]
    fn an_operator_connects_with_an_untrusted_issuer() {
        let token = claims("https://auth.spacetimedb.com/oidc", &["cli"], None);
        assert_eq!(
            check_connection(true, false, Some(&token), false, &workos_rows()),
            Ok(Admission::Operator)
        );
    }

    // check_connection: services (MOD2, transparency/services.rs)

    #[test]
    fn a_registered_service_connects_with_devices_off_and_no_trusted_issuer() {
        assert_eq!(
            check_connection(false, true, Some(&device()), false, &[]),
            Ok(Admission::Service)
        );
    }

    #[test]
    fn a_registered_service_connects_with_a_token_from_an_untrusted_issuer() {
        let token = claims("https://api.workos.com", &["m2m"], Some("client_M2M"));
        assert_eq!(
            check_connection(false, true, Some(&token), false, &workos_rows()),
            Ok(Admission::Service)
        );
    }

    #[test]
    fn a_removed_service_is_let_in_only_by_the_usual_rules() {
        assert!(check_connection(false, false, Some(&device()), false, &[]).is_err());
    }

    #[test]
    fn an_operator_who_is_also_a_service_is_admitted_as_an_operator() {
        assert_eq!(
            check_connection(true, true, Some(&device()), false, &[]),
            Ok(Admission::Operator)
        );
    }

    // check_connection: device identities

    #[test]
    fn a_device_identity_connects_while_devices_are_allowed() {
        assert_eq!(
            check_connection(false, false, Some(&device()), true, &[]),
            Ok(Admission::Device)
        );
    }

    #[test]
    fn a_device_identity_is_refused_when_devices_are_off() {
        assert_eq!(
            check_connection(false, false, Some(&device()), false, &workos_rows()).unwrap_err(),
            "This database does not accept device identities; sign in with an account it trusts."
        );
    }

    #[test]
    fn a_connection_without_a_token_counts_as_a_device_identity() {
        assert_eq!(
            check_connection(false, false, None, true, &[]),
            Ok(Admission::Device)
        );
    }

    #[test]
    fn a_connection_without_a_token_is_refused_when_devices_are_off() {
        assert!(check_connection(false, false, None, false, &workos_rows()).is_err());
    }

    #[test]
    fn localhost_cannot_be_trusted_as_an_issuer_to_get_round_the_device_switch() {
        let rows = vec![trusted(1, DEVICE_ISSUER, "spacetimedb")];
        assert!(check_connection(false, false, Some(&device()), false, &rows).is_err());
    }

    // check_connection: trusted issuers

    #[test]
    fn a_workos_token_connects_by_its_client_id_when_it_has_no_aud() {
        let token = claims(WORKOS, &[], Some("client_01TEST"));
        assert_eq!(
            check_connection(false, false, Some(&token), false, &workos_rows()),
            Ok(Admission::Trusted)
        );
    }

    #[test]
    fn a_token_connects_when_one_of_its_audiences_is_trusted() {
        let token = claims(WORKOS, &["other", "client_01TEST"], None);
        assert_eq!(
            check_connection(false, false, Some(&token), false, &workos_rows()),
            Ok(Admission::Trusted)
        );
    }

    #[test]
    fn an_aud_claim_wins_over_client_id() {
        let token = claims(WORKOS, &["other"], Some("client_01TEST"));
        assert_eq!(
            check_connection(false, false, Some(&token), true, &workos_rows()).unwrap_err(),
            format!("This database does not trust tokens from {WORKOS} for audience other.")
        );
    }

    #[test]
    fn a_token_without_any_audience_is_refused() {
        let token = claims(WORKOS, &[], None);
        assert_eq!(
            check_connection(false, false, Some(&token), true, &workos_rows()).unwrap_err(),
            format!("This database does not trust tokens from {WORKOS} for audience (none).")
        );
    }

    #[test]
    fn a_token_from_an_untrusted_issuer_is_refused() {
        let token = claims("https://evil.example", &["client_01TEST"], None);
        assert_eq!(
            check_connection(false, false, Some(&token), true, &workos_rows()).unwrap_err(),
            "This database does not trust tokens from https://evil.example."
        );
    }

    #[test]
    fn the_issuer_must_match_exactly_including_a_trailing_slash() {
        let token = claims(&format!("{WORKOS}/"), &["client_01TEST"], None);
        assert!(check_connection(false, false, Some(&token), true, &workos_rows()).is_err());
    }

    #[test]
    fn the_audience_of_another_trusted_row_does_not_count() {
        let rows = vec![
            trusted(1, WORKOS, "client_01TEST"),
            trusted(2, "https://idp.example", "flowtrace"),
        ];
        let token = claims(WORKOS, &["flowtrace"], None);
        assert!(check_connection(false, false, Some(&token), true, &rows).is_err());
    }

    #[test]
    fn a_trusted_token_connects_while_devices_are_also_allowed() {
        let token = claims(WORKOS, &["client_01TEST"], None);
        assert_eq!(
            check_connection(false, false, Some(&token), true, &workos_rows()),
            Ok(Admission::Trusted)
        );
    }

    // device_identities_allowed

    #[test]
    fn device_identities_are_allowed_until_an_operator_says_otherwise() {
        assert!(device_identities_allowed(None));
    }

    #[test]
    fn device_identities_follow_the_setting() {
        assert_eq!(
            (
                device_identities_allowed(Some(&setting(false))),
                device_identities_allowed(Some(&setting(true)))
            ),
            (false, true)
        );
    }

    #[test]
    fn set_device_identities_needs_an_operator() {
        assert_eq!(
            check_set_device_identities(false).unwrap_err(),
            "Only an operator can change whether device identities connect."
        );
    }

    #[test]
    fn set_device_identities_lets_an_operator_through() {
        assert_eq!(check_set_device_identities(true), Ok(()));
    }

    // add_trusted_issuer

    #[test]
    fn add_trusted_issuer_needs_an_operator() {
        assert_eq!(
            plan_add_trusted_issuer(false, &[], WORKOS, "client_01TEST", "WorkOS").unwrap_err(),
            "Only an operator can manage trusted issuers."
        );
    }

    #[test]
    fn add_trusted_issuer_trims_and_adds_a_new_row() {
        assert_eq!(
            plan_add_trusted_issuer(
                true,
                &[],
                &format!(" {WORKOS}\n"),
                " client_01TEST ",
                " WorkOS staging "
            ),
            Ok(IssuerChange {
                issuer: WORKOS.to_owned(),
                audience: "client_01TEST".to_owned(),
                label: "WorkOS staging".to_owned(),
                replaces: None,
            })
        );
    }

    #[test]
    fn add_trusted_issuer_labels_a_blank_label_with_the_issuer() {
        assert_eq!(
            plan_add_trusted_issuer(true, &[], WORKOS, "client_01TEST", "  ")
                .unwrap()
                .label,
            WORKOS
        );
    }

    #[test]
    fn add_trusted_issuer_relabels_the_row_with_the_same_issuer_and_audience() {
        let rows = vec![
            trusted(4, WORKOS, "other"),
            trusted(7, WORKOS, "client_01TEST"),
        ];
        assert_eq!(
            plan_add_trusted_issuer(true, &rows, WORKOS, "client_01TEST", "New")
                .unwrap()
                .replaces,
            Some(7)
        );
    }

    #[test]
    fn add_trusted_issuer_adds_a_second_audience_for_one_issuer() {
        assert_eq!(
            plan_add_trusted_issuer(true, &workos_rows(), WORKOS, "client_02DESKTOP", "")
                .unwrap()
                .replaces,
            None
        );
    }

    #[test]
    fn add_trusted_issuer_refuses_the_hosts_own_issuer() {
        assert_eq!(
            plan_add_trusted_issuer(true, &[], "localhost", "spacetimedb", "").unwrap_err(),
            "localhost is this host's own issuer: device identities are turned on and off with set_device_identities."
        );
    }

    #[test]
    fn add_trusted_issuer_refuses_an_issuer_that_is_not_a_url() {
        assert_eq!(
            plan_add_trusted_issuer(true, &[], "api.workos.com", "client_01TEST", "").unwrap_err(),
            "An issuer is the URL in the tokens' iss claim, starting with https:// or http://."
        );
    }

    #[test]
    fn add_trusted_issuer_accepts_a_plain_http_issuer_for_a_local_test() {
        assert!(
            plan_add_trusted_issuer(true, &[], "http://127.0.0.1:3199/trusted", "proof", "")
                .is_ok()
        );
    }

    #[test]
    fn add_trusted_issuer_refuses_an_issuer_longer_than_the_host_accepts() {
        let long = format!("https://{}", "a".repeat(121));
        assert_eq!(long.len(), 129);
        assert_eq!(
            plan_add_trusted_issuer(true, &[], &long, "client_01TEST", "").unwrap_err(),
            "An issuer is at most 128 bytes."
        );
    }

    #[test]
    fn add_trusted_issuer_accepts_an_issuer_of_128_bytes() {
        let long = format!("https://{}", "a".repeat(120));
        assert!(plan_add_trusted_issuer(true, &[], &long, "client_01TEST", "").is_ok());
    }

    #[test]
    fn add_trusted_issuer_refuses_a_blank_audience() {
        assert_eq!(
            plan_add_trusted_issuer(true, &[], WORKOS, " ", "").unwrap_err(),
            "A trusted issuer needs the audience its tokens carry: the aud claim, or the WorkOS client id."
        );
    }

    // remove_trusted_issuer

    #[test]
    fn remove_trusted_issuer_needs_an_operator() {
        assert_eq!(
            plan_remove_trusted_issuer(false, &workos_rows(), WORKOS, "client_01TEST").unwrap_err(),
            "Only an operator can manage trusted issuers."
        );
    }

    #[test]
    fn remove_trusted_issuer_returns_the_rows_id() {
        let rows = vec![
            trusted(4, WORKOS, "other"),
            trusted(7, WORKOS, "client_01TEST"),
        ];
        assert_eq!(
            plan_remove_trusted_issuer(true, &rows, WORKOS, " client_01TEST "),
            Ok(7)
        );
    }

    #[test]
    fn remove_trusted_issuer_refuses_an_issuer_audience_pair_that_is_not_trusted() {
        assert_eq!(
            plan_remove_trusted_issuer(true, &workos_rows(), WORKOS, "other").unwrap_err(),
            format!("No trusted issuer {WORKOS} for audience other.")
        );
    }

    // add_member_by_subject

    #[test]
    fn member_identity_is_the_identity_the_host_derives() {
        assert_eq!(
            member_identity(&workos_rows(), WORKOS, " user_01ABC "),
            Ok(Identity::from_claims(WORKOS, "user_01ABC"))
        );
    }

    #[test]
    fn member_identity_equals_the_identity_of_a_token_for_that_subject() {
        let auth = AuthCtx::from_jwt_payload(format!(
            r#"{{"iss":"{WORKOS}","sub":"user_01ABC","client_id":"client_01TEST"}}"#
        ));
        assert_eq!(
            member_identity(&workos_rows(), WORKOS, "user_01ABC").unwrap(),
            auth.jwt().unwrap().identity()
        );
    }

    #[test]
    fn member_identity_needs_a_trusted_issuer() {
        assert_eq!(
            member_identity(&workos_rows(), &format!("{WORKOS}/"), "user_01ABC").unwrap_err(),
            format!(
                "No trusted issuer {WORKOS}/; an operator adds it with add_trusted_issuer first."
            )
        );
    }

    #[test]
    fn member_identity_refuses_the_device_issuer() {
        assert!(member_identity(&[], DEVICE_ISSUER, "user_01ABC").is_err());
    }

    #[test]
    fn member_identity_needs_a_subject() {
        assert_eq!(
            member_identity(&workos_rows(), WORKOS, " ").unwrap_err(),
            "A member needs the subject (sub claim) their tokens carry, such as a WorkOS user id."
        );
    }

    #[test]
    fn member_identity_refuses_a_subject_longer_than_the_host_accepts() {
        assert_eq!(
            member_identity(&workos_rows(), WORKOS, &"u".repeat(129)).unwrap_err(),
            "A subject is at most 128 bytes."
        );
    }

    // add_member_by_subject

    const OWNER: Identity = Identity::from_byte_array([1; 32]);
    const CLERK: Identity = Identity::from_byte_array([2; 32]);

    fn member(identity: Identity, user_id: &str, can_close: bool) -> Member {
        Member {
            key: rules::member_key("org-1", identity),
            identity,
            org_id: "org-1".to_owned(),
            user_id: user_id.to_owned(),
            name: user_id.to_uppercase(),
            can_post: true,
            can_approve: false,
            can_close,
        }
    }

    fn members() -> Vec<Member> {
        vec![member(OWNER, "owner", true), member(CLERK, "clerk", false)]
    }

    fn by_subject<'a>(issuer: &'a str, subject: &'a str) -> SubjectArgs<'a> {
        SubjectArgs {
            issuer,
            subject,
            name: " Wanjiru ",
            can_post: true,
            can_approve: true,
            can_close: false,
        }
    }

    #[test]
    fn add_member_by_subject_adds_the_derived_identity_with_the_subject_as_user_id() {
        let change = plan_add_member_by_subject(
            "org-1",
            OWNER,
            &members(),
            &workos_rows(),
            &by_subject(WORKOS, " user_01ABC "),
        )
        .unwrap();
        let identity = Identity::from_claims(WORKOS, "user_01ABC");
        assert_eq!(
            (change.row, change.replaces),
            (
                Member {
                    key: rules::member_key("org-1", identity),
                    identity,
                    org_id: "org-1".to_owned(),
                    user_id: "user_01ABC".to_owned(),
                    name: "Wanjiru".to_owned(),
                    can_post: true,
                    can_approve: true,
                    can_close: false,
                },
                false
            )
        );
    }

    #[test]
    fn add_member_by_subject_audits_member_add() {
        let change = plan_add_member_by_subject(
            "org-1",
            OWNER,
            &members(),
            &workos_rows(),
            &by_subject(WORKOS, "user_01ABC"),
        )
        .unwrap();
        assert_eq!(
            (
                change.audit.action,
                change.audit.subject,
                change.audit.detail
            ),
            (
                "member.add",
                "user_01ABC".to_owned(),
                "Wanjiru: post, approve".to_owned()
            )
        );
    }

    #[test]
    fn add_member_by_subject_updates_a_member_who_already_connected() {
        let identity = Identity::from_claims(WORKOS, "user_01ABC");
        let mut rows = members();
        rows.push(member(identity, "user_01ABC", false));
        let change = plan_add_member_by_subject(
            "org-1",
            OWNER,
            &rows,
            &workos_rows(),
            &by_subject(WORKOS, "user_01ABC"),
        )
        .unwrap();
        assert_eq!(
            (change.replaces, change.audit.action),
            (true, "member.update")
        );
    }

    #[test]
    fn add_member_by_subject_refuses_a_non_member_before_looking_at_the_issuer() {
        assert_eq!(
            plan_add_member_by_subject(
                "org-1",
                OPERATOR,
                &members(),
                &[],
                &by_subject("https://unknown.example", "user_01ABC"),
            )
            .unwrap_err(),
            "You are not a member of organisation org-1."
        );
    }

    #[test]
    fn add_member_by_subject_needs_close_permission_before_looking_at_the_issuer() {
        assert_eq!(
            plan_add_member_by_subject(
                "org-1",
                CLERK,
                &members(),
                &[],
                &by_subject("https://unknown.example", "user_01ABC"),
            )
            .unwrap_err(),
            "Only members who can close periods can manage members."
        );
    }

    #[test]
    fn add_member_by_subject_needs_a_trusted_issuer() {
        assert_eq!(
            plan_add_member_by_subject(
                "org-1",
                OWNER,
                &members(),
                &workos_rows(),
                &by_subject("https://unknown.example", "user_01ABC"),
            )
            .unwrap_err(),
            "No trusted issuer https://unknown.example; an operator adds it with add_trusted_issuer first."
        );
    }

    #[test]
    fn add_member_by_subject_refuses_a_subject_that_is_another_members_user_id() {
        assert_eq!(
            plan_add_member_by_subject(
                "org-1",
                OWNER,
                &members(),
                &workos_rows(),
                &by_subject(WORKOS, "clerk"),
            )
            .unwrap_err(),
            "That user id belongs to another member."
        );
    }
}
