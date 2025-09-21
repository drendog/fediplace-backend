use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub struct BlockingTaskError {
    pub message: String,
}

pub trait WebPEncodingPort: Send + Sync {
    fn encode_webp_lossless(
        &self,
        rgba_pixels: Vec<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, BlockingTaskError>> + Send + 'static>>;
}

pub type DynWebPEncodingPort = Arc<dyn WebPEncodingPort>;
