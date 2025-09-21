use domain::color::pack_rgba;
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::image_codec::ImageCodecPort,
};
use image::{ImageBuffer, ImageFormat, Rgba};
use std::io::Cursor;
use tracing::{debug, instrument, trace};

#[derive(Copy, Clone)]
pub struct ImageWebpConfig {
    pub tile_size: usize,
}

#[derive(Clone)]
pub struct ImageWebpAdapter {
    tile_size: usize,
}

impl ImageWebpAdapter {
    pub fn new(config: ImageWebpConfig) -> Self {
        Self {
            tile_size: config.tile_size,
        }
    }

    #[instrument(skip(self, rgba_pixels))]
    fn encode_lossless_impl(&self, rgba_pixels: &[u32]) -> AppResult<Vec<u8>> {
        let expected_pixels = self.tile_size * self.tile_size;
        if rgba_pixels.len() != expected_pixels {
            return Err(AppError::CodecError {
                message: format!(
                    "Expected {} pixels, got {}",
                    expected_pixels,
                    rgba_pixels.len()
                ),
            });
        }

        trace!(
            "Encoding WebP, first few pixels: {:?}",
            rgba_pixels.get(0..4.min(rgba_pixels.len())).unwrap_or(&[])
        );

        let mut rgba_bytes = Vec::with_capacity(rgba_pixels.len() * 4);
        for &pixel in rgba_pixels {
            rgba_bytes.extend_from_slice(&pixel.to_le_bytes());
        }

        let img_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
            self.tile_size as u32,
            self.tile_size as u32,
            rgba_bytes,
        )
        .ok_or_else(|| AppError::CodecError {
            message: "Failed to create image buffer from RGBA data".to_string(),
        })?;

        let mut webp_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut webp_bytes);

        img_buffer
            .write_to(&mut cursor, ImageFormat::WebP)
            .map_err(|e| AppError::CodecError {
                message: format!("Failed to encode WebP: {}", e),
            })?;

        debug!("Encoded WebP: {} bytes", webp_bytes.len());

        if webp_bytes.is_empty() {
            return Err(AppError::CodecError {
                message: "WebP encoding produced empty output".to_string(),
            });
        }

        Ok(webp_bytes)
    }

    #[instrument(skip(self, webp_data))]
    fn decode_to_rgba_impl(&self, webp_data: &[u8]) -> AppResult<Vec<u32>> {
        let cursor = Cursor::new(webp_data);
        let reader = image::ImageReader::with_format(cursor, ImageFormat::WebP);

        let img = reader.decode().map_err(|e| AppError::CodecError {
            message: format!("Failed to decode WebP: {}", e),
        })?;

        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        if width as usize != self.tile_size || height as usize != self.tile_size {
            return Err(AppError::CodecError {
                message: format!(
                    "WebP dimensions {}x{} don't match expected tile size {}x{}",
                    width, height, self.tile_size, self.tile_size
                ),
            });
        }

        let rgba_bytes = rgba_img.as_raw();
        let rgba_pixels: Vec<u32> = rgba_bytes
            .chunks_exact(4)
            .map(|chunk| {
                let bytes: [u8; 4] = chunk.try_into().unwrap_or([0, 0, 0, 255]);
                pack_rgba(bytes[0], bytes[1], bytes[2], bytes[3])
            })
            .collect();

        debug!(
            "Decoded WebP: {} bytes -> {} pixels",
            webp_data.len(),
            rgba_pixels.len()
        );
        Ok(rgba_pixels)
    }
}

impl ImageCodecPort for ImageWebpAdapter {
    fn encode_lossless(&self, rgba_pixels: &[u32]) -> AppResult<Vec<u8>> {
        self.encode_lossless_impl(rgba_pixels)
    }
    fn decode_to_rgba(&self, webp_data: &[u8]) -> AppResult<Vec<u32>> {
        self.decode_to_rgba_impl(webp_data)
    }
}
