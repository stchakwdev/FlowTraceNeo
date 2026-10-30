#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;

const ORIGIN: &str = "https://flowtrace.app";

/// The web's literal output, from running the `TrustBadgeEmbed` expressions
/// (`embed-code-generator.tsx:291-297`) under bun with this origin and org id.
#[test]
fn the_codes_match_the_web_byte_for_byte() {
    assert_eq!(
        badge_url(ORIGIN, "org-123", BadgeStyle::Flat, "Trust Score"),
        "https://flowtrace.app/badge/trust/org-123?style=flat&label=Trust+Score"
    );
    assert_eq!(
        markdown_snippet(ORIGIN, "org-123", BadgeStyle::Flat, "Trust Score"),
        "[![Trust Score](https://flowtrace.app/badge/trust/org-123?style=flat&label=Trust+Score)](https://flowtrace.app/badge/trust/org-123)"
    );
    assert_eq!(
        html_snippet(ORIGIN, "org-123", BadgeStyle::Flat, "Trust Score"),
        "<a href=\"https://flowtrace.app/badge/trust/org-123\" target=\"_blank\" rel=\"noopener noreferrer\">\n  <img src=\"https://flowtrace.app/badge/trust/org-123?style=flat&label=Trust+Score\" alt=\"Trust Score on FlowTrace\" />\n</a>"
    );
    let odd = "Verified & \"ours\" \u{e9}/\u{fc}+*-._~!";
    assert_eq!(
        badge_url(ORIGIN, "org-123", BadgeStyle::ForTheBadge, odd),
        "https://flowtrace.app/badge/trust/org-123?style=for-the-badge&label=Verified+%26+%22ours%22+%C3%A9%2F%C3%BC%2B*-._%7E%21"
    );
    assert_eq!(
        markdown_snippet(ORIGIN, "org-123", BadgeStyle::ForTheBadge, odd),
        "[![Verified & \"ours\" \u{e9}/\u{fc}+*-._~!](https://flowtrace.app/badge/trust/org-123?style=for-the-badge&label=Verified+%26+%22ours%22+%C3%A9%2F%C3%BC%2B*-._%7E%21)](https://flowtrace.app/badge/trust/org-123)"
    );
    assert_eq!(
        html_snippet(ORIGIN, "org-123", BadgeStyle::ForTheBadge, odd),
        "<a href=\"https://flowtrace.app/badge/trust/org-123\" target=\"_blank\" rel=\"noopener noreferrer\">\n  <img src=\"https://flowtrace.app/badge/trust/org-123?style=for-the-badge&label=Verified+%26+%22ours%22+%C3%A9%2F%C3%BC%2B*-._%7E%21\" alt=\"Verified & &quot;ours&quot; \u{e9}/\u{fc}+*-._~! on FlowTrace\" />\n</a>"
    );
}

#[test]
fn a_blank_label_falls_back_and_a_label_is_trimmed() {
    assert_eq!(shown_label("   "), "Trust Score");
    assert_eq!(shown_label("  Our score "), "Our score");
}

#[test]
fn widths_heights_and_colours_follow_render_badge() {
    let flat = Badge::score("Trust Score", 87, BadgeStyle::Flat);
    assert_eq!(
        (flat.left_width, flat.right_width),
        (86, 59),
        "11 x 6.5 + 14, 6 x 7.5 + 14"
    );
    assert_eq!((flat.height, flat.text_y, flat.caps), (20, 14, false));
    assert_eq!(flat.right, "87/100");
    assert_eq!(flat.title, "Trust Score: 87/100");
    assert_eq!(flat.label_color, colors::INK);
    assert_eq!(flat.value_color, colors::REPORT);

    let large = Badge::score("Trust Score", 87, BadgeStyle::ForTheBadge);
    assert_eq!(
        (large.left_width, large.right_width),
        (103, 74),
        "x 1.1, pad 12"
    );
    assert_eq!((large.height, large.text_y, large.caps), (28, 18, true));
    assert_eq!(large.left, "TRUST SCORE");
    assert_eq!(large.width(), 177);

    for style in [BadgeStyle::FlatSquare, BadgeStyle::Plastic] {
        assert_eq!(
            Badge::score("Trust Score", 87, style),
            flat,
            "{style:?} renders as flat, as on the web"
        );
    }

    let grade = Badge::grade("FlowTrace", 45, BadgeStyle::Flat);
    assert_eq!(grade.right, "F");
    assert_eq!(
        (grade.left_width, grade.right_width),
        (73, 23),
        "glyph 9 for the grade"
    );
    assert_eq!(grade.title, "FlowTrace: Grade F");
    assert_eq!(grade.value_color, colors::MIDDLING);
    assert_eq!(
        Badge::score("x", 39, BadgeStyle::Flat).value_color,
        colors::FLAG
    );
    assert_eq!(
        Badge::score("x", 60, BadgeStyle::Flat).value_color,
        colors::REPORT
    );

    let escaped = Badge::score("R&D", 70, BadgeStyle::Flat);
    assert_eq!(
        escaped.left_width, 60,
        "the escaped R&amp;D is 7 long: 7 x 6.5 + 14 = 59.5"
    );
    assert_eq!(escaped.left, "R&D");
}

#[test]
fn styles_read_back_from_their_url_values() {
    let labels: Vec<&str> = BadgeStyle::ALL.iter().map(|s| s.label()).collect();
    assert_eq!(labels, ["Flat", "Flat, square corners", "Large", "Plastic"]);
    for s in BadgeStyle::ALL {
        assert_eq!(BadgeStyle::from_slug(s.slug()).unwrap(), s);
    }
    assert_eq!(BadgeStyle::from_slug("round"), None);
}
