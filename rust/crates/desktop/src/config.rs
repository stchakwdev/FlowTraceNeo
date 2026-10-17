//! Which books the app opens: the server named by `--server` and `--database`, else the one in
//! `flowtrace-books/config.toml` under the platform config dir (`dirs::config_dir`, so
//! `~/.config/flowtrace-books/config.toml` on Linux), else the in-process demo books.
//!
//! ```toml
//! server = "https://maincloud.spacetimedb.com"
//! database = "flowtrace-ledger-dev"
//! # optional; by default token-<server host> next to this file
//! token_file = "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com"
//! # optional; the proofs Worker for receipts and bill proof (workers/proofs/README.md)
//! proofs_url = "https://proofs.example.org"
//! # optional: sign in with WorkOS AuthKit through the system browser (crate::signin)
//! workos_client_id = "client_01..."
//! # optional; by default https://api.workos.com/user_management/<workos_client_id>
//! workos_issuer = "https://api.workos.com/user_management/client_01..."
//! # optional; by default https://api.workos.com (the local proof points it at a test server)
//! workos_api = "https://api.workos.com"
//! ```
//!
//! The token file is a credential: `ledger_client::TokenFile` writes it with mode 0600 and
//! nothing here reads or prints it. Keeping it per server host means a local server and
//! maincloud each keep their own identity. A WorkOS sign-in keeps its refresh token in the same
//! way, in the token file's path with `.workos` appended ([`crate::signin::refresh_file`]).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backend::ConnectSettings;
use crate::model::DEFAULT_DATABASE;
use crate::signin::WorkosSettings;

/// The app's folder under the platform config dir.
pub const APP_DIR: &str = "flowtrace-books";

/// The config file's name in [`APP_DIR`].
pub const CONFIG_FILE: &str = "config.toml";

/// The keys of `config.toml`; every key is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Server URL, for example `https://maincloud.spacetimedb.com`. Without it the app opens the
    /// in-process demo books.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// Database name on that server; [`DEFAULT_DATABASE`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// Where the auth token lives; [`token_path`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_file: Option<String>,
    /// The proofs Worker (`workers/proofs/README.md`). Without it "Attach receipt" and
    /// "Attach proof" say so instead of uploading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proofs_url: Option<String>,
    /// The WorkOS environment's client id. With it the Connect screen offers "Sign in with
    /// WorkOS"; without it the app connects with its device identity only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workos_client_id: Option<String>,
    /// The `iss` of that environment's access tokens ([`WorkosSettings::issuer`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workos_issuer: Option<String>,
    /// The WorkOS API host ([`WorkosSettings::api`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workos_api: Option<String>,
}

/// Why the config could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file exists but could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The config file.
        path: PathBuf,
        /// What failed.
        source: io::Error,
    },
    /// The file is not valid TOML with the keys above.
    #[error("{path}: {message}")]
    Parse {
        /// The config file.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },
}

/// `flowtrace-books` under the platform config dir; `None` where the platform has none.
#[must_use]
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_DIR))
}

/// The default token file for `server` in `dir`: `token-<host>`, with any character other than
/// letters, digits, `.` and `-` written as `_` (`https://maincloud.spacetimedb.com` gives
/// `token-maincloud.spacetimedb.com`, `http://127.0.0.1:3100` gives `token-127.0.0.1_3100`).
#[must_use]
pub fn token_path(dir: &Path, server: &str) -> PathBuf {
    let host = ["https://", "http://", "wss://", "ws://"]
        .iter()
        .find_map(|scheme| server.strip_prefix(scheme))
        .unwrap_or(server)
        .trim_end_matches('/');
    let slug: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("token-{slug}"))
}

impl Config {
    /// The config at `path`; an absent file is the empty config.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when the file cannot be read or does not parse.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(ConfigError::Io {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        toml::from_str(&text).map_err(|e| ConfigError::Parse {
            path: path.to_owned(),
            message: e.message().to_owned(),
        })
    }

    /// Writes the config to `path`, creating its folder.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] when it cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let io = |source| ConfigError::Io {
            path: path.to_owned(),
            source,
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(io)?;
        }
        let text = toml::to_string(self).map_err(|e| ConfigError::Parse {
            path: path.to_owned(),
            message: e.to_string(),
        })?;
        fs::write(path, text).map_err(io)
    }

    /// The config with `server` and `database` from a connect, keeping every other key.
    #[must_use]
    pub fn remember(&self, settings: &ConnectSettings) -> Self {
        Self {
            server: Some(settings.host.clone()),
            database: Some(settings.database.clone()),
            ..self.clone()
        }
    }

    /// The WorkOS sign-in settings, when a non-blank `workos_client_id` is set.
    #[must_use]
    pub fn workos(&self) -> Option<WorkosSettings> {
        let client_id = self.workos_client_id.as_deref()?.trim();
        (!client_id.is_empty()).then(|| {
            WorkosSettings::new(
                client_id,
                self.workos_issuer.as_deref(),
                self.workos_api.as_deref(),
            )
        })
    }

    /// The server to open, flags first: `None` (the in-process demo) when neither the flags nor
    /// the config name a server. The token file is the config's, else [`token_path`] in `dir`.
    #[must_use]
    pub fn settings(
        &self,
        server: Option<&str>,
        database: Option<&str>,
        dir: &Path,
    ) -> Option<ConnectSettings> {
        let host = server.or(self.server.as_deref())?.trim().to_owned();
        let database = database
            .or(self.database.as_deref())
            .unwrap_or(DEFAULT_DATABASE)
            .trim()
            .to_owned();
        let token_path = self
            .token_file
            .clone()
            .unwrap_or_else(|| token_path(dir, &host).to_string_lossy().into_owned());
        Some(ConnectSettings {
            host,
            database,
            token_path,
            access_token: None,
        })
    }
}

/// `settings` with the default token file in `dir` when it names none (the Connect screen's
/// optional field).
#[must_use]
pub fn with_token(mut settings: ConnectSettings, dir: &Path) -> ConnectSettings {
    if settings.token_path.is_empty() {
        settings.token_path = token_path(dir, &settings.host)
            .to_string_lossy()
            .into_owned();
    }
    settings
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    fn dir() -> PathBuf {
        PathBuf::from("/home/me/.config/flowtrace-books")
    }

    #[test]
    fn no_server_anywhere_opens_the_demo() {
        assert_eq!(Config::default().settings(None, None, &dir()), None);
        assert_eq!(
            Config::default().settings(None, Some("flowtrace-ledger-dev"), &dir()),
            None,
            "a database alone names no server"
        );
    }

    #[test]
    fn flags_win_over_the_config_and_the_token_sits_next_to_it() {
        let config = Config {
            server: Some("http://127.0.0.1:3100".to_owned()),
            database: Some("ft-local".to_owned()),
            token_file: None,
            proofs_url: None,
            ..Config::default()
        };
        let from_config = config.settings(None, None, &dir()).unwrap();
        assert_eq!(from_config.host, "http://127.0.0.1:3100");
        assert_eq!(from_config.database, "ft-local");
        assert_eq!(
            from_config.token_path,
            "/home/me/.config/flowtrace-books/token-127.0.0.1_3100"
        );

        let flagged = config
            .settings(
                Some("https://maincloud.spacetimedb.com"),
                Some("flowtrace-ledger-dev"),
                &dir(),
            )
            .unwrap();
        assert_eq!(flagged.host, "https://maincloud.spacetimedb.com");
        assert_eq!(flagged.database, "flowtrace-ledger-dev");
        assert_eq!(
            flagged.token_path,
            "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com"
        );

        let only_server = Config::default()
            .settings(Some("https://maincloud.spacetimedb.com"), None, &dir())
            .unwrap();
        assert_eq!(only_server.database, DEFAULT_DATABASE);
    }

    #[test]
    fn a_set_token_file_is_kept() {
        let config = Config {
            server: Some("https://maincloud.spacetimedb.com".to_owned()),
            database: None,
            token_file: Some("/secrets/ft-token".to_owned()),
            proofs_url: Some("https://proofs.example.org".to_owned()),
            ..Config::default()
        };
        let settings = config.settings(None, None, &dir()).unwrap();
        assert_eq!(settings.token_path, "/secrets/ft-token");
        assert_eq!(config.remember(&settings).token_file, config.token_file);
        assert_eq!(config.remember(&settings).proofs_url, config.proofs_url);
        assert_eq!(
            with_token(settings.clone(), &dir()).token_path,
            "/secrets/ft-token"
        );
        let blank = ConnectSettings {
            token_path: String::new(),
            ..settings
        };
        assert_eq!(
            with_token(blank, &dir()).token_path,
            "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com"
        );
    }

    #[test]
    fn the_config_file_round_trips_and_refuses_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("ft-u10-config-{}", std::process::id()));
        let path = dir.join(APP_DIR).join(CONFIG_FILE);
        assert_eq!(
            Config::load(&path).unwrap(),
            Config::default(),
            "absent file"
        );

        let config = Config {
            server: Some("https://maincloud.spacetimedb.com".to_owned()),
            database: Some("flowtrace-ledger-dev".to_owned()),
            token_file: None,
            proofs_url: None,
            ..Config::default()
        };
        config.save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "server = \"https://maincloud.spacetimedb.com\"\ndatabase = \"flowtrace-ledger-dev\"\n"
        );
        assert_eq!(Config::load(&path).unwrap(), config);

        fs::write(&path, "server = \"x\"\nport = 3\n").unwrap();
        let err = Config::load(&path).unwrap_err();
        assert!(err.to_string().contains("port"), "{err}");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// `release/config.toml`, which `tools/release.sh` packs into every release archive, opens
    /// the production database through this loader, with the token file in its default place.
    #[test]
    fn the_release_config_opens_the_production_database() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("release")
            .join(CONFIG_FILE);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "server = \"https://maincloud.spacetimedb.com\"\ndatabase = \"flowtrace-ledger\"\n"
        );
        let config = Config::load(&path).unwrap();
        assert_eq!(config.token_file, None);
        let settings = config.settings(None, None, &dir()).unwrap();
        assert_eq!(settings.host, "https://maincloud.spacetimedb.com");
        assert_eq!(settings.database, "flowtrace-ledger");
        assert_eq!(
            settings.token_path,
            "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com"
        );
    }

    #[test]
    fn the_workos_keys_turn_sign_in_on_and_survive_a_connect() {
        assert_eq!(Config::default().workos(), None);
        let blank = Config {
            workos_client_id: Some("  ".to_owned()),
            ..Config::default()
        };
        assert_eq!(blank.workos(), None, "a blank client id is no sign-in");

        let text = "server = \"http://127.0.0.1:3100\"\nworkos_client_id = \"client_01TEST\"\n";
        let config: Config = toml::from_str(text).unwrap();
        let workos = config.workos().unwrap();
        assert_eq!(workos.client_id, "client_01TEST");
        assert_eq!(
            workos.issuer,
            "https://api.workos.com/user_management/client_01TEST"
        );
        assert_eq!(workos.api, "https://api.workos.com");

        let local: Config = toml::from_str(
            "workos_client_id = \"client_01TEST\"\nworkos_issuer = \"http://127.0.0.1:4000/trusted\"\nworkos_api = \"http://127.0.0.1:4000/trusted\"\n",
        )
        .unwrap();
        assert_eq!(
            local.workos().unwrap().issuer,
            "http://127.0.0.1:4000/trusted"
        );
        assert_eq!(local.workos().unwrap().api, "http://127.0.0.1:4000/trusted");

        let settings = config
            .settings(Some("https://maincloud.spacetimedb.com"), None, &dir())
            .unwrap();
        assert_eq!(config.remember(&settings).workos(), config.workos());
    }

    #[test]
    fn the_config_dir_is_named_for_the_app() {
        if let Some(dir) = config_dir() {
            assert!(dir.ends_with(APP_DIR));
        }
    }

    #[test]
    fn the_proofs_worker_is_read_from_the_file() {
        let config: Config = toml::from_str(
            "server = \"https://maincloud.spacetimedb.com\"\nproofs_url = \"https://proofs.example.org\"\n",
        )
        .unwrap();
        assert_eq!(
            config.proofs_url.as_deref(),
            Some("https://proofs.example.org")
        );
    }
}
