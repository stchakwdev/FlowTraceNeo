//! Starting the app: gpui-kit init, the theme, the store over the in-process backend and the
//! window (<https://gpui-kit.com/docs/getting-started>; `open_window` wraps the view in a
//! `Root` itself, ADR 0002).

use gpui_kit::component::ThemeMode;
use gpui_kit::{
    AppContext as _, Bounds, TitlebarOptions, WindowAppearance, WindowBounds, WindowOptions, px,
    size,
};

use crate::backend::InProcessBackend;
use crate::model::Navigation;
use crate::panels::general_ledger::GlLaunch;
use crate::shell::Shell;
use crate::statements::ReportKind;
use crate::store::LedgerStore;
use crate::theme;

/// The window title.
pub const TITLE: &str = "FlowTrace Books";

/// The Wayland app id and X11 class, which `hyprctl clients -j` reports as `class`.
pub const APP_ID: &str = "flowtrace-books";

/// What the window shows first.
#[derive(Debug, Clone, Default)]
pub struct Launch {
    /// `Some(true)` for dark, `Some(false)` for light, `None` to follow the system.
    pub dark: Option<bool>,
    /// The screen and nav entry to open on.
    pub nav: Navigation,
    /// A report to open the Reports entry on.
    pub report: Option<ReportKind>,
    /// Add synthetic books with this many posted lines, listed and selected first.
    pub large_books: Option<usize>,
    /// What the General ledger shows first.
    pub general_ledger: GlLaunch,
}

/// Opens the window and runs until it closes.
///
/// # Errors
///
/// When the in-process books cannot be built.
pub fn run(launch: Launch) -> anyhow::Result<()> {
    let mut backend = InProcessBackend::with_demo_and_empty()?;
    if let Some(lines) = launch.large_books {
        let started = std::time::Instant::now();
        backend = backend.with_large_books(lines)?;
        eprintln!(
            "desktop: large books with {lines} lines built in {:.1} s",
            started.elapsed().as_secs_f64()
        );
    }
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            let dark = launch.dark.unwrap_or(matches!(
                cx.window_appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ));
            theme::apply(
                if dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                },
                cx,
            );

            let store = cx.new(|cx| LedgerStore::new(Box::new(backend), cx));
            let options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some(TITLE.into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.0), px(780.0)),
                    cx,
                ))),
                app_id: Some(APP_ID.to_owned()),
                ..Default::default()
            };
            let opened = gpui_kit::open_window(options, cx, |window, cx| {
                let shell = cx.new(|cx| Shell::new(store, launch.nav, window, cx));
                if let Some(kind) = launch.report {
                    shell.update(cx, |shell, cx| shell.show_report(kind, window, cx));
                }
                let panel = shell.read(cx).general_ledger().clone();
                panel.update(cx, |panel, cx| {
                    panel.launch(&launch.general_ledger, window, cx);
                });
                shell
            });
            if let Err(err) = opened {
                eprintln!("desktop: could not open a window: {err:#}");
                cx.quit();
            }
        });
    Ok(())
}
