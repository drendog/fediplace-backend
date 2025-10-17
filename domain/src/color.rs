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
    pub fn from_rgba_u32(rgba: u32) -> Self {
        Self {
            r: u8::try_from(rgba & 0xFF).unwrap_or(0),
            g: u8::try_from((rgba >> 8) & 0xFF).unwrap_or(0),
            b: u8::try_from((rgba >> 16) & 0xFF).unwrap_or(0),
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexColor(pub String);

impl HexColor {
    #[must_use]
    pub fn new(hex: String) -> Self {
        Self(hex)
    }

    #[must_use]
    pub fn to_rgba_u32(&self) -> Option<u32> {
        if !self.0.starts_with('#') || self.0.len() != 9 {
            return None;
        }

        let hex_digits = &self.0[1..];
        u32::from_str_radix(hex_digits, 16).ok().map(|rgba| {
            let r = (rgba >> 24) & 0xFF;
            let g = (rgba >> 16) & 0xFF;
            let b = (rgba >> 8) & 0xFF;
            let a = rgba & 0xFF;
            (a << 24) | (b << 16) | (g << 8) | r
        })
    }

    #[must_use]
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a))
    }

    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.0.ends_with("00")
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(
    feature = "docs",
    schema(
        description = "Color palette index (-1 for transparent/None, 0-255 for palette colors)",
        example = 0
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColorId(pub i16);

impl ColorId {
    pub const TRANSPARENT: i16 = -1;

    #[must_use]
    pub fn transparent() -> Self {
        Self(Self::TRANSPARENT)
    }

    #[must_use]
    pub fn new(id: i16) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn id(&self) -> i16 {
        self.0
    }

    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.0 == Self::TRANSPARENT
    }
}

impl fmt::Display for ColorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i16> for ColorId {
    fn from(id: i16) -> Self {
        Self(id)
    }
}

impl From<ColorId> for i16 {
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
