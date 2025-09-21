use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct TimeoutError;

pub trait WebPTimeoutPort: Send + Sync {
    fn encode_webp_with_timeout(
        &self,
        rgba_pixels: Vec<u32>,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, TimeoutError>> + Send + 'static>>;
}

pub type DynWebPTimeoutPort = Arc<dyn WebPTimeoutPort>;
