//! Rate limiting middleware using token bucket algorithm

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use governor::{
    clock::QuantaClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter using governor crate
pub type GlobalRateLimiter = RateLimiter<NotKeyed, InMemoryState, QuantaClock>;

/// Create a new rate limiter
pub fn create_rate_limiter(requests_per_second: u32, burst: u32) -> Arc<GlobalRateLimiter> {
    let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
        .allow_burst(NonZeroU32::new(burst).unwrap());
    
    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    limiter: Arc<GlobalRateLimiter>,
) -> Result<Response, StatusCode> {
    match limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => {
            tracing::warn!("Rate limit exceeded");
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limiter_creation() {
        let limiter = create_rate_limiter(100, 200);
        assert!(limiter.check().is_ok());
    }
}
