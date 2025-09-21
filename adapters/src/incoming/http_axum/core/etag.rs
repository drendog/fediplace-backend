use axum_extra::headers::ETag;
use std::str::FromStr;

pub fn from_version(version: u64) -> String {
    format!("\"{version}\"")
}

pub fn parse(etag_str: &str) -> Option<ETag> {
    ETag::from_str(etag_str).ok()
}
