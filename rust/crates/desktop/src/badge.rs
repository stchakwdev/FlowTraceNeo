//! The Trust Score badge (DT4), plain Rust with no GPUI: the geometry and colours of
//! `flowtrace/src/lib/badge-utils.ts` (`renderBadge`, `generateTrustBadgeSVG`,
//! `generateGradeBadgeSVG`) and the codes of the "Trust Score badge" section,
//! `flowtrace/src/components/embed-code-generator.tsx:274-378` (`TrustBadgeEmbed`).
//!
//! The web draws every style as two flat, square-cornered fields; only "Large"
//! (`for-the-badge`) differs, in height, padding and capitals. Flat, "Flat, square corners" and
//! "Plastic" render alike there, and so they do here.

use std::fmt::Write as _;

use ledger_module::transparency::trust::trust_grade;

#[cfg(test)]
mod tests;

/// A badge style, as the badge URL names it (`BadgeStyle`, `badge-utils.ts:1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeStyle {
    /// `flat`.
    #[default]
    Flat,
    /// `flat-square`.
    FlatSquare,
    /// `for-the-badge`, shown as "Large".
    ForTheBadge,
    /// `plastic`.
    Plastic,
}

impl BadgeStyle {
    /// The four in the Style select's order (`BADGE_STYLES`, `embed-code-generator.tsx:274-279`).
    pub const ALL: [Self; 4] = [
        Self::Flat,
        Self::FlatSquare,
        Self::ForTheBadge,
        Self::Plastic,
    ];

    /// The URL value.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::FlatSquare => "flat-square",
            Self::ForTheBadge => "for-the-badge",
            Self::Plastic => "plastic",
        }
    }

    /// The Style select's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::FlatSquare => "Flat, square corners",
            Self::ForTheBadge => "Large",
            Self::Plastic => "Plastic",
        }
    }

    /// The style a URL value names.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.slug() == slug)
    }
}

/// The report palette (`BADGE_COLORS`, `badge-utils.ts:25-30`), `0xRRGGBB`.
pub mod colors {
    /// Ink, the label field.
    pub const INK: u32 = 0x0013_202D;
    /// Report blue, a sound score (60 and up).
    pub const REPORT: u32 = 0x001C_5FA8;
    /// Ink grey, a middling score (40 to 59).
    pub const MIDDLING: u32 = 0x004A_5866;
    /// Signal orange, a score low enough to be a flag.
    pub const FLAG: u32 = 0x00C4_501B;
}

/// `getScoreColor(score)` (`badge-utils.ts:34-38`).
#[must_use]
pub const fn score_color(score: i64) -> u32 {
    if score >= 60 {
        colors::REPORT
    } else if score >= 40 {
        colors::MIDDLING
    } else {
        colors::FLAG
    }
}

/// The label the Label field leaves when blank (`useState('Trust Score')`).
pub const DEFAULT_LABEL: &str = "Trust Score";

/// The longest label the field takes (`maxLength={24}`).
pub const LABEL_MAX: usize = 24;

/// The Label hint.
pub const LABEL_HINT: &str = "The words on the left of the badge. Up to 24 characters.";

/// One badge, laid out as `renderBadge` lays it out (`badge-utils.ts:64-95`): sizes in pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    /// The label text as drawn: capitals for Large.
    pub left: String,
    /// `87/100` or the grade.
    pub right: String,
    /// The label field's width.
    pub left_width: u32,
    /// The value field's width.
    pub right_width: u32,
    /// 20, or 28 for Large.
    pub height: u32,
    /// The text baseline from the top: 14, or 18 for Large.
    pub text_y: u32,
    /// Capitals with 0.08em letter spacing: Large only.
    pub caps: bool,
    /// The label field's colour.
    pub label_color: u32,
    /// The value field's colour.
    pub value_color: u32,
    /// The badge's title, `Trust Score: 87/100`.
    pub title: String,
}

/// The font sizes and weights the badge text is set in (`badge-utils.ts:85-89`): Archivo at
/// 11px, the label 600 and the value 800.
pub const FONT_SIZE: f32 = 11.0;

/// `escapeXml(text)` (`badge-utils.ts:13-20`): the label as the SVG holds it, whose length sets
/// the width.
#[must_use]
pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `Math.round` of a field width: half up, as the badge widths are positive.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a badge field is a few hundred pixels wide"
)]
fn round_width(w: f64) -> u32 {
    (w + 0.5).floor() as u32
}

/// A field's width: its text's length in UTF-16 units times the glyph width, plus padding.
#[expect(
    clippy::cast_precision_loss,
    reason = "a label is a few dozen characters"
)]
fn width(text: &str, glyph: f64, large: bool) -> u32 {
    let pad = if large { 12.0 } else { 7.0 };
    let scale = if large { 1.1 } else { 1.0 };
    round_width(text.encode_utf16().count() as f64 * glyph * scale + pad * 2.0)
}

impl Badge {
    fn lay_out(
        label: &str,
        right: String,
        right_glyph: f64,
        style: BadgeStyle,
        score: i64,
    ) -> Self {
        let large = style == BadgeStyle::ForTheBadge;
        let safe = escape_xml(label);
        Self {
            left_width: width(&safe, 6.5, large),
            right_width: width(&right, right_glyph, large),
            left: if large {
                label.to_uppercase()
            } else {
                label.to_owned()
            },
            height: if large { 28 } else { 20 },
            text_y: if large { 18 } else { 14 },
            caps: large,
            label_color: colors::INK,
            value_color: score_color(score),
            title: String::new(),
            right,
        }
    }

    /// `generateTrustBadgeSVG` (`badge-utils.ts:97-118`): `label` beside `score/100`.
    #[must_use]
    pub fn score(label: &str, score: i64, style: BadgeStyle) -> Self {
        let right = format!("{score}/100");
        let title = format!("{label}: {right}");
        Self {
            title,
            ..Self::lay_out(label, right, 7.5, style, score)
        }
    }

    /// `generateGradeBadgeSVG` (`badge-utils.ts:120-142`): `label` beside the grade.
    #[must_use]
    pub fn grade(label: &str, score: i64, style: BadgeStyle) -> Self {
        let grade = trust_grade(score).to_owned();
        let title = format!("{label}: Grade {grade}");
        Self {
            title,
            ..Self::lay_out(label, grade, 9.0, style, score)
        }
    }

    /// The whole width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.left_width + self.right_width
    }
}

/// The label shown: the field trimmed, else [`DEFAULT_LABEL`] (`shownLabel`,
/// `embed-code-generator.tsx:289`).
#[must_use]
pub fn shown_label(field: &str) -> &str {
    let t = field.trim();
    if t.is_empty() { DEFAULT_LABEL } else { t }
}

/// `application/x-www-form-urlencoded` of one value, as `URLSearchParams.toString` writes it:
/// ASCII letters, digits and `*-._` kept, space as `+`, every other byte `%XX`.
#[must_use]
pub fn form_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(char::from(b));
            }
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

/// The organisation's public badge page (`profileUrl`).
#[must_use]
pub fn profile_url(origin: &str, org_id: &str) -> String {
    format!("{origin}/badge/trust/{org_id}")
}

/// `badgeUrl` (`embed-code-generator.tsx:291-293`): the badge page with the style and label
/// form-encoded.
#[must_use]
pub fn badge_url(origin: &str, org_id: &str, style: BadgeStyle, label: &str) -> String {
    format!(
        "{}?style={}&label={}",
        profile_url(origin, org_id),
        form_encode(style.slug()),
        form_encode(label)
    )
}

/// The "HTML, for websites" code (`html`, `embed-code-generator.tsx:295-297`), byte for byte.
#[must_use]
pub fn html_snippet(origin: &str, org_id: &str, style: BadgeStyle, label: &str) -> String {
    format!(
        "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">\n  <img src=\"{}\" alt=\"{} on FlowTrace\" />\n</a>",
        profile_url(origin, org_id),
        badge_url(origin, org_id, style, label),
        label.replace('"', "&quot;")
    )
}

/// The "Markdown, for READMEs and docs" code (`markdown`, `embed-code-generator.tsx:294`).
#[must_use]
pub fn markdown_snippet(origin: &str, org_id: &str, style: BadgeStyle, label: &str) -> String {
    format!(
        "[![{label}]({})]({})",
        badge_url(origin, org_id, style, label),
        profile_url(origin, org_id)
    )
}
