#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;
use crate::backend::demo_transparency;

fn form() -> ProfileForm {
    Profile::of(&demo_transparency().unwrap(), "x", "KES").form
}

#[test]
fn the_demo_profile_fills_the_form() {
    let t = demo_transparency().unwrap();
    let p = Profile::of(&t, "ignored", "USD");
    assert_eq!(
        p.form,
        ProfileForm {
            name: "Demo NGO Kenya".to_owned(),
            org_type: OrgType::Ngo,
            country: "KE".to_owned(),
            currency: "KES".to_owned(),
            public_slug: "demo-ngo".to_owned(),
        }
    );
    assert!(p.is_public);
    assert!(p.last_changed.unwrap().starts_with("Last changed "));
}

#[test]
fn without_a_profile_the_form_starts_from_the_books() {
    let p = Profile::of(&OrgTransparency::default(), "New organisation", "KES");
    assert_eq!(p.form.name, "New organisation");
    assert_eq!(p.form.org_type, OrgType::Ngo);
    assert_eq!(p.form.country, "");
    assert_eq!(p.form.currency, "KES");
    assert_eq!(p.form.public_slug, "");
    assert!(!p.is_public);
    assert_eq!(p.last_changed, None);
}

#[test]
fn validation_uses_the_web_texts() {
    let mut f = form();
    assert!(f.validate().is_empty());
    f.name = " A ".to_owned();
    assert_eq!(
        f.validate().name,
        Some("Enter your organisation\u{2019}s name.")
    );
    assert_eq!(
        f.shown_errors(false).name,
        None,
        "the name waits for a submit"
    );
    assert!(f.shown_errors(true).name.is_some());
    for (slug, error) in [
        ("", "Choose a public link name."),
        (
            "Demo",
            "Use lowercase letters, numbers and single hyphens, like demo-ngo.",
        ),
        (
            "demo--ngo",
            "Use lowercase letters, numbers and single hyphens, like demo-ngo.",
        ),
        (
            "-demo",
            "Use lowercase letters, numbers and single hyphens, like demo-ngo.",
        ),
        (
            "demo_ngo",
            "Use lowercase letters, numbers and single hyphens, like demo-ngo.",
        ),
        (&"a".repeat(49), "Keep it under 48 characters."),
    ] {
        f.public_slug = slug.to_owned();
        assert_eq!(f.validate().public_slug, Some(error), "{slug}");
        assert_eq!(
            f.shown_errors(false).public_slug,
            Some(error),
            "shown at once"
        );
    }
    f.public_slug = "a".repeat(48);
    assert_eq!(f.validate().public_slug, None);
    assert!(is_slug("water-2026-ke"));
}

#[test]
fn the_link_name_is_lowercased_and_trimmed_as_typed() {
    assert_eq!(normalise_slug(" Demo-NGO "), "demo-ngo");
    assert_eq!(
        slug_hint(""),
        "Used in your public campaign pages and badges: flowtrace.app/your-name"
    );
    assert_eq!(
        slug_hint("demo-ngo"),
        "Used in your public campaign pages and badges: flowtrace.app/demo-ngo"
    );
}

#[test]
fn saving_sends_every_field_with_the_name_trimmed() {
    let mut f = form();
    f.name = "  Turkana Water  ".to_owned();
    assert_eq!(
        serde_json::to_string(&f.patch()).unwrap(),
        r#"{"name":"Turkana Water","type":"NGO","country":"KE","currency":"KES","public_slug":"demo-ngo"}"#
    );
    assert_eq!(
        serde_json::to_string(&public_patch(true)).unwrap(),
        r#"{"is_public":true}"#,
        "the switch sends is_public alone"
    );
}

#[test]
fn status_lines_follow_the_web() {
    assert_eq!(
        save_status(
            Some("That public link is already taken. Choose another."),
            false,
            true
        )
        .unwrap(),
        "Not saved. That public link is already taken. Choose another."
    );
    assert_eq!(
        save_status(Some(""), false, true).unwrap(),
        "Not saved. Try again in a moment."
    );
    assert_eq!(
        save_status(None, true, false).unwrap(),
        "Saved. Your profile is up to date."
    );
    assert_eq!(
        save_status(None, false, true).unwrap(),
        "You have unsaved changes."
    );
    assert_eq!(save_status(None, false, false), None);
    assert_eq!(
        public_status(true, None),
        "Your name, Trust Score and public campaigns can be seen by anyone with the link."
    );
    assert_eq!(
        public_status(false, None),
        "Hidden. Donation pages, badges and embeds show nothing about you."
    );
    assert_eq!(public_status(true, Some("No.")), "Not changed. No.");
}

#[test]
fn option_lists_and_links_match_the_web() {
    assert_eq!(type_label(OrgType::Sme), "Business");
    assert_eq!(country_label("ZA"), "South Africa");
    assert_eq!(country_label("FR"), "FR");
    assert_eq!(currency_label("GBP"), "GBP \u{b7} Pound sterling");
    assert_eq!(CURRENCIES.len(), 11);
    let l = links(5);
    assert_eq!(
        l[0].description,
        "Who sees which depth of your flows. 5 groups, from board to public."
    );
    assert_eq!(l[1].title, "Embed & badges");
    assert_eq!(l[2].title, "Chain links");
    assert_eq!(
        lede(Some("Demo NGO Kenya")),
        "Demo NGO Kenya: profile, access and what you share publicly."
    );
    assert_eq!(
        lede(None),
        "Your organisation: profile, access and what you share publicly."
    );
}
