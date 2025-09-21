use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait TaskSpawnPort: Send + Sync {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);
}

pub type DynTaskSpawnPort = Arc<dyn TaskSpawnPort>;
