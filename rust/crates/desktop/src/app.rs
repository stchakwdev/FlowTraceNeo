//! Starting the app: gpui-kit init, the theme, the store over the backend the config chooses
//! (`config.rs`: the SpacetimeDB server named by the flags or `config.toml`, else the in-process
//! demo books) and the window (<https://gpui-kit.com/docs/getting-started>; `open_window` wraps the view in a
//! `Root` itself, ADR 0002).

use gpui_kit::component::ThemeMode;
use gpui_kit::{
    AppContext as _, Bounds, TitlebarOptions, WindowAppearance, WindowBounds, WindowOptions, px,
    size,
};

use std::path::PathBuf;
use std::time::Instant;

use crate::backend::{ConnectSettings, InProcessBackend, LedgerBackend, SpacetimeBackend};
use crate::config::{self, CONFIG_FILE, Config};
use crate::model::{ConnectForm, Navigation};
use crate::panels::general_ledger::GlLaunch;
use crate::panels::grants::GrantsView;
use crate::shell::Shell;
use crate::statements::ReportKind;
use crate::store::{Connector, LedgerStore};
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
    /// What the Grants entry shows first: `list`, `budgets` or a grant id.
    pub grants: Option<String>,
    /// `--server`: the SpacetimeDB server, over the config file's.
    pub server: Option<String>,
    /// `--database`: the database on it, over the config file's.
    pub database: Option<String>,
    /// `--timings on`: write start-up and round-trip times to stderr.
    pub timings: bool,
}

/// Opens the window and runs until it closes.
///
/// # Errors
///
/// When the in-process books cannot be built.
pub fn run(launch: Launch) -> anyhow::Result<()> {
    let started = Instant::now();
    let dir = config::config_dir().unwrap_or_else(|| PathBuf::from(config::APP_DIR));
    let config_path = dir.join(CONFIG_FILE);
    let config = Config::load(&config_path).unwrap_or_else(|err| {
        eprintln!("desktop: {err}; starting without the config file");
        Config::default()
    });
    let settings = config.settings(launch.server.as_deref(), launch.database.as_deref(), &dir);
    let backend = open_backend(settings.as_ref(), launch.large_books)?;
    let form = settings.map_or_else(ConnectForm::default, |s| ConnectForm {
        host: s.host,
        database: s.database,
        token_path: s.token_path,
    });
    let proofs_url = config.proofs_url.clone();
    let connector = connector(config, config_path, dir);
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

            let store = cx.new(|cx| {
                let mut store = LedgerStore::new(backend, connector, cx);
                store.set_proofs_url(proofs_url);
                store
            });
            if launch.timings {
                store.update(cx, |store, _| store.log_timings(started));
            }
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
                let shell = cx.new(|cx| Shell::new(store, launch.nav, &form, window, cx));
                if let Some(kind) = launch.report {
                    shell.update(cx, |shell, cx| shell.show_report(kind, window, cx));
                }
                if let Some(value) = &launch.grants {
                    let grants = shell.read(cx).grants().clone();
                    grants.update(cx, |panel, cx| {
                        panel.show(GrantsView::from_launch(value), window, cx);
                    });
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

/// The backend for `settings`: the server it names, or the in-process demo books (with
/// `large_books` synthetic lines first) when it names none. A connection that cannot even start
/// still opens the app, offline, so the Connection screen can fix the settings.
fn open_backend(
    settings: Option<&ConnectSettings>,
    large_books: Option<usize>,
) -> anyhow::Result<Box<dyn LedgerBackend>> {
    if let Some(settings) = settings {
        eprintln!(
            "desktop: opening {} on {}",
            settings.database, settings.host
        );
        let backend = SpacetimeBackend::connected(settings.clone()).unwrap_or_else(|err| {
            eprintln!("desktop: could not start the connection: {err}");
            SpacetimeBackend::new()
        });
        return Ok(Box::new(backend));
    }
    let mut backend = InProcessBackend::with_demo_and_empty()?;
    if let Some(lines) = large_books {
        let started = Instant::now();
        backend = backend.with_large_books(lines)?;
        eprintln!(
            "desktop: large books with {lines} lines built in {:.1} s",
            started.elapsed().as_secs_f64()
        );
    }
    Ok(Box::new(backend))
}

/// What a connect from the Connection screen does: fill in the default token file in `dir`,
/// remember the server and database in `config.toml`, then open a [`SpacetimeBackend`].
fn connector(mut config: Config, path: PathBuf, dir: PathBuf) -> Connector {
    Box::new(move |settings| {
        let settings = config::with_token(settings, &dir);
        config = config.remember(&settings);
        if let Err(err) = config.save(&path) {
            eprintln!("desktop: the server is not remembered: {err}");
        }
        Ok(Box::new(SpacetimeBackend::connected(settings)?))
    })
}
