//! The Settings screen's view models (DT4), plain Rust with no GPUI: the web's
//! `flowtrace/src/routes/_authenticated/settings/index.tsx` (the "Organisation profile" form and
//! the "Access and sharing" links) and `components/public-pages-switch.tsx` ("Show on public
//! pages", bound to the profile's `is_public`).
//!
//! The module checks only who may save and that a public link is not another organisation's
//! (`plan::update_organization`); the form's own checks are the web's (`validate`,
//! `settings/index.tsx:78-89`), with its texts.

use ledger_core::types::Nullish;
use ledger_module::transparency::OrgTransparency;
use ledger_module::transparency::types::{OrgType, OrganizationPatch};

use crate::web_format::format_date;

#[cfg(test)]
mod tests;

/// The Type select (`ORG_TYPES`, `settings/index.tsx:26-31`).
pub const ORG_TYPES: [(OrgType, &str); 4] = [
    (OrgType::Ngo, "NGO or nonprofit"),
    (OrgType::SocialEnterprise, "Social enterprise"),
    (OrgType::Sme, "Business"),
    (OrgType::Government, "Government body"),
];

/// The Country select (`COUNTRIES`, `settings/index.tsx:33-42`).
pub const COUNTRIES: [(&str, &str); 8] = [
    ("KE", "Kenya"),
    ("UG", "Uganda"),
    ("TZ", "Tanzania"),
    ("RW", "Rwanda"),
    ("ET", "Ethiopia"),
    ("NG", "Nigeria"),
    ("GH", "Ghana"),
    ("ZA", "South Africa"),
];

/// The Reporting currency select (`CURRENCIES`, `settings/index.tsx:44-56`).
pub const CURRENCIES: [(&str, &str); 11] = [
    ("KES", "KES \u{b7} Kenyan shilling"),
    ("UGX", "UGX \u{b7} Ugandan shilling"),
    ("TZS", "TZS \u{b7} Tanzanian shilling"),
    ("RWF", "RWF \u{b7} Rwandan franc"),
    ("ETB", "ETB \u{b7} Ethiopian birr"),
    ("NGN", "NGN \u{b7} Nigerian naira"),
    ("GHS", "GHS \u{b7} Ghanaian cedi"),
    ("ZAR", "ZAR \u{b7} South African rand"),
    ("USD", "USD \u{b7} US dollar"),
    ("EUR", "EUR \u{b7} Euro"),
    ("GBP", "GBP \u{b7} Pound sterling"),
];

/// The Name hint.
pub const NAME_HINT: &str = "As funders and the public will see it.";

/// The Reporting currency hint.
pub const CURRENCY_HINT: &str = "Amounts across your reports are shown in this currency.";

/// The label of a type.
#[must_use]
pub fn type_label(t: OrgType) -> &'static str {
    ORG_TYPES
        .iter()
        .find(|(x, _)| *x == t)
        .map_or("", |(_, label)| label)
}

/// The label of a country code; the code itself when it is not listed.
#[must_use]
pub fn country_label(code: &str) -> &str {
    COUNTRIES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code, |(_, label)| label)
}

/// The label of a currency code; the code itself when it is not listed.
#[must_use]
pub fn currency_label(code: &str) -> &str {
    CURRENCIES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code, |(_, label)| label)
}

/// The profile form (`ProfileForm`, `settings/index.tsx:60-66`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileForm {
    /// Name.
    pub name: String,
    /// Type; NGO when the profile has none.
    pub org_type: OrgType,
    /// Country code; empty when none is set.
    pub country: String,
    /// Currency code.
    pub currency: String,
    /// The public link name.
    pub public_slug: String,
}

/// What the profile reads as (`organizationFrom`,
/// `flowtrace/src/lib/ledger/transparency-model.ts:502-527`): the stored profile, else the
/// organisation's own name and the books' currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The form's starting values.
    pub form: ProfileForm,
    /// Shown on public pages.
    pub is_public: bool,
    /// `Last changed 4 Feb 2026`, when the profile was ever saved.
    pub last_changed: Option<String>,
}

impl Profile {
    /// The profile of `t`; `name` and `base_currency` stand in while none is stored.
    #[must_use]
    pub fn of(t: &OrgTransparency, name: &str, base_currency: &str) -> Self {
        match &t.profile {
            Some(p) => Self {
                form: ProfileForm {
                    name: p.name.clone(),
                    org_type: p.org_type.unwrap_or(OrgType::Ngo),
                    country: p.country.clone(),
                    currency: p.currency.clone(),
                    public_slug: p.public_slug.clone().unwrap_or_default(),
                },
                is_public: p.is_public,
                last_changed: Some(format!("Last changed {}", format_date(&p.updated_at))),
            },
            None => Self {
                form: ProfileForm {
                    name: name.to_owned(),
                    org_type: OrgType::Ngo,
                    country: String::new(),
                    currency: base_currency.to_owned(),
                    public_slug: String::new(),
                },
                is_public: false,
                last_changed: None,
            },
        }
    }
}

/// The form's refusals by field (`validate`, `settings/index.tsx:78-89`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileErrors {
    /// Name.
    pub name: Option<&'static str>,
    /// Public link name.
    pub public_slug: Option<&'static str>,
}

impl ProfileErrors {
    /// Nothing to fix.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.name.is_none() && self.public_slug.is_none()
    }
}

/// `SLUG_PATTERN` (`settings/index.tsx:58`): lowercase letters and digits in runs joined by
/// single hyphens.
#[must_use]
pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|run| {
            !run.is_empty()
                && run
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// What the Public link name field keeps of a keystroke: the text lowercased and trimmed
/// (`e.target.value.toLowerCase().trim()`).
#[must_use]
pub fn normalise_slug(input: &str) -> String {
    input.to_lowercase().trim().to_owned()
}

/// The Public link name hint (`settings/index.tsx:305`).
#[must_use]
pub fn slug_hint(slug: &str) -> String {
    let shown = if slug.is_empty() { "your-name" } else { slug };
    format!("Used in your public campaign pages and badges: flowtrace.app/{shown}")
}

impl ProfileForm {
    /// The form's refusals.
    #[must_use]
    pub fn validate(&self) -> ProfileErrors {
        let name = (self.name.trim().chars().count() < 2)
            .then_some("Enter your organisation\u{2019}s name.");
        let public_slug = if self.public_slug.is_empty() {
            Some("Choose a public link name.")
        } else if !is_slug(&self.public_slug) {
            Some("Use lowercase letters, numbers and single hyphens, like demo-ngo.")
        } else if self.public_slug.chars().count() > 48 {
            Some("Keep it under 48 characters.")
        } else {
            None
        };
        ProfileErrors { name, public_slug }
    }

    /// The errors shown: the name's only once the form was submitted, the link's at once
    /// (`showError`, `settings/index.tsx:147-148`).
    #[must_use]
    pub fn shown_errors(&self, submitted: bool) -> ProfileErrors {
        let all = self.validate();
        ProfileErrors {
            name: all.name.filter(|_| submitted),
            public_slug: all.public_slug,
        }
    }

    /// The patch the form saves: every field, the name trimmed (`save`,
    /// `settings/index.tsx:111-116`).
    #[must_use]
    pub fn patch(&self) -> OrganizationPatch {
        OrganizationPatch {
            name: Some(self.name.trim().to_owned()),
            org_type: Nullish::Value(self.org_type),
            country: Some(self.country.clone()),
            currency: Some(self.currency.clone()),
            public_slug: Nullish::Value(self.public_slug.clone()),
            ..OrganizationPatch::default()
        }
    }
}

/// The line beside the form's buttons (`settings/index.tsx:345-361`).
#[must_use]
pub fn save_status(error: Option<&str>, saved: bool, dirty: bool) -> Option<String> {
    if let Some(e) = error {
        let e = if e.is_empty() {
            "Try again in a moment."
        } else {
            e
        };
        return Some(format!("Not saved. {e}"));
    }
    if saved {
        return Some("Saved. Your profile is up to date.".to_owned());
    }
    dirty.then(|| "You have unsaved changes.".to_owned())
}

/// The "Show on public pages" switch's patch: `is_public` alone (`PublicPagesSwitch`).
#[must_use]
pub fn public_patch(on: bool) -> OrganizationPatch {
    OrganizationPatch {
        is_public: Some(on),
        ..OrganizationPatch::default()
    }
}

/// The switch's line (`public-pages-switch.tsx:44-57`).
#[must_use]
pub fn public_status(on: bool, error: Option<&str>) -> String {
    if let Some(e) = error {
        let e = if e.is_empty() {
            "Try again in a moment."
        } else {
            e
        };
        return format!("Not changed. {e}");
    }
    if on {
        "Your name, Trust Score and public campaigns can be seen by anyone with the link."
            .to_owned()
    } else {
        "Hidden. Donation pages, badges and embeds show nothing about you.".to_owned()
    }
}

/// The header's lede.
#[must_use]
pub fn lede(name: Option<&str>) -> String {
    format!(
        "{}: profile, access and what you share publicly.",
        name.unwrap_or("Your organisation")
    )
}

/// One "Access and sharing" link (`SettingsLink`, `settings/index.tsx:421-463`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLink {
    /// Title.
    pub title: &'static str,
    /// The line under it.
    pub description: String,
}

/// The three links, the access groups' with their count.
#[must_use]
pub fn links(groups: usize) -> Vec<SettingsLink> {
    vec![
        SettingsLink {
            title: "Access groups",
            description: format!(
                "Who sees which depth of your flows. {groups} groups, from board to public."
            ),
        },
        SettingsLink {
            title: "Embed & badges",
            description: "Campaign widgets and Trust Score badges for your website.".to_owned(),
        },
        SettingsLink {
            title: "Chain links",
            description: "Partners and funders whose flows are linked to yours.".to_owned(),
        },
    ]
}
