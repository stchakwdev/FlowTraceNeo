//! Env (skeleton).
/// Timestamp.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Timestamp(String);
impl Timestamp {
    /// Skeleton.
    pub fn parse(_s: &str) -> Result<Self, String> {
        todo!()
    }
    /// Skeleton.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// Env.
pub trait LedgerEnv {
    /// Now.
    fn now(&self) -> Timestamp;
    /// New id.
    fn new_id(&mut self) -> String;
}
/// Fixed envs.
pub mod fixed {
    /// Skeleton.
    pub struct FixedEnv;
    impl super::LedgerEnv for FixedEnv {
        fn now(&self) -> super::Timestamp {
            todo!()
        }
        fn new_id(&mut self) -> String {
            todo!()
        }
    }
    /// Skeleton.
    #[must_use]
    pub fn demo() -> FixedEnv {
        FixedEnv
    }
    /// Skeleton.
    #[must_use]
    pub fn test() -> FixedEnv {
        FixedEnv
    }
    /// Skeleton.
    #[must_use]
    pub fn fixture() -> FixedEnv {
        FixedEnv
    }
}
