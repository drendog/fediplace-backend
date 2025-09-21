use axum::extract::Path;
use axum_extra::{TypedHeader, headers::IfNoneMatch};
use fedi_wplace_application::error::AppError;

use crate::incoming::http_axum::error_mapper::HttpError;
use domain::coords::TileCoord;

pub type TilePath = Path<(i32, i32)>;
pub type IfNoneMatchHeader = Option<TypedHeader<IfNoneMatch>>;

pub fn extract_tile_coord(Path((x, y)): TilePath) -> Result<TileCoord, HttpError> {
    let coord = TileCoord::new(x, y);
    coord
        .validate_bounds()
        .map_err(|e| HttpError(AppError::from(e)))?;
    Ok(coord)
}
