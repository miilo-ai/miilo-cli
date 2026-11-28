//! Outbound persistence boundary. Transactions commit only successful use cases.
use crate::domain::State;
pub trait Repository {
    fn transaction<T>(
        &self,
        operation: impl FnOnce(&mut State) -> Result<T, String>,
    ) -> Result<T, String>;
}
