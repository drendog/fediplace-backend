use crate::error::AppResult;
use std::sync::Arc;

pub trait ImageCodecPort: Send + Sync {
    fn encode_lossless(&self, rgba_pixels: &[u32]) -> AppResult<Vec<u8>>;
    fn decode_to_rgba(&self, webp_data: &[u8]) -> AppResult<Vec<u32>>;
}

pub type DynImageCodecPort = Arc<dyn ImageCodecPort>;
