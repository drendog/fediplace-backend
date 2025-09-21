use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

use crate::error::{DomainError, DomainResult};

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Tile coordinate pair for addressing tiles in the infinite canvas",
    example = json!({"x": 0, "y": 0})
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    #[cfg_attr(feature = "docs", schema(example = 0))]
    pub x: i32,
    #[cfg_attr(feature = "docs", schema(example = 0))]
    pub y: i32,
}

impl TileCoord {
    #[must_use]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn validate(&self) -> DomainResult<()> {
        Ok(())
    }

    pub fn validate_bounds(&self) -> DomainResult<()> {
        self.validate()
    }
}

impl fmt::Display for TileCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.x, self.y)
    }
}

impl FromStr for TileCoord {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(DomainError::InvalidCoordinates(format!(
                "Expected format 'x/y', got '{s}'"
            )));
        }

        #[allow(clippy::indexing_slicing)] // safe because we checked parts.len() == 2
        let x = parts[0].parse::<i32>().map_err(|e| {
            DomainError::InvalidCoordinates(format!("Invalid x coordinate '{}': {e}", parts[0]))
        })?;

        #[allow(clippy::indexing_slicing)] // safe because we checked parts.len() == 2
        let y = parts[1].parse::<i32>().map_err(|e| {
            DomainError::InvalidCoordinates(format!("Invalid y coordinate '{}': {e}", parts[1]))
        })?;

        Ok(TileCoord::new(x, y))
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Pixel coordinate within a tile. Range depends on configured tile size (e.g., 0-511 for 512x512 tiles, 0-255 for 256x256 tiles)",
    example = json!({"x": 256, "y": 128})
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PixelCoord {
    #[cfg_attr(feature = "docs", schema(example = 256, minimum = 0))]
    pub x: usize,
    #[cfg_attr(feature = "docs", schema(example = 128, minimum = 0))]
    pub y: usize,
}

impl PixelCoord {
    #[must_use]
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn validate(&self, tile_size: usize) -> DomainResult<()> {
        if self.x >= tile_size || self.y >= tile_size {
            return Err(DomainError::InvalidPixelCoordinates(format!(
                "Pixel coordinates ({}, {}) exceed tile size {tile_size}",
                self.x, self.y
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn to_index(&self, tile_size: usize) -> usize {
        self.y * tile_size + self.x
    }

    #[must_use]
    pub fn snap_to_grid(&self, pixel_size: usize) -> Self {
        let new_x = (self.x / pixel_size) * pixel_size;
        let new_y = (self.y / pixel_size) * pixel_size;
        Self { x: new_x, y: new_y }
    }

    pub fn validate_bounds(&self, tile_size: usize) -> DomainResult<()> {
        self.validate(tile_size)
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Global pixel coordinate in the infinite canvas. Can be converted to tile coordinates and relative pixel coordinates.",
    example = json!({"x": 1024, "y": 768})
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalCoord {
    #[cfg_attr(feature = "docs", schema(example = 1024))]
    pub x: i32,
    #[cfg_attr(feature = "docs", schema(example = 768))]
    pub y: i32,
}

impl GlobalCoord {
    #[must_use]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn to_tile_coord(&self, tile_size: usize) -> TileCoord {
        let tile_size = tile_size as i32;
        TileCoord::new(self.x.div_euclid(tile_size), self.y.div_euclid(tile_size))
    }

    #[must_use]
    pub fn to_pixel_coord(&self, tile_size: usize) -> PixelCoord {
        let tile_size = tile_size as i32;
        PixelCoord::new(
            self.x.rem_euclid(tile_size) as usize,
            self.y.rem_euclid(tile_size) as usize,
        )
    }

    #[must_use]
    pub fn from_tile_and_pixel(
        tile_coord: TileCoord,
        pixel_coord: PixelCoord,
        tile_size: usize,
    ) -> Self {
        let tile_size = tile_size as i32;
        Self::new(
            tile_coord.x * tile_size + pixel_coord.x as i32,
            tile_coord.y * tile_size + pixel_coord.y as i32,
        )
    }

    pub fn validate(&self) -> DomainResult<()> {
        Ok(())
    }
}

impl fmt::Display for GlobalCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
