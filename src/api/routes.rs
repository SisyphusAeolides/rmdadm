//! API route definitions

use axum::{
    routing::{get, post, delete},
    Router,
    middleware,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::handlers::{self, AppState};
use super::auth::{self, AuthState, UserRole};
use super::web;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

/// Array management routes with authentication
pub fn array_routes(auth_state: AuthState) -> Router {
    Router::new()
        // Read-only routes
        .route("/api/v1/arrays", get(handlers::list_arrays))
        .route("/api/v1/arrays/:name", get(handlers::get_array_detail))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth::auth_middleware,
        ))
        // Operator routes (manage existing arrays)
        .route("/api/v1/arrays/:name/manage", post(handlers::manage_array))
        .route("/api/v1/arrays/:name/scrub", post(handlers::scrub_array))
        .route_layer(middleware::from_fn(auth::require_role(UserRole::Operator)))
        // Admin routes (create/delete arrays)
        .route("/api/v1/arrays", post(handlers::create_array))
        .route("/api/v1/arrays/:name", delete(handlers::stop_array))
        .route_layer(middleware::from_fn(auth::require_role(UserRole::Admin)))
}

/// Authentication routes (no auth required)
pub fn auth_routes(auth_state: AuthState) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh_token)
            .route_layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth::auth_middleware,
            )))
        .with_state(auth_state)
}

/// Health check routes (no auth required)
pub fn health_routes() -> Router {
    let state = Arc::new(RwLock::new(AppState::default()));
    
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/health", get(handlers::health_check))
        .with_state(state)
}

/// Metrics routes (no auth required for Prometheus scraping)
pub fn metrics_routes() -> Router {
    Router::new()
        .route("/metrics", get(handlers::metrics))
        .route("/api/v1/metrics", get(handlers::metrics))
}

/// Swagger UI routes
pub fn swagger_routes() -> Router {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", super::openapi::ApiDoc::openapi())
        .into()
}

/// Web UI routes (dashboard)
pub fn web_routes() -> Router {
    Router::new()
        .route("/", get(web::serve_dashboard))
        .route("/dashboard", get(web::serve_dashboard))
        .route("/static/*path", get(web::serve_static))
}
