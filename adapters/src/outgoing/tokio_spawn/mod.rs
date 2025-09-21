pub mod webp_timeout_tokio;

use std::future::Future;
use std::pin::Pin;

use fedi_wplace_application::ports::outgoing::task_spawn::TaskSpawnPort;

pub struct TokioTaskSpawnAdapter;

impl TokioTaskSpawnAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokioTaskSpawnAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskSpawnPort for TokioTaskSpawnAdapter {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        tokio::spawn(future);
    }
}
