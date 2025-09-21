use serde::{Deserialize, Serialize};
use std::fmt;
#[cfg(feature = "docs")]
use utoipa::ToSchema;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    #[must_use]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    #[must_use]
    pub fn to_rgba_u32(&self) -> u32 {
        (255u32 << 24) | (u32::from(self.b) << 16) | (u32::from(self.g) << 8) | u32::from(self.r)
    }

    #[must_use]
    pub fn transparent_rgba_u32() -> u32 {
        0x0000_0000
    }

    #[must_use]
    pub fn from_rgba_u32(rgba: u32) -> Self {
        Self {
            r: u8::try_from(rgba & 0xFF).unwrap_or(0),
            g: u8::try_from((rgba >> 8) & 0xFF).unwrap_or(0),
            b: u8::try_from((rgba >> 16) & 0xFF).unwrap_or(0),
        }
    }

    #[must_use]
    pub fn fully_transparent() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    #[must_use]
    pub fn transparent() -> Self {
        Self::fully_transparent()
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(
    feature = "docs",
    schema(
        description = "Color palette ID (0-255) that maps to a predefined RGBA color",
        example = 0
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColorId(pub u8);

impl ColorId {
    #[must_use]
    pub fn new(id: u8) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn id(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for ColorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u8> for ColorId {
    fn from(id: u8) -> Self {
        Self(id)
    }
}

impl From<ColorId> for u8 {
    fn from(color_id: ColorId) -> Self {
        color_id.0
    }
}

pub type Color = RgbColor;

#[inline]
#[must_use]
pub fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (u32::from(a) << 24) | (u32::from(b) << 16) | (u32::from(g) << 8) | u32::from(r)
}
