//! The connection token, kept in a file the caller names, so the desktop keeps one identity
//! across runs (the TS client keeps it in `localStorage`, `spacetime-backend.ts:58-72`).
//!
//! The token is a credential: it is never logged or put in an error, and [`TokenFile`]'s
//! `Debug` shows only the path. On Unix the file is written with mode 0600.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::error::ClientError;

/// Where the token lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenFile {
    path: PathBuf,
}

impl TokenFile {
    /// The token kept at `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved token, or `None` when there is no file yet or it is empty (a first run, which
    /// connects anonymously and gets a new identity).
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the file exists but cannot be read.
    pub fn load(&self) -> Result<Option<String>, ClientError> {
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                let token = text.trim();
                Ok((!token.is_empty()).then(|| token.to_owned()))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(self.error(source)),
        }
    }

    /// Saves `token`, replacing the file through a sibling temporary file so a crash never
    /// leaves half a token. Creates the parent directory if needed.
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the file cannot be written.
    pub fn save(&self, token: &str) -> Result<(), ClientError> {
        self.write(token).map_err(|source| self.error(source))
    }

    /// Removes the saved token (a sign-out); no file is fine.
    ///
    /// # Errors
    ///
    /// [`ClientError::Token`] when the file exists but cannot be removed.
    pub fn clear(&self) -> Result<(), ClientError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(self.error(source)),
        }
    }

    fn write(&self, token: &str) -> io::Result<()> {
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(token.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, &self.path)
    }

    fn error(&self, source: io::Error) -> ClientError {
        ClientError::Token {
            path: self.path.clone(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        // Cargo's temp dir for the default target dir (`CARGO_TARGET_TMPDIR` is only set for
        // integration tests), so test files stay inside the worktree.
        let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tmp"))
            .join(format!("ledger-client-token-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn no_file_is_no_token() {
        let dir = scratch("none");
        assert_eq!(TokenFile::new(dir.join("token")).load().unwrap(), None);
    }

    #[test]
    fn a_saved_token_loads_back_and_the_directory_is_made() {
        let dir = scratch("roundtrip");
        let file = TokenFile::new(dir.join("nested").join("token"));
        file.save("tok.en-1").unwrap();
        assert_eq!(file.load().unwrap().as_deref(), Some("tok.en-1"));
        file.save("tok.en-2").unwrap();
        assert_eq!(file.load().unwrap().as_deref(), Some("tok.en-2"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clear_removes_the_token_and_a_missing_file_is_fine() {
        let dir = scratch("clear");
        let file = TokenFile::new(dir.join("token"));
        file.clear().unwrap();
        file.save("refresh").unwrap();
        file.clear().unwrap();
        assert_eq!(file.load().unwrap(), None);
        assert!(!file.path().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_empty_file_is_no_token() {
        let dir = scratch("empty");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("token"), "\n").unwrap();
        assert_eq!(TokenFile::new(dir.join("token")).load().unwrap(), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private_to_the_user() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = scratch("mode");
        let file = TokenFile::new(dir.join("token"));
        file.save("secret").unwrap();
        let mode = fs::metadata(file.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn debug_shows_the_path_and_never_a_token() {
        let dir = scratch("debug");
        let file = TokenFile::new(dir.join("token"));
        file.save("do-not-print-me").unwrap();
        assert!(!format!("{file:?}").contains("do-not-print-me"));
        fs::remove_dir_all(dir).unwrap();
    }
}
