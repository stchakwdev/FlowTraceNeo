//! The `DESIGN.md` tokens (colours, type scale, spacing, radii) and how they go onto gpui-kit's
//! theme.
//!
//! Colours are written as `DESIGN.md` writes them, in OKLCH, and converted to sRGB here, so a
//! token can be checked against the design file by eye. Both palettes are defined there: each
//! light token has a `-dark` sibling. [`apply`] switches gpui-kit to the mode first and then
//! edits the colours in a second `Theme::update`, as gpui-kit asks: setting the mode loads that
//! mode's registered theme over any colours set in the same closure (gpui-component 0.7.3
//! `theme/mod.rs`, `Theme::update`; <https://gpui-kit.com/component/theme>).

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Pixels, Rgba, px};

/// A colour in OKLCH with an alpha, as `DESIGN.md` gives it: `oklch(L C H / A)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Lightness, 0 to 1.
    pub l: f32,
    /// Chroma.
    pub c: f32,
    /// Hue in degrees.
    pub h: f32,
    /// Alpha, 0 to 1.
    pub a: f32,
}

/// `oklch(l c h)`, opaque.
#[must_use]
pub const fn oklch(l: f32, c: f32, h: f32) -> Oklch {
    Oklch { l, c, h, a: 1.0 }
}

/// `oklch(l c h / a)`.
#[must_use]
pub const fn oklcha(l: f32, c: f32, h: f32, a: f32) -> Oklch {
    Oklch { l, c, h, a }
}

impl Oklch {
    /// This colour in sRGB, each channel clamped to 0..=1 (Björn Ottosson's OKLab matrices,
    /// <https://bottosson.github.io/posts/oklab/>, and the sRGB transfer function).
    #[must_use]
    pub fn to_rgba(self) -> Rgba {
        let (sin, cos) = self.h.to_radians().sin_cos();
        let (a, b) = (self.c * cos, self.c * sin);
        let l_ = self.l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = self.l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = self.l - 0.089_484_18 * a - 1.291_485_5 * b;
        let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
        let r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
        let g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
        let b = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;
        Rgba {
            r: encode_srgb(r),
            g: encode_srgb(g),
            b: encode_srgb(b),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// This colour as GPUI's [`Hsla`].
    #[must_use]
    pub fn to_hsla(self) -> Hsla {
        self.to_rgba().into()
    }
}

/// Linear light to the sRGB transfer curve, clamped into gamut.
fn encode_srgb(linear: f32) -> f32 {
    let x = linear.clamp(0.0, 1.0);
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// The colour tokens of `DESIGN.md` (frontmatter `colors`), one palette per mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// `report-blue`: the one action and "evidenced" colour.
    pub report_blue: Oklch,
    /// `report-blue-ink`: text on report blue.
    pub report_blue_ink: Oklch,
    /// `coverage-blue`: received but not yet evidenced; selection halo.
    pub coverage_blue: Oklch,
    /// `signal-orange`: flags only.
    pub signal_orange: Oklch,
    /// `rejection-red`: rejected proof, destructive actions, field errors.
    pub rejection_red: Oklch,
    /// `report-paper`: the page background.
    pub report_paper: Oklch,
    /// `sheet`: cards, inputs, dialogs; one step brighter than paper.
    pub sheet: Oklch,
    /// `ink-navy`: all text, heavy rules, the inverted active nav item.
    pub ink_navy: Oklch,
    /// `ink-muted`: secondary text and labels.
    pub ink_muted: Oklch,
    /// `gap-grey`: the unfunded remainder of a coverage bar.
    pub gap_grey: Oklch,
    /// `hairline`: 1px dividers.
    pub hairline: Oklch,
    /// `field-stroke`: input borders.
    pub field_stroke: Oklch,
    /// `wash`: hover background of ghost buttons and list rows.
    pub wash: Oklch,
    /// `rail`: the nav rail.
    pub rail: Oklch,
    /// `rail-hover`: a nav item under the pointer.
    pub rail_hover: Oklch,
    /// `masthead-ink`: the masthead band.
    pub masthead_ink: Oklch,
    /// `masthead-paper`: text on the masthead.
    pub masthead_paper: Oklch,
}

/// The light palette (`DESIGN.md` frontmatter, the unsuffixed tokens).
pub const LIGHT: Palette = Palette {
    report_blue: oklch(0.48, 0.13, 255.0),
    report_blue_ink: oklch(0.99, 0.004, 255.0),
    coverage_blue: oklch(0.71, 0.08, 255.0),
    signal_orange: oklch(0.6, 0.18, 40.0),
    rejection_red: oklch(0.55, 0.19, 35.0),
    report_paper: oklch(0.972, 0.004, 120.0),
    sheet: oklch(0.993, 0.002, 120.0),
    ink_navy: oklch(0.24, 0.035, 250.0),
    ink_muted: oklch(0.45, 0.03, 250.0),
    gap_grey: oklch(0.89, 0.012, 250.0),
    hairline: oklch(0.86, 0.014, 250.0),
    field_stroke: oklch(0.8, 0.018, 250.0),
    wash: oklch(0.94, 0.006, 250.0),
    rail: oklch(0.948, 0.007, 250.0),
    rail_hover: oklch(0.9, 0.02, 255.0),
    masthead_ink: oklch(0.24, 0.035, 250.0),
    masthead_paper: oklch(0.96, 0.006, 250.0),
};

/// The dark palette (`DESIGN.md` frontmatter, the `-dark` tokens). `DESIGN.md` gives no dark
/// `rail-hover` or `masthead-paper`; they are the light ones' roles on the dark rail and
/// masthead: `wash-dark` for the hover and `ink-navy-dark` for masthead text.
pub const DARK: Palette = Palette {
    report_blue: oklch(0.72, 0.11, 255.0),
    report_blue_ink: oklch(0.17, 0.03, 255.0),
    coverage_blue: oklch(0.52, 0.08, 255.0),
    signal_orange: oklch(0.72, 0.16, 45.0),
    rejection_red: oklch(0.68, 0.17, 35.0),
    report_paper: oklch(0.185, 0.02, 250.0),
    sheet: oklch(0.215, 0.022, 250.0),
    ink_navy: oklch(0.94, 0.008, 250.0),
    ink_muted: oklch(0.72, 0.02, 250.0),
    gap_grey: oklch(0.31, 0.02, 250.0),
    hairline: oklcha(1.0, 0.0, 0.0, 0.13),
    field_stroke: oklcha(1.0, 0.0, 0.0, 0.20),
    wash: oklch(0.25, 0.02, 250.0),
    rail: oklch(0.165, 0.02, 250.0),
    rail_hover: oklch(0.25, 0.02, 250.0),
    masthead_ink: oklch(0.13, 0.02, 250.0),
    masthead_paper: oklch(0.94, 0.008, 250.0),
};

/// The palette of a mode.
#[must_use]
pub fn palette(mode: ThemeMode) -> &'static Palette {
    if mode.is_dark() { &DARK } else { &LIGHT }
}

/// `DESIGN.md` type scale (frontmatter `typography`) in pixels at a 16px rem. Display sizes
/// that `DESIGN.md` gives as a `clamp` take their desktop value.
pub mod text {
    /// `headline`: 3rem, app page titles.
    pub const HEADLINE: f32 = 48.0;
    /// `title`: 1.5rem, panel and dialog titles.
    pub const TITLE: f32 = 24.0;
    /// `figure`: 2.75rem, key figures.
    pub const FIGURE: f32 = 44.0;
    /// `lede`: 1.125rem.
    pub const LEDE: f32 = 18.0;
    /// `body`: 0.875rem, prose and table cells.
    pub const BODY: f32 = 14.0;
    /// `section-label`: 0.75rem, uppercase, the title of a report section.
    pub const SECTION_LABEL: f32 = 12.0;
    /// `label`: 0.6875rem, badges and column heads.
    pub const LABEL: f32 = 11.0;
    /// `mono`: 0.8125rem, identifiers and codes.
    pub const MONO: f32 = 13.0;
    /// The display family `DESIGN.md` names first; GPUI falls back when it is not installed.
    pub const FAMILY: &str = "Archivo";
    /// The mono family `DESIGN.md` names first.
    pub const MONO_FAMILY: &str = "IBM Plex Mono";
}

/// `DESIGN.md` spacing (frontmatter `spacing`) and the shell sizes from its Layout and
/// Navigation sections, in pixels.
pub mod space {
    /// `cell`: padding inside a figure cell.
    pub const CELL: f32 = 16.0;
    /// `gutter`: page side gutter on desktop.
    pub const GUTTER: f32 = 32.0;
    /// `section-app`: space between app sections.
    pub const SECTION: f32 = 32.0;
    /// `rule-heavy`: the ink rule that opens every section.
    pub const RULE_HEAVY: f32 = 3.0;
    /// A hairline divider.
    pub const HAIRLINE: f32 = 1.0;
    /// The masthead band's height.
    pub const MASTHEAD: f32 = 56.0;
    /// The nav rail's width.
    pub const RAIL: f32 = 224.0;
    /// A nav item's height.
    pub const NAV_ITEM: f32 = 40.0;
    /// A section header row's minimum height.
    pub const SECTION_HEADER: f32 = 44.0;
    /// An input's height.
    pub const INPUT: f32 = 36.0;
    /// A button's height.
    pub const BUTTON: f32 = 40.0;
    /// The status bar's height.
    pub const STATUS_BAR: f32 = 28.0;
}

/// `DESIGN.md` radii (frontmatter `rounded`): square everywhere, 2px at most.
pub mod radius {
    /// `none`: buttons, inputs, cards, nav items.
    pub const NONE: f32 = 0.0;
    /// `hairline`: the largest radius allowed, for dialogs.
    pub const HAIRLINE: f32 = 2.0;
}

/// A pixel length from a token.
#[must_use]
pub fn px_of(token: f32) -> Pixels {
    px(token)
}

/// Switches gpui-kit to `mode` and puts this palette, the radii and the fonts onto its theme.
///
/// Token to gpui-kit field mapping, by the role `DESIGN.md` gives each token: paper is the
/// background, sheet the popover, table and input surface, ink the foreground, report blue every
/// primary and focus colour (buttons read their own `button_primary*` fields, ADR 0002), the
/// rail the sidebar with its active item inverted to ink, signal orange the warning and
/// rejection red the danger colour. Hover and active shades of report blue are its 88% and 80%
/// opacity steps (`DESIGN.md`, Buttons).
pub fn apply(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    let p = palette(mode);
    Theme::update(cx, |theme| {
        let c = |token: Oklch| token.to_hsla();
        let blue = c(p.report_blue);
        let blue_hover = blue.opacity(0.88);
        let blue_active = blue.opacity(0.8);
        let ink = c(p.report_blue_ink);

        theme.background = c(p.report_paper);
        theme.foreground = c(p.ink_navy);
        theme.muted = c(p.wash);
        theme.muted_foreground = c(p.ink_muted);
        theme.border = c(p.hairline);
        theme.input = c(p.field_stroke);
        theme.ring = blue;
        theme.caret = blue;
        theme.selection = c(p.coverage_blue).opacity(0.5);
        theme.link = blue;
        theme.link_hover = blue_hover;
        theme.link_active = blue_active;
        theme.accent = c(p.wash);
        theme.accent_foreground = c(p.ink_navy);
        theme.popover = c(p.sheet);
        theme.popover_foreground = c(p.ink_navy);

        theme.primary = blue;
        theme.primary_hover = blue_hover;
        theme.primary_active = blue_active;
        theme.primary_foreground = ink;
        theme.button_primary = blue;
        theme.button_primary_hover = blue_hover;
        theme.button_primary_active = blue_active;
        theme.button_primary_foreground = ink;
        theme.secondary = c(p.sheet);
        theme.secondary_hover = c(p.wash);
        theme.secondary_active = c(p.gap_grey);
        theme.secondary_foreground = c(p.ink_navy);

        theme.danger = c(p.rejection_red);
        theme.danger_foreground = c(p.report_blue_ink);
        theme.warning = c(p.signal_orange);
        theme.success = blue;

        theme.sidebar = c(p.rail);
        theme.sidebar_foreground = c(p.ink_navy);
        theme.sidebar_border = c(p.hairline);
        theme.sidebar_accent = c(p.rail_hover);
        theme.sidebar_accent_foreground = c(p.ink_navy);
        theme.sidebar_primary = c(p.ink_navy);
        theme.sidebar_primary_foreground = c(p.report_paper);

        theme.title_bar = c(p.masthead_ink);
        theme.title_bar_border = c(p.masthead_ink);
        theme.status_bar = c(p.rail);
        theme.status_bar_border = c(p.hairline);

        theme.table = c(p.sheet);
        theme.table_head = c(p.wash);
        theme.table_head_foreground = c(p.ink_muted);
        theme.table_even = c(p.report_paper);
        theme.table_hover = c(p.wash);
        theme.table_row_border = c(p.hairline);
        theme.colors.list = c(p.sheet);
        theme.list_hover = c(p.wash);

        theme.radius = px(radius::NONE);
        theme.radius_lg = px(radius::HAIRLINE);
        theme.shadow = false;
        theme.font_size = px(text::BODY);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "channels are 0..=1 times 255"
    )]
    fn hex(c: Oklch) -> u32 {
        let rgba = c.to_rgba();
        let byte = |v: f32| (v * 255.0).round() as u32;
        (byte(rgba.r) << 16) | (byte(rgba.g) << 8) | byte(rgba.b)
    }

    #[test]
    fn oklch_converts_to_the_srgb_the_web_app_renders() {
        // Hex values a browser renders for these DESIGN.md tokens; the F3 spike used the same.
        assert_eq!(hex(LIGHT.report_blue), 0x0021_5da5);
        assert_eq!(hex(LIGHT.report_paper), 0x00f5_f6f3);
        assert_eq!(hex(LIGHT.ink_navy), 0x0012_202f);
        assert_eq!(hex(LIGHT.ink_muted), 0x0049_5766);
        assert_eq!(hex(LIGHT.hairline), 0x00ca_d2da);
        assert_eq!(hex(LIGHT.field_stroke), 0x00b5_bfc9);
        assert_eq!(hex(LIGHT.sheet), 0x00fc_fdfb);
        assert_eq!(hex(LIGHT.report_blue_ink), 0x00fa_fcfe);
    }

    #[test]
    fn white_black_and_alpha() {
        assert_eq!(hex(oklch(1.0, 0.0, 0.0)), 0x00ff_ffff);
        assert_eq!(hex(oklch(0.0, 0.0, 0.0)), 0);
        let hairline = DARK.hairline.to_rgba();
        assert!((hairline.a - 0.13).abs() < 1e-6);
        assert_eq!(hex(DARK.hairline), 0x00ff_ffff);
    }

    #[test]
    fn channels_stay_in_gamut() {
        for c in [oklch(0.7, 0.4, 145.0), oklch(0.3, 0.4, 300.0)] {
            let rgba = c.to_rgba();
            for v in [rgba.r, rgba.g, rgba.b] {
                assert!((0.0..=1.0).contains(&v), "{v} out of range for {c:?}");
            }
        }
    }

    #[test]
    fn dark_mode_inverts_paper_and_ink() {
        let light = palette(ThemeMode::Light);
        let dark = palette(ThemeMode::Dark);
        assert!(light.report_paper.l > light.ink_navy.l);
        assert!(dark.report_paper.l < dark.ink_navy.l);
        assert_eq!(*light, LIGHT);
        assert_eq!(*dark, DARK);
    }
}
