//! Errors (skeleton).
/// Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueCode {
    /// Placeholder.
    LineZero,
    /// Placeholder.
    Unbalanced,
    /// Placeholder.
    NotFound,
}
impl IssueCode {
    /// Skeleton.
    pub const ALL: [IssueCode; 0] = [];
    /// Skeleton.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        todo!()
    }
}
/// Issue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Issue {}
impl Issue {
    /// Skeleton.
    #[must_use]
    pub fn new(_code: IssueCode, _message: &str) -> Self {
        todo!()
    }
    /// Skeleton.
    #[must_use]
    pub fn on_line(self, _line: usize, _overridable: bool) -> Self {
        todo!()
    }
}
/// Error.
#[derive(Debug, thiserror::Error)]
#[error("todo")]
pub struct LedgerError;
impl LedgerError {
    /// Skeleton.
    #[must_use]
    pub fn new(_issues: Vec<Issue>) -> Self {
        todo!()
    }
    /// Skeleton.
    #[must_use]
    pub fn fail(_code: IssueCode, _message: &str) -> Self {
        todo!()
    }
    /// Skeleton.
    #[must_use]
    pub fn code(&self) -> IssueCode {
        todo!()
    }
    /// Skeleton.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        todo!()
    }
}
