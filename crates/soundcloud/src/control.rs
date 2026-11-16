use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// One token per operation. Clones can cancel that operation from another thread.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub struct RequestOptions {
    pub timeout: Duration,
    pub cancellation: CancellationToken,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(45),
            cancellation: CancellationToken::default(),
        }
    }
}
