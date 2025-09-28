use domain::{
    color::ColorId,
    coords::{PixelCoord, TileCoord},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(feature = "docs")]
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to paint a pixel at specific coordinates with a color ID from the configured palette. The px and py coordinates must be within the configured tile size (e.g., 0 to 511 for 512x512 tiles, 0 to 255 for 256x256 tiles).",
    example = json!({
        "x": 0,
        "y": 0,
        "px": 128,
        "py": 256,
        "color_id": 2
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PaintRequest {
    #[cfg_attr(feature = "docs", schema(example = 0))]
    pub x: i32,
    #[cfg_attr(feature = "docs", schema(example = 0))]
    pub y: i32,
    #[cfg_attr(feature = "docs", schema(example = 128, minimum = 0))]
    pub px: usize,
    #[cfg_attr(feature = "docs", schema(example = 256, minimum = 0))]
    pub py: usize,
    #[cfg_attr(feature = "docs", schema(example = 2))]
    pub color_id: u8,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Pixel paint operation within a batch request.",
    example = json!({
        "px": 128,
        "py": 256,
        "color_id": 2
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPixelPaint {
    #[cfg_attr(feature = "docs", schema(example = 128, minimum = 0))]
    pub px: usize,
    #[cfg_attr(feature = "docs", schema(example = 256, minimum = 0))]
    pub py: usize,
    #[cfg_attr(feature = "docs", schema(example = 2))]
    pub color_id: u8,
}

impl BatchPixelPaint {
    #[must_use]
    pub fn pixel_coord(&self) -> PixelCoord {
        PixelCoord::new(self.px, self.py)
    }

    #[must_use]
    pub fn color_id(&self) -> ColorId {
        ColorId::new(self.color_id)
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to paint multiple pixels within a single tile. All pixels will be painted atomically with a single version increment.",
    example = json!({
        "pixels": [
            {"px": 128, "py": 256, "color_id": 2},
            {"px": 129, "py": 256, "color_id": 3},
            {"px": 130, "py": 256, "color_id": 1}
        ]
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchPaintPixelsRequest {
    #[validate(
        length(
            min = 1,
            max = 1000,
            message = "Must paint between 1 and 1000 pixels per batch"
        ),
        custom(
            function = "validate_no_duplicate_pixels",
            message = "Duplicate pixel coordinates are not allowed"
        )
    )]
    pub pixels: Vec<BatchPixelPaint>,
}

fn validate_no_duplicate_pixels(pixels: &[BatchPixelPaint]) -> Result<(), ValidationError> {
    let mut seen_coords = HashSet::new();
    for pixel in pixels {
        let coord = (pixel.px, pixel.py);
        if !seen_coords.insert(coord) {
            return Err(ValidationError::new("duplicate_pixels"));
        }
    }
    Ok(())
}

impl PaintRequest {
    #[must_use]
    pub fn tile_coord(&self) -> TileCoord {
        TileCoord::new(self.x, self.y)
    }

    #[must_use]
    pub fn pixel_coord(&self) -> PixelCoord {
        PixelCoord::new(self.px, self.py)
    }

    #[must_use]
    pub fn color_id(&self) -> ColorId {
        ColorId::new(self.color_id)
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to register a new user account with email and password. Password strength is evaluated using zxcvbn with a minimum score of 3/4 (strong). zxcvbn evaluates passwords based on real-world attack patterns and provides helpful feedback for weak passwords.",
    example = json!({
        "email": "user@example.com",
        "username": "johndoe",
        "password": "MyVerySecure!Password123"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[cfg_attr(feature = "docs", schema(example = "user@example.com"))]
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[cfg_attr(feature = "docs", schema(example = "johndoe"))]
    #[validate(length(
        min = 4,
        max = 32,
        message = "Username must be between 4 and 32 characters"
    ))]
    pub username: String,

    #[cfg_attr(feature = "docs", schema(example = "MyVerySecure!Password123"))]
    #[validate(length(min = 1, message = "Password cannot be empty"))]
    pub password: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to login with email and password",
    example = json!({
        "email": "user@example.com",
        "password": "MyVerySecure!Password123"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[cfg_attr(feature = "docs", schema(example = "user@example.com"))]
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[cfg_attr(feature = "docs", schema(example = "secure_password"))]
    #[validate(length(min = 1, message = "Password cannot be empty"))]
    pub password: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to update username",
    example = json!({
        "username": "new_username"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUsernameRequest {
    #[cfg_attr(feature = "docs", schema(example = "new_username"))]
    #[validate(length(
        min = 4,
        max = 32,
        message = "Username must be between 4 and 32 characters"
    ))]
    pub username: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Request to ban a user with reason and optional expiration date",
    example = json!({
        "reason": "Rule violation",
        "expires_at": "2024-12-31T23:59:59Z"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BanUserRequest {
    #[cfg_attr(feature = "docs", schema(example = "Rule violation"))]
    pub reason: String,

    #[cfg_attr(feature = "docs", schema(example = "2024-12-31T23:59:59Z"))]
    pub expires_at: Option<String>,
}
