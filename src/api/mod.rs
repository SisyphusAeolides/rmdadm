//! REST API module for remote management
//! Provides HTTP endpoints for array management and monitoring

pub mod auth;
pub mod handlers;
pub mod models;
pub mod openapi;
pub mod rate_limit;
pub mod routes;
pub mod web;

use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::{info, error, warn};

/// API error response
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": self.message,
            "status": self.status.as_u16(),
        }));
        (self.status, body).into_response()
    }
}

impl From<crate::error::MdError> for ApiError {
    fn from(err: crate::error::MdError) -> Self {
        error!("API error: {}", err);
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

pub async fn start_server(addr: SocketAddr, config: crate::config::Config) -> Result<(), crate::error::MdError> {
    info!("Starting daemon on {}", addr);
    
    // Initialize authentication state from config
    let auth_config = auth::AuthConfig {
        jwt_secret: config.auth.jwt_secret.clone(),
        token_expiry_hours: config.auth.token_expiry_hours,
    };
    let auth_state = std::sync::Arc::new(tokio::sync::RwLock::new(auth_config));
    
    // Set environment variables for auth module compatibility
    if let Some(api_key) = &config.auth.api_key {
        std::env::set_var("RMDADM_API_KEY", api_key);
    }
    std::env::set_var("RMDADM_ADMIN_USER", &config.auth.admin_user);
    std::env::set_var("RMDADM_ADMIN_PASSWORD", &config.auth.admin_password);
    if config.auth.disable_auth {
        std::env::set_var("RMDADM_DISABLE_AUTH", "1");
    }
    
    // Initialize rate limiter from config
    let rate_limit_config = rate_limit::RateLimitConfig {
        max_requests: config.rate_limit.max_requests,
        window_duration: config.rate_limit_duration(),
        enabled: config.rate_limit.enabled,
    };
    let rate_limiter = std::sync::Arc::new(rate_limit::RateLimiter::new(rate_limit_config.clone()));
    
    // Log authentication configuration
    if std::env::var("RMDADM_DISABLE_AUTH").is_ok() {
        warn!("⚠️  Authentication is DISABLED - not recommended for production!");
    } else if auth::is_api_key_enabled() {
        info!("🔑 API Key authentication enabled");
    } else {
        info!("🔐 JWT authentication enabled");
        info!("Default credentials: admin/changeme (change via RMDADM_ADMIN_USER/RMDADM_ADMIN_PASSWORD)");
    }
    
    // Log rate limiting configuration
    if rate_limit_config.enabled {
        info!("🚦 Rate limiting enabled: {} requests per {} seconds", 
              rate_limit_config.max_requests, 
              rate_limit_config.window_duration.as_secs());
    } else {
        warn!("⚠️  Rate limiting is DISABLED");
    }
    
    // Spawn background monitoring task
    tokio::spawn(crate::daemon::run_monitor_loop());

    let app = Router::new()
        .merge(routes::web_routes())
        .merge(routes::swagger_routes())
        .merge(routes::auth_routes(auth_state.clone()))
        .merge(routes::array_routes(auth_state.clone()))
        .merge(routes::health_routes())
        .merge(routes::metrics_routes())
        .layer(axum::middleware::from_fn(move |addr, req, next| {
            let limiter = rate_limiter.clone();
            rate_limit::rate_limit_middleware(addr, limiter, req, next)
        }))
        .layer(TraceLayer::new_for_http());
    
    info!("📚 API documentation available at http://{}/swagger-ui", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::error::MdError::Io)?;
    
    info!("API server listening on {}", addr);
    
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(crate::error::MdError::Io)?;
    
    Ok(())
}
