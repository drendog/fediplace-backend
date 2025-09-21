#[cfg(feature = "docs")]
use utoipa::ToResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Not Modified"))]
pub struct NotModifiedResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Bad Request"))]
pub struct BadRequestResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Not Acceptable"))]
pub struct NotAcceptableResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(
    description = "Rate limit exceeded",
    headers(
        ("RateLimit-Limit" = u32),
        ("RateLimit-Remaining" = u32),
        ("RateLimit-Reset" = u64),
        ("Retry-After" = u64)
    )
))]
pub struct RateLimitExceededResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Internal Server Error"))]
pub struct InternalServerErrorResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Unauthorized"))]
pub struct UnauthorizedResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Forbidden"))]
pub struct ForbiddenResponse;

#[allow(dead_code)]
#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(description = "Validation Error"))]
pub struct ValidationErrorResponse;
