//! Which books the app opens: the server named by `--server` and `--database`, else the one in
//! `flowtrace-books/config.toml` under the platform config dir (`dirs::config_dir`, so
//! `~/.config/flowtrace-books/config.toml` on Linux), else the in-process demo books.
//!
//! ```toml
//! server = "https://maincloud.spacetimedb.com"
//! database = "flowtrace-ledger-dev"
//! # optional; by default token-<server host> next to this file
//! token_file = "/home/me/.config/flowtrace-books/token-maincloud.spacetimedb.com"
//! ```
//!
//! The token file is a credential: `ledger_client::TokenFile` writes it with mode 0600 and
//! nothing here reads or prints it. Keeping it per server host means a local server and
//! maincloud each keep their own identity.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backend::ConnectSettings;
use crate::model::DEFAULT_DATABASE;

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

    /// The config with `server` and `database` from a connect, keeping a set token file.
    #[must_use]
    pub fn remember(&self, settings: &ConnectSettings) -> Self {
        Self {
            server: Some(settings.host.clone()),
            database: Some(settings.database.clone()),
            token_file: self.token_file.clone(),
        }
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
        };
        let settings = config.settings(None, None, &dir()).unwrap();
        assert_eq!(settings.token_path, "/secrets/ft-token");
        assert_eq!(config.remember(&settings).token_file, config.token_file);
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

    #[test]
    fn the_config_dir_is_named_for_the_app() {
        if let Some(dir) = config_dir() {
            assert!(dir.ends_with(APP_DIR));
        }
    }
}
