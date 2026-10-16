//! Cooperative cancellation for local jobs, including an atomic publication boundary.
use crate::error::{Error, ErrorCode, Result};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct State {
    cancelled: bool,
    committed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<Mutex<State>>);

impl CancellationToken {
    /// False means publication already won the race; callers must retain its result.
    pub fn cancel(&self) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|poison| poison.into_inner());
        if state.committed {
            return false;
        }
        state.cancelled = true;
        true
    }

    pub fn check(&self) -> Result<()> {
        let state = self.0.lock().unwrap_or_else(|poison| poison.into_inner());
        if state.cancelled {
            Err(cancelled())
        } else {
            Ok(())
        }
    }

    pub(crate) fn publish<T>(&self, action: impl FnOnce() -> Result<T>) -> Result<T> {
        let mut state = self.0.lock().unwrap_or_else(|poison| poison.into_inner());
        if state.cancelled {
            return Err(cancelled());
        }
        let result = action();
        if result.is_ok() {
            state.committed = true;
        }
        result
    }
}

fn cancelled() -> Error {
    Error::new(
        ErrorCode::ExecutionCancelled,
        "Local processing was cancelled before an output was saved.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_prevents_publication() {
        let token = CancellationToken::default();
        assert!(token.cancel());
        assert_eq!(
            token
                .publish(|| -> Result<()> { panic!("must not publish") })
                .unwrap_err()
                .code,
            ErrorCode::ExecutionCancelled
        );
    }
    #[test]
    fn committed_output_wins_late_cancellation() {
        let token = CancellationToken::default();
        token.publish(|| Ok(())).unwrap();
        assert!(!token.cancel());
        token.check().unwrap();
    }

    #[test]
    fn concurrent_publication_and_cancel_have_one_truthful_winner() {
        for _ in 0..100 {
            let token = CancellationToken::default();
            let worker = token.clone();
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let start = barrier.clone();
            let handle = std::thread::spawn(move || {
                start.wait();
                worker.publish(|| Ok(())).is_ok()
            });
            barrier.wait();
            let cancelled = token.cancel();
            let published = handle.join().unwrap();
            assert_ne!(cancelled, published, "exactly one result must win");
        }
    }
}
