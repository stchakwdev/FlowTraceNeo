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
use crate::shell::Shell;
use crate::store::LedgerStore;
use crate::theme;

/// The window title.
pub const TITLE: &str = "FlowTrace Books";

/// The Wayland app id and X11 class, which `hyprctl clients -j` reports as `class`.
pub const APP_ID: &str = "flowtrace-books";

/// What the window shows first.
#[derive(Debug, Clone, Copy, Default)]
pub struct Launch {
    /// `Some(true)` for dark, `Some(false)` for light, `None` to follow the system.
    pub dark: Option<bool>,
    /// The screen and nav entry to open on.
    pub nav: Navigation,
}

/// Opens the window and runs until it closes.
///
/// # Errors
///
/// When the in-process books cannot be built.
pub fn run(launch: Launch) -> anyhow::Result<()> {
    let backend = InProcessBackend::with_demo_and_empty()?;
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
                cx.new(|cx| Shell::new(store, launch.nav, window, cx))
            });
            if let Err(err) = opened {
                eprintln!("desktop: could not open a window: {err:#}");
                cx.quit();
            }
        });
    Ok(())
}
