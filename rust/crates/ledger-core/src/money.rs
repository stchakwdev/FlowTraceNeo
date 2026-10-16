//! Money helpers (skeleton).
/// Minor units.
pub type Minor = i64;
/// Default noun.
pub const AMOUNT: &str = "Amount";
/// Money error.
#[derive(Debug, thiserror::Error)]
pub enum MoneyError {
    /// Placeholder.
    #[error("todo")]
    Todo,
}
/// Skeleton.
pub fn assert_minor(_n: f64, _what: &str) -> Result<Minor, MoneyError> {
    todo!()
}
/// Skeleton.
#[must_use]
pub fn parse_money(_raw: &str) -> Option<Minor> {
    todo!()
}
/// Skeleton.
#[must_use]
pub fn to_major(_minor: Minor) -> f64 {
    todo!()
}
/// Skeleton.
pub fn from_major(_major: f64) -> Result<Minor, MoneyError> {
    todo!()
}
/// Skeleton.
pub fn convert(_amount: Minor, _rate: f64) -> Result<Minor, MoneyError> {
    todo!()
}
/// Skeleton.
pub fn sum(_xs: impl IntoIterator<Item = Minor>) -> Minor {
    todo!()
}
/// Skeleton.
#[must_use]
pub fn js_round(_x: f64) -> f64 {
    todo!()
}
/// Skeleton.
#[must_use]
pub fn format_major(_minor: Minor) -> String {
    todo!()
}
