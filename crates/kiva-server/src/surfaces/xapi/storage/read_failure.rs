//! Test-only one-shot failure at the real repository read boundary.

use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::surfaces::xapi::outcome::StatementStoreError;

#[derive(Debug, Clone, Default)]
pub(crate) struct ReadFailure(Arc<Mutex<(Option<Uuid>, Vec<Uuid>)>>);

impl ReadFailure {
    pub(crate) fn arm(&self) -> Uuid {
        let mut state = self.0.lock().expect("read failure fixture");
        assert!(state.0.is_none(), "an unconsumed failure is already armed");
        let id = Uuid::new_v4();
        state.0 = Some(id);
        id
    }

    pub(crate) fn consumed(&self) -> Vec<Uuid> {
        self.0.lock().expect("read failure fixture").1.clone()
    }

    pub(crate) fn check(&self) -> Result<(), StatementStoreError> {
        let mut state = self.0.lock().expect("read failure fixture");
        if let Some(id) = state.0.take() {
            state.1.push(id);
            drop(state);
            return Err(sqlx::Error::Io(std::io::Error::other(format!(
                "injected repository read failure {id}"
            )))
            .into());
        }
        Ok(())
    }
}
