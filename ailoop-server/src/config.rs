use crate::error::AiloopError;

/// Configuration for starting or embedding an ailoop server.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Host to bind (ignored by embedders who supply their own listener).
    pub host: String,
    /// Port to bind (same caveat as host).
    pub port: u16,
    /// Default channel name.
    pub default_channel: String,
    /// Path prefix for all routes, e.g. `"/hil"`. Must start with `/`, no trailing `/`.
    /// When `None`, routes mount at the root.
    pub base_path: Option<String>,
    /// Enable embedded web UI (requires `web-ui` Cargo feature).
    pub web: bool,
    /// When `Some`, all REST routes and WS upgrades require authentication.
    pub auth: Option<AuthConfig>,
    /// CORS policy; `None` means no CORS headers are added.
    pub cors: Option<CorsConfig>,
}

/// Authentication configuration: a list of accepted bearer tokens / API keys.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Accepted tokens. Checked against `Authorization: Bearer <t>` or `X-Api-Key: <t>`.
    pub tokens: Vec<String>,
}

/// CORS configuration applied as a `tower_http::cors::CorsLayer`.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins, e.g. `["https://app.example.com"]`.
    pub allowed_origins: Vec<String>,
    /// Whether credentials (cookies) are allowed.
    pub allow_credentials: bool,
}

impl ServeConfig {
    /// Normalizes and validates `base_path`.
    ///
    /// Rules:
    /// - Must start with exactly one `/`.
    /// - Must not end with `/` (unless the whole string is `/`, treated as `None`).
    /// - Must not contain `//`.
    /// - Must not equal `/api` (collision with internal REST routes).
    ///
    /// Returns `Ok(None)` when `base_path` is `None` or `"/"`.
    pub fn normalized_base_path(&self) -> Result<Option<String>, AiloopError> {
        match &self.base_path {
            None => Ok(None),
            Some(path) => {
                if path == "/" {
                    return Ok(None);
                }
                if !path.starts_with('/') {
                    return Err(AiloopError::InvalidBasePath(format!(
                        "must start with '/', got: '{path}'"
                    )));
                }
                if path.ends_with('/') {
                    return Err(AiloopError::InvalidBasePath(format!(
                        "must not end with '/', got: '{path}'"
                    )));
                }
                if path.contains("//") {
                    return Err(AiloopError::InvalidBasePath(format!(
                        "must not contain '//', got: '{path}'"
                    )));
                }
                if path == "/api" {
                    return Err(AiloopError::InvalidBasePath(
                        "'/api' collides with internal REST routes".to_string(),
                    ));
                }
                Ok(Some(path.clone()))
            }
        }
    }
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            default_channel: "default".to_string(),
            base_path: None,
            web: false,
            auth: None,
            cors: None,
        }
    }
}
