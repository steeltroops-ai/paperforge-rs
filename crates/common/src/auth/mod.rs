//! Authentication and authorization utilities
//!
//! Provides:
//! - API key validation
//! - JWT token generation and validation
//! - Tenant context extraction

use crate::errors::{AppError, Result};
use axum::{
    extract::{FromRequestParts, Request},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Extracted authentication context available to handlers
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Tenant ID
    pub tenant_id: Uuid,
    
    /// API key (if authenticated via API key)
    pub api_key: Option<String>,
    
    /// User ID (if authenticated via JWT)
    pub user_id: Option<Uuid>,
    
    /// Scopes/permissions
    pub scopes: Vec<String>,
    
    /// Request ID for tracing
    pub request_id: String,
}

impl AuthContext {
    /// Check if the context has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string()) || self.scopes.contains(&"admin".to_string())
    }
    
    /// Require a specific scope, returning error if not present
    pub fn require_scope(&self, scope: &str) -> Result<()> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(AppError::Forbidden {
                message: format!("Missing required scope: {}", scope),
            })
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    
    /// Tenant ID
    pub tenant_id: String,
    
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    
    /// Issued at (Unix timestamp)
    pub iat: i64,
    
    /// Scopes
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// JWT token manager
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiration_secs: i64,
}

impl JwtManager {
    /// Create a new JWT manager with the given secret
    pub fn new(secret: &str, expiration_secs: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiration_secs: expiration_secs as i64,
        }
    }
    
    /// Generate a new JWT token
    pub fn generate_token(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        scopes: Vec<String>,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.expiration_secs);
        
        let claims = JwtClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            scopes,
        };
        
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal { 
                message: format!("Failed to generate token: {}", e) 
            })
    }
    
    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims> {
        decode::<JwtClaims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AppError::ExpiredToken
                    }
                    _ => AppError::InvalidApiKey,
                }
            })
    }
}

/// Hash an API key for storage
pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate an API key against a stored hash
pub fn validate_api_key(api_key: &str, stored_hash: &str) -> bool {
    hash_api_key(api_key) == stored_hash
}

/// Generate a new API key
pub fn generate_api_key() -> String {
    let random_bytes: [u8; 32] = rand::random();
    format!("pk_{}", hex::encode(random_bytes))
}

/// Generate an idempotency key from content
pub fn generate_idempotency_key(title: &str, abstract_text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\x00");
    hasher.update(abstract_text.as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract API key from Authorization header
pub fn extract_api_key(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// Axum extractor for AuthContext
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;
    
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
        // Extract request ID
        let request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Extract tenant ID
        let tenant_id = parts
            .headers
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| AppError::Unauthorized {
                message: "Missing or invalid X-Tenant-ID header".to_string(),
            })?;
        
        // Extract API key or JWT
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized {
                message: "Missing Authorization header".to_string(),
            })?;
        
        let api_key = extract_api_key(auth_header)
            .map(String::from);
        
        // For now, accept any API key starting with "pk_"
        // In production, this would validate against the database
        if let Some(ref key) = api_key {
            if !key.starts_with("pk_") {
                return Err(AppError::InvalidApiKey);
            }
        }
        
        Ok(AuthContext {
            tenant_id,
            api_key,
            user_id: None,
            scopes: vec!["read".to_string(), "write".to_string()],
            request_id,
        })
    }
}

/// Middleware for API key authentication
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> std::result::Result<Response, AppError> {
    // Check for Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());
    
    if auth_header.is_none() {
        return Err(AppError::Unauthorized {
            message: "Missing Authorization header".to_string(),
        });
    }
    
    // Check for Tenant ID header
    let tenant_header = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok());
    
    if tenant_header.is_none() {
        return Err(AppError::Unauthorized {
            message: "Missing X-Tenant-ID header".to_string(),
        });
    }
    
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_api_key() {
        let key = "pk_test_12345";
        let hash = hash_api_key(key);
        assert!(validate_api_key(key, &hash));
        assert!(!validate_api_key("wrong_key", &hash));
    }
    
    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("pk_"));
        assert!(key.len() > 10);
    }
    
    #[test]
    fn test_idempotency_key() {
        let key1 = generate_idempotency_key("Title A", "Abstract A");
        let key2 = generate_idempotency_key("Title A", "Abstract A");
        let key3 = generate_idempotency_key("Title B", "Abstract A");
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
    
    #[test]
    fn test_extract_api_key() {
        assert_eq!(extract_api_key("Bearer pk_123"), Some("pk_123"));
        assert_eq!(extract_api_key("pk_123"), None);
        assert_eq!(extract_api_key("Basic abc"), None);
    }
    
    #[test]
    fn test_jwt_roundtrip() {
        let manager = JwtManager::new("test_secret", 3600);
        
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let scopes = vec!["read".to_string(), "write".to_string()];
        
        let token = manager.generate_token(user_id, tenant_id, scopes.clone()).unwrap();
        let claims = manager.validate_token(&token).unwrap();
        
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());
        assert_eq!(claims.scopes, scopes);
    }
}
