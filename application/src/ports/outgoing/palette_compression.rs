use crate::error::AppResult;
use std::sync::Arc;

pub trait PaletteCompressionPort: Send + Sync {
    fn compress(&self, palette_data: &[u8]) -> AppResult<Vec<u8>>;
    fn decompress(&self, compressed_data: &[u8]) -> AppResult<Vec<u8>>;
}

pub type DynPaletteCompressionPort = Arc<dyn PaletteCompressionPort>;
