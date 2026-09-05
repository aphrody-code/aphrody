// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
//
// HTTP authentication helpers. Ports the `UserBuilder` pattern from
// `packages/gemini-cli/packages/a2a-server/src/http/app.ts` — extracts
// a `User` from an `Authorization` header supporting `Bearer` and `Basic`
// schemes. Pluggable validators allow callers to verify tokens or
// credentials against any backend (in-memory map, JWT, OIDC, etc.).
//
// Returned `User` flows through to `middleware::ServiceParams` so
// downstream handlers and executors can branch on authentication.

use std::{collections::HashMap, sync::Arc};

use a2a::A2AError;
use async_trait::async_trait;
use axum::http::{HeaderMap, header};
use base64::Engine as _;

use crate::middleware::User;

/// Asynchronously resolves an `Authorization` header into a `User`.
///
/// Implementations may consult any backend. Return `Ok(None)` for
/// "no credentials supplied" (anonymous). Return `Err` only for
/// hard auth failures that should produce a 401.
#[async_trait]
pub trait UserBuilder: Send + Sync + 'static {
    async fn build(&self, headers: &HeaderMap) -> Result<Option<User>, A2AError>;
}

/// Default user builder that always returns anonymous.
pub struct AnonymousUserBuilder;

#[async_trait]
impl UserBuilder for AnonymousUserBuilder {
    async fn build(&self, _headers: &HeaderMap) -> Result<Option<User>, A2AError> {
        Ok(None)
    }
}

/// Asynchronously validates a Bearer token. Returns the resolved
/// `User` on success, `None` if the token is unknown.
#[async_trait]
pub trait BearerValidator: Send + Sync + 'static {
    async fn validate(&self, token: &str) -> Result<Option<User>, A2AError>;
}

/// Asynchronously validates Basic Auth username/password credentials.
#[async_trait]
pub trait BasicValidator: Send + Sync + 'static {
    async fn validate(&self, username: &str, password: &str) -> Result<Option<User>, A2AError>;
}

/// Static token-to-username map (intended for tests and simple deployments).
pub struct StaticBearerValidator {
    tokens: HashMap<String, String>,
}

impl StaticBearerValidator {
    pub fn new() -> Self {
        StaticBearerValidator { tokens: HashMap::new() }
    }

    pub fn with_token(mut self, token: impl Into<String>, username: impl Into<String>) -> Self {
        self.tokens.insert(token.into(), username.into());
        self
    }
}

impl Default for StaticBearerValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BearerValidator for StaticBearerValidator {
    async fn validate(&self, token: &str) -> Result<Option<User>, A2AError> {
        Ok(self.tokens.get(token).map(|name| User::authenticated(name.clone())))
    }
}

/// Static `username:password` -> `User` map (intended for tests).
pub struct StaticBasicValidator {
    creds: HashMap<(String, String), String>,
}

impl StaticBasicValidator {
    pub fn new() -> Self {
        StaticBasicValidator { creds: HashMap::new() }
    }

    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let u = username.into();
        let p = password.into();
        let name = u.clone();
        self.creds.insert((u, p), name);
        self
    }
}

impl Default for StaticBasicValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BasicValidator for StaticBasicValidator {
    async fn validate(&self, username: &str, password: &str) -> Result<Option<User>, A2AError> {
        Ok(self
            .creds
            .get(&(username.to_string(), password.to_string()))
            .map(|name| User::authenticated(name.clone())))
    }
}

/// Composable user builder that delegates to a Bearer and/or Basic
/// validator. If neither matches, returns `None` (anonymous).
pub struct AuthUserBuilder {
    bearer: Option<Arc<dyn BearerValidator>>,
    basic: Option<Arc<dyn BasicValidator>>,
}

impl AuthUserBuilder {
    pub fn new() -> Self {
        AuthUserBuilder { bearer: None, basic: None }
    }

    pub fn with_bearer(mut self, validator: impl BearerValidator) -> Self {
        self.bearer = Some(Arc::new(validator));
        self
    }

    pub fn with_basic(mut self, validator: impl BasicValidator) -> Self {
        self.basic = Some(Arc::new(validator));
        self
    }
}

impl Default for AuthUserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserBuilder for AuthUserBuilder {
    async fn build(&self, headers: &HeaderMap) -> Result<Option<User>, A2AError> {
        let Some(auth) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
            return Ok(None);
        };

        if let Some(rest) = auth.strip_prefix("Bearer ").or_else(|| auth.strip_prefix("bearer "))
        {
            if let Some(validator) = self.bearer.as_ref() {
                let token = rest.trim();
                return validator.validate(token).await;
            }
            return Ok(None);
        }

        if let Some(rest) = auth.strip_prefix("Basic ").or_else(|| auth.strip_prefix("basic ")) {
            if let Some(validator) = self.basic.as_ref() {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(rest.trim().as_bytes())
                    .map_err(|e| {
                        A2AError::invalid_request(format!("invalid Basic auth payload: {e}"))
                    })?;
                let decoded = std::str::from_utf8(&decoded).map_err(|_| {
                    A2AError::invalid_request("Basic auth payload not UTF-8")
                })?;
                let mut parts = decoded.splitn(2, ':');
                let username = parts.next().unwrap_or("");
                let password = parts.next().unwrap_or("");
                return validator.validate(username, password).await;
            }
            return Ok(None);
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderName, HeaderValue};

    use super::*;

    fn headers_with(auth: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static("authorization"), HeaderValue::from_str(auth).unwrap());
        h
    }

    #[tokio::test]
    async fn test_anonymous_returns_none() {
        let builder = AnonymousUserBuilder;
        let h = HeaderMap::new();
        assert!(builder.build(&h).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_bearer_validates_known_token() {
        let validator = StaticBearerValidator::new().with_token("valid-token", "alice");
        let builder = AuthUserBuilder::new().with_bearer(validator);
        let h = headers_with("Bearer valid-token");
        let user = builder.build(&h).await.unwrap().expect("user");
        assert_eq!(user.name, "alice");
        assert!(user.authenticated);
    }

    #[tokio::test]
    async fn test_bearer_unknown_token_returns_none() {
        let validator = StaticBearerValidator::new().with_token("valid", "alice");
        let builder = AuthUserBuilder::new().with_bearer(validator);
        let h = headers_with("Bearer wrong");
        assert!(builder.build(&h).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_basic_validates() {
        let validator = StaticBasicValidator::new().with_credentials("admin", "password");
        let builder = AuthUserBuilder::new().with_basic(validator);
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:password");
        let h = headers_with(&format!("Basic {encoded}"));
        let user = builder.build(&h).await.unwrap().expect("user");
        assert_eq!(user.name, "admin");
    }

    #[tokio::test]
    async fn test_basic_wrong_password_returns_none() {
        let validator = StaticBasicValidator::new().with_credentials("admin", "password");
        let builder = AuthUserBuilder::new().with_basic(validator);
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:wrong");
        let h = headers_with(&format!("Basic {encoded}"));
        assert!(builder.build(&h).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_basic_invalid_base64_errors() {
        let validator = StaticBasicValidator::new();
        let builder = AuthUserBuilder::new().with_basic(validator);
        let h = headers_with("Basic !!!not-base64");
        assert!(builder.build(&h).await.is_err());
    }

    #[tokio::test]
    async fn test_no_auth_header_returns_none() {
        let validator = StaticBearerValidator::new().with_token("v", "u");
        let builder = AuthUserBuilder::new().with_bearer(validator);
        let h = HeaderMap::new();
        assert!(builder.build(&h).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_unsupported_scheme_returns_none() {
        let validator = StaticBearerValidator::new();
        let builder = AuthUserBuilder::new().with_bearer(validator);
        let h = headers_with("Digest abcd");
        assert!(builder.build(&h).await.unwrap().is_none());
    }
}
